# eaasp-l4-orchestration

EAASP v2.0 L4 Orchestration Plane — AgentOS event room + four-card dispatch.

**Spec:** `docs/design/EAASP/EAASP_v2_0_MVP_SCOPE.md` §3.3
**Status:** Phase 0 skeleton (S3.T3 will implement)

## Responsibilities (Ring-2 MVP scope)

- Accept session-create request; resolve PolicyContext (via L3) + UserPreferences + SkillInstructions
- Build structured `SessionPayload` (P1-P5 priority blocks)
- Dispatch to selected L1 runtime via RuntimeSelector (grid-runtime / hermes)
- Receive telemetry + memory refs on session end; hand to L2 memory engine

## Out of scope (deferred to Phase 1+)

- Multi-session event fan-out (ADR-V2-003)
- Session clustering
- Multi-tenant isolation beyond per-session

## Stack

Python 3.12+, FastAPI, grpcio, pydantic v2.
