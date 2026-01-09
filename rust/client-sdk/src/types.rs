//! Core data types for the Q-Distributed-Database Client SDK
//!
//! This module defines the fundamental data types used throughout the SDK,
//! including node identifiers, values, timestamps, and configuration types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a database node
///
/// Each node in the distributed database cluster has a unique ID.
pub type NodeId = u64;

/// Unique identifier for a transaction
pub type TransactionId = u64;

/// Unique identifier for a prepared statement
pub type StatementId = u64;

/// Unique identifier for a user
pub type UserId = u64;

/// Unix timestamp in milliseconds
pub type Timestamp = i64;

/// Database value type
///
/// Represents all possible value types that can be stored in the database
/// and exchanged between client and server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// 64-bit signed integer
    Int(i64),
    /// 64-bit floating point number
    Float(f64),
    /// UTF-8 string
    String(String),
    /// Binary data
    Bytes(Vec<u8>),
    /// Timestamp with timezone
    Timestamp(DateTime<Utc>),
}

impl Value {
    /// Returns true if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Attempts to convert the value to a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Attempts to convert the value to an integer
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Attempts to convert the value to a float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Attempts to convert the value to a string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Attempts to convert the value to bytes
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b.as_slice()),
            _ => None,
        }
    }

    /// Attempts to convert the value to a timestamp
    pub fn as_timestamp(&self) -> Option<DateTime<Utc>> {
        match self {
            Value::Timestamp(ts) => Some(*ts),
            _ => None,
        }
    }

    /// Returns the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "Bool",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::Bytes(_) => "Bytes",
            Value::Timestamp(_) => "Timestamp",
        }
    }

    /// Converts the value to i64, returning an error if conversion fails
    pub fn as_i64(&self) -> crate::Result<i64> {
        match self {
            Value::Int(i) => Ok(*i),
            _ => Err(crate::error::DatabaseError::TypeConversionError {
                from: self.type_name().to_string(),
                to: "i64",
                value: format!("{:?}", self),
            }),
        }
    }

    /// Converts the value to f64, returning an error if conversion fails
    /// Supports conversion from Int to f64
    pub fn as_f64(&self) -> crate::Result<f64> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Int(i) => Ok(*i as f64),
            _ => Err(crate::error::DatabaseError::TypeConversionError {
                from: self.type_name().to_string(),
                to: "f64",
                value: format!("{:?}", self),
            }),
        }
    }

    /// Converts the value to String, returning an error if conversion fails
    pub fn as_string(&self) -> crate::Result<String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            _ => Err(crate::error::DatabaseError::TypeConversionError {
                from: self.type_name().to_string(),
                to: "String",
                value: format!("{:?}", self),
            }),
        }
    }

    /// Converts the value to bool, returning an error if conversion fails
    pub fn as_bool_result(&self) -> crate::Result<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(crate::error::DatabaseError::TypeConversionError {
                from: self.type_name().to_string(),
                to: "bool",
                value: format!("{:?}", self),
            }),
        }
    }

    /// Converts the value to Vec<u8>, returning an error if conversion fails
    pub fn as_bytes_vec(&self) -> crate::Result<Vec<u8>> {
        match self {
            Value::Bytes(b) => Ok(b.clone()),
            _ => Err(crate::error::DatabaseError::TypeConversionError {
                from: self.type_name().to_string(),
                to: "Vec<u8>",
                value: format!("{:?}", self),
            }),
        }
    }

    /// Converts the value to DateTime<Utc>, returning an error if conversion fails
    pub fn as_timestamp_result(&self) -> crate::Result<DateTime<Utc>> {
        match self {
            Value::Timestamp(ts) => Ok(*ts),
            _ => Err(crate::error::DatabaseError::TypeConversionError {
                from: self.type_name().to_string(),
                to: "DateTime<Utc>",
                value: format!("{:?}", self),
            }),
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Self {
        Value::Bytes(b)
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(ts: DateTime<Utc>) -> Self {
        Value::Timestamp(ts)
    }
}

/// Connection configuration
///
/// Configures how the client connects to the database cluster.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// List of database node addresses (host:port)
    pub hosts: Vec<String>,
    /// Username for authentication
    pub username: String,
    /// Password for authentication (optional if using certificate auth)
    pub password: Option<String>,
    /// Client certificate for TLS authentication
    pub certificate: Option<Vec<u8>>,
    /// Enable TLS encryption
    pub enable_tls: bool,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
    /// Connection pool configuration
    pub pool_config: PoolConfig,
    /// Retry configuration
    pub retry_config: RetryConfig,
    /// Enable message compression
    pub compression_enabled: bool,
    /// Compression threshold in bytes
    pub compression_threshold: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            hosts: vec!["localhost:7000".to_string()],
            username: String::new(),
            password: None,
            certificate: None,
            enable_tls: false,
            timeout_ms: 5000,
            pool_config: PoolConfig::default(),
            retry_config: RetryConfig::default(),
            compression_enabled: false,
            compression_threshold: 1024,
        }
    }
}

impl ConnectionConfig {
    /// Creates a new connection configuration with the given hosts
    pub fn new(hosts: Vec<String>) -> Self {
        Self {
            hosts,
            ..Default::default()
        }
    }

    /// Sets the hosts
    pub fn with_hosts(mut self, hosts: Vec<String>) -> Self {
        self.hosts = hosts;
        self
    }

    /// Sets the credentials
    pub fn with_credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = username.into();
        self.password = Some(password.into());
        self
    }

    /// Enables TLS
    pub fn with_tls(mut self, enabled: bool) -> Self {
        self.enable_tls = enabled;
        self
    }

    /// Sets the connection timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets the pool configuration
    pub fn with_pool_config(mut self, pool_config: PoolConfig) -> Self {
        self.pool_config = pool_config;
        self
    }

    /// Sets the retry configuration
    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Enables compression
    pub fn with_compression(mut self, enabled: bool, threshold: usize) -> Self {
        self.compression_enabled = enabled;
        self.compression_threshold = threshold;
        self
    }
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    /// Idle timeout in milliseconds
    pub idle_timeout_ms: u64,
    /// Maximum connection lifetime in milliseconds
    pub max_lifetime_ms: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 20,
            connection_timeout_ms: 5000,
            idle_timeout_ms: 60000,
            max_lifetime_ms: 1800000, // 30 minutes
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u64,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Creates a new retry configuration with custom parameters
    pub fn new(
        max_retries: u32,
        initial_backoff_ms: u64,
        max_backoff_ms: u64,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            max_retries,
            initial_backoff_ms,
            max_backoff_ms,
            backoff_multiplier,
        }
    }

    /// Creates a retry configuration with no retries
    ///
    /// Use this when you want operations to fail immediately without retrying.
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            initial_backoff_ms: 0,
            max_backoff_ms: 0,
            backoff_multiplier: 1.0,
        }
    }

    /// Creates an aggressive retry configuration
    ///
    /// More retries with shorter delays, suitable for operations that need
    /// to succeed quickly or fail fast.
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            initial_backoff_ms: 50,
            max_backoff_ms: 2000,
            backoff_multiplier: 1.5,
        }
    }

    /// Creates a conservative retry configuration
    ///
    /// Fewer retries with longer delays, suitable for operations that can
    /// tolerate longer wait times.
    pub fn conservative() -> Self {
        Self {
            max_retries: 2,
            initial_backoff_ms: 200,
            max_backoff_ms: 10000,
            backoff_multiplier: 3.0,
        }
    }
}

/// User role in the database
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Administrator with full privileges
    Admin,
    /// Regular user with read/write access
    User,
    /// Read-only user
    ReadOnly,
}

/// User permission
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    /// Can read data
    Read,
    /// Can write data
    Write,
    /// Can delete data
    Delete,
    /// Can create tables
    CreateTable,
    /// Can drop tables
    DropTable,
    /// Can manage users
    ManageUsers,
    /// Can manage cluster
    ManageCluster,
}

/// Node health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeHealthStatus {
    /// Node is healthy and accepting requests
    Healthy,
    /// Node is degraded but still functional
    Degraded,
    /// Node is unhealthy and not accepting requests
    Unhealthy,
}

/// Information about a database node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub node_id: NodeId,
    /// Node address (host:port)
    pub address: String,
    /// Node health status
    pub health_status: NodeHealthStatus,
    /// Last health check timestamp
    pub last_check: Timestamp,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
}

// ============================================================================
// Admin Client Types
// ============================================================================

/// Node status in the cluster
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is healthy and accepting requests
    Healthy,
    /// Node is degraded but still functional
    Degraded,
    /// Node is unhealthy and not accepting requests
    Unhealthy,
    /// Node is offline
    Offline,
}

/// Node role in the cluster
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Primary node
    Primary,
    /// Replica node
    Replica,
    /// Coordinator node
    Coordinator,
}

/// Information about a cluster node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNodeInfo {
    /// Node identifier
    pub node_id: NodeId,
    /// Hostname
    pub hostname: String,
    /// Port
    pub port: u16,
    /// Node status
    pub status: NodeStatus,
    /// Node role
    pub role: NodeRole,
}

/// Node health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthMetrics {
    /// Node identifier
    pub node_id: NodeId,
    /// CPU usage percentage (0.0 to 100.0)
    pub cpu_usage: f64,
    /// Memory usage percentage (0.0 to 100.0)
    pub memory_usage: f64,
    /// Disk usage percentage (0.0 to 100.0)
    pub disk_usage: f64,
    /// Number of active connections
    pub connection_count: u32,
    /// Query throughput (queries per second)
    pub query_throughput: f64,
}

/// Cluster-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    /// Total number of queries processed
    pub total_queries: u64,
    /// Average query latency in milliseconds
    pub average_latency_ms: f64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Total storage usage in bytes
    pub storage_usage_bytes: u64,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User identifier
    pub user_id: UserId,
    /// Username
    pub username: String,
    /// User roles
    pub roles: Vec<Role>,
    /// User permissions
    pub permissions: Vec<Permission>,
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
}

/// User update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdate {
    /// New password (optional)
    pub password: Option<String>,
    /// New roles (optional)
    pub roles: Option<Vec<Role>>,
}

/// Feature enumeration for protocol feature negotiation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feature {
    /// Message compression support
    Compression,
    /// Heartbeat support
    Heartbeat,
    /// Streaming support
    Streaming,
}

/// Feature negotiation request/response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureNegotiation {
    /// List of supported features
    pub supported_features: Vec<Feature>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // Value Type Tests
    #[test]
    fn test_value_null() {
        let v = Value::Null;
        assert!(v.is_null());
        assert!(v.as_bool().is_none());
        assert!(v.as_int().is_none());
        assert!(v.as_float().is_none());
        assert!(v.as_str().is_none());
        assert!(v.as_bytes().is_none());
        assert!(v.as_timestamp().is_none());
    }

    #[test]
    fn test_value_bool_conversions() {
        let v = Value::from(true);
        assert_eq!(v.as_bool(), Some(true));
        assert!(!v.is_null());
        assert!(v.as_int().is_none());
        assert!(v.as_float().is_none());

        let v = Value::from(false);
        assert_eq!(v.as_bool(), Some(false));
    }

    #[test]
    fn test_value_int_conversions() {
        // Test i64 conversion
        let v = Value::from(42i64);
        assert_eq!(v.as_int(), Some(42));
        assert_eq!(v.as_float(), Some(42.0));
        assert!(v.as_bool().is_none());
        assert!(v.as_str().is_none());

        // Test i32 conversion
        let v = Value::from(100i32);
        assert_eq!(v.as_int(), Some(100));
        assert_eq!(v.as_float(), Some(100.0));

        // Test negative numbers
        let v = Value::from(-42i64);
        assert_eq!(v.as_int(), Some(-42));
        assert_eq!(v.as_float(), Some(-42.0));

        // Test zero
        let v = Value::from(0i64);
        assert_eq!(v.as_int(), Some(0));
        assert_eq!(v.as_float(), Some(0.0));
    }

    #[test]
    fn test_value_float_conversions() {
        let v = Value::from(3.14f64);
        assert_eq!(v.as_float(), Some(3.14));
        assert!(v.as_int().is_none());
        assert!(v.as_bool().is_none());

        // Test negative float
        let v = Value::from(-2.5f64);
        assert_eq!(v.as_float(), Some(-2.5));

        // Test zero float
        let v = Value::from(0.0f64);
        assert_eq!(v.as_float(), Some(0.0));
    }

    #[test]
    fn test_value_string_conversions() {
        // Test String conversion
        let v = Value::from("hello".to_string());
        assert_eq!(v.as_str(), Some("hello"));
        assert!(v.as_int().is_none());
        assert!(v.as_bool().is_none());

        // Test &str conversion
        let v = Value::from("world");
        assert_eq!(v.as_str(), Some("world"));

        // Test empty string
        let v = Value::from("");
        assert_eq!(v.as_str(), Some(""));
    }

    #[test]
    fn test_value_bytes_conversions() {
        let bytes = vec![1, 2, 3, 4, 5];
        let v = Value::from(bytes.clone());
        assert_eq!(v.as_bytes(), Some(bytes.as_slice()));
        assert!(v.as_str().is_none());
        assert!(v.as_int().is_none());

        // Test empty bytes
        let v = Value::from(Vec::<u8>::new());
        assert_eq!(v.as_bytes(), Some(&[] as &[u8]));
    }

    #[test]
    fn test_value_timestamp_conversions() {
        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let v = Value::from(ts);
        assert_eq!(v.as_timestamp(), Some(ts));
        assert!(v.as_int().is_none());
        assert!(v.as_str().is_none());
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_eq!(Value::Float(3.14), Value::Float(3.14));
        assert_eq!(
            Value::String("test".to_string()),
            Value::String("test".to_string())
        );
        assert_eq!(Value::Bytes(vec![1, 2, 3]), Value::Bytes(vec![1, 2, 3]));

        // Test inequality
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_ne!(Value::Null, Value::Int(0));
    }

    #[test]
    fn test_value_clone() {
        let v1 = Value::String("test".to_string());
        let v2 = v1.clone();
        assert_eq!(v1, v2);

        let v1 = Value::Bytes(vec![1, 2, 3]);
        let v2 = v1.clone();
        assert_eq!(v1, v2);
    }

    // ConnectionConfig Tests
    #[test]
    fn test_connection_config_default() {
        let config = ConnectionConfig::default();
        assert_eq!(config.hosts, vec!["localhost:7000".to_string()]);
        assert_eq!(config.username, "");
        assert!(config.password.is_none());
        assert!(config.certificate.is_none());
        assert!(!config.enable_tls);
        assert_eq!(config.timeout_ms, 5000);
        assert!(!config.compression_enabled);
        assert_eq!(config.compression_threshold, 1024);
    }

    #[test]
    fn test_connection_config_new() {
        let hosts = vec!["node1:7000".to_string(), "node2:7000".to_string()];
        let config = ConnectionConfig::new(hosts.clone());
        assert_eq!(config.hosts, hosts);
        assert_eq!(config.timeout_ms, 5000); // Default timeout
    }

    #[test]
    fn test_connection_config_builder() {
        let config = ConnectionConfig::default()
            .with_hosts(vec!["node1:7000".to_string(), "node2:7000".to_string()])
            .with_credentials("admin", "password")
            .with_timeout(10000)
            .with_tls(true)
            .with_compression(true, 2048);

        assert_eq!(config.hosts.len(), 2);
        assert_eq!(config.hosts[0], "node1:7000");
        assert_eq!(config.hosts[1], "node2:7000");
        assert_eq!(config.username, "admin");
        assert_eq!(config.password, Some("password".to_string()));
        assert_eq!(config.timeout_ms, 10000);
        assert!(config.enable_tls);
        assert!(config.compression_enabled);
        assert_eq!(config.compression_threshold, 2048);
    }

    #[test]
    fn test_connection_config_with_pool_config() {
        let pool_config = PoolConfig {
            min_connections: 10,
            max_connections: 50,
            connection_timeout_ms: 3000,
            idle_timeout_ms: 30000,
            max_lifetime_ms: 900000,
        };

        let config = ConnectionConfig::default().with_pool_config(pool_config.clone());
        assert_eq!(config.pool_config.min_connections, 10);
        assert_eq!(config.pool_config.max_connections, 50);
    }

    #[test]
    fn test_connection_config_with_retry_config() {
        let retry_config = RetryConfig {
            max_retries: 5,
            initial_backoff_ms: 200,
            max_backoff_ms: 10000,
            backoff_multiplier: 3.0,
        };

        let config = ConnectionConfig::default().with_retry_config(retry_config.clone());
        assert_eq!(config.retry_config.max_retries, 5);
        assert_eq!(config.retry_config.initial_backoff_ms, 200);
    }

    // PoolConfig Tests
    #[test]
    fn test_pool_config_defaults() {
        let config = PoolConfig::default();
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.connection_timeout_ms, 5000);
        assert_eq!(config.idle_timeout_ms, 60000);
        assert_eq!(config.max_lifetime_ms, 1800000);
    }

    #[test]
    fn test_pool_config_clone() {
        let config1 = PoolConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1.min_connections, config2.min_connections);
        assert_eq!(config1.max_connections, config2.max_connections);
    }

    // RetryConfig Tests
    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 100);
        assert_eq!(config.max_backoff_ms, 5000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_retry_config_clone() {
        let config1 = RetryConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1.max_retries, config2.max_retries);
        assert_eq!(config1.backoff_multiplier, config2.backoff_multiplier);
    }

    #[test]
    fn test_retry_config_new() {
        let config = RetryConfig::new(5, 200, 10000, 3.0);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_backoff_ms, 200);
        assert_eq!(config.max_backoff_ms, 10000);
        assert_eq!(config.backoff_multiplier, 3.0);
    }

    #[test]
    fn test_retry_config_no_retry() {
        let config = RetryConfig::no_retry();
        assert_eq!(config.max_retries, 0);
        assert_eq!(config.initial_backoff_ms, 0);
        assert_eq!(config.max_backoff_ms, 0);
        assert_eq!(config.backoff_multiplier, 1.0);
    }

    #[test]
    fn test_retry_config_aggressive() {
        let config = RetryConfig::aggressive();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_backoff_ms, 50);
        assert_eq!(config.max_backoff_ms, 2000);
        assert_eq!(config.backoff_multiplier, 1.5);
    }

    #[test]
    fn test_retry_config_conservative() {
        let config = RetryConfig::conservative();
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.initial_backoff_ms, 200);
        assert_eq!(config.max_backoff_ms, 10000);
        assert_eq!(config.backoff_multiplier, 3.0);
    }

    // Role and Permission Tests
    #[test]
    fn test_role_equality() {
        assert_eq!(Role::Admin, Role::Admin);
        assert_eq!(Role::User, Role::User);
        assert_eq!(Role::ReadOnly, Role::ReadOnly);
        assert_ne!(Role::Admin, Role::User);
    }

    #[test]
    fn test_permission_equality() {
        assert_eq!(Permission::Read, Permission::Read);
        assert_eq!(Permission::Write, Permission::Write);
        assert_ne!(Permission::Read, Permission::Write);
    }

    // NodeHealthStatus Tests
    #[test]
    fn test_node_health_status_equality() {
        assert_eq!(NodeHealthStatus::Healthy, NodeHealthStatus::Healthy);
        assert_eq!(NodeHealthStatus::Degraded, NodeHealthStatus::Degraded);
        assert_eq!(NodeHealthStatus::Unhealthy, NodeHealthStatus::Unhealthy);
        assert_ne!(NodeHealthStatus::Healthy, NodeHealthStatus::Unhealthy);
    }

    // NodeInfo Tests
    #[test]
    fn test_node_info_creation() {
        let node = NodeInfo {
            node_id: 1,
            address: "localhost:7000".to_string(),
            health_status: NodeHealthStatus::Healthy,
            last_check: 1704067200000,
            consecutive_failures: 0,
        };

        assert_eq!(node.node_id, 1);
        assert_eq!(node.address, "localhost:7000");
        assert_eq!(node.health_status, NodeHealthStatus::Healthy);
        assert_eq!(node.consecutive_failures, 0);
    }

    // Feature Tests
    #[test]
    fn test_feature_equality() {
        assert_eq!(Feature::Compression, Feature::Compression);
        assert_eq!(Feature::Heartbeat, Feature::Heartbeat);
        assert_eq!(Feature::Streaming, Feature::Streaming);
        assert_ne!(Feature::Compression, Feature::Heartbeat);
    }

    #[test]
    fn test_feature_negotiation_creation() {
        let features = vec![Feature::Compression, Feature::Heartbeat];
        let negotiation = FeatureNegotiation {
            supported_features: features.clone(),
        };
        assert_eq!(negotiation.supported_features.len(), 2);
        assert!(negotiation.supported_features.contains(&Feature::Compression));
        assert!(negotiation.supported_features.contains(&Feature::Heartbeat));
    }

    #[test]
    fn test_feature_negotiation_intersection() {
        let client_features = vec![Feature::Compression, Feature::Heartbeat];
        let server_features = vec![Feature::Compression, Feature::Streaming];
        
        // Calculate intersection
        let negotiated: Vec<Feature> = client_features
            .into_iter()
            .filter(|f| server_features.contains(f))
            .collect();
        
        assert_eq!(negotiated.len(), 1);
        assert!(negotiated.contains(&Feature::Compression));
        assert!(!negotiated.contains(&Feature::Heartbeat));
        assert!(!negotiated.contains(&Feature::Streaming));
    }
}
