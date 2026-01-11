//! Admin Operations Example
//!
//! This example demonstrates cluster and user management operations
//! using the AdminClient.
//!
//! Run with: cargo run --example admin_operations

use q_distributed_db_client::{Client, ConnectionConfig, Permission, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Q-Distributed-Database Admin Operations Example ===\n");

    // Connect to database with admin credentials
    println!("Connecting to database as admin...");
    let config = ConnectionConfig::default()
        .with_hosts(vec!["localhost:7000".to_string()])
        .with_credentials("admin", "admin_password");

    let client = Client::connect(config).await?;
    println!("✓ Connected\n");

    // ========================================================================
    // Cluster Management
    // ========================================================================
    println!("=== CLUSTER MANAGEMENT ===\n");

    // List all nodes in the cluster
    println!("1. Listing cluster nodes...");
    let nodes = client.admin().list_nodes().await?;
    println!("   Found {} nodes:", nodes.len());

    for node in &nodes {
        println!(
            "     - Node {}: {}:{} ({:?}, {:?})",
            node.node_id, node.hostname, node.port, node.status, node.role
        );
    }
    println!();

    // Get health metrics for each node
    println!("2. Checking node health metrics...");
    for node in &nodes {
        match client.admin().get_node_health(node.node_id).await {
            Ok(health) => {
                println!("   Node {} health:", node.node_id);
                println!("     CPU usage: {:.1}%", health.cpu_usage);
                println!("     Memory usage: {:.1}%", health.memory_usage);
                println!("     Disk usage: {:.1}%", health.disk_usage);
                println!("     Active connections: {}", health.connection_count);
                println!("     Query throughput: {:.1} qps", health.query_throughput);
            }
            Err(e) => {
                eprintln!("   Failed to get health for node {}: {}", node.node_id, e);
            }
        }
    }
    println!();

    // Get cluster-wide metrics
    println!("3. Getting cluster metrics...");
    let metrics = client.admin().get_cluster_metrics().await?;
    println!("   Cluster metrics:");
    println!("     Total queries: {}", metrics.total_queries);
    println!("     Average latency: {:.2}ms", metrics.average_latency_ms);
    println!("     Error rate: {:.4}%", metrics.error_rate * 100.0);
    println!(
        "     Storage usage: {:.2} GB",
        metrics.storage_usage_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    );
    println!();

    // ========================================================================
    // User Management
    // ========================================================================
    println!("=== USER MANAGEMENT ===\n");

    // List existing users
    println!("4. Listing existing users...");
    let users = client.admin().list_users().await?;
    println!("   Found {} users:", users.len());

    for user in &users {
        println!(
            "     - {} (ID: {}): {:?}",
            user.username, user.user_id, user.roles
        );
    }
    println!();

    // Create a new user
    println!("5. Creating new user 'developer'...");
    let user_id = client
        .admin()
        .create_user("developer", "dev_password", &[Role::User])
        .await?;
    println!("   ✓ User created with ID: {}\n", user_id);

    // Grant read permission
    println!("6. Granting read permission to 'developer'...");
    client
        .admin()
        .grant_permission(user_id, Permission::Read)
        .await?;
    println!("   ✓ Read permission granted\n");

    // Grant write permission
    println!("7. Granting write permission to 'developer'...");
    client
        .admin()
        .grant_permission(user_id, Permission::Write)
        .await?;
    println!("   ✓ Write permission granted\n");

    // List users again to see the new user
    println!("8. Listing users after creation...");
    let users = client.admin().list_users().await?;
    println!("   Found {} users:", users.len());

    for user in &users {
        println!(
            "     - {} (ID: {}): {:?}, Permissions: {:?}",
            user.username, user.user_id, user.roles, user.permissions
        );
    }
    println!();

    // Update user (add admin role)
    println!("9. Updating user 'developer' to add ReadOnly role...");
    let update = q_distributed_db_client::types::UserUpdate {
        password: None,
        roles: Some(vec![Role::User, Role::ReadOnly]),
    };

    client.admin().update_user(user_id, update).await?;
    println!("   ✓ User updated\n");

    // Verify the update
    println!("10. Verifying user update...");
    let users = client.admin().list_users().await?;
    if let Some(user) = users.iter().find(|u| u.user_id == user_id) {
        println!(
            "   User '{}' now has roles: {:?}\n",
            user.username, user.roles
        );
    }

    // Revoke a permission
    println!("11. Revoking write permission from 'developer'...");
    client
        .admin()
        .revoke_permission(user_id, Permission::Write)
        .await?;
    println!("   ✓ Write permission revoked\n");

    // Delete the test user
    println!("12. Deleting test user...");
    client.admin().delete_user(user_id).await?;
    println!("   ✓ User deleted\n");

    // Verify deletion
    println!("13. Verifying user deletion...");
    let users = client.admin().list_users().await?;
    let user_exists = users.iter().any(|u| u.user_id == user_id);

    if user_exists {
        println!("   ⚠ User still exists (unexpected)");
    } else {
        println!("   ✓ User successfully deleted\n");
    }

    // ========================================================================
    // Advanced: Cluster Operations
    // ========================================================================
    println!("=== ADVANCED CLUSTER OPERATIONS ===\n");

    // Note: These operations require appropriate cluster setup and permissions

    println!("14. Demonstrating cluster operations...");
    println!("   Note: The following operations require a multi-node cluster\n");

    // Example: Add a new node (commented out - requires actual node)
    println!("   Example: Adding a new node");
    println!("     client.admin().add_node(\"node4.example.com:7000\").await?;");
    println!();

    // Example: Remove a node (commented out - requires actual node)
    println!("   Example: Removing a node");
    println!("     client.admin().remove_node(node_id).await?;");
    println!();

    // Example: Rebalance partitions (commented out - requires multi-node cluster)
    println!("   Example: Rebalancing partitions");
    println!("     client.admin().rebalance_partitions().await?;");
    println!();

    // Disconnect
    println!("Disconnecting...");
    client.disconnect().await?;
    println!("✓ Disconnected\n");

    println!("=== Example completed successfully! ===");
    println!("\nKey Takeaways:");
    println!("  • Connection pooling enables efficient concurrent operations");
    println!("  • The SDK automatically manages connection lifecycle");
    println!("  • Admin operations require appropriate permissions");
    println!("  • Cluster operations work seamlessly across multiple nodes");

    Ok(())
}
