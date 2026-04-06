//! Blindbox comparison — run same prompt against two runtimes anonymously.
//!
//! Results are labeled "A" / "B" with the actual runtime IDs hidden
//! until the user votes. This enables unbiased quality comparison.

use std::time::Instant;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::runtime_pool::RuntimeEntry;
use crate::runtime_proto;
use crate::runtime_proto::runtime_service_client::RuntimeServiceClient;

/// A single runtime's execution result (anonymized).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindboxResult {
    /// Anonymous label: "A" or "B"
    pub label: String,
    /// Collected response text
    pub response_text: String,
    /// Execution time in milliseconds
    pub duration_ms: u64,
    /// Hidden: actual runtime ID (revealed after voting)
    #[serde(skip_serializing)]
    pub runtime_id: String,
}

/// User vote for blindbox comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlindboxVote {
    AWins,
    BWins,
    Tie,
}

/// Complete blindbox comparison record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindboxRecord {
    pub prompt: String,
    pub result_a: BlindboxResult,
    pub result_b: BlindboxResult,
    pub vote: Option<BlindboxVote>,
    pub revealed: bool,
}

impl BlindboxRecord {
    /// Reveal which runtime produced which result.
    pub fn reveal(&mut self) -> (String, String) {
        self.revealed = true;
        (
            format!("A = {}", self.result_a.runtime_id),
            format!("B = {}", self.result_b.runtime_id),
        )
    }
}

/// Execute the same prompt on two runtimes in parallel, collect anonymized responses.
pub async fn execute_blindbox(
    runtimes: &[RuntimeEntry; 2],
    prompt: &str,
) -> Result<BlindboxRecord> {
    info!(
        runtime_a = %runtimes[0].id,
        runtime_b = %runtimes[1].id,
        "Starting blindbox comparison"
    );

    // Randomize assignment to labels A/B
    let swap = rand_bool();
    let (first, second) = if swap {
        (&runtimes[1], &runtimes[0])
    } else {
        (&runtimes[0], &runtimes[1])
    };

    // Execute in parallel
    let (result_a, result_b) = tokio::join!(
        execute_single(first, prompt, "A"),
        execute_single(second, prompt, "B"),
    );

    Ok(BlindboxRecord {
        prompt: prompt.to_string(),
        result_a: result_a?,
        result_b: result_b?,
        vote: None,
        revealed: false,
    })
}

async fn execute_single(
    runtime: &RuntimeEntry,
    prompt: &str,
    label: &str,
) -> Result<BlindboxResult> {
    let start = Instant::now();

    let mut client = RuntimeServiceClient::connect(runtime.endpoint.clone()).await?;

    // Initialize session
    let init_resp = client
        .initialize(tonic::Request::new(runtime_proto::InitializeRequest {
            payload: Some(runtime_proto::SessionPayload {
                user_id: "blindbox-user".into(),
                user_role: "tester".into(),
                org_unit: "qa".into(),
                ..Default::default()
            }),
        }))
        .await?;

    let session_id = init_resp.into_inner().session_id;

    // Send prompt
    let mut stream = client
        .send(tonic::Request::new(runtime_proto::SendRequest {
            session_id: session_id.clone(),
            message: Some(runtime_proto::UserMessage {
                content: prompt.into(),
                message_type: "text".into(),
                metadata: Default::default(),
            }),
        }))
        .await?
        .into_inner();

    // Collect response text
    let mut text = String::new();
    while let Some(chunk) = stream.message().await? {
        if chunk.chunk_type == "text_delta" {
            text.push_str(&chunk.content);
        }
    }

    // Terminate session
    let _ = client
        .terminate(tonic::Request::new(runtime_proto::TerminateRequest {
            session_id,
        }))
        .await;

    let duration = start.elapsed().as_millis() as u64;

    Ok(BlindboxResult {
        label: label.into(),
        response_text: text,
        duration_ms: duration,
        runtime_id: runtime.id.clone(),
    })
}

/// Simple deterministic-ish bool from system time nanoseconds.
fn rand_bool() -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        % 2
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blindbox_record_reveal() {
        let mut record = BlindboxRecord {
            prompt: "test prompt".into(),
            result_a: BlindboxResult {
                label: "A".into(),
                response_text: "hello from grid".into(),
                duration_ms: 100,
                runtime_id: "grid-harness".into(),
            },
            result_b: BlindboxResult {
                label: "B".into(),
                response_text: "hello from cc".into(),
                duration_ms: 200,
                runtime_id: "claude-code".into(),
            },
            vote: None,
            revealed: false,
        };

        assert!(!record.revealed);
        let (a_reveal, b_reveal) = record.reveal();
        assert!(record.revealed);
        assert_eq!(a_reveal, "A = grid-harness");
        assert_eq!(b_reveal, "B = claude-code");
    }

    #[test]
    fn blindbox_vote_serialization() {
        let vote_a = BlindboxVote::AWins;
        let vote_b = BlindboxVote::BWins;
        let vote_tie = BlindboxVote::Tie;

        let json_a = serde_json::to_string(&vote_a).unwrap();
        let json_b = serde_json::to_string(&vote_b).unwrap();
        let json_tie = serde_json::to_string(&vote_tie).unwrap();

        assert!(json_a.contains("AWins"));
        assert!(json_b.contains("BWins"));
        assert!(json_tie.contains("Tie"));

        // Round-trip
        let restored: BlindboxVote = serde_json::from_str(&json_a).unwrap();
        assert!(matches!(restored, BlindboxVote::AWins));
    }

    #[test]
    fn blindbox_result_skip_runtime_id_in_serialization() {
        let result = BlindboxResult {
            label: "A".into(),
            response_text: "test output".into(),
            duration_ms: 150,
            runtime_id: "secret-runtime".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        // runtime_id should be skipped in serialization (serde(skip_serializing))
        assert!(!json.contains("secret-runtime"));
        assert!(json.contains("test output"));
        assert!(json.contains("150"));
    }
}
