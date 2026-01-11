//! Metrics collection module for Q-Distributed-Database Client SDK
//!
//! This module implements comprehensive metrics collection for monitoring
//! operation latency, success/error rates, and connection pool statistics.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Percentile latency statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Percentiles {
    /// 50th percentile (median) latency in milliseconds
    pub p50: f64,
    /// 95th percentile latency in milliseconds
    pub p95: f64,
    /// 99th percentile latency in milliseconds
    pub p99: f64,
}

impl Default for Percentiles {
    fn default() -> Self {
        Self {
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
        }
    }
}

/// Operation metrics for tracking counts, success/error rates, and latencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    /// Total number of operations
    pub total_count: u64,
    /// Number of successful operations
    pub success_count: u64,
    /// Number of failed operations
    pub error_count: u64,
    /// Minimum latency in milliseconds
    pub min_latency_ms: f64,
    /// Maximum latency in milliseconds
    pub max_latency_ms: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Latency percentiles
    pub percentiles: Percentiles,
}

impl Default for OperationMetrics {
    fn default() -> Self {
        Self {
            total_count: 0,
            success_count: 0,
            error_count: 0,
            min_latency_ms: f64::MAX,
            max_latency_ms: 0.0,
            avg_latency_ms: 0.0,
            percentiles: Percentiles::default(),
        }
    }
}

/// Connection pool metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    /// Number of active connections
    pub active_connections: u32,
    /// Number of idle connections
    pub idle_connections: u32,
    /// Total number of connections
    pub total_connections: u32,
    /// Number of connection errors
    pub connection_errors: u64,
    /// Number of connection timeouts
    pub connection_timeouts: u64,
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self {
            active_connections: 0,
            idle_connections: 0,
            total_connections: 0,
            connection_errors: 0,
            connection_timeouts: 0,
        }
    }
}

/// Public metrics API exposed to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMetrics {
    /// Query operation metrics
    pub query_metrics: OperationMetrics,
    /// Execute operation metrics
    pub execute_metrics: OperationMetrics,
    /// Transaction operation metrics
    pub transaction_metrics: OperationMetrics,
    /// Authentication metrics
    pub auth_metrics: OperationMetrics,
    /// Connection pool metrics
    pub connection_metrics: ConnectionMetrics,
}

impl Default for ClientMetrics {
    fn default() -> Self {
        Self {
            query_metrics: OperationMetrics::default(),
            execute_metrics: OperationMetrics::default(),
            transaction_metrics: OperationMetrics::default(),
            auth_metrics: OperationMetrics::default(),
            connection_metrics: ConnectionMetrics::default(),
        }
    }
}

/// Internal operation tracker for calculating metrics
#[derive(Debug)]
struct OperationTracker {
    total_count: AtomicU64,
    success_count: AtomicU64,
    error_count: AtomicU64,
    latencies: Arc<RwLock<Vec<f64>>>,
}

impl OperationTracker {
    fn new() -> Self {
        Self {
            total_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            latencies: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn record(&self, success: bool, latency_ms: f64) {
        self.total_count.fetch_add(1, Ordering::SeqCst);
        if success {
            self.success_count.fetch_add(1, Ordering::SeqCst);
        } else {
            self.error_count.fetch_add(1, Ordering::SeqCst);
        }

        let mut latencies = self.latencies.write().await;
        latencies.push(latency_ms);
        
        // Keep only last 1000 latencies to prevent unbounded growth
        if latencies.len() > 1000 {
            let excess = latencies.len() - 1000;
            latencies.drain(0..excess);
        }
    }

    async fn get_metrics(&self) -> OperationMetrics {
        let total_count = self.total_count.load(Ordering::SeqCst);
        let success_count = self.success_count.load(Ordering::SeqCst);
        let error_count = self.error_count.load(Ordering::SeqCst);

        let latencies = self.latencies.read().await;
        
        if latencies.is_empty() {
            return OperationMetrics {
                total_count,
                success_count,
                error_count,
                ..Default::default()
            };
        }

        let min_latency_ms = latencies.iter().copied().fold(f64::MAX, f64::min);
        let max_latency_ms = latencies.iter().copied().fold(0.0, f64::max);
        let sum: f64 = latencies.iter().sum();
        let avg_latency_ms = sum / latencies.len() as f64;

        // Calculate percentiles
        let mut sorted_latencies = latencies.clone();
        sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let p50_idx = (sorted_latencies.len() as f64 * 0.50) as usize;
        let p95_idx = (sorted_latencies.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted_latencies.len() as f64 * 0.99) as usize;

        let percentiles = Percentiles {
            p50: sorted_latencies.get(p50_idx).copied().unwrap_or(0.0),
            p95: sorted_latencies.get(p95_idx).copied().unwrap_or(0.0),
            p99: sorted_latencies.get(p99_idx).copied().unwrap_or(0.0),
        };

        OperationMetrics {
            total_count,
            success_count,
            error_count,
            min_latency_ms,
            max_latency_ms,
            avg_latency_ms,
            percentiles,
        }
    }
}

/// Metrics collector for tracking all SDK operations
pub struct MetricsCollector {
    query_tracker: OperationTracker,
    execute_tracker: OperationTracker,
    transaction_tracker: OperationTracker,
    auth_tracker: OperationTracker,
    connection_metrics: Arc<RwLock<ConnectionMetrics>>,
}

impl MetricsCollector {
    /// Creates a new metrics collector
    pub fn new() -> Self {
        Self {
            query_tracker: OperationTracker::new(),
            execute_tracker: OperationTracker::new(),
            transaction_tracker: OperationTracker::new(),
            auth_tracker: OperationTracker::new(),
            connection_metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
        }
    }

    /// Records a query operation
    pub async fn record_query(&self, success: bool, latency_ms: f64) {
        self.query_tracker.record(success, latency_ms).await;
    }

    /// Records an execute operation
    pub async fn record_execute(&self, success: bool, latency_ms: f64) {
        self.execute_tracker.record(success, latency_ms).await;
    }

    /// Records a transaction operation
    pub async fn record_transaction(&self, success: bool, latency_ms: f64) {
        self.transaction_tracker.record(success, latency_ms).await;
    }

    /// Records an authentication attempt
    pub async fn record_auth_attempt(&self, success: bool, latency_ms: f64) {
        self.auth_tracker.record(success, latency_ms).await;
    }

    /// Updates connection pool metrics
    pub async fn update_connection_metrics(
        &self,
        active: u32,
        idle: u32,
        total: u32,
    ) {
        let mut metrics = self.connection_metrics.write().await;
        metrics.active_connections = active;
        metrics.idle_connections = idle;
        metrics.total_connections = total;
    }

    /// Records a connection error
    pub async fn record_connection_error(&self) {
        let mut metrics = self.connection_metrics.write().await;
        metrics.connection_errors += 1;
    }

    /// Records a connection timeout
    pub async fn record_connection_timeout(&self) {
        let mut metrics = self.connection_metrics.write().await;
        metrics.connection_timeouts += 1;
    }

    /// Gets the current metrics snapshot
    pub async fn get_metrics(&self) -> ClientMetrics {
        ClientMetrics {
            query_metrics: self.query_tracker.get_metrics().await,
            execute_metrics: self.execute_tracker.get_metrics().await,
            transaction_metrics: self.transaction_tracker.get_metrics().await,
            auth_metrics: self.auth_tracker.get_metrics().await,
            connection_metrics: self.connection_metrics.read().await.clone(),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let metrics = collector.get_metrics().await;
        
        assert_eq!(metrics.query_metrics.total_count, 0);
        assert_eq!(metrics.execute_metrics.total_count, 0);
        assert_eq!(metrics.transaction_metrics.total_count, 0);
        assert_eq!(metrics.auth_metrics.total_count, 0);
    }

    #[tokio::test]
    async fn test_record_query() {
        let collector = MetricsCollector::new();
        
        collector.record_query(true, 10.5).await;
        collector.record_query(true, 15.2).await;
        collector.record_query(false, 20.0).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.query_metrics.total_count, 3);
        assert_eq!(metrics.query_metrics.success_count, 2);
        assert_eq!(metrics.query_metrics.error_count, 1);
        assert!(metrics.query_metrics.avg_latency_ms > 0.0);
    }

    #[tokio::test]
    async fn test_record_execute() {
        let collector = MetricsCollector::new();
        
        collector.record_execute(true, 5.0).await;
        collector.record_execute(true, 10.0).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.execute_metrics.total_count, 2);
        assert_eq!(metrics.execute_metrics.success_count, 2);
        assert_eq!(metrics.execute_metrics.error_count, 0);
    }

    #[tokio::test]
    async fn test_record_transaction() {
        let collector = MetricsCollector::new();
        
        collector.record_transaction(true, 25.0).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.transaction_metrics.total_count, 1);
        assert_eq!(metrics.transaction_metrics.success_count, 1);
    }

    #[tokio::test]
    async fn test_record_auth_attempt() {
        let collector = MetricsCollector::new();
        
        collector.record_auth_attempt(true, 100.0).await;
        collector.record_auth_attempt(false, 50.0).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.auth_metrics.total_count, 2);
        assert_eq!(metrics.auth_metrics.success_count, 1);
        assert_eq!(metrics.auth_metrics.error_count, 1);
    }

    #[tokio::test]
    async fn test_update_connection_metrics() {
        let collector = MetricsCollector::new();
        
        collector.update_connection_metrics(5, 10, 15).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.connection_metrics.active_connections, 5);
        assert_eq!(metrics.connection_metrics.idle_connections, 10);
        assert_eq!(metrics.connection_metrics.total_connections, 15);
    }

    #[tokio::test]
    async fn test_record_connection_error() {
        let collector = MetricsCollector::new();
        
        collector.record_connection_error().await;
        collector.record_connection_error().await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.connection_metrics.connection_errors, 2);
    }

    #[tokio::test]
    async fn test_record_connection_timeout() {
        let collector = MetricsCollector::new();
        
        collector.record_connection_timeout().await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.connection_metrics.connection_timeouts, 1);
    }

    #[tokio::test]
    async fn test_latency_percentiles() {
        let collector = MetricsCollector::new();
        
        // Record latencies: 1, 2, 3, ..., 100
        for i in 1..=100 {
            collector.record_query(true, i as f64).await;
        }
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.query_metrics.total_count, 100);
        assert_eq!(metrics.query_metrics.min_latency_ms, 1.0);
        assert_eq!(metrics.query_metrics.max_latency_ms, 100.0);
        
        // Check percentiles are reasonable
        assert!(metrics.query_metrics.percentiles.p50 >= 40.0 && metrics.query_metrics.percentiles.p50 <= 60.0);
        assert!(metrics.query_metrics.percentiles.p95 >= 90.0 && metrics.query_metrics.percentiles.p95 <= 100.0);
        assert!(metrics.query_metrics.percentiles.p99 >= 95.0 && metrics.query_metrics.percentiles.p99 <= 100.0);
    }

    #[tokio::test]
    async fn test_latency_buffer_limit() {
        let collector = MetricsCollector::new();
        
        // Record more than 1000 latencies
        for i in 1..=1500 {
            collector.record_query(true, i as f64).await;
        }
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.query_metrics.total_count, 1500);
        
        // Latency buffer should be capped at 1000
        let latencies = collector.query_tracker.latencies.read().await;
        assert_eq!(latencies.len(), 1000);
    }
}
