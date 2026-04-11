# eaasp-cli-v2

EAASP v2.0 CLI — Developer-facing command line for skill submission, session inspection, memory queries.

**Spec:** `docs/design/EAASP/EAASP_v2_0_MVP_SCOPE.md` §3.3
**Status:** Phase 0 skeleton (S3.T4 will implement)

## Responsibilities (Ring-2 MVP scope)

- `eaasp run <skill_id>` — end-to-end session kickoff through L4
- `eaasp session list/inspect` — surface SessionPayload and memory refs
- `eaasp memory query <user_id> <skill_id>` — read L2 memory engine
- `eaasp verify` — invoke eaasp-certifier against a runtime

## Out of scope (deferred)

- Interactive TUI
- Skill authoring scaffolding (reuse sdk/python/authoring for now)

## Stack

Python 3.12+, typer, rich, grpcio, pydantic v2.
