# Requirements Document - Client SDK

## Current Context

This document contains the minimal requirements context needed for the **current implementation task**.

## Key Requirements Summary

The Q-Distributed-Database Client SDK provides a multi-language client library for interacting with q-distributed-database. Key requirements include:

- **Connection Management**: TCP connections on port 7000, connection pooling, automatic failover
- **Authentication**: Token-based auth with username/password, 24-hour TTL
- **CRUD Operations**: Full support for INSERT, SELECT, UPDATE, DELETE
- **Query Building**: Fluent API with SQL injection prevention
- **Transactions**: ACID transactions with automatic rollback
- **Message Protocol**: Bincode serialization with CRC32 checksums, length-prefixed framing
- **Error Handling**: Automatic retry with exponential backoff
- **Result Handling**: Streaming support, type conversion
- **Multi-Language**: Rust, Python, TypeScript implementations

## Technical Specifications

- **Protocol**: TCP (primary), UDP, TLS
- **Port**: 7000 (default)
- **Serialization**: Bincode with CRC32 checksums
- **Max Message Size**: 1MB (configurable)
- **Connection Pool**: 5-20 connections (configurable)
- **Timeout**: 5000ms (default)
- **Token TTL**: 24 hours (default)

## Current Task Requirements

### Task 9: Implement Transaction Support

This task implements ACID transaction capabilities, allowing multiple database operations to be executed atomically with commit and rollback support.

#### Transaction Overview

Transactions provide atomicity, consistency, isolation, and durability (ACID) guarantees for database operations. The SDK will support:
- Beginning transactions with configurable isolation levels
- Executing multiple operations within a transaction context
- Committing transactions to persist all changes
- Rolling back transactions to discard all changes
- Automatic rollback on errors or when transactions are dropped

#### Key Requirements

**From Requirement 5: Transaction Management**

1. **Transaction Context Creation (5.1)**
   - WHEN starting a transaction, THE Client_SDK SHALL create a Transaction_Context with isolation level configuration
   - Transaction context must have a unique transaction ID
   - Transaction must maintain its own connection from the pool

2. **Operation Association (5.2)**
   - WHEN executing operations within a transaction, THE Client_SDK SHALL associate all operations with the transaction context
   - All operations must include the transaction ID in requests
   - Operations must use the transaction's dedicated connection

3. **Atomic Commit (5.3)**
   - WHEN committing a transaction, THE Client_SDK SHALL persist all changes atomically
   - Either all operations succeed or all fail (no partial commits)
   - Commit must respect the configured consistency model

4. **Rollback (5.4)**
   - WHEN rolling back a transaction, THE Client_SDK SHALL discard all changes made within the transaction
   - Rolled-back changes must not be visible in subsequent queries
   - Rollback must release the transaction's resources

5. **Automatic Rollback on Error (5.5)**
   - IF a transaction fails, THEN THE Client_SDK SHALL automatically rollback and return error details
   - Errors during transaction operations must trigger automatic rollback
   - Transaction must be marked as failed after automatic rollback

6. **Nested Transaction Prevention (5.6)**
   - WHERE nested transactions are attempted, THE Client_SDK SHALL return an error
   - Only one active transaction per connection is allowed

#### Implementation Components

**1. Transaction Struct**
- Store connection, auth token, transaction ID
- Track commit status to prevent double-commit
- Implement execute() and query() methods for operations within transaction
- Implement Drop trait for automatic rollback

**2. Transaction Lifecycle**
```
begin_transaction() → Transaction created
    ↓
execute() / query() → Operations within transaction
    ↓
commit() → Changes persisted (success path)
    OR
rollback() → Changes discarded (explicit rollback)
    OR
Drop → Automatic rollback (if not committed)
```

**3. DataClient Integration**
- Add begin_transaction() method to DataClient
- Acquire dedicated connection from pool for transaction
- Send BEGIN TRANSACTION message to server
- Return Transaction instance to caller

#### Technical Specifications

**Transaction Message Types:**
- BEGIN TRANSACTION: Start a new transaction
- COMMIT: Persist all transaction changes
- ROLLBACK: Discard all transaction changes
- EXECUTE (with transaction_id): Execute operation within transaction
- QUERY (with transaction_id): Query within transaction

**Transaction ID:**
- Unique identifier for each transaction
- Generated client-side (UUID or sequential)
- Included in all transaction-related messages

**Isolation Levels:**
- Read Uncommitted
- Read Committed (default)
- Repeatable Read
- Serializable

**Error Handling:**
- Transaction errors trigger automatic rollback
- Rollback errors are logged but don't prevent error propagation
- Double-commit attempts return error
- Operations on committed/rolled-back transactions return error

#### Success Criteria

- ✅ Transaction struct implemented with all required methods
- ✅ begin_transaction() creates valid transaction context
- ✅ Operations within transaction include transaction ID
- ✅ commit() persists all changes atomically
- ✅ rollback() discards all changes
- ✅ Automatic rollback on error works correctly
- ✅ Drop trait implements automatic rollback
- ✅ All property tests pass (Properties 22-26)
- ✅ Integration with DataClient complete

#### What Has Been Implemented So Far

**Completed Components:**
- ✅ Message protocol layer (Task 2)
- ✅ Connection management (Task 3)
- ✅ Authentication (Task 5)
- ✅ Data client for CRUD operations (Task 6)
- ✅ Query builder (Task 7)
- ✅ Checkpoint 8 - All tests passing

**Ready for Transaction Support:**
- Connection pooling can provide dedicated connections for transactions
- Message protocol supports Transaction message type
- Authentication provides valid tokens for transaction requests
- DataClient can be extended with transaction methods
- Error handling supports automatic retry and rollback logic

#### What Comes Next

After Task 9, the next tasks are:
- **Task 10: Implement admin client** - Cluster and user management operations
- **Task 11: Implement result handling** - Enhanced result processing and type conversion
- **Task 12: Implement error handling** - Comprehensive error types and retry policies

---

**Full requirements available in**: `.kiro/specs/client-sdk/FOUNDATION/requirements.md`
