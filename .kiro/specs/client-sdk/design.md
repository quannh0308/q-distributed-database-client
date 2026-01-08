# Design Document - Client SDK

## Current Context

This document contains the minimal design context needed for the **current implementation task**.

## Architecture Overview

The Client SDK follows a layered architecture:

```
Client Application
       ↓
Public API Layer (DataClient, AdminClient, QueryBuilder)
       ↓
Connection Management Layer (ConnectionManager, ConnectionPool)
       ↓
Protocol Layer (MessageCodec, Message serialization)
       ↓
Transport Layer (TCP/TLS)
       ↓
Q-Distributed-Database Cluster
```

## Key Components

1. **Client**: Main entry point, manages all sub-components
2. **ConnectionManager**: Connection pooling, health monitoring, failover
3. **MessageCodec**: Bincode serialization with CRC32 checksums
4. **AuthenticationManager**: Token-based authentication
5. **DataClient**: CRUD operations, queries, transactions
6. **QueryBuilder**: Fluent API for SQL construction
7. **AdminClient**: Cluster and user management

## Message Protocol

- **Format**: Bincode serialization
- **Framing**: 4-byte big-endian length prefix + message data
- **Integrity**: CRC32 checksum validation
- **Types**: Ping, Pong, Data, Ack, Error, Heartbeat, ClusterJoin, ClusterLeave, Replication, Transaction

## Current Task Design

### Task 6: Implement Data Client for CRUD Operations

This task implements the DataClient component that handles all CRUD (Create, Read, Update, Delete) operations on database tables.

#### Design Overview

The DataClient is the primary interface for executing database operations. It manages:

1. **Connection Acquisition**: Gets connections from the ConnectionManager
2. **Authentication**: Uses AuthenticationManager to ensure valid tokens
3. **Query Execution**: Sends SQL queries to the database
4. **Result Processing**: Parses and returns query results
5. **Prepared Statements**: Caches prepared statements for performance
6. **Batch Operations**: Groups multiple operations for efficiency
7. **Streaming**: Handles large result sets with bounded memory

#### Component Design

**1. DataClient Structure**

The main struct that handles all data operations:

```rust
pub struct DataClient {
    connection_manager: Arc<ConnectionManager>,
    auth_manager: Arc<AuthenticationManager>,
    prepared_statements: Arc<RwLock<HashMap<String, PreparedStatement>>>,
}

impl DataClient {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        auth_manager: Arc<AuthenticationManager>
    ) -> Self {
        Self {
            connection_manager,
            auth_manager,
            prepared_statements: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn execute(&self, sql: &str) -> Result<ExecuteResult>;
    pub async fn execute_with_params(&self, sql: &str, params: &[Value]) -> Result<ExecuteResult>;
    pub async fn query(&self, sql: &str) -> Result<QueryResult>;
    pub async fn query_with_params(&self, sql: &str, params: &[Value]) -> Result<QueryResult>;
    pub async fn query_stream(&self, sql: &str) -> Result<ResultStream>;
    pub async fn prepare(&self, sql: &str) -> Result<PreparedStatement>;
    pub async fn begin_transaction(&self) -> Result<Transaction>;
}
```

**Fields:**
- `connection_manager`: Manages connection pool and node health
- `auth_manager`: Handles authentication and token management
- `prepared_statements`: Cache of prepared statements for reuse

**2. Execute Operations**

Execute operations are used for INSERT, UPDATE, DELETE statements:

```rust
pub async fn execute(&self, sql: &str) -> Result<ExecuteResult> {
    self.execute_with_params(sql, &[]).await
}

pub async fn execute_with_params(&self, sql: &str, params: &[Value]) -> Result<ExecuteResult> {
    // 1. Get connection from pool
    let mut conn = self.connection_manager.get_connection().await?;
    
    // 2. Get valid auth token
    let token = self.auth_manager.get_valid_token(&mut conn).await?;
    
    // 3. Build execute request
    let request = Request::Execute(ExecuteRequest {
        sql: sql.to_string(),
        params: params.to_vec(),
        prepared_statement_id: None,
        auth_token: Some(token),
    });
    
    // 4. Send request and receive response
    let response = conn.send_request(request).await?;
    
    // 5. Parse response
    match response {
        Response::Execute(result) => Ok(result),
        Response::Error(err) => Err(err.into()),
        _ => Err(DatabaseError::ProtocolError { 
            message: "Unexpected response type".to_string() 
        }),
    }
}
```

**ExecuteResult Structure:**
```rust
pub struct ExecuteResult {
    pub rows_affected: u64,
    pub last_insert_id: Option<i64>,
}
```

**3. Query Operations**

Query operations are used for SELECT statements:

```rust
pub async fn query(&self, sql: &str) -> Result<QueryResult> {
    self.query_with_params(sql, &[]).await
}

pub async fn query_with_params(&self, sql: &str, params: &[Value]) -> Result<QueryResult> {
    // 1. Get connection from pool
    let mut conn = self.connection_manager.get_connection().await?;
    
    // 2. Get valid auth token
    let token = self.auth_manager.get_valid_token(&mut conn).await?;
    
    // 3. Build query request
    let request = Request::Query(QueryRequest {
        sql: sql.to_string(),
        params: params.to_vec(),
        prepared_statement_id: None,
        auth_token: Some(token),
    });
    
    // 4. Send request and receive response
    let response = conn.send_request(request).await?;
    
    // 5. Parse response
    match response {
        Response::Query(result) => {
            // Convert server response to QueryResult
            Ok(QueryResult {
                columns: result.columns,
                rows: result.rows.into_iter()
                    .map(|values| Row { values })
                    .collect(),
            })
        },
        Response::Error(err) => Err(err.into()),
        _ => Err(DatabaseError::ProtocolError { 
            message: "Unexpected response type".to_string() 
        }),
    }
}
```

**QueryResult Structure:**
```rust
pub struct QueryResult {
    pub columns: Vec<ColumnMetadata>,
    pub rows: Vec<Row>,
}

pub struct ColumnMetadata {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

pub struct Row {
    values: Vec<Value>,
}

impl Row {
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }
    
    pub fn get_by_name(&self, name: &str, columns: &[ColumnMetadata]) -> Option<&Value> {
        columns.iter()
            .position(|col| col.name == name)
            .and_then(|idx| self.values.get(idx))
    }
}
```

**4. Streaming Results**

For large result sets, streaming avoids loading all data into memory:

```rust
pub async fn query_stream(&self, sql: &str) -> Result<ResultStream> {
    // 1. Get connection from pool
    let mut conn = self.connection_manager.get_connection().await?;
    
    // 2. Get valid auth token
    let token = self.auth_manager.get_valid_token(&mut conn).await?;
    
    // 3. Build streaming query request
    let request = Request::Query(QueryRequest {
        sql: sql.to_string(),
        params: vec![],
        prepared_statement_id: None,
        auth_token: Some(token),
        streaming: true,  // Enable streaming mode
    });
    
    // 4. Send request
    conn.send_message(request.into()).await?;
    
    // 5. Return stream that will fetch rows incrementally
    Ok(ResultStream::new(conn))
}

pub struct ResultStream {
    connection: PooledConnection,
    columns: Option<Vec<ColumnMetadata>>,
    finished: bool,
}

impl ResultStream {
    pub async fn next(&mut self) -> Result<Option<Row>> {
        if self.finished {
            return Ok(None);
        }
        
        // Receive next message from server
        let message = self.connection.receive_message().await?;
        
        match message.message_type {
            MessageType::Data => {
                // Parse row data
                let row = parse_row(&message.payload)?;
                Ok(Some(row))
            },
            MessageType::Ack => {
                // End of stream
                self.finished = true;
                Ok(None)
            },
            MessageType::Error => {
                let error = parse_error(&message.payload)?;
                Err(error)
            },
            _ => Err(DatabaseError::ProtocolError {
                message: "Unexpected message type in stream".to_string()
            }),
        }
    }
}
```

**5. Batch Operations**

Batch operations group multiple statements for atomic execution:

```rust
pub async fn batch(&self) -> Result<BatchContext> {
    // Get connection from pool
    let conn = self.connection_manager.get_connection().await?;
    
    Ok(BatchContext {
        connection: conn,
        auth_manager: self.auth_manager.clone(),
        operations: Vec::new(),
    })
}

pub struct BatchContext {
    connection: PooledConnection,
    auth_manager: Arc<AuthenticationManager>,
    operations: Vec<BatchOperation>,
}

impl BatchContext {
    pub fn add_execute(&mut self, sql: &str, params: &[Value]) {
        self.operations.push(BatchOperation::Execute {
            sql: sql.to_string(),
            params: params.to_vec(),
        });
    }
    
    pub async fn execute(mut self) -> Result<Vec<ExecuteResult>> {
        // 1. Get valid auth token
        let token = self.auth_manager.get_valid_token(&mut self.connection).await?;
        
        // 2. Build batch request
        let request = Request::Batch(BatchRequest {
            operations: self.operations,
            auth_token: Some(token),
        });
        
        // 3. Send request and receive response
        let response = self.connection.send_request(request).await?;
        
        // 4. Parse response
        match response {
            Response::Batch(results) => Ok(results),
            Response::Error(err) => Err(err.into()),
            _ => Err(DatabaseError::ProtocolError {
                message: "Unexpected response type".to_string()
            }),
        }
    }
}
```

**6. Prepared Statements**

Prepared statements are cached for performance:

```rust
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
    let token = self.auth_manager.get_valid_token(&mut conn).await?;
    
    let request = Request::Prepare(PrepareRequest {
        sql: sql.to_string(),
        auth_token: Some(token),
    });
    
    let response = conn.send_request(request).await?;
    
    match response {
        Response::Prepare(stmt) => {
            // Add to cache
            let mut cache = self.prepared_statements.write().await;
            cache.insert(sql.to_string(), stmt.clone());
            Ok(stmt)
        },
        Response::Error(err) => Err(err.into()),
        _ => Err(DatabaseError::ProtocolError {
            message: "Unexpected response type".to_string()
        }),
    }
}

pub struct PreparedStatement {
    pub statement_id: StatementId,
    pub sql: String,
    pub param_count: usize,
}
```

#### Request/Response Flow

**Execute Flow:**
```
Client                          Server
  │                               │
  ├─── ExecuteRequest ───────────>│
  │    (SQL, params, token)       │
  │                               │
  │<─── ExecuteResponse ──────────┤
  │    (rows_affected, insert_id) │
```

**Query Flow:**
```
Client                          Server
  │                               │
  ├─── QueryRequest ─────────────>│
  │    (SQL, params, token)       │
  │                               │
  │<─── QueryResponse ────────────┤
  │    (columns, rows)            │
```

**Streaming Flow:**
```
Client                          Server
  │                               │
  ├─── QueryRequest ─────────────>│
  │    (SQL, streaming=true)      │
  │                               │
  │<─── Data ─────────────────────┤
  │    (row 1)                    │
  │                               │
  │<─── Data ─────────────────────┤
  │    (row 2)                    │
  │                               │
  │<─── Ack ──────────────────────┤
  │    (end of stream)            │
```

**Batch Flow:**
```
Client                          Server
  │                               │
  ├─── BatchRequest ─────────────>│
  │    (operations[], token)      │
  │                               │
  │<─── BatchResponse ────────────┤
  │    (results[])                │
```

#### Error Handling

**Execute/Query Errors:**
- `SyntaxError`: Invalid SQL syntax
- `TableNotFound`: Referenced table doesn't exist
- `ColumnNotFound`: Referenced column doesn't exist
- `ConstraintViolation`: Constraint check failed
- `AuthenticationFailed`: Invalid or expired token
- `TimeoutError`: Operation exceeded timeout

**Error Handling Strategy:**
1. Parse error response from server
2. Convert to appropriate DatabaseError variant
3. Include context (SQL, parameters) in error
4. Return error to caller

#### Integration with Existing Components

**Connection Manager Integration:**
```rust
// DataClient uses ConnectionManager to get connections
let mut conn = self.connection_manager.get_connection().await?;

// Connection is automatically returned to pool when dropped
```

**Authentication Manager Integration:**
```rust
// DataClient uses AuthenticationManager for tokens
let token = self.auth_manager.get_valid_token(&mut conn).await?;

// Token is automatically refreshed if expired
```

**Client Integration:**
```rust
impl Client {
    pub fn data(&self) -> &DataClient {
        &self.data_client
    }
}

// Usage:
let result = client.data().query("SELECT * FROM users").await?;
```

#### Testing Strategy

**Unit Tests:**
- Test ExecuteResult structure creation
- Test QueryResult structure creation
- Test Row access methods (get, get_by_name)
- Test prepared statement caching

**Property Tests:**

**Property 13: Insert-Then-Retrieve Consistency**
- Generate random records
- Insert record using execute_with_params()
- Query for record using query_with_params()
- Verify returned values match inserted values

**Property 14: Update Visibility**
- Generate random records and updates
- Insert record, then update it
- Query for record
- Verify returned values match updated values

**Property 15: Delete Removes Record**
- Generate random records
- Insert record, then delete it
- Query for record
- Verify no results returned

**Property 16: Operation Result Structure**
- Generate random operations
- Execute operations
- Verify result contains rows_affected or error details

**Property 17: Batch Operation Atomicity**
- Generate batch of operations with one that will fail
- Execute batch
- Verify either all succeed or all fail (no partial success)

**Property 35: Streaming Memory Efficiency**
- Generate large result set
- Stream results using query_stream()
- Monitor memory usage
- Verify memory remains bounded

#### Performance Considerations

**Connection Pooling:**
- Reuse connections from pool
- Avoid connection overhead per query
- Return connections promptly

**Prepared Statements:**
- Cache prepared statements on client
- Reduce server-side parsing overhead
- Reuse parsed query plans

**Batch Operations:**
- Group multiple operations
- Reduce network round trips
- Improve throughput

**Streaming:**
- Process rows incrementally
- Minimize memory usage
- Support backpressure

#### Implementation Notes

**Concurrency:**
- DataClient is thread-safe (uses Arc for shared state)
- Multiple queries can execute concurrently
- Prepared statement cache uses RwLock for concurrent access

**Memory Management:**
- Streaming results avoid loading all data
- Connections returned to pool automatically
- Prepared statement cache has bounded size

**Error Recovery:**
- Failed operations don't affect connection pool
- Connections are validated before reuse
- Automatic retry for transient errors

---

**Full design with 42 correctness properties available in**: `.kiro/specs/client-sdk/FOUNDATION/design.md`
