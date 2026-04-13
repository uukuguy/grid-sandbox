//! L2 Memory Engine HTTP client.
//!
//! Provides async methods to write evidence anchors and memory files
//! to the EAASP L2 Memory Engine via its REST API (MCP tool facade).

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// L2 Memory Engine client.
#[derive(Clone)]
pub struct L2MemoryClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize)]
pub struct WriteAnchorRequest {
    pub event_id: String,
    pub session_id: String,
    #[serde(rename = "type")]
    pub anchor_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_system: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WriteFileRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<String>,
    pub scope: String,
    pub category: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// L2 returns AnchorOut with many fields; we only care about anchor_id.
/// anchor_id is a string in L2 (not i64).
#[derive(Debug, Deserialize)]
pub struct WriteAnchorResponse {
    pub anchor_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WriteFileResponse {
    pub memory_id: Option<String>,
    pub version: Option<i64>,
}

impl L2MemoryClient {
    pub fn new(base_url: &str) -> Self {
        // IMPORTANT: no_proxy() to avoid macOS proxy issues (MEMORY.md known issue:
        // Clash proxy turns localhost calls into 502). Learned from S4.T2 debugging.
        let client = Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Create from environment variables EAASP_L2_HOST / EAASP_L2_PORT.
    pub fn from_env() -> Self {
        let port = std::env::var("EAASP_L2_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(18085);
        let host = std::env::var("EAASP_L2_HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        Self::new(&format!("http://{}:{}", host, port))
    }

    /// Write an evidence anchor to L2 via the MCP tool facade.
    pub async fn write_anchor(
        &self,
        req: &WriteAnchorRequest,
    ) -> anyhow::Result<WriteAnchorResponse> {
        let url = format!("{}/tools/memory_write_anchor/invoke", self.base_url);
        // L2 MCP facade expects {"args": {<fields>}}
        let body = serde_json::json!({
            "args": {
                "event_id": req.event_id,
                "session_id": req.session_id,
                "type": req.anchor_type,
                "data_ref": req.data_ref,
                "snapshot_hash": req.snapshot_hash,
                "source_system": req.source_system,
            }
        });

        debug!(url = %url, "Writing evidence anchor to L2");

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("L2 anchor write failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "L2 anchor write HTTP {}: {}",
                status,
                text
            ));
        }

        resp.json()
            .await
            .map_err(|e| anyhow::anyhow!("L2 anchor response parse error: {}", e))
    }

    /// Write a memory file to L2 via the MCP tool facade.
    pub async fn write_file(
        &self,
        req: &WriteFileRequest,
    ) -> anyhow::Result<WriteFileResponse> {
        let url = format!("{}/tools/memory_write_file/invoke", self.base_url);
        let body = serde_json::json!({
            "args": {
                "memory_id": req.memory_id,
                "scope": req.scope,
                "category": req.category,
                "content": req.content,
                "evidence_refs": req.evidence_refs,
                "status": req.status,
            }
        });

        debug!(url = %url, "Writing memory file to L2");

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("L2 file write failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "L2 file write HTTP {}: {}",
                status,
                text
            ));
        }

        resp.json()
            .await
            .map_err(|e| anyhow::anyhow!("L2 file response parse error: {}", e))
    }

    /// Health check against L2 /health endpoint.
    pub async fn health(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Return the configured base URL (for diagnostics).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_default_port() {
        // When env vars are not set, defaults to 127.0.0.1:18085
        std::env::remove_var("EAASP_L2_PORT");
        std::env::remove_var("EAASP_L2_HOST");
        let client = L2MemoryClient::from_env();
        assert_eq!(client.base_url, "http://127.0.0.1:18085");
    }

    #[test]
    fn new_trims_trailing_slash() {
        let client = L2MemoryClient::new("http://localhost:18085/");
        assert_eq!(client.base_url, "http://localhost:18085");
    }

    #[test]
    fn new_no_trailing_slash() {
        let client = L2MemoryClient::new("http://localhost:18085");
        assert_eq!(client.base_url, "http://localhost:18085");
    }

    #[test]
    fn write_anchor_request_serialization() {
        let req = WriteAnchorRequest {
            event_id: "evt-1".into(),
            session_id: "sess-1".into(),
            anchor_type: "tool_execution".into(),
            data_ref: Some("output data".into()),
            snapshot_hash: None,
            source_system: Some("grid-runtime".into()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["event_id"], "evt-1");
        assert_eq!(json["type"], "tool_execution");
        assert_eq!(json["data_ref"], "output data");
        // snapshot_hash is None => skipped
        assert!(json.get("snapshot_hash").is_none());
        assert_eq!(json["source_system"], "grid-runtime");
    }

    #[test]
    fn write_file_request_serialization() {
        let req = WriteFileRequest {
            memory_id: None,
            scope: "session:s1".into(),
            category: "tool_evidence".into(),
            content: "tool output here".into(),
            evidence_refs: Some(vec!["anchor-1".into()]),
            status: Some("agent_suggested".into()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["scope"], "session:s1");
        assert_eq!(json["category"], "tool_evidence");
        // memory_id is None => skipped
        assert!(json.get("memory_id").is_none());
        assert_eq!(json["evidence_refs"][0], "anchor-1");
    }

    #[test]
    fn write_anchor_response_deserialize() {
        // L2 returns anchor_id as string + many extra fields; we ignore extras.
        let json = r#"{"anchor_id": "anc-42", "event_id": "e1", "session_id": "s1", "type": "tool_execution", "created_at": 123}"#;
        let resp: WriteAnchorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.anchor_id.as_deref(), Some("anc-42"));
    }

    /// 断点 7: L2 AnchorOut has many fields that WriteAnchorResponse must
    /// tolerate (serde deny_unknown_fields would break this).
    #[test]
    fn write_anchor_response_deserialize_full_l2_format() {
        let json = r#"{
            "anchor_id": "anc-123",
            "event_id": "tool-bash-1234",
            "session_id": "sess-abc",
            "type": "tool_execution",
            "data_ref": "output data",
            "snapshot_hash": null,
            "source_system": "grid-runtime",
            "tool_version": null,
            "model_version": null,
            "rule_version": null,
            "created_at": 1776000000,
            "metadata": {}
        }"#;
        let resp: WriteAnchorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.anchor_id.as_deref(), Some("anc-123"));
    }

    /// L2 anchor_id could be null in edge cases; WriteAnchorResponse should
    /// deserialize without panicking.
    #[test]
    fn write_anchor_response_deserialize_null_anchor_id() {
        let json = r#"{"anchor_id": null, "event_id": "e1"}"#;
        let resp: WriteAnchorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.anchor_id, None);
    }

    #[test]
    fn write_file_response_deserialize() {
        let json = r#"{"memory_id": "mem-abc", "version": 3}"#;
        let resp: WriteFileResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.memory_id.as_deref(), Some("mem-abc"));
        assert_eq!(resp.version, Some(3));
    }
}
