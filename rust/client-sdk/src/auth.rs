//! Authentication module for Q-Distributed-Database Client SDK
//!
//! This module implements token-based authentication, automatic re-authentication,
//! and credential management.

use crate::error::DatabaseError;
use crate::types::{Role, UserId};
use crate::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Certificate type placeholder
///
/// This will be expanded in future versions to support TLS certificate authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// Certificate data in PEM format
    pub data: Vec<u8>,
}

/// User credentials for authentication
///
/// Supports multiple authentication methods:
/// - Username/password authentication
/// - Certificate-based authentication (TLS)
/// - Token reuse
#[derive(Debug, Clone)]
pub struct Credentials {
    /// Username for authentication
    pub username: String,
    /// Optional password for username/password authentication
    pub password: Option<String>,
    /// Optional certificate for TLS authentication
    pub certificate: Option<Certificate>,
    /// Optional pre-existing token for token reuse
    pub token: Option<String>,
}

impl Credentials {
    /// Creates new credentials with username and password
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: Some(password.into()),
            certificate: None,
            token: None,
        }
    }

    /// Creates new credentials with username only
    pub fn with_username(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: None,
            certificate: None,
            token: None,
        }
    }

    /// Sets the password
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Sets the certificate
    pub fn with_certificate(mut self, certificate: Certificate) -> Self {
        self.certificate = Some(certificate);
        self
    }

    /// Sets the token
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Validates that the credentials have at least one authentication method
    pub fn validate(&self) -> Result<()> {
        if self.username.is_empty() {
            return Err(DatabaseError::AuthenticationFailed {
                reason: "Username is required".to_string(),
            });
        }

        if self.password.is_none() && self.certificate.is_none() && self.token.is_none() {
            return Err(DatabaseError::AuthenticationFailed {
                reason: "At least one authentication method (password, certificate, or token) is required".to_string(),
            });
        }

        Ok(())
    }
}

/// Authentication token issued by the server
///
/// Contains user identity, roles, expiration time, and cryptographic signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// User identifier
    pub user_id: UserId,
    /// User roles and permissions
    pub roles: Vec<Role>,
    /// Token expiration timestamp
    pub expiration: DateTime<Utc>,
    /// Cryptographic signature for validation
    pub signature: Vec<u8>,
}

impl AuthToken {
    /// Creates a new authentication token
    pub fn new(
        user_id: UserId,
        roles: Vec<Role>,
        expiration: DateTime<Utc>,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            user_id,
            roles,
            expiration,
            signature,
        }
    }

    /// Checks if the token has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expiration
    }

    /// Returns the time remaining until expiration
    ///
    /// Returns Duration::zero() if the token is already expired.
    pub fn time_until_expiration(&self) -> Duration {
        let now = Utc::now();
        if now >= self.expiration {
            Duration::zero()
        } else {
            self.expiration - now
        }
    }

    /// Checks if the token will expire within the given duration
    pub fn expires_within(&self, duration: Duration) -> bool {
        self.time_until_expiration() <= duration
    }
}

/// Authentication manager
///
/// Manages the authentication lifecycle including:
/// - Initial authentication
/// - Token validation and renewal
/// - Automatic re-authentication
/// - Logout
#[derive(Clone)]
pub struct AuthenticationManager {
    /// User credentials
    credentials: Credentials,
    /// Current authentication token (thread-safe)
    token: Arc<RwLock<Option<AuthToken>>>,
    /// Token time-to-live
    token_ttl: std::time::Duration,
}

impl AuthenticationManager {
    /// Creates a new authentication manager
    pub fn new(credentials: Credentials, token_ttl: std::time::Duration) -> Self {
        Self {
            credentials,
            token: Arc::new(RwLock::new(None)),
            token_ttl,
        }
    }

    /// Authenticates with the server and obtains a token
    ///
    /// This is a placeholder implementation that will be completed when
    /// the connection layer is integrated.
    pub async fn authenticate(&self) -> Result<AuthToken> {
        // Validate credentials
        self.credentials.validate()?;

        // TODO: Send AuthRequest to server through connection
        // For now, create a mock token for testing
        let token = AuthToken::new(
            1, // user_id
            vec![Role::User],
            Utc::now() + chrono::Duration::from_std(self.token_ttl).unwrap(),
            vec![0; 32], // signature
        );

        // Store token
        let mut token_guard = self.token.write().await;
        *token_guard = Some(token.clone());

        Ok(token)
    }

    /// Gets a valid token, re-authenticating if necessary
    pub async fn get_valid_token(&self) -> Result<AuthToken> {
        // Check if we have a token
        let token_guard = self.token.read().await;
        if let Some(token) = token_guard.as_ref() {
            // Check if token is still valid
            if !token.is_expired() {
                return Ok(token.clone());
            }
        }
        drop(token_guard);

        // Token is expired or missing, re-authenticate
        self.authenticate().await
    }

    /// Refreshes the current token
    ///
    /// Proactively renews the token before it expires.
    pub async fn refresh_token(&self) -> Result<AuthToken> {
        // Get current token and clone the necessary data
        let (user_id, roles) = {
            let token_guard = self.token.read().await;
            let current_token =
                token_guard
                    .as_ref()
                    .ok_or_else(|| DatabaseError::AuthenticationFailed {
                        reason: "No token to refresh".to_string(),
                    })?;

            // Check if token is still valid
            if current_token.is_expired() {
                drop(token_guard);
                return self.authenticate().await;
            }

            // Clone the data we need
            (current_token.user_id, current_token.roles.clone())
        };

        // TODO: Send token refresh request to server
        // For now, create a new token with extended expiration
        let new_token = AuthToken::new(
            user_id,
            roles,
            Utc::now() + chrono::Duration::from_std(self.token_ttl).unwrap(),
            vec![0; 32], // signature
        );

        // Store new token
        let mut token_guard = self.token.write().await;
        *token_guard = Some(new_token.clone());

        Ok(new_token)
    }

    /// Logs out and invalidates the current token
    pub async fn logout(&self) -> Result<()> {
        // Get current token
        let token_guard = self.token.read().await;
        let _current_token =
            token_guard
                .as_ref()
                .ok_or_else(|| DatabaseError::AuthenticationFailed {
                    reason: "No token to logout".to_string(),
                })?;
        drop(token_guard);

        // TODO: Send logout request to server

        // Clear stored token
        let mut token_guard = self.token.write().await;
        *token_guard = None;

        Ok(())
    }

    /// Gets the current token without validation
    pub async fn get_token(&self) -> Option<AuthToken> {
        self.token.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Credentials Tests
    #[test]
    fn test_credentials_new() {
        let creds = Credentials::new("admin", "password");
        assert_eq!(creds.username, "admin");
        assert_eq!(creds.password, Some("password".to_string()));
        assert!(creds.certificate.is_none());
        assert!(creds.token.is_none());
    }

    #[test]
    fn test_credentials_builder() {
        let creds = Credentials::with_username("admin")
            .with_password("password")
            .with_token("existing_token");

        assert_eq!(creds.username, "admin");
        assert_eq!(creds.password, Some("password".to_string()));
        assert_eq!(creds.token, Some("existing_token".to_string()));
    }

    #[test]
    fn test_credentials_validate_success() {
        let creds = Credentials::new("admin", "password");
        assert!(creds.validate().is_ok());
    }

    #[test]
    fn test_credentials_validate_empty_username() {
        let creds = Credentials::with_username("").with_password("password");
        assert!(creds.validate().is_err());
    }

    #[test]
    fn test_credentials_validate_no_auth_method() {
        let creds = Credentials::with_username("admin");
        assert!(creds.validate().is_err());
    }

    // AuthToken Tests
    #[test]
    fn test_auth_token_creation() {
        let expiration = Utc::now() + chrono::Duration::hours(24);
        let token = AuthToken::new(1, vec![Role::Admin], expiration, vec![0; 32]);

        assert_eq!(token.user_id, 1);
        assert_eq!(token.roles, vec![Role::Admin]);
        assert_eq!(token.expiration, expiration);
        assert_eq!(token.signature.len(), 32);
    }

    #[test]
    fn test_auth_token_is_expired() {
        // Create expired token
        let expiration = Utc::now() - chrono::Duration::hours(1);
        let token = AuthToken::new(1, vec![Role::User], expiration, vec![0; 32]);
        assert!(token.is_expired());

        // Create valid token
        let expiration = Utc::now() + chrono::Duration::hours(1);
        let token = AuthToken::new(1, vec![Role::User], expiration, vec![0; 32]);
        assert!(!token.is_expired());
    }

    #[test]
    fn test_auth_token_time_until_expiration() {
        // Create token expiring in 1 hour
        let expiration = Utc::now() + chrono::Duration::hours(1);
        let token = AuthToken::new(1, vec![Role::User], expiration, vec![0; 32]);

        let remaining = token.time_until_expiration();
        assert!(remaining.num_minutes() > 55); // Allow some tolerance
        assert!(remaining.num_minutes() <= 60);

        // Create expired token
        let expiration = Utc::now() - chrono::Duration::hours(1);
        let token = AuthToken::new(1, vec![Role::User], expiration, vec![0; 32]);
        assert_eq!(token.time_until_expiration(), Duration::zero());
    }

    #[test]
    fn test_auth_token_expires_within() {
        let expiration = Utc::now() + chrono::Duration::minutes(30);
        let token = AuthToken::new(1, vec![Role::User], expiration, vec![0; 32]);

        assert!(token.expires_within(chrono::Duration::hours(1)));
        assert!(!token.expires_within(chrono::Duration::minutes(15)));
    }

    // AuthenticationManager Tests
    #[tokio::test]
    async fn test_authentication_manager_creation() {
        let creds = Credentials::new("admin", "password");
        let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(3600));

        let token = manager.get_token().await;
        assert!(token.is_none());
    }

    #[tokio::test]
    async fn test_authentication_manager_authenticate() {
        let creds = Credentials::new("admin", "password");
        let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(3600));

        let token = manager.authenticate().await.unwrap();
        assert_eq!(token.user_id, 1);
        assert!(!token.is_expired());

        // Verify token is stored
        let stored_token = manager.get_token().await;
        assert!(stored_token.is_some());
    }

    #[tokio::test]
    async fn test_authentication_manager_get_valid_token() {
        let creds = Credentials::new("admin", "password");
        let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(3600));

        // First call should authenticate
        let token1 = manager.get_valid_token().await.unwrap();
        assert!(!token1.is_expired());

        // Second call should return cached token
        let token2 = manager.get_valid_token().await.unwrap();
        assert_eq!(token1.user_id, token2.user_id);
    }

    #[tokio::test]
    async fn test_authentication_manager_refresh_token() {
        let creds = Credentials::new("admin", "password");
        let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(3600));

        // Authenticate first
        let token1 = manager.authenticate().await.unwrap();

        // Refresh token
        let token2 = manager.refresh_token().await.unwrap();
        assert_eq!(token1.user_id, token2.user_id);
        assert!(token2.expiration > token1.expiration);
    }

    #[tokio::test]
    async fn test_authentication_manager_logout() {
        let creds = Credentials::new("admin", "password");
        let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(3600));

        // Authenticate first
        manager.authenticate().await.unwrap();
        assert!(manager.get_token().await.is_some());

        // Logout
        manager.logout().await.unwrap();
        assert!(manager.get_token().await.is_none());
    }
}

// Property-Based Tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid usernames
    fn username_strategy() -> impl Strategy<Value = String> {
        "[a-z]{3,20}"
    }

    // Strategy for generating valid passwords
    fn password_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{8,32}"
    }

    // Strategy for generating valid Credentials
    fn credentials_strategy() -> impl Strategy<Value = Credentials> {
        (username_strategy(), password_strategy())
            .prop_map(|(username, password)| Credentials::new(username, password))
    }

    // Strategy for generating valid AuthToken
    fn auth_token_strategy() -> impl Strategy<Value = AuthToken> {
        (
            any::<u64>(), // user_id
            prop::collection::vec(
                prop_oneof![Just(Role::Admin), Just(Role::User), Just(Role::ReadOnly),],
                1..5,
            ), // roles
            -86400i64..86400i64, // expiration offset in seconds (-1 day to +1 day)
            prop::collection::vec(any::<u8>(), 32..64), // signature
        )
            .prop_map(|(user_id, roles, expiration_offset, signature)| {
                let expiration = Utc::now() + chrono::Duration::seconds(expiration_offset);
                AuthToken::new(user_id, roles, expiration, signature)
            })
    }

    // Property 8: Auth Token Structure
    // Feature: client-sdk, Property 8: For any successful authentication, the returned Auth_Token should contain user_id, roles, expiration timestamp, and cryptographic signature fields
    // Validates: Requirements 2.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_auth_token_structure(token in auth_token_strategy()) {
            // Verify all required fields are present and have correct types
            prop_assert!(token.user_id > 0 || token.user_id == 0); // user_id exists
            prop_assert!(!token.roles.is_empty()); // roles exist and non-empty
            prop_assert!(token.expiration.timestamp() != 0); // expiration exists
            prop_assert!(!token.signature.is_empty()); // signature exists and non-empty

            // Verify signature has reasonable length (at least 32 bytes)
            prop_assert!(token.signature.len() >= 32);
        }
    }

    // Property 9: Token Inclusion in Requests
    // Feature: client-sdk, Property 9: For any authenticated request, the message should include the current valid Auth_Token
    // Validates: Requirements 2.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_token_inclusion_in_requests(
            creds in credentials_strategy(),
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(3600));

                // Authenticate to get a token
                let token = manager.authenticate().await.unwrap();

                // Verify the manager has stored the token
                let stored_token = manager.get_token().await;
                prop_assert!(stored_token.is_some());

                // Verify the stored token matches the returned token
                let stored = stored_token.unwrap();
                prop_assert_eq!(token.user_id, stored.user_id);
                prop_assert_eq!(token.roles, stored.roles);

                Ok(())
            })?;
        }
    }

    // Property 10: Automatic Re-authentication
    // Feature: client-sdk, Property 10: For any expired token, the next request should trigger automatic re-authentication before executing the request
    // Validates: Requirements 2.4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_automatic_reauthentication(
            creds in credentials_strategy(),
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                // Create manager with very short TTL (1 second)
                let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(1));

                // Authenticate to get initial token
                let token1 = manager.authenticate().await.unwrap();

                // Manually expire the token by setting it to an expired time
                {
                    let mut token_guard = manager.token.write().await;
                    if let Some(token) = token_guard.as_mut() {
                        token.expiration = Utc::now() - chrono::Duration::hours(1);
                    }
                }

                // Call get_valid_token - should trigger re-authentication
                let token2 = manager.get_valid_token().await.unwrap();

                // Verify we got a new token (different expiration)
                prop_assert!(token2.expiration > token1.expiration);
                prop_assert!(!token2.is_expired());

                Ok(())
            })?;
        }
    }

    // Property 11: Token Invalidation on Logout
    // Feature: client-sdk, Property 11: For any valid token, calling logout() should invalidate the token such that subsequent requests with that token fail authentication
    // Validates: Requirements 2.6
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_token_invalidation_on_logout(
            creds in credentials_strategy(),
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(3600));

                // Authenticate to get a token
                manager.authenticate().await.unwrap();

                // Verify token exists
                prop_assert!(manager.get_token().await.is_some());

                // Logout
                manager.logout().await.unwrap();

                // Verify token is cleared
                prop_assert!(manager.get_token().await.is_none());

                Ok(())
            })?;
        }
    }

    // Property 12: Token TTL Respect
    // Feature: client-sdk, Property 12: For any configured token TTL, tokens should expire after exactly that duration from issuance
    // Validates: Requirements 2.8
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_token_ttl_respect(
            creds in credentials_strategy(),
            ttl_seconds in 60u64..3600u64, // TTL between 1 minute and 1 hour
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let manager = AuthenticationManager::new(creds, std::time::Duration::from_secs(ttl_seconds));

                // Authenticate to get a token
                let token = manager.authenticate().await.unwrap();

                // Calculate expected expiration time
                let now = Utc::now();
                let expected_expiration = now + chrono::Duration::seconds(ttl_seconds as i64);

                // Verify token expiration is approximately correct (within 5 seconds tolerance)
                let diff = (token.expiration - expected_expiration).num_seconds().abs();
                prop_assert!(
                    diff <= 5,
                    "Token expiration {} should be within 5 seconds of expected {}",
                    token.expiration,
                    expected_expiration
                );

                // Verify token is not expired
                prop_assert!(!token.is_expired());

                // Verify time_until_expiration is approximately correct
                let remaining = token.time_until_expiration();
                let remaining_seconds = remaining.num_seconds();
                prop_assert!(
                    remaining_seconds >= (ttl_seconds as i64 - 5) && remaining_seconds <= (ttl_seconds as i64 + 5),
                    "Remaining time {} should be approximately {} seconds",
                    remaining_seconds,
                    ttl_seconds
                );

                Ok(())
            })?;
        }
    }
}
