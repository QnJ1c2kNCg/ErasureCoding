//! Failure simulation utilities
//!
//! This module provides utilities for simulating various types of node failures
//! and network issues that can occur in distributed storage systems.

use crate::storage::NodeId;
use rand::prelude::*;
use std::time::Duration;

/// Failure pattern generator for creating realistic failure scenarios
pub struct FailureGenerator {
    rng: ThreadRng,
    /// Base probability of failure per node
    base_failure_rate: f64,
    /// Correlation factor for cascading failures
    cascade_factor: f64,
}

impl FailureGenerator {
    /// Create a new failure generator with default parameters
    pub fn new() -> Self {
        Self {
            rng: thread_rng(),
            base_failure_rate: 0.01, // 1% per time unit
            cascade_factor: 1.5,     // 50% increase in failure probability after each failure
        }
    }

    /// Create a failure generator with custom parameters
    pub fn with_rates(base_failure_rate: f64, cascade_factor: f64) -> Self {
        Self {
            rng: thread_rng(),
            base_failure_rate,
            cascade_factor,
        }
    }

    /// Generate a failure schedule for a set of nodes
    pub fn generate_failure_schedule(
        &mut self,
        node_ids: &[NodeId],
        duration: Duration,
    ) -> Vec<FailureEvent> {
        let mut events = Vec::new();
        let time_step = Duration::from_millis(100); // Check every 100ms
        let steps = duration.as_millis() / time_step.as_millis();

        let mut current_failure_rate = self.base_failure_rate;

        for step in 0..steps {
            let timestamp = Duration::from_millis(step as u64 * time_step.as_millis() as u64);

            for &node_id in node_ids {
                if self.rng.gen::<f64>() < current_failure_rate {
                    events.push(FailureEvent {
                        node_id,
                        timestamp,
                        failure_type: self.generate_failure_type(),
                    });

                    // Increase failure probability for cascade effect
                    current_failure_rate *= self.cascade_factor;
                    current_failure_rate = current_failure_rate.min(0.5); // Cap at 50%
                }
            }

            // Gradually decrease failure rate back to baseline
            current_failure_rate = (current_failure_rate * 0.99).max(self.base_failure_rate);
        }

        // Sort events by timestamp
        events.sort_by_key(|e| e.timestamp);
        events
    }

    /// Generate a random failure type
    fn generate_failure_type(&mut self) -> FailureType {
        let rand_val = self.rng.gen::<f64>();
        match rand_val {
            r if r < 0.6 => FailureType::HardwareFailure,
            r if r < 0.8 => FailureType::NetworkTimeout,
            r if r < 0.9 => FailureType::DiskFull,
            _ => FailureType::PowerOutage,
        }
    }

    /// Generate correlated failures (nodes that tend to fail together)
    pub fn generate_correlated_failures(
        &mut self,
        node_groups: &[Vec<NodeId>],
        correlation: f64,
    ) -> Vec<FailureEvent> {
        let mut events = Vec::new();

        for group in node_groups {
            if self.rng.gen::<f64>() < correlation {
                // This group will experience correlated failures
                let base_time = Duration::from_millis(self.rng.gen_range(0..10000));
                let failure_type = self.generate_failure_type();

                for &node_id in group {
                    let jitter = Duration::from_millis(self.rng.gen_range(0..1000));
                    events.push(FailureEvent {
                        node_id,
                        timestamp: base_time + jitter,
                        failure_type: failure_type.clone(),
                    });
                }
            }
        }

        events.sort_by_key(|e| e.timestamp);
        events
    }
}

impl Default for FailureGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a scheduled failure event
#[derive(Debug, Clone)]
pub struct FailureEvent {
    /// The node that will fail
    pub node_id: NodeId,
    /// When the failure will occur
    pub timestamp: Duration,
    /// Type of failure
    pub failure_type: FailureType,
}

/// Types of failures that can occur
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FailureType {
    /// Complete hardware failure (node goes offline)
    HardwareFailure,
    /// Network connectivity issues (timeouts, packet loss)
    NetworkTimeout,
    /// Storage disk becomes full
    DiskFull,
    /// Power outage affecting the node
    PowerOutage,
    /// Software crash or corruption
    SoftwareFailure,
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureType::HardwareFailure => write!(f, "Hardware Failure"),
            FailureType::NetworkTimeout => write!(f, "Network Timeout"),
            FailureType::DiskFull => write!(f, "Disk Full"),
            FailureType::PowerOutage => write!(f, "Power Outage"),
            FailureType::SoftwareFailure => write!(f, "Software Failure"),
        }
    }
}

/// Recovery time estimator based on failure type
pub fn estimate_recovery_time(failure_type: &FailureType) -> Duration {
    match failure_type {
        FailureType::NetworkTimeout => Duration::from_secs(30), // Quick network recovery
        FailureType::SoftwareFailure => Duration::from_secs(120), // Software restart
        FailureType::DiskFull => Duration::from_secs(300),      // Clear disk space
        FailureType::HardwareFailure => Duration::from_secs(3600), // Replace hardware
        FailureType::PowerOutage => Duration::from_secs(1800),  // Restore power
    }
}

/// Predefined failure scenarios for common testing patterns
pub struct FailureScenarios;

impl FailureScenarios {
    /// Classic "rack failure" - multiple nodes fail simultaneously
    pub fn rack_failure(rack_nodes: Vec<NodeId>) -> Vec<FailureEvent> {
        let base_time = Duration::from_secs(5);
        rack_nodes
            .into_iter()
            .enumerate()
            .map(|(i, node_id)| FailureEvent {
                node_id,
                timestamp: base_time + Duration::from_millis(i as u64 * 50), // Small stagger
                failure_type: FailureType::PowerOutage,
            })
            .collect()
    }

    /// Rolling failure - nodes fail one after another in sequence
    pub fn rolling_failure(node_ids: Vec<NodeId>, interval: Duration) -> Vec<FailureEvent> {
        node_ids
            .into_iter()
            .enumerate()
            .map(|(i, node_id)| FailureEvent {
                node_id,
                timestamp: Duration::from_millis(i as u64 * interval.as_millis() as u64),
                failure_type: FailureType::HardwareFailure,
            })
            .collect()
    }

    /// Byzantine failure - nodes fail in unpredictable patterns
    pub fn byzantine_failure(node_ids: Vec<NodeId>) -> Vec<FailureEvent> {
        let mut rng = thread_rng();
        let mut events = Vec::new();

        for node_id in node_ids {
            if rng.gen::<f64>() < 0.4 {
                // 40% chance each node fails
                events.push(FailureEvent {
                    node_id,
                    timestamp: Duration::from_millis(rng.gen_range(0..30000)),
                    failure_type: match rng.gen_range(0..3) {
                        0 => FailureType::SoftwareFailure,
                        1 => FailureType::NetworkTimeout,
                        _ => FailureType::HardwareFailure,
                    },
                });
            }
        }

        events.sort_by_key(|e| e.timestamp);
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_generator() {
        let mut generator = FailureGenerator::new();
        let nodes = vec![0, 1, 2, 3, 4];
        let events = generator.generate_failure_schedule(&nodes, Duration::from_secs(10));

        // Should generate some events (probabilistic, so might be 0)
        assert!(events.len() <= nodes.len() * 100); // Sanity check

        // Events should be sorted by timestamp
        for window in events.windows(2) {
            assert!(window[0].timestamp <= window[1].timestamp);
        }
    }

    #[test]
    fn test_failure_scenarios() {
        let nodes = vec![0, 1, 2];

        let rack_events = FailureScenarios::rack_failure(nodes.clone());
        assert_eq!(rack_events.len(), 3);
        assert!(rack_events
            .iter()
            .all(|e| e.failure_type == FailureType::PowerOutage));

        let rolling_events = FailureScenarios::rolling_failure(nodes, Duration::from_secs(1));
        assert_eq!(rolling_events.len(), 3);

        // Should be spaced 1 second apart
        assert_eq!(
            rolling_events[1].timestamp - rolling_events[0].timestamp,
            Duration::from_secs(1)
        );
    }

    #[test]
    fn test_recovery_time_estimates() {
        assert!(
            estimate_recovery_time(&FailureType::NetworkTimeout)
                < estimate_recovery_time(&FailureType::HardwareFailure)
        );
        assert!(
            estimate_recovery_time(&FailureType::SoftwareFailure)
                < estimate_recovery_time(&FailureType::DiskFull)
        );
    }
}
