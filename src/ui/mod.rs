//! Terminal user interface module
//!
//! This module provides a terminal-based user interface for visualizing
//! erasure coding operations, node states, and recovery processes.

pub mod demo;
pub mod terminal;

pub use terminal::TerminalUI;

use crossterm::event::{KeyCode, KeyEvent};

/// Events that can be triggered by user input
#[derive(Debug, Clone, PartialEq)]
pub enum UIEvent {
    /// User wants to quit the application
    Quit,
    /// User wants to start a demo
    StartDemo,
    /// User wants to pause/unpause the demo
    TogglePause,
    /// User wants to fail a random node
    FailRandomNode,
    /// User wants to recover a random failed node
    RecoverRandomNode,
    /// User wants to fail all nodes
    FailAllNodes,
    /// User wants to recover all nodes
    RecoverAllNodes,
    /// User wants to store test data
    StoreData,
    /// User wants to retrieve test data
    RetrieveData,
    /// User wants to reset the simulation
    Reset,
    /// User wants to increase simulation speed
    IncreaseSpeed,
    /// User wants to decrease simulation speed
    DecreaseSpeed,
    /// User wants to show help
    ShowHelp,
    /// User pressed an unrecognized key
    Unknown(KeyCode),
}

impl From<KeyEvent> for UIEvent {
    fn from(key_event: KeyEvent) -> Self {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => UIEvent::Quit,
            KeyCode::Char('s') | KeyCode::Char('S') => UIEvent::StartDemo,
            KeyCode::Char(' ') => UIEvent::TogglePause,
            KeyCode::Char('f') | KeyCode::Char('F') => UIEvent::FailRandomNode,
            KeyCode::Char('r') | KeyCode::Char('R') => UIEvent::RecoverRandomNode,
            KeyCode::Char('a') | KeyCode::Char('A') => UIEvent::FailAllNodes,
            KeyCode::Char('c') | KeyCode::Char('C') => UIEvent::RecoverAllNodes,
            KeyCode::Char('d') | KeyCode::Char('D') => UIEvent::StoreData,
            KeyCode::Char('g') | KeyCode::Char('G') => UIEvent::RetrieveData,
            KeyCode::Char('x') | KeyCode::Char('X') => UIEvent::Reset,
            KeyCode::Char('+') | KeyCode::Char('=') => UIEvent::IncreaseSpeed,
            KeyCode::Char('-') | KeyCode::Char('_') => UIEvent::DecreaseSpeed,
            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::F(1) => UIEvent::ShowHelp,
            KeyCode::Esc => UIEvent::Quit,
            other => UIEvent::Unknown(other),
        }
    }
}

/// Color scheme for the UI
#[derive(Debug, Clone, Copy)]
pub struct ColorScheme {
    pub healthy: ratatui::style::Color,
    pub degraded: ratatui::style::Color,
    pub failed: ratatui::style::Color,
    pub background: ratatui::style::Color,
    pub text: ratatui::style::Color,
    pub highlight: ratatui::style::Color,
    pub success: ratatui::style::Color,
    pub warning: ratatui::style::Color,
    pub error: ratatui::style::Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            healthy: ratatui::style::Color::Green,
            degraded: ratatui::style::Color::Yellow,
            failed: ratatui::style::Color::Red,
            background: ratatui::style::Color::Black,
            text: ratatui::style::Color::White,
            highlight: ratatui::style::Color::Cyan,
            success: ratatui::style::Color::Green,
            warning: ratatui::style::Color::Yellow,
            error: ratatui::style::Color::Red,
        }
    }
}

/// Configuration for UI rendering
#[derive(Debug, Clone)]
pub struct UIConfig {
    /// Color scheme to use
    pub colors: ColorScheme,
    /// Update frequency in milliseconds
    pub update_interval_ms: u64,
    /// Whether to show detailed statistics
    pub show_stats: bool,
    /// Whether to show help panel
    pub show_help: bool,
    /// Maximum number of log entries to keep
    pub max_log_entries: usize,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            colors: ColorScheme::default(),
            update_interval_ms: 100,
            show_stats: true,
            show_help: false,
            max_log_entries: 100,
        }
    }
}

/// State of the UI application
#[derive(Debug, Clone, PartialEq)]
pub enum UIState {
    /// Main menu/startup screen
    Menu,
    /// Demo is running
    Running,
    /// Demo is paused
    Paused,
    /// Help screen is shown
    Help,
    /// Application is shutting down
    Shutdown,
}

/// Log entry for displaying messages to the user
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp of the log entry
    pub timestamp: std::time::Instant,
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: std::time::Instant::now(),
            level,
            message,
        }
    }

    /// Create an info log entry
    pub fn info(message: String) -> Self {
        Self::new(LogLevel::Info, message)
    }

    /// Create a warning log entry
    pub fn warn(message: String) -> Self {
        Self::new(LogLevel::Warning, message)
    }

    /// Create an error log entry
    pub fn error(message: String) -> Self {
        Self::new(LogLevel::Error, message)
    }

    /// Create a success log entry
    pub fn success(message: String) -> Self {
        Self::new(LogLevel::Success, message)
    }

    /// Format the log entry for display
    pub fn format(&self) -> String {
        let elapsed = self.timestamp.elapsed();
        let seconds = elapsed.as_secs();
        let prefix = match self.level {
            LogLevel::Info => "[INFO]",
            LogLevel::Warning => "[WARN]",
            LogLevel::Error => "[ERROR]",
            LogLevel::Success => "[OK]",
        };
        format!("[{:3}s] {} {}", seconds, prefix, self.message)
    }
}

/// Log levels for UI messages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
}

impl LogLevel {
    /// Get the color for this log level
    pub fn color(&self, colors: &ColorScheme) -> ratatui::style::Color {
        match self {
            LogLevel::Info => colors.text,
            LogLevel::Warning => colors.warning,
            LogLevel::Error => colors.error,
            LogLevel::Success => colors.success,
        }
    }
}

/// Help text for the application
pub const HELP_TEXT: &str = r#"
Erasure Coding Demo - Controls

Navigation:
  Q, Esc    - Quit application
  H, F1     - Show/hide this help

Demo Controls:
  S         - Start/restart demo
  Space     - Pause/unpause demo
  X         - Reset simulation

Node Operations:
  F         - Fail random node
  R         - Recover random failed node
  A         - Fail all nodes
  C         - Recover all nodes

Data Operations:
  D         - Store test data
  G         - Retrieve test data

Speed Control:
  +, =      - Increase simulation speed
  -, _      - Decrease simulation speed

The demo shows:
- Green nodes: Healthy
- Yellow nodes: Degraded
- Red nodes: Failed
- Data chunks are distributed across nodes
- Recovery happens automatically when possible

Press any key to return to the demo.
"#;

/// Trait for components that can be rendered in the terminal
pub trait Renderable {
    /// Render this component to a ratatui frame
    fn render(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, config: &UIConfig);
}

/// Utility functions for UI rendering
pub mod utils {
    use ratatui::layout::{Constraint, Direction, Layout, Rect};

    /// Split a rectangle horizontally with given ratios
    pub fn horizontal_split(area: Rect, ratios: &[u16]) -> Vec<Rect> {
        let total: u32 = ratios.iter().map(|&x| x as u32).sum();
        let constraints: Vec<Constraint> = ratios
            .iter()
            .map(|&ratio| Constraint::Ratio(ratio as u32, total))
            .collect();

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }

    /// Split a rectangle vertically with given ratios
    pub fn vertical_split(area: Rect, ratios: &[u16]) -> Vec<Rect> {
        let total: u32 = ratios.iter().map(|&x| x as u32).sum();
        let constraints: Vec<Constraint> = ratios
            .iter()
            .map(|&ratio| Constraint::Ratio(ratio as u32, total))
            .collect();

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }

    /// Create a centered rectangle with given width and height
    pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height.saturating_sub(height)) / 2),
                Constraint::Length(height),
                Constraint::Length((area.height.saturating_sub(height)) / 2),
            ])
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((area.width.saturating_sub(width)) / 2),
                Constraint::Length(width),
                Constraint::Length((area.width.saturating_sub(width)) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}
