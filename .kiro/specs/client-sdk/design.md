# Design Document - Client SDK (Task 17)

## Current Context

This document contains the minimal design context needed for **Task 17: Create Documentation and Examples**.

## Task 17 Overview

Task 17 creates comprehensive documentation and example applications to help developers understand and use the SDK effectively. This includes API documentation, getting started guides, and practical examples.

## Documentation Architecture

The documentation system consists of three main components:

```
┌─────────────────────────────────────────────────────────────┐
│                   Documentation System                       │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           API Documentation (Rustdoc)                 │  │
│  │  - Module docs                                        │  │
│  │  - Struct/enum docs                                   │  │
│  │  - Method docs with examples                         │  │
│  │  - Error documentation                                │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           Getting Started Guide                       │  │
│  │  - Installation instructions                          │  │
│  │  - Basic usage examples                               │  │
│  │  - Configuration reference                            │  │
│  │  - Troubleshooting                                    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           Example Applications                        │  │
│  │  - basic_crud.rs                                      │  │
│  │  - transactions.rs                                    │  │
│  │  - connection_pooling.rs                              │  │
│  │  - admin_operations.rs                                │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## 1. API Documentation (Rustdoc)

### Module-Level Documentation

Each module should have comprehensive documentation explaining its purpose and usage:

```rust
//! # Connection Management
//!
//! This module provides connection management functionality for the q-distributed-database
//! client SDK. It handles connection pooling, health checking, and automatic failover.
//!
//! ## Key Components
//!
//! - [`ConnectionManager`]: Manages the connection pool and node health
//! - [`Connection`]: Represents a single connection to a database node
//! - [`ConnectionConfig`]: Configuration for connection behavior
//!
//! ## Example
//!
//! ```rust
//! use distributed_db_client::{ConnectionConfig, ConnectionManager};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig {
//!     hosts: vec!["localhost:7000".to_string()],
//!     ..Default::default()
//! };
//!
//! let manager = ConnectionManager::new(config).await?;
//! let conn = manager.get_connection().await?;
//! # Ok(())
//! # }
//! ```
```

### Struct Documentation

Each public struct should document its purpose, fields, and usage:

```rust
/// Configuration for client connections to q-distributed-database.
///
/// This struct contains all configuration options for establishing and managing
/// connections to the database cluster.
///
/// # Examples
///
/// ```rust
/// use distributed_db_client::ConnectionConfig;
///
/// let config = ConnectionConfig {
///     hosts: vec!["localhost:7000".to_string()],
///     username: "admin".to_string(),
///     password: Some("password".to_string()),
///     timeout_ms: 5000,
///     ..Default::default()
/// };
/// ```
pub struct ConnectionConfig {
    /// List of database node addresses in "host:port" format
    pub hosts: Vec<String>,
    
    /// Username for authentication
    pub username: String,
    
    /// Password for authentication (optional if using certificate auth)
    pub password: Option<String>,
    
    /// Connection timeout in milliseconds (default: 5000)
    pub timeout_ms: u64,
    
    // ... other fields
}
```

### Method Documentation

Each public method should document parameters, return values, errors, and include examples:

```rust
impl Client {
    /// Connects to the q-distributed-database cluster.
    ///
    /// This method establishes connections to the database nodes, performs
    /// authentication, and initializes all client components.
    ///
    /// # Arguments
    ///
    /// * `config` - Connection configuration including hosts, credentials, and timeouts
    ///
    /// # Returns
    ///
    /// Returns a connected `Client` instance on success.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - No database nodes are reachable
    /// - Authentication fails
    /// - Configuration is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// use distributed_db_client::{Client, ConnectionConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig {
    ///     hosts: vec!["localhost:7000".to_string()],
    ///     username: "admin".to_string(),
    ///     password: Some("password".to_string()),
    ///     ..Default::default()
    /// };
    ///
    /// let client = Client::connect(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(config: ConnectionConfig) -> Result<Self> {
        // Implementation
    }
}
```

### Error Documentation

Document all error types with explanations of when they occur:

```rust
/// Errors that can occur when using the client SDK.
///
/// This enum represents all possible errors that can be returned by SDK operations.
/// Errors are categorized by their source (connection, authentication, query, etc.).
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// Connection to the database timed out.
    ///
    /// This error occurs when a connection attempt exceeds the configured timeout.
    /// Consider increasing the timeout or checking network connectivity.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use distributed_db_client::DatabaseError;
    /// match error {
    ///     DatabaseError::ConnectionTimeout { host, timeout_ms } => {
    ///         eprintln!("Failed to connect to {} within {}ms", host, timeout_ms);
    ///     }
    ///     _ => {}
    /// }
    /// ```
    #[error("Connection to {host} timed out after {timeout_ms}ms")]
    ConnectionTimeout {
        host: String,
        timeout_ms: u64,
    },
    
    // ... other error variants
}
```

## 2. Getting Started Guide

### Structure

The getting started guide should be located at `docs/getting-started.md` and include:

```markdown
# Getting Started with Q-Distributed-Database Client SDK

## Installation

Add the SDK to your `Cargo.toml`:

```toml
[dependencies]
distributed-db-client = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

Here's a simple example to get you started:

```rust
use distributed_db_client::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create configuration
    let config = ConnectionConfig {
        hosts: vec!["localhost:7000".to_string()],
        username: "admin".to_string(),
        password: Some("password".to_string()),
        ..Default::default()
    };
    
    // 2. Connect to database
    let client = Client::connect(config).await?;
    
    // 3. Execute a query
    let result = client.data().query("SELECT * FROM users").await?;
    
    println!("Found {} rows", result.rows.len());
    
    // 4. Disconnect
    client.disconnect().await?;
    
    Ok(())
}
```

## Configuration

### Basic Configuration

The minimum configuration requires:
- `hosts`: List of database node addresses
- `username`: Authentication username
- `password`: Authentication password (or certificate)

### Advanced Configuration

```rust
let config = ConnectionConfig {
    hosts: vec!["node1:7000".to_string(), "node2:7000".to_string()],
    username: "admin".to_string(),
    password: Some("password".to_string()),
    enable_tls: true,
    timeout_ms: 10000,
    pool_config: PoolConfig {
        min_connections: 5,
        max_connections: 20,
        ..Default::default()
    },
    retry_config: RetryConfig {
        max_retries: 3,
        initial_backoff_ms: 100,
        ..Default::default()
    },
    ..Default::default()
};
```

## Common Operations

### CRUD Operations

[Examples of INSERT, SELECT, UPDATE, DELETE]

### Transactions

[Example of transaction usage]

### Error Handling

[Example of error handling patterns]

## Troubleshooting

### Connection Issues

**Problem**: Connection timeout errors

**Solution**: 
- Check that the database is running
- Verify the host and port are correct
- Increase the timeout if needed

[More troubleshooting scenarios]
```

## 3. Example Applications

### Example Structure

Each example should be a complete, runnable Rust program in the `examples/` directory:

```
examples/
├── basic_crud.rs
├── transactions.rs
├── connection_pooling.rs
└── admin_operations.rs
```

### Example: basic_crud.rs

```rust
//! Basic CRUD Operations Example
//!
//! This example demonstrates how to perform basic CRUD operations
//! (Create, Read, Update, Delete) using the q-distributed-database client SDK.
//!
//! Run with: cargo run --example basic_crud

use distributed_db_client::{Client, ConnectionConfig, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to database
    let config = ConnectionConfig {
        hosts: vec!["localhost:7000".to_string()],
        username: "admin".to_string(),
        password: Some("password".to_string()),
        ..Default::default()
    };
    
    let client = Client::connect(config).await?;
    println!("Connected to database");
    
    // CREATE: Create a table
    client.data().execute(
        "CREATE TABLE users (id INT PRIMARY KEY, name TEXT, email TEXT)"
    ).await?;
    println!("Created table 'users'");
    
    // INSERT: Add records
    client.data().execute_with_params(
        "INSERT INTO users VALUES (?, ?, ?)",
        &[
            Value::Int(1),
            Value::String("Alice".to_string()),
            Value::String("alice@example.com".to_string()),
        ]
    ).await?;
    println!("Inserted user: Alice");
    
    client.data().execute_with_params(
        "INSERT INTO users VALUES (?, ?, ?)",
        &[
            Value::Int(2),
            Value::String("Bob".to_string()),
            Value::String("bob@example.com".to_string()),
        ]
    ).await?;
    println!("Inserted user: Bob");
    
    // READ: Query records
    let result = client.data().query("SELECT * FROM users").await?;
    println!("\nAll users:");
    for row in &result.rows {
        let id = row.get(0).unwrap();
        let name = row.get(1).unwrap();
        let email = row.get(2).unwrap();
        println!("  ID: {:?}, Name: {:?}, Email: {:?}", id, name, email);
    }
    
    // UPDATE: Modify a record
    client.data().execute_with_params(
        "UPDATE users SET email = ? WHERE id = ?",
        &[
            Value::String("alice.new@example.com".to_string()),
            Value::Int(1),
        ]
    ).await?;
    println!("\nUpdated Alice's email");
    
    // Verify update
    let result = client.data().query_with_params(
        "SELECT * FROM users WHERE id = ?",
        &[Value::Int(1)]
    ).await?;
    println!("Alice's new email: {:?}", result.rows[0].get(2).unwrap());
    
    // DELETE: Remove a record
    client.data().execute_with_params(
        "DELETE FROM users WHERE id = ?",
        &[Value::Int(2)]
    ).await?;
    println!("\nDeleted user: Bob");
    
    // Verify deletion
    let result = client.data().query("SELECT * FROM users").await?;
    println!("Remaining users: {}", result.rows.len());
    
    // Cleanup
    client.data().execute("DROP TABLE users").await?;
    client.disconnect().await?;
    
    println!("\nExample completed successfully!");
    Ok(())
}
```

### Example: transactions.rs

```rust
//! Transaction Example
//!
//! This example demonstrates how to use transactions for atomic operations.
//!
//! Run with: cargo run --example transactions

use distributed_db_client::{Client, ConnectionConfig, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig {
        hosts: vec!["localhost:7000".to_string()],
        username: "admin".to_string(),
        password: Some("password".to_string()),
        ..Default::default()
    };
    
    let client = Client::connect(config).await?;
    
    // Setup: Create accounts table
    client.data().execute(
        "CREATE TABLE accounts (id INT PRIMARY KEY, name TEXT, balance INT)"
    ).await?;
    
    client.data().execute_with_params(
        "INSERT INTO accounts VALUES (?, ?, ?)",
        &[Value::Int(1), Value::String("Alice".to_string()), Value::Int(1000)]
    ).await?;
    
    client.data().execute_with_params(
        "INSERT INTO accounts VALUES (?, ?, ?)",
        &[Value::Int(2), Value::String("Bob".to_string()), Value::Int(500)]
    ).await?;
    
    println!("Initial balances:");
    print_balances(&client).await?;
    
    // Example 1: Successful transaction (commit)
    println!("\n--- Transaction 1: Transfer $200 from Alice to Bob ---");
    {
        let mut tx = client.data().begin_transaction().await?;
        
        // Deduct from Alice
        tx.execute_with_params(
            "UPDATE accounts SET balance = balance - ? WHERE id = ?",
            &[Value::Int(200), Value::Int(1)]
        ).await?;
        
        // Add to Bob
        tx.execute_with_params(
            "UPDATE accounts SET balance = balance + ? WHERE id = ?",
            &[Value::Int(200), Value::Int(2)]
        ).await?;
        
        // Commit transaction
        tx.commit().await?;
        println!("Transaction committed");
    }
    
    print_balances(&client).await?;
    
    // Example 2: Failed transaction (rollback)
    println!("\n--- Transaction 2: Attempt invalid transfer (will rollback) ---");
    {
        let mut tx = client.data().begin_transaction().await?;
        
        // Try to deduct more than Alice has
        tx.execute_with_params(
            "UPDATE accounts SET balance = balance - ? WHERE id = ?",
            &[Value::Int(2000), Value::Int(1)]
        ).await?;
        
        // Manually rollback
        tx.rollback().await?;
        println!("Transaction rolled back");
    }
    
    print_balances(&client).await?;
    
    // Cleanup
    client.data().execute("DROP TABLE accounts").await?;
    client.disconnect().await?;
    
    Ok(())
}

async fn print_balances(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let result = client.data().query("SELECT * FROM accounts ORDER BY id").await?;
    for row in &result.rows {
        let name = row.get(1).unwrap();
        let balance = row.get(2).unwrap();
        println!("  {:?}: ${:?}", name, balance);
    }
    Ok(())
}
```

### Example: connection_pooling.rs

```rust
//! Connection Pooling Example
//!
//! This example demonstrates connection pool configuration and behavior.
//!
//! Run with: cargo run --example connection_pooling

use distributed_db_client::{Client, ConnectionConfig, PoolConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure connection pool
    let config = ConnectionConfig {
        hosts: vec!["localhost:7000".to_string()],
        username: "admin".to_string(),
        password: Some("password".to_string()),
        pool_config: PoolConfig {
            min_connections: 5,
            max_connections: 20,
            connection_timeout_ms: 5000,
            idle_timeout_ms: 60000,
            max_lifetime_ms: 1800000,
        },
        ..Default::default()
    };
    
    let client = Client::connect(config).await?;
    println!("Connected with connection pool");
    
    // Simulate concurrent operations
    let mut handles = vec![];
    
    for i in 0..10 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            let result = client.data().query("SELECT 1").await;
            println!("Query {} completed: {:?}", i, result.is_ok());
        });
        handles.push(handle);
    }
    
    // Wait for all operations
    for handle in handles {
        handle.await?;
    }
    
    println!("\nAll concurrent operations completed");
    
    // Check cluster health
    let health = client.health_check().await?;
    println!("\nCluster health: {}/{} nodes healthy", 
        health.healthy_nodes, health.total_nodes);
    
    client.disconnect().await?;
    Ok(())
}
```

### Example: admin_operations.rs

```rust
//! Admin Operations Example
//!
//! This example demonstrates cluster and user management operations.
//!
//! Run with: cargo run --example admin_operations

use distributed_db_client::{Client, ConnectionConfig, Role, Permission};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig {
        hosts: vec!["localhost:7000".to_string()],
        username: "admin".to_string(),
        password: Some("password".to_string()),
        ..Default::default()
    };
    
    let client = Client::connect(config).await?;
    
    // Cluster Management
    println!("=== Cluster Management ===\n");
    
    // List nodes
    let nodes = client.admin().list_nodes().await?;
    println!("Cluster nodes:");
    for node in &nodes {
        println!("  - Node {}: {} ({})", 
            node.id, node.address, 
            if node.is_healthy { "healthy" } else { "unhealthy" }
        );
    }
    
    // Get cluster metrics
    let metrics = client.admin().get_cluster_metrics().await?;
    println!("\nCluster metrics:");
    println!("  Total nodes: {}", metrics.total_nodes);
    println!("  Total storage: {} GB", metrics.total_storage_gb);
    println!("  Queries per second: {}", metrics.queries_per_second);
    
    // User Management
    println!("\n=== User Management ===\n");
    
    // Create a new user
    let user_id = client.admin().create_user(
        "developer",
        "dev_password",
        &[Role::Developer]
    ).await?;
    println!("Created user 'developer' with ID: {:?}", user_id);
    
    // Grant permissions
    client.admin().grant_permission(
        user_id,
        Permission::Read { database: "app_db".to_string() }
    ).await?;
    println!("Granted read permission on 'app_db'");
    
    // List all users
    let users = client.admin().list_users().await?;
    println!("\nAll users:");
    for user in &users {
        println!("  - {}: {:?}", user.username, user.roles);
    }
    
    // Cleanup: Delete the test user
    client.admin().delete_user(user_id).await?;
    println!("\nDeleted test user");
    
    client.disconnect().await?;
    Ok(())
}
```

## Documentation Generation

### Building Documentation

```bash
# Generate API documentation
cargo doc --no-deps --open

# Run examples
cargo run --example basic_crud
cargo run --example transactions
cargo run --example connection_pooling
cargo run --example admin_operations
```

### Documentation Testing

Rust's documentation system supports testing code examples:

```rust
/// Connects to the database.
///
/// # Examples
///
/// ```
/// use distributed_db_client::{Client, ConnectionConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ConnectionConfig::default();
/// let client = Client::connect(config).await?;
/// # Ok(())
/// # }
/// ```
```

Run documentation tests with:
```bash
cargo test --doc
```

## Implementation Notes

1. **Completeness**: Document all public APIs without exception
2. **Examples**: Every public method should have at least one example
3. **Accuracy**: All examples must compile and run successfully
4. **Clarity**: Use simple, clear language accessible to all skill levels
5. **Consistency**: Follow Rust documentation conventions throughout

---

**Full design available in**: `.kiro/specs/client-sdk/FOUNDATION/design.md`
