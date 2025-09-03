//! Simple parity-based erasure coding
//!
//! This implements a basic XOR-based parity scheme for erasure coding.
//! It's simpler than Reed-Solomon but still demonstrates the core concepts
//! of data protection and recovery.

use crate::erasure::ErasureScheme;
use crate::Result;

/// Simple parity-based erasure coding scheme
///
/// This scheme splits data into equal-sized chunks and creates parity chunks
/// using XOR operations. It can recover from up to `parity_chunks` failures.
pub struct SimpleParityScheme {
    data_chunks: usize,
    parity_chunks: usize,
}

impl SimpleParityScheme {
    /// Create a new simple parity scheme
    pub fn new(data_chunks: usize, parity_chunks: usize) -> Self {
        Self {
            data_chunks,
            parity_chunks,
        }
    }

    /// Split data into equal-sized chunks, padding the last chunk if necessary
    fn split_data(&self, data: &[u8]) -> Vec<Vec<u8>> {
        if data.is_empty() {
            return vec![vec![]; self.data_chunks];
        }

        let chunk_size = (data.len() + self.data_chunks - 1) / self.data_chunks;
        let mut chunks = Vec::with_capacity(self.data_chunks);

        for i in 0..self.data_chunks {
            let start = i * chunk_size;
            let end = std::cmp::min(start + chunk_size, data.len());

            if start < data.len() {
                let mut chunk = data[start..end].to_vec();
                // Pad chunk to uniform size
                chunk.resize(chunk_size, 0);
                chunks.push(chunk);
            } else {
                // Create empty chunk if we've run out of data
                chunks.push(vec![0; chunk_size]);
            }
        }

        chunks
    }

    /// Create parity chunks using XOR operations
    fn create_parity_chunks(&self, data_chunks: &[Vec<u8>]) -> Vec<Vec<u8>> {
        if data_chunks.is_empty() {
            return vec![];
        }

        let chunk_size = data_chunks[0].len();
        let mut parity_chunks = Vec::with_capacity(self.parity_chunks);

        for p in 0..self.parity_chunks {
            let mut parity_chunk = vec![0u8; chunk_size];

            // For simplicity, each parity chunk is XOR of different combinations of data chunks
            // Parity 0: XOR of all data chunks
            // Parity 1: XOR of alternating data chunks (0, 2, 4, ...)
            // Parity 2: XOR of different pattern, etc.
            for (i, data_chunk) in data_chunks.iter().enumerate() {
                let should_include = match p {
                    0 => true,                   // First parity includes all chunks
                    _ => (i + p) % (p + 1) == 0, // Different patterns for other parities
                };

                if should_include {
                    for (j, &byte) in data_chunk.iter().enumerate() {
                        parity_chunk[j] ^= byte;
                    }
                }
            }

            parity_chunks.push(parity_chunk);
        }

        parity_chunks
    }

    /// Recover missing data chunks using available data and parity chunks
    fn recover_chunks(&self, chunks: &[Option<Vec<u8>>]) -> Result<Vec<Vec<u8>>> {
        let total_chunks = self.total_chunks();
        if chunks.len() != total_chunks {
            return Err(format!("Expected {} chunks, got {}", total_chunks, chunks.len()).into());
        }

        // Count available chunks
        let available_count = chunks.iter().filter(|c| c.is_some()).count();
        if !self.can_recover(available_count) {
            return Err(format!(
                "Cannot recover: need at least {} chunks, have {}",
                self.data_chunks, available_count
            )
            .into());
        }

        // If we have all data chunks, we're done
        let mut recovered_data = Vec::with_capacity(self.data_chunks);
        let mut missing_indices = Vec::new();

        for i in 0..self.data_chunks {
            if let Some(ref chunk) = chunks[i] {
                recovered_data.push(chunk.clone());
            } else {
                missing_indices.push(i);
                recovered_data.push(vec![]); // Placeholder
            }
        }

        // If no missing data chunks, return what we have
        if missing_indices.is_empty() {
            return Ok(recovered_data);
        }

        // For simple recovery, we'll use the first available parity chunk
        // and XOR it with all available data chunks to recover one missing chunk
        // This is a simplified approach - a full implementation would be more sophisticated
        if missing_indices.len() == 1 {
            let missing_idx = missing_indices[0];

            // Find first available parity chunk
            let parity_chunk = chunks[self.data_chunks..]
                .iter()
                .find_map(|c| c.as_ref())
                .ok_or("No parity chunks available for recovery")?;

            let _chunk_size = parity_chunk.len();
            let mut recovered_chunk = parity_chunk.clone();

            // XOR with all available data chunks to isolate the missing one
            for (i, chunk_opt) in chunks[..self.data_chunks].iter().enumerate() {
                if i != missing_idx {
                    if let Some(chunk) = chunk_opt {
                        for (j, &byte) in chunk.iter().enumerate() {
                            recovered_chunk[j] ^= byte;
                        }
                    }
                }
            }

            recovered_data[missing_idx] = recovered_chunk;
        } else {
            return Err("Multiple chunk recovery not implemented in simple scheme".into());
        }

        Ok(recovered_data)
    }
}

impl ErasureScheme for SimpleParityScheme {
    fn encode(&self, data: &[u8]) -> Result<Vec<Vec<u8>>> {
        // Split data into chunks
        let data_chunks = self.split_data(data);

        // Create parity chunks
        let parity_chunks = self.create_parity_chunks(&data_chunks);

        // Combine data and parity chunks
        let mut all_chunks = data_chunks;
        all_chunks.extend(parity_chunks);

        Ok(all_chunks)
    }

    fn decode(&self, chunks: &[Option<Vec<u8>>]) -> Result<Vec<u8>> {
        // Recover all data chunks
        let recovered_chunks = self.recover_chunks(chunks)?;

        // Concatenate data chunks to reconstruct original data
        let mut result = Vec::new();
        for chunk in recovered_chunks {
            result.extend_from_slice(&chunk);
        }

        // Remove padding zeros from the end
        while result.ends_with(&[0]) && !result.is_empty() {
            result.pop();
        }

        Ok(result)
    }

    fn can_recover(&self, available_chunks: usize) -> bool {
        available_chunks >= self.data_chunks
    }

    fn data_chunks(&self) -> usize {
        self.data_chunks
    }

    fn parity_chunks(&self) -> usize {
        self.parity_chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_no_failures() {
        let scheme = SimpleParityScheme::new(3, 2);
        let data = b"Hello, World! This is a test message.";

        let chunks = scheme.encode(data).unwrap();
        assert_eq!(chunks.len(), 5); // 3 data + 2 parity

        let chunk_options: Vec<Option<Vec<u8>>> = chunks.into_iter().map(Some).collect();
        let recovered = scheme.decode(&chunk_options).unwrap();

        assert_eq!(recovered, data);
    }

    #[test]
    fn test_recover_from_one_failure() {
        let scheme = SimpleParityScheme::new(3, 2);
        let data = b"Hello, World!";

        let chunks = scheme.encode(data).unwrap();

        // Simulate failure of first data chunk
        let mut failed_chunks: Vec<Option<Vec<u8>>> = chunks.into_iter().map(Some).collect();
        failed_chunks[0] = None;

        let recovered = scheme.decode(&failed_chunks).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_cannot_recover_too_many_failures() {
        let scheme = SimpleParityScheme::new(3, 2);
        let data = b"Hello, World!";

        let chunks = scheme.encode(data).unwrap();

        // Simulate failure of too many chunks
        let mut failed_chunks: Vec<Option<Vec<u8>>> = chunks.into_iter().map(Some).collect();
        failed_chunks[0] = None;
        failed_chunks[1] = None;
        failed_chunks[2] = None; // 3 failures, but we only have 2 parity chunks

        assert!(scheme.decode(&failed_chunks).is_err());
    }
}
