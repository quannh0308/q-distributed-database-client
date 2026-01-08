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

### Task 7: Implement Query Builder

This task implements the QueryBuilder component that provides a fluent API for constructing type-safe database queries.

#### Design Overview

The QueryBuilder provides a safe, ergonomic way to construct SQL queries programmatically. It:

1. **Fluent API**: Provides method chaining for readable query construction
2. **SQL Injection Prevention**: Uses parameterized queries exclusively
3. **Type Safety**: Validates query structure at build time
4. **Performance**: Integrates with prepared statement caching
5. **Flexibility**: Supports SELECT, INSERT, UPDATE, DELETE operations

#### Component Design

**1. QueryBuilder Structure**

The main struct that builds queries:

```rust
pub struct QueryBuilder {
    query_type: QueryType,
    table: Option<String>,
    columns: Vec<String>,
    conditions: Vec<Condition>,
    params: Vec<Value>,
    values: Vec<Vec<Value>>,
    updates: Vec<(String, Value)>,
    joins: Vec<Join>,
    order_by: Vec<OrderBy>,
    limit: Option<u64>,
    offset: Option<u64>,
}

pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
}

pub struct Condition {
    pub clause: String,
    pub operator: LogicalOperator,
}

pub enum LogicalOperator {
    None,  // First condition
    And,
    Or,
}

pub struct Join {
    pub join_type: JoinType,
    pub table: String,
    pub on_clause: String,
}

pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

pub struct OrderBy {
    pub column: String,
    pub direction: OrderDirection,
}

pub enum OrderDirection {
    Asc,
    Desc,
}
```

**2. Query Construction Methods**

**SELECT Queries:**
```rust
impl QueryBuilder {
    pub fn select(columns: &[&str]) -> Self {
        Self {
            query_type: QueryType::Select,
            columns: columns.iter().map(|s| s.to_string()).collect(),
            table: None,
            conditions: Vec::new(),
            params: Vec::new(),
            values: Vec::new(),
            updates: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }
    
    pub fn from(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }
    
    pub fn where_clause(mut self, condition: &str, value: Value) -> Self {
        self.conditions.push(Condition {
            clause: condition.to_string(),
            operator: if self.conditions.is_empty() {
                LogicalOperator::None
            } else {
                LogicalOperator::And
            },
        });
        self.params.push(value);
        self
    }
    
    pub fn and(mut self, condition: &str, value: Value) -> Self {
        self.conditions.push(Condition {
            clause: condition.to_string(),
            operator: LogicalOperator::And,
        });
        self.params.push(value);
        self
    }
    
    pub fn or(mut self, condition: &str, value: Value) -> Self {
        self.conditions.push(Condition {
            clause: condition.to_string(),
            operator: LogicalOperator::Or,
        });
        self.params.push(value);
        self
    }
    
    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        self.order_by.push(OrderBy {
            column: column.to_string(),
            direction,
        });
        self
    }
    
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
    
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}
```

**INSERT Queries:**
```rust
impl QueryBuilder {
    pub fn insert_into(table: &str) -> Self {
        Self {
            query_type: QueryType::Insert,
            table: Some(table.to_string()),
            columns: Vec::new(),
            conditions: Vec::new(),
            params: Vec::new(),
            values: Vec::new(),
            updates: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }
    
    pub fn columns(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }
    
    pub fn values(mut self, values: &[Value]) -> Self {
        self.values.push(values.to_vec());
        self
    }
}
```

**UPDATE Queries:**
```rust
impl QueryBuilder {
    pub fn update(table: &str) -> Self {
        Self {
            query_type: QueryType::Update,
            table: Some(table.to_string()),
            columns: Vec::new(),
            conditions: Vec::new(),
            params: Vec::new(),
            values: Vec::new(),
            updates: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }
    
    pub fn set(mut self, column: &str, value: Value) -> Self {
        self.updates.push((column.to_string(), value));
        self
    }
}
```

**DELETE Queries:**
```rust
impl QueryBuilder {
    pub fn delete_from(table: &str) -> Self {
        Self {
            query_type: QueryType::Delete,
            table: Some(table.to_string()),
            columns: Vec::new(),
            conditions: Vec::new(),
            params: Vec::new(),
            values: Vec::new(),
            updates: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }
}
```

**3. SQL Generation**

The build() method generates SQL and parameters:

```rust
impl QueryBuilder {
    pub fn build(self) -> Result<(String, Vec<Value>)> {
        let sql = match self.query_type {
            QueryType::Select => self.build_select()?,
            QueryType::Insert => self.build_insert()?,
            QueryType::Update => self.build_update()?,
            QueryType::Delete => self.build_delete()?,
        };
        
        Ok((sql, self.params))
    }
    
    fn build_select(&self) -> Result<String> {
        let table = self.table.as_ref()
            .ok_or(DatabaseError::InvalidQuery { 
                message: "SELECT requires FROM clause".to_string() 
            })?;
        
        let columns = if self.columns.is_empty() {
            "*".to_string()
        } else {
            self.columns.join(", ")
        };
        
        let mut sql = format!("SELECT {} FROM {}", columns, table);
        
        // Add WHERE clause
        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            for (i, condition) in self.conditions.iter().enumerate() {
                if i > 0 {
                    match condition.operator {
                        LogicalOperator::And => sql.push_str(" AND "),
                        LogicalOperator::Or => sql.push_str(" OR "),
                        LogicalOperator::None => {},
                    }
                }
                sql.push_str(&condition.clause);
            }
        }
        
        // Add ORDER BY clause
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_clauses: Vec<String> = self.order_by.iter()
                .map(|o| format!("{} {}", o.column, 
                    match o.direction {
                        OrderDirection::Asc => "ASC",
                        OrderDirection::Desc => "DESC",
                    }))
                .collect();
            sql.push_str(&order_clauses.join(", "));
        }
        
        // Add LIMIT clause
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        // Add OFFSET clause
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
        
        Ok(sql)
    }
    
    fn build_insert(&self) -> Result<String> {
        let table = self.table.as_ref()
            .ok_or(DatabaseError::InvalidQuery { 
                message: "INSERT requires table name".to_string() 
            })?;
        
        if self.columns.is_empty() || self.values.is_empty() {
            return Err(DatabaseError::InvalidQuery {
                message: "INSERT requires columns and values".to_string()
            });
        }
        
        let columns = self.columns.join(", ");
        let placeholders = (0..self.columns.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table, columns, placeholders
        );
        
        Ok(sql)
    }
    
    fn build_update(&self) -> Result<String> {
        let table = self.table.as_ref()
            .ok_or(DatabaseError::InvalidQuery { 
                message: "UPDATE requires table name".to_string() 
            })?;
        
        if self.updates.is_empty() {
            return Err(DatabaseError::InvalidQuery {
                message: "UPDATE requires SET clause".to_string()
            });
        }
        
        let set_clauses: Vec<String> = self.updates.iter()
            .map(|(col, _)| format!("{} = ?", col))
            .collect();
        
        let mut sql = format!("UPDATE {} SET {}", table, set_clauses.join(", "));
        
        // Add WHERE clause
        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            for (i, condition) in self.conditions.iter().enumerate() {
                if i > 0 {
                    match condition.operator {
                        LogicalOperator::And => sql.push_str(" AND "),
                        LogicalOperator::Or => sql.push_str(" OR "),
                        LogicalOperator::None => {},
                    }
                }
                sql.push_str(&condition.clause);
            }
        }
        
        Ok(sql)
    }
    
    fn build_delete(&self) -> Result<String> {
        let table = self.table.as_ref()
            .ok_or(DatabaseError::InvalidQuery { 
                message: "DELETE requires table name".to_string() 
            })?;
        
        let mut sql = format!("DELETE FROM {}", table);
        
        // Add WHERE clause
        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            for (i, condition) in self.conditions.iter().enumerate() {
                if i > 0 {
                    match condition.operator {
                        LogicalOperator::And => sql.push_str(" AND "),
                        LogicalOperator::Or => sql.push_str(" OR "),
                        LogicalOperator::None => {},
                    }
                }
                sql.push_str(&condition.clause);
            }
        }
        
        Ok(sql)
    }
}
```

**4. SQL Injection Prevention**

The QueryBuilder prevents SQL injection through:

1. **Parameterized Queries**: All user values are passed as parameters, never concatenated into SQL
2. **Placeholder Substitution**: Uses `?` placeholders that are replaced by the database driver
3. **No String Concatenation**: SQL structure is built separately from user data
4. **Validation**: Validates query structure before building SQL

**Example of Safe Query Construction:**
```rust
// SAFE: User input is parameterized
let (sql, params) = QueryBuilder::select(&["*"])
    .from("users")
    .where_clause("username = ?", Value::String(user_input))
    .build()?;
// Generates: "SELECT * FROM users WHERE username = ?"
// Parameters: [user_input]

// UNSAFE (NOT DONE): Direct concatenation
// let sql = format!("SELECT * FROM users WHERE username = '{}'", user_input);
// This would allow SQL injection!
```

**5. Integration with DataClient**

The QueryBuilder integrates with DataClient for execution:

```rust
impl DataClient {
    pub async fn query_builder(&self, builder: QueryBuilder) -> Result<QueryResult> {
        let (sql, params) = builder.build()?;
        self.query_with_params(&sql, &params).await
    }
    
    pub async fn execute_builder(&self, builder: QueryBuilder) -> Result<ExecuteResult> {
        let (sql, params) = builder.build()?;
        self.execute_with_params(&sql, &params).await
    }
}
```

**6. Prepared Statement Caching**

The QueryBuilder works with prepared statement caching:

```rust
impl DataClient {
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
}

pub struct PreparedStatement {
    pub statement_id: StatementId,
    pub sql: String,
    pub param_count: usize,
}
```

#### Usage Examples

**SELECT Query:**
```rust
let (sql, params) = QueryBuilder::select(&["id", "name", "email"])
    .from("users")
    .where_clause("age > ?", Value::Int(18))
    .and("status = ?", Value::String("active".to_string()))
    .order_by("name", OrderDirection::Asc)
    .limit(10)
    .build()?;

let result = client.data().query_with_params(&sql, &params).await?;
```

**INSERT Query:**
```rust
let (sql, params) = QueryBuilder::insert_into("users")
    .columns(&["name", "email", "age"])
    .values(&[
        Value::String("Alice".to_string()),
        Value::String("alice@example.com".to_string()),
        Value::Int(25)
    ])
    .build()?;

let result = client.data().execute_with_params(&sql, &params).await?;
```

**UPDATE Query:**
```rust
let (sql, params) = QueryBuilder::update("users")
    .set("status", Value::String("inactive".to_string()))
    .set("updated_at", Value::Timestamp(Utc::now()))
    .where_clause("id = ?", Value::Int(123))
    .build()?;

let result = client.data().execute_with_params(&sql, &params).await?;
```

**DELETE Query:**
```rust
let (sql, params) = QueryBuilder::delete_from("users")
    .where_clause("status = ?", Value::String("deleted".to_string()))
    .and("created_at < ?", Value::Timestamp(cutoff_date))
    .build()?;

let result = client.data().execute_with_params(&sql, &params).await?;
```

#### Testing Strategy

**Unit Tests:**
- Test SELECT query generation
- Test INSERT query generation
- Test UPDATE query generation
- Test DELETE query generation
- Test WHERE clause construction
- Test ORDER BY clause construction
- Test LIMIT/OFFSET clauses
- Test error cases (missing table, missing columns, etc.)

**Property Tests:**

**Property 18: Query Builder Produces Valid SQL**
- Generate random valid query builder sequences
- Build SQL from each sequence
- Verify SQL is syntactically valid
- Verify all placeholders match parameter count

**Property 19: Condition Logic Correctness**
- Generate random combinations of AND/OR conditions
- Build SQL with conditions
- Verify logical operators are correctly placed
- Verify condition order is preserved

**Property 20: SQL Injection Prevention**
- Generate random strings with SQL special characters
- Use strings as parameter values
- Build and execute queries
- Verify strings are treated as data, not SQL code
- Verify no SQL syntax errors from special characters

#### Performance Considerations

**Query Building:**
- Minimal allocations during construction
- String building optimized with capacity hints
- Reuse of common query patterns

**Prepared Statements:**
- Cache prepared statements on client side
- Reduce server-side parsing overhead
- Reuse parsed query plans

**Parameter Binding:**
- Efficient parameter serialization
- Batch parameter binding when possible

#### Error Handling

**Query Building Errors:**
- `InvalidQuery`: Missing required clauses (FROM, SET, etc.)
- `TooManyParameters`: Parameter count exceeds limit
- `InvalidColumnName`: Column name contains invalid characters

**Error Handling Strategy:**
1. Validate query structure during build()
2. Return clear error messages indicating what's missing
3. Prevent invalid queries from being sent to server

#### Implementation Notes

**Immutability:**
- QueryBuilder methods consume self and return new instance
- Enables method chaining
- Prevents accidental mutation

**Type Safety:**
- QueryType enum ensures correct method usage
- Compile-time validation of query structure
- Prevents mixing incompatible operations

**Extensibility:**
- Easy to add new query types (MERGE, UPSERT)
- Easy to add new clauses (GROUP BY, HAVING)
- Easy to add new join types

---

**Full design with 42 correctness properties available in**: `.kiro/specs/client-sdk/FOUNDATION/design.md`
