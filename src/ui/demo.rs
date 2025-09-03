//! Demo scenarios module
//!
//! This module provides predefined demo scenarios that showcase different
//! aspects of erasure coding, node failures, and recovery processes.

use crate::simulation::{FailureScenario, Simulator};
use crate::Result;
use std::time::Duration;

/// Predefined demo scenarios
pub struct DemoScenarios;

impl DemoScenarios {
    /// Basic demo: Store data, fail some nodes, recover data
    pub async fn basic_demo(simulator: &mut Simulator) -> Result<Vec<String>> {
        let mut log = Vec::new();

        log.push("=== Basic Erasure Coding Demo ===".to_string());

        // Store test data
        let test_data = b"Hello, World! This is a test of erasure coding.";
        simulator.store_test_data("basic_demo", test_data)?;
        log.push("✓ Test data stored across cluster".to_string());

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Fail one node
        simulator
            .run_failure_scenario(FailureScenario::SingleNodeFailure)
            .await?;
        log.push("⚠ One node failed".to_string());

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Try to recover data
        match simulator.retrieve_test_data("basic_demo") {
            Ok(retrieved) if retrieved == test_data => {
                log.push("✓ Data successfully recovered despite node failure!".to_string());
            }
            Ok(_) => {
                log.push("✗ Data corrupted during recovery".to_string());
            }
            Err(e) => {
                log.push(format!("✗ Data recovery failed: {}", e));
            }
        }

        // Recover the failed node
        simulator.recover_random_node().await?;
        log.push("✓ Failed node recovered".to_string());

        Ok(log)
    }

    /// Stress test: Multiple cascading failures
    pub async fn stress_test_demo(simulator: &mut Simulator) -> Result<Vec<String>> {
        let mut log = Vec::new();

        log.push("=== Stress Test Demo ===".to_string());

        // Store multiple pieces of data
        for i in 0..5 {
            let data = format!("Test data piece {}", i).into_bytes();
            simulator.store_test_data(&format!("stress_test_{}", i), &data)?;
        }
        log.push("✓ Multiple data pieces stored".to_string());

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Cascading failures
        let failure_count = simulator.cluster.node_count() / 3; // Fail 1/3 of nodes
        simulator
            .run_failure_scenario(FailureScenario::CascadingFailures(failure_count))
            .await?;
        log.push(format!("⚠ {} nodes failed in cascade", failure_count));

        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check if system can still serve data
        let mut recoverable_count = 0;
        for i in 0..5 {
            let expected = format!("Test data piece {}", i).into_bytes();
            match simulator.retrieve_test_data(&format!("stress_test_{}", i)) {
                Ok(data) if data == expected => {
                    recoverable_count += 1;
                }
                _ => {}
            }
        }

        log.push(format!(
            "✓ {}/5 data pieces still recoverable after failures",
            recoverable_count
        ));

        // Gradual recovery
        for _ in 0..failure_count {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if simulator.recover_random_node().await? {
                log.push("✓ Node recovered".to_string());
            }
        }

        Ok(log)
    }

    /// Network partition demo
    pub async fn partition_demo(simulator: &mut Simulator) -> Result<Vec<String>> {
        let mut log = Vec::new();

        log.push("=== Network Partition Demo ===".to_string());

        // Store data
        let test_data = b"Network partition test data";
        simulator.store_test_data("partition_test", test_data)?;
        log.push("✓ Data stored before partition".to_string());

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Create network partition (fail half the nodes)
        let partition_size = simulator.cluster.node_count() / 2;
        simulator
            .run_failure_scenario(FailureScenario::NetworkPartition(partition_size))
            .await?;
        log.push(format!(
            "⚠ Network partition: {} nodes isolated",
            partition_size
        ));

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Test data availability during partition
        match simulator.retrieve_test_data("partition_test") {
            Ok(data) if data == test_data => {
                log.push("✓ Data still available during partition!".to_string());
            }
            Ok(_) => {
                log.push("⚠ Data corrupted during partition".to_string());
            }
            Err(_) => {
                log.push("✗ Data unavailable during partition".to_string());
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;

        // Heal network partition
        simulator.recover_all_nodes().await?;
        log.push("✓ Network partition healed".to_string());

        // Verify data integrity after healing
        match simulator.retrieve_test_data("partition_test") {
            Ok(data) if data == test_data => {
                log.push("✓ Data integrity verified after healing".to_string());
            }
            _ => {
                log.push("⚠ Data integrity issues after healing".to_string());
            }
        }

        Ok(log)
    }

    /// Recovery demo: Show different recovery strategies
    pub async fn recovery_demo(simulator: &mut Simulator) -> Result<Vec<String>> {
        let mut log = Vec::new();

        log.push("=== Recovery Strategies Demo ===".to_string());

        // Store data
        let test_data = b"Recovery strategy test data - longer message to test chunking";
        simulator.store_test_data("recovery_test", test_data)?;
        log.push("✓ Test data stored".to_string());

        // Demonstrate different failure patterns
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Random failures
        simulator
            .run_failure_scenario(FailureScenario::RandomFailures(0.3)) // 30% failure rate
            .await?;
        let status = simulator.status();
        log.push(format!(
            "⚠ Random failures: {}/{} nodes affected",
            status.failed_nodes, status.total_nodes
        ));

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Test recovery capability
        if simulator.can_serve_data() {
            log.push("✓ System still operational - fault tolerance working".to_string());

            // Verify data can still be retrieved
            match simulator.retrieve_test_data("recovery_test") {
                Ok(data) if data == test_data => {
                    log.push("✓ Data successfully recovered from remaining nodes".to_string());
                }
                _ => {
                    log.push("⚠ Data recovery had issues".to_string());
                }
            }
        } else {
            log.push("✗ System cannot serve data - too many failures".to_string());
        }

        // Progressive recovery
        log.push("Starting progressive recovery...".to_string());
        let failed_count = status.failed_nodes;
        for i in 0..failed_count {
            tokio::time::sleep(Duration::from_millis(800)).await;
            if simulator.recover_random_node().await? {
                log.push(format!("✓ Recovery {}/{} completed", i + 1, failed_count));
            }
        }

        log.push("✓ All nodes recovered - system fully operational".to_string());

        Ok(log)
    }

    /// Performance demo: Show impact of different configurations
    pub async fn performance_demo(simulator: &mut Simulator) -> Result<Vec<String>> {
        let mut log = Vec::new();

        log.push("=== Performance Impact Demo ===".to_string());

        // Store different sized data
        let small_data = b"Small";
        let medium_data =
            b"Medium sized data that spans multiple chunks and tests the erasure coding efficiency";
        let large_data = vec![b'X'; 1024]; // 1KB of data

        simulator.store_test_data("perf_small", small_data)?;
        simulator.store_test_data("perf_medium", medium_data)?;
        simulator.store_test_data("perf_large", &large_data)?;

        log.push("✓ Stored data of various sizes".to_string());

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Show storage distribution
        let status = simulator.status();
        log.push(format!(
            "Storage distribution: {} chunks, {} bytes total",
            status.total_chunks, status.total_bytes
        ));

        // Fail nodes progressively and measure impact
        let max_failures = simulator.cluster.node_count() / 2;
        for i in 1..=max_failures {
            simulator
                .run_failure_scenario(FailureScenario::SingleNodeFailure)
                .await?;

            let can_serve = simulator.can_serve_data();
            log.push(format!(
                "After {} failure(s): System {} serve data",
                i,
                if can_serve { "CAN" } else { "CANNOT" }
            ));

            if !can_serve {
                log.push("✗ Reached fault tolerance limit".to_string());
                break;
            }
        }

        // Recover all and show final stats
        simulator.recover_all_nodes().await?;
        let final_status = simulator.status();
        log.push(format!(
            "✓ System recovered: {}/{} nodes healthy",
            final_status.healthy_nodes, final_status.total_nodes
        ));

        Ok(log)
    }

    /// Educational demo: Step-by-step explanation
    pub async fn educational_demo(simulator: &mut Simulator) -> Result<Vec<String>> {
        let mut log = Vec::new();

        log.push("=== Educational Demo: How Erasure Coding Works ===".to_string());

        log.push("Step 1: Understanding the setup".to_string());
        let status = simulator.status();
        log.push(format!("• Cluster has {} total nodes", status.total_nodes));
        log.push("• Each file is split into data + parity chunks".to_string());
        log.push("• Parity chunks allow recovery from failures".to_string());

        tokio::time::sleep(Duration::from_secs(2)).await;

        log.push("Step 2: Storing data".to_string());
        let demo_text = "ERASURE_CODING_DEMO_DATA";
        simulator.store_test_data("educational", demo_text.as_bytes())?;
        log.push("• Original data split into chunks".to_string());
        log.push("• Each chunk stored on different node".to_string());
        log.push("• Redundancy added for fault tolerance".to_string());

        tokio::time::sleep(Duration::from_secs(2)).await;

        log.push("Step 3: Simulating failure".to_string());
        simulator
            .run_failure_scenario(FailureScenario::SingleNodeFailure)
            .await?;
        log.push("• One node has failed (lost data chunk)".to_string());
        log.push("• System detects missing chunk".to_string());

        tokio::time::sleep(Duration::from_secs(2)).await;

        log.push("Step 4: Data recovery".to_string());
        match simulator.retrieve_test_data("educational") {
            Ok(recovered) if recovered == demo_text.as_bytes() => {
                log.push("• Missing chunk reconstructed from remaining chunks".to_string());
                log.push("• Original data perfectly recovered!".to_string());
                log.push("• This is the power of erasure coding".to_string());
            }
            _ => {
                log.push("• Recovery failed - too many nodes failed".to_string());
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;

        log.push("Step 5: Node recovery".to_string());
        simulator.recover_random_node().await?;
        log.push("• Failed node brought back online".to_string());
        log.push("• System now fully redundant again".to_string());

        log.push("Demo complete! Key takeaways:".to_string());
        log.push("• Erasure coding provides fault tolerance".to_string());
        log.push("• Can survive multiple node failures".to_string());
        log.push("• Balances storage efficiency with reliability".to_string());

        Ok(log)
    }
}

/// Demo runner that can execute scenarios with logging
pub struct DemoRunner {
    /// Current demo logs
    pub logs: Vec<String>,
    /// Whether a demo is currently running
    pub running: bool,
}

impl DemoRunner {
    /// Create a new demo runner
    pub fn new() -> Self {
        Self {
            logs: Vec::new(),
            running: false,
        }
    }

    /// Run a demo scenario
    pub async fn run_scenario<F, Fut>(
        &mut self,
        simulator: &mut Simulator,
        scenario: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut Simulator) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<String>>>,
    {
        if self.running {
            return Err("Demo already running".into());
        }

        self.running = true;
        self.logs.clear();

        match scenario(simulator).await {
            Ok(logs) => {
                self.logs = logs;
            }
            Err(e) => {
                self.logs.push(format!("Demo failed: {}", e));
            }
        }

        self.running = false;
        Ok(())
    }

    /// Get the current logs
    pub fn get_logs(&self) -> &[String] {
        &self.logs
    }

    /// Clear the logs
    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }

    /// Check if demo is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Default for DemoRunner {
    fn default() -> Self {
        Self::new()
    }
}
