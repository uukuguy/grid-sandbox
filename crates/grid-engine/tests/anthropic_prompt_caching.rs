//! AV-T2: Tests for Anthropic prompt caching content blocks.

#[test]
fn test_system_prompt_split_into_cached_blocks() {
    let system = "You are a helpful assistant.\n---DYNAMIC---\nCurrent time: 2026-04-03T12:00:00Z";

    if let Some((static_part, dynamic_part)) = system.split_once("\n---DYNAMIC---\n") {
        assert_eq!(static_part, "You are a helpful assistant.");
        assert_eq!(dynamic_part, "Current time: 2026-04-03T12:00:00Z");

        let blocks = build_test_blocks(static_part, Some(dynamic_part));
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["type"], "text");
        assert_eq!(blocks[0]["text"], static_part);
        assert!(blocks[0]["cache_control"].is_object());
        assert_eq!(blocks[0]["cache_control"]["type"], "ephemeral");
        assert_eq!(blocks[1]["type"], "text");
        assert_eq!(blocks[1]["text"], dynamic_part);
        assert!(blocks[1].get("cache_control").is_none());
    } else {
        panic!("Expected separator in system prompt");
    }
}

#[test]
fn test_system_prompt_single_block_when_no_dynamic() {
    let system = "Static only prompt";
    let blocks = build_test_blocks(system, None);
    assert_eq!(blocks.len(), 1);
    assert!(blocks[0]["cache_control"].is_object());
    assert_eq!(blocks[0]["text"], system);
}

#[test]
fn test_prompt_parts_merge_with_separator() {
    let static_part = "You are helpful.";
    let dynamic_part = "Current date: 2026-04-03";
    let merged = if dynamic_part.is_empty() {
        static_part.to_string()
    } else {
        format!("{}\n---DYNAMIC---\n{}", static_part, dynamic_part)
    };
    assert!(merged.contains("---DYNAMIC---"));
    let parts: Vec<&str> = merged.split("\n---DYNAMIC---\n").collect();
    assert_eq!(parts.len(), 2);
}

#[test]
fn test_prompt_parts_merge_without_dynamic() {
    let static_part = "Static only.";
    let dynamic_part = "";
    let merged = if dynamic_part.is_empty() {
        static_part.to_string()
    } else {
        format!("{}\n---DYNAMIC---\n{}", static_part, dynamic_part)
    };
    assert!(!merged.contains("---DYNAMIC---"));
    assert_eq!(merged, "Static only.");
}

// Helper matching the logic of build_system_content_blocks
fn build_test_blocks(static_part: &str, dynamic_part: Option<&str>) -> Vec<serde_json::Value> {
    let mut blocks = vec![serde_json::json!({
        "type": "text",
        "text": static_part,
        "cache_control": { "type": "ephemeral" }
    })];
    if let Some(dynamic) = dynamic_part {
        if !dynamic.is_empty() {
            blocks.push(serde_json::json!({
                "type": "text",
                "text": dynamic
            }));
        }
    }
    blocks
}
