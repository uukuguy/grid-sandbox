//! S4.T4 — Thread-scoped interrupt integration tests.
//!
//! Proves that `AgentRuntime::cancel_session` fires the target session's
//! cancel path in isolation from peer sessions. Uses a real
//! `AgentRuntime` (mirrors the pattern in `multi_session.rs`) but does
//! NOT drive a live provider — we only spawn sessions and dispatch the
//! public cancel API, inspecting the registered token's flag and the
//! post-stop cleanup state.
//!
//! Ties to:
//! - `docs/plans/2026-04-14-v2-phase2-plan.md` S4.T4 acceptance: "多
//!   session 并发跑，cancel session A，验证 session B 不受影响。"
//! - `docs/design/EAASP/AGENT_LOOP_PATTERNS_TO_ADOPT.md` #10 (hermes
//!   per-thread interrupt flag).

use std::sync::Arc;

use grid_engine::providers::ProviderConfig;
use grid_engine::{AgentCatalog, AgentRuntime, AgentRuntimeConfig, TenantContext};
use grid_types::{SandboxId, SessionId, TenantId, UserId};

/// Build a test `AgentRuntime`. Mirrors `multi_session.rs::create_test_runtime_with_limit`.
async fn make_runtime() -> Arc<AgentRuntime> {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let db_path_str = db_path.to_str().unwrap().to_string();

    let catalog = Arc::new(AgentCatalog::new());
    let runtime_config = AgentRuntimeConfig::from_parts(
        db_path_str,
        ProviderConfig::default(),
        vec![],
        None,
        None,
        false,
    );

    let tenant_context = TenantContext::for_single_user(
        TenantId::from_string("test-tenant"),
        UserId::from_string("test-user"),
    );

    // Keep the tempdir alive for the runtime's lifetime.
    std::mem::forget(db_dir);

    Arc::new(
        AgentRuntime::new(catalog, runtime_config, Some(tenant_context))
            .await
            .expect("AgentRuntime::new should succeed"),
    )
}

#[tokio::test]
async fn cancel_session_fires_target_only_not_peers() {
    // S4.T4 acceptance: cancel session A, verify session B unaffected.
    let runtime = make_runtime().await;

    let sid_a = SessionId::from_string("sess-a-S4T4");
    let sid_b = SessionId::from_string("sess-b-S4T4");
    let user = UserId::from_string("u1");

    runtime
        .start_session(sid_a.clone(), user.clone(), SandboxId::new(), vec![], None)
        .await
        .expect("start A");
    runtime
        .start_session(sid_b.clone(), user.clone(), SandboxId::new(), vec![], None)
        .await
        .expect("start B");

    // Grab the SessionEntry cancel_token clones BEFORE cancelling so we can
    // observe the flag after the cancel call returns. These are the
    // session-lifetime tokens registered by start_session_full.
    let token_a = runtime
        .get_session_cancel_token(&sid_a)
        .expect("A registered");
    let token_b = runtime
        .get_session_cancel_token(&sid_b)
        .expect("B registered");

    assert!(!token_a.is_cancelled(), "A starts uncancelled");
    assert!(!token_b.is_cancelled(), "B starts uncancelled");

    // Registry should contain exactly the two sessions.
    assert!(runtime.session_interrupts().contains(&sid_a));
    assert!(runtime.session_interrupts().contains(&sid_b));
    assert_eq!(runtime.session_interrupts().len(), 2);

    // Fire cancel for A only.
    let fired = runtime.cancel_session(&sid_a).await;
    assert!(
        fired,
        "cancel_session should return true for a registered session"
    );

    // Thread-scoped isolation: A is cancelled, B is not.
    assert!(
        token_a.is_cancelled(),
        "session A's session-lifetime token must be cancelled"
    );
    assert!(
        !token_b.is_cancelled(),
        "session B must NOT be affected by A's cancel — thread-scoped isolation"
    );
}

#[tokio::test]
async fn cancel_session_unknown_returns_false() {
    let runtime = make_runtime().await;
    let ghost = SessionId::from_string("never-started");

    let fired = runtime.cancel_session(&ghost).await;
    assert!(
        !fired,
        "cancel_session for an unknown session must return false (no error)"
    );
}

#[tokio::test]
async fn stop_session_removes_interrupt_entry() {
    let runtime = make_runtime().await;
    let sid = SessionId::from_string("sess-stop-removes");
    let user = UserId::from_string("u1");

    runtime
        .start_session(sid.clone(), user, SandboxId::new(), vec![], None)
        .await
        .expect("start");

    assert!(runtime.session_interrupts().contains(&sid));
    runtime.stop_session(&sid).await;
    assert!(
        !runtime.session_interrupts().contains(&sid),
        "stop_session must drop the interrupt registry entry"
    );
    assert!(runtime.session_interrupts().is_empty());
}

#[tokio::test]
async fn cancel_then_stop_does_not_panic_and_registry_cleared() {
    let runtime = make_runtime().await;
    let sid = SessionId::from_string("sess-cancel-then-stop");
    let user = UserId::from_string("u1");

    runtime
        .start_session(sid.clone(), user, SandboxId::new(), vec![], None)
        .await
        .expect("start");

    let token = runtime.get_session_cancel_token(&sid).expect("registered");

    assert!(runtime.cancel_session(&sid).await);
    assert!(token.is_cancelled());

    // Stop after cancel — must still clean up gracefully.
    runtime.stop_session(&sid).await;
    assert!(runtime.session_interrupts().is_empty());

    // Calling cancel again on a stopped session returns false.
    assert!(!runtime.cancel_session(&sid).await);
}

#[tokio::test]
async fn concurrent_cancels_preserve_isolation_across_many_sessions() {
    // Stress variant of the acceptance test: with N sessions, cancelling
    // odd indices from parallel tasks must not leak into even indices.
    let runtime = make_runtime().await;
    let user = UserId::from_string("u1");

    const N: usize = 8;
    let mut sids = Vec::with_capacity(N);
    let mut tokens = Vec::with_capacity(N);

    for i in 0..N {
        let sid = SessionId::from_string(format!("sess-stress-{i}"));
        runtime
            .start_session(sid.clone(), user.clone(), SandboxId::new(), vec![], None)
            .await
            .expect("start");
        let token = runtime.get_session_cancel_token(&sid).unwrap();
        tokens.push(token);
        sids.push(sid);
    }

    // Cancel odd indices concurrently.
    let mut handles = Vec::new();
    for i in (1..N).step_by(2) {
        let rt = runtime.clone();
        let sid = sids[i].clone();
        handles.push(tokio::spawn(async move {
            rt.cancel_session(&sid).await;
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    for (i, tok) in tokens.iter().enumerate() {
        if i % 2 == 1 {
            assert!(tok.is_cancelled(), "odd session {i} should be cancelled");
        } else {
            assert!(
                !tok.is_cancelled(),
                "even session {i} must NOT be cancelled — cross-session isolation under concurrent cancels"
            );
        }
    }
}
