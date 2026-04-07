"""Skill scaffold generator — 4 template types."""

from __future__ import annotations

from pathlib import Path

import yaml


# Default hook configurations per skill type
_TEMPLATES: dict[str, dict] = {
    "workflow": {
        "hooks": [
            {
                "event": "Stop",
                "handler_type": "prompt",
                "config": {
                    "prompt": "Verify that all workflow steps have been completed successfully."
                },
            }
        ],
        "prose": (
            "You are a workflow automation assistant.\n\n"
            "## Workflow Steps\n\n"
            "1. [Step 1: Describe the first step]\n"
            "2. [Step 2: Describe the second step]\n"
            "3. [Step 3: Describe the third step]\n\n"
            "## Quality Standards\n\n"
            "- All steps must be completed in order\n"
            "- Each step must be verified before proceeding\n"
        ),
    },
    "production": {
        "hooks": [
            {
                "event": "PostToolUse",
                "handler_type": "command",
                "config": {"command": "python hooks/validate_output.py"},
                "match": {"tool_name": "file_write"},
            }
        ],
        "prose": (
            "You are a production-grade assistant with strict output validation.\n\n"
            "## Responsibilities\n\n"
            "- [Describe primary responsibilities]\n\n"
            "## Output Format\n\n"
            "- All outputs must conform to the required format\n"
            "- Validation hooks ensure quality\n"
        ),
    },
    "domain": {
        "hooks": [
            {
                "event": "PreToolUse",
                "handler_type": "prompt",
                "config": {
                    "prompt": "Check if this action complies with domain regulations and policies."
                },
            }
        ],
        "prose": (
            "You are a domain expert assistant with compliance checks.\n\n"
            "## Domain Rules\n\n"
            "- [Rule 1: Describe compliance requirement]\n"
            "- [Rule 2: Describe compliance requirement]\n\n"
            "## Expertise\n\n"
            "- [Describe domain expertise area]\n"
        ),
    },
    "meta": {
        "hooks": [
            {
                "event": "PostToolUse",
                "handler_type": "agent",
                "config": {"agent": "evaluator", "prompt": "Evaluate the quality of this output."},
            }
        ],
        "prose": (
            "You are a meta-level assistant that evaluates and improves other Skills.\n\n"
            "## Evaluation Criteria\n\n"
            "- [Criterion 1: Describe evaluation dimension]\n"
            "- [Criterion 2: Describe evaluation dimension]\n\n"
            "## Process\n\n"
            "- Analyze the target Skill\n"
            "- Provide actionable improvement recommendations\n"
        ),
    },
}


class SkillScaffold:
    """Generate Skill project scaffolds."""

    @staticmethod
    def create(
        name: str,
        skill_type: str = "workflow",
        output_dir: Path = Path("."),
    ) -> Path:
        """Create a Skill project skeleton.

        Returns the path to the created Skill directory.
        """
        if skill_type not in _TEMPLATES:
            raise ValueError(f"Unknown skill_type '{skill_type}', must be one of {set(_TEMPLATES)}")

        skill_dir = output_dir / name
        skill_dir.mkdir(parents=True, exist_ok=True)
        (skill_dir / "hooks").mkdir(exist_ok=True)
        (skill_dir / "tests").mkdir(exist_ok=True)

        # Build frontmatter
        template = _TEMPLATES[skill_type]
        frontmatter = {
            "name": name,
            "version": "1.0.0",
            "description": f"[TODO] Describe the {name} skill",
            "author": "[TODO] your-team",
            "tags": [skill_type],
            "skill_type": skill_type,
            "scope": "team",
            "hooks": template["hooks"],
        }
        yaml_str = yaml.dump(
            frontmatter, default_flow_style=False, allow_unicode=True, sort_keys=False
        )
        prose = template["prose"]
        skill_md = f"---\n{yaml_str}---\n\n{prose}"
        (skill_dir / "SKILL.md").write_text(skill_md, encoding="utf-8")

        # Empty test cases file
        (skill_dir / "tests" / "test_cases.jsonl").write_text("", encoding="utf-8")

        return skill_dir
