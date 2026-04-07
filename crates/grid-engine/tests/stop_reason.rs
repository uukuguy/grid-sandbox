use grid_engine::NormalizedStopReason;
use grid_types::StopReason;

#[test]
fn test_end_turn_normalizes() {
    let normalized: NormalizedStopReason = StopReason::EndTurn.into();
    assert_eq!(normalized, NormalizedStopReason::EndTurn);
}

#[test]
fn test_tool_use_normalizes_to_tool_call() {
    let normalized: NormalizedStopReason = StopReason::ToolUse.into();
    assert_eq!(normalized, NormalizedStopReason::ToolCall);
}

#[test]
fn test_max_tokens_normalizes() {
    let normalized: NormalizedStopReason = StopReason::MaxTokens.into();
    assert_eq!(normalized, NormalizedStopReason::MaxTokens);
}

#[test]
fn test_stop_sequence_normalizes_to_end_turn() {
    let normalized: NormalizedStopReason = StopReason::StopSequence.into();
    assert_eq!(normalized, NormalizedStopReason::EndTurn);
}

#[test]
fn test_none_normalizes_to_end_turn() {
    let normalized: NormalizedStopReason = None::<StopReason>.into();
    assert_eq!(normalized, NormalizedStopReason::EndTurn);
}

#[test]
fn test_some_normalizes() {
    let normalized: NormalizedStopReason = Some(StopReason::ToolUse).into();
    assert_eq!(normalized, NormalizedStopReason::ToolCall);
}

#[test]
fn test_from_str_lossy_end_turn() {
    assert_eq!(
        NormalizedStopReason::from_str_lossy("end_turn"),
        NormalizedStopReason::EndTurn
    );
    assert_eq!(
        NormalizedStopReason::from_str_lossy("stop"),
        NormalizedStopReason::EndTurn
    );
}

#[test]
fn test_from_str_lossy_tool_call() {
    assert_eq!(
        NormalizedStopReason::from_str_lossy("tool_use"),
        NormalizedStopReason::ToolCall
    );
    assert_eq!(
        NormalizedStopReason::from_str_lossy("tool_calls"),
        NormalizedStopReason::ToolCall
    );
}

#[test]
fn test_from_str_lossy_max_tokens() {
    assert_eq!(
        NormalizedStopReason::from_str_lossy("max_tokens"),
        NormalizedStopReason::MaxTokens
    );
    assert_eq!(
        NormalizedStopReason::from_str_lossy("length"),
        NormalizedStopReason::MaxTokens
    );
}

#[test]
fn test_from_str_lossy_unknown_defaults_end_turn() {
    assert_eq!(
        NormalizedStopReason::from_str_lossy("something_else"),
        NormalizedStopReason::EndTurn
    );
}

#[test]
fn test_is_terminal() {
    assert!(NormalizedStopReason::EndTurn.is_terminal());
    assert!(NormalizedStopReason::MaxIterations.is_terminal());
    assert!(NormalizedStopReason::ContextOverflow.is_terminal());
    assert!(NormalizedStopReason::SafetyBlocked.is_terminal());
    assert!(NormalizedStopReason::Cancelled.is_terminal());
    assert!(NormalizedStopReason::Error.is_terminal());
    // Non-terminal:
    assert!(!NormalizedStopReason::ToolCall.is_terminal());
    assert!(!NormalizedStopReason::MaxTokens.is_terminal());
}
