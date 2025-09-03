//! Cluster management for coordinating multiple storage nodes
//!
//! This module implements a cluster of storage nodes that can coordinate
//! erasure coding operations, handle node failures, and manage data distribution.

use crate::erasure::ErasureScheme;
use crate::storage::{Node, NodeId, NodeState, Storage};
use crate::Result;
use std::collections::HashMap;

/// A cluster of storage nodes
pub struct Cluster {
    /// Map of node ID to node
    nodes: HashMap<NodeId, Node>,
    /// Next available node ID
    next_id: NodeId,
    /// Erasure coding scheme used by this cluster
    scheme: Option<Box<dyn ErasureScheme>>,
}

impl Cluster {
    /// Create a new empty cluster
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            scheme: None,
        }
    }

    /// Create a cluster with a specified number of nodes
    pub fn with_nodes(node_count: usize) -> Self {
        let mut cluster = Self::new();
        for _ in 0..node_count {
            cluster.add_node();
        }
        cluster
    }

    /// Set the erasure coding scheme for this cluster
    pub fn set_scheme(&mut self, scheme: Box<dyn ErasureScheme>) {
        self.scheme = Some(scheme);
    }

    /// Add a new healthy node to the cluster
    pub fn add_node(&mut self) -> NodeId {
        let id = self.next_id;
        self.nodes.insert(id, Node::new(id));
        self.next_id += 1;
        id
    }

    /// Add a node with a specific state
    pub fn add_node_with_state(&mut self, state: NodeState) -> NodeId {
        let id = self.next_id;
        self.nodes.insert(id, Node::with_state(id, state));
        self.next_id += 1;
        id
    }

    /// Remove a node from the cluster
    pub fn remove_node(&mut self, id: NodeId) -> Result<()> {
        if self.nodes.remove(&id).is_some() {
            Ok(())
        } else {
            Err(format!("Node {} not found in cluster", id).into())
        }
    }

    /// Get a reference to a specific node
    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Get a mutable reference to a specific node
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    /// Get all node IDs in the cluster
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }

    /// Get all nodes in the cluster
    pub fn nodes(&self) -> Vec<&Node> {
        self.nodes.values().collect()
    }

    /// Get the number of nodes in the cluster
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of healthy nodes
    pub fn healthy_node_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|n| n.state() == &NodeState::Healthy)
            .count()
    }

    /// Get the number of failed nodes
    pub fn failed_node_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|n| n.state() == &NodeState::Failed)
            .count()
    }

    /// Get the number of available nodes (healthy + degraded)
    pub fn available_node_count(&self) -> usize {
        self.nodes.values().filter(|n| n.is_available()).count()
    }

    /// Fail a specific node
    pub fn fail_node(&mut self, id: NodeId) -> Result<()> {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.fail();
            Ok(())
        } else {
            Err(format!("Node {} not found", id).into())
        }
    }

    /// Recover a specific node
    pub fn recover_node(&mut self, id: NodeId) -> Result<()> {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.recover();
            Ok(())
        } else {
            Err(format!("Node {} not found", id).into())
        }
    }

    /// Degrade a specific node
    pub fn degrade_node(&mut self, id: NodeId) -> Result<()> {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.degrade();
            Ok(())
        } else {
            Err(format!("Node {} not found", id).into())
        }
    }

    /// Store data across the cluster using erasure coding
    pub fn store_data(&mut self, key: &str, data: &[u8]) -> Result<()> {
        let scheme = self
            .scheme
            .as_ref()
            .ok_or("No erasure coding scheme configured")?;

        // Encode the data into chunks
        let chunks = scheme.encode(data)?;

        if chunks.len() > self.node_count() {
            return Err(format!(
                "Not enough nodes: need {}, have {}",
                chunks.len(),
                self.node_count()
            )
            .into());
        }

        // Distribute chunks across nodes
        let node_ids: Vec<NodeId> = self.node_ids();
        for (i, chunk) in chunks.into_iter().enumerate() {
            if i < node_ids.len() {
                let node_id = node_ids[i];
                let chunk_key = format!("{}_{}", key, i);

                if let Some(node) = self.nodes.get_mut(&node_id) {
                    if node.is_available() {
                        node.store(&chunk_key, chunk)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Retrieve and reconstruct data from the cluster
    pub fn retrieve_data(&self, key: &str) -> Result<Vec<u8>> {
        let scheme = self
            .scheme
            .as_ref()
            .ok_or("No erasure coding scheme configured")?;

        let total_chunks = scheme.total_chunks();
        let mut chunks = vec![None; total_chunks];

        // Collect available chunks from nodes
        let node_ids: Vec<NodeId> = self.node_ids();
        for (i, &node_id) in node_ids.iter().enumerate().take(total_chunks) {
            let chunk_key = format!("{}_{}", key, i);

            if let Some(node) = self.nodes.get(&node_id) {
                if let Ok(Some(chunk_data)) = node.retrieve(&chunk_key) {
                    chunks[i] = Some(chunk_data);
                }
            }
        }

        // Decode the data from available chunks
        scheme.decode(&chunks)
    }

    /// Check if data can be recovered given current node states
    pub fn can_recover_data(&self, _key: &str) -> bool {
        if let Some(ref scheme) = self.scheme {
            scheme.can_recover(self.available_node_count())
        } else {
            false
        }
    }

    /// Get cluster health status
    pub fn health_status(&self) -> ClusterHealth {
        let total = self.node_count();
        let healthy = self.healthy_node_count();
        let failed = self.failed_node_count();
        let degraded = total - healthy - failed;

        ClusterHealth {
            total_nodes: total,
            healthy_nodes: healthy,
            degraded_nodes: degraded,
            failed_nodes: failed,
            can_recover: self.can_recover_data(""), // Generic check
        }
    }

    /// Get detailed statistics for all nodes
    pub fn get_statistics(&self) -> ClusterStatistics {
        let mut total_chunks = 0;
        let mut total_bytes = 0;
        let mut node_stats = Vec::new();

        for node in self.nodes.values() {
            let stats = node.stats();
            total_chunks += stats.total_chunks;
            total_bytes += stats.total_bytes;

            node_stats.push(NodeStatistics {
                node_id: node.id,
                state: node.state().clone(),
                chunks: node.chunk_count(),
                bytes: node.bytes_stored(),
                latency_ms: node.latency_ms,
            });
        }

        ClusterStatistics {
            total_chunks,
            total_bytes,
            node_stats,
        }
    }
}

impl Default for Cluster {
    fn default() -> Self {
        Self::new()
    }
}

/// Health status of the cluster
#[derive(Debug, Clone)]
pub struct ClusterHealth {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub failed_nodes: usize,
    pub can_recover: bool,
}

impl ClusterHealth {
    /// Get the failure tolerance (how many more nodes can fail)
    pub fn failure_tolerance(&self) -> usize {
        if self.can_recover {
            self.healthy_nodes.saturating_sub(1)
        } else {
            0
        }
    }

    /// Check if the cluster is in a critical state
    pub fn is_critical(&self) -> bool {
        !self.can_recover || self.failure_tolerance() == 0
    }
}

/// Statistics for a single node
#[derive(Debug, Clone)]
pub struct NodeStatistics {
    pub node_id: NodeId,
    pub state: NodeState,
    pub chunks: usize,
    pub bytes: usize,
    pub latency_ms: u64,
}

/// Overall cluster statistics
#[derive(Debug, Clone)]
pub struct ClusterStatistics {
    pub total_chunks: usize,
    pub total_bytes: usize,
    pub node_stats: Vec<NodeStatistics>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erasure;

    #[test]
    fn test_cluster_creation() {
        let cluster = Cluster::with_nodes(5);
        assert_eq!(cluster.node_count(), 5);
        assert_eq!(cluster.healthy_node_count(), 5);
        assert_eq!(cluster.failed_node_count(), 0);
    }

    #[test]
    fn test_node_management() {
        let mut cluster = Cluster::new();

        let id1 = cluster.add_node();
        let id2 = cluster.add_node();

        assert_eq!(cluster.node_count(), 2);

        cluster.fail_node(id1).unwrap();
        assert_eq!(cluster.healthy_node_count(), 1);
        assert_eq!(cluster.failed_node_count(), 1);

        cluster.recover_node(id1).unwrap();
        assert_eq!(cluster.healthy_node_count(), 2);
        assert_eq!(cluster.failed_node_count(), 0);

        cluster.remove_node(id2).unwrap();
        assert_eq!(cluster.node_count(), 1);
    }

    #[test]
    fn test_data_storage_and_retrieval() {
        let mut cluster = Cluster::with_nodes(6);
        let scheme = erasure::create_simple_parity(4, 2);
        cluster.set_scheme(scheme);

        let test_data = b"Hello, World! This is test data.";

        // Store data
        cluster.store_data("test", test_data).unwrap();

        // Retrieve data
        let retrieved = cluster.retrieve_data("test").unwrap();
        assert_eq!(retrieved, test_data);
    }

    #[test]
    fn test_recovery_after_failure() {
        let mut cluster = Cluster::with_nodes(6);
        let scheme = erasure::create_simple_parity(4, 2);
        cluster.set_scheme(scheme);

        let test_data = b"Test data for recovery";

        // Store data
        cluster.store_data("test", test_data).unwrap();

        // Fail one node
        let node_ids = cluster.node_ids();
        cluster.fail_node(node_ids[0]).unwrap();

        // Should still be able to retrieve
        let retrieved = cluster.retrieve_data("test").unwrap();
        assert_eq!(retrieved, test_data);
    }

    #[test]
    fn test_cluster_health() {
        let mut cluster = Cluster::with_nodes(5);

        let health = cluster.health_status();
        assert_eq!(health.total_nodes, 5);
        assert_eq!(health.healthy_nodes, 5);
        assert_eq!(health.failed_nodes, 0);

        // Fail a node
        let node_ids = cluster.node_ids();
        cluster.fail_node(node_ids[0]).unwrap();

        let health = cluster.health_status();
        assert_eq!(health.healthy_nodes, 4);
        assert_eq!(health.failed_nodes, 1);
    }
}
