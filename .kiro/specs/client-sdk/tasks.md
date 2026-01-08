# Automated Task Execution Cycle

**Current Task**: 6 - Implement data client for CRUD operations

This is an automated 2-task cycle designed to minimize token consumption by loading only the current task context instead of the entire massive project specification.

## Tasks

- [ ] 1. Execute Current Task (6): Implement data client for CRUD operations
  - **Task Objective**: Implement the DataClient component that handles all CRUD (Create, Read, Update, Delete) operations on database tables, including query execution, result handling, and batch operations
  
  - **Implementation Steps**:
    
    **Step 1: Implement DataClient struct (Subtask 6.1)**
    
    1. **Create DataClient in lib.rs**
       ```rust
       // Add to rust/client-sdk/src/lib.rs
       pub mod data_client;
       ```
    
    2. **Create data_client.rs module**
       ```bash
       # Create new file: rust/client-sdk/src/data_client.rs
       ```
    
    3. **Define DataClient struct**
       - Add fields: connection_manager (Arc<ConnectionManager>), auth_manager (Arc<AuthenticationManager>), prepared_statements (Arc<RwLock<HashMap<String, PreparedStatement>>>)
       - Implement constructor new()
       - Store references to ConnectionManager and AuthenticationManager
    
    4. **Define result types**
       - Define ExecuteResult struct with rows_affected and last_insert_id
       - Define QueryResult struct with columns and rows
       - Define Row struct with values vector
       - Define ColumnMetadata struct
    
    _Requirements: 3.1, 3.2, 3.3, 3.4_
    
    **Step 2: Implement execute() and execute_with_params() (Subtask 6.2)**
    
    1. **Implement execute() method**
       - Call execute_with_params() with empty params array
       - Return ExecuteResult
    
    2. **Implement execute_with_params() method**
       - Get connection from ConnectionManager
       - Get valid auth token from AuthenticationManager
       - Build ExecuteRequest message with SQL and parameters
       - Send request through connection
       - Parse ExecuteResponse and extract result
       - Return ExecuteResult with rows_affected and last_insert_id
    
    3. **Handle errors**
       - Convert server errors to DatabaseError
       - Include SQL and parameters in error context
       - Return connection to pool on error
    
    _Requirements: 3.1, 3.3, 3.4, 3.5_
    
    **Step 3: Implement query() and query_with_params() (Subtask 6.3)**
    
    1. **Implement query() method**
       - Call query_with_params() with empty params array
       - Return QueryResult
    
    2. **Implement query_with_params() method**
       - Get connection from ConnectionManager
       - Get valid auth token from AuthenticationManager
       - Build QueryRequest message with SQL and parameters
       - Send request through connection
       - Parse QueryResponse and extract columns and rows
       - Convert server response to QueryResult
       - Return QueryResult
    
    3. **Implement Row access methods**
       - Implement get() for index-based column access
       - Implement get_by_name() for name-based column access
       - Handle out-of-bounds and missing column errors
    
    _Requirements: 3.2, 3.5_
    
    **Step 4: Write property test for insert-then-retrieve consistency (Subtask 6.4)***
    
    1. **Add property test in data_client.rs**
       - Generate random records
       - Insert record using execute_with_params()
       - Query for record using query_with_params()
       - Verify returned values match inserted values
       - **Property 13: Insert-Then-Retrieve Consistency**
       - **Validates: Requirements 3.1, 3.2**
       - Minimum 100 iterations
    
    _Requirements: 3.1, 3.2_
    
    **Step 5: Write property test for update visibility (Subtask 6.5)***
    
    1. **Add property test in data_client.rs**
       - Generate random records and updates
       - Insert record, then update it
       - Query for record
       - Verify returned values match updated values
       - **Property 14: Update Visibility**
       - **Validates: Requirements 3.3**
       - Minimum 100 iterations
    
    _Requirements: 3.3_
    
    **Step 6: Write property test for delete removes record (Subtask 6.6)***
    
    1. **Add property test in data_client.rs**
       - Generate random records
       - Insert record, then delete it
       - Query for record
       - Verify no results returned
       - **Property 15: Delete Removes Record**
       - **Validates: Requirements 3.4**
       - Minimum 100 iterations
    
    _Requirements: 3.4_
    
    **Step 7: Write property test for operation result structure (Subtask 6.7)***
    
    1. **Add property test in data_client.rs**
       - Generate random operations
       - Execute operations
       - Verify result contains rows_affected or error details
       - **Property 16: Operation Result Structure**
       - **Validates: Requirements 3.5**
       - Minimum 100 iterations
    
    _Requirements: 3.5_
    
    **Step 8: Implement query_stream() for streaming results (Subtask 6.8)**
    
    1. **Define ResultStream struct**
       - Add fields: connection, columns, finished
       - Implement next() method to fetch rows incrementally
    
    2. **Implement query_stream() method**
       - Get connection from ConnectionManager
       - Get valid auth token from AuthenticationManager
       - Build streaming QueryRequest
       - Send request
       - Return ResultStream
    
    3. **Implement ResultStream::next()**
       - Receive next message from server
       - Parse row data or end-of-stream marker
       - Return Option<Row>
       - Handle backpressure
    
    _Requirements: 9.4_
    
    **Step 9: Write property test for streaming memory efficiency (Subtask 6.9)***
    
    1. **Add property test in data_client.rs**
       - Generate large result set
       - Stream results using query_stream()
       - Monitor memory usage
       - Verify memory remains bounded
       - **Property 35: Streaming Memory Efficiency**
       - **Validates: Requirements 9.4**
       - Minimum 100 iterations
    
    _Requirements: 9.4_
    
    **Step 10: Implement batch operations (Subtask 6.10)**
    
    1. **Define BatchContext struct**
       - Add fields: connection, auth_manager, operations
       - Implement add_execute() to add operations
       - Implement execute() to run batch atomically
    
    2. **Implement batch() method in DataClient**
       - Get connection from ConnectionManager
       - Create BatchContext
       - Return BatchContext
    
    3. **Implement BatchContext::execute()**
       - Get valid auth token
       - Build BatchRequest with all operations
       - Send request
       - Parse BatchResponse
       - Return results
    
    _Requirements: 3.6_
    
    **Step 11: Write property test for batch operation atomicity (Subtask 6.11)***
    
    1. **Add property test in data_client.rs**
       - Generate batch of operations with one that will fail
       - Execute batch
       - Verify either all succeed or all fail (no partial success)
       - **Property 17: Batch Operation Atomicity**
       - **Validates: Requirements 3.6**
       - Minimum 100 iterations
    
    _Requirements: 3.6_
    
    **Step 12: Integration and Testing**
    
    1. **Update Client struct**
       - Add data_client field
       - Initialize DataClient in connect()
       - Implement data() accessor method
    
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
    - ✅ DataClient struct implemented with all required fields
    - ✅ execute() and execute_with_params() methods working
    - ✅ query() and query_with_params() methods working
    - ✅ Row access methods (get, get_by_name) working
    - ✅ query_stream() implemented for large result sets
    - ✅ batch() operations working atomically
    - ✅ All property tests passing (Properties 13-17, 35)
    - ✅ All unit tests passing
    - ✅ Code compiles without errors
    - ✅ No critical warnings
  
  - **Subtasks**:
    - [ ] 6.1 Implement DataClient struct
    - [ ] 6.2 Implement execute() and execute_with_params()
    - [ ] 6.3 Implement query() and query_with_params()
    - [ ]* 6.4 Write property test for insert-then-retrieve consistency
    - [ ]* 6.5 Write property test for update visibility
    - [ ]* 6.6 Write property test for delete removes record
    - [ ]* 6.7 Write property test for operation result structure
    - [ ] 6.8 Implement query_stream() for streaming results
    - [ ]* 6.9 Write property test for streaming memory efficiency
    - [ ] 6.10 Implement batch operations
    - [ ]* 6.11 Write property test for batch operation atomicity
  
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 9.4_

- [ ] 2. Complete and Setup Next Task: Mark Task 6 complete and setup Task 7 context
  - **Automation Steps**:
    1. Update FOUNDATION/tasks.md: Change `- [ ] 6` to `- [x] 6`
    2. Identify Next Task: Task 7 - Implement query builder
    3. Extract Context: Get query builder requirements and design from FOUNDATION files
    4. Update Active Files:
       - Update requirements.md with Task 7 context
       - Update design.md with Task 7 context
       - Update this tasks.md with new 2-task cycle for Task 7
    5. Commit Changes: Create git commit documenting Task 6 completion
  - **Expected Result**: Complete automation setup for Task 7 execution with minimal token consumption

---

## Automation Benefits

- **Token Reduction**: 80-90% reduction by loading minimal context vs full specification
- **Seamless Workflow**: "Click Start task → Click Start task → repeat" pattern
- **Full Coverage**: All 18 major tasks + 70+ subtasks remain accessible in FOUNDATION
- **Progress Tracking**: Automatic completion marking and next task identification
- **Context Preservation**: Relevant requirements and design context extracted for each task

**Full Project Context**: Available in `.kiro/specs/client-sdk/FOUNDATION/` directory
