# Design Document - Client SDK (Task 15)

## Current Context

This document contains the minimal design context needed for **Task 15: Implement Main Client Interface**.

## Task 15 Overview

Task 15 wires all previously implemented components together into a unified `Client` struct that serves as the main entry point for applications using the SDK.

## Client Architecture

The Client is the top-level interface that applications interact with. It orchestrates all sub-components:

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Application                       │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Client (Main Interface)                    │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  - config: ConnectionConfig                          │  │
│  │  - connection_manager: Arc<ConnectionManager>        │  │
│  │  - auth_manager: Arc<AuthenticationManager>          │  │
│  │  - data_client: DataClient                           │  │
│  │  - admin_client: AdminClient                         │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  Methods:                                                    │
│  - connect(config) -> Result<Client>                        │
│  - disconnect() -> Result<()>                               │
│  - data() -> &DataClient                                    │
│  - admin() -> &AdminClient                                  │
│  - health_check() -> Result<ClusterHealth>                  │
└─────────────────────────────────────────────────────────────┘
```

## Component Integration

### 1. Client Struct

```rust
pub struct Client {
    config: ConnectionConfig,
    connection_manager: Arc<ConnectionManager>,
    auth_manager: Arc<AuthenticationManager>,
    data_client: DataClient,
    admin_client: AdminClient,
}
```

**Fields**:
- `config`: Stores the connection configuration for reference
- `connection_manager`: Manages connection pool and node health (shared via Arc)
- `auth_manager`: Handles authentication and token management (shared via Arc)
- `data_client`: Provides CRUD operations and query execution
- `admin_client`: Provides cluster and user management operations

### 2. Initialization (connect)

```rust
impl Client {
    pub async fn connect(config: ConnectionConfig) -> Result<Self> {
        // 1. Validate configuration
        config.validate()?;
        
        // 2. Create ConnectionManager
        let connection_manager = Arc::new(
            ConnectionManager::new(config.clone()).await?
        );
        
        // 3. Create AuthenticationManager and authenticate
        let credentials = Credentials {
            username: config.username.clone(),
            password: config.password.clone(),
            certificate: config.certificate.clone(),
            token: None,
        };
        
        let auth_manager = Arc::new(
            AuthenticationManager::new(credentials, config.token_ttl)
        );
        
        // Perform initial authentication
        let mut conn = connection_manager.get_connection().await?;
        auth_manager.authenticate(&mut conn).await?;
        connection_manager.return_connection(conn).await;
        
        // 4. Create DataClient
        let data_client = DataClient::new(
            Arc::clone(&connection_manager),
            Arc::clone(&auth_manager),
        );
        
        // 5. Create AdminClient
        let admin_client = AdminClient::new(
            Arc::clone(&connection_manager),
            Arc::clone(&auth_manager),
        );
        
        Ok(Client {
            config,
            connection_manager,
            auth_manager,
            data_client,
            admin_client,
        })
    }
}
```

**Initialization Steps**:
1. Validate configuration parameters
2. Create ConnectionManager (establishes connection pool)
3. Create AuthenticationManager and perform initial authentication
4. Create DataClient with shared ConnectionManager and AuthenticationManager
5. Create AdminClient with shared ConnectionManager and AuthenticationManager
6. Return initialized Client

### 3. Access Methods

```rust
impl Client {
    /// Get reference to DataClient for CRUD operations
    pub fn data(&self) -> &DataClient {
        &self.data_client
    }
    
    /// Get reference to AdminClient for admin operations
    pub fn admin(&self) -> &AdminClient {
        &self.admin_client
    }
}
```

These methods provide access to the sub-clients for performing operations.

### 4. Health Check

```rust
impl Client {
    pub async fn health_check(&self) -> Result<ClusterHealth> {
        // Query health from all nodes
        let node_healths = self.connection_manager
            .health_check_all_nodes()
            .await?;
        
        // Aggregate into cluster health
        let total_nodes = node_healths.len();
        let healthy_nodes = node_healths.iter()
            .filter(|h| h.is_healthy)
            .count();
        
        Ok(ClusterHealth {
            total_nodes,
            healthy_nodes,
            node_healths,
        })
    }
}

pub struct ClusterHealth {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub node_healths: Vec<NodeHealth>,
}
```

**Health Check Logic**:
1. Query health status from all nodes via ConnectionManager
2. Aggregate results into ClusterHealth structure
3. Return overall cluster health status

### 5. Graceful Shutdown

```rust
impl Client {
    pub async fn disconnect(&self) -> Result<()> {
        // 1. Logout to invalidate token
        let mut conn = self.connection_manager.get_connection().await?;
        if let Err(e) = self.auth_manager.logout(&mut conn).await {
            eprintln!("Warning: logout failed during disconnect: {}", e);
        }
        self.connection_manager.return_connection(conn).await;
        
        // 2. Close all connections in the pool
        self.connection_manager.disconnect().await?;
        
        Ok(())
    }
}
```

**Shutdown Steps**:
1. Logout to invalidate authentication token (best effort)
2. Close all connections in the connection pool
3. Release all resources

## Usage Example

```rust
use distributed_db_client::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create configuration
    let config = ConnectionConfig {
        hosts: vec!["localhost:7000".to_string()],
        username: "admin".to_string(),
        password: Some("password".to_string()),
        ..Default::default()
    };
    
    // 2. Connect to database
    let client = Client::connect(config).await?;
    
    // 3. Check cluster health
    let health = client.health_check().await?;
    println!("Cluster: {}/{} nodes healthy", 
        health.healthy_nodes, health.total_nodes);
    
    // 4. Perform data operations
    client.data().execute(
        "CREATE TABLE users (id INT, name TEXT, email TEXT)"
    ).await?;
    
    client.data().execute_with_params(
        "INSERT INTO users VALUES (?, ?, ?)",
        &[1.into(), "Alice".into(), "alice@example.com".into()]
    ).await?;
    
    let results = client.data().query(
        "SELECT * FROM users"
    ).await?;
    
    // 5. Perform admin operations
    let nodes = client.admin().list_nodes().await?;
    println!("Cluster has {} nodes", nodes.len());
    
    // 6. Disconnect gracefully
    client.disconnect().await?;
    
    Ok(())
}
```

## Error Handling

The Client propagates errors from sub-components:

- **Connection errors**: From ConnectionManager during initialization
- **Authentication errors**: From AuthenticationManager during connect
- **Configuration errors**: From config validation
- **Operation errors**: From DataClient and AdminClient methods

All errors use the `DatabaseError` enum defined in the error module.

## Testing Strategy

### Unit Tests

Test Client initialization and lifecycle:
- Test successful connection with valid config
- Test connection failure with invalid config
- Test authentication failure
- Test graceful disconnect
- Test health check aggregation

### Integration Tests

Test full Client lifecycle with real database:
- Test connect → operations → disconnect flow
- Test CRUD operations through Client
- Test transaction operations through Client
- Test admin operations through Client
- Test failover behavior
- Test re-authentication on token expiry

## Implementation Notes

1. **Shared Ownership**: ConnectionManager and AuthenticationManager are wrapped in `Arc` because they need to be shared between DataClient and AdminClient

2. **Async Initialization**: The `connect` method is async because it performs network operations (connection establishment and authentication)

3. **Configuration Validation**: Validate configuration early to fail fast with clear error messages

4. **Resource Cleanup**: The `disconnect` method ensures all resources are properly released

5. **Error Propagation**: Use `?` operator to propagate errors from sub-components with proper context

---

**Full design available in**: `.kiro/specs/client-sdk/FOUNDATION/design.md`
