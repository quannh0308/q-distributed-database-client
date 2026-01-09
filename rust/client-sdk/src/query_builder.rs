//! Query builder module for constructing type-safe SQL queries
//!
//! This module provides a fluent API for building SQL queries with automatic
//! SQL injection prevention through parameterization.

use crate::error::DatabaseError;
use crate::types::Value;
use crate::Result;

/// Type of SQL query
#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    /// SELECT query
    Select,
    /// INSERT query
    Insert,
    /// UPDATE query
    Update,
    /// DELETE query
    Delete,
}

/// Logical operator for combining conditions
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOperator {
    /// No operator (first condition)
    None,
    /// AND operator
    And,
    /// OR operator
    Or,
}

/// A condition in a WHERE clause
#[derive(Debug, Clone)]
pub struct Condition {
    /// The condition clause (e.g., "age > ?")
    pub clause: String,
    /// The logical operator connecting this condition to the previous one
    pub operator: LogicalOperator,
}

/// Sort direction for ORDER BY clause
#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

/// An ORDER BY clause
#[derive(Debug, Clone)]
pub struct OrderBy {
    /// Column name
    pub column: String,
    /// Sort direction
    pub direction: OrderDirection,
}

/// Query builder for constructing SQL queries with a fluent API
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    /// Type of query
    query_type: QueryType,
    /// Table name
    table: Option<String>,
    /// Column names (for SELECT or INSERT)
    columns: Vec<String>,
    /// WHERE conditions
    conditions: Vec<Condition>,
    /// Query parameters
    params: Vec<Value>,
    /// Values for INSERT (can have multiple rows)
    values: Vec<Vec<Value>>,
    /// UPDATE assignments (column, value)
    updates: Vec<(String, Value)>,
    /// ORDER BY clauses
    order_by: Vec<OrderBy>,
    /// LIMIT clause
    limit: Option<u64>,
    /// OFFSET clause
    offset: Option<u64>,
}

impl QueryBuilder {
    /// Creates a new SELECT query
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::select(&["id", "name", "email"]);
    /// ```
    pub fn select(columns: &[&str]) -> Self {
        Self {
            query_type: QueryType::Select,
            columns: columns.iter().map(|s| s.to_string()).collect(),
            table: None,
            conditions: Vec::new(),
            params: Vec::new(),
            values: Vec::new(),
            updates: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Creates a new INSERT query
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::insert_into("users");
    /// ```
    pub fn insert_into(table: &str) -> Self {
        Self {
            query_type: QueryType::Insert,
            table: Some(table.to_string()),
            columns: Vec::new(),
            conditions: Vec::new(),
            params: Vec::new(),
            values: Vec::new(),
            updates: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Creates a new UPDATE query
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::update("users");
    /// ```
    pub fn update(table: &str) -> Self {
        Self {
            query_type: QueryType::Update,
            table: Some(table.to_string()),
            columns: Vec::new(),
            conditions: Vec::new(),
            params: Vec::new(),
            values: Vec::new(),
            updates: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Creates a new DELETE query
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::delete_from("users");
    /// ```
    pub fn delete_from(table: &str) -> Self {
        Self {
            query_type: QueryType::Delete,
            table: Some(table.to_string()),
            columns: Vec::new(),
            conditions: Vec::new(),
            params: Vec::new(),
            values: Vec::new(),
            updates: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Sets the table name for SELECT queries
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::select(&["*"]).from("users");
    /// ```
    pub fn from(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    /// Adds a WHERE condition
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::select(&["*"])
    ///     .from("users")
    ///     .where_clause("age > ?", Value::Int(18));
    /// ```
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

    /// Adds an AND condition
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::select(&["*"])
    ///     .from("users")
    ///     .where_clause("age > ?", Value::Int(18))
    ///     .and("status = ?", Value::String("active".to_string()));
    /// ```
    pub fn and(mut self, condition: &str, value: Value) -> Self {
        self.conditions.push(Condition {
            clause: condition.to_string(),
            operator: LogicalOperator::And,
        });
        self.params.push(value);
        self
    }

    /// Adds an OR condition
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::select(&["*"])
    ///     .from("users")
    ///     .where_clause("age > ?", Value::Int(18))
    ///     .or("status = ?", Value::String("admin".to_string()));
    /// ```
    pub fn or(mut self, condition: &str, value: Value) -> Self {
        self.conditions.push(Condition {
            clause: condition.to_string(),
            operator: LogicalOperator::Or,
        });
        self.params.push(value);
        self
    }

    /// Sets the column list for INSERT queries
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::insert_into("users")
    ///     .columns(&["name", "email", "age"]);
    /// ```
    pub fn columns(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Adds values for INSERT queries
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::insert_into("users")
    ///     .columns(&["name", "email"])
    ///     .values(&[Value::String("Alice".to_string()), Value::String("alice@example.com".to_string())]);
    /// ```
    pub fn values(mut self, values: &[Value]) -> Self {
        self.values.push(values.to_vec());
        self
    }

    /// Adds a SET clause for UPDATE queries
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::update("users")
    ///     .set("status", Value::String("inactive".to_string()));
    /// ```
    pub fn set(mut self, column: &str, value: Value) -> Self {
        self.updates.push((column.to_string(), value));
        self
    }

    /// Adds an ORDER BY clause
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::select(&["*"])
    ///     .from("users")
    ///     .order_by("name", OrderDirection::Asc);
    /// ```
    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        self.order_by.push(OrderBy {
            column: column.to_string(),
            direction,
        });
        self
    }

    /// Sets the LIMIT clause
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::select(&["*"])
    ///     .from("users")
    ///     .limit(10);
    /// ```
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the OFFSET clause
    ///
    /// # Example
    /// ```ignore
    /// let query = QueryBuilder::select(&["*"])
    ///     .from("users")
    ///     .limit(10)
    ///     .offset(20);
    /// ```
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Builds the SQL query and returns it with parameters
    ///
    /// # Returns
    /// A tuple of (SQL string, parameters vector)
    ///
    /// # Errors
    /// Returns an error if the query is invalid (e.g., missing required clauses)
    pub fn build(self) -> Result<(String, Vec<Value>)> {
        let sql = match self.query_type {
            QueryType::Select => self.build_select()?,
            QueryType::Insert => self.build_insert()?,
            QueryType::Update => self.build_update()?,
            QueryType::Delete => self.build_delete()?,
        };

        // Collect all parameters in the correct order
        let mut all_params = Vec::new();
        
        // For UPDATE: SET parameters come first, then WHERE parameters
        if self.query_type == QueryType::Update {
            for (_, value) in &self.updates {
                all_params.push(value.clone());
            }
            all_params.extend(self.params.clone());
        }
        // For INSERT: values come from the values field
        else if self.query_type == QueryType::Insert {
            for row in &self.values {
                all_params.extend(row.clone());
            }
        }
        // For SELECT and DELETE: just use params (WHERE conditions)
        else {
            all_params = self.params.clone();
        }

        Ok((sql, all_params))
    }

    /// Builds a SELECT query
    fn build_select(&self) -> Result<String> {
        let table = self.table.as_ref().ok_or(DatabaseError::InternalError {
            component: "QueryBuilder".to_string(),
            details: "SELECT requires FROM clause".to_string(),
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
                        LogicalOperator::None => {}
                    }
                }
                sql.push_str(&condition.clause);
            }
        }

        // Add ORDER BY clause
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_clauses: Vec<String> = self
                .order_by
                .iter()
                .map(|o| {
                    format!(
                        "{} {}",
                        o.column,
                        match o.direction {
                            OrderDirection::Asc => "ASC",
                            OrderDirection::Desc => "DESC",
                        }
                    )
                })
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

    /// Builds an INSERT query
    fn build_insert(&self) -> Result<String> {
        let table = self.table.as_ref().ok_or(DatabaseError::InternalError {
            component: "QueryBuilder".to_string(),
            details: "INSERT requires table name".to_string(),
        })?;

        if self.columns.is_empty() || self.values.is_empty() {
            return Err(DatabaseError::InternalError {
                component: "QueryBuilder".to_string(),
                details: "INSERT requires columns and values".to_string(),
            });
        }

        let columns = self.columns.join(", ");
        let placeholders = (0..self.columns.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!("INSERT INTO {} ({}) VALUES ({})", table, columns, placeholders);

        Ok(sql)
    }

    /// Builds an UPDATE query
    fn build_update(&self) -> Result<String> {
        let table = self.table.as_ref().ok_or(DatabaseError::InternalError {
            component: "QueryBuilder".to_string(),
            details: "UPDATE requires table name".to_string(),
        })?;

        if self.updates.is_empty() {
            return Err(DatabaseError::InternalError {
                component: "QueryBuilder".to_string(),
                details: "UPDATE requires SET clause".to_string(),
            });
        }

        let set_clauses: Vec<String> = self.updates.iter().map(|(col, _)| format!("{} = ?", col)).collect();

        let mut sql = format!("UPDATE {} SET {}", table, set_clauses.join(", "));

        // Add WHERE clause
        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            for (i, condition) in self.conditions.iter().enumerate() {
                if i > 0 {
                    match condition.operator {
                        LogicalOperator::And => sql.push_str(" AND "),
                        LogicalOperator::Or => sql.push_str(" OR "),
                        LogicalOperator::None => {}
                    }
                }
                sql.push_str(&condition.clause);
            }
        }

        Ok(sql)
    }

    /// Builds a DELETE query
    fn build_delete(&self) -> Result<String> {
        let table = self.table.as_ref().ok_or(DatabaseError::InternalError {
            component: "QueryBuilder".to_string(),
            details: "DELETE requires table name".to_string(),
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
                        LogicalOperator::None => {}
                    }
                }
                sql.push_str(&condition.clause);
            }
        }

        Ok(sql)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_basic() {
        let (sql, params) = QueryBuilder::select(&["id", "name", "email"])
            .from("users")
            .build()
            .unwrap();

        assert_eq!(sql, "SELECT id, name, email FROM users");
        assert!(params.is_empty());
    }

    #[test]
    fn test_select_all_columns() {
        let (sql, params) = QueryBuilder::select(&[])
            .from("users")
            .build()
            .unwrap();

        assert_eq!(sql, "SELECT * FROM users");
        assert!(params.is_empty());
    }

    #[test]
    fn test_select_with_where() {
        let (sql, params) = QueryBuilder::select(&["*"])
            .from("users")
            .where_clause("age > ?", Value::Int(18))
            .build()
            .unwrap();

        assert_eq!(sql, "SELECT * FROM users WHERE age > ?");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::Int(18));
    }

    #[test]
    fn test_select_with_and() {
        let (sql, params) = QueryBuilder::select(&["*"])
            .from("users")
            .where_clause("age > ?", Value::Int(18))
            .and("status = ?", Value::String("active".to_string()))
            .build()
            .unwrap();

        assert_eq!(sql, "SELECT * FROM users WHERE age > ? AND status = ?");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], Value::Int(18));
        assert_eq!(params[1], Value::String("active".to_string()));
    }

    #[test]
    fn test_select_with_or() {
        let (sql, params) = QueryBuilder::select(&["*"])
            .from("users")
            .where_clause("age > ?", Value::Int(18))
            .or("status = ?", Value::String("admin".to_string()))
            .build()
            .unwrap();

        assert_eq!(sql, "SELECT * FROM users WHERE age > ? OR status = ?");
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_select_with_order_by() {
        let (sql, _) = QueryBuilder::select(&["*"])
            .from("users")
            .order_by("name", OrderDirection::Asc)
            .build()
            .unwrap();

        assert_eq!(sql, "SELECT * FROM users ORDER BY name ASC");
    }

    #[test]
    fn test_select_with_limit() {
        let (sql, _) = QueryBuilder::select(&["*"])
            .from("users")
            .limit(10)
            .build()
            .unwrap();

        assert_eq!(sql, "SELECT * FROM users LIMIT 10");
    }

    #[test]
    fn test_select_with_offset() {
        let (sql, _) = QueryBuilder::select(&["*"])
            .from("users")
            .limit(10)
            .offset(20)
            .build()
            .unwrap();

        assert_eq!(sql, "SELECT * FROM users LIMIT 10 OFFSET 20");
    }

    #[test]
    fn test_insert_basic() {
        let (sql, params) = QueryBuilder::insert_into("users")
            .columns(&["name", "email"])
            .values(&[
                Value::String("Alice".to_string()),
                Value::String("alice@example.com".to_string()),
            ])
            .build()
            .unwrap();

        assert_eq!(sql, "INSERT INTO users (name, email) VALUES (?, ?)");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], Value::String("Alice".to_string()));
        assert_eq!(params[1], Value::String("alice@example.com".to_string()));
    }

    #[test]
    fn test_update_basic() {
        let (sql, params) = QueryBuilder::update("users")
            .set("status", Value::String("inactive".to_string()))
            .where_clause("id = ?", Value::Int(123))
            .build()
            .unwrap();

        assert_eq!(sql, "UPDATE users SET status = ? WHERE id = ?");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], Value::String("inactive".to_string()));
        assert_eq!(params[1], Value::Int(123));
    }

    #[test]
    fn test_delete_basic() {
        let (sql, params) = QueryBuilder::delete_from("users")
            .where_clause("status = ?", Value::String("deleted".to_string()))
            .build()
            .unwrap();

        assert_eq!(sql, "DELETE FROM users WHERE status = ?");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::String("deleted".to_string()));
    }

    #[test]
    fn test_select_missing_from() {
        let result = QueryBuilder::select(&["*"]).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_missing_columns() {
        let result = QueryBuilder::insert_into("users").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_missing_set() {
        let result = QueryBuilder::update("users").build();
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Helper to generate valid SQL identifiers
    fn sql_identifier() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,10}".prop_map(|s| s.to_string())
    }

    // Helper to generate Value instances
    fn value_strategy() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(Value::Int),
            any::<f64>().prop_map(Value::Float),
            ".*".prop_map(|s| Value::String(s)),
        ]
    }

    // Property 18: Query Builder Produces Valid SQL
    // **Validates: Requirements 4.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_query_builder_produces_valid_sql(
            table in sql_identifier(),
            columns in prop::collection::vec(sql_identifier(), 1..5),
            num_conditions in 0usize..5,
        ) {
            // Build a SELECT query
            let mut builder = QueryBuilder::select(&columns.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .from(&table);
            
            // Add random conditions
            for i in 0..num_conditions {
                let condition = format!("col{} = ?", i);
                let value = Value::Int(i as i64);
                if i == 0 {
                    builder = builder.where_clause(&condition, value);
                } else {
                    builder = builder.and(&condition, value);
                }
            }
            
            // Build the query
            let result = builder.build();
            prop_assert!(result.is_ok());
            
            let (sql, params) = result.unwrap();
            
            // Verify SQL is not empty
            prop_assert!(!sql.is_empty());
            
            // Verify SQL starts with SELECT
            prop_assert!(sql.starts_with("SELECT"));
            
            // Verify SQL contains FROM
            prop_assert!(sql.contains("FROM"));
            
            // Verify placeholder count matches parameter count
            let placeholder_count = sql.matches('?').count();
            prop_assert_eq!(placeholder_count, params.len());
            
            // Verify parameter count matches number of conditions
            prop_assert_eq!(params.len(), num_conditions);
        }
    }

    // Property 19: Condition Logic Correctness
    // **Validates: Requirements 4.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_condition_logic_correctness(
            table in sql_identifier(),
            num_and_conditions in 1usize..5,
            num_or_conditions in 0usize..3,
        ) {
            // Build a query with AND and OR conditions
            let mut builder = QueryBuilder::select(&["*"]).from(&table);
            
            // Add AND conditions
            for i in 0..num_and_conditions {
                let condition = format!("col{} = ?", i);
                let value = Value::Int(i as i64);
                if i == 0 {
                    builder = builder.where_clause(&condition, value);
                } else {
                    builder = builder.and(&condition, value);
                }
            }
            
            // Add OR conditions
            for i in 0..num_or_conditions {
                let condition = format!("or_col{} = ?", i);
                let value = Value::Int(i as i64);
                builder = builder.or(&condition, value);
            }
            
            let (sql, params) = builder.build().unwrap();
            
            // Verify AND operators are present
            if num_and_conditions > 1 {
                let and_count = sql.matches(" AND ").count();
                prop_assert_eq!(and_count, num_and_conditions - 1);
            }
            
            // Verify OR operators are present
            if num_or_conditions > 0 {
                let or_count = sql.matches(" OR ").count();
                prop_assert_eq!(or_count, num_or_conditions);
            }
            
            // Verify total parameter count
            prop_assert_eq!(params.len(), num_and_conditions + num_or_conditions);
            
            // Verify WHERE clause exists
            prop_assert!(sql.contains("WHERE"));
        }
    }

    // Property 20: SQL Injection Prevention
    // **Validates: Requirements 4.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_sql_injection_prevention(
            table in sql_identifier(),
            malicious_input in ".*[';\"\\-\\\\].*",
        ) {
            // Try to inject SQL through parameter values
            let builder = QueryBuilder::select(&["*"])
                .from(&table)
                .where_clause("name = ?", Value::String(malicious_input.clone()));
            
            let (sql, params) = builder.build().unwrap();
            
            // Verify the malicious input is NOT in the SQL string
            // It should only be in the parameters
            prop_assert!(!sql.contains(&malicious_input));
            
            // Verify the parameter contains the malicious input
            prop_assert_eq!(params.len(), 1);
            if let Value::String(s) = &params[0] {
                prop_assert_eq!(s, &malicious_input);
            } else {
                prop_assert!(false, "Expected String parameter");
            }
            
            // Verify SQL structure is intact
            prop_assert!(sql.starts_with("SELECT"));
            prop_assert!(sql.contains("WHERE"));
            prop_assert!(sql.contains("= ?"));
            
            // Verify only one placeholder
            prop_assert_eq!(sql.matches('?').count(), 1);
        }
    }

    // Additional property test for INSERT queries
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_insert_query_valid(
            table in sql_identifier(),
            columns in prop::collection::vec(sql_identifier(), 1..5),
            values in prop::collection::vec(value_strategy(), 1..5),
        ) {
            // Ensure columns and values have the same length
            let min_len = columns.len().min(values.len());
            let columns = &columns[..min_len];
            let values = &values[..min_len];
            
            let builder = QueryBuilder::insert_into(&table)
                .columns(&columns.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .values(values);
            
            let result = builder.build();
            prop_assert!(result.is_ok());
            
            let (sql, params) = result.unwrap();
            
            // Verify SQL structure
            prop_assert!(sql.starts_with("INSERT INTO"));
            prop_assert!(sql.contains("VALUES"));
            
            // Verify placeholder count matches value count
            let placeholder_count = sql.matches('?').count();
            prop_assert_eq!(placeholder_count, min_len);
            prop_assert_eq!(params.len(), min_len);
        }
    }

    // Additional property test for UPDATE queries
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_update_query_valid(
            table in sql_identifier(),
            set_columns in prop::collection::vec(sql_identifier(), 1..5),
            set_values in prop::collection::vec(value_strategy(), 1..5),
            where_value in value_strategy(),
        ) {
            // Ensure columns and values have the same length
            let min_len = set_columns.len().min(set_values.len());
            
            let mut builder = QueryBuilder::update(&table);
            
            // Add SET clauses
            for i in 0..min_len {
                builder = builder.set(&set_columns[i], set_values[i].clone());
            }
            
            // Add WHERE clause
            builder = builder.where_clause("id = ?", where_value);
            
            let result = builder.build();
            prop_assert!(result.is_ok());
            
            let (sql, params) = result.unwrap();
            
            // Verify SQL structure
            prop_assert!(sql.starts_with("UPDATE"));
            prop_assert!(sql.contains("SET"));
            prop_assert!(sql.contains("WHERE"));
            
            // Verify parameter count (SET values + WHERE value)
            prop_assert_eq!(params.len(), min_len + 1);
        }
    }

    // Additional property test for DELETE queries
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_delete_query_valid(
            table in sql_identifier(),
            num_conditions in 1usize..5,
        ) {
            let mut builder = QueryBuilder::delete_from(&table);
            
            // Add conditions
            for i in 0..num_conditions {
                let condition = format!("col{} = ?", i);
                let value = Value::Int(i as i64);
                if i == 0 {
                    builder = builder.where_clause(&condition, value);
                } else {
                    builder = builder.and(&condition, value);
                }
            }
            
            let result = builder.build();
            prop_assert!(result.is_ok());
            
            let (sql, params) = result.unwrap();
            
            // Verify SQL structure
            prop_assert!(sql.starts_with("DELETE FROM"));
            prop_assert!(sql.contains("WHERE"));
            
            // Verify parameter count
            prop_assert_eq!(params.len(), num_conditions);
        }
    }
}
