#!/usr/bin/env python3
"""Load per-project .adr-config.yaml with fallback to defaults.

Usage:
    from adr_config import load_config
    cfg = load_config()  # auto-detects project root
    cfg = load_config(project_root="/path/to/project")

Stdlib only — no external YAML parser. Uses a minimal homegrown YAML subset
(key: value, lists with "- " prefix, nested dicts by indent).
"""
from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


DEFAULTS = {
    "adr_root": "docs/adrs",
    "contract_test_root": "tests",
    "downgrade_targets": {
        "plan": "docs/plans/completed",
        "decision": "docs/decisions",
    },
    "ci_workflows": [".github/workflows/"],
    "adr_id_prefix": "ADR",
    # Fallback ADR root candidates in preference order (used if default missing)
    "adr_root_candidates": [
        "docs/adrs",
        "docs/design/adrs",
        "docs/design/EAASP/adrs",
        "docs/architecture/adrs",
    ],
}


@dataclass
class AdrConfig:
    project_root: Path
    adr_root: Path
    contract_test_root: Path
    downgrade_targets: dict
    ci_workflows: list
    adr_id_prefix: str
    raw: dict = field(default_factory=dict)

    def resolve(self, rel: str | Path) -> Path:
        """Resolve a path relative to project root."""
        p = Path(rel)
        if p.is_absolute():
            return p
        return self.project_root / p


def _parse_simple_yaml(text: str) -> dict:
    """Minimal YAML parser for flat and 1-level-nested dicts + lists.

    Supports:
        key: value
        key:
          - item1
          - item2
        key:
          subkey: subvalue
    """
    result: dict = {}
    current_key: str | None = None
    current_list: list | None = None
    current_dict: dict | None = None

    lines = text.splitlines()
    for raw in lines:
        # Strip comments (but preserve # inside strings — naive, good enough for config)
        line = raw.split("#", 1)[0] if not raw.lstrip().startswith("#") else ""
        if not line.strip():
            continue

        indent = len(line) - len(line.lstrip())
        stripped = line.strip()

        if indent == 0:
            # Top-level key
            current_list = None
            current_dict = None
            if ":" not in stripped:
                continue
            key, _, val = stripped.partition(":")
            key = key.strip()
            val = val.strip()
            if val:
                # Inline value
                result[key] = _coerce(val)
                current_key = None
            else:
                # Nested block follows
                current_key = key
                result[key] = None
        elif indent >= 2 and current_key is not None:
            if stripped.startswith("- "):
                # List item under current_key
                if current_list is None:
                    current_list = []
                    result[current_key] = current_list
                current_list.append(_coerce(stripped[2:].strip()))
            elif ":" in stripped:
                # Dict item under current_key
                if current_dict is None:
                    current_dict = {}
                    result[current_key] = current_dict
                sub_key, _, sub_val = stripped.partition(":")
                current_dict[sub_key.strip()] = _coerce(sub_val.strip())
    return result


def _coerce(val: str) -> Any:
    """Coerce string value to Python type (best-effort)."""
    if val == "null" or val == "~" or val == "":
        return None
    if val == "true":
        return True
    if val == "false":
        return False
    if val.startswith('"') and val.endswith('"'):
        return val[1:-1]
    if val.startswith("'") and val.endswith("'"):
        return val[1:-1]
    if val.startswith("[") and val.endswith("]"):
        # Inline list
        inner = val[1:-1].strip()
        if not inner:
            return []
        return [_coerce(x.strip()) for x in inner.split(",")]
    try:
        return int(val)
    except ValueError:
        pass
    try:
        return float(val)
    except ValueError:
        pass
    return val


def find_project_root(start: Path | None = None) -> Path:
    """Walk up from start (or cwd) to find project root markers."""
    markers = {".git", "pyproject.toml", "Cargo.toml", "package.json", ".adr-config.yaml"}
    p = (start or Path.cwd()).resolve()
    while p != p.parent:
        if any((p / m).exists() for m in markers):
            return p
        p = p.parent
    return (start or Path.cwd()).resolve()


def load_config(project_root: str | Path | None = None) -> AdrConfig:
    """Load .adr-config.yaml from project root, with fallback to defaults."""
    root = Path(project_root).resolve() if project_root else find_project_root()

    config_path = root / ".adr-config.yaml"
    user_config = {}
    if config_path.exists():
        user_config = _parse_simple_yaml(config_path.read_text(encoding="utf-8"))

    # Merge with defaults
    merged = dict(DEFAULTS)
    merged.update(user_config)

    # Resolve adr_root: if user didn't specify and default doesn't exist,
    # try the candidates in order
    adr_root_str = merged.get("adr_root", DEFAULTS["adr_root"])
    adr_root = root / adr_root_str
    if not adr_root.exists() and "adr_root" not in user_config:
        for candidate in DEFAULTS["adr_root_candidates"]:
            cand_path = root / candidate
            if cand_path.exists():
                adr_root = cand_path
                adr_root_str = candidate
                break

    return AdrConfig(
        project_root=root,
        adr_root=adr_root,
        contract_test_root=root / merged.get("contract_test_root", "tests"),
        downgrade_targets=merged.get("downgrade_targets", DEFAULTS["downgrade_targets"]),
        ci_workflows=merged.get("ci_workflows", DEFAULTS["ci_workflows"]),
        adr_id_prefix=merged.get("adr_id_prefix", "ADR"),
        raw=merged,
    )


if __name__ == "__main__":
    cfg = load_config()
    print(f"project_root:       {cfg.project_root}")
    print(f"adr_root:           {cfg.adr_root}")
    print(f"contract_test_root: {cfg.contract_test_root}")
    print(f"downgrade_targets:  {cfg.downgrade_targets}")
    print(f"ci_workflows:       {cfg.ci_workflows}")
    print(f"adr_id_prefix:      {cfg.adr_id_prefix}")
