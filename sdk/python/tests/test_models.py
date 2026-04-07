"""Tests for eaasp.models — Pydantic v2 models and SKILL.md round-trip."""

import json
import tempfile
from pathlib import Path

import pytest
from pydantic import ValidationError

from eaasp.models.skill import Skill, SkillFrontmatter, ScopedHook
from eaasp.models.policy import Policy, PolicyRule
from eaasp.models.playbook import Playbook, PlaybookStep
from eaasp.models.tool import ToolDef, McpServerConfig
from eaasp.models.message import UserMessage, ResponseChunk
from eaasp.models.session import SessionConfig, SessionState
from eaasp.models.agent import AgentCapability, CapabilityManifest, CostEstimate


# ── Skill tests ──


class TestScopedHook:
    def test_valid_hook(self):
        hook = ScopedHook(
            event="PreToolUse",
            handler_type="command",
            config={"command": "python check.py"},
            match={"tool_name": "bash"},
        )
        assert hook.event == "PreToolUse"
        assert hook.handler_type == "command"
        assert hook.match == {"tool_name": "bash"}

    def test_invalid_event_rejected(self):
        with pytest.raises(ValidationError):
            ScopedHook(event="InvalidEvent", handler_type="command")

    def test_invalid_handler_type_rejected(self):
        with pytest.raises(ValidationError):
            ScopedHook(event="PreToolUse", handler_type="invalid")


class TestSkillFrontmatter:
    def test_minimal(self):
        fm = SkillFrontmatter(name="test-skill")
        assert fm.name == "test-skill"
        assert fm.version == "1.0.0"
        assert fm.skill_type == "workflow"
        assert fm.scope == "team"
        assert fm.hooks == []
        assert fm.dependencies == []

    def test_full(self):
        fm = SkillFrontmatter(
            name="hr-onboarding",
            version="2.0.0",
            description="HR workflow",
            author="hr-team",
            tags=["hr", "workflow"],
            skill_type="workflow",
            preferred_runtime="grid",
            compatible_runtimes=["grid", "claude-code"],
            hooks=[ScopedHook(event="Stop", handler_type="prompt", config={"prompt": "check"})],
            dependencies=["org/it-setup"],
            scope="bu",
        )
        assert fm.skill_type == "workflow"
        assert len(fm.hooks) == 1
        assert fm.hooks[0].event == "Stop"


class TestSkill:
    @pytest.fixture
    def sample_skill(self) -> Skill:
        return Skill(
            frontmatter=SkillFrontmatter(
                name="test-skill",
                version="1.0.0",
                description="A test skill for validation",
                author="test-author",
                tags=["test"],
                skill_type="workflow",
                hooks=[
                    ScopedHook(
                        event="PreToolUse",
                        handler_type="command",
                        config={"command": "python check.py"},
                    )
                ],
            ),
            prose="You are a helpful assistant.\n\n## Instructions\n\nDo the thing carefully.",
        )

    def test_to_skill_md(self, sample_skill: Skill):
        md = sample_skill.to_skill_md()
        assert md.startswith("---\n")
        assert "name: test-skill" in md
        assert "You are a helpful assistant." in md

    def test_from_skill_md(self, sample_skill: Skill):
        md = sample_skill.to_skill_md()
        parsed = Skill.from_skill_md(md)
        assert parsed.frontmatter.name == "test-skill"
        assert parsed.frontmatter.version == "1.0.0"
        assert len(parsed.frontmatter.hooks) == 1
        assert "helpful assistant" in parsed.prose

    def test_round_trip(self, sample_skill: Skill):
        md = sample_skill.to_skill_md()
        parsed = Skill.from_skill_md(md)
        assert parsed.frontmatter.name == sample_skill.frontmatter.name
        assert parsed.frontmatter.version == sample_skill.frontmatter.version
        assert parsed.frontmatter.skill_type == sample_skill.frontmatter.skill_type
        assert len(parsed.frontmatter.hooks) == len(sample_skill.frontmatter.hooks)
        # Prose may have minor whitespace differences, check core content
        assert "helpful assistant" in parsed.prose

    def test_from_skill_md_no_delimiters(self):
        with pytest.raises(ValueError, match="YAML frontmatter"):
            Skill.from_skill_md("no frontmatter here")

    def test_from_file(self, sample_skill: Skill, tmp_path: Path):
        md = sample_skill.to_skill_md()
        skill_file = tmp_path / "SKILL.md"
        skill_file.write_text(md, encoding="utf-8")
        loaded = Skill.from_file(skill_file)
        assert loaded.frontmatter.name == "test-skill"

    def test_serialization_json(self, sample_skill: Skill):
        data = sample_skill.model_dump()
        assert data["frontmatter"]["name"] == "test-skill"
        # Ensure JSON-serializable
        json_str = json.dumps(data)
        assert "test-skill" in json_str


# ── Policy tests ──


class TestPolicy:
    def test_policy_creation(self):
        policy = Policy(
            name="security-baseline",
            version="1.0.0",
            author="security-team",
            rules=[
                PolicyRule(
                    name="deny-bash",
                    condition="tool_name == 'bash'",
                    action="deny",
                    priority=10,
                ),
                PolicyRule(
                    name="audit-file-write",
                    condition="tool_name == 'file_write'",
                    action="audit",
                ),
            ],
        )
        assert len(policy.rules) == 2
        assert policy.rules[0].action == "deny"
        assert policy.rules[0].priority == 10


# ── Playbook tests ──


class TestPlaybook:
    def test_playbook_creation(self):
        pb = Playbook(
            name="onboarding-flow",
            steps=[
                PlaybookStep(
                    name="create-account",
                    skill_id="org/it-account-setup",
                    output_key="account",
                ),
                PlaybookStep(
                    name="provision-badge",
                    skill_id="org/badge-provisioning",
                    condition="account.success",
                    on_failure="skip",
                ),
            ],
        )
        assert len(pb.steps) == 2
        assert pb.steps[1].on_failure == "skip"


# ── Tool tests ──


class TestToolDef:
    def test_tool_def(self):
        tool = ToolDef(
            name="file_read",
            description="Read a file",
            input_schema={"type": "object", "properties": {"path": {"type": "string"}}},
            category="file",
        )
        assert tool.name == "file_read"
        assert tool.category == "file"


class TestMcpServerConfig:
    def test_stdio_config(self):
        cfg = McpServerConfig(
            name="filesystem",
            transport="stdio",
            command="npx",
            args=["-y", "@modelcontextprotocol/server-filesystem"],
        )
        assert cfg.transport == "stdio"
        assert cfg.command == "npx"

    def test_sse_config(self):
        cfg = McpServerConfig(
            name="remote-service",
            transport="sse",
            url="http://localhost:8080/sse",
        )
        assert cfg.transport == "sse"
        assert cfg.url is not None


# ── Message tests ──


class TestMessage:
    def test_user_message(self):
        msg = UserMessage(content="Hello", metadata={"source": "web"})
        assert msg.message_type == "text"
        assert msg.metadata["source"] == "web"

    def test_response_chunk_text(self):
        chunk = ResponseChunk(chunk_type="text_delta", content="Hello")
        assert chunk.chunk_type == "text_delta"
        assert not chunk.is_error

    def test_response_chunk_tool(self):
        chunk = ResponseChunk(
            chunk_type="tool_start",
            tool_name="bash",
            tool_id="tool_001",
        )
        assert chunk.tool_name == "bash"

    def test_response_chunk_error(self):
        chunk = ResponseChunk(chunk_type="error", content="rate limit", is_error=True)
        assert chunk.is_error


# ── Session tests ──


class TestSession:
    def test_session_config_minimal(self):
        cfg = SessionConfig()
        assert cfg.user_id == ""
        assert cfg.allowed_skill_search is False

    def test_session_config_full(self):
        cfg = SessionConfig(
            user_id="user-123",
            user_role="developer",
            org_unit="engineering",
            skill_ids=["org/hr-onboarding"],
            skill_registry_url="http://localhost:8081",
            allowed_skill_search=True,
            skill_search_scope=["org/erp/*"],
        )
        assert cfg.user_id == "user-123"
        assert len(cfg.skill_ids) == 1

    def test_session_state(self):
        state = SessionState(
            session_id="sess-001",
            runtime_id="grid-main",
            state_format="rust-serde-v1",
        )
        assert state.session_id == "sess-001"
        assert state.state_format == "rust-serde-v1"


# ── Agent tests ──


class TestAgent:
    def test_capability(self):
        cap = AgentCapability(name="code_execution", supported=True, description="Can run code")
        assert cap.supported

    def test_capability_manifest_all_tiers(self):
        for tier in ("harness", "aligned", "framework"):
            manifest = CapabilityManifest(
                runtime_id=f"runtime-{tier}",
                tier=tier,
                model="claude-sonnet-4-20250514",
                context_window=200000,
                supported_tools=["bash", "file_read"],
                native_hooks=(tier == "aligned"),
                native_mcp=True,
                cost=CostEstimate(input_cost_per_1k=0.003, output_cost_per_1k=0.015),
            )
            assert manifest.tier == tier
            assert manifest.runtime_id == f"runtime-{tier}"

    def test_cost_estimate_defaults(self):
        cost = CostEstimate()
        assert cost.input_cost_per_1k == 0.0
        assert cost.output_cost_per_1k == 0.0


# ── JSON Schema validation (cross-check) ──


class TestSchemaValidation:
    def test_skill_schema_matches_model(self):
        """Verify that a Skill model instance validates against the JSON Schema."""
        jsonschema = pytest.importorskip("jsonschema")
        # tests/ → python/ → sdk/ → specs/
        schema_path = Path(__file__).parent.parent.parent / "specs" / "skill.schema.json"
        if not schema_path.exists():
            pytest.skip("skill.schema.json not found")
        schema = json.loads(schema_path.read_text())
        skill = Skill(
            frontmatter=SkillFrontmatter(
                name="test",
                description="Test skill",
                author="tester",
                hooks=[ScopedHook(event="Stop", handler_type="prompt", config={"prompt": "ok"})],
            ),
            prose="This is the skill prose content that is long enough.",
        )
        data = skill.model_dump()
        jsonschema.validate(instance=data, schema=schema)
