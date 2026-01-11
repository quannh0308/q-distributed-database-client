# Q-Distributed-Database Client SDK (Rust)

A high-performance Rust client library for interacting with the [q-distributed-database](https://github.com/quannh0308/q-distributed-database) distributed database system.

## Features

- **Connection Management**: Connection pooling, automatic failover, and health monitoring
- **Authentication**: Token-based authentication with automatic re-authentication
- **CRUD Operations**: Full support for INSERT, SELECT, UPDATE, DELETE operations
- **Query Building**: Type-safe query builder with SQL injection prevention
- **Transactions**: ACID transactions with automatic rollback on failure
- **Async/Await**: Built on tokio for high-performance async I/O
- **Error Handling**: Comprehensive error types with automatic retry logic
- **Message Protocol**: Bincode serialization with CRC32 checksums

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
q-distributed-db-client = "0.1.0"
```

## Quick Start

```rust
use q_distributed_db_client::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure connection
    let config = ConnectionConfig::default()
        .with_hosts(vec!["localhost:7000".to_string()])
        .with_credentials("admin", "password");
    
    // Connect to database
    let client = Client::connect(config).await?;
    
    // Execute queries
    let result = client.data().query("SELECT * FROM users").await?;
    
    Ok(())
}
```

## Project Structure

```
client-sdk/
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── client.rs           # Main Client struct
│   ├── connection.rs       # Connection management and pooling
│   ├── auth.rs             # Authentication and token management
│   ├── data_client.rs      # CRUD operations
│   ├── query_builder.rs    # Type-safe query builder
│   ├── transaction.rs      # Transaction support
│   ├── admin_client.rs     # Admin operations
│   ├── result.rs           # Query result handling
│   ├── protocol.rs         # Message protocol
│   ├── types.rs            # Core data types
│   ├── error.rs            # Error types
│   └── metrics.rs          # Metrics collection
├── examples/
│   ├── basic_crud.rs       # Basic CRUD example
│   ├── transactions.rs     # Transaction example
│   ├── connection_pooling.rs  # Connection pooling example
│   └── admin_operations.rs # Admin operations example
├── tests/
│   └── client_integration.rs  # Integration tests
├── Cargo.toml              # Package configuration
└── README.md               # This file
```

## Core Types

### Error Types

The SDK provides a comprehensive `DatabaseError` enum covering all failure modes:

- **Connection Errors**: `ConnectionTimeout`, `ConnectionRefused`, `ConnectionLost`
- **Authentication Errors**: `AuthenticationFailed`, `TokenExpired`, `InvalidCredentials`
- **Query Errors**: `SyntaxError`, `TableNotFound`, `ColumnNotFound`, `ConstraintViolation`
- **Transaction Errors**: `TransactionAborted`, `DeadlockDetected`, `IsolationViolation`
- **Protocol Errors**: `SerializationError`, `ChecksumMismatch`, `MessageTooLarge`
- **Network Errors**: `NetworkError`, `TimeoutError`

### Data Types

- **Value**: Enum representing all database value types (Null, Bool, Int, Float, String, Bytes, Timestamp)
- **ConnectionConfig**: Configuration for database connections
- **PoolConfig**: Connection pool configuration
- **RetryConfig**: Retry behavior configuration
- **NodeInfo**: Information about database nodes
- **ColumnMetadata**: Column information for query results

## Configuration

### Connection Configuration

```rust
let config = ConnectionConfig::default()
    .with_hosts(vec!["node1:7000".to_string(), "node2:7000".to_string()])
    .with_credentials("username", "password")
    .with_timeout(10000)  // 10 seconds
    .with_tls(true)
    .with_compression(true, 1024);  // Compress messages > 1KB
```

### Pool Configuration

```rust
let pool_config = PoolConfig {
    min_connections: 5,
    max_connections: 20,
    connection_timeout_ms: 5000,
    idle_timeout_ms: 60000,
    max_lifetime_ms: 1800000,  // 30 minutes
};

let config = ConnectionConfig::default()
    .with_pool_config(pool_config);
```

### Retry Configuration

```rust
let retry_config = RetryConfig {
    max_retries: 3,
    initial_backoff_ms: 100,
    max_backoff_ms: 5000,
    backoff_multiplier: 2.0,
};

let config = ConnectionConfig::default()
    .with_retry_config(retry_config);
```

## Development Status

This SDK is feature-complete and ready for use. All core components are implemented:

- ✅ Core error types
- ✅ Core data types and configuration
- ✅ Message protocol layer with bincode serialization
- ✅ Connection management with pooling and health monitoring
- ✅ Authentication with token management
- ✅ Data client with CRUD operations
- ✅ Query builder with SQL injection prevention
- ✅ Transaction support with ACID guarantees
- ✅ Admin client for cluster and user management
- ✅ Metrics collection and monitoring
- ✅ Comprehensive documentation and examples

## Documentation

- **[Getting Started Guide](../../docs/getting-started.md)** - Installation, configuration, and basic usage
- **[API Documentation](https://docs.rs/q-distributed-db-client)** - Complete API reference
- **[Examples](./examples/)** - Practical examples demonstrating common use cases

## Examples

The SDK includes several example applications:

- **[basic_crud.rs](./examples/basic_crud.rs)** - Basic CRUD operations
- **[transactions.rs](./examples/transactions.rs)** - Transaction usage with commit/rollback
- **[connection_pooling.rs](./examples/connection_pooling.rs)** - Connection pool configuration
- **[admin_operations.rs](./examples/admin_operations.rs)** - Cluster and user management

Run examples with:

```bash
cargo run --example basic_crud
cargo run --example transactions
cargo run --example connection_pooling
cargo run --example admin_operations
```

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please see the main repository for contribution guidelines.

## Links

- [q-distributed-database](https://github.com/quannh0308/q-distributed-database) - The database server
- [Documentation](https://docs.rs/q-distributed-db-client) - API documentation
- [Examples](../examples/) - Usage examples
