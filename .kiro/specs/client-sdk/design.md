# Design Document - Client SDK

## Current Context

This document contains the minimal design context needed for the **current implementation task**.

## Architecture Overview

The Client SDK follows a layered architecture:

```
Client Application
       ↓
Public API Layer (DataClient, AdminClient, QueryBuilder)
       ↓
Connection Management Layer (ConnectionManager, ConnectionPool)
       ↓
Protocol Layer (MessageCodec, Message serialization)
       ↓
Transport Layer (TCP/TLS)
       ↓
Q-Distributed-Database Cluster
```

## Key Components

1. **Client**: Main entry point, manages all sub-components
2. **ConnectionManager**: Connection pooling, health monitoring, failover
3. **MessageCodec**: Bincode serialization with CRC32 checksums
4. **AuthenticationManager**: Token-based authentication
5. **DataClient**: CRUD operations, queries, transactions
6. **QueryBuilder**: Fluent API for SQL construction
7. **AdminClient**: Cluster and user management

## Message Protocol

- **Format**: Bincode serialization
- **Framing**: 4-byte big-endian length prefix + message data
- **Integrity**: CRC32 checksum validation
- **Types**: Ping, Pong, Data, Ack, Error, Heartbeat, ClusterJoin, ClusterLeave, Replication, Transaction

## Current Task Design

### Task 3: Connection Management Implementation

This task implements the connection management layer that handles TCP connections, connection pooling, health monitoring, and automatic failover.

#### Key Components

**1. Connection Struct**
Represents a single TCP connection to a database node:
```rust
pub struct Connection {
    socket: TcpStream,
    node_id: NodeId,
    codec: MessageCodec,
    sequence_number: AtomicU64,
}

impl Connection {
    pub async fn send_message(&mut self, msg: Message) -> Result<()>;
    pub async fn receive_message(&mut self) -> Result<Message>;
    pub async fn send_request(&mut self, request: Request) -> Result<Response>;
    pub fn node_id(&self) -> NodeId;
}
```

**2. ConnectionConfig**
Configuration for connection behavior:
```rust
pub struct ConnectionConfig {
    pub hosts: Vec<String>,           // ["localhost:7000", "node2:7000"]
    pub username: String,
    pub password: Option<String>,
    pub certificate: Option<Certificate>,
    pub enable_tls: bool,
    pub timeout_ms: u64,              // Default: 5000
    pub pool_config: PoolConfig,
    pub retry_config: RetryConfig,
    pub compression_enabled: bool,
    pub compression_threshold: usize,
}

pub struct PoolConfig {
    pub min_connections: u32,         // Default: 5
    pub max_connections: u32,         // Default: 20
    pub connection_timeout_ms: u64,   // Default: 5000
    pub idle_timeout_ms: u64,         // Default: 60000
    pub max_lifetime_ms: u64,         // Default: 1800000 (30 min)
}

pub struct RetryConfig {
    pub max_retries: u32,             // Default: 3
    pub initial_backoff_ms: u64,      // Default: 100
    pub max_backoff_ms: u64,          // Default: 5000
    pub backoff_multiplier: f64,      // Default: 2.0
}
```

**3. ConnectionPool**
Manages a pool of reusable connections:
```rust
pub struct ConnectionPool {
    available: Arc<Mutex<VecDeque<PooledConnection>>>,
    config: PoolConfig,
    total_connections: AtomicU32,
}

impl ConnectionPool {
    pub async fn get_connection(&self) -> Result<PooledConnection>;
    pub async fn return_connection(&self, conn: PooledConnection);
    pub async fn create_connection(&self, host: &str) -> Result<Connection>;
}
```

**4. ConnectionManager**
Orchestrates connection pool and node health:
```rust
pub struct ConnectionManager {
    pool: ConnectionPool,
    node_health: Arc<RwLock<HashMap<NodeId, NodeHealth>>>,
    config: ConnectionConfig,
}

impl ConnectionManager {
    pub async fn get_connection(&self) -> Result<PooledConnection>;
    pub async fn return_connection(&self, conn: PooledConnection);
    pub async fn health_check_all_nodes(&self) -> Result<Vec<NodeHealth>>;
    pub async fn mark_node_unhealthy(&self, node_id: NodeId);
    pub async fn mark_node_healthy(&self, node_id: NodeId);
}

pub struct NodeHealth {
    pub node_id: NodeId,
    pub is_healthy: bool,
    pub last_check: Timestamp,
    pub consecutive_failures: u32,
}
```

**5. Retry Logic with Exponential Backoff**
```rust
async fn execute_with_retry<F, T>(
    &self,
    operation: F,
    retry_config: &RetryConfig,
) -> Result<T>
where
    F: Fn() -> Future<Output = Result<T>>,
{
    let mut retries = 0;
    let mut delay = Duration::from_millis(retry_config.initial_backoff_ms);
    
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if retries < retry_config.max_retries && is_retryable(&e) => {
                retries += 1;
                tokio::time::sleep(delay).await;
                delay = std::cmp::min(
                    Duration::from_millis(
                        (delay.as_millis() as f64 * retry_config.backoff_multiplier) as u64
                    ),
                    Duration::from_millis(retry_config.max_backoff_ms)
                );
            }
            Err(e) => return Err(e),
        }
    }
}

fn is_retryable(error: &DatabaseError) -> bool {
    matches!(error,
        DatabaseError::ConnectionTimeout { .. } |
        DatabaseError::ConnectionLost { .. } |
        DatabaseError::NetworkError { .. } |
        DatabaseError::TimeoutError { .. }
    )
}
```

#### Correctness Properties for Task 3

**Property 1: Connection Establishment**
*For any* valid configuration with reachable hosts, initializing the client should successfully establish at least one TCP connection to a q-distributed-database node.
**Validates: Requirements 1.1**

**Property 2: Exponential Backoff on Retry**
*For any* connection failure, the retry delays should increase exponentially according to the configured backoff multiplier (delay_n = delay_(n-1) * multiplier).
**Validates: Requirements 1.2**

**Property 3: Load Distribution**
*For any* set of healthy nodes, requests should be distributed across all nodes (no single node receives all requests when multiple nodes are available).
**Validates: Requirements 1.3**

**Property 4: Unhealthy Node Avoidance**
*For any* node marked as unhealthy, subsequent requests should not be routed to that node until it is marked healthy again.
**Validates: Requirements 1.4**

**Property 5: Connection Reuse**
*For any* sequence of requests within the connection idle timeout, the same underlying connection should be reused rather than creating new connections.
**Validates: Requirements 1.5**

**Property 6: Graceful Shutdown**
*For any* client with active connections, calling disconnect() should close all connections and release all resources.
**Validates: Requirements 1.6**

**Property 7: Protocol Selection Priority**
*For any* set of mutually supported protocols, the client should select the protocol with highest priority (TLS > TCP > UDP).
**Validates: Requirements 1.8**

**Property 27: Retry with Exponential Backoff**
*For any* retryable error, the client should retry with exponentially increasing delays up to max_retries.
**Validates: Requirements 8.1, 8.4**

#### Implementation Notes

- Use `tokio::net::TcpStream` for async TCP connections
- Implement connection pooling with `Arc<Mutex<VecDeque<>>>` for thread-safe access
- Track sequence numbers per connection for request-response matching
- Implement health checking with periodic ping messages
- Support graceful shutdown with proper resource cleanup
- Use `Default` trait for configuration structs with sensible defaults

---

**Full design with 42 correctness properties available in**: `.kiro/specs/client-sdk/FOUNDATION/design.md`
