use eaasp_goose_runtime::acp_parser::AcpEvent;

#[test]
fn test_parse_acp_chunk_event() {
    let raw = r#"{"jsonrpc":"2.0","method":"session/update","params":{"kind":"agent_message_chunk","content":{"type":"text","text":"hello"}}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    assert!(
        matches!(parsed, AcpEvent::Chunk { text: ref t, .. } if t == "hello"),
        "expected Chunk with text=hello, got {parsed:?}"
    );
}

#[test]
fn test_parse_acp_chunk_event_with_session_id() {
    let raw = r#"{"jsonrpc":"2.0","method":"session/update","params":{"kind":"agent_message_chunk","session_id":"sess-1","content":{"type":"text","text":"world"}}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    match parsed {
        AcpEvent::Chunk { text, session_id } => {
            assert_eq!(text, "world");
            assert_eq!(session_id.as_deref(), Some("sess-1"));
        }
        other => panic!("expected Chunk, got {other:?}"),
    }
}

#[test]
fn test_parse_acp_tool_call_event() {
    let raw = r#"{"jsonrpc":"2.0","method":"session/update","params":{"kind":"tool_use","content":{"name":"memory_search","id":"tc-123","input":{"query":"test"}}}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    match parsed {
        AcpEvent::ToolCall { tool_name, tool_id, input_json, .. } => {
            assert_eq!(tool_name, "memory_search");
            assert_eq!(tool_id, "tc-123");
            assert!(input_json.contains("query"), "input_json should contain 'query'");
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
}

#[test]
fn test_parse_acp_tool_result_event_is_unknown() {
    // goose does not emit tool_result over ACP; this variant would appear as Unknown
    let raw = r#"{"jsonrpc":"2.0","method":"session/update","params":{"kind":"tool_result","content":"ok"}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    assert!(
        matches!(parsed, AcpEvent::Unknown { .. }),
        "tool_result kind should be Unknown, got {parsed:?}"
    );
}

#[test]
fn test_parse_acp_finish_event() {
    let raw = r#"{"jsonrpc":"2.0","method":"session/update","params":{"kind":"finish"}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    assert!(
        matches!(parsed, AcpEvent::Stop { ref reason, .. } if reason == "finish"),
        "expected Stop(finish), got {parsed:?}"
    );
}

#[test]
fn test_parse_acp_session_stopped() {
    let raw = r#"{"jsonrpc":"2.0","method":"session/stopped","params":{"reason":"end_of_turn","session_id":"sess-abc"}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    match parsed {
        AcpEvent::Stop { reason, session_id } => {
            assert_eq!(reason, "end_of_turn");
            assert_eq!(session_id.as_deref(), Some("sess-abc"));
        }
        other => panic!("expected Stop, got {other:?}"),
    }
}

#[test]
fn test_parse_acp_error_event() {
    let raw = r#"{"jsonrpc":"2.0","method":"session/error","params":{"message":"provider failed","session_id":"sess-1"}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    match parsed {
        AcpEvent::Error { message, .. } => {
            assert!(message.contains("provider"), "expected error message about provider, got: {message}");
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn test_parse_acp_malformed_fails_gracefully() {
    // Must not panic — returns Err
    let raw = "{not-valid-json";
    let result = AcpEvent::try_from(raw);
    assert!(result.is_err(), "malformed JSON should return Err, not panic");
}

#[test]
fn test_parse_acp_unknown_method_is_unknown() {
    let raw = r#"{"jsonrpc":"2.0","method":"session/ping","params":{}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    assert!(
        matches!(parsed, AcpEvent::Unknown { .. }),
        "unknown method should be Unknown, got {parsed:?}"
    );
}

#[test]
fn test_parse_acp_jsonrpc_error_object() {
    let raw = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
    let parsed = AcpEvent::try_from(raw).unwrap();
    match parsed {
        AcpEvent::Error { message, .. } => {
            assert!(message.contains("Invalid"), "expected 'Invalid' in message, got: {message}");
        }
        other => panic!("expected Error, got {other:?}"),
    }
}
