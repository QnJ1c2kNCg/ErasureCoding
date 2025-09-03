//! Storage node implementation
//!
//! This module implements individual storage nodes that can be in different
//! states (healthy, degraded, failed) and simulate real-world storage behavior.

use crate::storage::{Storage, StorageStats};
use crate::Result;
use std::collections::HashMap;

/// Unique identifier for a storage node
pub type NodeId = usize;

/// State of a storage node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// Node is healthy and operating normally
    Healthy,
    /// Node is degraded but still functional (slower responses, etc.)
    Degraded,
    /// Node has failed and is not accessible
    Failed,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Healthy => write!(f, "Healthy"),
            NodeState::Degraded => write!(f, "Degraded"),
            NodeState::Failed => write!(f, "Failed"),
        }
    }
}

/// A storage node that can hold data chunks
#[derive(Debug, Clone)]
pub struct Node {
    /// Unique identifier for this node
    pub id: NodeId,
    /// Current state of the node
    pub state: NodeState,
    /// In-memory storage for data chunks
    data: HashMap<String, Vec<u8>>,
    /// Storage statistics
    stats: StorageStats,
    /// Simulated latency in milliseconds
    pub latency_ms: u64,
}

impl Node {
    /// Create a new healthy storage node
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            state: NodeState::Healthy,
            data: HashMap::new(),
            stats: StorageStats::new(),
            latency_ms: 10, // Default 10ms latency
        }
    }

    /// Create a new node with specified state
    pub fn with_state(id: NodeId, state: NodeState) -> Self {
        let mut node = Self::new(id);
        node.state = state;
        node.latency_ms = match state {
            NodeState::Healthy => 10,
            NodeState::Degraded => 100, // Slower when degraded
            NodeState::Failed => 0,     // No response when failed
        };
        node
    }

    /// Get the node's current state
    pub fn state(&self) -> &NodeState {
        &self.state
    }

    /// Set the node's state
    pub fn set_state(&mut self, state: NodeState) {
        self.state = state;
        self.latency_ms = match state {
            NodeState::Healthy => 10,
            NodeState::Degraded => 100,
            NodeState::Failed => 0,
        };
    }

    /// Check if the node is available (not failed)
    pub fn is_available(&self) -> bool {
        self.state != NodeState::Failed
    }

    /// Get the number of chunks stored on this node
    pub fn chunk_count(&self) -> usize {
        self.data.len()
    }

    /// Get total bytes stored on this node
    pub fn bytes_stored(&self) -> usize {
        self.data.values().map(|v| v.len()).sum()
    }

    /// Simulate node failure
    pub fn fail(&mut self) {
        self.set_state(NodeState::Failed);
    }

    /// Simulate node recovery
    pub fn recover(&mut self) {
        self.set_state(NodeState::Healthy);
    }

    /// Simulate node degradation
    pub fn degrade(&mut self) {
        if self.state == NodeState::Healthy {
            self.set_state(NodeState::Degraded);
        }
    }

    /// Get a copy of all stored keys (for debugging/visualization)
    pub fn get_stored_keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// Clear all stored data (simulate data loss)
    pub fn clear_data(&mut self) {
        self.data.clear();
        self.stats = StorageStats::new();
    }
}

impl Storage for Node {
    fn store(&mut self, key: &str, data: Vec<u8>) -> Result<()> {
        if self.state == NodeState::Failed {
            return Err("Node is failed and cannot store data".into());
        }

        let data_size = data.len();
        self.data.insert(key.to_string(), data);
        self.stats.record_write(data_size);

        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>> {
        if self.state == NodeState::Failed {
            return Err("Node is failed and cannot retrieve data".into());
        }

        let result = self.data.get(key).cloned();
        // Note: We don't record reads here to avoid mutable borrow issues
        // In a real system, stats would be handled differently

        Ok(result)
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        if self.state == NodeState::Failed {
            return Err("Node is failed and cannot delete data".into());
        }

        if let Some(data) = self.data.remove(key) {
            self.stats.record_delete(data.len());
        }

        Ok(())
    }

    fn list_keys(&self) -> Vec<String> {
        if self.state == NodeState::Failed {
            return vec![];
        }

        self.data.keys().cloned().collect()
    }

    fn stats(&self) -> StorageStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new(1);
        assert_eq!(node.id, 1);
        assert_eq!(node.state, NodeState::Healthy);
        assert!(node.is_available());
        assert_eq!(node.chunk_count(), 0);
    }

    #[test]
    fn test_node_state_changes() {
        let mut node = Node::new(1);

        node.degrade();
        assert_eq!(node.state, NodeState::Degraded);
        assert!(node.is_available());

        node.fail();
        assert_eq!(node.state, NodeState::Failed);
        assert!(!node.is_available());

        node.recover();
        assert_eq!(node.state, NodeState::Healthy);
        assert!(node.is_available());
    }

    #[test]
    fn test_storage_operations() {
        let mut node = Node::new(1);
        let data = vec![1, 2, 3, 4, 5];

        // Store data
        assert!(node.store("test_key", data.clone()).is_ok());
        assert_eq!(node.chunk_count(), 1);

        // Retrieve data
        let retrieved = node.retrieve("test_key").unwrap();
        assert_eq!(retrieved, Some(data));

        // Delete data
        assert!(node.delete("test_key").is_ok());
        assert_eq!(node.chunk_count(), 0);
    }

    #[test]
    fn test_failed_node_operations() {
        let mut node = Node::new(1);
        node.fail();

        let data = vec![1, 2, 3];

        // Failed node should reject operations
        assert!(node.store("test", data).is_err());
        assert!(node.retrieve("test").is_err());
        assert!(node.delete("test").is_err());
        assert!(node.list_keys().is_empty());
    }
}
