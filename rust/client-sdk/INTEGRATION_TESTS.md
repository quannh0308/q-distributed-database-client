# Integration Test Requirements for Connection Management

## Overview

The connection management layer includes several property-based tests that require a running q-distributed-database server. These tests validate real network interactions and cannot be executed as unit tests without a live server or sophisticated mock.

## Required Infrastructure

### Test Database Server
- A running q-distributed-database instance (or cluster)
- Configurable test endpoints (e.g., localhost:7000, localhost:7001, localhost:7002)
- Support for health check ping/pong messages
- Ability to simulate node failures and recoveries

### Test Environment Setup
```bash
# Example setup (to be implemented when server is available)
# 1. Start test database cluster
./scripts/start-test-cluster.sh

# 2. Run integration tests
cargo test --test integration_tests

# 3. Cleanup
./scripts/stop-test-cluster.sh
```

## Deferred Property-Based Tests

### 1. Property 1: Connection Establishment (Task 1.1)
**Requirement:** Requirements 1.1  
**Test Coverage:** For any valid configuration with reachable hosts, initializing the client should successfully establish at least one TCP connection

**Implementation Requirements:**
- Test database server running on configurable ports
- Generate random valid configurations with test server addresses
- Verify successful TCP connection establishment
- Verify connection can send/receive messages

**Test Strategy:**
```rust
#[tokio::test]
async fn prop_connection_establishment() {
    // Start test server on random port
    let server = TestServer::start().await;
    
    // Generate random valid configurations
    let config = ConnectionConfig::new(vec![server.address()]);
    
    // Attempt connection
    let manager = ConnectionManager::new(config);
    let conn = manager.get_connection().await;
    
    // Verify connection established
    assert!(conn.is_ok());
}
```

### 2. Property 5: Connection Reuse (Task 1.2)
**Requirement:** Requirements 1.5  
**Test Coverage:** For any sequence of requests within the connection idle timeout, the same underlying connection should be reused

**Implementation Requirements:**
- Test server that tracks connection IDs
- Multiple sequential requests within idle timeout window
- Verification that same connection is reused

**Test Strategy:**
```rust
#[tokio::test]
async fn prop_connection_reuse() {
    let server = TestServer::start().await;
    let config = ConnectionConfig::new(vec![server.address()]);
    let manager = ConnectionManager::new(config);
    
    // Get connection and track its ID
    let conn1 = manager.get_connection().await.unwrap();
    let conn1_id = conn1.node_id();
    manager.return_connection(conn1).await;
    
    // Get connection again within idle timeout
    let conn2 = manager.get_connection().await.unwrap();
    let conn2_id = conn2.node_id();
    
    // Verify same connection was reused
    assert_eq!(conn1_id, conn2_id);
}
```

### 3. Property 3: Load Distribution (Task 1.3)
**Requirement:** Requirements 1.3  
**Test Coverage:** For any set of healthy nodes, requests should be distributed across all nodes

**Implementation Requirements:**
- Multiple test server instances (3+ nodes)
- Request tracking per node
- Statistical verification of distribution

**Test Strategy:**
```rust
#[tokio::test]
async fn prop_load_distribution() {
    // Start 3 test servers
    let servers = vec![
        TestServer::start().await,
        TestServer::start().await,
        TestServer::start().await,
    ];
    
    let config = ConnectionConfig::new(
        servers.iter().map(|s| s.address()).collect()
    );
    let manager = ConnectionManager::new(config);
    
    // Execute many requests
    let mut node_counts = HashMap::new();
    for _ in 0..100 {
        let conn = manager.get_connection().await.unwrap();
        *node_counts.entry(conn.node_id()).or_insert(0) += 1;
        manager.return_connection(conn).await;
    }
    
    // Verify distribution (no single node gets all requests)
    for count in node_counts.values() {
        assert!(*count < 100, "Requests should be distributed");
        assert!(*count > 0, "All nodes should receive requests");
    }
}
```

### 4. Property 4: Unhealthy Node Avoidance (Task 1.4)
**Requirement:** Requirements 1.4  
**Test Coverage:** For any node marked as unhealthy, subsequent requests should not be routed to that node

**Implementation Requirements:**
- Multiple test servers with controllable health status
- Ability to mark nodes as unhealthy
- Request routing verification

**Test Strategy:**
```rust
#[tokio::test]
async fn prop_unhealthy_node_avoidance() {
    let servers = vec![
        TestServer::start().await,
        TestServer::start().await,
    ];
    
    let config = ConnectionConfig::new(
        servers.iter().map(|s| s.address()).collect()
    );
    let manager = ConnectionManager::new(config);
    
    // Mark first node as unhealthy
    manager.mark_node_unhealthy(1).await;
    
    // Execute requests
    for _ in 0..10 {
        let conn = manager.get_connection().await.unwrap();
        assert_ne!(conn.node_id(), 1, "Should not route to unhealthy node");
        manager.return_connection(conn).await;
    }
}
```

### 5. Property 27: Retry with Exponential Backoff (Task 1.6)
**Requirement:** Requirements 8.1, 8.4  
**Test Coverage:** For any retryable error, the client should retry with exponentially increasing delays up to max_retries

**Implementation Requirements:**
- Test server that can simulate retryable errors
- Timing measurement for retry delays
- Verification of retry count and backoff delays

**Test Strategy:**
```rust
#[tokio::test]
async fn prop_retry_with_exponential_backoff() {
    let server = TestServer::start_with_failures(2).await; // Fail first 2 attempts
    let config = ConnectionConfig::new(vec![server.address()]);
    let manager = ConnectionManager::new(config);
    
    let start = Instant::now();
    let result = manager.execute_with_retry(|| async {
        // Operation that fails first 2 times
        server.send_request().await
    }).await;
    
    let elapsed = start.elapsed();
    
    // Verify success after retries
    assert!(result.is_ok());
    
    // Verify exponential backoff timing
    // Expected: 100ms + 200ms = 300ms minimum
    assert!(elapsed >= Duration::from_millis(300));
}
```

## Test Server Implementation

A mock test server needs to be implemented to support these tests:

```rust
// tests/common/test_server.rs
pub struct TestServer {
    addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl TestServer {
    pub async fn start() -> Self {
        // Start TCP listener
        // Handle ping/pong messages
        // Track connections
    }
    
    pub fn address(&self) -> String {
        format!("localhost:{}", self.addr.port())
    }
    
    pub async fn stop(self) {
        // Graceful shutdown
    }
}
```

## Running Integration Tests

Once the test infrastructure is in place:

```bash
# Run only integration tests
cargo test --test integration_tests

# Run with specific test server configuration
TEST_SERVER_PORTS=7000,7001,7002 cargo test --test integration_tests

# Run with verbose output
cargo test --test integration_tests -- --nocapture
```

## Future Work

1. **Test Server Implementation**
   - Create `tests/common/test_server.rs`
   - Implement basic message protocol handling
   - Add connection tracking and health simulation

2. **Integration Test Suite**
   - Create `tests/integration_tests.rs`
   - Implement all deferred property tests
   - Add test fixtures and helpers

3. **CI/CD Integration**
   - Add integration test stage to CI pipeline
   - Set up test database cluster in CI environment
   - Configure test timeouts and retries

4. **Documentation**
   - Add integration test examples
   - Document test server API
   - Create troubleshooting guide

## Notes

- These tests are intentionally separated from unit tests because they require external dependencies
- Unit tests (including property tests for exponential backoff and protocol selection) are complete and passing
- Integration tests should be run in a controlled environment with predictable network conditions
- Consider using Docker containers for test database instances to ensure consistency
