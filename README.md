# Erasure Coding Demo

A terminal-based interactive demonstration of erasure coding techniques in distributed storage systems. This project shows how data can be protected and recovered from node failures using mathematical redundancy.

## What is Erasure Coding?

Erasure coding is a method of data protection where data is broken into fragments, expanded with redundant data pieces, and stored across multiple locations (nodes). If some nodes fail, the original data can be reconstructed from the remaining fragments using mathematical algorithms.

This is more storage-efficient than simple replication while providing the same or better fault tolerance.

## Features

- 🎮 **Interactive Terminal UI**: Real-time visualization of node states and recovery processes
- 🔧 **Configurable Parameters**: Customize data chunks, parity chunks, and node count
- 📊 **Multiple Demo Scenarios**: Educational, stress testing, network partition simulations
- 🧮 **Simple Parity Implementation**: Easy-to-understand XOR-based erasure coding
- 🔄 **Node Failure Simulation**: Realistic failure patterns and recovery mechanisms
- 📈 **Performance Metrics**: Track storage efficiency and fault tolerance limits

## Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))

### Installation

```bash
git clone <repository-url>
cd ErasureCoding
cargo build --release
```

### Basic Usage

```bash
# Start interactive demo with default settings (4 data + 2 parity chunks, 6 nodes)
cargo run

# Run educational demo in text mode
cargo run -- --demo educational

# Custom configuration: 8 nodes, 5 data + 3 parity chunks
cargo run -- --nodes 8 --data-chunks 5 --parity-chunks 3

# Run headless demo for testing
cargo run -- --headless --demo basic
```

## Interactive Controls

Once the terminal UI is running:

### Navigation
- **Q, Esc**: Quit application
- **H, F1**: Show/hide help

### Demo Controls
- **S**: Start/restart demo
- **Space**: Pause/unpause demo
- **X**: Reset simulation

### Node Operations
- **F**: Fail random node
- **R**: Recover random failed node
- **A**: Fail all nodes
- **C**: Recover all nodes

### Data Operations
- **D**: Store test data
- **G**: Retrieve test data

### Speed Control
- **+, =**: Increase simulation speed
- **-, _**: Decrease simulation speed

## Demo Scenarios

### Educational Demo (`--demo educational`)
Step-by-step explanation of how erasure coding works, perfect for learning.

### Basic Demo (`--demo basic`)
Simple demonstration of storing data, simulating failures, and recovering data.

### Stress Test (`--demo stress`)
Tests the system with cascading failures to find the breaking point.

### Network Partition (`--demo partition`)
Simulates network splits where groups of nodes become isolated.

### Performance Demo (`--demo performance`)
Shows storage efficiency and fault tolerance trade-offs.

### Recovery Demo (`--demo recovery`)
Demonstrates different recovery strategies and their effectiveness.

## Architecture

### Core Components

```
src/
├── erasure/           # Erasure coding algorithms
│   ├── simple_parity.rs   # XOR-based parity scheme
│   └── mod.rs             # Trait definitions
├── storage/           # Storage node simulation
│   ├── node.rs            # Individual node implementation
│   ├── cluster.rs         # Cluster management
│   └── mod.rs             # Storage abstractions
├── simulation/        # Demo orchestration
│   ├── failure.rs         # Failure pattern generation
│   ├── recovery.rs        # Recovery coordination
│   └── mod.rs             # Simulation control
├── ui/               # Terminal interface
│   ├── terminal.rs        # Main UI implementation
│   ├── demo.rs           # Demo scenarios
│   └── mod.rs            # UI utilities
└── main.rs           # Application entry point
```

### Erasure Coding Implementation

The project currently implements a simple XOR-based parity scheme:

- **Data Chunks**: Original data split into N equal parts
- **Parity Chunks**: M redundant chunks created using XOR operations
- **Recovery**: Can recover from up to M node failures
- **Storage Overhead**: (N + M) / N ratio (e.g., 4+2 = 1.5x overhead)

## Configuration Examples

### High Availability (3+3)
```bash
cargo run -- --data-chunks 3 --parity-chunks 3 --nodes 6
```
- Can survive 3 node failures
- 2x storage overhead
- Good for critical data

### Storage Efficient (8+2)
```bash
cargo run -- --data-chunks 8 --parity-chunks 2 --nodes 10
```
- Can survive 2 node failures
- 1.25x storage overhead
- Good for large datasets

### Balanced (4+2)
```bash
cargo run -- --data-chunks 4 --parity-chunks 2 --nodes 6
```
- Can survive 2 node failures
- 1.5x storage overhead
- Good general purpose setting

## Understanding the Display

### Node States
- 🟢 **Green (●)**: Healthy node
- 🟡 **Yellow (◐)**: Degraded node (slower responses)
- 🔴 **Red (○)**: Failed node

### Health Gauge
- **Green (90-100%)**: Excellent health
- **Yellow (70-89%)**: Good health
- **Orange (50-69%)**: Fair health
- **Red (<50%)**: Poor/Critical health

### Statistics Panel
- **Failure Tolerance**: How many more nodes can fail
- **Can Recover**: Whether stored data is still accessible
- **Total Chunks/Bytes**: Storage utilization

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

The project includes tests for:
- Erasure coding algorithms
- Node failure simulation
- Cluster management
- Recovery processes
- UI components

## Educational Value

This project demonstrates several computer science concepts:

1. **Distributed Systems**: Node coordination and failure handling
2. **Error Correction**: Mathematical redundancy for fault tolerance
3. **Storage Systems**: Trade-offs between space, time, and reliability
4. **Linear Algebra**: XOR operations and Galois field arithmetic
5. **Systems Programming**: Rust async programming and terminal UIs

## Future Enhancements

- [ ] Reed-Solomon codes for better efficiency
- [ ] Network topology awareness
- [ ] Disk I/O simulation
- [ ] Byzantine fault tolerance
- [ ] Performance benchmarking
- [ ] Web-based UI
- [ ] Distributed deployment

## References

- [Reed-Solomon Error Correction](https://en.wikipedia.org/wiki/Reed%E2%80%93Solomon_error_correction)
- [Erasure Coding for Distributed Storage](https://web.eecs.utk.edu/~jplank/plank/papers/CS-08-627.html)
- [RAID and Erasure Coding](https://queue.acm.org/detail.cfm?id=1317400)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## Useful Links

* [An introduction to Reed-Solomon codes: principles, architecture and implementation](https://www.cs.cmu.edu/~guyb/realworld/reedsolomon/reed_solomon_codes.html)
* [Reed–Solomon error correction (Wiki)](https://en.wikipedia.org/wiki/Reed%E2%80%93Solomon_error_correction)
* [The paper](https://sites.math.rutgers.edu/~zeilberg/akherim/ReedS1960.pdf)