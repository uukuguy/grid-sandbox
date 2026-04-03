//! Interrupt manager wrapping octo-engine's CancellationToken for Ctrl+C handling.
//!
//! Single Ctrl+C sends Cancel to the agent; double Ctrl+C exits the TUI.

use grid_engine::agent::{AgentExecutorHandle, AgentMessage};

/// Manages Ctrl+C interrupt behavior for the TUI.
pub struct InterruptManager {
    ctrl_c_count: u8,
    handle: Option<AgentExecutorHandle>,
}

impl InterruptManager {
    /// Create without a handle (for initial setup before agent is started).
    pub fn new() -> Self {
        Self {
            ctrl_c_count: 0,
            handle: None,
        }
    }

    /// Create with an existing agent handle.
    pub fn with_handle(handle: AgentExecutorHandle) -> Self {
        Self {
            ctrl_c_count: 0,
            handle: Some(handle),
        }
    }

    /// Set or replace the agent handle.
    pub fn set_handle(&mut self, handle: AgentExecutorHandle) {
        self.handle = Some(handle);
    }

    /// Handle a Ctrl+C press. Returns `true` if the TUI should exit (double press).
    pub async fn handle_ctrl_c(&mut self) -> bool {
        self.ctrl_c_count += 1;
        if self.ctrl_c_count >= 2 {
            return true; // double press → exit TUI
        }
        // First press → cancel current agent operation
        if let Some(ref handle) = self.handle {
            let _ = handle.send(AgentMessage::Cancel).await;
        }
        false
    }

    /// Reset the Ctrl+C counter (call on user input or successful operation).
    pub fn reset(&mut self) {
        self.ctrl_c_count = 0;
    }

    /// Current Ctrl+C press count.
    pub fn press_count(&self) -> u8 {
        self.ctrl_c_count
    }
}

impl Default for InterruptManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_zero_count() {
        let mgr = InterruptManager::new();
        assert_eq!(mgr.press_count(), 0);
    }

    #[tokio::test]
    async fn test_first_ctrl_c_returns_false() {
        let mut mgr = InterruptManager::new();
        // No handle set — should still not panic, just return false
        let should_exit = mgr.handle_ctrl_c().await;
        assert!(!should_exit);
        assert_eq!(mgr.press_count(), 1);
    }

    #[tokio::test]
    async fn test_double_ctrl_c_returns_true() {
        let mut mgr = InterruptManager::new();
        mgr.handle_ctrl_c().await;
        let should_exit = mgr.handle_ctrl_c().await;
        assert!(should_exit);
        assert_eq!(mgr.press_count(), 2);
    }

    #[tokio::test]
    async fn test_reset_clears_count() {
        let mut mgr = InterruptManager::new();
        mgr.handle_ctrl_c().await;
        assert_eq!(mgr.press_count(), 1);
        mgr.reset();
        assert_eq!(mgr.press_count(), 0);

        // After reset, single press should not exit
        let should_exit = mgr.handle_ctrl_c().await;
        assert!(!should_exit);
    }
}
