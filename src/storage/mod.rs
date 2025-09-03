//! Storage simulation module
//!
//! This module simulates distributed storage nodes and clusters for
//! demonstrating erasure coding in action.

pub mod cluster;
pub mod node;

pub use cluster::Cluster;
pub use node::{Node, NodeId, NodeState};

use crate::Result;

/// Trait for storage backends
pub trait Storage {
    /// Store a chunk of data
    fn store(&mut self, key: &str, data: Vec<u8>) -> Result<()>;

    /// Retrieve a chunk of data
    fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Delete a chunk of data
    fn delete(&mut self, key: &str) -> Result<()>;

    /// List all stored keys
    fn list_keys(&self) -> Vec<String>;

    /// Get storage statistics
    fn stats(&self) -> StorageStats;
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total number of stored chunks
    pub total_chunks: usize,
    /// Total bytes stored
    pub total_bytes: usize,
    /// Number of read operations
    pub reads: usize,
    /// Number of write operations
    pub writes: usize,
}

impl StorageStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a read operation
    pub fn record_read(&mut self) {
        self.reads += 1;
    }

    /// Record a write operation
    pub fn record_write(&mut self, bytes: usize) {
        self.writes += 1;
        self.total_bytes += bytes;
        self.total_chunks += 1;
    }

    /// Record a delete operation
    pub fn record_delete(&mut self, bytes: usize) {
        self.total_bytes = self.total_bytes.saturating_sub(bytes);
        self.total_chunks = self.total_chunks.saturating_sub(1);
    }
}
