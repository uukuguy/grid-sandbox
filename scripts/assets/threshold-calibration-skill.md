---
id: threshold-calibration-mvp
name: Threshold Calibration Assistant (MVP verify)
description: Minimal SKILL.md fixture used by scripts/verify-v2-mvp.py — flat frontmatter so the cli-v2 simple YAML parser can extract id/name/version/author.
version: 0.1.0
author: eaasp-verify
---

# Threshold Calibration Assistant (verify fixture)

This file is a **verification fixture**, not the real skill. The real
`threshold-calibration` skill with nested `runtime_affinity` / `scoped_hooks`
lives in `examples/skills/threshold-calibration/SKILL.md`.

The `eaasp skill submit` CLI uses a deliberately minimal key-value YAML parser
(see `_parse_simple_yaml` in `tools/eaasp-cli-v2/src/eaasp_cli_v2/cmd_skill.py`)
that only understands `key: value` lines. The real skill's nested frontmatter
would cause `meta.get("id")` to return `None` and the CLI would fall back to
`path.stem == "SKILL"`, which is not what the 15-assertion verify gate expects.

This fixture's frontmatter is intentionally flat so the submitted skill row has
a predictable `id = threshold-calibration-mvp` + `version = 0.1.0` — those are
the values `scripts/verify-v2-mvp.py` uses to drive `skill promote` and the
subsequent L4 `session create --skill threshold-calibration-mvp`.

## Workflow

When invoked, this skill would analyze SCADA data from a Transformer and
suggest new temperature / load thresholds. The verify script does not execute
the skill — it only asserts that the skill can be submitted, promoted, and
referenced by a session.
