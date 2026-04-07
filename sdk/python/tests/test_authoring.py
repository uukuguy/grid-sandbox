"""Tests for eaasp.authoring — parser, validator, scaffold, hook builder."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from eaasp.authoring.hook_builder import HookBuilder
from eaasp.authoring.skill_parser import SkillParser
from eaasp.authoring.skill_scaffold import SkillScaffold
from eaasp.authoring.skill_validator import SkillValidator
from eaasp.models.skill import Skill, SkillFrontmatter, ScopedHook


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

VALID_SKILL_MD = """\
---
name: test-skill
version: "1.0.0"
description: A test skill for validation
author: test-team
tags:
  - test
skill_type: workflow
scope: team
hooks:
  - event: Stop
    handler_type: prompt
    config:
      prompt: "Check completion"
dependencies:
  - org/dep-one
---

You are a test assistant that helps with testing workflows.

## Steps

1. First step of the workflow
2. Second step of the workflow
"""

VALID_SKILL_MD_NO_HOOKS = """\
---
name: simple-skill
version: "1.0.0"
description: A simple skill without hooks
author: simple-team
tags: []
skill_type: domain
scope: global
---

You are a domain expert assistant for testing purposes.

## Expertise

- Domain knowledge area one
- Domain knowledge area two
"""


def _make_skill(**overrides) -> Skill:
    """Helper to create a valid Skill with optional overrides."""
    fm_defaults = {
        "name": "test-skill",
        "version": "1.0.0",
        "description": "A test skill for validation",
        "author": "test-team",
        "tags": ["test"],
        "skill_type": "workflow",
        "scope": "team",
        "hooks": [
            ScopedHook(
                event="Stop",
                handler_type="prompt",
                config={"prompt": "Check completion"},
            )
        ],
        "dependencies": ["org/dep-one"],
    }
    fm_defaults.update(overrides)
    fm = SkillFrontmatter(**fm_defaults)
    prose = "You are a test assistant.\n\n## Steps\n\n1. Do something useful in the workflow."
    return Skill(frontmatter=fm, prose=prose)


# ---------------------------------------------------------------------------
# SkillParser tests
# ---------------------------------------------------------------------------


class TestSkillParser:
    def test_parse_valid_skill_md(self):
        skill = SkillParser.parse(VALID_SKILL_MD)
        assert skill.frontmatter.name == "test-skill"
        assert skill.frontmatter.skill_type == "workflow"
        assert len(skill.frontmatter.hooks) == 1
        assert skill.frontmatter.hooks[0].event == "Stop"
        assert "test assistant" in skill.prose

    def test_parse_skill_md_no_hooks(self):
        skill = SkillParser.parse(VALID_SKILL_MD_NO_HOOKS)
        assert skill.frontmatter.name == "simple-skill"
        assert skill.frontmatter.hooks == []

    def test_parse_invalid_yaml_raises(self):
        bad_content = "no frontmatter here"
        with pytest.raises(ValueError, match="YAML frontmatter"):
            SkillParser.parse(bad_content)

    def test_render_produces_valid_skill_md(self):
        skill = _make_skill()
        rendered = SkillParser.render(skill)
        assert rendered.startswith("---\n")
        assert "name: test-skill" in rendered
        assert skill.prose in rendered

    def test_round_trip_parse_render(self):
        skill = _make_skill()
        rendered = SkillParser.render(skill)
        reparsed = SkillParser.parse(rendered)
        assert reparsed.frontmatter.name == skill.frontmatter.name
        assert reparsed.frontmatter.hooks[0].event == skill.frontmatter.hooks[0].event
        assert reparsed.frontmatter.dependencies == skill.frontmatter.dependencies

    def test_parse_file(self, tmp_path: Path):
        md_file = tmp_path / "SKILL.md"
        md_file.write_text(VALID_SKILL_MD, encoding="utf-8")
        skill = SkillParser.parse_file(md_file)
        assert skill.frontmatter.name == "test-skill"


# ---------------------------------------------------------------------------
# SkillValidator tests
# ---------------------------------------------------------------------------


class TestSkillValidator:
    def test_valid_skill_passes(self):
        skill = _make_skill()
        result = SkillValidator.validate(skill)
        assert result.valid is True
        assert result.errors == []

    def test_empty_name_error(self):
        skill = _make_skill(name="")
        result = SkillValidator.validate(skill)
        assert result.valid is False
        assert any(e.rule == "required_fields" and "name" in e.message for e in result.errors)

    def test_empty_description_error(self):
        skill = _make_skill(description="")
        result = SkillValidator.validate(skill)
        assert result.valid is False
        assert any(e.rule == "required_fields" and "description" in e.message for e in result.errors)

    def test_bad_dependency_format_error(self):
        skill = _make_skill(dependencies=["invalid-format"])
        result = SkillValidator.validate(skill)
        assert result.valid is False
        assert any(e.rule == "dependency_format" for e in result.errors)

    def test_short_prose_error(self):
        skill = _make_skill()
        skill.prose = "too short"
        result = SkillValidator.validate(skill)
        assert result.valid is False
        assert any(e.rule == "prose_length" for e in result.errors)

    def test_workflow_without_stop_hook_warning(self):
        skill = _make_skill(skill_type="workflow", hooks=[])
        result = SkillValidator.validate(skill)
        assert any(w.rule == "type_hook" for w in result.warnings)

    def test_agent_handler_without_runtime_warning(self):
        hook = ScopedHook(
            event="PostToolUse",
            handler_type="agent",
            config={"agent": "evaluator"},
        )
        skill = _make_skill(hooks=[hook], preferred_runtime=None)
        result = SkillValidator.validate(skill)
        assert any(w.rule == "affinity_handler" for w in result.warnings)

    def test_all_8_rules_pass_for_well_formed_skill(self):
        hook_stop = ScopedHook(event="Stop", handler_type="prompt", config={"prompt": "ok"})
        hook_pre = ScopedHook(event="PreToolUse", handler_type="command", config={"command": "check.py"})
        skill = _make_skill(
            name="good-skill",
            description="A well-formed skill",
            author="good-team",
            hooks=[hook_stop, hook_pre],
            dependencies=["org/dep-a", "org/dep-b"],
            scope="bu",
            preferred_runtime="grid",
        )
        result = SkillValidator.validate(skill)
        assert result.valid is True
        assert result.errors == []
        assert result.warnings == []


# ---------------------------------------------------------------------------
# SkillScaffold tests
# ---------------------------------------------------------------------------


class TestSkillScaffold:
    def test_create_workflow_scaffold(self, tmp_path: Path):
        skill_dir = SkillScaffold.create("my-workflow", skill_type="workflow", output_dir=tmp_path)
        assert skill_dir.exists()
        assert (skill_dir / "SKILL.md").exists()
        assert (skill_dir / "hooks").is_dir()
        assert (skill_dir / "tests" / "test_cases.jsonl").exists()

    def test_scaffold_skill_md_is_parseable(self, tmp_path: Path):
        skill_dir = SkillScaffold.create("my-domain", skill_type="domain", output_dir=tmp_path)
        skill = SkillParser.parse_file(skill_dir / "SKILL.md")
        assert skill.frontmatter.name == "my-domain"
        assert skill.frontmatter.skill_type == "domain"

    def test_all_four_types_scaffoldable(self, tmp_path: Path):
        for stype in ("workflow", "production", "domain", "meta"):
            skill_dir = SkillScaffold.create(f"test-{stype}", skill_type=stype, output_dir=tmp_path)
            assert (skill_dir / "SKILL.md").exists()

    def test_unknown_type_raises(self, tmp_path: Path):
        with pytest.raises(ValueError, match="Unknown skill_type"):
            SkillScaffold.create("bad", skill_type="unknown", output_dir=tmp_path)


# ---------------------------------------------------------------------------
# HookBuilder tests
# ---------------------------------------------------------------------------


class TestHookBuilder:
    def test_command_handler_is_valid_python(self):
        script = HookBuilder.command_handler("pii-check", "PreToolUse")
        assert "import json" in script
        assert "import sys" in script
        assert "pii-check" in script
        # Verify it's syntactically valid Python
        compile(script, "<test>", "exec")

    def test_http_handler_is_valid_python(self):
        script = HookBuilder.http_handler("validate-output", "PostToolUse")
        assert "FastAPI" in script
        assert "validate-output" in script
        compile(script, "<test>", "exec")

    def test_prompt_handler_returns_correct_dict(self):
        result = HookBuilder.prompt_handler("Check compliance")
        assert result["handler_type"] == "prompt"
        assert result["config"]["prompt"] == "Check compliance"
