//! Simulation module for orchestrating erasure coding demos
//!
//! This module provides tools for creating realistic failure scenarios,
//! running demonstrations, and coordinating the overall simulation flow.

pub mod failure;
pub mod recovery;

use crate::storage::{Cluster, NodeId};
use crate::Result;
use rand::Rng;
use std::time::Duration;

/// A simulation coordinator that manages demo scenarios
pub struct Simulator {
    /// The storage cluster being simulated
    pub cluster: Cluster,
    /// Random number generator for failure simulation
    rng: rand::rngs::ThreadRng,
    /// Simulation speed multiplier
    speed_multiplier: f64,
}

impl Simulator {
    /// Create a new simulator with the given cluster
    pub fn new(cluster: Cluster) -> Self {
        Self {
            cluster,
            rng: rand::thread_rng(),
            speed_multiplier: 1.0,
        }
    }

    /// Set the simulation speed multiplier (higher = faster)
    pub fn set_speed(&mut self, multiplier: f64) {
        self.speed_multiplier = multiplier.max(0.1);
    }

    /// Get the current simulation speed
    pub fn speed(&self) -> f64 {
        self.speed_multiplier
    }

    /// Store test data in the cluster
    pub fn store_test_data(&mut self, key: &str, data: &[u8]) -> Result<()> {
        self.cluster.store_data(key, data)
    }

    /// Retrieve test data from the cluster
    pub fn retrieve_test_data(&self, key: &str) -> Result<Vec<u8>> {
        self.cluster.retrieve_data(key)
    }

    /// Run a failure scenario
    pub async fn run_failure_scenario(&mut self, scenario: FailureScenario) -> Result<()> {
        match scenario {
            FailureScenario::SingleNodeFailure => self.simulate_single_failure().await,
            FailureScenario::CascadingFailures(count) => {
                self.simulate_cascading_failures(count).await
            }
            FailureScenario::RandomFailures(probability) => {
                self.simulate_random_failures(probability).await
            }
            FailureScenario::NetworkPartition(partition_size) => {
                self.simulate_network_partition(partition_size).await
            }
        }
    }

    /// Simulate a single random node failure
    async fn simulate_single_failure(&mut self) -> Result<()> {
        let healthy_nodes: Vec<NodeId> = self
            .cluster
            .node_ids()
            .into_iter()
            .filter(|&id| {
                self.cluster
                    .get_node(id)
                    .map(|n| n.is_available())
                    .unwrap_or(false)
            })
            .collect();

        if healthy_nodes.is_empty() {
            return Err("No healthy nodes available to fail".into());
        }

        let node_to_fail = healthy_nodes[self.rng.gen_range(0..healthy_nodes.len())];

        self.sleep_scaled(Duration::from_millis(500)).await;
        self.cluster.fail_node(node_to_fail)?;

        Ok(())
    }

    /// Simulate cascading failures
    async fn simulate_cascading_failures(&mut self, count: usize) -> Result<()> {
        for i in 0..count {
            if self.cluster.available_node_count() == 0 {
                break;
            }

            // Delay between failures increases with each failure (system stress)
            let delay = Duration::from_millis(200 + (i as u64 * 300));
            self.sleep_scaled(delay).await;

            self.simulate_single_failure().await?;
        }
        Ok(())
    }

    /// Simulate random failures based on probability
    async fn simulate_random_failures(&mut self, probability: f64) -> Result<()> {
        let node_ids = self.cluster.node_ids();

        for &node_id in &node_ids {
            if self.rng.gen::<f64>() < probability {
                if let Some(node) = self.cluster.get_node(node_id) {
                    if node.is_available() {
                        self.sleep_scaled(Duration::from_millis(100)).await;
                        self.cluster.fail_node(node_id)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Simulate network partition by failing a group of nodes
    async fn simulate_network_partition(&mut self, partition_size: usize) -> Result<()> {
        let mut available_nodes: Vec<NodeId> = self
            .cluster
            .node_ids()
            .into_iter()
            .filter(|&id| {
                self.cluster
                    .get_node(id)
                    .map(|n| n.is_available())
                    .unwrap_or(false)
            })
            .collect();

        if available_nodes.len() < partition_size {
            return Err("Not enough available nodes for partition".into());
        }

        // Shuffle and take the first `partition_size` nodes
        use rand::seq::SliceRandom;
        available_nodes.shuffle(&mut self.rng);

        for &node_id in available_nodes.iter().take(partition_size) {
            self.sleep_scaled(Duration::from_millis(50)).await;
            self.cluster.fail_node(node_id)?;
        }

        Ok(())
    }

    /// Recover a random failed node
    pub async fn recover_random_node(&mut self) -> Result<bool> {
        let failed_nodes: Vec<NodeId> = self
            .cluster
            .node_ids()
            .into_iter()
            .filter(|&id| {
                self.cluster
                    .get_node(id)
                    .map(|n| !n.is_available())
                    .unwrap_or(false)
            })
            .collect();

        if failed_nodes.is_empty() {
            return Ok(false);
        }

        let node_to_recover = failed_nodes[self.rng.gen_range(0..failed_nodes.len())];

        self.sleep_scaled(Duration::from_secs(1)).await; // Recovery takes longer
        self.cluster.recover_node(node_to_recover)?;

        Ok(true)
    }

    /// Recover all failed nodes
    pub async fn recover_all_nodes(&mut self) -> Result<usize> {
        let failed_nodes: Vec<NodeId> = self
            .cluster
            .node_ids()
            .into_iter()
            .filter(|&id| {
                self.cluster
                    .get_node(id)
                    .map(|n| !n.is_available())
                    .unwrap_or(false)
            })
            .collect();

        let recovery_count = failed_nodes.len();

        for node_id in failed_nodes {
            self.sleep_scaled(Duration::from_millis(500)).await;
            self.cluster.recover_node(node_id)?;
        }

        Ok(recovery_count)
    }

    /// Sleep for a duration scaled by the simulation speed
    async fn sleep_scaled(&self, duration: Duration) {
        let scaled_duration =
            Duration::from_millis((duration.as_millis() as f64 / self.speed_multiplier) as u64);
        tokio::time::sleep(scaled_duration).await;
    }

    /// Check if the cluster can still serve data
    pub fn can_serve_data(&self) -> bool {
        self.cluster.can_recover_data("")
    }

    /// Get simulation status
    pub fn status(&self) -> SimulationStatus {
        let health = self.cluster.health_status();
        let stats = self.cluster.get_statistics();

        SimulationStatus {
            total_nodes: health.total_nodes,
            healthy_nodes: health.healthy_nodes,
            degraded_nodes: health.degraded_nodes,
            failed_nodes: health.failed_nodes,
            can_recover: health.can_recover,
            failure_tolerance: health.failure_tolerance(),
            is_critical: health.is_critical(),
            total_chunks: stats.total_chunks,
            total_bytes: stats.total_bytes,
        }
    }
}

/// Types of failure scenarios that can be simulated
#[derive(Debug, Clone)]
pub enum FailureScenario {
    /// Fail a single random node
    SingleNodeFailure,
    /// Fail multiple nodes in sequence (cascading failure)
    CascadingFailures(usize),
    /// Fail nodes randomly based on probability
    RandomFailures(f64),
    /// Simulate network partition by failing a group of nodes
    NetworkPartition(usize),
}

impl std::fmt::Display for FailureScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureScenario::SingleNodeFailure => write!(f, "Single Node Failure"),
            FailureScenario::CascadingFailures(n) => write!(f, "Cascading Failures ({})", n),
            FailureScenario::RandomFailures(p) => write!(f, "Random Failures ({:.1}%)", p * 100.0),
            FailureScenario::NetworkPartition(n) => write!(f, "Network Partition ({})", n),
        }
    }
}

/// Current status of the simulation
#[derive(Debug, Clone)]
pub struct SimulationStatus {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub failed_nodes: usize,
    pub can_recover: bool,
    pub failure_tolerance: usize,
    pub is_critical: bool,
    pub total_chunks: usize,
    pub total_bytes: usize,
}

impl SimulationStatus {
    /// Get the overall health percentage
    pub fn health_percentage(&self) -> f64 {
        if self.total_nodes == 0 {
            return 100.0;
        }
        (self.healthy_nodes as f64 / self.total_nodes as f64) * 100.0
    }

    /// Get a health description
    pub fn health_description(&self) -> &'static str {
        match self.health_percentage() {
            p if p >= 90.0 => "Excellent",
            p if p >= 70.0 => "Good",
            p if p >= 50.0 => "Fair",
            p if p >= 30.0 => "Poor",
            _ => "Critical",
        }
    }
}
