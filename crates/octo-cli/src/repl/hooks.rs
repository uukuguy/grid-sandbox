//! Hook system for REPL lifecycle events
//!
//! Allows registering callbacks for tool execution events.
//! Future: load hooks from config file, support external process hooks.

/// Hook event types
#[derive(Debug, Clone)]
pub enum HookEvent {
    /// Fired before a tool is executed
    PreToolUse {
        tool_name: String,
        tool_id: String,
        input: serde_json::Value,
    },
    /// Fired after a tool completes
    PostToolUse {
        tool_name: String,
        tool_id: String,
        output: String,
        success: bool,
    },
    /// Fired when a session starts
    SessionStart { session_id: String },
    /// Fired when a session ends
    SessionEnd { session_id: String },
}

/// Hook handler trait
pub trait HookHandler: Send + Sync {
    fn on_event(&self, event: &HookEvent);
}

/// Simple logging hook handler (default)
pub struct LoggingHookHandler;

impl HookHandler for LoggingHookHandler {
    fn on_event(&self, event: &HookEvent) {
        match event {
            HookEvent::PreToolUse { tool_name, .. } => {
                tracing::debug!("hook: pre_tool_use {}", tool_name);
            }
            HookEvent::PostToolUse {
                tool_name, success, ..
            } => {
                tracing::debug!("hook: post_tool_use {} success={}", tool_name, success);
            }
            HookEvent::SessionStart { session_id } => {
                tracing::debug!("hook: session_start {}", session_id);
            }
            HookEvent::SessionEnd { session_id } => {
                tracing::debug!("hook: session_end {}", session_id);
            }
        }
    }
}

/// Hook dispatcher — manages registered hooks and dispatches events
pub struct HookDispatcher {
    handlers: Vec<Box<dyn HookHandler>>,
}

impl HookDispatcher {
    /// Create a new dispatcher with the default logging handler
    pub fn new() -> Self {
        Self {
            handlers: vec![Box::new(LoggingHookHandler)],
        }
    }

    /// Create a dispatcher with no handlers
    pub fn empty() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register an additional hook handler
    pub fn add_handler(&mut self, handler: Box<dyn HookHandler>) {
        self.handlers.push(handler);
    }

    /// Number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Dispatch an event to all registered handlers
    pub fn dispatch(&self, event: &HookEvent) {
        for handler in &self.handlers {
            handler.on_event(event);
        }
    }
}

impl Default for HookDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A test hook handler that counts events
    struct CountingHandler {
        count: Arc<AtomicUsize>,
    }

    impl HookHandler for CountingHandler {
        fn on_event(&self, _event: &HookEvent) {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// A test handler that records event names
    struct RecordingHandler {
        events: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl HookHandler for RecordingHandler {
        fn on_event(&self, event: &HookEvent) {
            let name = match event {
                HookEvent::PreToolUse { tool_name, .. } => {
                    format!("pre_tool_use:{}", tool_name)
                }
                HookEvent::PostToolUse { tool_name, .. } => {
                    format!("post_tool_use:{}", tool_name)
                }
                HookEvent::SessionStart { session_id } => {
                    format!("session_start:{}", session_id)
                }
                HookEvent::SessionEnd { session_id } => {
                    format!("session_end:{}", session_id)
                }
            };
            self.events.lock().unwrap().push(name);
        }
    }

    #[test]
    fn test_hook_event_debug() {
        let event = HookEvent::PreToolUse {
            tool_name: "bash".into(),
            tool_id: "t1".into(),
            input: serde_json::json!({"cmd": "ls"}),
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("PreToolUse"));
        assert!(debug.contains("bash"));
    }

    #[test]
    fn test_hook_event_clone() {
        let event = HookEvent::PostToolUse {
            tool_name: "file_read".into(),
            tool_id: "t2".into(),
            output: "contents".into(),
            success: true,
        };
        let cloned = event.clone();
        if let HookEvent::PostToolUse {
            tool_name, success, ..
        } = cloned
        {
            assert_eq!(tool_name, "file_read");
            assert!(success);
        } else {
            panic!("clone should preserve variant");
        }
    }

    #[test]
    fn test_dispatcher_default_has_logging_handler() {
        let dispatcher = HookDispatcher::new();
        assert_eq!(dispatcher.handler_count(), 1);
    }

    #[test]
    fn test_dispatcher_default_trait() {
        let dispatcher = HookDispatcher::default();
        assert_eq!(dispatcher.handler_count(), 1);
    }

    #[test]
    fn test_dispatcher_empty() {
        let dispatcher = HookDispatcher::empty();
        assert_eq!(dispatcher.handler_count(), 0);
    }

    #[test]
    fn test_dispatcher_add_handler() {
        let mut dispatcher = HookDispatcher::empty();
        let count = Arc::new(AtomicUsize::new(0));
        dispatcher.add_handler(Box::new(CountingHandler {
            count: count.clone(),
        }));
        assert_eq!(dispatcher.handler_count(), 1);

        dispatcher.dispatch(&HookEvent::SessionStart {
            session_id: "s1".into(),
        });
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_dispatcher_multiple_handlers() {
        let mut dispatcher = HookDispatcher::empty();
        let count1 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::new(AtomicUsize::new(0));

        dispatcher.add_handler(Box::new(CountingHandler {
            count: count1.clone(),
        }));
        dispatcher.add_handler(Box::new(CountingHandler {
            count: count2.clone(),
        }));

        let event = HookEvent::PreToolUse {
            tool_name: "bash".into(),
            tool_id: "t1".into(),
            input: serde_json::json!(null),
        };
        dispatcher.dispatch(&event);

        assert_eq!(count1.load(Ordering::Relaxed), 1);
        assert_eq!(count2.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_dispatcher_records_all_event_types() {
        let mut dispatcher = HookDispatcher::empty();
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        dispatcher.add_handler(Box::new(RecordingHandler {
            events: events.clone(),
        }));

        dispatcher.dispatch(&HookEvent::SessionStart {
            session_id: "s1".into(),
        });
        dispatcher.dispatch(&HookEvent::PreToolUse {
            tool_name: "bash".into(),
            tool_id: "t1".into(),
            input: serde_json::json!({"cmd": "ls"}),
        });
        dispatcher.dispatch(&HookEvent::PostToolUse {
            tool_name: "bash".into(),
            tool_id: "t1".into(),
            output: "file.txt".into(),
            success: true,
        });
        dispatcher.dispatch(&HookEvent::SessionEnd {
            session_id: "s1".into(),
        });

        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 4);
        assert_eq!(recorded[0], "session_start:s1");
        assert_eq!(recorded[1], "pre_tool_use:bash");
        assert_eq!(recorded[2], "post_tool_use:bash");
        assert_eq!(recorded[3], "session_end:s1");
    }

    #[test]
    fn test_logging_handler_does_not_panic() {
        let handler = LoggingHookHandler;
        handler.on_event(&HookEvent::PreToolUse {
            tool_name: "test".into(),
            tool_id: "t1".into(),
            input: serde_json::json!(null),
        });
        handler.on_event(&HookEvent::PostToolUse {
            tool_name: "test".into(),
            tool_id: "t1".into(),
            output: "ok".into(),
            success: false,
        });
        handler.on_event(&HookEvent::SessionStart {
            session_id: "s1".into(),
        });
        handler.on_event(&HookEvent::SessionEnd {
            session_id: "s1".into(),
        });
    }

    #[test]
    fn test_dispatch_with_no_handlers_is_noop() {
        let dispatcher = HookDispatcher::empty();
        // Should not panic
        dispatcher.dispatch(&HookEvent::PreToolUse {
            tool_name: "bash".into(),
            tool_id: "t1".into(),
            input: serde_json::json!(null),
        });
    }
}
