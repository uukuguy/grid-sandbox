# Vendored ADR Plugin Scripts

This directory holds a frozen copy of the ADR governance plugin scripts
from `~/.claude/skills/adr-governance/scripts/`. They are committed to the
repo so CI runners and collaborators without the global plugin can still
run `/adr:audit` and `/adr:review`.

## Files

- `scripts/*.py` — vendored scripts (do NOT edit here; edit upstream)
- `VERSION` — version string from the global plugin at vendor time
- `README.md` — this file

## Updating

After the global plugin updates, run:

    /adr:sync-scripts

or directly:

    python ~/.claude/skills/adr-governance/scripts/adr_sync.py

This re-copies the scripts and updates VERSION. Commit the diff.

## Do NOT

- Do NOT edit scripts here directly. Upstream is the source of truth.
- Do NOT delete this directory — CI depends on it.
