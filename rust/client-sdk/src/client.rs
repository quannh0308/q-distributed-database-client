//! Main client entry point for Q-Distributed-Database Client SDK

use crate::auth::{AuthenticationManager, Credentials};
use crate::connection::ConnectionManager;
use crate::data_client::DataClient;
use crate::types::ConnectionConfig;
use crate::Result;
use std::sync::Arc;

/// Main client for interacting with the database
pub struct Client {
    /// Connection manager
    connection_manager: Arc<ConnectionManager>,
    /// Authentication manager
    auth_manager: Arc<AuthenticationManager>,
    /// Data client for CRUD operations
    data_client: DataClient,
}

impl Client {
    /// Connects to the database cluster
    pub async fn connect(config: ConnectionConfig) -> Result<Self> {
        // Create connection manager
        let connection_manager = Arc::new(ConnectionManager::new(config.clone()));

        // Create credentials
        let credentials = Credentials::new(
            config.username.clone(),
            config.password.clone().unwrap_or_default(),
        );

        // Create authentication manager
        let auth_manager = Arc::new(AuthenticationManager::new(
            credentials,
            std::time::Duration::from_secs(86400), // 24 hours
        ));

        // Create data client
        let data_client = DataClient::new(connection_manager.clone(), auth_manager.clone());

        Ok(Self {
            connection_manager,
            auth_manager,
            data_client,
        })
    }

    /// Returns a reference to the data client
    pub fn data(&self) -> &DataClient {
        &self.data_client
    }

    /// Disconnects from the database cluster
    pub async fn disconnect(self) {
        self.connection_manager.disconnect().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // This is a basic structure test
        // Full integration tests require a running database server
    }
}
