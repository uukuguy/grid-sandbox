//! Extension system for octo-engine.
//!
//! Provides runtime extensibility through:
//! - Extension trait: lifecycle hooks for agent execution
//! - ExtensionHostActions: host actions available to extensions
//! - HostcallInterceptor: intercept and modify tool calls
//! - ExtensionManager: manages all registered extensions

pub mod context;
pub mod manager;
pub mod traits;

pub use context::ExtensionContext;
pub use manager::{ExtensionManager, LoggingExtension};
pub use traits::{
    AgentResult, Extension, ExtensionEvent, ExtensionHostActions, HostcallInterceptor,
    InMemoryExtensionHostActions,
};
