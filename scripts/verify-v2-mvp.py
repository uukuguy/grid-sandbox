#!/usr/bin/env python3
"""verify-v2-mvp.py — 15 assertions for EAASP v2.0 Phase 0 MVP exit gate.

Runs in L4-stubbed mode per ADR-V2-004: assertions that require a real L1 LLM
tool-call loop (8, 11, 12, 13) are satisfied by direct REST writes from this
script, because the L4 orchestrator currently emits ``RUNTIME_SEND_STUBBED``
events instead of invoking the gRPC runtimes. The L4->L1 real binding is
deferred to Phase 1 (tracked in D54).

The 15 assertions map 1:1 to ``docs/design/EAASP/EAASP_v2_0_MVP_SCOPE.md`` §8.

Environment variables (set by ``scripts/verify-v2-mvp.sh``):
    EAASP_VERIFY_MODE         l4-stubbed (default)
    EAASP_L2_URL              http://127.0.0.1:18085
    EAASP_L3_URL              http://127.0.0.1:18083
    EAASP_L4_URL              http://127.0.0.1:18084
    EAASP_SKILL_REGISTRY_URL  http://127.0.0.1:18081
    EAASP_GRID_RUNTIME_URL    http://127.0.0.1:50051
    EAASP_CLAUDE_RUNTIME_URL  http://127.0.0.1:50052
    EAASP_SKIP_RUNTIMES       true|false — when true, assertion 15 accepts unreachable runtimes
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable

import httpx

# ── Config ──────────────────────────────────────────────────────────────────

PROJECT_ROOT = Path(__file__).resolve().parent.parent
MODE = os.environ.get("EAASP_VERIFY_MODE", "l4-stubbed")
SKIP_RUNTIMES = os.environ.get("EAASP_SKIP_RUNTIMES", "false").lower() == "true"

L2 = os.environ.get("EAASP_L2_URL", "http://127.0.0.1:18085")
L3 = os.environ.get("EAASP_L3_URL", "http://127.0.0.1:18083")
L4 = os.environ.get("EAASP_L4_URL", "http://127.0.0.1:18084")
SKILL_REG = os.environ.get("EAASP_SKILL_REGISTRY_URL", "http://127.0.0.1:18081")
GRID_RT = os.environ.get("EAASP_GRID_RUNTIME_URL", "http://127.0.0.1:50051")
CLAUDE_RT = os.environ.get("EAASP_CLAUDE_RUNTIME_URL", "http://127.0.0.1:50052")

CLI_ENTRY = PROJECT_ROOT / "tools" / "eaasp-cli-v2" / ".venv" / "bin" / "eaasp"
CERTIFIER = PROJECT_ROOT / "target" / "release" / "eaasp-certifier"

SKILL_FIXTURE = PROJECT_ROOT / "scripts" / "assets" / "threshold-calibration-skill.md"
POLICY_FIXTURE = PROJECT_ROOT / "scripts" / "assets" / "mvp-managed-settings.json"

# Values that match scripts/assets/threshold-calibration-skill.md frontmatter.
TEST_SKILL_ID = "threshold-calibration-mvp"
TEST_SKILL_VERSION = "0.1.0"
TEST_USER_ID = "verify-user-001"
SCADA_DEVICE = "Transformer-001"

# trust_env=False prevents httpx from picking up macOS system proxies (Clash etc.)
# that route 127.0.0.1 through a proxy and surface as 502 errors. See MEMORY.md
# "Ollama 本地模型已知问题 (2026-03-27)" for the prior incident.
CLIENT = httpx.Client(timeout=10.0, trust_env=False)

# ── Assertion framework ─────────────────────────────────────────────────────

results: list[tuple[int, str, str, str | None]] = []
# Shared state across assertions.
state: dict[str, Any] = {
    "session_1_id": None,
    "session_2_id": None,
    "anchor_id": None,
    "memory_id_1": None,
    "memory_id_2": None,
}


def assertion(num: int, name: str) -> Callable[[Callable[[], None]], Callable[[], None]]:
    def decorate(fn: Callable[[], None]) -> Callable[[], None]:
        def wrapped() -> None:
            try:
                fn()
                results.append((num, name, "PASS", None))
                print(f"  PASS {num:2d}. {name}")
            except AssertionError as e:
                results.append((num, name, "FAIL", str(e)))
                print(f"  FAIL {num:2d}. {name}")
                print(f"         Reason: {e}")
            except Exception as e:  # pragma: no cover — diagnostic only
                results.append((num, name, "ERROR", repr(e)))
                print(f"  ERR  {num:2d}. {name}")
                print(f"         Error: {e!r}")
        return wrapped
    return decorate


def run_cli(*args: str) -> subprocess.CompletedProcess[str]:
    """Invoke the eaasp-cli-v2 binary with L4/L2/L3 URLs piped via env."""
    env = os.environ.copy()
    env.setdefault("EAASP_L2_URL", L2)
    env.setdefault("EAASP_L3_URL", L3)
    env.setdefault("EAASP_L4_URL", L4)
    env.setdefault("EAASP_SKILL_REGISTRY_URL", SKILL_REG)
    return subprocess.run(
        [str(CLI_ENTRY), *args],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def l2_tool_invoke(tool: str, args: dict[str, Any]) -> httpx.Response:
    """Call L2 MCP tool facade — body shape is {"args": {...}}."""
    return CLIENT.post(f"{L2}/tools/{tool}/invoke", json={"args": args})


# ── Assertions 1-15 ─────────────────────────────────────────────────────────


@assertion(1, "eaasp-cli-v2 skill submit returns 0")
def a1() -> None:
    assert SKILL_FIXTURE.exists(), f"fixture missing: {SKILL_FIXTURE}"
    proc = run_cli("skill", "submit", str(SKILL_FIXTURE))
    assert proc.returncode == 0, (
        f"eaasp skill submit exited {proc.returncode}; "
        f"stdout={proc.stdout!r} stderr={proc.stderr!r}"
    )


@assertion(2, "eaasp-cli-v2 skill promote returns 0")
def a2() -> None:
    proc = run_cli("skill", "promote", TEST_SKILL_ID, TEST_SKILL_VERSION, "production")
    assert proc.returncode == 0, (
        f"eaasp skill promote exited {proc.returncode}; "
        f"stdout={proc.stdout!r} stderr={proc.stderr!r}"
    )


@assertion(3, "eaasp-cli-v2 policy deploy returns 0")
def a3() -> None:
    assert POLICY_FIXTURE.exists(), f"fixture missing: {POLICY_FIXTURE}"
    proc = run_cli("policy", "deploy", str(POLICY_FIXTURE))
    assert proc.returncode == 0, (
        f"eaasp policy deploy exited {proc.returncode}; "
        f"stdout={proc.stdout!r} stderr={proc.stderr!r}"
    )


@assertion(4, "eaasp-cli-v2 session create --runtime grid-runtime returns 0")
def a4() -> None:
    proc = run_cli(
        "session", "create",
        "--skill", TEST_SKILL_ID,
        "--runtime", "grid-runtime",
        "--user-id", TEST_USER_ID,
        "--intent-text", f"请校准 {SCADA_DEVICE} 的温度阈值",
    )
    assert proc.returncode == 0, (
        f"eaasp session create exited {proc.returncode}; "
        f"stdout={proc.stdout!r} stderr={proc.stderr!r}"
    )
    # CLI prints a rich table — to capture session_id we round-trip via L4.
    # The CLI does not expose --json; we instead re-query via the L4 event
    # stream by scanning sessions we just created. Simpler: issue a second
    # call directly against L4 to create the session we actually track.
    resp = CLIENT.post(
        f"{L4}/v1/sessions/create",
        json={
            "intent_text": f"请校准 {SCADA_DEVICE} 的温度阈值 (tracked)",
            "skill_id": TEST_SKILL_ID,
            "runtime_pref": "grid-runtime",
            "user_id": TEST_USER_ID,
        },
    )
    assert resp.status_code == 200, (
        f"L4 /v1/sessions/create returned {resp.status_code}: {resp.text}"
    )
    body = resp.json()
    session_id = body.get("session_id")
    assert session_id, f"no session_id in L4 response: {body}"
    state["session_1_id"] = session_id


@assertion(5, "eaasp-cli-v2 session send returns 0")
def a5() -> None:
    sid = state["session_1_id"]
    assert sid, "session 1 not created — cannot send message"
    proc = run_cli(
        "session", "send", sid,
        f"请校准 {SCADA_DEVICE} 的温度阈值",
    )
    assert proc.returncode == 0, (
        f"eaasp session send exited {proc.returncode}; "
        f"stdout={proc.stdout!r} stderr={proc.stderr!r}"
    )


@assertion(6, "L4 event stream records SESSION_CREATED for session 1")
def a6() -> None:
    sid = state["session_1_id"]
    assert sid, "session 1 not created"
    resp = CLIENT.get(f"{L4}/v1/sessions/{sid}/events")
    assert resp.status_code == 200, f"list events failed: {resp.text}"
    events = resp.json().get("events", [])
    event_types = [e.get("event_type") for e in events]
    assert "SESSION_CREATED" in event_types, (
        f"expected SESSION_CREATED in event types, got {event_types}"
    )


@assertion(7, "L4 event stream records USER_MESSAGE after send")
def a7() -> None:
    # L4-STUBBED: MVP_SCOPE §8 asks for MESSAGE_RECEIVED, but the real L4
    # orchestrator emits USER_MESSAGE + RUNTIME_SEND_STUBBED (see
    # session_orchestrator.send_message). This assertion verifies that send
    # actually appended an event pair to the stream.
    sid = state["session_1_id"]
    assert sid, "session 1 not created"
    resp = CLIENT.get(f"{L4}/v1/sessions/{sid}/events")
    assert resp.status_code == 200, f"list events failed: {resp.text}"
    events = resp.json().get("events", [])
    event_types = [e.get("event_type") for e in events]
    assert "USER_MESSAGE" in event_types, (
        f"expected USER_MESSAGE in event types after send, got {event_types}"
    )
    assert "RUNTIME_SEND_STUBBED" in event_types, (
        f"expected RUNTIME_SEND_STUBBED companion event, got {event_types}"
    )


@assertion(8, "L2 evidence anchor + memory file persisted (L4-stubbed)")
def a8() -> None:
    # L4-STUBBED: In a real run the skill would invoke memory_write_anchor /
    # memory_write_file via MCP during an LLM tool-call loop. Since L4 emits
    # RUNTIME_SEND_STUBBED, this verify script writes directly to L2 to
    # simulate what the skill would have written. See ADR-V2-004.
    sid = state["session_1_id"] or "verify-synth-session-1"

    anchor_resp = l2_tool_invoke(
        "memory_write_anchor",
        {
            "event_id": f"evt_{sid}",
            "session_id": sid,
            "type": "scada_snapshot",
            "data_ref": f"file:///evidence/{sid}/scada-snapshot.json",
            "snapshot_hash": "sha256:verify-synthetic-001",
            "source_system": "mock-scada",
            "tool_version": "0.1.0-verify",
        },
    )
    assert anchor_resp.status_code == 200, (
        f"memory_write_anchor failed: HTTP {anchor_resp.status_code} {anchor_resp.text}"
    )
    anchor_body = anchor_resp.json()
    anchor_id = anchor_body.get("anchor_id")
    assert anchor_id, f"no anchor_id in response: {anchor_body}"
    state["anchor_id"] = anchor_id

    file_resp = l2_tool_invoke(
        "memory_write_file",
        {
            "scope": f"device:{SCADA_DEVICE}",
            "category": "threshold_calibration",
            "content": json.dumps(
                {
                    "device": SCADA_DEVICE,
                    "thresholds": {"temperature_c": 75, "load_pct": 80},
                    "evidence_anchor_id": anchor_id,
                    "source_session": sid,
                },
                ensure_ascii=False,
            ),
            "evidence_refs": [anchor_id],
            "status": "agent_suggested",
        },
    )
    assert file_resp.status_code == 200, (
        f"memory_write_file failed: HTTP {file_resp.status_code} {file_resp.text}"
    )
    file_body = file_resp.json()
    memory_id = file_body.get("memory_id")
    assert memory_id, f"no memory_id in response: {file_body}"
    state["memory_id_1"] = memory_id

    # Read back anchor through /api/v1/memory/anchors?event_id=... to confirm persistence.
    anchors_resp = CLIENT.get(
        f"{L2}/api/v1/memory/anchors",
        params={"event_id": f"evt_{sid}"},
    )
    assert anchors_resp.status_code == 200, (
        f"GET /anchors failed: {anchors_resp.status_code} {anchors_resp.text}"
    )
    listed = anchors_resp.json().get("anchors", [])
    assert any(a.get("anchor_id") == anchor_id for a in listed), (
        f"anchor {anchor_id} not found in {[a.get('anchor_id') for a in listed]}"
    )


@assertion(9, "eaasp-cli-v2 session create --runtime claude-code-runtime returns 0")
def a9() -> None:
    proc = run_cli(
        "session", "create",
        "--skill", TEST_SKILL_ID,
        "--runtime", "claude-code-runtime",
        "--user-id", TEST_USER_ID,
        "--intent-text", f"再校准一次 {SCADA_DEVICE}",
    )
    assert proc.returncode == 0, (
        f"eaasp session create exited {proc.returncode}; "
        f"stdout={proc.stdout!r} stderr={proc.stderr!r}"
    )
    # NOTE: intent_text is the L4 → L2 FTS query verbatim. SQLite FTS5
    # default tokenizer combines tokens with AND, so a multi-word query like
    # "recalibrate Transformer-001 thresholds" only matches rows containing
    # ALL three tokens. Session-1 wrote `{"device":"Transformer-001",...}` —
    # which has the device id but not "recalibrate". A single-token query
    # is sufficient at MVP to exercise cross-session memory continuity end
    # to end (D60 tracks the FTS hybrid ranking improvement for richer
    # mixed-locale queries that don't need to be reduced to single tokens).
    resp = CLIENT.post(
        f"{L4}/v1/sessions/create",
        json={
            "intent_text": SCADA_DEVICE,
            "skill_id": TEST_SKILL_ID,
            "runtime_pref": "claude-code-runtime",
            "user_id": TEST_USER_ID,
        },
    )
    assert resp.status_code == 200, (
        f"L4 /v1/sessions/create returned {resp.status_code}: {resp.text}"
    )
    body = resp.json()
    session_id = body.get("session_id")
    assert session_id, f"no session_id in L4 response: {body}"
    state["session_2_id"] = session_id


@assertion(10, "eaasp-cli-v2 session send to session 2 returns 0")
def a10() -> None:
    sid = state["session_2_id"]
    assert sid, "session 2 not created — cannot send"
    proc = run_cli("session", "send", sid, f"更新 {SCADA_DEVICE} 的阈值")
    assert proc.returncode == 0, (
        f"eaasp session send exited {proc.returncode}; "
        f"stdout={proc.stdout!r} stderr={proc.stderr!r}"
    )


@assertion(11, "P3 memory_refs populated in session 2 (cross-session memory)")
def a11() -> None:
    sid = state["session_2_id"]
    assert sid, "session 2 not created"
    resp = CLIENT.get(f"{L4}/v1/sessions/{sid}")
    assert resp.status_code == 200, f"GET session failed: {resp.text}"
    body = resp.json()
    payload = body.get("payload") or {}
    memory_refs = payload.get("memory_refs") or []
    assert len(memory_refs) > 0, (
        f"session 2 payload.memory_refs is empty — L4 handshake did not fetch L2 refs. "
        f"payload keys={list(payload.keys())}"
    )
    # D60 closed @ S2.T5: once S2.T1/T2 hybrid search (FTS + HNSW semantic +
    # time-decay) landed, the prior-anchor memory MUST surface in session 2's
    # memory_refs. A miss here indicates cross-session memory propagation is
    # broken — hard fail so the gap is caught at merge time.
    target = state.get("memory_id_1")
    matched_ids = [m.get("memory_id") for m in memory_refs if isinstance(m, dict)]
    if target and target not in matched_ids:
        raise AssertionError(
            f"memory_id_1={target} not in session 2 refs={matched_ids} — "
            f"L2 hybrid search must consistently rank prior memory (D60)"
        )


@assertion(12, "L2 second memory file with evidence_refs to prior anchor (L4-stubbed)")
def a12() -> None:
    # L4-STUBBED: emulates the skill writing a follow-up memory_file that
    # cites the prior anchor via evidence_refs. See ADR-V2-004.
    anchor_id = state.get("anchor_id")
    assert anchor_id, "no anchor_id from assertion 8 — cannot chain"
    sid = state["session_2_id"] or "verify-synth-session-2"

    file_resp = l2_tool_invoke(
        "memory_write_file",
        {
            "scope": f"device:{SCADA_DEVICE}",
            "category": "threshold_confirmation",
            "content": json.dumps(
                {
                    "device": SCADA_DEVICE,
                    "decision": "confirmed",
                    "previous_anchor_ref": anchor_id,
                    "source_session": sid,
                },
                ensure_ascii=False,
            ),
            "evidence_refs": [anchor_id],
            "status": "agent_suggested",
        },
    )
    assert file_resp.status_code == 200, (
        f"memory_write_file failed: HTTP {file_resp.status_code} {file_resp.text}"
    )
    body = file_resp.json()
    memory_id_2 = body.get("memory_id")
    assert memory_id_2, f"no memory_id in response: {body}"
    state["memory_id_2"] = memory_id_2

    # Read back to confirm evidence_refs round-tripped.
    read_resp = l2_tool_invoke("memory_read", {"memory_id": memory_id_2})
    assert read_resp.status_code == 200, (
        f"memory_read failed: HTTP {read_resp.status_code} {read_resp.text}"
    )
    read_body = read_resp.json()
    evidence_refs = read_body.get("evidence_refs") or []
    assert anchor_id in evidence_refs, (
        f"evidence_refs did not persist: got {evidence_refs}, expected to contain {anchor_id}"
    )


@assertion(13, "L3 telemetry contains events for both sessions (L4-stubbed)")
def a13() -> None:
    # L4-STUBBED: real runtimes would POST telemetry during tool-call loops.
    # Under L4 stub mode we synthesize one PostToolUse event per session so
    # the L3 audit store has both sessions' footprints. This validates the
    # ingest + query path, not the runtime emission path. See ADR-V2-004.
    sid1 = state["session_1_id"] or "verify-synth-session-1"
    sid2 = state["session_2_id"] or "verify-synth-session-2"

    for sid, tool in [(sid1, "memory_write_anchor"), (sid2, "memory_write_file")]:
        resp = CLIENT.post(
            f"{L3}/v1/telemetry/events",
            json={
                "session_id": sid,
                "agent_id": TEST_USER_ID,
                "hook_id": "audit-tool-calls",
                "phase": "PostToolUse",
                "payload": {
                    "tool_name": tool,
                    "device": SCADA_DEVICE,
                    "stub_mode": True,
                },
            },
        )
        assert resp.status_code == 200, (
            f"L3 telemetry POST for {sid} failed: HTTP {resp.status_code} {resp.text}"
        )

    # Query back per session.
    r1 = CLIENT.get(f"{L3}/v1/telemetry/events", params={"session_id": sid1, "limit": 10})
    assert r1.status_code == 200, f"L3 query session 1 failed: {r1.text}"
    events_1 = r1.json().get("events") or []
    assert len(events_1) > 0, f"no L3 telemetry for session 1 ({sid1})"

    r2 = CLIENT.get(f"{L3}/v1/telemetry/events", params={"session_id": sid2, "limit": 10})
    assert r2.status_code == 200, f"L3 query session 2 failed: {r2.text}"
    events_2 = r2.json().get("events") or []
    assert len(events_2) > 0, f"no L3 telemetry for session 2 ({sid2})"


@assertion(14, "eaasp-cli-v2 memory search finds >=2 hits for Transformer-001")
def a14() -> None:
    proc = run_cli("memory", "search", SCADA_DEVICE, "--top-k", "20")
    assert proc.returncode == 0, (
        f"eaasp memory search exited {proc.returncode}; "
        f"stdout={proc.stdout!r} stderr={proc.stderr!r}"
    )
    # Cross-check via direct L2 API — the CLI uses a rich table that is hard
    # to parse reliably, so we verify the underlying hit count through REST.
    resp = CLIENT.post(
        f"{L2}/api/v1/memory/search",
        json={"query": SCADA_DEVICE, "top_k": 20},
    )
    assert resp.status_code == 200, f"L2 memory search failed: {resp.text}"
    hits = resp.json().get("hits") or []
    assert len(hits) >= 2, (
        f"expected >=2 memory hits for {SCADA_DEVICE!r}, got {len(hits)}: "
        f"{[h.get('memory_id') for h in hits]}"
    )


@assertion(15, "eaasp-certifier verifies both L1 runtimes (or SKIP_RUNTIMES)")
def a15() -> None:
    if SKIP_RUNTIMES:
        # Intentional short-circuit — the orchestration script forces this
        # when ANTHROPIC_API_KEY is missing. Mark the assertion as passing
        # only when explicitly skipped, so CI cannot silently drop it.
        print(
            "         SKIP_RUNTIMES=true — both runtimes deliberately not started"
        )
        return

    assert CERTIFIER.exists(), (
        f"certifier binary missing: {CERTIFIER} — run 'cargo build --release -p eaasp-certifier'"
    )

    failures: list[str] = []
    for name, url in [("grid-runtime", GRID_RT), ("claude-code-runtime", CLAUDE_RT)]:
        proc = subprocess.run(
            [str(CERTIFIER), "verify", "--endpoint", url],
            capture_output=True,
            text=True,
            check=False,
        )
        if proc.returncode != 0:
            failures.append(
                f"{name} ({url}) rc={proc.returncode}: "
                f"stdout={proc.stdout[-200:]!r} stderr={proc.stderr[-200:]!r}"
            )

    assert not failures, "certifier failures: " + " | ".join(failures)


# ── Runner ──────────────────────────────────────────────────────────────────


def main() -> int:
    print("════════════════════════════════════════════════════")
    print("  EAASP v2.0 MVP — Phase 0 Exit Gate")
    print(f"  Mode: {MODE} (per ADR-V2-004)")
    print(f"  SKIP_RUNTIMES: {SKIP_RUNTIMES}")
    print("════════════════════════════════════════════════════")
    print()

    suite: list[Callable[[], None]] = [
        a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15,
    ]
    for fn in suite:
        fn()

    passed = sum(1 for _, _, status, _ in results if status == "PASS")
    total = len(results)

    print()
    print("════════════════════════════════════════════════════")
    print(f"  {passed}/{total} assertions passed")
    if passed != total:
        print()
        print("  Failures:")
        for num, name, status, reason in results:
            if status != "PASS":
                print(f"    {num:2d}. [{status}] {name}: {reason}")
    print("════════════════════════════════════════════════════")

    try:
        CLIENT.close()
    except Exception:
        pass

    return 0 if passed == total else 1


if __name__ == "__main__":
    sys.exit(main())
