//! Transaction Example
//!
//! This example demonstrates how to use transactions for atomic operations,
//! including both successful commits and rollback scenarios.
//!
//! Run with: cargo run --example transactions

use q_distributed_db_client::{Client, ConnectionConfig, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Q-Distributed-Database Transaction Example ===\n");

    // Connect to database
    println!("Connecting to database...");
    let config = ConnectionConfig::default()
        .with_hosts(vec!["localhost:7000".to_string()])
        .with_credentials("admin", "password");

    let client = Client::connect(config).await?;
    println!("✓ Connected\n");

    // Setup: Create accounts table
    println!("Setting up accounts table...");
    client
        .data()
        .execute("CREATE TABLE accounts (id INT PRIMARY KEY, name TEXT, balance INT)")
        .await?;

    // Insert initial account data
    client
        .data()
        .execute_with_params(
            "INSERT INTO accounts (id, name, balance) VALUES (?, ?, ?)",
            &[
                Value::Int(1),
                Value::String("Alice".to_string()),
                Value::Int(1000),
            ],
        )
        .await?;

    client
        .data()
        .execute_with_params(
            "INSERT INTO accounts (id, name, balance) VALUES (?, ?, ?)",
            &[
                Value::Int(2),
                Value::String("Bob".to_string()),
                Value::Int(500),
            ],
        )
        .await?;

    println!("✓ Setup complete\n");

    // Display initial balances
    println!("Initial balances:");
    print_balances(&client).await?;
    println!();

    // ========================================================================
    // Example 1: Successful Transaction (Commit)
    // ========================================================================
    println!("--- Example 1: Transfer $200 from Alice to Bob ---\n");

    {
        // Begin a new transaction
        println!("Beginning transaction...");
        let mut txn = client.data().begin_transaction().await?;
        println!("✓ Transaction started (ID: {})", txn.transaction_id());

        // Deduct from Alice's account
        println!("  Deducting $200 from Alice...");
        txn.execute_with_params(
            "UPDATE accounts SET balance = balance - ? WHERE id = ?",
            &[Value::Int(200), Value::Int(1)],
        )
        .await?;
        println!("  ✓ Deducted");

        // Add to Bob's account
        println!("  Adding $200 to Bob...");
        txn.execute_with_params(
            "UPDATE accounts SET balance = balance + ? WHERE id = ?",
            &[Value::Int(200), Value::Int(2)],
        )
        .await?;
        println!("  ✓ Added");

        // Commit the transaction
        println!("  Committing transaction...");
        txn.commit().await?;
        println!("✓ Transaction committed successfully\n");
    }

    // Display balances after successful transaction
    println!("Balances after transfer:");
    print_balances(&client).await?;
    println!();

    // ========================================================================
    // Example 2: Failed Transaction (Manual Rollback)
    // ========================================================================
    println!("--- Example 2: Attempt invalid transfer (manual rollback) ---\n");

    {
        println!("Beginning transaction...");
        let mut txn = client.data().begin_transaction().await?;
        println!("✓ Transaction started (ID: {})", txn.transaction_id());

        // Try to deduct more than Alice has (this would succeed in the transaction)
        println!("  Attempting to deduct $2000 from Alice (more than balance)...");
        txn.execute_with_params(
            "UPDATE accounts SET balance = balance - ? WHERE id = ?",
            &[Value::Int(2000), Value::Int(1)],
        )
        .await?;
        println!("  ✓ Operation executed (but not committed)");

        // Check Alice's balance within the transaction
        let result = txn
            .query_with_params("SELECT balance FROM accounts WHERE id = ?", &[Value::Int(1)])
            .await?;
        
        if let Some(row) = result.rows.first() {
            let balance = row.get_i64(0)?;
            println!("  Alice's balance in transaction: ${}", balance);
            
            // Decide to rollback because balance is negative
            if balance < 0 {
                println!("  ⚠ Balance is negative, rolling back transaction...");
                txn.rollback().await?;
                println!("✓ Transaction rolled back\n");
            }
        }
    }

    // Display balances after rollback
    println!("Balances after rollback (unchanged):");
    print_balances(&client).await?;
    println!();

    // ========================================================================
    // Example 3: Automatic Rollback on Error
    // ========================================================================
    println!("--- Example 3: Automatic rollback on error ---\n");

    {
        println!("Beginning transaction...");
        let mut txn = client.data().begin_transaction().await?;
        println!("✓ Transaction started (ID: {})", txn.transaction_id());

        // Execute a valid operation
        println!("  Deducting $50 from Alice...");
        txn.execute_with_params(
            "UPDATE accounts SET balance = balance - ? WHERE id = ?",
            &[Value::Int(50), Value::Int(1)],
        )
        .await?;
        println!("  ✓ Deducted");

        // Execute an invalid operation (syntax error)
        println!("  Attempting invalid SQL...");
        match txn.execute("INVALID SQL STATEMENT").await {
            Ok(_) => {
                println!("  Unexpected success");
            }
            Err(e) => {
                println!("  ✗ Error occurred: {}", e);
                println!("  ✓ Transaction automatically rolled back");
            }
        }
        
        // Transaction is now rolled back, cannot commit
        println!();
    }

    // Display balances after automatic rollback
    println!("Balances after automatic rollback (unchanged):");
    print_balances(&client).await?;
    println!();

    // ========================================================================
    // Example 4: Multiple Operations in Transaction
    // ========================================================================
    println!("--- Example 4: Multiple operations in one transaction ---\n");

    {
        println!("Beginning transaction...");
        let mut txn = client.data().begin_transaction().await?;
        println!("✓ Transaction started (ID: {})", txn.transaction_id());

        // Transfer $100 from Alice to Bob
        println!("  Transferring $100 from Alice to Bob...");
        txn.execute_with_params(
            "UPDATE accounts SET balance = balance - ? WHERE id = ?",
            &[Value::Int(100), Value::Int(1)],
        )
        .await?;
        
        txn.execute_with_params(
            "UPDATE accounts SET balance = balance + ? WHERE id = ?",
            &[Value::Int(100), Value::Int(2)],
        )
        .await?;
        println!("  ✓ Transfer complete");

        // Query balances within transaction
        println!("  Checking balances within transaction...");
        let result = txn.query("SELECT name, balance FROM accounts ORDER BY id").await?;
        for row in &result.rows {
            let name = row.get_string(0)?;
            let balance = row.get_i64(1)?;
            println!("    {}: ${}", name, balance);
        }

        // Commit all operations atomically
        println!("  Committing transaction...");
        txn.commit().await?;
        println!("✓ All operations committed atomically\n");
    }

    // Display final balances
    println!("Final balances:");
    print_balances(&client).await?;
    println!();

    // Cleanup
    println!("Cleaning up...");
    client.data().execute("DROP TABLE accounts").await?;
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
