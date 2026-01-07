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

pub mod error;
pub mod types;

pub use error::DatabaseError;
pub use types::*;

/// Result type alias using DatabaseError
pub type Result<T> = std::result::Result<T, DatabaseError>;
