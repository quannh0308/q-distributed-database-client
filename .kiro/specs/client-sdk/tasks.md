# Automated Task Execution Cycle

**Current Task**: 7 - Implement query builder

This is an automated 2-task cycle designed to minimize token consumption by loading only the current task context instead of the entire massive project specification.

## Tasks

- [ ] 1. Execute Current Task (7): Implement query builder
  - **Task Objective**: Implement the QueryBuilder component that provides a fluent API for constructing type-safe database queries with SQL injection prevention
  
  - **Implementation Steps**:
    
    **Step 1: Implement QueryBuilder struct (Subtask 7.1)**
    
    1. **Create QueryBuilder in lib.rs**
       ```rust
       // Add to rust/client-sdk/src/lib.rs
       pub mod query_builder;
       ```
    
    2. **Create query_builder.rs module**
       ```bash
       # Create new file: rust/client-sdk/src/query_builder.rs
       ```
    
    3. **Define QueryBuilder struct and enums**
       - Define QueryType enum (Select, Insert, Update, Delete)
       - Define LogicalOperator enum (None, And, Or)
       - Define Condition struct with clause and operator
       - Define OrderDirection enum (Asc, Desc)
       - Define OrderBy struct
       - Define QueryBuilder struct with all fields
    
    4. **Implement constructor methods**
       - Implement select() for SELECT queries
       - Implement insert_into() for INSERT queries
       - Implement update() for UPDATE queries
       - Implement delete_from() for DELETE queries
    
    5. **Implement fluent API methods**
       - Implement from() to set table name
       - Implement where_clause() for first condition
       - Implement and() for AND conditions
       - Implement or() for OR conditions
       - Implement columns() for INSERT column list
       - Implement values() for INSERT values
       - Implement set() for UPDATE assignments
       - Implement order_by() for ORDER BY clause
       - Implement limit() for LIMIT clause
       - Implement offset() for OFFSET clause
    
    _Requirements: 4.1, 4.2_
    
    **Step 2: Implement build() method (Subtask 7.2)**
    
    1. **Implement build() dispatcher**
       - Match on query_type
       - Call appropriate build method
       - Return (String, Vec<Value>) tuple
    
    2. **Implement build_select()**
       - Validate FROM clause exists
       - Build SELECT clause with columns
       - Build WHERE clause with conditions
       - Build ORDER BY clause
       - Build LIMIT/OFFSET clauses
       - Return complete SQL string
    
    3. **Implement build_insert()**
       - Validate table, columns, and values exist
       - Build INSERT INTO clause
       - Build column list
       - Build VALUES clause with placeholders
       - Return complete SQL string
    
    4. **Implement build_update()**
       - Validate table and SET clause exist
       - Build UPDATE clause
       - Build SET clause with placeholders
       - Build WHERE clause
       - Return complete SQL string
    
    5. **Implement build_delete()**
       - Validate table exists
       - Build DELETE FROM clause
       - Build WHERE clause
       - Return complete SQL string
    
    _Requirements: 4.1, 4.2_
    
    **Step 3: Write property test for valid SQL generation (Subtask 7.3)***
    
    1. **Add property test in query_builder.rs**
       - Generate random valid query builder sequences
       - Build SQL from each sequence
       - Verify SQL is syntactically valid
       - Verify placeholder count matches parameter count
       - **Property 18: Query Builder Produces Valid SQL**
       - **Validates: Requirements 4.1**
       - Minimum 100 iterations
    
    _Requirements: 4.1_
    
    **Step 4: Write property test for condition logic (Subtask 7.4)***
    
    1. **Add property test in query_builder.rs**
       - Generate random AND/OR condition combinations
       - Build SQL with conditions
       - Verify logical operators correctly placed
       - Verify condition order preserved
       - **Property 19: Condition Logic Correctness**
       - **Validates: Requirements 4.2**
       - Minimum 100 iterations
    
    _Requirements: 4.2_
    
    **Step 5: Write property test for SQL injection prevention (Subtask 7.5)***
    
    1. **Add property test in query_builder.rs**
       - Generate random strings with SQL special characters
       - Use strings as parameter values
       - Build queries
       - Verify strings treated as data, not SQL code
       - Verify no SQL syntax errors from special characters
       - **Property 20: SQL Injection Prevention**
       - **Validates: Requirements 4.3**
       - Minimum 100 iterations
    
    _Requirements: 4.3_
    
    **Step 6: Implement prepared statement caching (Subtask 7.6)**
    
    1. **Implement prepare() in DataClient**
       - Check prepared_statements cache first
       - If cached, return cached statement
       - If not cached, send PrepareRequest to server
       - Parse PrepareResponse
       - Add to cache
       - Return PreparedStatement
    
    2. **Define PreparedStatement struct**
       - Add statement_id field
       - Add sql field
       - Add param_count field
    
    3. **Update execute_with_params() to use prepared statements**
       - Check if SQL is in cache
       - If cached, use prepared statement ID
       - If not cached, execute normally
    
    _Requirements: 4.1_
    
    **Step 7: Integration and Testing**
    
    1. **Update DataClient**
       - Add query_builder() method
       - Add execute_builder() method
    
    2. **Run all tests**
       ```bash
       cd rust/client-sdk
       cargo test --all-features
       ```
    
    3. **Run property tests**
       ```bash
       cargo test --all-features -- --include-ignored
       ```
    
    4. **Check for warnings**
       ```bash
       cargo clippy --all-features
       ```
  
  - **Success Criteria**:
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
    - ✅ No critical warnings
  
  - **Subtasks**:
    - [ ] 7.1 Implement QueryBuilder with fluent API
    - [ ] 7.2 Implement build() method for SQL generation
    - [ ]* 7.3 Write property test for query builder produces valid SQL
    - [ ]* 7.4 Write property test for condition logic correctness
    - [ ]* 7.5 Write property test for SQL injection prevention
    - [ ] 7.6 Implement prepared statement caching
  
  - _Requirements: 4.1, 4.2, 4.3_

- [ ] 2. Complete and Setup Next Task: Mark Task 7 complete and setup Task 8 context
  - **Automation Steps**:
    1. Update FOUNDATION/tasks.md: Change `- [ ] 7` to `- [x] 7`
    2. Identify Next Task: Task 8 - Checkpoint - Ensure all tests pass
    3. Extract Context: Get checkpoint requirements from FOUNDATION files
    4. Update Active Files:
       - Update requirements.md with Task 8 context
       - Update design.md with Task 8 context
       - Update this tasks.md with new 2-task cycle for Task 8
    5. Commit Changes: Create git commit documenting Task 7 completion
  - **Expected Result**: Complete automation setup for Task 8 execution with minimal token consumption

---

## Automation Benefits

- **Token Reduction**: 80-90% reduction by loading minimal context vs full specification
- **Seamless Workflow**: "Click Start task → Click Start task → repeat" pattern
- **Full Coverage**: All 18 major tasks + 70+ subtasks remain accessible in FOUNDATION
- **Progress Tracking**: Automatic completion marking and next task identification
- **Context Preservation**: Relevant requirements and design context extracted for each task

**Full Project Context**: Available in `.kiro/specs/client-sdk/FOUNDATION/` directory
