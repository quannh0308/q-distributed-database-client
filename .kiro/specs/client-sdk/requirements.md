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

### Task 7: Implement Query Builder

This task implements the QueryBuilder component that provides a fluent API for constructing type-safe database queries with SQL injection prevention.

#### Query Builder Objectives

1. **Implement Fluent API**
   - Create QueryBuilder struct with method chaining
   - Implement select(), insert_into(), update(), delete_from() methods
   - Implement from(), where_clause(), and(), or() methods
   - Implement values(), set() for INSERT/UPDATE operations
   - Implement build() to generate SQL and parameters

2. **Ensure SQL Injection Prevention**
   - Always use parameterized queries
   - Never concatenate user input into SQL
   - Validate parameter binding
   - Prevent SQL injection through proper escaping

3. **Implement Prepared Statement Caching**
   - Cache prepared statements by SQL string
   - Reuse prepared statements for performance
   - Implement prepare() method in DataClient

#### Detailed Requirements

**Requirement 4: Query Building and Execution**

**User Story:** As a developer, I want to construct and execute database queries programmatically, so that I can retrieve data flexibly and safely from q-distributed-database.

**Acceptance Criteria:**

1. **Query Construction (4.1)**
   - WHEN building queries, THE Query_Builder SHALL provide a fluent API for constructing SELECT, INSERT, UPDATE, and DELETE statements compatible with q-distributed-database

2. **Condition Logic (4.2)**
   - WHEN adding conditions, THE Query_Builder SHALL support WHERE clauses with AND/OR logic

3. **SQL Injection Prevention (4.3)**
   - WHEN parameterizing queries, THE Query_Builder SHALL prevent SQL injection through parameter binding

4. **Query Execution (4.4)**
   - WHEN executing queries, THE Data_Client SHALL send the query to q-distributed-database and return results

5. **Error Handling (4.5)**
   - WHEN queries fail, THE Data_Client SHALL return detailed error information including error codes and messages

6. **Complex Queries (4.6)**
   - WHERE complex queries are needed, THE Query_Builder SHALL support JOINs, aggregations, and subqueries as supported by q-distributed-database

7. **OLTP Optimization (4.7)**
   - WHEN working with OLTP workloads, THE Query_Builder SHALL optimize for transactional queries

8. **OLAP Optimization (4.8)**
   - WHEN working with OLAP workloads, THE Query_Builder SHALL optimize for analytical queries

#### Implementation Details

**QueryBuilder Structure:**
```rust
pub struct QueryBuilder {
    query_type: QueryType,
    table: Option<String>,
    columns: Vec<String>,
    conditions: Vec<Condition>,
    params: Vec<Value>,
}
```

**Fluent API Methods:**
```rust
pub fn select(columns: &[&str]) -> Self;
pub fn insert_into(table: &str) -> Self;
pub fn update(table: &str) -> Self;
pub fn delete_from(table: &str) -> Self;

pub fn from(mut self, table: &str) -> Self;
pub fn where_clause(mut self, condition: &str, value: Value) -> Self;
pub fn and(mut self, condition: &str, value: Value) -> Self;
pub fn or(mut self, condition: &str, value: Value) -> Self;
pub fn values(mut self, values: &[Value]) -> Self;
pub fn set(mut self, column: &str, value: Value) -> Self;

pub fn build(self) -> Result<(String, Vec<Value>)>;
```

**Example Usage:**
```rust
let (sql, params) = QueryBuilder::select(&["id", "name", "email"])
    .from("users")
    .where_clause("age > ?", Value::Int(18))
    .and("status = ?", Value::String("active".to_string()))
    .build()?;
```

#### Success Criteria

- ✅ QueryBuilder struct implemented with fluent API
- ✅ select(), insert_into(), update(), delete_from() methods working
- ✅ from(), where_clause(), and(), or() methods working
- ✅ values(), set() methods working for INSERT/UPDATE
- ✅ build() method generates valid SQL with parameters
- ✅ SQL injection prevention through parameterization
- ✅ Prepared statement caching implemented
- ✅ All property tests passing (Properties 18-20)
- ✅ All unit tests passing
- ✅ Code compiles without errors

#### Property Tests for Task 7

**Property 18: Query Builder Produces Valid SQL**
*For any* valid sequence of query builder method calls, the resulting SQL should be syntactically valid.
**Validates: Requirements 4.1**

**Property 19: Condition Logic Correctness**
*For any* query with AND/OR conditions, the generated SQL should correctly represent the logical combination.
**Validates: Requirements 4.2**

**Property 20: SQL Injection Prevention**
*For any* parameter value containing SQL special characters, the parameterized query should treat it as data, not SQL code.
**Validates: Requirements 4.3**

---

**Full requirements available in**: `.kiro/specs/client-sdk/FOUNDATION/requirements.md`
