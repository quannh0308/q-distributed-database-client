//! Q-Distributed-Database Client SDK
//!
//! A high-performance client library for interacting with the q-distributed-database
//! distributed database system.
//!
//! # Features
//!
//! - Connection pooling and automatic failover
//! - Type-safe query building
//! - Transaction support with ACID guarantees
//! - Async/await API using tokio
//! - Automatic retry with exponential backoff
//! - TLS/SSL support
//!
//! # Example
//!
//! ```ignore
//! use q_distributed_db_client::{Client, ConnectionConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ConnectionConfig::default()
//!         .with_hosts(vec!["localhost:7000".to_string()])
//!         .with_credentials("admin", "password");
//!     
//!     let client = Client::connect(config).await?;
//!     let result = client.data().query("SELECT * FROM users").await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod client;
pub mod connection;
pub mod data_client;
pub mod error;
pub mod protocol;
pub mod query_builder;
pub mod result;
pub mod transaction;
pub mod types;
pub mod admin_client;

pub use admin_client::AdminClient;
pub use auth::{AuthToken, AuthenticationManager, Certificate, Credentials};
pub use client::Client;
pub use connection::{Connection, ConnectionManager, ConnectionPool, NodeHealth, PooledConnection, ProtocolType};
pub use data_client::{BatchContext, DataClient, ExecuteResult, PreparedStatement, ResultStream};
pub use error::DatabaseError;
pub use protocol::{AdminRequest, AdminResponse, Message, MessageCodec, MessageType, Request, Response};
pub use query_builder::{QueryBuilder, QueryType, OrderDirection};
pub use result::{ColumnMetadata, DataType, QueryResult, Row};
pub use transaction::{IsolationLevel, Transaction, TransactionRequest, TransactionResponse};
pub use types::*;

/// Result type alias using DatabaseError
pub type Result<T> = std::result::Result<T, DatabaseError>;
