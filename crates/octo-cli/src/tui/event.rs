//! Unified event system for the TUI.
//!
//! Supports both the legacy 12-Tab architecture and the new
//! conversation-centric + AgentEvent-driven architecture.

use crossterm::event::KeyEvent;
use octo_engine::agent::AgentEvent;

use super::Tab;

/// Application-level events
#[derive(Debug, Clone)]
pub enum AppEvent {
    // ── Terminal events ──
    /// Key event forwarded to active screen
    Key(KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick event (for animations/updates)
    Tick,

    // ── Agent events (new: conversation-centric TUI) ──
    /// Agent lifecycle event bridged from broadcast::Receiver<AgentEvent>
    Agent(AgentEvent),
    /// User submitted input text
    UserSubmit(String),

    // ── Application control ──
    /// Quit the application
    Quit,

    // ── Legacy events (12-Tab architecture, to be removed in T2-8) ──
    /// Switch to next tab
    NextTab,
    /// Switch to previous tab
    PrevTab,
    /// Select a specific tab
    SelectTab(Tab),
    /// Set status bar message
    SetStatus(String),
    /// Clear status bar
    ClearStatus,
    /// Switch to Ops view mode
    SwitchToOps,
    /// Switch to Dev view mode
    SwitchToDev,
}
