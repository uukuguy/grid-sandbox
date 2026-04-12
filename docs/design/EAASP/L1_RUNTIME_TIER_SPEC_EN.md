# EAASP L1 Runtime Tier Classification

> **Purpose**: Replacement chapter for v2.0 Design Specification §8.5
> **Last updated**: 2026-04-12 (post R1-R4 source verification)

---

## 8.5.1 Tier Definitions

The L1 Runtime Pool classifies candidate runtimes into four tiers based on **adapter thickness** — the engineering effort required to wrap a runtime into a compliant EAASP L1 instance. The goal is ecosystem coverage, not ranking: each tier represents a different team background and onboarding path.

### T0 — Harness-Tools Container Isolation

**Definition**: The agent body (harness) and the tools execution environment (container / VM / remote sandbox) are physically separated, communicating via a decoupled protocol (Computer Protocol / Sandbox API / RPC). Credentials and governance policies are injected through the protocol layer, not embedded in either side.

**Distinguishing features**:

| Feature | Description |
|---------|-------------|
| Process-container separation | Harness and tools reside in different processes, containers, or machines |
| Protocol-layer decoupling | The protocol layer is the decoupling mechanism, not a shared library |
| Tools replaceability | Tool containers can be replaced without affecting the harness |

**Use case**: Cross-trust-domain deployments ("agent in the cloud, tools in the customer's intranet") and production requirements for independent scaling, isolation, and replacement of tool containers.

**Representative projects**:

| Project | Language | Verification | Notes |
|---------|----------|-------------|-------|
| **HexAgent** | Python | R4 source-verified (2026-04-12) | Computer Protocol with 6 methods; LocalNative / Lima VM / E2B Cloud adapters. Adapter: 5-8 days |
| Anthropic Computer Use | Commercial | Web research | Same conceptual origin, closed-source |
| E2B.dev | Python/TS | Known ecosystem | Cloud sandbox as remote tool container |

**Current status**: T0 not delivered. Excluded from Phase 0.

---

### T1 — Complete Triad + Thin Adapter

**Definition**: The runtime natively provides the **MCP Client + Skills (Markdown + YAML frontmatter) + Hooks (PreToolUse / PostToolUse)** triad, and the triad directly aligns with EAASP specification requirements. The adapter is thin — protocol forwarding only.

**Triad qualification matrix**:

| Dimension | T1 Requirement |
|-----------|---------------|
| **MCP** | Native MCP Client (stdio + SSE minimum). Must consume `SessionPayload.mcp_servers` 5-block |
| **Skills** | Markdown + YAML frontmatter format with `name / description / version / allowed-tools` fields. Must losslessly load or map EAASP Skill v2 extension fields (`runtime_affinity / access_scope / scoped_hooks / dependencies`) |
| **Hooks** | Per-tool-call PreToolUse / PostToolUse interception points. Return semantics must map to `{Allow / Deny / Modify}` ternary decision |

**Adapter thickness**: 1-4 days (protocol wrapping only).

**Representative projects**:

| Project | Language | Status | Adapter |
|---------|----------|--------|---------|
| **claude-code-runtime** | Python | Delivered, production-ready | Done |
| **hermes-runtime** | Python | Delivered, production-ready | Done |
| **OpenCode** | TypeScript | R1 source-verified T1 (2026-04-12) | 3-4 days |
| CCB | TypeScript/Bun | Source read, pending formal evaluation | TBD |
| claw-code | Rust | Source read, strict hook alignment but no skill/server | Possibly T2 |

**Current status**: 2 instances delivered (both Python). TypeScript instance (OpenCode) verified, adapter not yet built.

---

### T2 — Agent Framework, Partially Incomplete

**Definition**: The runtime is a complete agent framework, but at least one item in the MCP / Skills / Hooks triad is incomplete or misaligned with the EAASP specification. The adapter must fill in the missing parts and perform mapping transformations.

**Adapter thickness**: 3-7 days (protocol wrapping + dimension completion).

**Typical incompleteness patterns**:

| Missing dimension | Typical representative | Adapter must provide |
|-------------------|----------------------|---------------------|
| No Skill manifest (recipe / code registration only) | Goose | Markdown frontmatter-to-Recipe mapping layer |
| Hook granularity mismatch (batch / run-level, not per-tool) | Nanobot, Agno 2.0 | Split / aggregate to per-tool hooks |
| Thin server layer | Nanobot | FastAPI / gRPC server layer |
| No MCP (code-only tools) | Some frameworks | MCP client adaptation layer |

**Representative projects**:

| Project | Language | Incomplete dimension | Status |
|---------|----------|---------------------|--------|
| **Agno 2.0** | Python | Hooks: agent-run level, not per-tool | R2 source-verified T2 (2026-04-12). Adapter: 5-7 days |
| **Goose** | Rust | Skills: Recipe, not Markdown frontmatter | Pending deep verification |
| **Nanobot** | Python | Hooks: batch-level + thin server layer | Pending deep verification |

**Current status**: T2 not delivered. Agno tier confirmed via source verification.

---

### T3 — Legacy AI Framework

**Definition**: Fundamentally lacks the concept of an "agent runtime" — it is a Python / TS library. The agent abstraction is a graph node (LangGraph) / crew (CrewAI) / conversation (AutoGen) / decorator function (Pydantic AI). Typically no MCP; hook semantics are misaligned (graph-node-level / conversation-level / absent).

**Adapter thickness**: 1-3 weeks (build per-tool interception + MCP adaptation + skill loader + session management).

**Classification detail**: The "agent" concept in these frameworks has a semantic mismatch with the EAASP "session + tool + hook + skill" model — forcing adaptation generates significant impedance mismatch across the 16-method gRPC contract. A T3 candidate is worth building as an L1 only to provide an onboarding path for teams with existing framework assets, not to select the best L1.

**Representative projects**:

| Project | Language | Difficulty | Notes |
|---------|----------|-----------|-------|
| **Pydantic AI** | Python | Medium | Cleanest hooks in T3 (decorator function filter) + native MCP |
| **Semantic Kernel** | .NET/Python | Medium | Function Invocation Filter + native MCP (2025-03) |
| **LangGraph** | Python | High | LangGraph Platform GA + MCP (2025-07). Graph-node-level hook mismatch |
| CrewAI | Python | High | Static workflow, hook semantic mismatch |
| AutoGen | Python | High | Conversation-level, hooks absent |
| Google ADK | Python | Not recommended | `before_tool_callback` exists but live path bug |

**Current status**: T3 not delivered. No source-code verification conducted.

---

## 8.5.2 T1/T2 Watershed

The T1/T2 watershed is not "whether hooks exist" — all mainstream runtimes in 2025-2026 have hooks — but the **per-tool hook granularity** and **alignment completeness of the triad**.

| Criterion | T1 | T2 |
|-----------|----|----|
| Hook trigger granularity | Fires on every tool call (per-tool) | Fires per agent run or batch-level |
| Hook params include tool context | `tool_name`, `tool_args`, `tool_call_id` available | Run-level context only |
| Ternary decision coverage | Allow + Deny + Modify all reachable (even via combined systems) | At least one missing or requiring invasive modification |
| Adapter effort | 1-4 days | 3-7 days |

**Source-verified examples**:

- **OpenCode (T1)**: `tool.execute.before/after` = per-tool hook; Permission system = Allow/Deny/Ask. Combined systems cover ternary decision; adapter only needs bridging.
- **Agno 2.0 (T2)**: `pre_hooks/post_hooks` = agent-run level, signature lacks `tool_name`. Requires invasive injection of interception points in `_run.py` tool-call loop (2-3 days).

---

## 8.5.3 Tier Summary

| Tier | Adapter | Triad requirement | Delivered | Source-verified |
|------|---------|-------------------|-----------|----------------|
| T0 | Protocol layer | N/A (separation architecture) | None | HexAgent (R4) |
| T1 | 1-4 days | Complete triad alignment | claude-code-runtime, hermes-runtime | OpenCode (R1) |
| T2 | 3-7 days | At least one item incomplete | None | Agno 2.0 (R2) |
| T3 | 1-3 weeks | Semantic mismatch, heavy adaptation | None | None |
