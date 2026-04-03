//! EvalMockTool — wraps a ToolSpec into an executable mock tool for evaluation.
//!
//! Used by the eval runner to inject task-declared tools (e.g. τ-bench business tools)
//! into the ToolRegistry so the agent can actually call them during evaluation.
//! The mock tool returns a plausible JSON response without a real backend.

use anyhow::Result;
use async_trait::async_trait;
use grid_types::{ApprovalRequirement, RiskLevel, ToolContext, ToolOutput, ToolSource, ToolSpec};

use grid_engine::tools::traits::Tool;

/// A mock tool that wraps any ToolSpec and returns a realistic stub response.
/// Enables evaluation of tasks that declare domain-specific tools (e.g. τ-bench retail tools).
pub struct EvalMockTool {
    spec: ToolSpec,
}

impl EvalMockTool {
    pub fn new(spec: ToolSpec) -> Self {
        Self { spec }
    }

    /// Build a realistic mock response based on tool name and params.
    fn mock_response(tool_name: &str, params: &serde_json::Value) -> String {
        match tool_name {
            "lookup_order" => {
                let order_id = params
                    .get("order_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN");
                serde_json::json!({
                    "order_id": order_id,
                    "status": "delivered",
                    "purchase_date": "2025-02-15",
                    "items": [{"name": "product", "quantity": 1, "price": 89.99}],
                    "total": 89.99,
                    "payment_method": "credit_card"
                })
                .to_string()
            }
            "check_return_eligibility" => {
                let order_id = params
                    .get("order_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN");
                serde_json::json!({
                    "order_id": order_id,
                    "eligible": true,
                    "reason": "within_return_window",
                    "days_since_purchase": 7,
                    "policy": "30_day_return"
                })
                .to_string()
            }
            "process_return" => {
                let order_id = params
                    .get("order_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN");
                let refund_type = params
                    .get("refund_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("full");
                serde_json::json!({
                    "success": true,
                    "return_id": format!("RET-{}", order_id),
                    "refund_type": refund_type,
                    "refund_amount": 89.99,
                    "status": "return_initiated",
                    "estimated_processing_days": 3
                })
                .to_string()
            }
            "send_confirmation" => {
                let order_id = params
                    .get("order_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN");
                serde_json::json!({
                    "success": true,
                    "order_id": order_id,
                    "channel": params.get("channel").and_then(|v| v.as_str()).unwrap_or("email"),
                    "message": "Confirmation sent successfully"
                })
                .to_string()
            }
            "update_inventory" => {
                serde_json::json!({
                    "success": true,
                    "action": params.get("action").and_then(|v| v.as_str()).unwrap_or("update"),
                    "message": "Inventory updated successfully"
                })
                .to_string()
            }
            _ => {
                // Generic mock response for any unknown tool
                serde_json::json!({
                    "success": true,
                    "tool": tool_name,
                    "result": "mock_executed",
                    "params_received": params
                })
                .to_string()
            }
        }
    }
}

#[async_trait]
impl Tool for EvalMockTool {
    fn name(&self) -> &str {
        &self.spec.name
    }

    fn description(&self) -> &str {
        &self.spec.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.spec.input_schema.clone()
    }

    async fn execute(&self, params: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let response = Self::mock_response(&self.spec.name, &params);
        Ok(ToolOutput::success(response))
    }

    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::ReadOnly
    }

    fn approval(&self) -> ApprovalRequirement {
        ApprovalRequirement::Never
    }

    fn category(&self) -> &str {
        "eval_mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use grid_types::SandboxId;

    fn make_spec(name: &str) -> ToolSpec {
        ToolSpec {
            name: name.to_string(),
            description: format!("Mock {} tool", name),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        }
    }

    fn test_ctx() -> ToolContext {
        ToolContext {
            sandbox_id: SandboxId::new(),
            user_id: grid_types::UserId::from_string(grid_types::id::DEFAULT_USER_ID),
            working_dir: PathBuf::from("/tmp"),
            path_validator: None,
        }
    }

    #[tokio::test]
    async fn test_mock_tool_lookup_order() {
        let tool = EvalMockTool::new(make_spec("lookup_order"));
        assert_eq!(tool.name(), "lookup_order");
        let ctx = test_ctx();
        let result = tool
            .execute(serde_json::json!({"order_id": "ORD-123"}), &ctx)
            .await
            .unwrap();
        assert!(!result.is_error);
        let json: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(json["order_id"], "ORD-123");
        assert_eq!(json["status"], "delivered");
    }

    #[tokio::test]
    async fn test_mock_tool_process_return() {
        let tool = EvalMockTool::new(make_spec("process_return"));
        let ctx = test_ctx();
        let result = tool
            .execute(
                serde_json::json!({"order_id": "ORD-456", "refund_type": "full"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let json: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["refund_type"], "full");
    }

    #[tokio::test]
    async fn test_mock_tool_generic() {
        let tool = EvalMockTool::new(make_spec("custom_business_tool"));
        let ctx = test_ctx();
        let result = tool
            .execute(serde_json::json!({"key": "value"}), &ctx)
            .await
            .unwrap();
        assert!(!result.is_error);
        let json: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["tool"], "custom_business_tool");
    }

    #[test]
    fn test_mock_tool_spec() {
        let spec = make_spec("lookup_order");
        let tool = EvalMockTool::new(spec);
        assert_eq!(tool.name(), "lookup_order");
        assert_eq!(tool.category(), "eval_mock");
        assert!(matches!(tool.risk_level(), RiskLevel::ReadOnly));
    }
}
