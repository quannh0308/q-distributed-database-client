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

### Task 5: Implement Authentication

This task implements the authentication system for the Client SDK, including token-based authentication, automatic re-authentication, and protocol negotiation.

#### Design Overview

The authentication system consists of three main components:

1. **Credentials and AuthToken Structures**: Data structures for storing authentication information
2. **AuthenticationManager**: Core component managing authentication lifecycle
3. **Protocol Negotiation**: System for selecting the best available protocol

#### Component Design

**1. Credentials Structure**

Stores authentication credentials for connecting to the database:

```rust
pub struct Credentials {
    pub username: String,
    pub password: Option<String>,
    pub certificate: Option<Certificate>,
    pub token: Option<String>,
}
```

**Fields:**
- `username`: Required username for authentication
- `password`: Optional password (used for username/password auth)
- `certificate`: Optional certificate (used for TLS certificate auth)
- `token`: Optional pre-existing token (for token reuse)

**2. AuthToken Structure**

Represents an authentication token issued by the server:

```rust
pub struct AuthToken {
    pub user_id: UserId,
    pub roles: Vec<Role>,
    pub expiration: DateTime<Utc>,
    pub signature: Vec<u8>,
}

impl AuthToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expiration
    }
    
    pub fn time_until_expiration(&self) -> Duration {
        (self.expiration - Utc::now()).to_std().unwrap_or(Duration::ZERO)
    }
}
```

**Fields:**
- `user_id`: Unique identifier for the authenticated user
- `roles`: List of roles/permissions assigned to the user
- `expiration`: Timestamp when the token expires
- `signature`: Cryptographic signature for token validation

**Methods:**
- `is_expired()`: Check if token has expired
- `time_until_expiration()`: Calculate remaining time before expiration

**3. AuthenticationManager**

Manages the authentication lifecycle:

```rust
pub struct AuthenticationManager {
    credentials: Credentials,
    token: Arc<RwLock<Option<AuthToken>>>,
    token_ttl: Duration,
}

impl AuthenticationManager {
    pub fn new(credentials: Credentials, token_ttl: Duration) -> Self;
    
    pub async fn authenticate(&self, conn: &mut Connection) -> Result<AuthToken>;
    
    pub async fn get_valid_token(&self, conn: &mut Connection) -> Result<AuthToken>;
    
    pub async fn refresh_token(&self, conn: &mut Connection) -> Result<AuthToken>;
    
    pub async fn logout(&self, conn: &mut Connection) -> Result<()>;
    
    fn is_token_expired(&self, token: &AuthToken) -> bool;
}
```

**Methods:**

- `new()`: Create new AuthenticationManager with credentials and TTL
- `authenticate()`: Send authentication request to server and receive token
  - Build AuthRequest message with username/password
  - Send request through connection
  - Parse AuthResponse and extract token
  - Store token in internal state
  - Return token to caller

- `get_valid_token()`: Return valid token or re-authenticate if expired
  - Check if token exists and is not expired
  - If valid, return existing token
  - If expired or missing, call authenticate() to get new token
  - Return new token

- `refresh_token()`: Proactively renew token before expiration
  - Send token refresh request to server
  - Receive new token with extended expiration
  - Update stored token
  - Return new token

- `logout()`: Invalidate token on server
  - Send logout request with current token
  - Server marks token as invalid
  - Clear stored token locally
  - Return success

- `is_token_expired()`: Check if token has expired
  - Compare token expiration with current time
  - Return true if expired, false otherwise

**Authentication Flow:**

```
Client                          Server
  │                               │
  ├─── AuthRequest ──────────────>│
  │    (username, password)       │
  │                               │
  │<─── AuthResponse ─────────────┤
  │    (AuthToken)                │
  │                               │
  ├─── QueryRequest ─────────────>│
  │    (with AuthToken)           │
  │                               │
  │<─── QueryResponse ────────────┤
  │                               │
  ├─── QueryRequest ─────────────>│
  │    (expired token)            │
  │                               │
  │<─── AuthError ────────────────┤
  │    (token expired)            │
  │                               │
  ├─── AuthRequest ──────────────>│
  │    (re-authenticate)          │
  │                               │
  │<─── AuthResponse ─────────────┤
  │    (new AuthToken)            │
```

**4. Protocol Negotiation**

Implements protocol selection with priority:

```rust
pub enum ProtocolType {
    TCP = 1,
    UDP = 2,
    TLS = 3,
}

impl ProtocolType {
    pub fn priority(&self) -> u8 {
        match self {
            ProtocolType::TLS => 3,  // Highest priority
            ProtocolType::TCP => 2,
            ProtocolType::UDP => 1,  // Lowest priority
        }
    }
}

pub struct ProtocolNegotiation {
    pub supported_protocols: Vec<ProtocolType>,
    pub preferred_protocol: ProtocolType,
}

impl ProtocolNegotiation {
    pub fn select_protocol(
        client_protocols: &[ProtocolType],
        server_protocols: &[ProtocolType]
    ) -> Option<ProtocolType> {
        // Find intersection of supported protocols
        let mut common: Vec<_> = client_protocols
            .iter()
            .filter(|p| server_protocols.contains(p))
            .collect();
        
        // Sort by priority (highest first)
        common.sort_by_key(|p| std::cmp::Reverse(p.priority()));
        
        // Return highest priority protocol
        common.first().map(|&&p| p)
    }
}
```

**Protocol Selection Logic:**
1. Client sends list of supported protocols
2. Server responds with its supported protocols
3. Client calculates intersection of both lists
4. Client selects protocol with highest priority
5. Connection uses selected protocol

**Priority Order:**
- TLS (priority 3): Most secure, preferred when available
- TCP (priority 2): Reliable, good default
- UDP (priority 1): Fast but unreliable, lowest priority

#### Integration with Existing Components

**Connection Integration:**

The Connection struct will be updated to include authentication:

```rust
pub struct Connection {
    socket: TcpStream,
    node_id: NodeId,
    codec: MessageCodec,
    sequence_number: AtomicU64,
    auth_token: Option<AuthToken>,  // NEW: Store auth token
    protocol: ProtocolType,          // NEW: Store negotiated protocol
}

impl Connection {
    pub async fn authenticate(&mut self, auth_manager: &AuthenticationManager) -> Result<()> {
        let token = auth_manager.authenticate(self).await?;
        self.auth_token = Some(token);
        Ok(())
    }
    
    pub async fn send_authenticated_request(&mut self, request: Request) -> Result<Response> {
        // Ensure we have a valid token
        if let Some(token) = &self.auth_token {
            if token.is_expired() {
                // Token expired, need to re-authenticate
                return Err(DatabaseError::TokenExpired { 
                    expired_at: token.expiration 
                });
            }
        } else {
            return Err(DatabaseError::AuthenticationFailed { 
                reason: "No auth token".to_string() 
            });
        }
        
        // Include token in request
        let mut request = request;
        request.auth_token = self.auth_token.clone();
        
        self.send_request(request).await
    }
}
```

**Client Integration:**

The main Client will initialize AuthenticationManager:

```rust
impl Client {
    pub async fn connect(config: ConnectionConfig) -> Result<Self> {
        // Create authentication manager
        let credentials = Credentials {
            username: config.username.clone(),
            password: config.password.clone(),
            certificate: config.certificate.clone(),
            token: None,
        };
        let auth_manager = Arc::new(AuthenticationManager::new(
            credentials,
            Duration::from_secs(config.token_ttl_seconds.unwrap_or(86400))
        ));
        
        // Create connection manager
        let connection_manager = Arc::new(ConnectionManager::new(config.clone()));
        
        // Get initial connection and authenticate
        let mut conn = connection_manager.get_connection().await?;
        auth_manager.authenticate(&mut conn).await?;
        connection_manager.return_connection(conn);
        
        // Create data and admin clients
        let data_client = DataClient::new(
            connection_manager.clone(),
            auth_manager.clone()
        );
        let admin_client = AdminClient::new(
            connection_manager.clone(),
            auth_manager.clone()
        );
        
        Ok(Self {
            config,
            connection_manager,
            auth_manager,
            data_client,
            admin_client,
        })
    }
}
```

#### Error Handling

**Authentication Errors:**

```rust
pub enum DatabaseError {
    // ... existing variants ...
    
    // Authentication Errors
    AuthenticationFailed { reason: String },
    TokenExpired { expired_at: Timestamp },
    InvalidCredentials,
}
```

**Error Handling Strategy:**
- `AuthenticationFailed`: Return immediately, do not retry
- `TokenExpired`: Trigger automatic re-authentication
- `InvalidCredentials`: Return immediately, do not retry

#### Testing Strategy

**Unit Tests:**
- Test Credentials struct creation
- Test AuthToken expiration checking
- Test protocol priority ordering
- Test protocol selection logic

**Property Tests:**

**Property 8: Auth Token Structure**
- Generate random successful authentication responses
- Verify all required fields are present (user_id, roles, expiration, signature)
- Verify fields have correct types

**Property 9: Token Inclusion in Requests**
- Generate random authenticated requests
- Verify each request includes the auth token
- Verify token is correctly serialized

**Property 10: Automatic Re-authentication**
- Generate expired tokens
- Simulate request with expired token
- Verify re-authentication is triggered
- Verify new token is obtained

**Property 11: Token Invalidation on Logout**
- Generate valid tokens
- Call logout()
- Verify subsequent requests with old token fail
- Verify token is cleared locally

**Property 12: Token TTL Respect**
- Generate tokens with various TTL values
- Verify tokens expire at correct time
- Verify expiration checking is accurate

**Property 7: Protocol Selection Priority**
- Generate various combinations of client/server protocols
- Verify highest priority common protocol is selected
- Verify TLS > TCP > UDP priority order

#### Implementation Notes

**Security Considerations:**
- Never log passwords or tokens
- Clear sensitive data from memory after use
- Use TLS for transmitting credentials
- Validate token signatures on client side

**Performance Considerations:**
- Cache tokens to avoid repeated authentication
- Proactively refresh tokens before expiration
- Use async operations to avoid blocking

**Concurrency Considerations:**
- Use Arc<RwLock<>> for thread-safe token storage
- Handle concurrent authentication requests
- Ensure token updates are atomic

---

**Full design with 42 correctness properties available in**: `.kiro/specs/client-sdk/FOUNDATION/design.md`
