//! Extension manager.

use std::sync::Arc;

use async_trait::async_trait;

use crate::extension::context::ExtensionContext;
use crate::extension::traits::{
    AgentResult, Extension, ExtensionEvent, ExtensionHostActions, HostcallInterceptor,
};

/// Manager for all registered extensions.
pub struct ExtensionManager {
    extensions: Vec<Box<dyn Extension>>,
    interceptors: Vec<Box<dyn HostcallInterceptor>>,
    host_actions: Arc<dyn ExtensionHostActions>,
}

impl ExtensionManager {
    /// Create a new extension manager.
    pub fn new(host_actions: Arc<dyn ExtensionHostActions>) -> Self {
        Self {
            extensions: Vec::new(),
            interceptors: Vec::new(),
            host_actions,
        }
    }

    /// Register an extension.
    pub fn register(&mut self, extension: Box<dyn Extension>) {
        tracing::info!(
            "Registering extension: {} v{}",
            extension.name(),
            extension.version()
        );
        self.extensions.push(extension);
    }

    /// Register a hostcall interceptor.
    pub fn register_interceptor(&mut self, interceptor: Box<dyn HostcallInterceptor>) {
        tracing::info!("Registering hostcall interceptor");
        self.interceptors.push(interceptor);
    }

    /// Get the host actions.
    pub fn host_actions(&self) -> &Arc<dyn ExtensionHostActions> {
        &self.host_actions
    }

    /// Get all extensions.
    pub fn extensions(&self) -> &[Box<dyn Extension>] {
        &self.extensions
    }

    /// Get all interceptors.
    pub fn interceptors(&self) -> &[Box<dyn HostcallInterceptor>] {
        &self.interceptors
    }

    /// Call on_agent_start for all extensions.
    pub async fn notify_agent_start(&self, ctx: &mut ExtensionContext) {
        for ext in &self.extensions {
            match ext.on_agent_start(ctx).await {
                Ok(_) => tracing::debug!("Extension {} started", ext.name()),
                Err(e) => tracing::warn!("Extension {} start failed: {}", ext.name(), e),
            }
        }
    }

    /// Call on_agent_end for all extensions.
    pub async fn notify_agent_end(&self, ctx: &mut ExtensionContext, result: &AgentResult) {
        for ext in &self.extensions {
            match ext.on_agent_end(ctx, result).await {
                Ok(_) => tracing::debug!("Extension {} ended", ext.name()),
                Err(e) => tracing::warn!("Extension {} end failed: {}", ext.name(), e),
            }
        }
    }

    /// Call on_tool_call for all extensions.
    /// Returns modified output if any extension intercepts.
    pub async fn notify_tool_call(
        &self,
        ctx: &mut ExtensionContext,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Option<String> {
        for ext in &self.extensions {
            match ext.on_tool_call(ctx, tool_name, arguments).await {
                Ok(Some(output)) => {
                    tracing::debug!("Extension {} intercepted tool call", ext.name());
                    return Some(output);
                }
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("Extension {} tool call failed: {}", ext.name(), e);
                }
            }
        }
        None
    }

    /// Call on_tool_result for all extensions.
    /// Returns modified result if any extension intercepts.
    pub async fn notify_tool_result(
        &self,
        ctx: &mut ExtensionContext,
        tool_name: &str,
        result: &str,
    ) -> Option<String> {
        for ext in &self.extensions {
            match ext.on_tool_result(ctx, tool_name, result).await {
                Ok(Some(output)) => {
                    tracing::debug!("Extension {} intercepted tool result", ext.name());
                    return Some(output);
                }
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!("Extension {} tool result failed: {}", ext.name(), e);
                }
            }
        }
        None
    }

    /// Call on_before_compaction for all extensions.
    pub async fn notify_before_compaction(
        &self,
        ctx: &mut ExtensionContext,
        messages_json: &str,
    ) {
        for ext in &self.extensions {
            match ext.on_before_compaction(ctx, messages_json).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(
                        "Extension {} before compaction failed: {}",
                        ext.name(),
                        e
                    );
                }
            }
        }
    }

    /// Call on_after_compaction for all extensions.
    pub async fn notify_after_compaction(
        &self,
        ctx: &mut ExtensionContext,
        messages_json: &str,
    ) {
        for ext in &self.extensions {
            match ext.on_after_compaction(ctx, messages_json).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(
                        "Extension {} after compaction failed: {}",
                        ext.name(),
                        e
                    );
                }
            }
        }
    }

    /// Check if path is allowed through interceptors.
    pub fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        for interceptor in &self.interceptors {
            if !interceptor.is_path_allowed(path) {
                return false;
            }
        }
        true
    }

    /// Check if command is allowed through interceptors.
    pub fn is_command_allowed(&self, command: &str) -> bool {
        for interceptor in &self.interceptors {
            if !interceptor.is_command_allowed(command) {
                return false;
            }
        }
        true
    }

    /// Intercept file read if any interceptor handles it.
    pub fn intercept_file_read(&self, path: &std::path::Path) -> Option<String> {
        for interceptor in &self.interceptors {
            if let Some(content) = interceptor.intercept_file_read(path) {
                return Some(content);
            }
        }
        None
    }

    /// Intercept file write if any interceptor handles it.
    pub fn intercept_file_write(
        &self,
        path: &std::path::Path,
        content: &str,
    ) -> Option<Result<(), String>> {
        for interceptor in &self.interceptors {
            if let Some(result) = interceptor.intercept_file_write(path, content) {
                return Some(result);
            }
        }
        None
    }

    /// Intercept shell command if any interceptor handles it.
    pub fn intercept_shell(&self, command: &str) -> Option<Result<String, String>> {
        for interceptor in &self.interceptors {
            if let Some(result) = interceptor.intercept_shell(command) {
                return Some(result);
            }
        }
        None
    }
}

/// Simple extension that logs all events.
pub struct LoggingExtension;

impl LoggingExtension {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Extension for LoggingExtension {
    fn name(&self) -> &str {
        "logging"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn on_agent_start(&self, ctx: &ExtensionContext) -> Result<(), String> {
        tracing::info!("Agent started: session={}", ctx.session_id);
        Ok(())
    }

    async fn on_agent_end(&self, ctx: &ExtensionContext, result: &AgentResult) -> Result<(), String> {
        tracing::info!(
            "Agent ended: session={}, success={}, rounds={}",
            ctx.session_id,
            result.success,
            result.rounds
        );
        Ok(())
    }

    async fn on_tool_call(
        &self,
        ctx: &ExtensionContext,
        tool_name: &str,
        _arguments: &serde_json::Value,
    ) -> Result<Option<String>, String> {
        tracing::debug!(
            "Tool call: session={}, tool={}, round={}",
            ctx.session_id,
            tool_name,
            ctx.round
        );
        Ok(None)
    }

    async fn on_tool_result(
        &self,
        ctx: &ExtensionContext,
        tool_name: &str,
        result: &str,
    ) -> Result<Option<String>, String> {
        let result_preview = if result.len() > 100 {
            format!("{}...", &result[..100])
        } else {
            result.to_string()
        };
        tracing::debug!(
            "Tool result: session={}, tool={}, result={}",
            ctx.session_id,
            tool_name,
            result_preview
        );
        Ok(None)
    }
}

impl Default for LoggingExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::context::ExtensionContextBuilder;
    use crate::extension::traits::InMemoryExtensionHostActions;
    use std::path::PathBuf;

    #[test]
    fn test_extension_manager_creation() {
        let host = Arc::new(InMemoryExtensionHostActions::new(
            PathBuf::from("/tmp"),
            "test".to_string(),
        ));
        let manager = ExtensionManager::new(host);
        assert!(manager.extensions().is_empty());
    }

    #[test]
    fn test_register_extension() {
        let host = Arc::new(InMemoryExtensionHostActions::new(
            PathBuf::from("/tmp"),
            "test".to_string(),
        ));
        let mut manager = ExtensionManager::new(host);
        manager.register(Box::new(LoggingExtension::new()));
        assert_eq!(manager.extensions().len(), 1);
    }

    #[test]
    fn test_path_allowed() {
        let host = Arc::new(InMemoryExtensionHostActions::new(
            PathBuf::from("/tmp"),
            "test".to_string(),
        ));
        let manager = ExtensionManager::new(host);
        assert!(manager.is_path_allowed(PathBuf::from("/tmp/test").as_path()));
    }

    #[tokio::test]
    async fn test_notify_agent_start() {
        let host = Arc::new(InMemoryExtensionHostActions::new(
            PathBuf::from("/tmp"),
            "test".to_string(),
        ));
        let mut manager = ExtensionManager::new(host);
        manager.register(Box::new(LoggingExtension::new()));

        let mut ctx = ExtensionContextBuilder::new()
            .session_id("test-session".to_string())
            .build();

        manager.notify_agent_start(&mut ctx).await;
        // No assertions needed - just verify it doesn't panic
    }
}
