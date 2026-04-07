"""Multi-rule Skill validator — 8 validation rules."""

from __future__ import annotations

import re

from pydantic import BaseModel

from eaasp.models.skill import Skill


class ValidationIssue(BaseModel):
    """A single validation issue (error or warning)."""

    rule: str
    message: str
    severity: str  # "error" or "warning"


class ValidationResult(BaseModel):
    """Aggregated validation result."""

    valid: bool
    errors: list[ValidationIssue] = []
    warnings: list[ValidationIssue] = []


_VALID_EVENTS = {"PreToolUse", "PostToolUse", "Stop"}
_VALID_HANDLER_TYPES = {"command", "http", "prompt", "agent"}
_VALID_SCOPES = {"global", "bu", "dept", "team"}
_DEP_PATTERN = re.compile(r"^[a-z0-9][a-z0-9-]*/[a-z0-9][a-z0-9-]*$")
_MIN_PROSE_LEN = 50


class SkillValidator:
    """Validate a Skill against 8 rules."""

    @staticmethod
    def validate(skill: Skill) -> ValidationResult:
        errors: list[ValidationIssue] = []
        warnings: list[ValidationIssue] = []
        fm = skill.frontmatter

        # Rule 1: required fields non-empty
        for field in ("name", "description", "author"):
            val = getattr(fm, field)
            if not val or not val.strip():
                errors.append(
                    ValidationIssue(
                        rule="required_fields",
                        message=f"Field '{field}' must be non-empty",
                        severity="error",
                    )
                )

        # Rule 2: hook event legality
        for i, hook in enumerate(fm.hooks):
            if hook.event not in _VALID_EVENTS:
                errors.append(
                    ValidationIssue(
                        rule="hook_event",
                        message=f"Hook #{i}: invalid event '{hook.event}', must be one of {_VALID_EVENTS}",
                        severity="error",
                    )
                )

        # Rule 3: handler_type legality
        for i, hook in enumerate(fm.hooks):
            if hook.handler_type not in _VALID_HANDLER_TYPES:
                errors.append(
                    ValidationIssue(
                        rule="handler_type",
                        message=f"Hook #{i}: invalid handler_type '{hook.handler_type}'",
                        severity="error",
                    )
                )

        # Rule 4: dependency ID format
        for dep in fm.dependencies:
            if not _DEP_PATTERN.match(dep):
                errors.append(
                    ValidationIssue(
                        rule="dependency_format",
                        message=f"Dependency '{dep}' must match 'org/name' pattern (lowercase alphanumeric + hyphens)",
                        severity="error",
                    )
                )

        # Rule 5: scope legality
        if fm.scope not in _VALID_SCOPES:
            errors.append(
                ValidationIssue(
                    rule="scope",
                    message=f"Scope '{fm.scope}' must be one of {_VALID_SCOPES}",
                    severity="error",
                )
            )

        # Rule 6: prose length
        if len(skill.prose.strip()) < _MIN_PROSE_LEN:
            errors.append(
                ValidationIssue(
                    rule="prose_length",
                    message=f"Prose must be at least {_MIN_PROSE_LEN} characters, got {len(skill.prose.strip())}",
                    severity="error",
                )
            )

        # Rule 7: affinity × handler — agent handler warns if preferred_runtime not set
        for i, hook in enumerate(fm.hooks):
            if hook.handler_type == "agent" and not fm.preferred_runtime:
                warnings.append(
                    ValidationIssue(
                        rule="affinity_handler",
                        message=f"Hook #{i}: agent handler without preferred_runtime set — runtime behavior may vary",
                        severity="warning",
                    )
                )

        # Rule 8: type × hook — workflow without Stop hook gets warning
        if fm.skill_type == "workflow":
            has_stop = any(h.event == "Stop" for h in fm.hooks)
            if not has_stop:
                warnings.append(
                    ValidationIssue(
                        rule="type_hook",
                        message="Workflow skill without Stop hook — completion check recommended",
                        severity="warning",
                    )
                )

        valid = len(errors) == 0
        return ValidationResult(valid=valid, errors=errors, warnings=warnings)
