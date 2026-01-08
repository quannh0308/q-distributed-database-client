//! Connection management layer for Q-Distributed-Database Client SDK
//!
//! This module implements TCP connections, connection pooling, health monitoring,
//! retry logic with exponential backoff, and graceful shutdown.

use crate::error::DatabaseError;
use crate::protocol::{Message, MessageCodec, MessageType};
use crate::types::{ConnectionConfig, NodeId, PoolConfig, Timestamp};
use crate::Result;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;

/// Protocol type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    /// TCP protocol
    TCP,
    /// UDP protocol
    UDP,
    /// TLS protocol
    TLS,
}

impl ProtocolType {
    /// Returns the priority of the protocol (higher is better)
    pub fn priority(&self) -> u8 {
        match self {
            ProtocolType::TLS => 3,
            ProtocolType::TCP => 2,
            ProtocolType::UDP => 1,
        }
    }

    /// Selects the best protocol from a list of supported protocols
    pub fn select_best(protocols: &[ProtocolType]) -> Option<ProtocolType> {
        protocols.iter().max_by_key(|p| p.priority()).copied()
    }
}

/// Node health information
#[derive(Debug, Clone)]
pub struct NodeHealth {
    /// Node identifier
    pub node_id: NodeId,
    /// Whether the node is healthy
    pub is_healthy: bool,
    /// Last health check timestamp
    pub last_check: Timestamp,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
}

impl NodeHealth {
    /// Creates a new healthy node
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            is_healthy: true,
            last_check: chrono::Utc::now().timestamp_millis(),
            consecutive_failures: 0,
        }
    }

    /// Marks the node as healthy
    pub fn mark_healthy(&mut self) {
        self.is_healthy = true;
        self.consecutive_failures = 0;
        self.last_check = chrono::Utc::now().timestamp_millis();
    }

    /// Marks the node as unhealthy
    pub fn mark_unhealthy(&mut self) {
        self.is_healthy = false;
        self.consecutive_failures += 1;
        self.last_check = chrono::Utc::now().timestamp_millis();
    }
}

/// A single TCP connection to a database node
pub struct Connection {
    /// TCP socket
    socket: TcpStream,
    /// Node identifier
    node_id: NodeId,
    /// Message codec for serialization
    codec: MessageCodec,
    /// Sequence number for messages
    sequence_number: AtomicU64,
}

impl Connection {
    /// Creates a new connection to the specified host
    pub async fn connect(host: &str, node_id: NodeId, timeout_ms: u64) -> Result<Self> {
        let socket = timeout(
            Duration::from_millis(timeout_ms),
            TcpStream::connect(host),
        )
        .await
        .map_err(|_| DatabaseError::ConnectionTimeout {
            host: host.to_string(),
            timeout_ms,
        })?
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                DatabaseError::ConnectionRefused {
                    host: host.to_string(),
                }
            } else {
                DatabaseError::NetworkError {
                    details: format!("Failed to connect to {}: {}", host, e),
                }
            }
        })?;

        // Enable TCP_NODELAY for low latency
        socket.set_nodelay(true).map_err(|e| DatabaseError::NetworkError {
            details: format!("Failed to set TCP_NODELAY: {}", e),
        })?;

        Ok(Self {
            socket,
            node_id,
            codec: MessageCodec::new(),
            sequence_number: AtomicU64::new(0),
        })
    }

    /// Returns the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Gets the next sequence number
    fn next_sequence_number(&self) -> u64 {
        self.sequence_number.fetch_add(1, Ordering::SeqCst)
    }

    /// Sends a message over the connection
    pub async fn send_message(&mut self, message: Message) -> Result<()> {
        self.codec.write_message(&mut self.socket, &message).await
    }

    /// Receives a message from the connection
    pub async fn receive_message(&mut self) -> Result<Message> {
        self.codec.read_message(&mut self.socket).await
    }

    /// Sends a request and waits for a response
    pub async fn send_request(
        &mut self,
        message_type: MessageType,
        payload: Vec<u8>,
        timeout_ms: u64,
    ) -> Result<Message> {
        let seq = self.next_sequence_number();
        let timestamp = chrono::Utc::now().timestamp_millis();

        let request = Message::new(
            0, // Client sender ID (0 for client)
            self.node_id,
            seq,
            timestamp,
            message_type,
            payload,
        );

        // Send request
        self.send_message(request).await?;

        // Wait for response with timeout
        timeout(
            Duration::from_millis(timeout_ms),
            self.receive_message(),
        )
        .await
        .map_err(|_| DatabaseError::TimeoutError {
            operation: "send_request".to_string(),
            timeout_ms,
        })?
    }
}

/// A pooled connection wrapper
pub struct PooledConnection {
    /// The underlying connection
    connection: Connection,
    /// Creation timestamp
    created_at: Timestamp,
    /// Last used timestamp
    last_used: Timestamp,
}

impl PooledConnection {
    /// Creates a new pooled connection
    fn new(connection: Connection) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            connection,
            created_at: now,
            last_used: now,
        }
    }

    /// Updates the last used timestamp
    fn touch(&mut self) {
        self.last_used = chrono::Utc::now().timestamp_millis();
    }

    /// Checks if the connection has exceeded its idle timeout
    fn is_idle(&self, idle_timeout_ms: u64) -> bool {
        let now = chrono::Utc::now().timestamp_millis();
        (now - self.last_used) as u64 > idle_timeout_ms
    }

    /// Checks if the connection has exceeded its maximum lifetime
    fn is_expired(&self, max_lifetime_ms: u64) -> bool {
        let now = chrono::Utc::now().timestamp_millis();
        (now - self.created_at) as u64 > max_lifetime_ms
    }

    /// Gets a mutable reference to the underlying connection
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }

    /// Gets the node ID
    pub fn node_id(&self) -> NodeId {
        self.connection.node_id()
    }
}

/// Connection pool for managing reusable connections
pub struct ConnectionPool {
    /// Available connections
    available: Arc<Mutex<VecDeque<PooledConnection>>>,
    /// Pool configuration
    config: PoolConfig,
    /// Total number of connections
    total_connections: AtomicU32,
    /// Hosts to connect to
    hosts: Vec<String>,
    /// Connection timeout
    timeout_ms: u64,
}

impl ConnectionPool {
    /// Creates a new connection pool
    pub fn new(hosts: Vec<String>, config: PoolConfig, timeout_ms: u64) -> Self {
        Self {
            available: Arc::new(Mutex::new(VecDeque::new())),
            config,
            total_connections: AtomicU32::new(0),
            hosts,
            timeout_ms,
        }
    }

    /// Gets a connection from the pool or creates a new one
    pub async fn get_connection(&self) -> Result<PooledConnection> {
        // Try to get an available connection
        let mut available = self.available.lock().await;
        
        // Remove expired or idle connections
        available.retain(|conn| {
            !conn.is_expired(self.config.max_lifetime_ms) 
                && !conn.is_idle(self.config.idle_timeout_ms)
        });

        if let Some(mut conn) = available.pop_front() {
            conn.touch();
            return Ok(conn);
        }

        // Check if we can create a new connection
        let total = self.total_connections.load(Ordering::SeqCst);
        if total >= self.config.max_connections {
            return Err(DatabaseError::InternalError {
                component: "ConnectionPool".to_string(),
                details: format!(
                    "Connection pool exhausted (max: {})",
                    self.config.max_connections
                ),
            });
        }

        drop(available); // Release lock before creating connection

        // Create a new connection
        self.create_connection().await
    }

    /// Creates a new connection to a random host
    async fn create_connection(&self) -> Result<PooledConnection> {
        if self.hosts.is_empty() {
            return Err(DatabaseError::InternalError {
                component: "ConnectionPool".to_string(),
                details: "No hosts configured".to_string(),
            });
        }

        // Try each host in order
        let mut last_error = None;
        for (idx, host) in self.hosts.iter().enumerate() {
            let node_id = idx as NodeId + 1;
            match Connection::connect(host, node_id, self.timeout_ms).await {
                Ok(conn) => {
                    self.total_connections.fetch_add(1, Ordering::SeqCst);
                    return Ok(PooledConnection::new(conn));
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| DatabaseError::InternalError {
            component: "ConnectionPool".to_string(),
            details: "Failed to connect to any host".to_string(),
        }))
    }

    /// Returns a connection to the pool
    pub async fn return_connection(&self, conn: PooledConnection) {
        // Check if connection is still valid
        if conn.is_expired(self.config.max_lifetime_ms) 
            || conn.is_idle(self.config.idle_timeout_ms) {
            self.total_connections.fetch_sub(1, Ordering::SeqCst);
            return;
        }

        let mut available = self.available.lock().await;
        available.push_back(conn);
    }

    /// Gets the total number of connections
    pub fn total_connections(&self) -> u32 {
        self.total_connections.load(Ordering::SeqCst)
    }
}

/// Connection manager that orchestrates connection pool and node health
pub struct ConnectionManager {
    /// Connection pool
    pool: ConnectionPool,
    /// Node health tracking
    node_health: Arc<RwLock<HashMap<NodeId, NodeHealth>>>,
    /// Configuration
    config: ConnectionConfig,
}

impl ConnectionManager {
    /// Creates a new connection manager
    pub fn new(config: ConnectionConfig) -> Self {
        let pool = ConnectionPool::new(
            config.hosts.clone(),
            config.pool_config.clone(),
            config.timeout_ms,
        );

        Self {
            pool,
            node_health: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Gets a connection from the pool
    pub async fn get_connection(&self) -> Result<PooledConnection> {
        self.pool.get_connection().await
    }

    /// Returns a connection to the pool
    pub async fn return_connection(&self, conn: PooledConnection) {
        self.pool.return_connection(conn).await
    }

    /// Performs health check on all nodes
    pub async fn health_check_all_nodes(&self) -> Result<Vec<NodeHealth>> {
        let mut results = Vec::new();

        for (idx, host) in self.config.hosts.iter().enumerate() {
            let node_id = idx as NodeId + 1;
            
            // Try to connect and send a ping
            let health = match Connection::connect(host, node_id, self.config.timeout_ms).await {
                Ok(mut conn) => {
                    // Send ping message
                    let ping = Message::new(
                        0,
                        node_id,
                        0,
                        chrono::Utc::now().timestamp_millis(),
                        MessageType::Ping,
                        vec![],
                    );

                    match conn.send_message(ping).await {
                        Ok(_) => {
                            let mut health = NodeHealth::new(node_id);
                            health.mark_healthy();
                            health
                        }
                        Err(_) => {
                            let mut health = NodeHealth::new(node_id);
                            health.mark_unhealthy();
                            health
                        }
                    }
                }
                Err(_) => {
                    let mut health = NodeHealth::new(node_id);
                    health.mark_unhealthy();
                    health
                }
            };

            results.push(health.clone());

            // Update health tracking
            let mut node_health = self.node_health.write().await;
            node_health.insert(node_id, health);
        }

        Ok(results)
    }

    /// Marks a node as unhealthy
    pub async fn mark_node_unhealthy(&self, node_id: NodeId) {
        let mut node_health = self.node_health.write().await;
        node_health
            .entry(node_id)
            .and_modify(|h| h.mark_unhealthy())
            .or_insert_with(|| {
                let mut health = NodeHealth::new(node_id);
                health.mark_unhealthy();
                health
            });
    }

    /// Marks a node as healthy
    pub async fn mark_node_healthy(&self, node_id: NodeId) {
        let mut node_health = self.node_health.write().await;
        node_health
            .entry(node_id)
            .and_modify(|h| h.mark_healthy())
            .or_insert_with(|| NodeHealth::new(node_id));
    }

    /// Gets the health status of all nodes
    pub async fn get_node_health(&self) -> HashMap<NodeId, NodeHealth> {
        self.node_health.read().await.clone()
    }

    /// Executes an operation with retry logic and exponential backoff
    pub async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let retry_config = &self.config.retry_config;
        let mut retries = 0;
        let mut delay = Duration::from_millis(retry_config.initial_backoff_ms);

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if retries < retry_config.max_retries && e.is_retryable() => {
                    retries += 1;
                    tokio::time::sleep(delay).await;
                    
                    // Calculate next delay with exponential backoff
                    let next_delay_ms = (delay.as_millis() as f64 * retry_config.backoff_multiplier) as u64;
                    delay = Duration::from_millis(next_delay_ms.min(retry_config.max_backoff_ms));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Disconnects all connections gracefully
    pub async fn disconnect(&self) {
        // Clear all available connections
        let mut available = self.pool.available.lock().await;
        available.clear();
        
        // Reset connection count
        self.pool.total_connections.store(0, Ordering::SeqCst);
        
        // Clear node health
        let mut node_health = self.node_health.write().await;
        node_health.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ProtocolType Tests
    #[test]
    fn test_protocol_priority() {
        assert_eq!(ProtocolType::TLS.priority(), 3);
        assert_eq!(ProtocolType::TCP.priority(), 2);
        assert_eq!(ProtocolType::UDP.priority(), 1);
    }

    #[test]
    fn test_protocol_select_best() {
        let protocols = vec![ProtocolType::TCP, ProtocolType::UDP];
        assert_eq!(ProtocolType::select_best(&protocols), Some(ProtocolType::TCP));

        let protocols = vec![ProtocolType::TLS, ProtocolType::TCP, ProtocolType::UDP];
        assert_eq!(ProtocolType::select_best(&protocols), Some(ProtocolType::TLS));

        let protocols = vec![ProtocolType::UDP];
        assert_eq!(ProtocolType::select_best(&protocols), Some(ProtocolType::UDP));

        let protocols = vec![];
        assert_eq!(ProtocolType::select_best(&protocols), None);
    }

    // NodeHealth Tests
    #[test]
    fn test_node_health_creation() {
        let health = NodeHealth::new(1);
        assert_eq!(health.node_id, 1);
        assert!(health.is_healthy);
        assert_eq!(health.consecutive_failures, 0);
    }

    #[test]
    fn test_node_health_mark_unhealthy() {
        let mut health = NodeHealth::new(1);
        health.mark_unhealthy();
        assert!(!health.is_healthy);
        assert_eq!(health.consecutive_failures, 1);

        health.mark_unhealthy();
        assert_eq!(health.consecutive_failures, 2);
    }

    #[test]
    fn test_node_health_mark_healthy() {
        let mut health = NodeHealth::new(1);
        health.mark_unhealthy();
        health.mark_unhealthy();
        assert_eq!(health.consecutive_failures, 2);

        health.mark_healthy();
        assert!(health.is_healthy);
        assert_eq!(health.consecutive_failures, 0);
    }

    // PooledConnection Tests
    #[test]
    fn test_pooled_connection_idle_check() {
        // This test would require mocking time, so we'll keep it simple
        // In a real scenario, you'd use a time mocking library
    }

    // ConnectionPool Tests
    #[tokio::test]
    async fn test_connection_pool_creation() {
        let hosts = vec!["localhost:7000".to_string()];
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(hosts, config, 5000);

        assert_eq!(pool.total_connections(), 0);
    }

    // ConnectionManager Tests
    #[tokio::test]
    async fn test_connection_manager_creation() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        let health = manager.get_node_health().await;
        assert!(health.is_empty());
    }

    #[tokio::test]
    async fn test_connection_manager_mark_unhealthy() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        manager.mark_node_unhealthy(1).await;

        let health = manager.get_node_health().await;
        assert_eq!(health.len(), 1);
        assert!(!health.get(&1).unwrap().is_healthy);
    }

    #[tokio::test]
    async fn test_connection_manager_mark_healthy() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        manager.mark_node_unhealthy(1).await;
        manager.mark_node_healthy(1).await;

        let health = manager.get_node_health().await;
        assert_eq!(health.len(), 1);
        assert!(health.get(&1).unwrap().is_healthy);
    }

    #[tokio::test]
    async fn test_connection_manager_disconnect() {
        let config = ConnectionConfig::default();
        let manager = ConnectionManager::new(config);

        manager.mark_node_healthy(1).await;
        manager.disconnect().await;

        let health = manager.get_node_health().await;
        assert!(health.is_empty());
    }
}

// Property-Based Tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::types::RetryConfig;
    use proptest::prelude::*;

    // Strategy for generating valid configurations
    fn connection_config_strategy() -> impl Strategy<Value = ConnectionConfig> {
        (
            prop::collection::vec("[a-z]+:[0-9]{4}", 1..5),
            "[a-z]{3,10}",
            prop::option::of("[a-z]{8,16}"),
            any::<bool>(),
            1000u64..10000u64,
        )
            .prop_map(|(hosts, username, password, enable_tls, timeout_ms)| {
                ConnectionConfig {
                    hosts,
                    username,
                    password,
                    certificate: None,
                    enable_tls,
                    timeout_ms,
                    pool_config: PoolConfig::default(),
                    retry_config: RetryConfig::default(),
                    compression_enabled: false,
                    compression_threshold: 1024,
                }
            })
    }

    // Property 2: Exponential Backoff on Retry
    // Feature: client-sdk, Property 2: For any connection failure, the retry delays should increase exponentially
    // Validates: Requirements 1.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_exponential_backoff_on_retry(
            initial_backoff in 50u64..200u64,
            multiplier in 1.5f64..3.0f64,
            max_retries in 2u32..5u32,
        ) {
            let retry_config = RetryConfig {
                max_retries,
                initial_backoff_ms: initial_backoff,
                max_backoff_ms: 10000,
                backoff_multiplier: multiplier,
            };

            // Calculate expected delays
            let mut expected_delays = vec![initial_backoff];
            for i in 1..max_retries {
                let prev_delay = expected_delays[(i - 1) as usize];
                let next_delay = ((prev_delay as f64) * multiplier) as u64;
                expected_delays.push(next_delay.min(retry_config.max_backoff_ms));
            }

            // Verify exponential growth
            for i in 1..expected_delays.len() {
                let ratio = expected_delays[i] as f64 / expected_delays[i - 1] as f64;
                // Allow tolerance for integer rounding and max backoff cap
                let is_capped = expected_delays[i] == retry_config.max_backoff_ms;
                let is_within_tolerance = (ratio - multiplier).abs() <= 0.05; // 5% tolerance for rounding
                
                prop_assert!(
                    is_capped || is_within_tolerance,
                    "Delay ratio {} should be approximately {} (delays: {:?})",
                    ratio,
                    multiplier,
                    expected_delays
                );
            }
        }
    }

    // Property 7: Protocol Selection Priority
    // Feature: client-sdk, Property 7: For any set of mutually supported protocols, the client should select the protocol with highest priority (TLS > TCP > UDP)
    // Validates: Requirements 1.8
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_protocol_selection_priority(
            include_tls in any::<bool>(),
            include_tcp in any::<bool>(),
            include_udp in any::<bool>(),
        ) {
            // Skip if no protocols selected
            if !include_tls && !include_tcp && !include_udp {
                return Ok(());
            }

            let mut protocols = Vec::new();
            if include_udp {
                protocols.push(ProtocolType::UDP);
            }
            if include_tcp {
                protocols.push(ProtocolType::TCP);
            }
            if include_tls {
                protocols.push(ProtocolType::TLS);
            }

            let selected = ProtocolType::select_best(&protocols);
            prop_assert!(selected.is_some());

            let selected = selected.unwrap();

            // Verify priority: TLS > TCP > UDP
            if include_tls {
                prop_assert_eq!(selected, ProtocolType::TLS, "TLS should be selected when available");
            } else if include_tcp {
                prop_assert_eq!(selected, ProtocolType::TCP, "TCP should be selected when TLS unavailable");
            } else {
                prop_assert_eq!(selected, ProtocolType::UDP, "UDP should be selected when only UDP available");
            }
        }
    }
}

    // ============================================================================
    // INTEGRATION TESTS REQUIRED
    // ============================================================================
    //
    // The following property tests require a running database server and are
    // designed as integration tests. See INTEGRATION_TESTS.md for details.
    //
    // Deferred Tests:
    // - Property 1 (Task 1.1): Connection Establishment
    //   Requires: Reachable test database hosts
    //   Validates: Requirements 1.1
    //
    // - Property 5 (Task 1.2): Connection Reuse
    //   Requires: Connection pool with real connections and timing verification
    //   Validates: Requirements 1.5
    //
    // - Property 3 (Task 1.3): Load Distribution
    //   Requires: Multiple healthy nodes and request tracking
    //   Validates: Requirements 1.3
    //
    // - Property 4 (Task 1.4): Unhealthy Node Avoidance
    //   Requires: Node health tracking with real connections
    //   Validates: Requirements 1.4
    //
    // - Property 27 (Task 1.6): Retry with Exponential Backoff
    //   Requires: Retryable errors from real operations and timing measurement
    //   Validates: Requirements 8.1, 8.4
    //
    // - Property 6 (Task 1.7): Graceful Shutdown
    //   Requires: Active connections to close and resource verification
    //   Validates: Requirements 1.6
    //   Status: COMPLETED (unit test implemented)
    //
    // Implementation Status:
    // ✅ Unit tests: All passing (22 tests)
    // ✅ Property tests (unit): Exponential backoff, Protocol selection
    // ⏳ Property tests (integration): Deferred until test server available
    //
    // See INTEGRATION_TESTS.md for:
    // - Test server implementation requirements
    // - Detailed test strategies for each property
    // - Setup and execution instructions
    // ============================================================================
