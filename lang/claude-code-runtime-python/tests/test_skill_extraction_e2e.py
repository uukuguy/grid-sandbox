"""S3.T3 — Skill Extraction meta-skill E2E deterministic replay.

Verifies the skill-extraction workflow end-to-end via:
  - fixture trace (6-step threshold-calibration session in L2 memory)
  - MockMemoryEngine (4 required_tools canned responses)
  - MockExtractionAgent (deterministic replay, no live LLM)
  - direct subprocess invocation of the two scoped hooks

No runtime harness dependency: S3.T5 owns the scoped-hook executor. Here we
invoke the hook bash scripts directly, matching what the runtime would do.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
SKILL_DIR = REPO_ROOT / "examples" / "skills" / "skill-extraction"
HOOK_VERIFY = SKILL_DIR / "hooks" / "verify_skill_draft.sh"
HOOK_STOP = SKILL_DIR / "hooks" / "check_final_output.sh"
FIXTURE_PATH = Path(__file__).parent / "fixtures" / "skill_extraction_input_trace.json"

REQUIRED_EVENT_KEYS = {"event_id", "event_type", "session_id", "payload", "created_at"}
REQUIRED_TOOLS = {"memory_search", "memory_read", "memory_write_anchor", "memory_write_file"}


@pytest.fixture(scope="module")
def fixture_trace() -> dict[str, Any]:
    return json.loads(FIXTURE_PATH.read_text())


def _invoke_hook(hook_path: Path, payload: dict[str, Any]) -> dict[str, Any]:
    result = subprocess.run(
        ["bash", str(hook_path)],
        input=json.dumps(payload).encode(),
        capture_output=True,
        timeout=5,
        check=False,
    )
    return {
        "exit_code": result.returncode,
        "stdout": result.stdout.decode(),
        "stderr": result.stderr.decode(),
        "decision": json.loads(result.stdout.decode()) if result.stdout else None,
    }


@dataclass
class MockMemoryEngine:
    """Canned 4-tool responses for the skill-extraction workflow.

    Records call sequence so tests can assert ordering matches the SKILL.md
    contract (search → read → write_anchor → write_file).
    """

    trace: dict[str, Any]
    calls: list[tuple[str, dict[str, Any]]] = field(default_factory=list)

    async def memory_search(self, query: str, category: str | None = None, top_k: int = 10) -> dict[str, Any]:
        self.calls.append(("memory_search", {"query": query, "category": category, "top_k": top_k}))
        return {
            "hits": [
                {
                    "memory_id": f"mem_{self.trace['cluster_id']}",
                    "relevance": 0.93,
                    "scope": self.trace["access_scope"],
                    "category": category or "event_cluster",
                }
            ]
        }

    async def memory_read(self, memory_id: str) -> dict[str, Any]:
        self.calls.append(("memory_read", {"memory_id": memory_id}))
        return {
            "memory_id": memory_id,
            "version": 1,
            "scope": self.trace["access_scope"],
            "category": "event_cluster",
            "status": "agent_suggested",
            "content": json.dumps(self.trace["events"]),
        }

    async def memory_write_anchor(self, **kwargs: Any) -> dict[str, Any]:
        self.calls.append(("memory_write_anchor", kwargs))
        return {
            "anchor_id": f"anc_skill_src_{self.trace['session_id']}",
            "event_id": kwargs["event_id"],
            "session_id": kwargs["session_id"],
            "type": kwargs["type"],
            "data_ref": kwargs.get("data_ref"),
            "created_at": 1713168020000,
        }

    async def memory_write_file(self, **kwargs: Any) -> dict[str, Any]:
        self.calls.append(("memory_write_file", kwargs))
        return {
            "memory_id": f"mem_skill_draft_{self.trace['session_id']}_v1",
            "version": 1,
            "scope": kwargs["scope"],
            "category": kwargs["category"],
            "status": kwargs["status"],
        }

    async def skill_submit_draft(self, **kwargs: Any) -> dict[str, Any]:
        # N14 honeypot: skill-extraction must never auto-submit drafts.
        self.calls.append(("skill_submit_draft", kwargs))
        raise AssertionError(
            "N14 violation: skill_submit_draft must never be called by skill-extraction "
            "(workflow.required_tools deliberately excludes it; human approval required)"
        )


class MockExtractionAgent:
    """Deterministic replay of the skill-extraction workflow, no LLM."""

    def __init__(self, memory: MockMemoryEngine, trace: dict[str, Any]) -> None:
        self.memory = memory
        self.trace = trace

    async def run(self) -> dict[str, Any]:
        search = await self.memory.memory_search(
            query=self.trace["cluster_id"],
            category="event_cluster",
            top_k=1,
        )
        cluster_mid = search["hits"][0]["memory_id"]
        cluster = await self.memory.memory_read(memory_id=cluster_mid)
        events = json.loads(cluster["content"])
        inferred_tools = sorted(
            {
                e["payload"]["tool_name"]
                for e in events
                if "tool_name" in e.get("payload", {})
            }
        )
        anchor = await self.memory.memory_write_anchor(
            event_id=f"evt_extract_{self.trace['session_id']}",
            session_id=self.trace["session_id"],
            type="skill_extraction_source",
            data_ref=self.trace["cluster_id"],
            source_system="skill-extraction",
            metadata={"event_count": len(events)},
        )
        draft_content = json.dumps(
            {
                "frontmatter_yaml": "---\nname: threshold-calibration-variant-A\n---\n",
                "prose": "# Extracted\n\n## Task\nReplay of 6-event cluster.",
                "suggested_skill_id": "threshold-calibration-variant-A",
                "suggested_name": "Threshold Calibration (Extracted)",
                "notes": f"Inferred from {len(events)} events",
                "inferred_tools": inferred_tools,
            }
        )
        draft = await self.memory.memory_write_file(
            scope=self.trace["access_scope"],
            category="skill_draft",
            content=draft_content,
            evidence_refs=[anchor["anchor_id"]],
            status="agent_suggested",
        )
        return {
            "draft_memory_id": draft["memory_id"],
            "source_cluster_id": self.trace["cluster_id"],
            "suggested_skill_id": "threshold-calibration-variant-A",
            "evidence_anchor_id": anchor["anchor_id"],
            "event_count": len(events),
            "analysis_summary": "Replay of threshold-calibration 6-event cluster.",
            "confidence_score": 0.87,
        }


def test_fixture_loads_and_matches_event_schema(fixture_trace: dict[str, Any]) -> None:
    assert fixture_trace["schema_version"] == 1
    assert fixture_trace["cluster_id"]
    assert fixture_trace["session_id"]
    assert len(fixture_trace["events"]) == 6
    for ev in fixture_trace["events"]:
        missing = REQUIRED_EVENT_KEYS - ev.keys()
        assert not missing, f"event {ev.get('event_id')} missing keys: {missing}"
        assert ev["session_id"] == fixture_trace["session_id"]
        # cluster_id is optional in the L4 Event model; guard with .get() so missing keys
        # surface as AssertionError, not KeyError.
        assert ev.get("cluster_id") == fixture_trace["cluster_id"], (
            f"event {ev['event_id']} missing/wrong cluster_id"
        )
        assert isinstance(ev["created_at"], int)


def test_post_tool_use_hook_allows_valid_write() -> None:
    result = _invoke_hook(
        HOOK_VERIFY,
        {
            "tool_name": "memory_write_file",
            "tool_result": {
                "memory_id": "mem_skill_draft_sess_001_v1",
                "status": "agent_suggested",
                "version": 1,
            },
        },
    )
    assert result["exit_code"] == 0
    assert result["decision"]["decision"] == "allow"


def test_post_tool_use_hook_continues_on_invalid() -> None:
    result = _invoke_hook(
        HOOK_VERIFY,
        {
            "tool_name": "memory_write_file",
            "tool_result": {"status": "agent_suggested"},
        },
    )
    assert result["exit_code"] == 2
    assert result["decision"]["decision"] == "continue"
    assert "memory_id" in result["decision"]["reason"]


def test_post_tool_use_hook_passthrough_for_other_tools() -> None:
    result = _invoke_hook(
        HOOK_VERIFY,
        {"tool_name": "memory_search", "tool_result": {"hits": []}},
    )
    assert result["exit_code"] == 0
    assert result["decision"]["decision"] == "allow"


def test_stop_hook_rejects_missing_draft_id() -> None:
    result = _invoke_hook(
        HOOK_STOP,
        {"output": {"draft_memory_id": "", "evidence_anchor_id": "anc_abc"}},
    )
    assert result["exit_code"] == 2
    assert result["decision"]["decision"] == "continue"


def test_stop_hook_rejects_missing_evidence_anchor() -> None:
    result = _invoke_hook(
        HOOK_STOP,
        {"output": {"draft_memory_id": "mem_x", "evidence_anchor_id": ""}},
    )
    assert result["exit_code"] == 2
    assert result["decision"]["decision"] == "continue"


def test_stop_hook_rejects_null_ids() -> None:
    result = _invoke_hook(
        HOOK_STOP,
        {"output": {"draft_memory_id": None, "evidence_anchor_id": "anc_abc"}},
    )
    assert result["exit_code"] == 2
    assert result["decision"]["decision"] == "continue"


def test_stop_hook_allows_complete_output() -> None:
    result = _invoke_hook(
        HOOK_STOP,
        {
            "output": {
                "draft_memory_id": "mem_skill_draft_sess_20260415_001_v1",
                "evidence_anchor_id": "anc_skill_src_sess_20260415_001",
            }
        },
    )
    assert result["exit_code"] == 0
    assert result["decision"]["decision"] == "allow"


@pytest.mark.asyncio
async def test_skill_extraction_workflow_replay(fixture_trace: dict[str, Any]) -> None:
    memory = MockMemoryEngine(trace=fixture_trace)
    agent = MockExtractionAgent(memory=memory, trace=fixture_trace)
    output = await agent.run()

    assert output["draft_memory_id"]
    assert output["evidence_anchor_id"]
    assert output["source_cluster_id"] == fixture_trace["cluster_id"]
    assert output["event_count"] == 6
    assert 0.0 <= output["confidence_score"] <= 1.0

    tool_order = [name for name, _ in memory.calls]
    assert tool_order == [
        "memory_search",
        "memory_read",
        "memory_write_anchor",
        "memory_write_file",
    ]

    anchor_call = dict(memory.calls[2][1])
    assert anchor_call["type"] == "skill_extraction_source"
    assert anchor_call["data_ref"] == fixture_trace["cluster_id"]
    file_call = dict(memory.calls[3][1])
    assert file_call["status"] == "agent_suggested"
    assert file_call["category"] == "skill_draft"
    assert file_call["evidence_refs"] == [output["evidence_anchor_id"]]


@pytest.mark.asyncio
async def test_replay_output_passes_stop_hook(fixture_trace: dict[str, Any]) -> None:
    memory = MockMemoryEngine(trace=fixture_trace)
    agent = MockExtractionAgent(memory=memory, trace=fixture_trace)
    output = await agent.run()

    result = _invoke_hook(HOOK_STOP, {"output": output})
    assert result["exit_code"] == 0
    assert result["decision"]["decision"] == "allow"


@pytest.mark.asyncio
async def test_replay_write_file_passes_post_tool_use_hook(
    fixture_trace: dict[str, Any],
) -> None:
    memory = MockMemoryEngine(trace=fixture_trace)
    agent = MockExtractionAgent(memory=memory, trace=fixture_trace)
    await agent.run()

    write_file_call = next(c for c in memory.calls if c[0] == "memory_write_file")
    synthesized_result = {
        "memory_id": f"mem_skill_draft_{fixture_trace['session_id']}_v1",
        "status": write_file_call[1]["status"],
        "version": 1,
    }
    result = _invoke_hook(
        HOOK_VERIFY,
        {"tool_name": "memory_write_file", "tool_result": synthesized_result},
    )
    assert result["exit_code"] == 0
    assert result["decision"]["decision"] == "allow"


@pytest.mark.asyncio
async def test_n14_compliance_no_skill_submit_draft_call(
    fixture_trace: dict[str, Any],
) -> None:
    """N14: skill-extraction must never auto-submit drafts to the registry."""
    memory = MockMemoryEngine(trace=fixture_trace)
    agent = MockExtractionAgent(memory=memory, trace=fixture_trace)
    output = await agent.run()

    call_names = {name for name, _ in memory.calls}
    assert "skill_submit_draft" not in call_names
    assert call_names.issubset(REQUIRED_TOOLS)

    output_str = json.dumps(output)
    assert "skill_submit_draft" not in output_str

    file_call = next(c for c in memory.calls if c[0] == "memory_write_file")
    assert file_call[1]["status"] == "agent_suggested"
