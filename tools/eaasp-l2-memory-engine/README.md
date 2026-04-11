# eaasp-l2-memory-engine

EAASP v2.0 L2 Memory Engine — Cross-session memory accumulation and retrieval.

**Spec:** `docs/design/EAASP/EAASP_v2_0_MVP_SCOPE.md` §3.3
**Status:** Phase 0 skeleton (S3.T1 will implement)

## Responsibilities (Ring-2 MVP scope)

- Persist `MemoryRef` rows keyed by `(user_id, skill_id, memory_type)`
- On session start: query top-N relevant MemoryRefs; attach to `SessionPayload.memory_refs` (P3)
- On session end: extract new memory candidates from runtime telemetry; persist
- Supply the "threshold calibration assistant" E2E scenario: thresholds from run N visible to run N+1

## Out of scope (deferred)

- HNSW index (reuse grid-engine's existing path or stub)
- Memory compression / eviction policy
- Cross-user memory sharing

## Stack

Python 3.12+, FastAPI, sqlite via aiosqlite, pydantic v2.
