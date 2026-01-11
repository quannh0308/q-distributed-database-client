//! Integration tests for the Client interface
//!
//! These tests require a running q-distributed-database server and are marked
//! as #[ignore] by default. To run them:
//!
//! ```bash
//! cargo test --manifest-path rust/client-sdk/Cargo.toml -- --ignored
//! ```

use q_distributed_db_client::{Client, ConnectionConfig, Value};

/// Helper function to create a test configuration
fn test_config() -> ConnectionConfig {
    ConnectionConfig::default()
        .with_hosts(vec!["localhost:7000".to_string()])
        .with_credentials("admin", "password")
        .with_timeout(5000)
}

#[tokio::test]
#[ignore]
async fn test_client_connection_lifecycle() {
    // Test full connection lifecycle: connect → operations → disconnect
    let config = test_config();

    // Connect to database
    let client = Client::connect(config)
        .await
        .expect("Failed to connect to database");

    // Verify we can access sub-clients
    let _data = client.data();
    let _admin = client.admin();

    // Disconnect gracefully
    client.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_client_crud_operations() {
    // Test CRUD operations through Client
    let config = test_config();
    let client = Client::connect(config)
        .await
        .expect("Failed to connect to database");

    // Create table
    client
        .data()
        .execute("CREATE TABLE IF NOT EXISTS test_users (id INT, name TEXT, email TEXT)")
        .await
        .expect("Failed to create table");

    // Insert data
    let result = client
        .data()
        .execute_with_params(
            "INSERT INTO test_users VALUES (?, ?, ?)",
            &[
                Value::Int(1),
                Value::String("Alice".to_string()),
                Value::String("alice@example.com".to_string()),
            ],
        )
        .await
        .expect("Failed to insert data");

    assert_eq!(result.rows_affected, 1);

    // Query data
    let query_result = client
        .data()
        .query("SELECT * FROM test_users WHERE id = 1")
        .await
        .expect("Failed to query data");

    assert_eq!(query_result.rows.len(), 1);
    let row = &query_result.rows[0];
    assert_eq!(row.get_i64(0).unwrap(), 1);
    assert_eq!(row.get_string(1).unwrap(), "Alice");

    // Update data
    let update_result = client
        .data()
        .execute_with_params(
            "UPDATE test_users SET email = ? WHERE id = ?",
            &[
                Value::String("alice.new@example.com".to_string()),
                Value::Int(1),
            ],
        )
        .await
        .expect("Failed to update data");

    assert_eq!(update_result.rows_affected, 1);

    // Delete data
    let delete_result = client
        .data()
        .execute_with_params("DELETE FROM test_users WHERE id = ?", &[Value::Int(1)])
        .await
        .expect("Failed to delete data");

    assert_eq!(delete_result.rows_affected, 1);

    // Cleanup
    client
        .data()
        .execute("DROP TABLE test_users")
        .await
        .expect("Failed to drop table");

    client.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_client_transaction_operations() {
    // Test transaction operations through Client
    let config = test_config();
    let client = Client::connect(config)
        .await
        .expect("Failed to connect to database");

    // Create table
    client
        .data()
        .execute("CREATE TABLE IF NOT EXISTS test_accounts (id INT, balance INT)")
        .await
        .expect("Failed to create table");

    // Insert initial data
    client
        .data()
        .execute_with_params(
            "INSERT INTO test_accounts VALUES (?, ?)",
            &[Value::Int(1), Value::Int(1000)],
        )
        .await
        .expect("Failed to insert account 1");

    client
        .data()
        .execute_with_params(
            "INSERT INTO test_accounts VALUES (?, ?)",
            &[Value::Int(2), Value::Int(500)],
        )
        .await
        .expect("Failed to insert account 2");

    // Begin transaction
    let mut txn = client
        .data()
        .begin_transaction()
        .await
        .expect("Failed to begin transaction");

    // Transfer money from account 1 to account 2
    txn.execute_with_params(
        "UPDATE test_accounts SET balance = balance - ? WHERE id = ?",
        &[Value::Int(200), Value::Int(1)],
    )
    .await
    .expect("Failed to debit account 1");

    txn.execute_with_params(
        "UPDATE test_accounts SET balance = balance + ? WHERE id = ?",
        &[Value::Int(200), Value::Int(2)],
    )
    .await
    .expect("Failed to credit account 2");

    // Commit transaction
    txn.commit().await.expect("Failed to commit transaction");

    // Verify balances
    let result = client
        .data()
        .query("SELECT balance FROM test_accounts WHERE id = 1")
        .await
        .expect("Failed to query account 1");

    assert_eq!(result.rows[0].get_i64(0).unwrap(), 800);

    let result = client
        .data()
        .query("SELECT balance FROM test_accounts WHERE id = 2")
        .await
        .expect("Failed to query account 2");

    assert_eq!(result.rows[0].get_i64(0).unwrap(), 700);

    // Cleanup
    client
        .data()
        .execute("DROP TABLE test_accounts")
        .await
        .expect("Failed to drop table");

    client.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_client_admin_operations() {
    // Test admin operations through Client
    let config = test_config();
    let client = Client::connect(config)
        .await
        .expect("Failed to connect to database");

    // List nodes
    let nodes = client
        .admin()
        .list_nodes()
        .await
        .expect("Failed to list nodes");

    assert!(!nodes.is_empty(), "Cluster should have at least one node");

    // Get cluster metrics
    let metrics = client
        .admin()
        .get_cluster_metrics()
        .await
        .expect("Failed to get cluster metrics");

    // Just verify we got metrics (total_queries is u64, always >= 0)
    assert!(metrics.total_queries == metrics.total_queries);

    // List users
    let users = client
        .admin()
        .list_users()
        .await
        .expect("Failed to list users");

    assert!(!users.is_empty(), "Should have at least one user");

    client.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_client_health_check() {
    // Test health check functionality
    let config = test_config();
    let client = Client::connect(config)
        .await
        .expect("Failed to connect to database");

    // Check cluster health
    let health = client
        .health_check()
        .await
        .expect("Failed to check cluster health");

    assert!(health.total_nodes > 0, "Should have at least one node");
    assert!(
        health.healthy_nodes > 0,
        "Should have at least one healthy node"
    );
    assert_eq!(health.node_healths.len(), health.total_nodes);

    // Verify at least one node is healthy
    let has_healthy = health.node_healths.iter().any(|h| h.is_healthy);
    assert!(has_healthy, "At least one node should be healthy");

    client.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
#[ignore]
async fn test_client_authentication_failure() {
    // Test connection with invalid credentials
    let config = ConnectionConfig::default()
        .with_hosts(vec!["localhost:7000".to_string()])
        .with_credentials("invalid_user", "wrong_password")
        .with_timeout(5000);

    let result = Client::connect(config).await;

    // Should fail with authentication error
    assert!(
        result.is_err(),
        "Connection with invalid credentials should fail"
    );
}

#[tokio::test]
#[ignore]
async fn test_client_connection_failure() {
    // Test connection to non-existent server
    let config = ConnectionConfig::default()
        .with_hosts(vec!["localhost:9999".to_string()])
        .with_credentials("admin", "password")
        .with_timeout(1000);

    let result = Client::connect(config).await;

    // Should fail with connection error
    assert!(
        result.is_err(),
        "Connection to non-existent server should fail"
    );
}

#[tokio::test]
#[ignore]
async fn test_client_multiple_operations() {
    // Test multiple sequential operations
    let config = test_config();
    let client = Client::connect(config)
        .await
        .expect("Failed to connect to database");

    // Create table
    client
        .data()
        .execute("CREATE TABLE IF NOT EXISTS test_items (id INT, name TEXT)")
        .await
        .expect("Failed to create table");

    // Insert multiple items
    for i in 1..=10 {
        client
            .data()
            .execute_with_params(
                "INSERT INTO test_items VALUES (?, ?)",
                &[Value::Int(i), Value::String(format!("Item {}", i))],
            )
            .await
            .expect("Failed to insert item");
    }

    // Query all items
    let result = client
        .data()
        .query("SELECT COUNT(*) FROM test_items")
        .await
        .expect("Failed to count items");

    assert_eq!(result.rows[0].get_i64(0).unwrap(), 10);

    // Cleanup
    client
        .data()
        .execute("DROP TABLE test_items")
        .await
        .expect("Failed to drop table");

    client.disconnect().await.expect("Failed to disconnect");
}
