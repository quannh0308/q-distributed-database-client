//! Main client entry point for Q-Distributed-Database Client SDK

use crate::admin_client::AdminClient;
use crate::auth::{AuthenticationManager, Credentials};
use crate::connection::{ConnectionManager, NodeHealth};
use crate::data_client::DataClient;
use crate::error::DatabaseError;
use crate::types::ConnectionConfig;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Cluster health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealth {
    /// Total number of nodes in the cluster
    pub total_nodes: usize,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Health information for each node
    pub node_healths: Vec<NodeHealth>,
}

/// Main client for interacting with the database
pub struct Client {
    /// Connection configuration
    config: ConnectionConfig,
    /// Connection manager
    connection_manager: Arc<ConnectionManager>,
    /// Authentication manager
    auth_manager: Arc<AuthenticationManager>,
    /// Data client for CRUD operations
    data_client: DataClient,
    /// Admin client for cluster and user management
    admin_client: AdminClient,
}

impl Client {
    /// Connects to the database cluster
    ///
    /// This method initializes all components and establishes connections to the database.
    /// It performs the following steps:
    /// 1. Validates the configuration
    /// 2. Creates the ConnectionManager
    /// 3. Creates the AuthenticationManager and performs initial authentication
    /// 4. Creates the DataClient with shared managers
    /// 5. Creates the AdminClient with shared managers
    ///
    /// # Arguments
    ///
    /// * `config` - Connection configuration including hosts, credentials, and settings
    ///
    /// # Returns
    ///
    /// Returns an initialized Client instance ready for use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - Connection to database nodes fails
    /// - Initial authentication fails
    pub async fn connect(config: ConnectionConfig) -> Result<Self> {
        // 1. Validate configuration
        config.validate()?;

        // 2. Create ConnectionManager
        let connection_manager = Arc::new(ConnectionManager::new(config.clone()));

        // 3. Create AuthenticationManager with credentials
        let credentials = if let Some(password) = &config.password {
            Credentials::new(config.username.clone(), password.clone())
        } else if let Some(cert_data) = &config.certificate {
            Credentials::with_username(config.username.clone())
                .with_certificate(crate::auth::Certificate {
                    data: cert_data.clone(),
                })
        } else {
            return Err(DatabaseError::AuthenticationFailed {
                reason: "No password or certificate provided".to_string(),
            });
        };

        let auth_manager = Arc::new(AuthenticationManager::new(
            credentials,
            std::time::Duration::from_secs(86400), // 24 hours default TTL
        ));

        // 4. Perform initial authentication
        let conn = connection_manager.get_connection().await?;
        auth_manager.authenticate().await?;
        connection_manager.return_connection(conn).await;

        // 5. Create DataClient with shared managers
        let data_client = DataClient::new(
            Arc::clone(&connection_manager),
            Arc::clone(&auth_manager),
        );

        // 6. Create AdminClient with shared managers
        let admin_client = AdminClient::new(
            Arc::clone(&connection_manager),
            Arc::clone(&auth_manager),
        );

        Ok(Self {
            config,
            connection_manager,
            auth_manager,
            data_client,
            admin_client,
        })
    }

    /// Returns a reference to the data client
    ///
    /// The data client provides access to CRUD operations, query execution,
    /// and transaction management.
    pub fn data(&self) -> &DataClient {
        &self.data_client
    }

    /// Returns a reference to the admin client
    ///
    /// The admin client provides access to cluster management and user
    /// management operations.
    pub fn admin(&self) -> &AdminClient {
        &self.admin_client
    }

    /// Returns a reference to the connection configuration
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// Checks the health of all nodes in the cluster
    ///
    /// Queries health status from all configured nodes and aggregates the results
    /// into a ClusterHealth structure.
    ///
    /// # Returns
    ///
    /// Returns cluster health information including:
    /// - Total number of nodes
    /// - Number of healthy nodes
    /// - Individual node health status
    ///
    /// # Errors
    ///
    /// Returns an error if health check fails for all nodes
    pub async fn health_check(&self) -> Result<ClusterHealth> {
        // Query health from all nodes via ConnectionManager
        let node_healths = self
            .connection_manager
            .health_check_all_nodes()
            .await?;

        // Aggregate results into cluster health
        let total_nodes = node_healths.len();
        let healthy_nodes = node_healths
            .iter()
            .filter(|h| h.is_healthy)
            .count();

        Ok(ClusterHealth {
            total_nodes,
            healthy_nodes,
            node_healths,
        })
    }

    /// Disconnects from the database cluster gracefully
    ///
    /// This method performs the following cleanup steps:
    /// 1. Logs out to invalidate the authentication token (best effort)
    /// 2. Closes all connections in the connection pool
    /// 3. Releases all resources
    ///
    /// # Errors
    ///
    /// Returns an error if logout or connection cleanup fails.
    /// Note: Logout errors are logged but don't prevent disconnection.
    pub async fn disconnect(self) -> Result<()> {
        // 1. Logout to invalidate token (best effort)
        let conn = self.connection_manager.get_connection().await?;
        if let Err(e) = self.auth_manager.logout().await {
            eprintln!("Warning: logout failed during disconnect: {}", e);
        }
        self.connection_manager.return_connection(conn).await;

        // 2. Close all connections in the pool
        self.connection_manager.disconnect().await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_health_creation() {
        let node_health = NodeHealth::new(1);
        let cluster_health = ClusterHealth {
            total_nodes: 3,
            healthy_nodes: 2,
            node_healths: vec![node_health],
        };

        assert_eq!(cluster_health.total_nodes, 3);
        assert_eq!(cluster_health.healthy_nodes, 2);
        assert_eq!(cluster_health.node_healths.len(), 1);
    }

    #[test]
    fn test_cluster_health_all_healthy() {
        let node1 = NodeHealth::new(1);
        let node2 = NodeHealth::new(2);
        let node3 = NodeHealth::new(3);

        let cluster_health = ClusterHealth {
            total_nodes: 3,
            healthy_nodes: 3,
            node_healths: vec![node1, node2, node3],
        };

        assert_eq!(cluster_health.total_nodes, cluster_health.healthy_nodes);
    }

    #[test]
    fn test_cluster_health_partial_healthy() {
        let mut node1 = NodeHealth::new(1);
        node1.mark_healthy();

        let mut node2 = NodeHealth::new(2);
        node2.mark_unhealthy();

        let cluster_health = ClusterHealth {
            total_nodes: 2,
            healthy_nodes: 1,
            node_healths: vec![node1, node2],
        };

        assert_eq!(cluster_health.total_nodes, 2);
        assert_eq!(cluster_health.healthy_nodes, 1);
        assert!(cluster_health.node_healths[0].is_healthy);
        assert!(!cluster_health.node_healths[1].is_healthy);
    }
}
