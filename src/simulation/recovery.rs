//! Recovery simulation utilities
//!
//! This module provides utilities for simulating data recovery processes,
//! node restoration, and system healing in distributed storage systems.

use crate::storage::{Cluster, NodeId, NodeState};
use crate::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Recovery coordinator that manages the restoration of failed nodes and data
pub struct RecoveryCoordinator {
    /// Planned recovery events
    recovery_schedule: Vec<RecoveryEvent>,
    /// Recovery strategies available
    strategies: Vec<RecoveryStrategy>,
    /// Statistics about recovery operations
    stats: RecoveryStats,
}

impl RecoveryCoordinator {
    /// Create a new recovery coordinator
    pub fn new() -> Self {
        Self {
            recovery_schedule: Vec::new(),
            strategies: vec![
                RecoveryStrategy::ImmediateRestart,
                RecoveryStrategy::GradualRecovery,
                RecoveryStrategy::HotSpare,
            ],
            stats: RecoveryStats::default(),
        }
    }

    /// Schedule a recovery event
    pub fn schedule_recovery(&mut self, event: RecoveryEvent) {
        self.recovery_schedule.push(event);
        self.recovery_schedule.sort_by_key(|e| e.timestamp);
    }

    /// Process pending recovery events
    pub async fn process_recovery_events(
        &mut self,
        cluster: &mut Cluster,
        current_time: Duration,
    ) -> Result<Vec<RecoveryResult>> {
        let mut results = Vec::new();

        // Collect events to process and remove them immediately
        let mut events_to_process = Vec::new();
        let mut remaining_events = Vec::new();

        for event in self.recovery_schedule.drain(..) {
            if event.timestamp <= current_time {
                events_to_process.push(event);
            } else {
                remaining_events.push(event);
            }
        }

        // Restore remaining events
        self.recovery_schedule = remaining_events;

        // Process the events
        for event in events_to_process {
            let result = self.execute_recovery(cluster, &event).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute a specific recovery event
    async fn execute_recovery(
        &mut self,
        cluster: &mut Cluster,
        event: &RecoveryEvent,
    ) -> Result<RecoveryResult> {
        let start_time = Instant::now();

        let success = match &event.recovery_type {
            RecoveryType::NodeRestart(node_id) => self.restart_node(cluster, *node_id).await?,
            RecoveryType::DataRebuild(node_id, keys) => {
                self.rebuild_data(cluster, *node_id, keys).await?
            }
            RecoveryType::HotSpareActivation(spare_id, failed_id) => {
                self.activate_hot_spare(cluster, *spare_id, *failed_id)
                    .await?
            }
            RecoveryType::NetworkRepair(node_ids) => self.repair_network(cluster, node_ids).await?,
        };

        let duration = start_time.elapsed();

        if success {
            self.stats.successful_recoveries += 1;
            self.stats.total_recovery_time += duration;
        } else {
            self.stats.failed_recoveries += 1;
        }

        Ok(RecoveryResult {
            node_id: event.get_primary_node_id(),
            recovery_type: event.recovery_type.clone(),
            success,
            duration,
            strategy_used: event.strategy.clone(),
        })
    }

    /// Restart a failed node
    async fn restart_node(&mut self, cluster: &mut Cluster, node_id: NodeId) -> Result<bool> {
        // Simulate restart delay
        tokio::time::sleep(Duration::from_millis(500)).await;

        if let Some(node) = cluster.get_node(node_id) {
            if node.state() == &NodeState::Failed {
                cluster.recover_node(node_id)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Rebuild data on a recovered node
    async fn rebuild_data(
        &mut self,
        cluster: &mut Cluster,
        node_id: NodeId,
        keys: &[String],
    ) -> Result<bool> {
        // First ensure the node is recovered
        if let Some(node) = cluster.get_node(node_id) {
            if !node.is_available() {
                cluster.recover_node(node_id)?;
            }
        }

        let mut successful_rebuilds = 0;

        // Rebuild each piece of data
        for key in keys {
            // Simulate data reconstruction time
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Try to retrieve and re-store the data
            match cluster.retrieve_data(key) {
                Ok(data) => {
                    if cluster.store_data(key, &data).is_ok() {
                        successful_rebuilds += 1;
                    }
                }
                Err(_) => {
                    // Data might be unrecoverable
                    continue;
                }
            }
        }

        Ok(successful_rebuilds == keys.len())
    }

    /// Activate a hot spare node
    async fn activate_hot_spare(
        &mut self,
        cluster: &mut Cluster,
        spare_id: NodeId,
        failed_id: NodeId,
    ) -> Result<bool> {
        // Simulate hot spare activation time
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Copy data from failed node if still accessible
        if let (Some(spare_node), Some(_failed_node)) =
            (cluster.get_node(spare_id), cluster.get_node(failed_id))
        {
            if spare_node.state() == &NodeState::Healthy {
                // In a real system, we'd copy data here
                // For simulation, we'll just mark the spare as active
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Repair network connectivity issues
    async fn repair_network(&mut self, cluster: &mut Cluster, node_ids: &[NodeId]) -> Result<bool> {
        // Simulate network repair time
        tokio::time::sleep(Duration::from_secs(2)).await;

        let mut repaired_count = 0;
        for &node_id in node_ids {
            if cluster.recover_node(node_id).is_ok() {
                repaired_count += 1;
            }
        }

        Ok(repaired_count == node_ids.len())
    }

    /// Plan optimal recovery strategy for a set of failed nodes
    pub fn plan_recovery_strategy(
        &self,
        cluster: &Cluster,
        failed_nodes: &[NodeId],
    ) -> Vec<RecoveryEvent> {
        let mut events = Vec::new();
        let base_time = Duration::from_secs(1); // Start recovery after 1 second

        // Prioritize recovery based on cluster health
        let health = cluster.health_status();

        if health.is_critical() {
            // Critical state - use fastest recovery
            for (i, &node_id) in failed_nodes.iter().enumerate() {
                events.push(RecoveryEvent {
                    node_id,
                    timestamp: base_time + Duration::from_millis(i as u64 * 100),
                    recovery_type: RecoveryType::NodeRestart(node_id),
                    strategy: RecoveryStrategy::ImmediateRestart,
                });
            }
        } else {
            // Non-critical - use gradual recovery
            for (i, &node_id) in failed_nodes.iter().enumerate() {
                events.push(RecoveryEvent {
                    node_id,
                    timestamp: base_time + Duration::from_secs(i as u64 * 2),
                    recovery_type: RecoveryType::NodeRestart(node_id),
                    strategy: RecoveryStrategy::GradualRecovery,
                });
            }
        }

        events
    }

    /// Get recovery statistics
    pub fn get_stats(&self) -> &RecoveryStats {
        &self.stats
    }

    /// Reset recovery statistics
    pub fn reset_stats(&mut self) {
        self.stats = RecoveryStats::default();
    }

    /// Estimate recovery time for a given scenario
    pub fn estimate_recovery_time(&self, failed_nodes: &[NodeId]) -> Duration {
        let base_recovery_time = Duration::from_secs(30); // Base time per node
        let parallelism_factor = 0.7; // 30% time savings from parallel recovery

        let total_base_time = base_recovery_time * failed_nodes.len() as u32;
        Duration::from_millis((total_base_time.as_millis() as f64 * parallelism_factor) as u64)
    }
}

impl Default for RecoveryCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a scheduled recovery event
#[derive(Debug, Clone)]
pub struct RecoveryEvent {
    /// Primary node involved in recovery
    pub node_id: NodeId,
    /// When the recovery should start
    pub timestamp: Duration,
    /// Type of recovery operation
    pub recovery_type: RecoveryType,
    /// Strategy used for recovery
    pub strategy: RecoveryStrategy,
}

impl RecoveryEvent {
    /// Get the primary node ID involved in this recovery
    pub fn get_primary_node_id(&self) -> NodeId {
        match &self.recovery_type {
            RecoveryType::NodeRestart(id) => *id,
            RecoveryType::DataRebuild(id, _) => *id,
            RecoveryType::HotSpareActivation(spare_id, _) => *spare_id,
            RecoveryType::NetworkRepair(ids) => ids.first().copied().unwrap_or(0),
        }
    }
}

/// Types of recovery operations
#[derive(Debug, Clone)]
pub enum RecoveryType {
    /// Simple node restart
    NodeRestart(NodeId),
    /// Rebuild data on a specific node
    DataRebuild(NodeId, Vec<String>),
    /// Activate hot spare to replace failed node
    HotSpareActivation(NodeId, NodeId), // (spare_id, failed_id)
    /// Repair network connectivity for multiple nodes
    NetworkRepair(Vec<NodeId>),
}

impl std::fmt::Display for RecoveryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryType::NodeRestart(id) => write!(f, "Node Restart ({})", id),
            RecoveryType::DataRebuild(id, keys) => {
                write!(f, "Data Rebuild ({}, {} keys)", id, keys.len())
            }
            RecoveryType::HotSpareActivation(spare, failed) => {
                write!(f, "Hot Spare Activation ({} -> {})", spare, failed)
            }
            RecoveryType::NetworkRepair(ids) => {
                write!(f, "Network Repair ({} nodes)", ids.len())
            }
        }
    }
}

/// Recovery strategies with different trade-offs
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    /// Immediate restart - fastest but may cause instability
    ImmediateRestart,
    /// Gradual recovery - slower but more stable
    GradualRecovery,
    /// Hot spare activation - fastest data recovery
    HotSpare,
    /// Network-aware recovery - considers network topology
    NetworkAware,
}

impl std::fmt::Display for RecoveryStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryStrategy::ImmediateRestart => write!(f, "Immediate Restart"),
            RecoveryStrategy::GradualRecovery => write!(f, "Gradual Recovery"),
            RecoveryStrategy::HotSpare => write!(f, "Hot Spare"),
            RecoveryStrategy::NetworkAware => write!(f, "Network Aware"),
        }
    }
}

/// Result of a recovery operation
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// Node that was recovered
    pub node_id: NodeId,
    /// Type of recovery performed
    pub recovery_type: RecoveryType,
    /// Whether the recovery was successful
    pub success: bool,
    /// How long the recovery took
    pub duration: Duration,
    /// Strategy used for recovery
    pub strategy_used: RecoveryStrategy,
}

/// Statistics about recovery operations
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    /// Number of successful recoveries
    pub successful_recoveries: usize,
    /// Number of failed recoveries
    pub failed_recoveries: usize,
    /// Total time spent on recoveries
    pub total_recovery_time: Duration,
    /// Recovery attempts by strategy
    pub strategy_usage: HashMap<String, usize>,
}

impl RecoveryStats {
    /// Get the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.successful_recoveries + self.failed_recoveries;
        if total == 0 {
            return 100.0;
        }
        (self.successful_recoveries as f64 / total as f64) * 100.0
    }

    /// Get the average recovery time
    pub fn average_recovery_time(&self) -> Duration {
        if self.successful_recoveries == 0 {
            return Duration::from_secs(0);
        }
        Duration::from_millis(
            self.total_recovery_time.as_millis() as u64 / self.successful_recoveries as u64,
        )
    }

    /// Get total recovery attempts
    pub fn total_attempts(&self) -> usize {
        self.successful_recoveries + self.failed_recoveries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erasure;
    use crate::storage::Cluster;

    #[tokio::test]
    async fn test_recovery_coordinator() {
        let mut coordinator = RecoveryCoordinator::new();
        let mut cluster = Cluster::with_nodes(5);
        let scheme = erasure::create_simple_parity(3, 2);
        cluster.set_scheme(scheme);

        // Fail a node
        cluster.fail_node(0).unwrap();

        // Schedule recovery
        let event = RecoveryEvent {
            node_id: 0,
            timestamp: Duration::from_millis(100),
            recovery_type: RecoveryType::NodeRestart(0),
            strategy: RecoveryStrategy::ImmediateRestart,
        };
        coordinator.schedule_recovery(event);

        // Process recovery
        let results = coordinator
            .process_recovery_events(&mut cluster, Duration::from_millis(200))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(coordinator.get_stats().successful_recoveries, 1);
    }

    #[test]
    fn test_recovery_planning() {
        let cluster = Cluster::with_nodes(6);
        let coordinator = RecoveryCoordinator::new();
        let failed_nodes = vec![0, 1, 2];

        let plan = coordinator.plan_recovery_strategy(&cluster, &failed_nodes);

        assert_eq!(plan.len(), 3);
        assert!(plan.windows(2).all(|w| w[0].timestamp <= w[1].timestamp));
    }

    #[test]
    fn test_recovery_stats() {
        let mut stats = RecoveryStats::default();

        assert_eq!(stats.success_rate(), 100.0); // No attempts yet
        assert_eq!(stats.total_attempts(), 0);

        stats.successful_recoveries = 8;
        stats.failed_recoveries = 2;
        stats.total_recovery_time = Duration::from_secs(80);

        assert_eq!(stats.success_rate(), 80.0);
        assert_eq!(stats.total_attempts(), 10);
        assert_eq!(stats.average_recovery_time(), Duration::from_secs(10));
    }

    #[test]
    fn test_recovery_time_estimation() {
        let coordinator = RecoveryCoordinator::new();
        let failed_nodes = vec![0, 1, 2];

        let estimate = coordinator.estimate_recovery_time(&failed_nodes);
        assert!(estimate > Duration::from_secs(30)); // Should be more than 30s for 3 nodes
        assert!(estimate < Duration::from_secs(90)); // But less than 90s due to parallelism
    }
}
