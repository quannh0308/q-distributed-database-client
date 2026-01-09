//! Error types for the Q-Distributed-Database Client SDK
//!
//! This module defines a comprehensive error hierarchy for all possible
//! failure modes in the client SDK.

use thiserror::Error;

/// The main error type for the Q-Distributed-Database Client SDK
///
/// This enum covers all possible error conditions that can occur during
/// client operations, from connection failures to query errors.
#[derive(Error, Debug, Clone)]
pub enum DatabaseError {
    // Connection Errors
    /// Connection attempt timed out
    #[error("Connection timeout to {host} after {timeout_ms}ms")]
    ConnectionTimeout {
        /// The host that failed to connect
        host: String,
        /// The timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// Connection was refused by the server
    #[error("Connection refused by {host}")]
    ConnectionRefused {
        /// The host that refused the connection
        host: String,
    },

    /// Connection was lost during operation
    #[error("Connection lost to node {node_id}")]
    ConnectionLost {
        /// The node ID that lost connection
        node_id: u64,
    },

    // Authentication Errors
    /// Authentication failed
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed {
        /// The reason for authentication failure
        reason: String,
    },

    /// Authentication token has expired
    #[error("Token expired at {expired_at}")]
    TokenExpired {
        /// The timestamp when the token expired
        expired_at: i64,
    },

    /// Invalid credentials provided
    #[error("Invalid credentials")]
    InvalidCredentials,

    // Query Errors
    /// SQL syntax error
    #[error("Syntax error in SQL at position {position}: {message}\nSQL: {sql}")]
    SyntaxError {
        /// The SQL query that failed
        sql: String,
        /// The position in the SQL where the error occurred
        position: usize,
        /// The error message
        message: String,
    },

    /// Table not found in database
    #[error("Table not found: {table_name}")]
    TableNotFound {
        /// The name of the table that was not found
        table_name: String,
    },

    /// Column not found in table
    #[error("Column not found: {column_name}")]
    ColumnNotFound {
        /// The name of the column that was not found
        column_name: String,
    },

    /// Database constraint violation
    #[error("Constraint violation: {constraint} - {details}")]
    ConstraintViolation {
        /// The constraint that was violated
        constraint: String,
        /// Additional details about the violation
        details: String,
    },

    // Transaction Errors
    /// Transaction was aborted
    #[error("Transaction {transaction_id} aborted: {reason}")]
    TransactionAborted {
        /// The transaction ID that was aborted
        transaction_id: u64,
        /// The reason for the abort
        reason: String,
    },

    /// Deadlock detected during transaction
    #[error("Deadlock detected in transaction {transaction_id}")]
    DeadlockDetected {
        /// The transaction ID involved in the deadlock
        transaction_id: u64,
    },

    /// Transaction isolation level violation
    #[error("Isolation violation: {details}")]
    IsolationViolation {
        /// Details about the isolation violation
        details: String,
    },

    // Protocol Errors
    /// Message serialization failed
    #[error("Serialization error: {message}")]
    SerializationError {
        /// The error message
        message: String,
    },

    /// Message checksum mismatch
    #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch {
        /// The expected checksum value
        expected: u32,
        /// The actual checksum value
        actual: u32,
    },

    /// Message exceeds maximum size
    #[error("Message too large: {size} bytes (max: {max_size} bytes)")]
    MessageTooLarge {
        /// The actual message size
        size: usize,
        /// The maximum allowed message size
        max_size: usize,
    },

    /// Protocol version mismatch
    #[error("Protocol version mismatch: client v{client_version}, server v{server_version}")]
    ProtocolVersionMismatch {
        /// The client protocol version
        client_version: u8,
        /// The server protocol version
        server_version: u8,
    },

    // Network Errors
    /// Generic network error
    #[error("Network error: {details}")]
    NetworkError {
        /// Details about the network error
        details: String,
    },

    /// Operation timed out
    #[error("Operation '{operation}' timed out after {timeout_ms}ms")]
    TimeoutError {
        /// The operation that timed out
        operation: String,
        /// The timeout duration in milliseconds
        timeout_ms: u64,
    },

    // Internal Errors
    /// Internal SDK error
    #[error("Internal error in {component}: {details}")]
    InternalError {
        /// The component where the error occurred
        component: String,
        /// Details about the internal error
        details: String,
    },

    // Admin Errors
    /// Node not found
    #[error("Node not found: {node_id}")]
    NodeNotFound {
        /// The node ID that was not found
        node_id: u64,
    },

    /// Node already exists
    #[error("Node already exists: {hostname}")]
    NodeAlreadyExists {
        /// The hostname of the existing node
        hostname: String,
    },

    /// Insufficient permissions
    #[error("Insufficient permissions: {required:?} required")]
    InsufficientPermissions {
        /// The required permission
        required: String,
    },

    /// User not found
    #[error("User not found: {user_id}")]
    UserNotFound {
        /// The user ID that was not found
        user_id: u64,
    },

    /// User already exists
    #[error("User already exists: {username}")]
    UserAlreadyExists {
        /// The username that already exists
        username: String,
    },

    /// Invalid role
    #[error("Invalid role: {role}")]
    InvalidRole {
        /// The invalid role name
        role: String,
    },

    /// Cannot remove last admin
    #[error("Cannot remove the last admin user")]
    CannotRemoveLastAdmin,
}

impl DatabaseError {
    /// Returns true if this error is retryable
    ///
    /// Retryable errors are transient failures that may succeed on retry,
    /// such as network timeouts or temporary connection issues.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            DatabaseError::ConnectionTimeout { .. }
                | DatabaseError::ConnectionLost { .. }
                | DatabaseError::NetworkError { .. }
                | DatabaseError::TimeoutError { .. }
        )
    }

    /// Returns true if this error is a connection error
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            DatabaseError::ConnectionTimeout { .. }
                | DatabaseError::ConnectionRefused { .. }
                | DatabaseError::ConnectionLost { .. }
        )
    }

    /// Returns true if this error is an authentication error
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            DatabaseError::AuthenticationFailed { .. }
                | DatabaseError::TokenExpired { .. }
                | DatabaseError::InvalidCredentials
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Connection Error Tests
    #[test]
    fn test_connection_timeout_display() {
        let err = DatabaseError::ConnectionTimeout {
            host: "localhost:7000".to_string(),
            timeout_ms: 5000,
        };
        assert_eq!(
            err.to_string(),
            "Connection timeout to localhost:7000 after 5000ms"
        );
    }

    #[test]
    fn test_connection_refused_display() {
        let err = DatabaseError::ConnectionRefused {
            host: "node1:7000".to_string(),
        };
        assert_eq!(err.to_string(), "Connection refused by node1:7000");
    }

    #[test]
    fn test_connection_lost_display() {
        let err = DatabaseError::ConnectionLost { node_id: 42 };
        assert_eq!(err.to_string(), "Connection lost to node 42");
    }

    // Authentication Error Tests
    #[test]
    fn test_authentication_failed_display() {
        let err = DatabaseError::AuthenticationFailed {
            reason: "Invalid token signature".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Authentication failed: Invalid token signature"
        );
    }

    #[test]
    fn test_token_expired_display() {
        let err = DatabaseError::TokenExpired {
            expired_at: 1704067200000,
        };
        assert_eq!(err.to_string(), "Token expired at 1704067200000");
    }

    #[test]
    fn test_invalid_credentials_display() {
        let err = DatabaseError::InvalidCredentials;
        assert_eq!(err.to_string(), "Invalid credentials");
    }

    // Query Error Tests
    #[test]
    fn test_syntax_error_display() {
        let err = DatabaseError::SyntaxError {
            sql: "SELECT * FORM users".to_string(),
            position: 9,
            message: "Expected FROM, got FORM".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Syntax error in SQL at position 9"));
        assert!(display.contains("Expected FROM, got FORM"));
        assert!(display.contains("SELECT * FORM users"));
    }

    #[test]
    fn test_table_not_found_display() {
        let err = DatabaseError::TableNotFound {
            table_name: "users".to_string(),
        };
        assert_eq!(err.to_string(), "Table not found: users");
    }

    #[test]
    fn test_column_not_found_display() {
        let err = DatabaseError::ColumnNotFound {
            column_name: "email".to_string(),
        };
        assert_eq!(err.to_string(), "Column not found: email");
    }

    #[test]
    fn test_constraint_violation_display() {
        let err = DatabaseError::ConstraintViolation {
            constraint: "unique_email".to_string(),
            details: "Duplicate email address".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Constraint violation: unique_email - Duplicate email address"
        );
    }

    // Transaction Error Tests
    #[test]
    fn test_transaction_aborted_display() {
        let err = DatabaseError::TransactionAborted {
            transaction_id: 123,
            reason: "Conflict detected".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Transaction 123 aborted: Conflict detected"
        );
    }

    #[test]
    fn test_deadlock_detected_display() {
        let err = DatabaseError::DeadlockDetected {
            transaction_id: 456,
        };
        assert_eq!(err.to_string(), "Deadlock detected in transaction 456");
    }

    #[test]
    fn test_isolation_violation_display() {
        let err = DatabaseError::IsolationViolation {
            details: "Read uncommitted data".to_string(),
        };
        assert_eq!(err.to_string(), "Isolation violation: Read uncommitted data");
    }

    // Protocol Error Tests
    #[test]
    fn test_serialization_error_display() {
        let err = DatabaseError::SerializationError {
            message: "Failed to encode message".to_string(),
        };
        assert_eq!(err.to_string(), "Serialization error: Failed to encode message");
    }

    #[test]
    fn test_checksum_mismatch_display() {
        let err = DatabaseError::ChecksumMismatch {
            expected: 0x12345678,
            actual: 0x87654321,
        };
        assert_eq!(
            err.to_string(),
            "Checksum mismatch: expected 0x12345678, got 0x87654321"
        );
    }

    #[test]
    fn test_message_too_large_display() {
        let err = DatabaseError::MessageTooLarge {
            size: 2_000_000,
            max_size: 1_048_576,
        };
        assert_eq!(
            err.to_string(),
            "Message too large: 2000000 bytes (max: 1048576 bytes)"
        );
    }

    #[test]
    fn test_protocol_version_mismatch_display() {
        let err = DatabaseError::ProtocolVersionMismatch {
            client_version: 2,
            server_version: 1,
        };
        assert_eq!(
            err.to_string(),
            "Protocol version mismatch: client v2, server v1"
        );
    }

    // Network Error Tests
    #[test]
    fn test_network_error_display() {
        let err = DatabaseError::NetworkError {
            details: "Connection reset by peer".to_string(),
        };
        assert_eq!(err.to_string(), "Network error: Connection reset by peer");
    }

    #[test]
    fn test_timeout_error_display() {
        let err = DatabaseError::TimeoutError {
            operation: "query".to_string(),
            timeout_ms: 5000,
        };
        assert_eq!(err.to_string(), "Operation 'query' timed out after 5000ms");
    }

    // Internal Error Tests
    #[test]
    fn test_internal_error_display() {
        let err = DatabaseError::InternalError {
            component: "ConnectionPool".to_string(),
            details: "Unexpected state".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Internal error in ConnectionPool: Unexpected state"
        );
    }

    // Error Classification Tests
    #[test]
    fn test_is_retryable() {
        // Retryable errors
        assert!(DatabaseError::ConnectionTimeout {
            host: "localhost:7000".to_string(),
            timeout_ms: 5000,
        }
        .is_retryable());

        assert!(DatabaseError::ConnectionLost { node_id: 1 }.is_retryable());

        assert!(DatabaseError::NetworkError {
            details: "Connection reset".to_string(),
        }
        .is_retryable());

        assert!(DatabaseError::TimeoutError {
            operation: "query".to_string(),
            timeout_ms: 5000,
        }
        .is_retryable());

        // Non-retryable errors
        assert!(!DatabaseError::InvalidCredentials.is_retryable());
        assert!(!DatabaseError::SyntaxError {
            sql: "BAD SQL".to_string(),
            position: 0,
            message: "Invalid".to_string(),
        }
        .is_retryable());
        assert!(!DatabaseError::TableNotFound {
            table_name: "users".to_string(),
        }
        .is_retryable());
    }

    #[test]
    fn test_is_connection_error() {
        // Connection errors
        assert!(DatabaseError::ConnectionTimeout {
            host: "localhost:7000".to_string(),
            timeout_ms: 5000,
        }
        .is_connection_error());

        assert!(DatabaseError::ConnectionRefused {
            host: "localhost:7000".to_string(),
        }
        .is_connection_error());

        assert!(DatabaseError::ConnectionLost { node_id: 1 }.is_connection_error());

        // Non-connection errors
        assert!(!DatabaseError::InvalidCredentials.is_connection_error());
        assert!(!DatabaseError::SyntaxError {
            sql: "BAD SQL".to_string(),
            position: 0,
            message: "Invalid".to_string(),
        }
        .is_connection_error());
    }

    #[test]
    fn test_is_auth_error() {
        // Auth errors
        assert!(DatabaseError::InvalidCredentials.is_auth_error());
        assert!(DatabaseError::AuthenticationFailed {
            reason: "Bad token".to_string(),
        }
        .is_auth_error());
        assert!(DatabaseError::TokenExpired {
            expired_at: 1234567890,
        }
        .is_auth_error());

        // Non-auth errors
        assert!(!DatabaseError::ConnectionTimeout {
            host: "localhost:7000".to_string(),
            timeout_ms: 5000,
        }
        .is_auth_error());
        assert!(!DatabaseError::SyntaxError {
            sql: "BAD SQL".to_string(),
            position: 0,
            message: "Invalid".to_string(),
        }
        .is_auth_error());
    }

    // Error Cloning Tests
    #[test]
    fn test_error_clone() {
        let err1 = DatabaseError::ConnectionTimeout {
            host: "localhost:7000".to_string(),
            timeout_ms: 5000,
        };
        let err2 = err1.clone();
        assert_eq!(err1.to_string(), err2.to_string());
    }
}
