use octo_engine::context::manager::{EstimateCounter, TokenCounter};
use octo_engine::context::token_counter::count_text_tokens;

#[test]
fn test_estimate_counter_ascii_only() {
    let counter = EstimateCounter;
    // 20 ASCII chars → 20 * 0.25 = 5
    assert_eq!(counter.count("01234567890123456789"), 5);
}

#[test]
fn test_estimate_counter_cjk_only() {
    let counter = EstimateCounter;
    // 3 CJK chars → 3 * 0.67 = 2.01 → ceil = 3
    assert_eq!(counter.count("你好世"), 3);
}

#[test]
fn test_estimate_counter_mixed_content() {
    let counter = EstimateCounter;
    // "Hello世界" = 5 ASCII (1.25) + 2 CJK (1.34) = 2.59 → ceil = 3
    assert_eq!(counter.count("Hello世界"), 3);
}

#[test]
fn test_estimate_counter_empty() {
    let counter = EstimateCounter;
    assert_eq!(counter.count(""), 0);
}

#[test]
fn test_estimate_counter_long_text() {
    let counter = EstimateCounter;
    // 400 ASCII chars → 400 * 0.25 = 100
    let text = "a".repeat(400);
    assert_eq!(counter.count(&text), 100);
}

#[test]
fn test_cjk_aware_counter_is_estimate_counter() {
    let counter = EstimateCounter;
    // Same behavior since EstimateCounter = EstimateCounter
    assert_eq!(counter.count("test"), 1);
}

#[test]
fn test_count_text_tokens_helper() {
    let counter = EstimateCounter;
    let result = count_text_tokens(&counter, "hello world");
    // 11 chars * 0.25 = 2.75 → ceil = 3
    assert_eq!(result, 3);
}

#[test]
fn test_counter_with_punctuation() {
    let counter = EstimateCounter;
    // "fn foo() { }" = 12 ASCII chars → 12 * 0.25 = 3.0 → ceil = 3
    assert_eq!(counter.count("fn foo() { }"), 3);
}

#[test]
fn test_counter_with_newlines() {
    let counter = EstimateCounter;
    // Newlines are ASCII
    let text = "line1\nline2\nline3";
    let expected = (text.len() as f64 * 0.25).ceil() as usize;
    assert_eq!(counter.count(text), expected);
}
