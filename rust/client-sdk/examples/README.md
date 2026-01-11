# Q-Distributed-Database Client SDK Examples

This directory contains example applications demonstrating how to use the Q-Distributed-Database Client SDK.

## Prerequisites

Before running these examples, ensure you have:
1. A running q-distributed-database server on `localhost:7000`
2. Valid admin credentials (username: `admin`, password: `password`)
3. Rust and Cargo installed

## Running Examples

Each example can be run using cargo:

```bash
# From the rust/client-sdk directory
cargo run --example basic_crud
cargo run --example transactions
cargo run --example connection_pooling
cargo run --example admin_operations
```

## Examples Overview

### basic_crud.rs

Demonstrates fundamental CRUD (Create, Read, Update, Delete) operations:
- Creating tables
- Inserting data with parameters
- Querying data with and without filters
- Updating records
- Deleting records
- Proper cleanup

**Key concepts**: Basic SQL operations, parameterized queries, result handling

### transactions.rs

Shows how to use transactions for atomic operations:
- Beginning transactions
- Executing multiple operations atomically
- Committing successful transactions
- Rolling back failed transactions
- Automatic rollback on errors

**Key concepts**: ACID transactions, commit/rollback, error handling

### connection_pooling.rs

Demonstrates connection pool configuration and behavior:
- Configuring pool size and timeouts
- Running concurrent operations
- Connection reuse
- Monitoring pool metrics
- Health checks

**Key concepts**: Connection pooling, concurrency, performance monitoring

### admin_operations.rs

Shows administrative operations for cluster and user management:
- Listing cluster nodes
- Checking node health
- Getting cluster metrics
- Creating and managing users
- Granting and revoking permissions
- User CRUD operations

**Key concepts**: Cluster management, user management, permissions

## Modifying Examples

Feel free to modify these examples to experiment with different scenarios:

- Change connection parameters in `ConnectionConfig`
- Try different SQL queries
- Experiment with error handling
- Add your own operations

## Troubleshooting

If examples fail to run:

1. **Connection errors**: Verify the database server is running on `localhost:7000`
2. **Authentication errors**: Check your credentials are correct
3. **Permission errors**: Ensure you're using an admin account for admin operations
4. **Compilation errors**: Run `cargo build` to see detailed error messages

## Next Steps

After running these examples:
- Read the [Getting Started Guide](../../../docs/getting-started.md)
- Explore the [API documentation](https://docs.rs/q-distributed-db-client)
- Build your own application using the SDK

## Additional Resources

- [SDK Documentation](../README.md)
- [Architecture Overview](../../../docs/architecture.md)
- [API Reference](https://docs.rs/q-distributed-db-client)
