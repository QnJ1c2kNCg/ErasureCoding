//! Erasure Coding Demo - Main Application
//!
//! A terminal-based demonstration of erasure coding techniques, showing
//! how distributed storage systems can recover from node failures using
//! redundancy and mathematical reconstruction.

use clap::{Arg, Command};
use ErasureCoding::simulation::Simulator;
use ErasureCoding::{erasure, storage::Cluster, Config, Result, TerminalUI};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("Erasure Coding Demo")
        .version("0.1.0")
        .author("Erasure Coding Demo")
        .about("Interactive demonstration of erasure coding in distributed storage")
        .arg(
            Arg::new("nodes")
                .short('n')
                .long("nodes")
                .value_name("COUNT")
                .help("Number of storage nodes")
                .default_value("6"),
        )
        .arg(
            Arg::new("data-chunks")
                .short('d')
                .long("data-chunks")
                .value_name("COUNT")
                .help("Number of data chunks")
                .default_value("4"),
        )
        .arg(
            Arg::new("parity-chunks")
                .short('p')
                .long("parity-chunks")
                .value_name("COUNT")
                .help("Number of parity chunks")
                .default_value("2"),
        )
        .arg(
            Arg::new("demo")
                .long("demo")
                .value_name("TYPE")
                .help("Run specific demo: basic, stress, partition, recovery, performance, educational")
                .value_parser(["basic", "stress", "partition", "recovery", "performance", "educational"]),
        )
        .arg(
            Arg::new("headless")
                .long("headless")
                .help("Run without terminal UI (for testing)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Parse configuration
    let node_count: usize = matches
        .get_one::<String>("nodes")
        .unwrap()
        .parse()
        .map_err(|_| "Invalid node count")?;

    let data_chunks: usize = matches
        .get_one::<String>("data-chunks")
        .unwrap()
        .parse()
        .map_err(|_| "Invalid data chunk count")?;

    let parity_chunks: usize = matches
        .get_one::<String>("parity-chunks")
        .unwrap()
        .parse()
        .map_err(|_| "Invalid parity chunk count")?;

    // Validate configuration
    let config = Config::new(data_chunks, parity_chunks);
    config.validate()?;

    if node_count < config.total_nodes {
        return Err(format!(
            "Not enough nodes: need at least {} for {}/{} scheme",
            config.total_nodes, data_chunks, parity_chunks
        )
        .into());
    }

    println!("🚀 Erasure Coding Demo Starting...");
    println!(
        "   Configuration: {}/{} (data/parity chunks)",
        data_chunks, parity_chunks
    );
    println!("   Nodes: {}", node_count);
    println!(
        "   Fault Tolerance: {} node failures",
        config.max_failures()
    );
    println!();

    // Create cluster and simulator
    let mut cluster = Cluster::with_nodes(node_count);
    let scheme = erasure::create_simple_parity(data_chunks, parity_chunks);
    cluster.set_scheme(scheme);

    let simulator = Simulator::new(cluster);

    // Check if running in headless mode or specific demo
    if matches.get_flag("headless") {
        run_headless_demo(simulator, matches.get_one::<String>("demo")).await
    } else if let Some(demo_type) = matches.get_one::<String>("demo") {
        run_specific_demo(simulator, demo_type).await
    } else {
        run_interactive_demo(simulator).await
    }
}

/// Run the interactive terminal UI demo
async fn run_interactive_demo(simulator: Simulator) -> Result<()> {
    println!("🎮 Starting interactive demo...");
    println!("   Use keyboard controls to interact with the simulation");
    println!("   Press 'H' for help once the UI loads");
    println!();

    // Small delay to let user read the messages
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let mut ui = TerminalUI::new()?;
    ui.run(simulator).await?;

    println!("👋 Demo completed. Thanks for exploring erasure coding!");
    Ok(())
}

/// Run a specific demo scenario without full UI
async fn run_specific_demo(mut simulator: Simulator, demo_type: &str) -> Result<()> {
    use ErasureCoding::ui::demo::DemoScenarios;

    println!("🎯 Running {} demo...", demo_type);
    println!();

    let logs = match demo_type {
        "basic" => DemoScenarios::basic_demo(&mut simulator).await?,
        "stress" => DemoScenarios::stress_test_demo(&mut simulator).await?,
        "partition" => DemoScenarios::partition_demo(&mut simulator).await?,
        "recovery" => DemoScenarios::recovery_demo(&mut simulator).await?,
        "performance" => DemoScenarios::performance_demo(&mut simulator).await?,
        "educational" => DemoScenarios::educational_demo(&mut simulator).await?,
        _ => return Err(format!("Unknown demo type: {}", demo_type).into()),
    };

    // Print demo results
    for log in logs {
        println!("{}", log);
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    println!();
    println!("✅ Demo completed successfully!");
    Ok(())
}

/// Run headless demo for testing/automation
async fn run_headless_demo(mut simulator: Simulator, demo_type: Option<&String>) -> Result<()> {
    println!("🤖 Running in headless mode...");

    // Store some test data
    let test_data = b"Headless demo test data for erasure coding validation";
    simulator.store_test_data("headless_test", test_data)?;
    println!("✓ Test data stored");

    // Show initial cluster state
    let status = simulator.status();
    println!(
        "📊 Initial state: {}/{} nodes healthy",
        status.healthy_nodes, status.total_nodes
    );

    // Simulate failures
    println!("⚡ Simulating node failures...");
    for i in 1..=status.total_nodes / 2 {
        simulator
            .run_failure_scenario(ErasureCoding::simulation::FailureScenario::SingleNodeFailure)
            .await?;

        let current_status = simulator.status();
        println!(
            "   Failure {}: {}/{} nodes healthy, can recover: {}",
            i, current_status.healthy_nodes, current_status.total_nodes, current_status.can_recover
        );

        // Try to retrieve data
        match simulator.retrieve_test_data("headless_test") {
            Ok(retrieved) if retrieved == test_data => {
                println!("   ✓ Data still recoverable");
            }
            Ok(_) => {
                println!("   ⚠ Data corrupted");
            }
            Err(_) => {
                println!("   ✗ Data not recoverable");
                break;
            }
        }
    }

    // Run specific demo if requested
    if let Some(demo_type) = demo_type {
        println!("\n🎯 Running {} scenario...", demo_type);
        run_specific_demo(simulator, demo_type).await?;
    }

    println!("🏁 Headless demo completed");
    Ok(())
}
