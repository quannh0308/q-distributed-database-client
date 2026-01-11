//! Admin client for cluster and user management operations
//!
//! This module provides administrative capabilities for q-distributed-database,
//! including cluster management and user management operations.

use crate::auth::AuthenticationManager;
use crate::connection::ConnectionManager;
use crate::error::DatabaseError;
use crate::protocol::{AdminRequest, AdminResponse, Request, Response};
use crate::types::{
    ClusterMetrics, ClusterNodeInfo, NodeHealthMetrics, NodeId, Permission, Role, UserId, UserInfo,
    UserUpdate,
};
use crate::Result;
use std::sync::Arc;

/// Admin client for cluster and user management
///
/// Provides administrative operations including:
/// - Cluster management (nodes, health, rebalancing)
/// - User management (CRUD, permissions)
/// - Metrics and monitoring
///
/// # Example
///
/// ```ignore
/// // List cluster nodes
/// let nodes = client.admin().list_nodes().await?;
///
/// // Create a new user
/// let user_id = client.admin().create_user(
///     "developer",
///     "password",
///     &[Role::User]
/// ).await?;
///
/// // Grant permissions
/// client.admin().grant_permission(user_id, Permission::Read).await?;
/// ```
#[derive(Clone)]
pub struct AdminClient {
    /// Connection manager for database connections
    connection_manager: Arc<ConnectionManager>,
    /// Authentication manager for token management
    auth_manager: Arc<AuthenticationManager>,
}

impl AdminClient {
    /// Creates a new admin client
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        auth_manager: Arc<AuthenticationManager>,
    ) -> Self {
        Self {
            connection_manager,
            auth_manager,
        }
    }

    // ========================================================================
    // Cluster Management Operations
    // ========================================================================

    /// Lists all nodes in the cluster
    ///
    /// Returns information about all nodes including their status and role.
    pub async fn list_nodes(&self) -> Result<Vec<ClusterNodeInfo>> {
        // Get connection from pool
        let mut connection = self.connection_manager.get_connection().await?;

        // Get valid auth token
        let _token = self.auth_manager.get_valid_token().await?;

        // Build admin request
        let request = Request::Admin(AdminRequest::ListNodes);
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        // Send request
        let response = connection
            .connection_mut()
            .send_request(
                crate::protocol::MessageType::Data,
                payload,
                5000, // 5 second timeout
            )
            .await?;

        // Parse response
        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::NodeList(nodes)) => Ok(nodes),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Gets health metrics for a specific node
    ///
    /// Returns detailed health information including CPU, memory, disk usage,
    /// connection count, and query throughput.
    pub async fn get_node_health(&self, node_id: NodeId) -> Result<NodeHealthMetrics> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::GetNodeHealth { node_id });
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::NodeHealth(health)) => Ok(health),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Adds a new node to the cluster
    ///
    /// Initiates the node join process. The host parameter should be in the
    /// format "hostname:port" (e.g., "node3.example.com:7000").
    pub async fn add_node(&self, host: &str) -> Result<NodeId> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        // Parse host:port
        let (hostname, port) = parse_host_port(host)?;

        let request = Request::Admin(AdminRequest::AddNode {
            host: hostname.to_string(),
            port,
        });
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::NodeAdded(node_id)) => Ok(node_id),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Removes a node from the cluster
    ///
    /// Gracefully removes a node from the cluster. Data will be migrated
    /// before the node is removed.
    pub async fn remove_node(&self, node_id: NodeId) -> Result<()> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::RemoveNode { node_id });
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::NodeRemoved) => Ok(()),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Triggers partition rebalancing across the cluster
    ///
    /// Rebalances data partitions to optimize distribution and performance.
    pub async fn rebalance_partitions(&self) -> Result<()> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::RebalancePartitions);
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::PartitionsRebalanced) => Ok(()),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Gets cluster-wide performance metrics
    ///
    /// Returns aggregated metrics including total queries, average latency,
    /// error rate, and storage usage.
    pub async fn get_cluster_metrics(&self) -> Result<ClusterMetrics> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::GetClusterMetrics);
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::ClusterMetrics(metrics)) => Ok(metrics),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    // ========================================================================
    // User Management Operations
    // ========================================================================

    /// Creates a new user account
    ///
    /// Creates a user with the specified username, password, and roles.
    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        roles: &[Role],
    ) -> Result<UserId> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::CreateUser {
            username: username.to_string(),
            password: password.to_string(),
            roles: roles.to_vec(),
        });
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::UserCreated(user_id)) => Ok(user_id),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Lists all user accounts
    ///
    /// Returns information about all users including their roles and permissions.
    pub async fn list_users(&self) -> Result<Vec<UserInfo>> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::ListUsers);
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::UserList(users)) => Ok(users),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Updates a user account
    ///
    /// Modifies user credentials or roles.
    pub async fn update_user(&self, user_id: UserId, update: UserUpdate) -> Result<()> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::UpdateUser { user_id, update });
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::UserUpdated) => Ok(()),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Deletes a user account
    ///
    /// Removes a user from the system. Active sessions will be invalidated.
    pub async fn delete_user(&self, user_id: UserId) -> Result<()> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::DeleteUser { user_id });
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::UserDeleted) => Ok(()),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Grants a permission to a user
    ///
    /// Adds a specific permission to the user's permission set.
    pub async fn grant_permission(&self, user_id: UserId, permission: Permission) -> Result<()> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::GrantPermission {
            user_id,
            permission,
        });
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::PermissionGranted) => Ok(()),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Revokes a permission from a user
    ///
    /// Removes a specific permission from the user's permission set.
    pub async fn revoke_permission(&self, user_id: UserId, permission: Permission) -> Result<()> {
        let mut connection = self.connection_manager.get_connection().await?;
        let _token = self.auth_manager.get_valid_token().await?;

        let request = Request::Admin(AdminRequest::RevokePermission {
            user_id,
            permission,
        });
        let payload =
            bincode::serialize(&request).map_err(|e| DatabaseError::SerializationError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        let response = connection
            .connection_mut()
            .send_request(crate::protocol::MessageType::Data, payload, 5000)
            .await?;

        let response: Response = bincode::deserialize(&response.payload).map_err(|e| {
            DatabaseError::SerializationError {
                message: format!("Failed to deserialize response: {}", e),
            }
        })?;

        match response {
            Response::Admin(AdminResponse::PermissionRevoked) => Ok(()),
            Response::Error(e) => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: e,
            }),
            _ => Err(DatabaseError::InternalError {
                component: "AdminClient".to_string(),
                details: "Unexpected response type".to_string(),
            }),
        }
    }
}

/// Parses a host:port string
fn parse_host_port(host: &str) -> Result<(&str, u16)> {
    let parts: Vec<&str> = host.split(':').collect();
    if parts.len() != 2 {
        return Err(DatabaseError::InternalError {
            component: "AdminClient".to_string(),
            details: format!("Invalid host format: {}. Expected 'hostname:port'", host),
        });
    }

    let hostname = parts[0];
    let port = parts[1]
        .parse::<u16>()
        .map_err(|_| DatabaseError::InternalError {
            component: "AdminClient".to_string(),
            details: format!("Invalid port number: {}", parts[1]),
        })?;

    Ok((hostname, port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NodeRole, NodeStatus, Permission, Role};
    use chrono::Utc;

    #[test]
    fn test_parse_host_port_valid() {
        let result = parse_host_port("localhost:7000");
        assert!(result.is_ok());
        let (hostname, port) = result.unwrap();
        assert_eq!(hostname, "localhost");
        assert_eq!(port, 7000);
    }

    #[test]
    fn test_parse_host_port_with_domain() {
        let result = parse_host_port("node1.example.com:8080");
        assert!(result.is_ok());
        let (hostname, port) = result.unwrap();
        assert_eq!(hostname, "node1.example.com");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_host_port_invalid_format() {
        let result = parse_host_port("localhost");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_host_port_invalid_port() {
        let result = parse_host_port("localhost:invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_host_port_empty() {
        let result = parse_host_port("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_host_port_port_out_of_range() {
        let result = parse_host_port("localhost:99999");
        assert!(result.is_err());
    }

    // Test data type creation
    #[test]
    fn test_cluster_node_info_creation() {
        let node = ClusterNodeInfo {
            node_id: 1,
            hostname: "node1.example.com".to_string(),
            port: 7000,
            status: NodeStatus::Healthy,
            role: NodeRole::Primary,
        };

        assert_eq!(node.node_id, 1);
        assert_eq!(node.hostname, "node1.example.com");
        assert_eq!(node.port, 7000);
        assert_eq!(node.status, NodeStatus::Healthy);
        assert_eq!(node.role, NodeRole::Primary);
    }

    #[test]
    fn test_node_health_metrics_creation() {
        let health = NodeHealthMetrics {
            node_id: 1,
            cpu_usage: 45.5,
            memory_usage: 60.2,
            disk_usage: 75.0,
            connection_count: 100,
            query_throughput: 1500.0,
        };

        assert_eq!(health.node_id, 1);
        assert_eq!(health.cpu_usage, 45.5);
        assert_eq!(health.memory_usage, 60.2);
        assert_eq!(health.disk_usage, 75.0);
        assert_eq!(health.connection_count, 100);
        assert_eq!(health.query_throughput, 1500.0);
    }

    #[test]
    fn test_cluster_metrics_creation() {
        let metrics = ClusterMetrics {
            total_queries: 1000000,
            average_latency_ms: 15.5,
            error_rate: 0.001,
            storage_usage_bytes: 1024 * 1024 * 1024,
        };

        assert_eq!(metrics.total_queries, 1000000);
        assert_eq!(metrics.average_latency_ms, 15.5);
        assert_eq!(metrics.error_rate, 0.001);
        assert_eq!(metrics.storage_usage_bytes, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_user_info_creation() {
        let user = UserInfo {
            user_id: 1,
            username: "admin".to_string(),
            roles: vec![Role::Admin],
            permissions: vec![Permission::Read, Permission::Write],
            created_at: Utc::now(),
        };

        assert_eq!(user.user_id, 1);
        assert_eq!(user.username, "admin");
        assert_eq!(user.roles, vec![Role::Admin]);
        assert_eq!(user.permissions.len(), 2);
    }

    #[test]
    fn test_user_update_creation() {
        let update = UserUpdate {
            password: Some("new_password".to_string()),
            roles: Some(vec![Role::User]),
        };

        assert!(update.password.is_some());
        assert!(update.roles.is_some());
        assert_eq!(update.password.unwrap(), "new_password");
    }

    #[test]
    fn test_user_update_partial() {
        let update = UserUpdate {
            password: Some("new_password".to_string()),
            roles: None,
        };

        assert!(update.password.is_some());
        assert!(update.roles.is_none());
    }

    // Test enum variants
    #[test]
    fn test_node_status_variants() {
        let statuses = vec![
            NodeStatus::Healthy,
            NodeStatus::Degraded,
            NodeStatus::Unhealthy,
            NodeStatus::Offline,
        ];

        assert_eq!(statuses.len(), 4);
        assert_eq!(statuses[0], NodeStatus::Healthy);
        assert_ne!(statuses[0], NodeStatus::Degraded);
    }

    #[test]
    fn test_node_role_variants() {
        let roles = vec![NodeRole::Primary, NodeRole::Replica, NodeRole::Coordinator];

        assert_eq!(roles.len(), 3);
        assert_eq!(roles[0], NodeRole::Primary);
        assert_ne!(roles[0], NodeRole::Replica);
    }
}
