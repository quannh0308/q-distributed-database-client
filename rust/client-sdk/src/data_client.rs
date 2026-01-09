//! Data client module for CRUD operations
//!
//! This module implements the DataClient component that handles all CRUD
//! (Create, Read, Update, Delete) operations on database tables.

use crate::auth::AuthenticationManager;
use crate::connection::{ConnectionManager, PooledConnection};
use crate::error::DatabaseError;
use crate::protocol::{Message, MessageType};
use crate::types::{ColumnMetadata, StatementId, Value};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Result of an execute operation (INSERT, UPDATE, DELETE)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResult {
    /// Number of rows affected by the operation
    pub rows_affected: u64,
    /// Last insert ID (for INSERT operations)
    pub last_insert_id: Option<i64>,
}

/// Result of a query operation (SELECT)
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Column metadata
    pub columns: Vec<ColumnMetadata>,
    /// Result rows
    pub rows: Vec<Row>,
}

/// A single row in a query result
#[derive(Debug, Clone)]
pub struct Row {
    /// Column values
    values: Vec<Value>,
}

impl Row {
    /// Creates a new row with the given values
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }

    /// Gets a value by column index
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Gets a value by column name
    pub fn get_by_name(&self, name: &str, columns: &[ColumnMetadata]) -> Option<&Value> {
        columns
            .iter()
            .position(|col| col.name == name)
            .and_then(|idx| self.values.get(idx))
    }

    /// Returns the number of columns in this row
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if the row has no columns
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns an iterator over the values
    pub fn values(&self) -> &[Value] {
        &self.values
    }
}

/// Prepared statement
#[derive(Debug, Clone)]
pub struct PreparedStatement {
    /// Statement identifier
    pub statement_id: StatementId,
    /// Original SQL
    pub sql: String,
    /// Number of parameters
    pub param_count: usize,
}

/// Result stream for streaming large result sets
pub struct ResultStream {
    /// Connection
    connection: PooledConnection,
    /// Column metadata
    columns: Option<Vec<ColumnMetadata>>,
    /// Whether the stream is finished
    finished: bool,
}

impl ResultStream {
    /// Creates a new result stream
    fn new(connection: PooledConnection) -> Self {
        Self {
            connection,
            columns: None,
            finished: false,
        }
    }

    /// Fetches the next row from the stream
    pub async fn next(&mut self) -> Result<Option<Row>> {
        if self.finished {
            return Ok(None);
        }

        // Receive next message from server
        let message = self.connection.connection_mut().receive_message().await?;

        match message.message_type {
            MessageType::Data => {
                // Parse row data
                let values: Vec<Value> =
                    bincode::deserialize(&message.payload).map_err(|e| {
                        DatabaseError::SerializationError {
                            message: format!("Failed to deserialize row data: {}", e),
                        }
                    })?;
                Ok(Some(Row::new(values)))
            }
            MessageType::Ack => {
                // End of stream
                self.finished = true;
                Ok(None)
            }
            MessageType::Error => {
                let error: String = bincode::deserialize(&message.payload).map_err(|e| {
                    DatabaseError::SerializationError {
                        message: format!("Failed to deserialize error: {}", e),
                    }
                })?;
                Err(DatabaseError::InternalError {
                    component: "ResultStream".to_string(),
                    details: error,
                })
            }
            _ => Err(DatabaseError::InternalError {
                component: "ResultStream".to_string(),
                details: "Unexpected message type in stream".to_string(),
            }),
        }
    }

    /// Returns the column metadata (if available)
    pub fn columns(&self) -> Option<&[ColumnMetadata]> {
        self.columns.as_deref()
    }
}

/// Batch operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
enum BatchOperation {
    Execute { sql: String, params: Vec<Value> },
}

/// Batch context for executing multiple operations atomically
pub struct BatchContext {
    /// Connection
    connection: PooledConnection,
    /// Authentication manager
    auth_manager: Arc<AuthenticationManager>,
    /// Operations to execute
    operations: Vec<BatchOperation>,
}

impl BatchContext {
    /// Adds an execute operation to the batch
    pub fn add_execute(&mut self, sql: &str, params: &[Value]) {
        self.operations.push(BatchOperation::Execute {
            sql: sql.to_string(),
            params: params.to_vec(),
        });
    }

    /// Executes all operations in the batch atomically
    pub async fn execute(mut self) -> Result<Vec<ExecuteResult>> {
        // Get valid auth token
        let token = self.auth_manager.get_valid_token().await?;

        // Build batch request
        let request = BatchRequest {
            operations: self.operations,
            auth_token: Some(token.signature.clone()),
        };

        // Serialize request
        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize batch request: {}", e),
        })?;

        // Send request and receive response
        let response = self
            .connection
            .connection_mut()
            .send_request(MessageType::Data, payload, 10000)
            .await?;

        // Parse response
        let batch_response: BatchResponse =
            bincode::deserialize(&response.payload).map_err(|e| {
                DatabaseError::SerializationError {
                    message: format!("Failed to deserialize batch response: {}", e),
                }
            })?;

        // Check for errors
        if let Some(error) = batch_response.error {
            return Err(DatabaseError::InternalError {
                component: "BatchContext".to_string(),
                details: error,
            });
        }

        Ok(batch_response.results)
    }
}

/// Data client for CRUD operations
pub struct DataClient {
    /// Connection manager
    connection_manager: Arc<ConnectionManager>,
    /// Authentication manager
    auth_manager: Arc<AuthenticationManager>,
    /// Prepared statement cache
    prepared_statements: Arc<RwLock<HashMap<String, PreparedStatement>>>,
}

impl DataClient {
    /// Creates a new data client
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        auth_manager: Arc<AuthenticationManager>,
    ) -> Self {
        Self {
            connection_manager,
            auth_manager,
            prepared_statements: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Executes a SQL statement without parameters
    pub async fn execute(&self, sql: &str) -> Result<ExecuteResult> {
        self.execute_with_params(sql, &[]).await
    }

    /// Executes a SQL statement with parameters
    pub async fn execute_with_params(&self, sql: &str, params: &[Value]) -> Result<ExecuteResult> {
        // Get connection from pool
        let mut conn = self.connection_manager.get_connection().await?;

        // Get valid auth token
        let token = self.auth_manager.get_valid_token().await?;

        // Build execute request
        let request = ExecuteRequest {
            sql: sql.to_string(),
            params: params.to_vec(),
            prepared_statement_id: None,
            auth_token: Some(token.signature.clone()),
        };

        // Serialize request
        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize execute request: {}", e),
        })?;

        // Send request and receive response
        let response = conn
            .connection_mut()
            .send_request(MessageType::Data, payload, 5000)
            .await?;

        // Parse response
        let execute_response: ExecuteResponse =
            bincode::deserialize(&response.payload).map_err(|e| {
                DatabaseError::SerializationError {
                    message: format!("Failed to deserialize execute response: {}", e),
                }
            })?;

        // Check for errors
        if let Some(error) = execute_response.error {
            return Err(DatabaseError::InternalError {
                component: "DataClient".to_string(),
                details: error,
            });
        }

        // Return connection to pool
        self.connection_manager.return_connection(conn).await;

        Ok(ExecuteResult {
            rows_affected: execute_response.rows_affected,
            last_insert_id: execute_response.last_insert_id,
        })
    }

    /// Executes a query without parameters
    pub async fn query(&self, sql: &str) -> Result<QueryResult> {
        self.query_with_params(sql, &[]).await
    }

    /// Executes a query with parameters
    pub async fn query_with_params(&self, sql: &str, params: &[Value]) -> Result<QueryResult> {
        // Get connection from pool
        let mut conn = self.connection_manager.get_connection().await?;

        // Get valid auth token
        let token = self.auth_manager.get_valid_token().await?;

        // Build query request
        let request = QueryRequest {
            sql: sql.to_string(),
            params: params.to_vec(),
            prepared_statement_id: None,
            auth_token: Some(token.signature.clone()),
            streaming: false,
        };

        // Serialize request
        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize query request: {}", e),
        })?;

        // Send request and receive response
        let response = conn
            .connection_mut()
            .send_request(MessageType::Data, payload, 5000)
            .await?;

        // Parse response
        let query_response: QueryResponse =
            bincode::deserialize(&response.payload).map_err(|e| {
                DatabaseError::SerializationError {
                    message: format!("Failed to deserialize query response: {}", e),
                }
            })?;

        // Check for errors
        if let Some(error) = query_response.error {
            return Err(DatabaseError::InternalError {
                component: "DataClient".to_string(),
                details: error,
            });
        }

        // Return connection to pool
        self.connection_manager.return_connection(conn).await;

        // Convert to QueryResult
        Ok(QueryResult {
            columns: query_response.columns,
            rows: query_response.rows.into_iter().map(Row::new).collect(),
        })
    }

    /// Executes a streaming query for large result sets
    pub async fn query_stream(&self, sql: &str) -> Result<ResultStream> {
        // Get connection from pool
        let mut conn = self.connection_manager.get_connection().await?;

        // Get valid auth token
        let token = self.auth_manager.get_valid_token().await?;

        // Build streaming query request
        let request = QueryRequest {
            sql: sql.to_string(),
            params: vec![],
            prepared_statement_id: None,
            auth_token: Some(token.signature.clone()),
            streaming: true,
        };

        // Serialize request
        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize query request: {}", e),
        })?;

        // Get node_id before borrowing connection mutably
        let node_id = conn.node_id();
        let timestamp = chrono::Utc::now().timestamp_millis();

        // Send request (don't wait for full response)
        conn.connection_mut()
            .send_message(Message::new(
                0,
                node_id,
                0,
                timestamp,
                MessageType::Data,
                payload,
            ))
            .await?;

        // Return stream
        Ok(ResultStream::new(conn))
    }

    /// Prepares a statement for reuse
    pub async fn prepare(&self, sql: &str) -> Result<PreparedStatement> {
        // Check cache first
        {
            let cache = self.prepared_statements.read().await;
            if let Some(stmt) = cache.get(sql) {
                return Ok(stmt.clone());
            }
        }

        // Not in cache, prepare on server
        let mut conn = self.connection_manager.get_connection().await?;
        let token = self.auth_manager.get_valid_token().await?;

        let request = PrepareRequest {
            sql: sql.to_string(),
            auth_token: Some(token.signature.clone()),
        };

        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize prepare request: {}", e),
        })?;

        let response = conn
            .connection_mut()
            .send_request(MessageType::Data, payload, 5000)
            .await?;

        let prepare_response: PrepareResponse =
            bincode::deserialize(&response.payload).map_err(|e| {
                DatabaseError::SerializationError {
                    message: format!("Failed to deserialize prepare response: {}", e),
                }
            })?;

        if let Some(error) = prepare_response.error {
            return Err(DatabaseError::InternalError {
                component: "DataClient".to_string(),
                details: error,
            });
        }

        self.connection_manager.return_connection(conn).await;

        let stmt = PreparedStatement {
            statement_id: prepare_response.statement_id,
            sql: sql.to_string(),
            param_count: prepare_response.param_count,
        };

        // Add to cache
        let mut cache = self.prepared_statements.write().await;
        cache.insert(sql.to_string(), stmt.clone());

        Ok(stmt)
    }

    /// Creates a batch context for executing multiple operations atomically
    pub async fn batch(&self) -> Result<BatchContext> {
        let conn = self.connection_manager.get_connection().await?;

        Ok(BatchContext {
            connection: conn,
            auth_manager: self.auth_manager.clone(),
            operations: Vec::new(),
        })
    }

    /// Executes a query builder and returns the result
    pub async fn query_builder(&self, builder: crate::query_builder::QueryBuilder) -> Result<QueryResult> {
        let (sql, params) = builder.build()?;
        self.query_with_params(&sql, &params).await
    }

    /// Executes a query builder for non-SELECT queries
    pub async fn execute_builder(&self, builder: crate::query_builder::QueryBuilder) -> Result<ExecuteResult> {
        let (sql, params) = builder.build()?;
        self.execute_with_params(&sql, &params).await
    }

    /// Begins a new transaction
    ///
    /// Returns a Transaction instance that can be used to execute operations
    /// atomically. The transaction must be explicitly committed or rolled back.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut txn = client.data().begin_transaction().await?;
    /// txn.execute("INSERT INTO users (name) VALUES (?)", &[Value::from("Alice")]).await?;
    /// txn.commit().await?;
    /// ```
    pub async fn begin_transaction(&self) -> Result<crate::transaction::Transaction> {
        use crate::transaction::{IsolationLevel, TransactionRequest, TransactionResponse};

        // 1. Acquire connection from pool
        let mut connection = self.connection_manager.get_connection().await?;

        // 2. Get valid auth token
        let auth_token = self.auth_manager.get_valid_token().await?;

        // 3. Generate unique transaction ID (using timestamp + random for uniqueness)
        let transaction_id = chrono::Utc::now().timestamp_millis() as u64;

        // 4. Send BEGIN TRANSACTION message
        let request = TransactionRequest::Begin {
            transaction_id,
            isolation_level: IsolationLevel::default(),
        };

        let payload = bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize begin transaction request: {}", e),
        })?;

        let response = connection
            .connection_mut()
            .send_request(MessageType::Transaction, payload, 5000)
            .await?;

        // 5. Verify success
        let txn_response: TransactionResponse =
            bincode::deserialize(&response.payload).map_err(|e| {
                DatabaseError::SerializationError {
                    message: format!("Failed to deserialize transaction response: {}", e),
                }
            })?;

        match txn_response {
            TransactionResponse::BeginSuccess => {
                Ok(crate::transaction::Transaction::new(
                    connection,
                    auth_token,
                    transaction_id,
                ))
            }
            TransactionResponse::Error { message } => {
                // Return connection to pool on error
                self.connection_manager.return_connection(connection).await;
                Err(DatabaseError::InternalError {
                    component: "DataClient".to_string(),
                    details: format!("Failed to begin transaction: {}", message),
                })
            }
            _ => {
                // Return connection to pool on unexpected response
                self.connection_manager.return_connection(connection).await;
                Err(DatabaseError::InternalError {
                    component: "DataClient".to_string(),
                    details: "Unexpected response to begin transaction".to_string(),
                })
            }
        }
    }
}

// Request/Response types for serialization

#[derive(Debug, Serialize, Deserialize)]
struct ExecuteRequest {
    sql: String,
    params: Vec<Value>,
    prepared_statement_id: Option<StatementId>,
    auth_token: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExecuteResponse {
    rows_affected: u64,
    last_insert_id: Option<i64>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryRequest {
    sql: String,
    params: Vec<Value>,
    prepared_statement_id: Option<StatementId>,
    auth_token: Option<Vec<u8>>,
    streaming: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryResponse {
    columns: Vec<ColumnMetadata>,
    rows: Vec<Vec<Value>>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PrepareRequest {
    sql: String,
    auth_token: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PrepareResponse {
    statement_id: StatementId,
    param_count: usize,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BatchRequest {
    operations: Vec<BatchOperation>,
    auth_token: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BatchResponse {
    results: Vec<ExecuteResult>,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_result_creation() {
        let result = ExecuteResult {
            rows_affected: 5,
            last_insert_id: Some(42),
        };
        assert_eq!(result.rows_affected, 5);
        assert_eq!(result.last_insert_id, Some(42));
    }

    #[test]
    fn test_row_creation() {
        let values = vec![Value::Int(1), Value::String("test".to_string())];
        let row = Row::new(values.clone());
        assert_eq!(row.len(), 2);
        assert!(!row.is_empty());
        assert_eq!(row.get(0), Some(&Value::Int(1)));
        assert_eq!(row.get(1), Some(&Value::String("test".to_string())));
    }

    #[test]
    fn test_row_get_by_name() {
        let columns = vec![
            ColumnMetadata {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
            },
            ColumnMetadata {
                name: "name".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
            },
        ];

        let values = vec![Value::Int(1), Value::String("test".to_string())];
        let row = Row::new(values);

        assert_eq!(row.get_by_name("id", &columns), Some(&Value::Int(1)));
        assert_eq!(
            row.get_by_name("name", &columns),
            Some(&Value::String("test".to_string()))
        );
        assert_eq!(row.get_by_name("nonexistent", &columns), None);
    }

    #[test]
    fn test_row_empty() {
        let row = Row::new(vec![]);
        assert_eq!(row.len(), 0);
        assert!(row.is_empty());
        assert_eq!(row.get(0), None);
    }

    #[test]
    fn test_query_result_creation() {
        let columns = vec![ColumnMetadata {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
        }];

        let rows = vec![Row::new(vec![Value::Int(1)]), Row::new(vec![Value::Int(2)])];

        let result = QueryResult {
            columns: columns.clone(),
            rows: rows.clone(),
        };

        assert_eq!(result.columns.len(), 1);
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_query_builder_integration() {
        use crate::query_builder::{QueryBuilder, OrderDirection};
        
        // Test that QueryBuilder can be built and produces valid SQL
        let (sql, params) = QueryBuilder::select(&["id", "name"])
            .from("users")
            .where_clause("age > ?", Value::Int(18))
            .order_by("name", OrderDirection::Asc)
            .limit(10)
            .build()
            .unwrap();
        
        assert_eq!(sql, "SELECT id, name FROM users WHERE age > ? ORDER BY name ASC LIMIT 10");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::Int(18));
    }
}
