//! Integration tests for Dual Agent Mode (D5).
//!
//! Tests cover AgentSlot, ToolFilterMode, DualAgentProfile, PlanStep,
//! DualAgentManager switching, plan step parsing, context generation,
//! and serde round-trips.

use grid_engine::{
    AgentEvent, AgentExecutorHandle, AgentSlot, DualAgentManager, DualAgentProfile, PlanStep,
    ToolFilterMode,
};
use grid_types::SessionId;
use tokio::sync::{broadcast, mpsc};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a pair of test handles (plan, build) with distinguishable session IDs.
fn make_test_handles() -> (AgentExecutorHandle, AgentExecutorHandle) {
    let (tx1, _rx1) = mpsc::channel(1);
    let (btx1, _) = broadcast::channel::<AgentEvent>(1);
    let plan = AgentExecutorHandle {
        tx: tx1,
        broadcast_tx: btx1,
        session_id: SessionId::from_string("plan-session"),
    };

    let (tx2, _rx2) = mpsc::channel(1);
    let (btx2, _) = broadcast::channel::<AgentEvent>(1);
    let build = AgentExecutorHandle {
        tx: tx2,
        broadcast_tx: btx2,
        session_id: SessionId::from_string("build-session"),
    };

    (plan, build)
}

/// Shorthand for creating a DualAgentManager with default test handles.
fn make_manager() -> DualAgentManager {
    let (plan, build) = make_test_handles();
    DualAgentManager::new(plan, build, SessionId::from_string("shared-session"))
}

// ---------------------------------------------------------------------------
// 1. Agent Slot and Profile Integration
// ---------------------------------------------------------------------------

#[test]
fn test_dual_agent_profiles_plan_vs_build() {
    let plan_profile = DualAgentProfile {
        name: "Planner".into(),
        system_prompt: "You are a planning agent. Reason step by step.".into(),
        tool_filter: ToolFilterMode::None,
        slot: AgentSlot::Plan,
    };

    let build_profile = DualAgentProfile {
        name: "Builder".into(),
        system_prompt: "You are a build agent. Execute the plan.".into(),
        tool_filter: ToolFilterMode::All,
        slot: AgentSlot::Build,
    };

    // Plan profile must NOT allow tools
    assert!(!plan_profile.tool_filter.allows_tools());
    assert_eq!(plan_profile.slot, AgentSlot::Plan);

    // Build profile must allow tools
    assert!(build_profile.tool_filter.allows_tools());
    assert_eq!(build_profile.slot, AgentSlot::Build);

    // Slots are different
    assert_ne!(plan_profile.slot, build_profile.slot);
}

// ---------------------------------------------------------------------------
// 2. DualAgentManager Switching
// ---------------------------------------------------------------------------

#[test]
fn test_dual_manager_switching() {
    let mut mgr = make_manager();

    // Default is Build
    assert_eq!(mgr.active_slot(), AgentSlot::Build);
    assert_eq!(mgr.active_handle().session_id.as_str(), "build-session");

    // Toggle to Plan
    let slot = mgr.switch();
    assert_eq!(slot, AgentSlot::Plan);
    assert_eq!(mgr.active_slot(), AgentSlot::Plan);
    assert_eq!(mgr.active_handle().session_id.as_str(), "plan-session");

    // Toggle back to Build
    let slot = mgr.switch();
    assert_eq!(slot, AgentSlot::Build);
    assert_eq!(mgr.active_slot(), AgentSlot::Build);

    // switch_to specific slot (idempotent)
    mgr.switch_to(AgentSlot::Plan);
    assert_eq!(mgr.active_slot(), AgentSlot::Plan);
    mgr.switch_to(AgentSlot::Plan); // no-op
    assert_eq!(mgr.active_slot(), AgentSlot::Plan);
}

// ---------------------------------------------------------------------------
// 3. Plan Step Parsing — Numbered List
// ---------------------------------------------------------------------------

#[test]
fn test_plan_step_parsing_numbered_list() {
    let text = "1. Analyze the codebase\n2. Identify key modules\n3. Create implementation plan";
    let steps = DualAgentManager::parse_plan_steps(text);

    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].number, 1);
    assert_eq!(steps[0].description, "Analyze the codebase");
    assert_eq!(steps[1].number, 2);
    assert_eq!(steps[1].description, "Identify key modules");
    assert_eq!(steps[2].number, 3);
    assert_eq!(steps[2].description, "Create implementation plan");

    // All steps start as not completed
    for step in &steps {
        assert!(!step.completed);
    }
}

// ---------------------------------------------------------------------------
// 4. Plan Step Parsing — Bullets
// ---------------------------------------------------------------------------

#[test]
fn test_plan_step_parsing_bullets() {
    let text = "- First step\n- Second step\n* Third step";
    let steps = DualAgentManager::parse_plan_steps(text);

    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].description, "First step");
    assert_eq!(steps[1].description, "Second step");
    assert_eq!(steps[2].description, "Third step");

    // Numbers are auto-assigned sequentially
    assert_eq!(steps[0].number, 1);
    assert_eq!(steps[1].number, 2);
    assert_eq!(steps[2].number, 3);
}

// ---------------------------------------------------------------------------
// 5. Plan Step Parsing — Mixed Content (ignores non-list text)
// ---------------------------------------------------------------------------

#[test]
fn test_plan_step_parsing_ignores_non_list_text() {
    let text =
        "Here's my plan:\n\n1. First step\nSome explanation\n2. Second step\n\nConclusion.";
    let steps = DualAgentManager::parse_plan_steps(text);

    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].description, "First step");
    assert_eq!(steps[1].description, "Second step");
}

// ---------------------------------------------------------------------------
// 6. Context String Generation
// ---------------------------------------------------------------------------

#[test]
fn test_plan_context_string_format() {
    let mut mgr = make_manager();

    mgr.add_plan_step(PlanStep {
        number: 1,
        description: "Read the source files".into(),
        completed: false,
    });
    mgr.add_plan_step(PlanStep {
        number: 2,
        description: "Write the tests".into(),
        completed: false,
    });

    let ctx = mgr.plan_context_string();
    assert!(ctx.starts_with("## Plan Steps\n"));
    assert!(ctx.contains("- [ ] Step 1: Read the source files"));
    assert!(ctx.contains("- [ ] Step 2: Write the tests"));

    // Mark step 1 complete, verify [x] vs [ ]
    mgr.complete_step(1);
    let ctx = mgr.plan_context_string();
    assert!(ctx.contains("- [x] Step 1: Read the source files"));
    assert!(ctx.contains("- [ ] Step 2: Write the tests"));
}

// ---------------------------------------------------------------------------
// 7. Complete Workflow
// ---------------------------------------------------------------------------

#[test]
fn test_dual_agent_full_workflow() {
    let mut mgr = make_manager();

    // 1. Start in Build mode (default)
    assert_eq!(mgr.active_slot(), AgentSlot::Build);

    // 2. Switch to Plan
    mgr.switch_to(AgentSlot::Plan);
    assert_eq!(mgr.active_slot(), AgentSlot::Plan);
    assert_eq!(mgr.active_handle().session_id.as_str(), "plan-session");

    // 3. Parse plan steps from simulated Plan Agent output
    let plan_output = "Here is the plan:\n\n1. Read all source files\n2. Refactor the module\n3. Run the test suite\n\nLet me know when ready.";
    let steps = DualAgentManager::parse_plan_steps(plan_output);
    assert_eq!(steps.len(), 3);
    for step in steps {
        mgr.add_plan_step(step);
    }

    // 4. Switch to Build
    mgr.switch_to(AgentSlot::Build);
    assert_eq!(mgr.active_slot(), AgentSlot::Build);
    assert_eq!(mgr.active_handle().session_id.as_str(), "build-session");

    // 5. Get plan context string for Build Agent
    let ctx = mgr.plan_context_string();
    assert!(ctx.contains("## Plan Steps"));
    assert!(ctx.contains("- [ ] Step 1: Read all source files"));
    assert!(ctx.contains("- [ ] Step 2: Refactor the module"));
    assert!(ctx.contains("- [ ] Step 3: Run the test suite"));

    // 6. Complete steps progressively
    assert!(mgr.complete_step(1));
    assert!(mgr.complete_step(2));

    let ctx = mgr.plan_context_string();
    assert!(ctx.contains("- [x] Step 1:"));
    assert!(ctx.contains("- [x] Step 2:"));
    assert!(ctx.contains("- [ ] Step 3:"));

    // 7. Complete final step
    assert!(mgr.complete_step(3));
    let ctx = mgr.plan_context_string();
    assert!(ctx.contains("- [x] Step 3:"));

    // 8. Session ID is preserved throughout
    assert_eq!(mgr.session_id().as_str(), "shared-session");
}

// ---------------------------------------------------------------------------
// 8. Serialization Round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_dual_types_serde_roundtrip() {
    // DualAgentProfile
    let profile = DualAgentProfile {
        name: "TestAgent".into(),
        system_prompt: "You are a test agent.".into(),
        tool_filter: ToolFilterMode::AllowList(vec!["bash".into(), "file_read".into()]),
        slot: AgentSlot::Plan,
    };
    let json = serde_json::to_string(&profile).unwrap();
    let decoded: DualAgentProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.name, "TestAgent");
    assert_eq!(decoded.slot, AgentSlot::Plan);
    assert!(decoded.tool_filter.allows_tools());

    // AgentSlot
    let slot_json = serde_json::to_string(&AgentSlot::Build).unwrap();
    let slot_back: AgentSlot = serde_json::from_str(&slot_json).unwrap();
    assert_eq!(slot_back, AgentSlot::Build);

    // PlanStep
    let step = PlanStep {
        number: 5,
        description: "Deploy to production".into(),
        completed: true,
    };
    let step_json = serde_json::to_string(&step).unwrap();
    let step_back: PlanStep = serde_json::from_str(&step_json).unwrap();
    assert_eq!(step_back.number, 5);
    assert_eq!(step_back.description, "Deploy to production");
    assert!(step_back.completed);
}

// ---------------------------------------------------------------------------
// 9. ToolFilterMode AllowList
// ---------------------------------------------------------------------------

#[test]
fn test_tool_filter_allowlist() {
    let filter = ToolFilterMode::AllowList(vec!["bash".into(), "file_read".into()]);
    assert!(filter.allows_tools());

    // None denies tools
    let none = ToolFilterMode::None;
    assert!(!none.allows_tools());

    // All allows tools
    let all = ToolFilterMode::All;
    assert!(all.allows_tools());

    // Empty AllowList still allows (zero tools, but mode is allow-list)
    let empty = ToolFilterMode::AllowList(vec![]);
    assert!(empty.allows_tools());
}

// ---------------------------------------------------------------------------
// 10. Edge Cases
// ---------------------------------------------------------------------------

#[test]
fn test_complete_nonexistent_step() {
    let mut mgr = make_manager();

    mgr.add_plan_step(PlanStep {
        number: 1,
        description: "Only step".into(),
        completed: false,
    });

    // Completing a non-existent step returns false
    assert!(!mgr.complete_step(99));
    assert!(!mgr.complete_step(0));

    // The existing step is unaffected
    assert!(!mgr.plan_steps()[0].completed);
}

#[test]
fn test_clear_steps() {
    let mut mgr = make_manager();

    mgr.add_plan_step(PlanStep {
        number: 1,
        description: "A".into(),
        completed: false,
    });
    mgr.add_plan_step(PlanStep {
        number: 2,
        description: "B".into(),
        completed: true,
    });
    assert_eq!(mgr.plan_steps().len(), 2);

    mgr.clear_steps();
    assert!(mgr.plan_steps().is_empty());
    assert_eq!(mgr.plan_context_string(), "");
}

#[test]
fn test_empty_plan_context_string() {
    let mgr = make_manager();
    assert_eq!(mgr.plan_context_string(), "");
}

// ---------------------------------------------------------------------------
// 11. AgentSlot Display and Default
// ---------------------------------------------------------------------------

#[test]
fn test_agent_slot_display_and_default() {
    assert_eq!(format!("{}", AgentSlot::Plan), "plan");
    assert_eq!(format!("{}", AgentSlot::Build), "build");
    assert_eq!(AgentSlot::default(), AgentSlot::Build);
}

// ---------------------------------------------------------------------------
// 12. Parse edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_parse_empty_and_no_steps() {
    // Empty input
    assert!(DualAgentManager::parse_plan_steps("").is_empty());

    // No list items
    let text = "Just a paragraph.\nNothing structured here.";
    assert!(DualAgentManager::parse_plan_steps(text).is_empty());
}

#[test]
fn test_parse_skips_empty_descriptions() {
    let text = "- \n- Real step\n* \n1. Another real step";
    let steps = DualAgentManager::parse_plan_steps(text);

    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].description, "Real step");
    assert_eq!(steps[1].description, "Another real step");
}

// ---------------------------------------------------------------------------
// 13. Plan and Build handles are distinct
// ---------------------------------------------------------------------------

#[test]
fn test_plan_and_build_handles_distinct() {
    let mgr = make_manager();

    assert_eq!(mgr.plan_handle().session_id.as_str(), "plan-session");
    assert_eq!(mgr.build_handle().session_id.as_str(), "build-session");

    // They are different
    assert_ne!(
        mgr.plan_handle().session_id.as_str(),
        mgr.build_handle().session_id.as_str()
    );
}
