# Getting Started with Q-Distributed-Database Client SDK

Welcome to the Q-Distributed-Database Client SDK! This guide will help you get up and running quickly.

## Installation

Add the SDK to your `Cargo.toml`:

```toml
[dependencies]
q-distributed-db-client = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

Here's a simple example to connect to the database and execute a query:

```rust
use q_distributed_db_client::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create configuration
    let config = ConnectionConfig::default()
        .with_hosts(vec!["localhost:7000".to_string()])
        .with_credentials("admin", "password");
    
    // 2. Connect to database
    let client = Client::connect(config).await?;
    println!("Connected to database!");
    
    // 3. Execute a query
    let result = client.data().query("SELECT * FROM users").await?;
    println!("Found {} rows", result.rows.len());
    
    // 4. Disconnect gracefully
    client.disconnect().await?;
    
    Ok(())
}
```

## Configuration

### Basic Configuration

The minimum configuration requires:
- `hosts`: List of database node addresses in "host:port" format
- `username`: Authentication username
- `password`: Authentication password (or certificate for TLS)

```rust
let config = ConnectionConfig::default()
    .with_hosts(vec!["localhost:7000".to_string()])
    .with_credentials("admin", "password");
```

### Advanced Configuration

For production use, you'll want to configure connection pooling, timeouts, and retry behavior:

```rust
use q_distributed_db_client::{ConnectionConfig, PoolConfig, RetryConfig};

let config = ConnectionConfig::default()
    .with_hosts(vec![
        "node1.example.com:7000".to_string(),
        "node2.example.com:7000".to_string(),
        "node3.example.com:7000".to_string(),
    ])
    .with_credentials("admin", "password")
    .with_timeout(10000)  // 10 second timeout
    .with_tls(true)       // Enable TLS encryption
    .with_pool_config(PoolConfig {
        min_connections: 5,
        max_connections: 20,
        connection_timeout_ms: 5000,
        idle_timeout_ms: 60000,
        max_lifetime_ms: 1800000,  // 30 minutes
    })
    .with_retry_config(RetryConfig {
        max_retries: 3,
        initial_backoff_ms: 100,
        max_backoff_ms: 5000,
        backoff_multiplier: 2.0,
    })
    .with_compression(true, 1024);  // Enable compression for messages > 1KB
```

### TLS Configuration

For secure connections, enable TLS and provide a client certificate:

```rust
let cert_data = std::fs::read("client-cert.pem")?;

let config = ConnectionConfig::default()
    .with_hosts(vec!["secure-node.example.com:7000".to_string()])
    .with_credentials("admin", "")
    .with_tls(true);
```

## Common Operations

### CRUD Operations

#### INSERT

```rust
use q_distributed_db_client::Value;

// Insert a single row
client.data().execute_with_params(
    "INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
    &[
        Value::Int(1),
        Value::String("Alice".to_string()),
        Value::String("alice@example.com".to_string()),
    ]
).await?;
```

#### SELECT

```rust
// Query all rows
let result = client.data().query("SELECT * FROM users").await?;

// Iterate through results
for row in result.rows {
    let id = row.get_i64(0)?;
    let name = row.get_string(1)?;
    let email = row.get_string(2)?;
    println!("User {}: {} ({})", id, name, email);
}

// Query with parameters
let result = client.data().query_with_params(
    "SELECT * FROM users WHERE age > ?",
    &[Value::Int(18)]
).await?;
```

#### UPDATE

```rust
// Update rows
let result = client.data().execute_with_params(
    "UPDATE users SET email = ? WHERE id = ?",
    &[
        Value::String("newemail@example.com".to_string()),
        Value::Int(1),
    ]
).await?;

println!("Updated {} rows", result.rows_affected);
```

#### DELETE

```rust
// Delete rows
let result = client.data().execute_with_params(
    "DELETE FROM users WHERE status = ?",
    &[Value::String("inactive".to_string())]
).await?;

println!("Deleted {} rows", result.rows_affected);
```

### Query Builder

For complex queries, use the fluent query builder API:

```rust
use q_distributed_db_client::{QueryBuilder, OrderDirection, Value};

let (sql, params) = QueryBuilder::select(&["id", "name", "email"])
    .from("users")
    .where_clause("age > ?", Value::Int(18))
    .and("status = ?", Value::String("active".to_string()))
    .order_by("name", OrderDirection::Asc)
    .limit(10)
    .build()?;

let result = client.data().query_with_params(&sql, &params).await?;
```

### Transactions

Use transactions for atomic operations:

```rust
// Begin a transaction
let mut txn = client.data().begin_transaction().await?;

// Execute operations within the transaction
txn.execute_with_params(
    "UPDATE accounts SET balance = balance - ? WHERE id = ?",
    &[Value::Int(100), Value::Int(1)]
).await?;

txn.execute_with_params(
    "UPDATE accounts SET balance = balance + ? WHERE id = ?",
    &[Value::Int(100), Value::Int(2)]
).await?;

// Commit the transaction
txn.commit().await?;
```

If an error occurs, the transaction will automatically rollback:

```rust
let mut txn = client.data().begin_transaction().await?;

// This will fail and trigger automatic rollback
match txn.execute("INVALID SQL").await {
    Ok(_) => txn.commit().await?,
    Err(e) => {
        println!("Transaction failed: {}", e);
        // Automatic rollback already happened
    }
}
```

### Admin Operations

Manage the cluster and users:

```rust
// List all nodes in the cluster
let nodes = client.admin().list_nodes().await?;
for node in nodes {
    println!("Node {}: {} ({})", node.node_id, node.hostname, node.status);
}

// Create a new user
let user_id = client.admin().create_user(
    "developer",
    "dev_password",
    &[Role::User]
).await?;

// Grant permissions
client.admin().grant_permission(
    user_id,
    Permission::Read
).await?;

// Get cluster metrics
let metrics = client.admin().get_cluster_metrics().await?;
println!("Total queries: {}", metrics.total_queries);
println!("Average latency: {}ms", metrics.average_latency_ms);
```

## Error Handling

The SDK provides comprehensive error types for different failure scenarios:

```rust
use q_distributed_db_client::DatabaseError;

match client.data().query("SELECT * FROM users").await {
    Ok(result) => {
        println!("Query succeeded: {} rows", result.rows.len());
    }
    Err(DatabaseError::ConnectionTimeout { host, timeout_ms }) => {
        eprintln!("Connection to {} timed out after {}ms", host, timeout_ms);
    }
    Err(DatabaseError::AuthenticationFailed { reason }) => {
        eprintln!("Authentication failed: {}", reason);
    }
    Err(DatabaseError::SyntaxError { sql, position, message }) => {
        eprintln!("SQL syntax error at position {}: {}", position, message);
        eprintln!("SQL: {}", sql);
    }
    Err(e) => {
        eprintln!("Query failed: {}", e);
    }
}
```

### Retryable Errors

Some errors are automatically retried by the SDK:

```rust
// These errors will be retried automatically:
// - ConnectionTimeout
// - ConnectionLost
// - NetworkError
// - TimeoutError

// Check if an error is retryable:
if error.is_retryable() {
    println!("This error will be retried automatically");
}
```

## Monitoring and Metrics

Get real-time metrics about SDK operations:

```rust
// Get current metrics
let metrics = client.get_metrics().await;

println!("Query metrics:");
println!("  Total: {}", metrics.query_metrics.total_count);
println!("  Success: {}", metrics.query_metrics.success_count);
println!("  Errors: {}", metrics.query_metrics.error_count);
println!("  Avg latency: {}ms", metrics.query_metrics.avg_latency_ms);
println!("  P95 latency: {}ms", metrics.query_metrics.percentiles.p95);

println!("\nConnection pool:");
println!("  Active: {}", metrics.connection_metrics.active_connections);
println!("  Idle: {}", metrics.connection_metrics.idle_connections);
println!("  Total: {}", metrics.connection_metrics.total_connections);
```

### Health Checks

Monitor cluster health:

```rust
let health = client.health_check().await?;
println!("Cluster health: {}/{} nodes healthy", 
    health.healthy_nodes, health.total_nodes);

for node in health.node_healths {
    println!("  Node {}: {}", 
        node.node_id, 
        if node.is_healthy { "healthy" } else { "unhealthy" }
    );
}
```

## Logging and Tracing

### Enable Logging

Configure structured logging:

```rust
use q_distributed_db_client::{ConnectionConfig, LogConfig, LogLevel, LogFormat};

let config = ConnectionConfig::default()
    .with_hosts(vec!["localhost:7000".to_string()])
    .with_credentials("admin", "password")
    .with_logging(LogConfig::new(LogLevel::Debug)
        .with_format(LogFormat::Json)
        .with_timestamps(true)
        .with_thread_ids(true)
    );

let client = Client::connect(config).await?;
```

### Enable Distributed Tracing

Configure OpenTelemetry tracing:

```rust
use q_distributed_db_client::{ConnectionConfig, TracingConfig};

let config = ConnectionConfig::default()
    .with_hosts(vec!["localhost:7000".to_string()])
    .with_credentials("admin", "password")
    .with_tracing(TracingConfig::new(
        "http://localhost:4317",
        "my-application"
    ));

let client = Client::connect(config).await?;
```

## Best Practices

### Connection Management

1. **Reuse client instances**: Create one client and share it across your application
2. **Configure connection pooling**: Adjust pool size based on your workload
3. **Use health checks**: Monitor cluster health before critical operations
4. **Handle disconnection gracefully**: Always call `disconnect()` when done

### Query Optimization

1. **Use prepared statements**: For repeated queries with different parameters
2. **Use query builder**: Prevents SQL injection and improves readability
3. **Use streaming**: For large result sets to avoid memory issues
4. **Use batch operations**: For multiple operations to reduce round trips

### Error Handling

1. **Check error types**: Use pattern matching to handle specific errors
2. **Implement retry logic**: For transient failures (already built-in)
3. **Log errors**: Use structured logging for debugging
4. **Monitor metrics**: Track error rates and latencies

### Transaction Management

1. **Keep transactions short**: Minimize lock contention
2. **Handle rollback**: Always handle transaction errors properly
3. **Use appropriate isolation**: Choose the right isolation level for your use case
4. **Avoid nested transactions**: Not currently supported

## Troubleshooting

### Connection Issues

**Problem**: `ConnectionTimeout` errors

**Solutions**:
- Verify the database is running: `telnet localhost 7000`
- Check firewall rules allow connections on port 7000
- Increase timeout: `.with_timeout(10000)`
- Check network connectivity between client and server

**Problem**: `ConnectionRefused` errors

**Solutions**:
- Verify the database server is running
- Check the host and port are correct
- Verify no other service is using port 7000

### Authentication Issues

**Problem**: `AuthenticationFailed` errors

**Solutions**:
- Verify username and password are correct
- Check user account exists and is active
- Verify user has necessary permissions
- Check token hasn't expired (24-hour default TTL)

**Problem**: `TokenExpired` errors

**Solutions**:
- The SDK automatically re-authenticates on token expiry
- If you see this error, check your credentials are still valid
- Verify the authentication server is reachable

### Query Issues

**Problem**: `SyntaxError` in SQL

**Solutions**:
- Check SQL syntax is correct
- Use query builder to avoid syntax errors
- Verify table and column names exist
- Check parameter placeholders match parameter count

**Problem**: `TableNotFound` or `ColumnNotFound` errors

**Solutions**:
- Verify the table/column exists in the database
- Check spelling and case sensitivity
- Ensure you're connected to the correct database

### Performance Issues

**Problem**: Slow query performance

**Solutions**:
- Check query metrics: `client.get_metrics().await`
- Use indexes on frequently queried columns
- Use LIMIT clauses for large result sets
- Consider using streaming for very large results
- Monitor cluster health: `client.health_check().await`

**Problem**: Connection pool exhaustion

**Solutions**:
- Increase max_connections in PoolConfig
- Ensure connections are returned to pool (automatic)
- Check for connection leaks in your code
- Monitor connection metrics

## Next Steps

- Explore the [API documentation](https://docs.rs/q-distributed-db-client)
- Check out the [examples](../examples/) directory
- Read about [transactions](../examples/transactions.rs)
- Learn about [admin operations](../examples/admin_operations.rs)

## Getting Help

- Check the [API documentation](https://docs.rs/q-distributed-db-client)
- Review the [examples](../examples/)
- File an issue on GitHub
- Contact the development team

## Additional Resources

- [Architecture Overview](./architecture.md)
- [Protocol Specification](./protocol.md)
- [Performance Tuning Guide](./performance.md)
- [Security Best Practices](./security.md)
