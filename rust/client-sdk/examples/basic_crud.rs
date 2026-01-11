//! Basic CRUD Operations Example
//!
//! This example demonstrates how to perform basic CRUD operations
//! (Create, Read, Update, Delete) using the q-distributed-database client SDK.
//!
//! Run with: cargo run --example basic_crud

use q_distributed_db_client::{Client, ConnectionConfig, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Q-Distributed-Database Basic CRUD Example ===\n");

    // Step 1: Connect to database
    println!("1. Connecting to database...");
    let config = ConnectionConfig::default()
        .with_hosts(vec!["localhost:7000".to_string()])
        .with_credentials("admin", "password");

    let client = Client::connect(config).await?;
    println!("   ✓ Connected successfully\n");

    // Step 2: CREATE - Create a table
    println!("2. Creating 'users' table...");
    client
        .data()
        .execute("CREATE TABLE users (id INT PRIMARY KEY, name TEXT, email TEXT, age INT)")
        .await?;
    println!("   ✓ Table created\n");

    // Step 3: INSERT - Add records
    println!("3. Inserting users...");
    
    client
        .data()
        .execute_with_params(
            "INSERT INTO users (id, name, email, age) VALUES (?, ?, ?, ?)",
            &[
                Value::Int(1),
                Value::String("Alice".to_string()),
                Value::String("alice@example.com".to_string()),
                Value::Int(30),
            ],
        )
        .await?;
    println!("   ✓ Inserted user: Alice");

    client
        .data()
        .execute_with_params(
            "INSERT INTO users (id, name, email, age) VALUES (?, ?, ?, ?)",
            &[
                Value::Int(2),
                Value::String("Bob".to_string()),
                Value::String("bob@example.com".to_string()),
                Value::Int(25),
            ],
        )
        .await?;
    println!("   ✓ Inserted user: Bob");

    client
        .data()
        .execute_with_params(
            "INSERT INTO users (id, name, email, age) VALUES (?, ?, ?, ?)",
            &[
                Value::Int(3),
                Value::String("Charlie".to_string()),
                Value::String("charlie@example.com".to_string()),
                Value::Int(35),
            ],
        )
        .await?;
    println!("   ✓ Inserted user: Charlie\n");

    // Step 4: READ - Query records
    println!("4. Querying all users...");
    let result = client.data().query("SELECT * FROM users ORDER BY id").await?;
    println!("   Found {} users:", result.rows.len());
    
    for row in &result.rows {
        let id = row.get_i64(0)?;
        let name = row.get_string(1)?;
        let email = row.get_string(2)?;
        let age = row.get_i64(3)?;
        println!("     - ID: {}, Name: {}, Email: {}, Age: {}", id, name, email, age);
    }
    println!();

    // Step 5: READ with filtering - Query specific records
    println!("5. Querying users older than 28...");
    let result = client
        .data()
        .query_with_params(
            "SELECT * FROM users WHERE age > ? ORDER BY age",
            &[Value::Int(28)],
        )
        .await?;
    println!("   Found {} users:", result.rows.len());
    
    for row in &result.rows {
        let name = row.get_string(1)?;
        let age = row.get_i64(3)?;
        println!("     - {}: {} years old", name, age);
    }
    println!();

    // Step 6: UPDATE - Modify a record
    println!("6. Updating Alice's email...");
    let update_result = client
        .data()
        .execute_with_params(
            "UPDATE users SET email = ? WHERE id = ?",
            &[
                Value::String("alice.new@example.com".to_string()),
                Value::Int(1),
            ],
        )
        .await?;
    println!("   ✓ Updated {} row(s)", update_result.rows_affected);

    // Verify the update
    let result = client
        .data()
        .query_with_params("SELECT name, email FROM users WHERE id = ?", &[Value::Int(1)])
        .await?;
    
    if let Some(row) = result.rows.first() {
        let name = row.get_string(0)?;
        let email = row.get_string(1)?;
        println!("   Verified: {}'s new email is {}\n", name, email);
    }

    // Step 7: DELETE - Remove a record
    println!("7. Deleting user Bob...");
    let delete_result = client
        .data()
        .execute_with_params("DELETE FROM users WHERE id = ?", &[Value::Int(2)])
        .await?;
    println!("   ✓ Deleted {} row(s)", delete_result.rows_affected);

    // Verify the deletion
    let result = client.data().query("SELECT COUNT(*) FROM users").await?;
    if let Some(row) = result.rows.first() {
        let count = row.get_i64(0)?;
        println!("   Remaining users: {}\n", count);
    }

    // Step 8: Cleanup - Drop the table
    println!("8. Cleaning up...");
    client.data().execute("DROP TABLE users").await?;
    println!("   ✓ Table dropped");

    // Step 9: Disconnect
    println!("\n9. Disconnecting...");
    client.disconnect().await?;
    println!("   ✓ Disconnected\n");

    println!("=== Example completed successfully! ===");
    Ok(())
}
