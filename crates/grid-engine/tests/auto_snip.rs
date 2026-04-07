//! AV-T3: Tests for auto_snip context compaction.

use grid_engine::context::CompactionPipeline;
use grid_types::message::{ChatMessage, ContentBlock, MessageRole};

#[test]
fn test_auto_snip_preserves_recent_messages() {
    let mut messages: Vec<ChatMessage> = (0..25)
        .map(|i| ChatMessage {
            role: if i % 2 == 0 {
                MessageRole::User
            } else {
                MessageRole::Assistant
            },
            content: vec![ContentBlock::Text {
                text: format!("Message {i}"),
            }],
        })
        .collect();

    let removed = CompactionPipeline::auto_snip(&mut messages, 10);
    assert_eq!(removed, 15); // 25 - 10
    // +1 for the summary marker inserted at position 0
    assert_eq!(messages.len(), 11); // 10 kept + 1 marker
    // Marker at position 0
    let marker_text = messages[0].text_content();
    assert!(marker_text.contains("Auto-snip"));
    assert!(marker_text.contains("15"));
    // First real message is "Message 15"
    let first_real = messages[1].text_content();
    assert_eq!(first_real, "Message 15");
}

#[test]
fn test_auto_snip_no_action_when_few_messages() {
    let mut messages: Vec<ChatMessage> = (0..5)
        .map(|i| ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: format!("Msg {i}"),
            }],
        })
        .collect();

    let removed = CompactionPipeline::auto_snip(&mut messages, 10);
    assert_eq!(removed, 0);
    assert_eq!(messages.len(), 5); // unchanged
}

#[test]
fn test_auto_snip_boundary_at_min_messages() {
    let mut messages: Vec<ChatMessage> = (0..8)
        .map(|i| ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: format!("Msg {i}"),
            }],
        })
        .collect();

    // 8 messages, keep 6 → boundary = 2, but len <= min_messages (8), so no action
    let removed = CompactionPipeline::auto_snip(&mut messages, 6);
    assert_eq!(removed, 0);
    assert_eq!(messages.len(), 8);
}

#[test]
fn test_auto_snip_just_above_min() {
    let mut messages: Vec<ChatMessage> = (0..9)
        .map(|i| ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: format!("Msg {i}"),
            }],
        })
        .collect();

    // 9 messages, keep 6 → boundary = 3 (>= 2), len > 8
    let removed = CompactionPipeline::auto_snip(&mut messages, 6);
    assert_eq!(removed, 3);
    assert_eq!(messages.len(), 7); // 6 kept + 1 marker
}

#[test]
fn test_auto_snip_keep_all_when_keep_exceeds_total() {
    let mut messages: Vec<ChatMessage> = (0..10)
        .map(|i| ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: format!("Msg {i}"),
            }],
        })
        .collect();

    let removed = CompactionPipeline::auto_snip(&mut messages, 20);
    assert_eq!(removed, 0);
    assert_eq!(messages.len(), 10);
}
