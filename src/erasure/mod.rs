//! Erasure coding implementations
//!
//! This module provides different erasure coding schemes for data protection
//! and recovery in distributed storage systems.

pub mod simple_parity;

use crate::Result;

/// Trait for erasure coding schemes
pub trait ErasureScheme {
    /// Encode data into chunks with redundancy
    ///
    /// Takes input data and splits it into data chunks plus parity chunks
    /// Returns a vector of encoded chunks that can be distributed across nodes
    fn encode(&self, data: &[u8]) -> Result<Vec<Vec<u8>>>;

    /// Decode data from available chunks
    ///
    /// Takes available chunks (some may be None if nodes failed) and reconstructs
    /// the original data if possible
    fn decode(&self, chunks: &[Option<Vec<u8>>]) -> Result<Vec<u8>>;

    /// Check if recovery is possible with given available chunks
    fn can_recover(&self, available_chunks: usize) -> bool;

    /// Get the number of data chunks this scheme produces
    fn data_chunks(&self) -> usize;

    /// Get the number of parity chunks this scheme produces
    fn parity_chunks(&self) -> usize;

    /// Get total number of chunks (data + parity)
    fn total_chunks(&self) -> usize {
        self.data_chunks() + self.parity_chunks()
    }
}

/// Create a simple parity-based erasure scheme
pub fn create_simple_parity(data_chunks: usize, parity_chunks: usize) -> Box<dyn ErasureScheme> {
    Box::new(simple_parity::SimpleParityScheme::new(
        data_chunks,
        parity_chunks,
    ))
}
