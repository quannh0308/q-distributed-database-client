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

### Task 9: Implement Transaction Support

This task implements ACID transaction capabilities for the Client SDK, allowing multiple database operations to be executed atomically.

#### Design Overview

Transactions provide atomicity guarantees - either all operations succeed or all fail. The SDK will implement:

1. **Transaction Struct**: Manages transaction lifecycle and state
2. **Transaction Context**: Associates operations with a transaction ID
3. **Automatic Rollback**: Ensures cleanup on errors or drop
4. **DataClient Integration**: Provides begin_transaction() API

#### Transaction Architecture

```
DataClient
    ↓
begin_transaction()
    ↓
Acquire Connection from Pool
    ↓
Send BEGIN TRANSACTION message
    ↓
Create Transaction instance
    ↓
Transaction {
    connection: PooledConnection,
    auth_token: AuthToken,
    transaction_id: TransactionId,
    is_committed: bool
}
    ↓
execute() / query() operations
    ↓
commit() OR rollback() OR Drop
```

#### Component Design

**1. Transaction Struct**

```rust
pub struct Transaction {
    connection: PooledConnection,
    auth_token: AuthToken,
    transaction_id: TransactionId,
    is_committed: bool,
}

impl Transaction {
    pub async fn execute(&mut self, sql: &str) -> Result<ExecuteResult>;
    pub async fn execute_with_params(&mut self, sql: &str, params: &[Value]) -> Result<ExecuteResult>;
    pub async fn query(&mut self, sql: &str) -> Result<QueryResult>;
    pub async fn query_with_params(&mut self, sql: &str, params: &[Value]) -> Result<QueryResult>;
    pub async fn commit(mut self) -> Result<()>;
    pub async fn rollback(mut self) -> Result<()>;
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.is_committed {
            // Attempt rollback on drop
            // Log warning if rollback fails
        }
    }
}
```

**Responsibilities:**
- Execute operations within transaction context
- Include transaction_id in all operation messages
- Track commit status to prevent double-commit
- Implement automatic rollback on drop

**2. DataClient Extension**

Add transaction support to DataClient:

```rust
impl DataClient {
    pub async fn begin_transaction(&self) -> Result<Transaction> {
        // 1. Acquire connection from pool
        let connection = self.connection_manager.get_connection().await?;
        
        // 2. Get valid auth token
        let auth_token = self.auth_manager.get_valid_token(&mut connection).await?;
        
        // 3. Generate unique transaction ID
        let transaction_id = TransactionId::new();
        
        // 4. Send BEGIN TRANSACTION message
        let request = Request::Transaction(TransactionRequest::Begin {
            transaction_id,
            isolation_level: IsolationLevel::ReadCommitted,
        });
        
        let response = connection.send_request(request).await?;
        
        // 5. Verify success
        match response {
            Response::Transaction(TransactionResponse::BeginSuccess) => {
                Ok(Transaction {
                    connection,
                    auth_token,
                    transaction_id,
                    is_committed: false,
                })
            }
            Response::Error(e) => Err(e.into()),
            _ => Err(DatabaseError::ProtocolError),
        }
    }
}
```

**3. Transaction Operations**

Operations within a transaction include the transaction ID:

```rust
impl Transaction {
    pub async fn execute(&mut self, sql: &str) -> Result<ExecuteResult> {
        let request = Request::Execute(ExecuteRequest {
            sql: sql.to_string(),
            params: vec![],
            transaction_id: Some(self.transaction_id),
        });
        
        let response = self.connection.send_request(request).await?;
        
        match response {
            Response::Execute(result) => Ok(result),
            Response::Error(e) => {
                // Automatic rollback on error
                self.rollback().await?;
                Err(e.into())
            }
            _ => Err(DatabaseError::ProtocolError),
        }
    }
}
```

**4. Commit Implementation**

```rust
impl Transaction {
    pub async fn commit(mut self) -> Result<()> {
        if self.is_committed {
            return Err(DatabaseError::TransactionAlreadyCommitted);
        }
        
        let request = Request::Transaction(TransactionRequest::Commit {
            transaction_id: self.transaction_id,
        });
        
        let response = self.connection.send_request(request).await?;
        
        match response {
            Response::Transaction(TransactionResponse::CommitSuccess) => {
                self.is_committed = true;
                Ok(())
            }
            Response::Error(e) => {
                // Rollback on commit failure
                self.rollback().await?;
                Err(e.into())
            }
            _ => Err(DatabaseError::ProtocolError),
        }
    }
}
```

**5. Rollback Implementation**

```rust
impl Transaction {
    pub async fn rollback(mut self) -> Result<()> {
        if self.is_committed {
            return Err(DatabaseError::TransactionAlreadyCommitted);
        }
        
        let request = Request::Transaction(TransactionRequest::Rollback {
            transaction_id: self.transaction_id,
        });
        
        let response = self.connection.send_request(request).await?;
        
        match response {
            Response::Transaction(TransactionResponse::RollbackSuccess) => {
                self.is_committed = true; // Mark as "done" to prevent Drop rollback
                Ok(())
            }
            Response::Error(e) => Err(e.into()),
            _ => Err(DatabaseError::ProtocolError),
        }
    }
}
```

**6. Automatic Rollback on Drop**

```rust
impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.is_committed {
            // Transaction was neither committed nor explicitly rolled back
            // Attempt automatic rollback
            
            let transaction_id = self.transaction_id;
            let connection = &mut self.connection;
            
            // Spawn blocking task to rollback
            // (Drop cannot be async, so we do best-effort sync rollback)
            if let Err(e) = block_on(async {
                let request = Request::Transaction(TransactionRequest::Rollback {
                    transaction_id,
                });
                connection.send_request(request).await
            }) {
                eprintln!("Warning: Failed to rollback transaction on drop: {:?}", e);
            }
        }
    }
}
```

#### Message Protocol Extensions

**Transaction Request Types:**

```rust
pub enum TransactionRequest {
    Begin {
        transaction_id: TransactionId,
        isolation_level: IsolationLevel,
    },
    Commit {
        transaction_id: TransactionId,
    },
    Rollback {
        transaction_id: TransactionId,
    },
}

pub enum TransactionResponse {
    BeginSuccess,
    CommitSuccess,
    RollbackSuccess,
}

pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}
```

**Execute/Query Request Extensions:**

```rust
pub struct ExecuteRequest {
    pub sql: String,
    pub params: Vec<Value>,
    pub transaction_id: Option<TransactionId>, // NEW: Optional transaction context
}

pub struct QueryRequest {
    pub sql: String,
    pub params: Vec<Value>,
    pub transaction_id: Option<TransactionId>, // NEW: Optional transaction context
}
```

#### Error Handling

**Transaction-Specific Errors:**

```rust
pub enum DatabaseError {
    // ... existing errors ...
    
    // Transaction Errors
    TransactionAborted { transaction_id: TransactionId, reason: String },
    TransactionAlreadyCommitted,
    TransactionAlreadyRolledBack,
    DeadlockDetected { transaction_id: TransactionId },
    IsolationViolation { details: String },
}
```

**Error Handling Strategy:**

1. **Operation Errors**: Automatically rollback transaction before returning error
2. **Commit Errors**: Attempt rollback, then return commit error
3. **Rollback Errors**: Log warning, return error (transaction is in unknown state)
4. **Drop Errors**: Log warning, don't panic (best-effort cleanup)

#### Correctness Properties

**Property 22: Transaction Context Creation**
*For any* begin_transaction() call, a valid Transaction_Context should be created with a unique transaction ID.
**Validates: Requirements 5.1**

**Property 23: Transaction Operation Association**
*For any* operation executed within a transaction, it should be associated with that transaction's ID.
**Validates: Requirements 5.2**

**Property 24: Transaction Atomicity**
*For any* committed transaction, either all operations are persisted or none are (no partial commits).
**Validates: Requirements 5.3**

**Property 25: Rollback Discards Changes**
*For any* rolled-back transaction, none of the operations should be visible in subsequent queries.
**Validates: Requirements 5.4**

**Property 26: Automatic Rollback on Failure**
*For any* transaction that encounters an error, the transaction should automatically rollback before returning the error.
**Validates: Requirements 5.5**

#### Testing Strategy

**Unit Tests:**
- Test transaction creation
- Test commit success
- Test explicit rollback
- Test double-commit prevention
- Test operations on committed transaction

**Property Tests:**
- Property 22: Transaction context creation
- Property 23: Operation association with transaction ID
- Property 24: Atomicity (all-or-nothing)
- Property 25: Rollback discards all changes
- Property 26: Automatic rollback on error

**Integration Tests:**
- Test multi-operation transaction
- Test transaction with query and execute
- Test transaction rollback on error
- Test automatic rollback on drop
- Test concurrent transactions

#### Implementation Notes

**Connection Management:**
- Each transaction gets a dedicated connection from the pool
- Connection is held for the transaction's lifetime
- Connection is returned to pool after commit/rollback

**Transaction ID Generation:**
- Use UUID v4 for globally unique IDs
- Alternative: Sequential IDs with client prefix

**Isolation Levels:**
- Default to ReadCommitted for balance of consistency and performance
- Allow configuration via ConnectionConfig
- Server enforces isolation level

**Nested Transactions:**
- Not supported in this implementation
- Attempting to begin transaction within transaction returns error
- Future enhancement: Savepoints for nested transaction simulation

#### Success Criteria

- ✅ Transaction struct implemented with all methods
- ✅ begin_transaction() creates valid transaction context
- ✅ Operations include transaction_id in requests
- ✅ commit() persists all changes atomically
- ✅ rollback() discards all changes
- ✅ Automatic rollback on error works
- ✅ Drop trait implements automatic rollback
- ✅ All property tests pass (Properties 22-26)
- ✅ Integration with DataClient complete
- ✅ Error handling comprehensive

#### What Comes Next

After Task 9, the next tasks are:
- **Task 10: Implement admin client** - Cluster and user management
- **Task 11: Implement result handling** - Enhanced result processing
- **Task 12: Implement error handling** - Comprehensive error types

---

**Full design with 42 correctness properties available in**: `.kiro/specs/client-sdk/FOUNDATION/design.md`
