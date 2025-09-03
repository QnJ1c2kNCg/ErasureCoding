//! Terminal UI implementation using ratatui
//!
//! This module provides the main terminal interface for the erasure coding demo,
//! displaying node states, statistics, and handling user interactions.

use crate::simulation::{SimulationStatus, Simulator};
use crate::storage::NodeState;
use crate::ui::{LogEntry, UIConfig, UIEvent, UIState, HELP_TEXT};
use crate::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Main terminal UI coordinator
pub struct TerminalUI {
    /// Terminal backend
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// UI configuration
    config: UIConfig,
    /// Current UI state
    state: UIState,
    /// Log entries to display
    log_entries: Vec<LogEntry>,
    /// Last update time
    last_update: Instant,
    /// Whether the demo is paused
    paused: bool,
    /// Current simulation speed multiplier
    speed: f64,
    /// Test data key for demo
    test_data_key: String,
    /// Test data content for demo
    test_data: Vec<u8>,
}

impl TerminalUI {
    /// Create a new terminal UI
    pub fn new() -> Result<Self> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            config: UIConfig::default(),
            state: UIState::Menu,
            log_entries: Vec::new(),
            last_update: Instant::now(),
            paused: false,
            speed: 1.0,
            test_data_key: "demo_data".to_string(),
            test_data:
                b"Hello, Erasure Coding Demo! This is test data that will be split across nodes."
                    .to_vec(),
        })
    }

    /// Run the terminal UI with the given simulator
    pub async fn run(&mut self, mut simulator: Simulator) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;

        self.log_info("Erasure Coding Demo started".to_string());
        self.log_info("Press 'H' for help, 'S' to start demo, 'Q' to quit".to_string());

        let mut event_receiver = self.setup_event_handling().await?;

        loop {
            // Handle events
            while let Ok(event) = event_receiver.try_recv() {
                match event {
                    UIEvent::Quit => {
                        self.state = UIState::Shutdown;
                        break;
                    }
                    UIEvent::StartDemo => {
                        self.state = UIState::Running;
                        self.paused = false;
                        simulator.set_speed(self.speed);
                        self.log_success("Demo started".to_string());
                    }
                    UIEvent::TogglePause => {
                        if self.state == UIState::Running {
                            self.paused = !self.paused;
                            let msg = if self.paused { "Paused" } else { "Resumed" };
                            self.log_info(msg.to_string());
                        }
                    }
                    UIEvent::ShowHelp => {
                        self.state = if self.state == UIState::Help {
                            UIState::Running
                        } else {
                            UIState::Help
                        };
                    }
                    UIEvent::FailRandomNode => {
                        if !self.paused {
                            match simulator
                                .run_failure_scenario(
                                    crate::simulation::FailureScenario::SingleNodeFailure,
                                )
                                .await
                            {
                                Ok(_) => self.log_warn("Random node failed".to_string()),
                                Err(e) => self.log_error(format!("Failed to fail node: {}", e)),
                            }
                        }
                    }
                    UIEvent::RecoverRandomNode => {
                        if !self.paused {
                            match simulator.recover_random_node().await {
                                Ok(true) => self.log_success("Node recovered".to_string()),
                                Ok(false) => {
                                    self.log_warn("No failed nodes to recover".to_string())
                                }
                                Err(e) => self.log_error(format!("Recovery failed: {}", e)),
                            }
                        }
                    }
                    UIEvent::FailAllNodes => {
                        if !self.paused {
                            let node_count = simulator.cluster.node_count();
                            match simulator
                                .run_failure_scenario(
                                    crate::simulation::FailureScenario::CascadingFailures(
                                        node_count,
                                    ),
                                )
                                .await
                            {
                                Ok(_) => self.log_error("All nodes failed!".to_string()),
                                Err(e) => self.log_error(format!("Failed to fail nodes: {}", e)),
                            }
                        }
                    }
                    UIEvent::RecoverAllNodes => {
                        if !self.paused {
                            match simulator.recover_all_nodes().await {
                                Ok(count) => self.log_success(format!("Recovered {} nodes", count)),
                                Err(e) => self.log_error(format!("Recovery failed: {}", e)),
                            }
                        }
                    }
                    UIEvent::StoreData => {
                        if !self.paused {
                            match simulator.store_test_data(&self.test_data_key, &self.test_data) {
                                Ok(_) => self.log_success("Test data stored".to_string()),
                                Err(e) => self.log_error(format!("Storage failed: {}", e)),
                            }
                        }
                    }
                    UIEvent::RetrieveData => {
                        if !self.paused {
                            match simulator.retrieve_test_data(&self.test_data_key) {
                                Ok(data) => {
                                    if data == self.test_data {
                                        self.log_success("Data retrieved successfully".to_string());
                                    } else {
                                        self.log_error(
                                            "Retrieved data doesn't match original".to_string(),
                                        );
                                    }
                                }
                                Err(e) => self.log_error(format!("Retrieval failed: {}", e)),
                            }
                        }
                    }
                    UIEvent::Reset => {
                        // Reset all nodes to healthy
                        let node_ids: Vec<_> = simulator.cluster.node_ids();
                        for node_id in node_ids {
                            let _ = simulator.cluster.recover_node(node_id);
                        }
                        self.log_info("Simulation reset".to_string());
                    }
                    UIEvent::IncreaseSpeed => {
                        self.speed = (self.speed * 1.5).min(10.0);
                        simulator.set_speed(self.speed);
                        self.log_info(format!("Speed: {:.1}x", self.speed));
                    }
                    UIEvent::DecreaseSpeed => {
                        self.speed = (self.speed / 1.5).max(0.1);
                        simulator.set_speed(self.speed);
                        self.log_info(format!("Speed: {:.1}x", self.speed));
                    }
                    UIEvent::Unknown(_) => {
                        // Ignore unknown keys
                    }
                }
            }

            if self.state == UIState::Shutdown {
                break;
            }

            // Update display
            if self.last_update.elapsed() >= Duration::from_millis(self.config.update_interval_ms) {
                let status = simulator.status();
                self.draw(&status)?;
                self.last_update = Instant::now();
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Cleanup
        self.cleanup()?;
        Ok(())
    }

    /// Setup event handling
    async fn setup_event_handling(&self) -> Result<mpsc::UnboundedReceiver<UIEvent>> {
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                if let Ok(true) = event::poll(Duration::from_millis(50)) {
                    if let Ok(event) = event::read() {
                        if let Event::Key(key) = event {
                            let ui_event = UIEvent::from(key);
                            if tx.send(ui_event).is_err() {
                                break; // Receiver dropped
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Draw the UI
    fn draw(&mut self, status: &SimulationStatus) -> Result<()> {
        let state = self.state.clone();
        let config = self.config.clone();
        let log_entries = self.log_entries.clone();
        let speed = self.speed;
        let paused = self.paused;

        self.terminal.draw(|f| match state {
            UIState::Help => {
                Self::render_help_static(f, &config);
            }
            _ => {
                Self::render_main_static(f, status, &config, &log_entries, speed, paused, &state);
            }
        })?;
        Ok(())
    }

    /// Render the main UI
    fn render_main_static(
        f: &mut Frame,
        status: &SimulationStatus,
        config: &UIConfig,
        log_entries: &[LogEntry],
        speed: f64,
        paused: bool,
        state: &UIState,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Main content
                Constraint::Length(8), // Logs
                Constraint::Length(3), // Status bar
            ])
            .split(f.size());

        // Title
        Self::render_title_static(f, chunks[0], config, paused, state);

        // Main content area
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(2, 3), // Node visualization
                Constraint::Ratio(1, 3), // Statistics
            ])
            .split(chunks[1]);

        Self::render_nodes_static(f, main_chunks[0], status, config);
        Self::render_statistics_static(f, main_chunks[1], status, config, speed);

        // Logs
        Self::render_logs_static(f, chunks[2], log_entries, config);

        // Status bar
        Self::render_status_bar_static(f, chunks[3], status, config, state, paused);
    }

    /// Render the title bar
    fn render_title_static(
        f: &mut Frame,
        area: Rect,
        config: &UIConfig,
        paused: bool,
        state: &UIState,
    ) {
        let title = match state {
            UIState::Menu => "Erasure Coding Demo - Press 'S' to start",
            UIState::Running if paused => "Erasure Coding Demo - PAUSED",
            UIState::Running => "Erasure Coding Demo - RUNNING",
            UIState::Paused => "Erasure Coding Demo - PAUSED",
            _ => "Erasure Coding Demo",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(config.colors.highlight));

        let paragraph = Paragraph::new(title)
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(paragraph, area);
    }

    /// Render node visualization
    fn render_nodes_static(
        f: &mut Frame,
        area: Rect,
        status: &SimulationStatus,
        config: &UIConfig,
    ) {
        let block = Block::default()
            .title("Storage Nodes")
            .borders(Borders::ALL);

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Calculate grid layout for nodes
        let cols = ((status.total_nodes as f64).sqrt().ceil() as u16).max(1);
        let rows = ((status.total_nodes as f64 / cols as f64).ceil() as u16).max(1);

        let cell_width = inner.width / cols;
        let cell_height = inner.height / rows;

        // Render each node
        for i in 0..status.total_nodes {
            let col = i % cols as usize;
            let row = i / cols as usize;

            let x = inner.x + (col as u16 * cell_width);
            let y = inner.y + (row as u16 * cell_height);

            let node_area = Rect {
                x,
                y,
                width: cell_width.saturating_sub(1),
                height: cell_height.saturating_sub(1),
            };

            // Determine node state (simplified for demo)
            let node_state = if i < status.healthy_nodes {
                NodeState::Healthy
            } else if i < status.healthy_nodes + status.degraded_nodes {
                NodeState::Degraded
            } else {
                NodeState::Failed
            };

            Self::render_single_node_static(f, node_area, i, &node_state, config);
        }
    }

    /// Render a single node
    fn render_single_node_static(
        f: &mut Frame,
        area: Rect,
        node_id: usize,
        state: &NodeState,
        config: &UIConfig,
    ) {
        let color = match state {
            NodeState::Healthy => config.colors.healthy,
            NodeState::Degraded => config.colors.degraded,
            NodeState::Failed => config.colors.failed,
        };

        let symbol = match state {
            NodeState::Healthy => "●",
            NodeState::Degraded => "◐",
            NodeState::Failed => "○",
        };

        let text = format!("{}\n{}", symbol, node_id);

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(color))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }

    /// Render statistics panel
    fn render_statistics_static(
        f: &mut Frame,
        area: Rect,
        status: &SimulationStatus,
        config: &UIConfig,
        speed: f64,
    ) {
        let block = Block::default().title("Statistics").borders(Borders::ALL);

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Health gauge
                Constraint::Min(5),    // Stats text
            ])
            .split(inner);

        // Health gauge
        let health_ratio = status.health_percentage() / 100.0;
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title("Cluster Health")
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(if health_ratio > 0.7 {
                config.colors.healthy
            } else if health_ratio > 0.3 {
                config.colors.degraded
            } else {
                config.colors.failed
            }))
            .ratio(health_ratio);

        f.render_widget(gauge, chunks[0]);

        // Statistics text
        let stats_text = format!(
            "Total Nodes: {}\nHealthy: {}\nDegraded: {}\nFailed: {}\n\nCan Recover: {}\nFailure Tolerance: {}\n\nTotal Chunks: {}\nTotal Bytes: {}\n\nSpeed: {:.1}x",
            status.total_nodes,
            status.healthy_nodes,
            status.degraded_nodes,
            status.failed_nodes,
            if status.can_recover { "Yes" } else { "No" },
            status.failure_tolerance,
            status.total_chunks,
            status.total_bytes,
            speed
        );

        let paragraph = Paragraph::new(stats_text).wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[1]);
    }

    /// Render logs panel
    fn render_logs_static(f: &mut Frame, area: Rect, log_entries: &[LogEntry], config: &UIConfig) {
        let block = Block::default().title("Activity Log").borders(Borders::ALL);

        let items: Vec<ListItem> = log_entries
            .iter()
            .rev()
            .take(area.height.saturating_sub(2) as usize)
            .map(|entry| {
                let color = entry.level.color(&config.colors);
                ListItem::new(entry.format()).style(Style::default().fg(color))
            })
            .collect();

        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }

    /// Render status bar
    fn render_status_bar_static(
        f: &mut Frame,
        area: Rect,
        status: &SimulationStatus,
        config: &UIConfig,
        state: &UIState,
        paused: bool,
    ) {
        let status_text = format!(
            "State: {} | Health: {} | Press 'H' for help",
            match (state, paused) {
                (UIState::Menu, _) => "Menu",
                (UIState::Running, true) => "Paused",
                (UIState::Running, false) => "Running",
                (UIState::Paused, _) => "Paused",
                (UIState::Help, _) => "Help",
                (UIState::Shutdown, _) => "Shutdown",
            },
            status.health_description()
        );

        let paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(config.colors.text))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }

    /// Render help screen
    fn render_help_static(f: &mut Frame, config: &UIConfig) {
        let area = f.size();

        // Clear the background
        f.render_widget(Clear, area);

        // Create centered help popup
        let popup_area = crate::ui::utils::centered_rect(60, 30, area);

        let block = Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .style(Style::default().fg(config.colors.highlight));

        let paragraph = Paragraph::new(HELP_TEXT)
            .block(block)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(config.colors.text));

        f.render_widget(paragraph, popup_area);
    }

    /// Add a log entry
    fn log(&mut self, entry: LogEntry) {
        self.log_entries.push(entry);
        if self.log_entries.len() > self.config.max_log_entries {
            self.log_entries.remove(0);
        }
    }

    /// Add an info log entry
    fn log_info(&mut self, message: String) {
        self.log(LogEntry::info(message));
    }

    /// Add a warning log entry
    fn log_warn(&mut self, message: String) {
        self.log(LogEntry::warn(message));
    }

    /// Add an error log entry
    fn log_error(&mut self, message: String) {
        self.log(LogEntry::error(message));
    }

    /// Add a success log entry
    fn log_success(&mut self, message: String) {
        self.log(LogEntry::success(message));
    }

    /// Cleanup terminal state
    fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TerminalUI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
