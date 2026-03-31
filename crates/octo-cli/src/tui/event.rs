//! Unified event system for the conversation-centric TUI.

use crossterm::event::KeyEvent;
use octo_engine::agent::AgentEvent;

/// Application-level events
#[derive(Debug, Clone)]
pub enum AppEvent {
    // ── Terminal events ──
    /// Key event forwarded to active handler
    Key(KeyEvent),
    /// Mouse scroll event: (direction_up, row)
    MouseScroll { up: bool, row: u16 },
    /// Terminal resize
    Resize(u16, u16),
    /// Tick event (for animations/updates)
    Tick,

    // ── Agent events ──
    /// Agent lifecycle event bridged from broadcast::Receiver<AgentEvent>
    Agent(AgentEvent),
    /// User submitted input text
    UserSubmit(String),

    // ── Focus events ──
    /// Terminal window gained focus
    FocusGained,
    /// Terminal window lost focus
    FocusLost,

    // ── Application control ──
    /// Quit the application
    Quit,
}
