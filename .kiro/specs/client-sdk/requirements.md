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
- **Admin Operations**: Cluster and user management capabilities
- **Result Handling**: Type-safe result processing with streaming support
- **Error Handling**: Comprehensive error types with automatic retry and exponential backoff
- **Message Protocol**: Bincode serialization with CRC32 checksums, length-prefixed framing
- **Multi-Language**: Rust, Python, TypeScript implementations

## Technical Specifications

- **Protocol**: TCP (primary), UDP, TLS
- **Port**: 7000 (default)
- **Serialization**: Bincode with CRC32 checksums
- **Max Message Size**: 1MB (configurable)
- **Connection Pool**: 5-20 connections (configurable)
- **Timeout**: 5000ms (default)
- **Token TTL**: 24 hours (default)
- **Retry Policy**: Max 3 retries, exponential backoff (100ms initial, 5000ms max)

## Current Task Requirements

### Task 13: Implement Compression Support

This task implements message compression using LZ4 and feature negotiation to optimize network bandwidth usage.

#### Compression Overview

The compression system enables:
- Automatic compression of large messages to reduce network bandwidth
- Configurable compression threshold to avoid overhead on small messages
- Feature negotiation between client and server for compression support
- Transparent compression/decompression for all message types

The compression components work at the protocol layer to optimize network communication without affecting higher-level APIs.

#### Key Requirements

**From Requirement 13: Message Protocol and Serialization**

1. **Message Compression (13.6)**
   - WHERE compression is enabled, THE Client_SDK SHALL compress messages above the configured threshold
   - Default compression threshold: 1024 bytes (1KB)
   - Compression algorithm: LZ4 for fast compression/decompression
   - Compressed messages must include compression flag in header

2. **Feature Negotiation (13.7)**
   - WHEN protocol features are negotiated, THE Client_SDK SHALL support compression and heartbeat features
   - Feature negotiation must occur during connection establishment
   - Client must send supported features to server
   - Server responds with mutually supported features
   - Connection stores negotiated features for use during communication

#### Implementation Components

**1. Compression in MessageCodec**

The MessageCodec will be enhanced to support compression:

```rust
use lz4_flex::{compress_prepend_size, decompress_size_prepended};

pub struct MessageCodec {
    compression_enabled: bool,
    compression_threshold: usize,  // Default: 1024 bytes
}

impl MessageCodec {
    pub fn new(compression_enabled: bool, compression_threshold: usize) -> Self {
        Self {
            compression_enabled,
            compression_threshold,
        }
    }
    
    pub fn encode(&self, message: &Message) -> Result<Vec<u8>> {
        // Serialize message with bincode
        let serialized = bincode::serialize(message)
            .map_err(|e| DatabaseError::SerializationError {
                message: e.to_string(),
            })?;
        
        // Compress if enabled and above threshold
        if self.compression_enabled && serialized.len() > self.compression_threshold {
            let compressed = compress_prepend_size(&serialized);
            Ok(compressed)
        } else {
            Ok(serialized)
        }
    }
    
    pub fn decode(&self, data: &[u8]) -> Result<Message> {
        // Try to decompress if compression is enabled
        let decompressed = if self.compression_enabled {
            match decompress_size_prepended(data) {
                Ok(d) => d,
                Err(_) => data.to_vec(), // Not compressed, use as-is
            }
        } else {
            data.to_vec()
        };
        
        // Deserialize message
        bincode::deserialize(&decompressed)
            .map_err(|e| DatabaseError::SerializationError {
                message: e.to_string(),
            })
    }
}
```

**2. Feature Negotiation**

Feature negotiation occurs during connection establishment:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureNegotiation {
    pub supported_features: Vec<Feature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Feature {
    Compression,
    Heartbeat,
    Streaming,
}

impl Connection {
    pub async fn negotiate_features(&mut self, client_features: Vec<Feature>) -> Result<Vec<Feature>> {
        // Send feature negotiation request
        let request = Message {
            message_type: MessageType::FeatureNegotiation,
            sender: self.node_id,
            recipient: None,
            sequence_number: self.next_sequence_number(),
            timestamp: Utc::now(),
            payload: bincode::serialize(&FeatureNegotiation {
                supported_features: client_features.clone(),
            })?,
            checksum: 0, // Will be calculated
        };
        
        self.send_message(request).await?;
        
        // Receive server's supported features
        let response = self.receive_message().await?;
        let server_features: FeatureNegotiation = bincode::deserialize(&response.payload)?;
        
        // Calculate intersection of features
        let negotiated = client_features
            .into_iter()
            .filter(|f| server_features.supported_features.contains(f))
            .collect();
        
        Ok(negotiated)
    }
}
```

**3. Connection with Negotiated Features**

The Connection struct will store negotiated features:

```rust
pub struct Connection {
    socket: TcpStream,
    node_id: NodeId,
    codec: MessageCodec,
    sequence_number: AtomicU64,
    negotiated_features: Vec<Feature>,
}

impl Connection {
    pub async fn connect(
        host: &str,
        config: &ConnectionConfig,
    ) -> Result<Self> {
        // Establish TCP connection
        let socket = TcpStream::connect(host).await?;
        
        // Create codec with compression settings
        let codec = MessageCodec::new(
            config.compression_enabled,
            config.compression_threshold,
        );
        
        let mut conn = Self {
            socket,
            node_id: NodeId::new(),
            codec,
            sequence_number: AtomicU64::new(0),
            negotiated_features: Vec::new(),
        };
        
        // Negotiate features
        let client_features = vec![Feature::Compression, Feature::Heartbeat];
        let negotiated = conn.negotiate_features(client_features).await?;
        conn.negotiated_features = negotiated;
        
        // Update codec based on negotiated features
        if !conn.negotiated_features.contains(&Feature::Compression) {
            conn.codec.compression_enabled = false;
        }
        
        Ok(conn)
    }
    
    pub fn has_feature(&self, feature: &Feature) -> bool {
        self.negotiated_features.contains(feature)
    }
}
```

**4. Configuration Updates**

Update ConnectionConfig to include compression settings:

```rust
pub struct ConnectionConfig {
    pub hosts: Vec<String>,
    pub username: String,
    pub password: Option<String>,
    pub certificate: Option<Certificate>,
    pub enable_tls: bool,
    pub timeout_ms: u64,
    pub pool_config: PoolConfig,
    pub retry_config: RetryConfig,
    pub compression_enabled: bool,      // Default: true
    pub compression_threshold: usize,   // Default: 1024 bytes
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            hosts: vec!["localhost:7000".to_string()],
            username: String::new(),
            password: None,
            certificate: None,
            enable_tls: false,
            timeout_ms: 5000,
            pool_config: PoolConfig::default(),
            retry_config: RetryConfig::default(),
            compression_enabled: true,
            compression_threshold: 1024,
        }
    }
}
```

#### Success Criteria

- ✅ MessageCodec supports LZ4 compression
- ✅ Compression applied to messages above threshold
- ✅ Compression threshold configurable
- ✅ Feature negotiation implemented
- ✅ Connection stores negotiated features
- ✅ Compression disabled if not negotiated
- ✅ Property tests for compression passing
- ✅ All tests passing

#### What Has Been Implemented So Far

**Completed Components:**
- ✅ Message protocol layer (Task 2)
- ✅ Connection management (Task 3)
- ✅ Authentication (Task 5)
- ✅ Data client for CRUD operations (Task 6)
- ✅ Query builder (Task 7)
- ✅ Transaction support (Task 9)
- ✅ Admin client (Task 10)
- ✅ Result handling (Task 11)
- ✅ Error handling (Task 12)

**Ready for Compression Enhancement:**
- MessageCodec exists in protocol.rs
- Connection struct exists in connection.rs
- Need to add LZ4 compression support
- Need to implement feature negotiation
- Need to update ConnectionConfig

#### What Comes Next

After Task 13, the next tasks are:
- **Task 14: Checkpoint** - Ensure all tests pass
- **Task 15: Implement main Client interface** - Wire all components together

---

**Full requirements available in**: `.kiro/specs/client-sdk/FOUNDATION/requirements.md`
