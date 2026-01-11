//! Connection Pooling Example
//!
//! This example demonstrates connection pool configuration and concurrent
//! operations using the connection pool.
//!
//! Run with: cargo run --example connection_pooling

use q_distributed_db_client::{Client, ConnectionConfig, PoolConfig, Value};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Q-Distributed-Database Connection Pooling Example ===\n");

    // Step 1: Configure connection pool
    println!("1. Configuring connection pool...");
    let pool_config = PoolConfig {
        min_connections: 5,
        max_connections: 20,
        connection_timeout_ms: 5000,
        idle_timeout_ms: 60000,      // 1 minute
        max_lifetime_ms: 1800000,    // 30 minutes
    };

    let config = ConnectionConfig::default()
        .with_hosts(vec!["localhost:7000".to_string()])
        .with_credentials("admin", "password")
        .with_pool_config(pool_config);

    println!("   Pool configuration:");
    println!("     Min connections: {}", config.pool_config.min_connections);
    println!("     Max connections: {}", config.pool_config.max_connections);
    println!("     Connection timeout: {}ms", config.pool_config.connection_timeout_ms);
    println!("     Idle timeout: {}ms", config.pool_config.idle_timeout_ms);
    println!("     Max lifetime: {}ms\n", config.pool_config.max_lifetime_ms);

    // Step 2: Connect to database
    println!("2. Connecting to database...");
    let client = Client::connect(config).await?;
    println!("   ✓ Connected with connection pool\n");

    // Step 3: Setup test table
    println!("3. Setting up test table...");
    client
        .data()
        .execute("CREATE TABLE test_data (id INT PRIMARY KEY, value TEXT)")
        .await?;
    println!("   ✓ Table created\n");

    // Step 4: Demonstrate concurrent operations
    println!("4. Running 20 concurrent queries...");
    let start = Instant::now();
    
    let mut handles = vec![];

    for i in 0..20 {
        // Clone the client (shares the connection pool)
        let client_clone = client.clone();
        
        // Spawn a task for each query
        let handle = tokio::spawn(async move {
            let query_start = Instant::now();
            
            // Execute a simple query
            let result = client_clone
                .data()
                .query("SELECT 1 as value")
                .await;
            
            let elapsed = query_start.elapsed();
            
            match result {
                Ok(_) => {
                    println!("   Query {} completed in {:?}", i + 1, elapsed);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("   Query {} failed: {}", i + 1, e);
                    Err(e)
                }
            }
        });
        
        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut success_count = 0;
    let mut error_count = 0;
    
    for handle in handles {
        match handle.await? {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    let total_elapsed = start.elapsed();
    println!("\n   Results:");
    println!("     Total time: {:?}", total_elapsed);
    println!("     Successful: {}", success_count);
    println!("     Failed: {}", error_count);
    println!("     Average time per query: {:?}\n", total_elapsed / 20);

    // Step 5: Check connection pool metrics
    println!("5. Checking connection pool metrics...");
    let metrics = client.get_metrics().await;
    println!("   Connection pool:");
    println!("     Active connections: {}", metrics.connection_metrics.active_connections);
    println!("     Idle connections: {}", metrics.connection_metrics.idle_connections);
    println!("     Total connections: {}", metrics.connection_metrics.total_connections);
    println!("     Connection errors: {}", metrics.connection_metrics.connection_errors);
    println!("     Connection timeouts: {}\n", metrics.connection_metrics.connection_timeouts);

    // Step 6: Check cluster health
    println!("6. Checking cluster health...");
    let health = client.health_check().await?;
    println!("   Cluster status: {}/{} nodes healthy", 
        health.healthy_nodes, health.total_nodes);
    
    for node_health in &health.node_healths {
        println!("     Node {}: {}", 
            node_health.node_id,
            if node_health.is_healthy { "✓ healthy" } else { "✗ unhealthy" }
        );
    }
    println!();

    // Step 7: Demonstrate connection reuse
    println!("7. Demonstrating connection reuse...");
    println!("   Executing 5 sequential queries (should reuse connections)...");
    
    for i in 1..=5 {
        let start = Instant::now();
        client.data().query("SELECT 1").await?;
        let elapsed = start.elapsed();
        println!("     Query {}: {:?}", i, elapsed);
    }
    println!();

    // Step 8: Check query metrics
    println!("8. Checking query metrics...");
    let metrics = client.get_metrics().await;
    println!("   Query metrics:");
    println!("     Total queries: {}", metrics.query_metrics.total_count);
    println!("     Successful: {}", metrics.query_metrics.success_count);
    println!("     Failed: {}", metrics.query_metrics.error_count);
    println!("     Min latency: {:.2}ms", metrics.query_metrics.min_latency_ms);
    println!("     Max latency: {:.2}ms", metrics.query_metrics.max_latency_ms);
    println!("     Avg latency: {:.2}ms", metrics.query_metrics.avg_latency_ms);
    println!("     P50 latency: {:.2}ms", metrics.query_metrics.percentiles.p50);
    println!("     P95 latency: {:.2}ms", metrics.query_metrics.percentiles.p95);
    println!("     P99 latency: {:.2}ms\n", metrics.query_metrics.percentiles.p99);

    // Cleanup
    println!("Cleaning up...");
    client.data().execute("DROP TABLE test_data").await?;
    println!("✓ Table dropped");

    client.disconnect().await?;
    println!("✓ Disconnected\n");

    println!("=== Example completed successfully! ===");
    Ok(())
}

/// Helper function to print account balances
async fn print_balances(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let result = client
        .data()
        .query("SELECT name, balance FROM accounts ORDER BY id")
        .await?;

    for row in &result.rows {
        let name = row.get_string(0)?;
        let balance = row.get_i64(1)?;
        println!("  {}: ${}", name, balance);
    }

    Ok(())
}
