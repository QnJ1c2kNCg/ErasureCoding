//! Erasure Coding Demo
//!
//! A demonstration of erasure coding techniques with terminal UI visualization.
//! This library provides in-memory simulation of distributed storage with
//! node failures and recovery mechanisms.

pub mod erasure;
pub mod simulation;
pub mod storage;
pub mod ui;

pub use erasure::ErasureScheme;
pub use simulation::{FailureScenario, Simulator};
pub use storage::{Cluster, Node};
pub use ui::TerminalUI;

/// Result type used throughout the library
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Configuration for the erasure coding demo
#[derive(Debug, Clone)]
pub struct Config {
    /// Number of data chunks
    pub data_chunks: usize,
    /// Number of parity chunks
    pub parity_chunks: usize,
    /// Total number of storage nodes
    pub total_nodes: usize,
    /// Size of each data chunk in bytes
    pub chunk_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_chunks: 4,
            parity_chunks: 2,
            total_nodes: 6,
            chunk_size: 1024,
        }
    }
}

impl Config {
    /// Create a new configuration
    pub fn new(data_chunks: usize, parity_chunks: usize) -> Self {
        Self {
            data_chunks,
            parity_chunks,
            total_nodes: data_chunks + parity_chunks,
            chunk_size: 1024,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.data_chunks == 0 {
            return Err("Data chunks must be greater than 0".into());
        }
        if self.parity_chunks == 0 {
            return Err("Parity chunks must be greater than 0".into());
        }
        if self.total_nodes < self.data_chunks + self.parity_chunks {
            return Err("Total nodes must be at least data_chunks + parity_chunks".into());
        }
        Ok(())
    }

    /// Maximum number of node failures we can tolerate
    pub fn max_failures(&self) -> usize {
        self.parity_chunks
    }
}
