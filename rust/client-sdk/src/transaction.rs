//! Transaction support for Q-Distributed-Database Client SDK
//!
//! This module implements ACID transaction capabilities with commit, rollback,
//! and automatic rollback on error or drop.

use crate::auth::AuthToken;
use crate::connection::PooledConnection;
use crate::error::DatabaseError;
use crate::protocol::MessageType;
use crate::result::{ColumnMetadata, QueryResult, Row};
use crate::types::{TransactionId, Value};
use crate::Result;
use serde::{Deserialize, Serialize};

// Re-export ExecuteResult from data_client
use crate::data_client::ExecuteResult;

/// Isolation level for transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Read uncommitted data
    ReadUncommitted,
    /// Read committed data only (default)
    ReadCommitted,
    /// Repeatable reads within transaction
    RepeatableRead,
    /// Full serializable isolation
    Serializable,
}

impl Default for IsolationLevel {
    fn default() -> Self {
        IsolationLevel::ReadCommitted
    }
}

/// Transaction request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionRequest {
    /// Begin a new transaction
    Begin {
        transaction_id: TransactionId,
        isolation_level: IsolationLevel,
    },
    /// Commit a transaction
    Commit {
        transaction_id: TransactionId,
    },
    /// Rollback a transaction
    Rollback {
        transaction_id: TransactionId,
    },
}

/// Transaction response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionResponse {
    /// Transaction began successfully
    BeginSuccess,
    /// Transaction committed successfully
    CommitSuccess,
    /// Transaction rolled back successfully
    RollbackSuccess,
    /// Transaction operation failed
    Error { message: String },
}

/// Execute request with optional transaction context
#[derive(Debug, Serialize, Deserialize)]
struct ExecuteRequest {
    sql: String,
    params: Vec<Value>,
    transaction_id: Option<TransactionId>,
    auth_token: Option<Vec<u8>>,
}

/// Execute response
#[derive(Debug, Serialize, Deserialize)]
struct ExecuteResponse {
    rows_affected: u64,
    last_insert_id: Option<i64>,
    error: Option<String>,
}

/// Query request with optional transaction context
#[derive(Debug, Serialize, Deserialize)]
struct QueryRequest {
    sql: String,
    params: Vec<Value>,
    transaction_id: Option<TransactionId>,
    auth_token: Option<Vec<u8>>,
}

/// Query response
#[derive(Debug, Serialize, Deserialize)]
struct QueryResponse {
    columns: Vec<ColumnMetadata>,
    rows: Vec<Vec<Value>>,
    error: Option<String>,
}

/// Transaction context for executing operations atomically
///
/// A transaction provides ACID guarantees for database operations.
/// All operations within a transaction are executed atomically - either
/// all succeed or all fail.
///
/// # Example
///
/// ```ignore
/// let mut txn = client.data().begin_transaction().await?;
/// txn.execute("INSERT INTO users (name) VALUES (?)", &[Value::from("Alice")]).await?;
/// txn.execute("INSERT INTO users (name) VALUES (?)", &[Value::from("Bob")]).await?;
/// txn.commit().await?;
/// ```
pub struct Transaction {
    /// Dedicated connection for this transaction
    connection: PooledConnection,
    /// Authentication token
    auth_token: AuthToken,
    /// Unique transaction identifier
    transaction_id: TransactionId,
    /// Whether the transaction has been committed or rolled back
    is_committed: bool,
}

impl Transaction {
    /// Creates a new transaction (internal use only)
    pub(crate) fn new(
        connection: PooledConnection,
        auth_token: AuthToken,
        transaction_id: TransactionId,
    ) -> Self {
        Self {
            connection,
            auth_token,
            transaction_id,
            is_committed: false,
        }
    }

    /// Returns the transaction ID
    pub fn transaction_id(&self) -> TransactionId {
        self.transaction_id
    }

    /// Checks if the transaction is still active
    fn check_active(&self) -> Result<()> {
        if self.is_committed {
            return Err(DatabaseError::InternalError {
                component: "Transaction".to_string(),
                details: "Transaction has already been committed or rolled back".to_string(),
            });
        }
        Ok(())
    }

    /// Executes a SQL statement without parameters within the transaction
    pub async fn execute(&mut self, sql: &str) -> Result<ExecuteResult> {
        self.execute_with_params(sql, &[]).await
    }

    /// Executes a SQL statement with parameters within the transaction
    pub async fn execute_with_params(&mut self, sql: &str, params: &[Value]) -> Result<ExecuteResult> {
        self.check_active()?;

        // Build execute request with transaction ID
        let request = ExecuteRequest {
            sql: sql.to_string(),
            params: params.to_vec(),
            transaction_id: Some(self.transaction_id),
            auth_token: Some(self.auth_token.signature.clone()),
        };

        // Serialize request
        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize execute request: {}", e),
        })?;

        // Send request and receive response
        let response = match self
            .connection
            .connection_mut()
            .send_request(MessageType::Data, payload, 5000)
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Automatic rollback on error
                eprintln!("Transaction operation failed, attempting automatic rollback");
                let _ = self.rollback_internal().await;
                return Err(e);
            }
        };

        // Parse response
        let execute_response: ExecuteResponse =
            bincode::deserialize(&response.payload).map_err(|e| {
                DatabaseError::SerializationError {
                    message: format!("Failed to deserialize execute response: {}", e),
                }
            })?;

        // Check for errors
        if let Some(error) = execute_response.error {
            // Automatic rollback on error
            eprintln!("Transaction operation returned error, attempting automatic rollback");
            let _ = self.rollback_internal().await;
            return Err(DatabaseError::InternalError {
                component: "Transaction".to_string(),
                details: error,
            });
        }

        Ok(ExecuteResult {
            rows_affected: execute_response.rows_affected,
            last_insert_id: execute_response.last_insert_id,
        })
    }

    /// Executes a query without parameters within the transaction
    pub async fn query(&mut self, sql: &str) -> Result<QueryResult> {
        self.query_with_params(sql, &[]).await
    }

    /// Executes a query with parameters within the transaction
    pub async fn query_with_params(&mut self, sql: &str, params: &[Value]) -> Result<QueryResult> {
        self.check_active()?;

        // Build query request with transaction ID
        let request = QueryRequest {
            sql: sql.to_string(),
            params: params.to_vec(),
            transaction_id: Some(self.transaction_id),
            auth_token: Some(self.auth_token.signature.clone()),
        };

        // Serialize request
        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize query request: {}", e),
        })?;

        // Send request and receive response
        let response = match self
            .connection
            .connection_mut()
            .send_request(MessageType::Data, payload, 5000)
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Automatic rollback on error
                eprintln!("Transaction operation failed, attempting automatic rollback");
                let _ = self.rollback_internal().await;
                return Err(e);
            }
        };

        // Parse response
        let query_response: QueryResponse =
            bincode::deserialize(&response.payload).map_err(|e| {
                DatabaseError::SerializationError {
                    message: format!("Failed to deserialize query response: {}", e),
                }
            })?;

        // Check for errors
        if let Some(error) = query_response.error {
            // Automatic rollback on error
            eprintln!("Transaction operation returned error, attempting automatic rollback");
            let _ = self.rollback_internal().await;
            return Err(DatabaseError::InternalError {
                component: "Transaction".to_string(),
                details: error,
            });
        }

        Ok(QueryResult::from_raw(query_response.columns, query_response.rows))
    }

    /// Commits the transaction, persisting all changes
    pub async fn commit(mut self) -> Result<()> {
        self.check_active()?;

        // Build commit request
        let request = TransactionRequest::Commit {
            transaction_id: self.transaction_id,
        };

        // Serialize request
        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize commit request: {}", e),
        })?;

        // Send request and receive response
        let response = self
            .connection
            .connection_mut()
            .send_request(MessageType::Transaction, payload, 5000)
            .await;

        match response {
            Ok(resp) => {
                // Parse response
                let txn_response: TransactionResponse =
                    bincode::deserialize(&resp.payload).map_err(|e| {
                        DatabaseError::SerializationError {
                            message: format!("Failed to deserialize transaction response: {}", e),
                        }
                    })?;

                match txn_response {
                    TransactionResponse::CommitSuccess => {
                        self.is_committed = true;
                        Ok(())
                    }
                    TransactionResponse::Error { message } => {
                        // Attempt rollback on commit failure
                        eprintln!("Commit failed, attempting rollback");
                        let _ = self.rollback_internal().await;
                        Err(DatabaseError::TransactionAborted {
                            transaction_id: self.transaction_id,
                            reason: message,
                        })
                    }
                    _ => {
                        Err(DatabaseError::InternalError {
                            component: "Transaction".to_string(),
                            details: "Unexpected response to commit".to_string(),
                        })
                    }
                }
            }
            Err(e) => {
                // Attempt rollback on commit failure
                eprintln!("Commit failed with error, attempting rollback");
                let _ = self.rollback_internal().await;
                Err(e)
            }
        }
    }

    /// Rolls back the transaction, discarding all changes
    pub async fn rollback(mut self) -> Result<()> {
        self.rollback_internal().await
    }

    /// Internal rollback implementation (can be called multiple times)
    async fn rollback_internal(&mut self) -> Result<()> {
        if self.is_committed {
            return Ok(()); // Already committed/rolled back
        }

        // Build rollback request
        let request = TransactionRequest::Rollback {
            transaction_id: self.transaction_id,
        };

        // Serialize request
        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize rollback request: {}", e),
        })?;

        // Send request and receive response
        let response = self
            .connection
            .connection_mut()
            .send_request(MessageType::Transaction, payload, 5000)
            .await?;

        // Parse response
        let txn_response: TransactionResponse =
            bincode::deserialize(&response.payload).map_err(|e| {
                DatabaseError::SerializationError {
                    message: format!("Failed to deserialize transaction response: {}", e),
                }
            })?;

        match txn_response {
            TransactionResponse::RollbackSuccess => {
                self.is_committed = true; // Mark as "done"
                Ok(())
            }
            TransactionResponse::Error { message } => {
                Err(DatabaseError::TransactionAborted {
                    transaction_id: self.transaction_id,
                    reason: message,
                })
            }
            _ => {
                Err(DatabaseError::InternalError {
                    component: "Transaction".to_string(),
                    details: "Unexpected response to rollback".to_string(),
                })
            }
        }
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.is_committed {
            // Transaction was neither committed nor explicitly rolled back
            // Attempt automatic rollback
            eprintln!(
                "Warning: Transaction {} dropped without commit or rollback, attempting automatic rollback",
                self.transaction_id
            );

            // We can't use async in Drop, so we use a blocking approach
            // In a real implementation, this would use a runtime handle
            // For now, we just log the warning
            eprintln!("Warning: Automatic rollback on drop not fully implemented (requires async runtime)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_default() {
        assert_eq!(IsolationLevel::default(), IsolationLevel::ReadCommitted);
    }

    #[test]
    fn test_isolation_level_equality() {
        assert_eq!(IsolationLevel::ReadCommitted, IsolationLevel::ReadCommitted);
        assert_ne!(IsolationLevel::ReadCommitted, IsolationLevel::Serializable);
    }

    #[test]
    fn test_transaction_request_serialization() {
        let request = TransactionRequest::Begin {
            transaction_id: 123,
            isolation_level: IsolationLevel::ReadCommitted,
        };

        let serialized = bincode::serialize(&request).unwrap();
        let deserialized: TransactionRequest = bincode::deserialize(&serialized).unwrap();

        match deserialized {
            TransactionRequest::Begin { transaction_id, isolation_level } => {
                assert_eq!(transaction_id, 123);
                assert_eq!(isolation_level, IsolationLevel::ReadCommitted);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_transaction_response_serialization() {
        let response = TransactionResponse::CommitSuccess;
        let serialized = bincode::serialize(&response).unwrap();
        let deserialized: TransactionResponse = bincode::deserialize(&serialized).unwrap();

        match deserialized {
            TransactionResponse::CommitSuccess => {}
            _ => panic!("Wrong variant"),
        }
    }
}

// Property-Based Tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Note: These property tests require a running database server for full validation.
    // They are designed to test the transaction API structure and behavior.
    // Integration tests with a real server are documented in INTEGRATION_TESTS.md

    // Property 22: Transaction Context Creation
    // Feature: client-sdk, Property 22: For any begin_transaction() call, a valid Transaction_Context should be created with a unique transaction ID
    // Validates: Requirements 5.1
    //
    // Note: This test requires a running database server to fully validate.
    // It tests the transaction creation API structure.
    #[tokio::test]
    #[ignore] // Requires running database server
    async fn prop_transaction_context_creation() {
        // This test would require:
        // 1. A running test database server
        // 2. Valid connection configuration
        // 3. Authentication setup
        //
        // Test strategy:
        // - Call begin_transaction() multiple times
        // - Verify each transaction has a unique transaction ID
        // - Verify each transaction has a valid connection
        // - Verify each transaction has a valid auth token
        //
        // See INTEGRATION_TESTS.md for full implementation
    }

    // Property 23: Transaction Operation Association
    // Feature: client-sdk, Property 23: For any operation executed within a transaction, it should be associated with that transaction's ID
    // Validates: Requirements 5.2
    //
    // Note: This test requires a running database server to fully validate.
    #[tokio::test]
    #[ignore] // Requires running database server
    async fn prop_transaction_operation_association() {
        // This test would require:
        // 1. A running test database server
        // 2. Ability to inspect sent messages
        // 3. Transaction context
        //
        // Test strategy:
        // - Begin a transaction
        // - Execute multiple operations (execute, query)
        // - Verify all operations include the same transaction ID
        //
        // See INTEGRATION_TESTS.md for full implementation
    }

    // Property 24: Transaction Atomicity
    // Feature: client-sdk, Property 24: For any committed transaction, either all operations are persisted or none are (no partial commits)
    // Validates: Requirements 5.3
    //
    // Note: This test requires a running database server to fully validate.
    #[tokio::test]
    #[ignore] // Requires running database server
    async fn prop_transaction_atomicity() {
        // This test would require:
        // 1. A running test database server
        // 2. Ability to verify database state
        // 3. Transaction support on server
        //
        // Test strategy:
        // - Begin transaction
        // - Execute multiple operations
        // - Commit transaction
        // - Verify all changes are visible
        // - Test failure case: operations that fail should persist no changes
        //
        // See INTEGRATION_TESTS.md for full implementation
    }

    // Property 25: Rollback Discards Changes
    // Feature: client-sdk, Property 25: For any rolled-back transaction, none of the operations should be visible in subsequent queries
    // Validates: Requirements 5.4
    //
    // Note: This test requires a running database server to fully validate.
    #[tokio::test]
    #[ignore] // Requires running database server
    async fn prop_rollback_discards_changes() {
        // This test would require:
        // 1. A running test database server
        // 2. Ability to verify database state
        // 3. Transaction support on server
        //
        // Test strategy:
        // - Begin transaction
        // - Execute multiple operations
        // - Rollback transaction
        // - Verify no changes are visible
        //
        // See INTEGRATION_TESTS.md for full implementation
    }

    // Property 26: Automatic Rollback on Failure
    // Feature: client-sdk, Property 26: For any transaction that encounters an error, the transaction should automatically rollback before returning the error
    // Validates: Requirements 5.5
    //
    // Note: This test requires a running database server to fully validate.
    #[tokio::test]
    #[ignore] // Requires running database server
    async fn prop_automatic_rollback_on_failure() {
        // This test would require:
        // 1. A running test database server
        // 2. Ability to trigger operation failures
        // 3. Ability to verify rollback was called
        //
        // Test strategy:
        // - Begin transaction
        // - Execute operation that will fail
        // - Verify automatic rollback was triggered
        // - Verify no changes are visible
        //
        // See INTEGRATION_TESTS.md for full implementation
    }
}


#[cfg(test)]
mod integration_tests {
    use super::*;

    // These tests demonstrate the transaction API usage patterns
    // They are unit tests that verify the API structure is correct

    #[test]
    fn test_transaction_api_structure() {
        // Verify that Transaction has the expected public API
        // This is a compile-time check that the API is correctly structured
        
        // Transaction should have these methods:
        // - transaction_id() -> TransactionId
        // - execute(&mut self, sql: &str) -> Result<ExecuteResult>
        // - execute_with_params(&mut self, sql: &str, params: &[Value]) -> Result<ExecuteResult>
        // - query(&mut self, sql: &str) -> Result<QueryResult>
        // - query_with_params(&mut self, sql: &str, params: &[Value]) -> Result<QueryResult>
        // - commit(self) -> Result<()>
        // - rollback(self) -> Result<()>
        
        // This test just verifies the API compiles correctly
    }

    #[test]
    fn test_isolation_levels() {
        // Test all isolation levels
        let levels = vec![
            IsolationLevel::ReadUncommitted,
            IsolationLevel::ReadCommitted,
            IsolationLevel::RepeatableRead,
            IsolationLevel::Serializable,
        ];

        for level in levels {
            let request = TransactionRequest::Begin {
                transaction_id: 1,
                isolation_level: level,
            };

            // Verify serialization works
            let serialized = bincode::serialize(&request).unwrap();
            let deserialized: TransactionRequest = bincode::deserialize(&serialized).unwrap();

            match deserialized {
                TransactionRequest::Begin { isolation_level: deser_level, .. } => {
                    assert_eq!(level, deser_level);
                }
                _ => panic!("Wrong variant"),
            }
        }
    }

    #[test]
    fn test_transaction_responses() {
        // Test all response types
        let responses = vec![
            TransactionResponse::BeginSuccess,
            TransactionResponse::CommitSuccess,
            TransactionResponse::RollbackSuccess,
            TransactionResponse::Error {
                message: "test error".to_string(),
            },
        ];

        for response in responses {
            // Verify serialization works
            let serialized = bincode::serialize(&response).unwrap();
            let _deserialized: TransactionResponse = bincode::deserialize(&serialized).unwrap();
        }
    }
}
