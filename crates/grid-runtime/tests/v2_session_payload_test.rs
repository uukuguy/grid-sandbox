//! v2.0 SessionPayload priority-block tests (P1-P5 trimming contract).
//!
//! These tests assert the trimming invariants spelled out in v2 §8.6:
//! - P1 PolicyContext is NEVER removed.
//! - P2 EventContext is NEVER removed.
//! - P3 → P4 → P5 are trimmed (reverse order) when budget is exceeded.
//! - Proto round-trip preserves all 5 blocks.

use grid_runtime::contract::{
    EventContext, MemoryRef, PolicyContext, SessionPayload, SkillInstructions, UserPreferences,
};
use grid_runtime::proto;

fn priority_payload() -> SessionPayload {
    let mut p = SessionPayload::new();
    p.session_id = "s-v2".into();
    p.user_id = "u-v2".into();
    p.runtime_id = "grid-harness".into();
    p.policy_context = Some(PolicyContext {
        org_unit: "engineering".into(),
        policy_version: "policy-hash-v1".repeat(4),
        ..Default::default()
    });
    p.event_context = Some(EventContext {
        event_id: "evt-1".into(),
        event_type: "deploy".into(),
        severity: "info".into(),
        payload_json: "z".repeat(400),
        ..Default::default()
    });
    p.memory_refs = vec![
        MemoryRef {
            memory_id: "m-low".into(),
            memory_type: "fact".into(),
            relevance_score: 0.1,
            content: "a".repeat(200),
            source_session_id: "s0".into(),
            created_at: "2026-01-01".into(),
            tags: Default::default(),
        },
        MemoryRef {
            memory_id: "m-high".into(),
            memory_type: "fact".into(),
            relevance_score: 0.9,
            content: "b".repeat(200),
            source_session_id: "s0".into(),
            created_at: "2026-01-01".into(),
            tags: Default::default(),
        },
    ];
    p.skill_instructions = Some(SkillInstructions {
        skill_id: "skill-1".into(),
        name: "big-skill".into(),
        content: "c".repeat(500),
        ..Default::default()
    });
    p.user_preferences = Some(UserPreferences {
        user_id: "u-v2".into(),
        language: "en-US".into(),
        timezone: "Asia/Shanghai".into(),
        ..Default::default()
    });
    p
}

#[test]
fn trim_for_budget_removes_p5_first() {
    let mut p = priority_payload();
    let full = p.estimated_tokens();
    assert!(full > 0);

    // Budget just a hair under the full size so only P5 gets dropped.
    p.trim_for_budget(full - 2);

    assert!(
        p.user_preferences.is_none(),
        "P5 must be trimmed before P4/P3"
    );
    assert!(p.policy_context.is_some(), "P1 must remain");
    assert!(p.event_context.is_some(), "P2 must remain");
    assert!(!p.memory_refs.is_empty(), "P3 must remain");
    assert!(p.skill_instructions.is_some(), "P4 must remain");
}

#[test]
fn trim_for_budget_never_removes_p1() {
    let mut p = priority_payload();
    p.allow_trim_p5 = true;
    p.allow_trim_p4 = true;
    p.allow_trim_p3 = true;

    // An absurdly small budget should drop everything trimmable but
    // leave P1 PolicyContext and P2 EventContext intact.
    p.trim_for_budget(0);

    assert!(p.policy_context.is_some(), "P1 must survive any budget");
    assert!(p.event_context.is_some(), "P2 must survive any budget");
    assert!(p.user_preferences.is_none());
    assert!(p.skill_instructions.is_none());
    assert!(p.memory_refs.is_empty());
}

#[test]
fn trim_order_is_p5_then_p4_then_p3() {
    // Step 1: a budget that leaves everything except P5 should drop
    // only P5. We compute "full - (P5 estimate) + slack".
    let mut p = priority_payload();
    p.allow_trim_p5 = true;
    p.allow_trim_p4 = true;
    p.allow_trim_p3 = true;

    let full = p.estimated_tokens();
    let mut clone = p.clone();
    clone.user_preferences = None;
    let without_p5 = clone.estimated_tokens();
    assert!(full > without_p5);

    // Budget halfway between full and without_p5 forces P5 drop but
    // leaves room for P4 and P3.
    let step1_budget = (full + without_p5) / 2;
    p.trim_for_budget(step1_budget);
    assert!(p.user_preferences.is_none(), "P5 must be first to go");
    assert!(
        p.skill_instructions.is_some(),
        "P4 should survive when only P5 removal suffices"
    );
    assert_eq!(p.memory_refs.len(), 2, "P3 should survive step 1");

    // Step 2: a budget that leaves only P1+P2 forces everything
    // trimmable to be removed.
    p.trim_for_budget(0);
    assert!(p.skill_instructions.is_none(), "P4 must be trimmed after P5");
    assert!(p.memory_refs.is_empty(), "P3 must be trimmed after P4");
    assert!(p.policy_context.is_some(), "P1 must survive");
    assert!(p.event_context.is_some(), "P2 must survive");
}

#[test]
fn trim_respects_allow_flags() {
    let mut p = priority_payload();
    p.allow_trim_p5 = false;
    p.allow_trim_p4 = false;
    p.allow_trim_p3 = false;
    p.trim_for_budget(0);
    // Nothing should be dropped because no block is flagged trimmable.
    assert!(p.user_preferences.is_some());
    assert!(p.skill_instructions.is_some());
    assert_eq!(p.memory_refs.len(), 2);
}

#[test]
fn session_payload_proto_roundtrip() {
    let p = priority_payload();
    let proto_p: proto::SessionPayload = p.clone().into();
    let back: SessionPayload = proto_p.into();

    assert_eq!(back.session_id, p.session_id);
    assert_eq!(back.user_id, p.user_id);
    assert_eq!(back.runtime_id, p.runtime_id);
    assert_eq!(back.allow_trim_p5, p.allow_trim_p5);

    assert_eq!(
        back.policy_context.as_ref().map(|c| c.org_unit.clone()),
        p.policy_context.as_ref().map(|c| c.org_unit.clone())
    );
    assert_eq!(
        back.event_context.as_ref().map(|c| c.event_type.clone()),
        p.event_context.as_ref().map(|c| c.event_type.clone())
    );
    assert_eq!(back.memory_refs.len(), p.memory_refs.len());
    assert_eq!(
        back.skill_instructions.as_ref().map(|s| s.skill_id.clone()),
        p.skill_instructions.as_ref().map(|s| s.skill_id.clone())
    );
    assert_eq!(
        back.user_preferences
            .as_ref()
            .map(|u| u.language.clone()),
        p.user_preferences.as_ref().map(|u| u.language.clone())
    );
}

#[test]
fn deny_scope_never_removes_p2_event_context() {
    // Regression guard: even a session with only P2 event data as
    // meaningful payload must keep P2 after trimming.
    let mut p = SessionPayload::new();
    p.event_context = Some(EventContext {
        event_id: "critical-1".into(),
        event_type: "outage".into(),
        severity: "critical".into(),
        payload_json: "x".repeat(5000),
        ..Default::default()
    });
    p.allow_trim_p5 = true;
    p.allow_trim_p4 = true;
    p.allow_trim_p3 = true;

    p.trim_for_budget(0);
    assert!(p.event_context.is_some());
    assert_eq!(
        p.event_context.as_ref().unwrap().severity,
        "critical"
    );
}
