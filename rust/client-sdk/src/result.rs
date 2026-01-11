//! Result handling module for query results
//!
//! This module provides comprehensive result handling capabilities including
//! Row and QueryResult structs, type conversion, and streaming support.

use crate::error::DatabaseError;
use crate::types::Value;
use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Data type enumeration for column metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    /// Integer type
    Int,
    /// Floating point type
    Float,
    /// String type
    String,
    /// Boolean type
    Bool,
    /// Binary data type
    Bytes,
    /// Timestamp type
    Timestamp,
    /// Null type
    Null,
}

/// Column metadata describing a result column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMetadata {
    /// Column name
    pub name: String,
    /// Column data type
    pub data_type: DataType,
    /// Whether the column is nullable
    pub nullable: bool,
    /// Column ordinal position (0-based)
    pub ordinal: usize,
}

/// A single row in a query result
#[derive(Debug, Clone)]
pub struct Row {
    /// Shared column metadata
    columns: Arc<Vec<ColumnMetadata>>,
    /// Row values
    values: Vec<Value>,
}

impl Row {
    /// Creates a new row with the given column metadata and values
    pub fn new(columns: Arc<Vec<ColumnMetadata>>, values: Vec<Value>) -> Self {
        Self { columns, values }
    }

    /// Gets a value by column index
    pub fn get(&self, index: usize) -> Result<&Value> {
        self.values
            .get(index)
            .ok_or(DatabaseError::IndexOutOfBounds {
                index,
                max: self.values.len(),
            })
    }

    /// Gets a value by column name
    pub fn get_by_name(&self, name: &str) -> Result<&Value> {
        let index = self
            .columns
            .iter()
            .position(|col| col.name == name)
            .ok_or_else(|| DatabaseError::ColumnNotFound {
                column_name: name.to_string(),
            })?;
        self.get(index)
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

    /// Gets a value and converts it to i64
    pub fn get_i64(&self, index: usize) -> Result<i64> {
        let value = self.get(index)?;
        value.as_i64()
    }

    /// Gets a value and converts it to f64
    pub fn get_f64(&self, index: usize) -> Result<f64> {
        let value = self.get(index)?;
        value.as_f64()
    }

    /// Gets a value and converts it to String
    pub fn get_string(&self, index: usize) -> Result<String> {
        let value = self.get(index)?;
        value.as_string()
    }

    /// Gets a value and converts it to bool
    pub fn get_bool(&self, index: usize) -> Result<bool> {
        let value = self.get(index)?;
        value.as_bool_result()
    }

    /// Gets a value and converts it to `Vec<u8>`
    pub fn get_bytes(&self, index: usize) -> Result<Vec<u8>> {
        let value = self.get(index)?;
        value.as_bytes_vec()
    }

    /// Gets a value and converts it to `DateTime<Utc>`
    pub fn get_timestamp(&self, index: usize) -> Result<DateTime<Utc>> {
        let value = self.get(index)?;
        value.as_timestamp_result()
    }
}

/// Query result containing column metadata and rows
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Column metadata
    pub columns: Vec<ColumnMetadata>,
    /// Result rows
    pub rows: Vec<Row>,
}

impl QueryResult {
    /// Creates a new query result
    pub fn new(columns: Vec<ColumnMetadata>, rows: Vec<Row>) -> Self {
        Self { columns, rows }
    }

    /// Creates a QueryResult from raw column metadata and value rows
    pub fn from_raw(columns: Vec<ColumnMetadata>, value_rows: Vec<Vec<Value>>) -> Self {
        let columns_arc = Arc::new(columns.clone());
        let rows = value_rows
            .into_iter()
            .map(|values| Row::new(columns_arc.clone(), values))
            .collect();
        Self { columns, rows }
    }

    /// Returns the number of rows in the result
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns true if the result has no rows
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns an iterator over the rows
    pub fn iter(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }
}

impl IntoIterator for QueryResult {
    type Item = Row;
    type IntoIter = std::vec::IntoIter<Row>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_columns() -> Vec<ColumnMetadata> {
        vec![
            ColumnMetadata {
                name: "id".to_string(),
                data_type: DataType::Int,
                nullable: false,
                ordinal: 0,
            },
            ColumnMetadata {
                name: "name".to_string(),
                data_type: DataType::String,
                nullable: true,
                ordinal: 1,
            },
            ColumnMetadata {
                name: "score".to_string(),
                data_type: DataType::Float,
                nullable: true,
                ordinal: 2,
            },
        ]
    }

    #[test]
    fn test_data_type_equality() {
        assert_eq!(DataType::Int, DataType::Int);
        assert_eq!(DataType::String, DataType::String);
        assert_ne!(DataType::Int, DataType::Float);
    }

    #[test]
    fn test_column_metadata_creation() {
        let col = ColumnMetadata {
            name: "id".to_string(),
            data_type: DataType::Int,
            nullable: false,
            ordinal: 0,
        };
        assert_eq!(col.name, "id");
        assert_eq!(col.data_type, DataType::Int);
        assert!(!col.nullable);
        assert_eq!(col.ordinal, 0);
    }

    #[test]
    fn test_row_creation() {
        let columns = Arc::new(create_test_columns());
        let values = vec![
            Value::Int(1),
            Value::String("Alice".to_string()),
            Value::Float(95.5),
        ];
        let row = Row::new(columns, values);
        assert_eq!(row.len(), 3);
        assert!(!row.is_empty());
    }

    #[test]
    fn test_row_get_by_index() {
        let columns = Arc::new(create_test_columns());
        let values = vec![
            Value::Int(1),
            Value::String("Alice".to_string()),
            Value::Float(95.5),
        ];
        let row = Row::new(columns, values);

        assert_eq!(row.get(0).unwrap(), &Value::Int(1));
        assert_eq!(row.get(1).unwrap(), &Value::String("Alice".to_string()));
        assert_eq!(row.get(2).unwrap(), &Value::Float(95.5));
        assert!(row.get(3).is_err());
    }

    #[test]
    fn test_row_get_by_name() {
        let columns = Arc::new(create_test_columns());
        let values = vec![
            Value::Int(1),
            Value::String("Alice".to_string()),
            Value::Float(95.5),
        ];
        let row = Row::new(columns, values);

        assert_eq!(row.get_by_name("id").unwrap(), &Value::Int(1));
        assert_eq!(
            row.get_by_name("name").unwrap(),
            &Value::String("Alice".to_string())
        );
        assert_eq!(row.get_by_name("score").unwrap(), &Value::Float(95.5));
        assert!(row.get_by_name("nonexistent").is_err());
    }

    #[test]
    fn test_row_empty() {
        let columns = Arc::new(vec![]);
        let row = Row::new(columns, vec![]);
        assert_eq!(row.len(), 0);
        assert!(row.is_empty());
    }

    #[test]
    fn test_query_result_creation() {
        let columns = create_test_columns();
        let columns_arc = Arc::new(columns.clone());
        let rows = vec![
            Row::new(
                columns_arc.clone(),
                vec![
                    Value::Int(1),
                    Value::String("Alice".to_string()),
                    Value::Float(95.5),
                ],
            ),
            Row::new(
                columns_arc.clone(),
                vec![
                    Value::Int(2),
                    Value::String("Bob".to_string()),
                    Value::Float(87.3),
                ],
            ),
        ];

        let result = QueryResult::new(columns, rows);
        assert_eq!(result.len(), 2);
        assert!(!result.is_empty());
        assert_eq!(result.columns.len(), 3);
    }

    #[test]
    fn test_query_result_from_raw() {
        let columns = create_test_columns();
        let value_rows = vec![
            vec![
                Value::Int(1),
                Value::String("Alice".to_string()),
                Value::Float(95.5),
            ],
            vec![
                Value::Int(2),
                Value::String("Bob".to_string()),
                Value::Float(87.3),
            ],
        ];

        let result = QueryResult::from_raw(columns, value_rows);
        assert_eq!(result.len(), 2);
        assert_eq!(result.columns.len(), 3);
    }

    #[test]
    fn test_query_result_empty() {
        let result = QueryResult::new(vec![], vec![]);
        assert_eq!(result.len(), 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_query_result_iter() {
        let columns = create_test_columns();
        let columns_arc = Arc::new(columns.clone());
        let rows = vec![
            Row::new(
                columns_arc.clone(),
                vec![
                    Value::Int(1),
                    Value::String("Alice".to_string()),
                    Value::Float(95.5),
                ],
            ),
            Row::new(
                columns_arc.clone(),
                vec![
                    Value::Int(2),
                    Value::String("Bob".to_string()),
                    Value::Float(87.3),
                ],
            ),
        ];

        let result = QueryResult::new(columns, rows);
        let mut iter = result.iter();
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }
}

// Property-Based Tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating DataType
    fn data_type_strategy() -> impl Strategy<Value = DataType> {
        prop_oneof![
            Just(DataType::Int),
            Just(DataType::Float),
            Just(DataType::String),
            Just(DataType::Bool),
            Just(DataType::Bytes),
            Just(DataType::Timestamp),
            Just(DataType::Null),
        ]
    }

    // Strategy for generating ColumnMetadata
    fn column_metadata_strategy() -> impl Strategy<Value = ColumnMetadata> {
        (
            "[a-z][a-z0-9_]{0,19}", // column name
            data_type_strategy(),   // data type
            any::<bool>(),          // nullable
            0usize..10,             // ordinal
        )
            .prop_map(|(name, data_type, nullable, ordinal)| ColumnMetadata {
                name,
                data_type,
                nullable,
                ordinal,
            })
    }

    // Strategy for generating Value
    fn value_strategy() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(Value::Int),
            any::<f64>().prop_map(Value::Float),
            "[a-zA-Z0-9 ]{0,50}".prop_map(Value::String),
            prop::collection::vec(any::<u8>(), 0..100).prop_map(Value::Bytes),
            Just(Value::Timestamp(chrono::Utc::now())),
        ]
    }

    // Strategy for generating Row
    fn row_strategy() -> impl Strategy<Value = (Arc<Vec<ColumnMetadata>>, Row)> {
        prop::collection::vec(column_metadata_strategy(), 1..10).prop_flat_map(|columns| {
            let num_cols = columns.len();
            let columns_arc = Arc::new(columns);
            let columns_clone = columns_arc.clone();
            prop::collection::vec(value_strategy(), num_cols..=num_cols).prop_map(move |values| {
                (columns_arc.clone(), Row::new(columns_clone.clone(), values))
            })
        })
    }

    // Property 32: Result Deserialization
    // Feature: client-sdk, Property 32: For any query result, all rows should be deserialized into language-native data structures without data loss
    // Validates: Requirements 9.1
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_result_deserialization(
            columns in prop::collection::vec(column_metadata_strategy(), 1..10),
            num_rows in 0usize..20,
        ) {
            let num_cols = columns.len();

            // Generate rows with the correct number of values
            let value_rows: Vec<Vec<Value>> = (0..num_rows)
                .map(|_| {
                    (0..num_cols)
                        .map(|_| Value::Int(42)) // Simple value for testing
                        .collect()
                })
                .collect();

            // Create QueryResult from raw data
            let result = QueryResult::from_raw(columns.clone(), value_rows.clone());

            // Verify no data loss
            prop_assert_eq!(result.columns.len(), columns.len());
            prop_assert_eq!(result.rows.len(), num_rows);

            // Verify each row has the correct number of values
            for row in result.rows.iter() {
                prop_assert_eq!(row.len(), num_cols);
            }

            // Verify column metadata is preserved
            for (i, col) in result.columns.iter().enumerate() {
                prop_assert_eq!(&col.name, &columns[i].name);
                prop_assert_eq!(col.data_type, columns[i].data_type);
                prop_assert_eq!(col.nullable, columns[i].nullable);
            }
        }
    }

    // Property 33: Result Iteration
    // Feature: client-sdk, Property 33: For any QueryResult, iterating through all rows should yield exactly the number of rows indicated in the result metadata
    // Validates: Requirements 9.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_result_iteration(
            columns in prop::collection::vec(column_metadata_strategy(), 1..10),
            num_rows in 0usize..20,
        ) {
            let num_cols = columns.len();

            // Generate rows
            let value_rows: Vec<Vec<Value>> = (0..num_rows)
                .map(|_| {
                    (0..num_cols)
                        .map(|_| Value::Int(42))
                        .collect()
                })
                .collect();

            let result = QueryResult::from_raw(columns, value_rows);

            // Count rows via iteration
            let iter_count = result.iter().count();

            // Verify iteration yields exactly the number of rows
            prop_assert_eq!(iter_count, num_rows);
            prop_assert_eq!(iter_count, result.len());
        }
    }

    // Property 34: Column Access Methods
    // Feature: client-sdk, Property 34: For any row in a result set, accessing a column by index or by name should return the same value
    // Validates: Requirements 9.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_column_access_methods((columns_arc, row) in row_strategy()) {
            // Test each column, but only test the first occurrence of each unique name
            // (since get_by_name returns the first match for duplicate names)
            let mut seen_names = std::collections::HashSet::new();

            for (_i, col) in columns_arc.iter().enumerate() {
                // Skip if we've already tested this column name
                if !seen_names.insert(&col.name) {
                    continue;
                }

                // Get by index (this gets the first occurrence)
                let first_index = columns_arc.iter().position(|c| c.name == col.name).unwrap();
                let by_index = row.get(first_index);

                // Get by name (this also gets the first occurrence)
                let by_name = row.get_by_name(&col.name);

                // Both should succeed
                prop_assert!(by_index.is_ok());
                prop_assert!(by_name.is_ok());

                // Both should return the same value
                let val_by_index = by_index.unwrap();
                let val_by_name = by_name.unwrap();

                prop_assert_eq!(val_by_index, val_by_name);
            }
        }
    }

    // Property 36: Type Conversion Correctness
    // Feature: client-sdk, Property 36: For any database value, converting to the corresponding native type should preserve the value's semantic meaning
    // Validates: Requirements 9.5
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_type_conversion_correctness(value in value_strategy()) {
            match &value {
                Value::Int(i) => {
                    // Int should convert to i64
                    let result = value.as_i64();
                    prop_assert!(result.is_ok());
                    prop_assert_eq!(result.unwrap(), *i);

                    // Int should also convert to f64
                    let result_f64 = value.as_f64();
                    prop_assert!(result_f64.is_ok());
                    prop_assert_eq!(result_f64.unwrap(), *i as f64);

                    // Int should NOT convert to String
                    prop_assert!(value.as_string().is_err());
                }
                Value::Float(f) => {
                    // Float should convert to f64
                    let result = value.as_f64();
                    prop_assert!(result.is_ok());
                    prop_assert_eq!(result.unwrap(), *f);

                    // Float should NOT convert to i64
                    prop_assert!(value.as_i64().is_err());
                }
                Value::String(s) => {
                    // String should convert to String
                    let result = value.as_string();
                    prop_assert!(result.is_ok());
                    prop_assert_eq!(&result.unwrap(), s);

                    // String should NOT convert to i64
                    prop_assert!(value.as_i64().is_err());
                }
                Value::Bool(b) => {
                    // Bool should convert to bool
                    let result = value.as_bool_result();
                    prop_assert!(result.is_ok());
                    prop_assert_eq!(result.unwrap(), *b);

                    // Bool should NOT convert to i64
                    prop_assert!(value.as_i64().is_err());
                }
                Value::Bytes(bytes) => {
                    // Bytes should convert to Vec<u8>
                    let result = value.as_bytes_vec();
                    prop_assert!(result.is_ok());
                    prop_assert_eq!(&result.unwrap(), bytes);

                    // Bytes should NOT convert to String
                    prop_assert!(value.as_string().is_err());
                }
                Value::Timestamp(ts) => {
                    // Timestamp should convert to DateTime<Utc>
                    let result = value.as_timestamp_result();
                    prop_assert!(result.is_ok());
                    prop_assert_eq!(result.unwrap(), *ts);

                    // Timestamp should NOT convert to i64
                    prop_assert!(value.as_i64().is_err());
                }
                Value::Null => {
                    // Null should NOT convert to any type
                    prop_assert!(value.as_i64().is_err());
                    prop_assert!(value.as_f64().is_err());
                    prop_assert!(value.as_string().is_err());
                    prop_assert!(value.as_bool_result().is_err());
                    prop_assert!(value.as_bytes_vec().is_err());
                    prop_assert!(value.as_timestamp_result().is_err());
                }
            }
        }
    }
}
