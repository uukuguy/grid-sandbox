# eaasp-l3-governance

EAASP v2.0 L3 Governance Plane — Policy compilation, approval workflow, audit trail.

**Spec:** `docs/design/EAASP/EAASP_v2_0_MVP_SCOPE.md` §3.3
**Status:** Phase 0 skeleton (S3.T2 will implement)

## Responsibilities (Ring-2 MVP scope)

- Compile declarative policy → ManagedHook set attached to `PolicyContext`
- Resolve `deny-always-wins` precedence between managed / frontmatter / user scopes
- Emit `PRE_POLICY_DEPLOY` hook event before any policy change is effective
- Expose REST API consumed by L4 Orchestration plane

## Out of scope (deferred to Phase 1+)

- Event stream backend (ADR-V2-002)
- Cross-tenant policy inheritance
- Approval workflow UI

## Stack

Python 3.12+, FastAPI, pydantic v2.
