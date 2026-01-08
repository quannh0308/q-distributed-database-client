//! Message protocol layer for Q-Distributed-Database Client SDK
//!
//! This module implements the message protocol with bincode serialization,
//! CRC32 checksum validation, and length-prefixed framing.

use crate::error::DatabaseError;
use crate::types::{NodeId, Timestamp};
use crate::connection::ProtocolType;
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Message type enumeration
///
/// Defines all possible message types in the protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// Ping message for connection health check
    Ping,
    /// Pong response to ping
    Pong,
    /// Data message containing query/response payload
    Data,
    /// Acknowledgment message
    Ack,
    /// Error message
    Error,
    /// Heartbeat message for connection keep-alive
    Heartbeat,
    /// Cluster join notification
    ClusterJoin,
    /// Cluster leave notification
    ClusterLeave,
    /// Replication message
    Replication,
    /// Transaction message
    Transaction,
}

/// Message structure
///
/// Represents a complete message in the protocol with all required fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Sender node identifier
    pub sender: NodeId,
    /// Recipient node identifier
    pub recipient: NodeId,
    /// Sequence number for ordering
    pub sequence_number: u64,
    /// Message timestamp
    pub timestamp: Timestamp,
    /// Type of message
    pub message_type: MessageType,
    /// Message payload
    pub payload: Vec<u8>,
    /// CRC32 checksum for integrity validation
    pub checksum: u32,
}

impl Message {
    /// Creates a new message with the given parameters
    ///
    /// The checksum is automatically calculated.
    pub fn new(
        sender: NodeId,
        recipient: NodeId,
        sequence_number: u64,
        timestamp: Timestamp,
        message_type: MessageType,
        payload: Vec<u8>,
    ) -> Self {
        let mut message = Self {
            sender,
            recipient,
            sequence_number,
            timestamp,
            message_type,
            payload,
            checksum: 0,
        };
        message.checksum = message.calculate_checksum();
        message
    }

    /// Calculates the CRC32 checksum of the message
    ///
    /// The checksum is calculated over all fields except the checksum field itself.
    pub fn calculate_checksum(&self) -> u32 {
        let mut hasher = Hasher::new();
        
        // Hash all fields except checksum
        hasher.update(&self.sender.to_le_bytes());
        hasher.update(&self.recipient.to_le_bytes());
        hasher.update(&self.sequence_number.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        
        // Hash message type as a discriminant
        let type_discriminant = match self.message_type {
            MessageType::Ping => 0u8,
            MessageType::Pong => 1u8,
            MessageType::Data => 2u8,
            MessageType::Ack => 3u8,
            MessageType::Error => 4u8,
            MessageType::Heartbeat => 5u8,
            MessageType::ClusterJoin => 6u8,
            MessageType::ClusterLeave => 7u8,
            MessageType::Replication => 8u8,
            MessageType::Transaction => 9u8,
        };
        hasher.update(&[type_discriminant]);
        
        // Hash payload
        hasher.update(&self.payload);
        
        hasher.finalize()
    }

    /// Verifies that the message checksum is valid
    ///
    /// Returns true if the stored checksum matches the calculated checksum.
    pub fn verify_checksum(&self) -> bool {
        let calculated = self.calculate_checksum();
        self.checksum == calculated
    }
}

/// Message codec for serialization and deserialization
///
/// Handles encoding/decoding messages with bincode, length-prefixed framing,
/// and message size validation.
pub struct MessageCodec {
    /// Maximum allowed message size in bytes
    max_message_size: usize,
}

impl MessageCodec {
    /// Creates a new message codec with the default maximum message size (1MB)
    pub fn new() -> Self {
        Self {
            max_message_size: 1024 * 1024, // 1MB default
        }
    }

    /// Creates a new message codec with a custom maximum message size
    pub fn with_max_size(max_message_size: usize) -> Self {
        Self { max_message_size }
    }

    /// Encodes a message to bytes using bincode
    ///
    /// Returns an error if serialization fails or if the message exceeds the size limit.
    pub fn encode(&self, message: &Message) -> Result<Vec<u8>, DatabaseError> {
        let encoded = bincode::serialize(message).map_err(|e| DatabaseError::SerializationError {
            message: format!("Failed to serialize message: {}", e),
        })?;

        // Check message size
        if encoded.len() > self.max_message_size {
            return Err(DatabaseError::MessageTooLarge {
                size: encoded.len(),
                max_size: self.max_message_size,
            });
        }

        Ok(encoded)
    }

    /// Decodes a message from bytes using bincode
    ///
    /// Returns an error if deserialization fails or if checksum validation fails.
    pub fn decode(&self, data: &[u8]) -> Result<Message, DatabaseError> {
        // Check message size
        if data.len() > self.max_message_size {
            return Err(DatabaseError::MessageTooLarge {
                size: data.len(),
                max_size: self.max_message_size,
            });
        }

        let message: Message =
            bincode::deserialize(data).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to deserialize message: {}", e),
            })?;

        // Verify checksum
        if !message.verify_checksum() {
            let expected = message.calculate_checksum();
            return Err(DatabaseError::ChecksumMismatch {
                expected,
                actual: message.checksum,
            });
        }

        Ok(message)
    }

    /// Encodes a message with a 4-byte big-endian length prefix
    ///
    /// The format is: [4-byte length][message data]
    pub fn encode_with_length(&self, message: &Message) -> Result<Vec<u8>, DatabaseError> {
        let encoded = self.encode(message)?;
        let length = encoded.len() as u32;

        let mut result = Vec::with_capacity(4 + encoded.len());
        result.extend_from_slice(&length.to_be_bytes());
        result.extend_from_slice(&encoded);

        Ok(result)
    }

    /// Reads a message from an async reader
    ///
    /// Reads the 4-byte length prefix first, then reads the message data.
    pub async fn read_message<R: AsyncRead + Unpin>(
        &self,
        reader: &mut R,
    ) -> Result<Message, DatabaseError> {
        // Read 4-byte length prefix
        let mut length_bytes = [0u8; 4];
        reader
            .read_exact(&mut length_bytes)
            .await
            .map_err(|e| DatabaseError::NetworkError {
                details: format!("Failed to read message length: {}", e),
            })?;

        let length = u32::from_be_bytes(length_bytes) as usize;

        // Validate message size before allocating
        if length > self.max_message_size {
            return Err(DatabaseError::MessageTooLarge {
                size: length,
                max_size: self.max_message_size,
            });
        }

        // Read message data
        let mut data = vec![0u8; length];
        reader
            .read_exact(&mut data)
            .await
            .map_err(|e| DatabaseError::NetworkError {
                details: format!("Failed to read message data: {}", e),
            })?;

        // Decode message
        self.decode(&data)
    }

    /// Writes a message to an async writer
    ///
    /// Writes the 4-byte length prefix followed by the message data.
    pub async fn write_message<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut W,
        message: &Message,
    ) -> Result<(), DatabaseError> {
        let encoded = self.encode_with_length(message)?;

        writer
            .write_all(&encoded)
            .await
            .map_err(|e| DatabaseError::NetworkError {
                details: format!("Failed to write message: {}", e),
            })?;

        writer
            .flush()
            .await
            .map_err(|e| DatabaseError::NetworkError {
                details: format!("Failed to flush writer: {}", e),
            })?;

        Ok(())
    }
}

impl Default for MessageCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// Protocol negotiation for selecting the best protocol
///
/// Handles protocol selection between client and server based on
/// mutually supported protocols and priority ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolNegotiation {
    /// List of protocols supported by the client
    pub supported_protocols: Vec<ProtocolType>,
    /// Preferred protocol (highest priority)
    pub preferred_protocol: ProtocolType,
}

impl ProtocolNegotiation {
    /// Creates a new protocol negotiation with the given supported protocols
    pub fn new(supported_protocols: Vec<ProtocolType>) -> Self {
        let preferred_protocol = ProtocolType::select_best(&supported_protocols)
            .unwrap_or(ProtocolType::TCP);

        Self {
            supported_protocols,
            preferred_protocol,
        }
    }

    /// Selects the best protocol from client and server supported protocols
    ///
    /// Returns the highest priority protocol that both client and server support.
    pub fn select_protocol(
        client_protocols: &[ProtocolType],
        server_protocols: &[ProtocolType],
    ) -> Option<ProtocolType> {
        // Find intersection of supported protocols
        let mut common: Vec<ProtocolType> = client_protocols
            .iter()
            .filter(|p| server_protocols.contains(p))
            .copied()
            .collect();

        // Sort by priority (highest first)
        common.sort_by_key(|p| std::cmp::Reverse(p.priority()));

        // Return highest priority protocol
        common.first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test message
    fn create_test_message() -> Message {
        Message::new(
            1,                          // sender
            2,                          // recipient
            100,                        // sequence_number
            1704067200000,              // timestamp
            MessageType::Data,          // message_type
            vec![1, 2, 3, 4, 5],        // payload
        )
    }

    // MessageType Tests
    #[test]
    fn test_message_type_equality() {
        assert_eq!(MessageType::Ping, MessageType::Ping);
        assert_eq!(MessageType::Data, MessageType::Data);
        assert_ne!(MessageType::Ping, MessageType::Pong);
    }

    #[test]
    fn test_message_type_clone() {
        let mt1 = MessageType::Data;
        let mt2 = mt1.clone();
        assert_eq!(mt1, mt2);
    }

    // Message Tests
    #[test]
    fn test_message_creation() {
        let msg = create_test_message();
        assert_eq!(msg.sender, 1);
        assert_eq!(msg.recipient, 2);
        assert_eq!(msg.sequence_number, 100);
        assert_eq!(msg.timestamp, 1704067200000);
        assert_eq!(msg.message_type, MessageType::Data);
        assert_eq!(msg.payload, vec![1, 2, 3, 4, 5]);
        assert_ne!(msg.checksum, 0);
    }

    #[test]
    fn test_message_checksum_calculation() {
        let msg = create_test_message();
        let calculated = msg.calculate_checksum();
        assert_eq!(msg.checksum, calculated);
    }

    #[test]
    fn test_message_checksum_verification() {
        let msg = create_test_message();
        assert!(msg.verify_checksum());
    }

    #[test]
    fn test_message_checksum_verification_fails_on_corruption() {
        let mut msg = create_test_message();
        msg.checksum = 0; // Corrupt the checksum
        assert!(!msg.verify_checksum());
    }

    #[test]
    fn test_message_checksum_changes_with_payload() {
        let msg1 = Message::new(1, 2, 100, 1704067200000, MessageType::Data, vec![1, 2, 3]);
        let msg2 = Message::new(1, 2, 100, 1704067200000, MessageType::Data, vec![4, 5, 6]);
        assert_ne!(msg1.checksum, msg2.checksum);
    }

    #[test]
    fn test_message_checksum_changes_with_sender() {
        let msg1 = Message::new(1, 2, 100, 1704067200000, MessageType::Data, vec![1, 2, 3]);
        let msg2 = Message::new(3, 2, 100, 1704067200000, MessageType::Data, vec![1, 2, 3]);
        assert_ne!(msg1.checksum, msg2.checksum);
    }

    // MessageCodec Tests
    #[test]
    fn test_codec_creation() {
        let codec = MessageCodec::new();
        assert_eq!(codec.max_message_size, 1024 * 1024);
    }

    #[test]
    fn test_codec_with_custom_max_size() {
        let codec = MessageCodec::with_max_size(512);
        assert_eq!(codec.max_message_size, 512);
    }

    #[test]
    fn test_codec_encode_decode() {
        let codec = MessageCodec::new();
        let msg = create_test_message();
        
        let encoded = codec.encode(&msg).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        
        assert_eq!(msg.sender, decoded.sender);
        assert_eq!(msg.recipient, decoded.recipient);
        assert_eq!(msg.sequence_number, decoded.sequence_number);
        assert_eq!(msg.timestamp, decoded.timestamp);
        assert_eq!(msg.message_type, decoded.message_type);
        assert_eq!(msg.payload, decoded.payload);
        assert_eq!(msg.checksum, decoded.checksum);
    }

    #[test]
    fn test_codec_encode_with_length() {
        let codec = MessageCodec::new();
        let msg = create_test_message();
        
        let encoded = codec.encode_with_length(&msg).unwrap();
        
        // Check that first 4 bytes are the length
        let length = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        assert_eq!(length, encoded.len() - 4);
    }

    #[test]
    fn test_codec_message_too_large() {
        let codec = MessageCodec::with_max_size(10);
        let msg = Message::new(
            1,
            2,
            100,
            1704067200000,
            MessageType::Data,
            vec![0; 100], // Large payload
        );
        
        let result = codec.encode(&msg);
        assert!(result.is_err());
        
        if let Err(DatabaseError::MessageTooLarge { size, max_size }) = result {
            assert!(size > max_size);
            assert_eq!(max_size, 10);
        } else {
            panic!("Expected MessageTooLarge error");
        }
    }

    #[test]
    fn test_codec_decode_rejects_oversized_message() {
        let codec = MessageCodec::with_max_size(10);
        let large_data = vec![0; 100];
        
        let result = codec.decode(&large_data);
        assert!(result.is_err());
        
        if let Err(DatabaseError::MessageTooLarge { size, max_size }) = result {
            assert_eq!(size, 100);
            assert_eq!(max_size, 10);
        } else {
            panic!("Expected MessageTooLarge error");
        }
    }

    #[test]
    fn test_codec_decode_detects_checksum_mismatch() {
        let codec = MessageCodec::new();
        let msg = create_test_message();
        
        // Encode the message
        let mut encoded = codec.encode(&msg).unwrap();
        
        // Corrupt the payload in the encoded data (skip the checksum field)
        if encoded.len() > 10 {
            encoded[10] ^= 0xFF;
        }
        
        // Try to decode - should fail checksum validation
        let result = codec.decode(&encoded);
        assert!(result.is_err());
        
        if let Err(DatabaseError::ChecksumMismatch { expected, actual }) = result {
            assert_ne!(expected, actual);
        } else {
            panic!("Expected ChecksumMismatch error");
        }
    }

    #[test]
    fn test_codec_default() {
        let codec = MessageCodec::default();
        assert_eq!(codec.max_message_size, 1024 * 1024);
    }

    // Async I/O Tests
    #[tokio::test]
    async fn test_codec_read_write_message() {
        let codec = MessageCodec::new();
        let msg = create_test_message();
        
        // Create an in-memory buffer
        let mut buffer = Vec::new();
        
        // Write message
        codec.write_message(&mut buffer, &msg).await.unwrap();
        
        // Read message back
        let mut cursor = &buffer[..];
        let decoded = codec.read_message(&mut cursor).await.unwrap();
        
        assert_eq!(msg.sender, decoded.sender);
        assert_eq!(msg.recipient, decoded.recipient);
        assert_eq!(msg.sequence_number, decoded.sequence_number);
        assert_eq!(msg.payload, decoded.payload);
    }

    #[tokio::test]
    async fn test_codec_read_message_validates_size() {
        let codec = MessageCodec::with_max_size(10);
        
        // Create a buffer with a length prefix indicating a large message
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&1000u32.to_be_bytes()); // Length = 1000
        
        let mut cursor = &buffer[..];
        let result = codec.read_message(&mut cursor).await;
        
        assert!(result.is_err());
        if let Err(DatabaseError::MessageTooLarge { size, max_size }) = result {
            assert_eq!(size, 1000);
            assert_eq!(max_size, 10);
        } else {
            panic!("Expected MessageTooLarge error");
        }
    }

    #[tokio::test]
    async fn test_codec_write_message_includes_length_prefix() {
        let codec = MessageCodec::new();
        let msg = create_test_message();
        
        let mut buffer = Vec::new();
        codec.write_message(&mut buffer, &msg).await.unwrap();
        
        // Check that buffer starts with length prefix
        assert!(buffer.len() >= 4);
        let length = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        assert_eq!(length, buffer.len() - 4);
    }

    // ProtocolNegotiation Tests
    #[test]
    fn test_protocol_negotiation_creation() {
        let protocols = vec![ProtocolType::TCP, ProtocolType::TLS];
        let negotiation = ProtocolNegotiation::new(protocols.clone());

        assert_eq!(negotiation.supported_protocols, protocols);
        assert_eq!(negotiation.preferred_protocol, ProtocolType::TLS);
    }

    #[test]
    fn test_protocol_negotiation_select_protocol() {
        let client = vec![ProtocolType::TCP, ProtocolType::UDP];
        let server = vec![ProtocolType::TCP, ProtocolType::TLS];

        let selected = ProtocolNegotiation::select_protocol(&client, &server);
        assert_eq!(selected, Some(ProtocolType::TCP));
    }

    #[test]
    fn test_protocol_negotiation_select_tls_priority() {
        let client = vec![ProtocolType::TCP, ProtocolType::TLS, ProtocolType::UDP];
        let server = vec![ProtocolType::TCP, ProtocolType::TLS];

        let selected = ProtocolNegotiation::select_protocol(&client, &server);
        assert_eq!(selected, Some(ProtocolType::TLS));
    }

    #[test]
    fn test_protocol_negotiation_no_common_protocol() {
        let client = vec![ProtocolType::TCP];
        let server = vec![ProtocolType::UDP];

        let selected = ProtocolNegotiation::select_protocol(&client, &server);
        assert_eq!(selected, None);
    }

    #[test]
    fn test_protocol_negotiation_empty_lists() {
        let client = vec![];
        let server = vec![ProtocolType::TCP];

        let selected = ProtocolNegotiation::select_protocol(&client, &server);
        assert_eq!(selected, None);
    }
}

// Property-Based Tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating MessageType
    fn message_type_strategy() -> impl Strategy<Value = MessageType> {
        prop_oneof![
            Just(MessageType::Ping),
            Just(MessageType::Pong),
            Just(MessageType::Data),
            Just(MessageType::Ack),
            Just(MessageType::Error),
            Just(MessageType::Heartbeat),
            Just(MessageType::ClusterJoin),
            Just(MessageType::ClusterLeave),
            Just(MessageType::Replication),
            Just(MessageType::Transaction),
        ]
    }

    // Strategy for generating valid Messages
    fn message_strategy() -> impl Strategy<Value = Message> {
        (
            any::<u64>(),                    // sender
            any::<u64>(),                    // recipient
            any::<u64>(),                    // sequence_number
            any::<i64>(),                    // timestamp
            message_type_strategy(),         // message_type
            prop::collection::vec(any::<u8>(), 0..1000), // payload (up to 1000 bytes)
        )
            .prop_map(|(sender, recipient, seq, ts, msg_type, payload)| {
                Message::new(sender, recipient, seq, ts, msg_type, payload)
            })
    }

    // Property 37: Message Serialization Round-Trip
    // Feature: client-sdk, Property 37: For any valid Message, encoding then decoding should produce an equivalent message
    // Validates: Requirements 13.1
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_message_serialization_round_trip(msg in message_strategy()) {
            let codec = MessageCodec::new();
            
            // Encode the message
            let encoded = codec.encode(&msg).expect("Encoding should succeed");
            
            // Decode the message
            let decoded = codec.decode(&encoded).expect("Decoding should succeed");
            
            // Verify all fields match
            prop_assert_eq!(msg.sender, decoded.sender);
            prop_assert_eq!(msg.recipient, decoded.recipient);
            prop_assert_eq!(msg.sequence_number, decoded.sequence_number);
            prop_assert_eq!(msg.timestamp, decoded.timestamp);
            prop_assert_eq!(msg.message_type, decoded.message_type);
            prop_assert_eq!(msg.payload, decoded.payload);
            prop_assert_eq!(msg.checksum, decoded.checksum);
        }
    }

    // Property 38: Checksum Validation Detects Corruption
    // Feature: client-sdk, Property 38: For any message with corrupted data, checksum validation should detect the corruption
    // Validates: Requirements 13.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_checksum_detects_corruption(
            msg in message_strategy(),
            corruption_index in 0usize..100,
        ) {
            let codec = MessageCodec::new();
            
            // Encode the message
            let mut encoded = codec.encode(&msg).expect("Encoding should succeed");
            
            // Skip test if message is too small to corrupt meaningfully
            if encoded.len() < 10 {
                return Ok(());
            }
            
            // Corrupt a byte in the encoded data (but not in a way that breaks deserialization structure)
            let idx = corruption_index % encoded.len();
            encoded[idx] ^= 0xFF;
            
            // Try to decode - should either fail deserialization or checksum validation
            let result = codec.decode(&encoded);
            
            // We expect either a serialization error or checksum mismatch
            prop_assert!(
                result.is_err(),
                "Corrupted message should fail validation"
            );
        }
    }

    // Property 39: Length-Prefixed Framing
    // Feature: client-sdk, Property 39: For any message, the length prefix should correctly represent the message size
    // Validates: Requirements 13.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_length_prefixed_framing(msg in message_strategy()) {
            let codec = MessageCodec::new();
            
            // Encode with length prefix
            let encoded_with_length = codec.encode_with_length(&msg).expect("Encoding should succeed");
            
            // Verify we have at least 4 bytes for the length prefix
            prop_assert!(encoded_with_length.len() >= 4);
            
            // Extract the length prefix
            let length = u32::from_be_bytes([
                encoded_with_length[0],
                encoded_with_length[1],
                encoded_with_length[2],
                encoded_with_length[3],
            ]) as usize;
            
            // Verify the length matches the actual message size
            prop_assert_eq!(length, encoded_with_length.len() - 4);
            
            // Verify we can decode the message correctly
            let message_data = &encoded_with_length[4..];
            let decoded = codec.decode(message_data).expect("Decoding should succeed");
            
            prop_assert_eq!(msg.sender, decoded.sender);
            prop_assert_eq!(msg.payload, decoded.payload);
        }
    }

    // Property 40: Message Size Limit Enforcement
    // Feature: client-sdk, Property 40: Messages exceeding max_message_size should be rejected
    // Validates: Requirements 13.5
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_message_size_limit_enforcement(
            sender in any::<u64>(),
            recipient in any::<u64>(),
            seq in any::<u64>(),
            ts in any::<i64>(),
            msg_type in message_type_strategy(),
            payload_size in 0usize..2000,
        ) {
            let max_size = 500; // Small limit for testing
            let codec = MessageCodec::with_max_size(max_size);
            
            // Create a message with the specified payload size
            let payload = vec![0u8; payload_size];
            let msg = Message::new(sender, recipient, seq, ts, msg_type, payload);
            
            // Try to encode the message
            let result = codec.encode(&msg);
            
            // If the encoded size would exceed the limit, it should fail
            if let Ok(encoded) = &result {
                prop_assert!(
                    encoded.len() <= max_size,
                    "Encoded message size {} should not exceed max_size {}",
                    encoded.len(),
                    max_size
                );
            } else {
                // If it failed, verify it's a MessageTooLarge error
                prop_assert!(
                    matches!(result, Err(DatabaseError::MessageTooLarge { .. })),
                    "Oversized message should return MessageTooLarge error"
                );
            }
        }
    }
}
