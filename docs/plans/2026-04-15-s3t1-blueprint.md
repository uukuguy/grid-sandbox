# S3.T1 Blueprint — PreCompact Hook 接入 (Gap-Closing Scope)

**Persisted**: 2026-04-15 (single source of truth, supersedes plan §S3.T1)
**Scout output**: 2026-04-15T00:00:00Z (Explore agent under swarm-1776204938811)
**Status**: APPROVED for execution

---

## User-Approved Decisions (2026-04-15, 7 questions)

| # | Question | Decision |
|---|---|---|
| 1 | Summary reuse depth | **Linear chain** — reuse only latest prior summary, not DAG |
| 2 | Task budget semantics | **Total-task budget**, decremented per turn, survives compaction |
| 3 | Proactive + reactive coexist | **Allow both** — proactive `summary_ratio=0.2` aggressive; reactive `summary_ratio=0.5` conservative |
| 4 | Head protection scope | **System + first user/asst pair only** (1 exchange) |
| 5 | SessionSummaryStore schema | **One row per session** (current upsert), history → Phase 3 |
| 6 | PRE_COMPACT hook | **Audit-only for MVP**; mutate protocol → ADR-V2-006 (S3.T5 prereq) |
| 7 | Summarizer model | **Session provider only** for MVP; cross-provider → Phase 3 |

## Cross-Cutting Amendment: Proto Schema (modified from "Rust-only")

Original proposal: Rust-only with proto field reserved for claude-code.
**Amendment** (after evidence audit): The Python implementation is no-op (claude-code-runtime delegates to Anthropic SDK), but **the proto schema must be uniform across languages** so:
- L3/L4 audit consumers see a typed `PreCompactHook` field (not generic JSON)
- Phase 2.5 goose-runtime can adopt the schema directly
- Cost: +20 min for `buf generate` + 4 pb2 diffs

**Action**: Edit `proto/eaasp/runtime/v2/hook.proto:46-55` adding `PreCompactHook pre_compact = 18;` oneof field. Regenerate Python `_pb2.py` / `_pb2.pyi` in:
- `tools/eaasp-l3-governance/`
- `tools/eaasp-l4-orchestration/`
- `lang/claude-code-runtime-python/`
- `lang/hermes-runtime-python/` (frozen runtime, but pb2 stays in sync)

**No Python implementation code changes** — only generated files.

## Companion ADR

ADR-V2-018 (new) — documents proto schema, iterative summary chain, cross-compaction budget. Cited as Phase 2.5 goose-runtime prerequisite.

---

## Executive Summary

**What exists (fully implemented)**:
- LLM-based `CompactionPipeline` (27.4K, 730 lines) with 9-section prompt, config, and PTL retry loop at `crates/grid-engine/src/context/compaction_pipeline.rs:1-730`
- Reactive call-site at `harness.rs:942-988` detecting `is_prompt_too_long()` and invoking pipeline
- Proto `PRE_COMPACT = 8` defined in `proto/eaasp/runtime/v2/runtime.proto`; Rust `HookPoint::ContextDegraded` exists at `hooks/mod.rs:35-36` but NOT yet a `PreCompact` variant
- `SessionSummaryStore` (episodic memory) at `memory/session_summary_store.rs` exists but NOT consumed by compaction pipeline (iterative reuse **not yet wired**)
- `TokenEscalation` (max_tokens upgrade) at `agent/token_escalation.rs` survives compaction boundary (correct)
- `ContextBudgetManager` with `update_actual_usage()` at `context/budget.rs:52-56` — actual token tracking infrastructure present

**What's partial (sketch exists, gaps remain)**:
- **Tail-protected split**: pipeline keeps last N **messages** (count-based, `keep_recent_messages: 6`), NOT token-based tail (~20K token window per plan)
- **Head protection**: no explicit head-protection logic (system prompt + first user/asst pair not marked/skipped)
- **Proactive threshold**: `CompactionPipelineConfig` has NO `proactive_threshold_pct` field — only reactive 413-on-PTL path exists
- **PRE_COMPACT event**: `HookPoint::ContextDegraded` fires at `harness.rs:732` **AFTER** compaction completes, not BEFORE (plan requires pre-event)
- **Summarizer model config**: `compact_model: Option<String>` exists, defaults to None (session model), but no per-session YAML binding
- **Iterative summary reuse**: `SessionSummaryStore` passed to pipeline context at `harness.rs:958` but pipeline never reads/writes it

**What's missing (true gaps)**:
- Config struct fields for proactive trigger (`proactive_threshold_pct`, `tail_protect_tokens`, `summary_ratio`, `summary_min/max`)
- Proactive check loop (threshold monitor before LLM call)
- Token-based tail protection (current: message count only)
- Head protection (system prompt + first exchange skip marker)
- PRE_COMPACT hook firing BEFORE summarizer LLM call
- Summary persistence & reuse bridge between compactions
- Cross-compaction task budget tracking (plan requires `taskBudgetRemaining` not reset on compact)

---

## Section 1. Compaction call-site mapping

### Active call sites (reactive-only, no proactive yet):

1. **harness.rs:942** — Main reactive 413 handler
   - Trigger: `is_prompt_too_long(&last_err)` AND `compact_attempts < 3`
   - Flow: error → pipeline.compact() → message rebuild → continue loop
   - Produces: `CompactionResult` with boundary_marker, summary_messages, kept_messages, reinjections, system_prompt_additions
   - Event: sends `AgentEvent::ContextCompacted { strategy: "llm_summary", pre_tokens, post_tokens }`
   - **No guard for "already attempted reactive once"** — loop allows up to 3 attempts per MAX_COMPACT_ATTEMPTS

2. **compaction_pipeline.rs:306** — PTL self-retry inside summarizer
   - Trigger: `is_prompt_too_long(&e)` during LLM call in `generate_summary()`
   - Flow: drops oldest 1/5 of to_summarize messages, retries (max `max_ptl_retries = 3`)
   - Produces: truncated summary or error after 3 failures

### Potential future call sites (not yet wired):
- Proactive check (would go before LLM call in main loop, ~line 700 estimate)
- SNIP marker processing at `compaction_pipeline.rs:513` (exists but not auto-triggered)

### Call graph summary:
```
harness.rs loop (L500+)
  ├─ stream LLM → error
  └─ is_prompt_too_long(error) [L105]
      └─ pipeline.compact(messages, provider, model, ctx) [L121]
          ├─ split at boundary = len - keep_recent [L128-135]
          ├─ preprocess_for_summary() [L213]
          ├─ generate_summary(provider, model, preprocessed, prompt) [L270]
          │   └─ PTL self-retry: drop oldest 1/5, retry up to 3x [L306-320]
          └─ rebuild_state(ctx) → fire SessionStart hooks [L370, L450]
```

---

## Section 2. PRE_COMPACT event wiring

### Proto definition:
- **runtime.proto**: `HookEventType::PRE_COMPACT = 8` defined (line ~180 in enum)
- **hook.proto**: NO oneof field for PreCompactHook in message HookEvent; hook.proto only has PreToolCallHook, PostToolResultHook, StopHook, etc.

### Rust hook enum:
- **hooks/mod.rs:22-57**: `HookPoint` enum has NO `PreCompact` variant
  - Closest: `ContextDegraded = 35-36` (comment says "replaces PreCompact") but is POST-compaction
  - Fired at: `harness.rs:732` **AFTER** compaction completes (at line 988), NOT before

### Current event sequence (WRONG per plan):
1. Loop detects PTL error (L942)
2. Invokes pipeline.compact() (L964)
3. **Summarizer LLM call happens** (L159)
4. Rebuild state + SessionStart hooks fired (L450) ← only hook point here
5. `ContextDegraded` event sent & hook fired (L732) ← AFTER all compaction done
6. Loop continues (L987)

### Plan requirement (§S3.T1):
- PRE_COMPACT event should fire **BEFORE** summarizer LLM is called (before L159)
- Allows L4 orchestrator to log/audit/mutate prompt before compression happens

### Current gap:
- NO hook fired before `generate_summary()` call
- `HookPoint::ContextDegraded` is misnomer — it's ContextCompacted (POST event)
- Proto has `PRE_COMPACT = 8` but Rust never emits it

---

## Section 3. Reactive (413 / context overflow) path audit

### Where is_prompt_too_long consumed:
- **harness.rs:105** — detection function (checks body for 400/prompt_too_long/context_length_exceeded)
- **harness.rs:942** — main trigger in loop error handler
- **compaction_pipeline.rs:306** — PTL self-retry trigger inside summarizer

### Reactive guard status:
- ❌ NO `hasAttemptedReactiveCompact` or equivalent
- ✅ BUT: `compact_attempts` counter exists at harness.rs:941-943, max = 3
- ✅ `MAX_COMPACT_ATTEMPTS = 3` hardcoded at harness.rs:115
- Risk: if summarizer itself 413s, we retry with smaller input (auto self-correcting), not a death spiral

### Summarizer PTL handling:
- ✅ `max_ptl_retries = 3` in config (default, compaction_pipeline.rs:47)
- ✅ On each PTL: drops oldest 1/5 of messages, retries (L307-317)
- ✅ After 3 failures: returns error, harness.rs:989-1000 falls back to auto_snip

### Error classifier integration:
- ✅ `FailoverReason::ContextOverflow` at error_classifier.rs:35
- ✅ Maps to `RecoveryActions { retryable: true, should_compress: true, ... }` at error_classifier.rs:178-183
- ✅ `FailoverReason::PayloadTooLarge` (distinct from ContextOverflow) maps similar
- **Gap**: harness.rs does NOT consult error_classifier for compaction decision — uses `is_prompt_too_long()` string match instead

---

## Section 4. Proactive (≥75% threshold) path audit

### ContextBudgetManager capabilities:
- ✅ `estimate_tokens(text: &str) -> u32` at budget.rs:59-61 (chars/4 heuristic)
- ✅ `estimate_messages_tokens(messages: &[ChatMessage]) -> u64` at budget.rs:64-80
- ✅ `update_actual_usage(input_tokens, msg_count)` at budget.rs:52-56 (stores last actual usage)
- ✅ `context_window: u32` field at budget.rs:25
- ❌ NO `usage_pct()` or `tokens_remaining()` helper method
- ❌ NO public getter for `last_actual_usage` — private field

### Proactive trigger:
- ❌ NO proactive check loop in harness.rs main loop
- ❌ NO `proactive_threshold_pct` field in `CompactionPipelineConfig`
- ❌ NO timer-based or threshold-based proactive firing
- Plan requires check at ~75% before LLM call; currently ZERO implementation

### Config field absence:
```rust
// Plan §S3.T1 specifies these config fields:
proactive_threshold_pct: 75
tail_protect_tokens: 20000
summary_ratio: 0.2
summary_min: 2000
summary_max: 12000
summarizer_model: gpt-4o-mini
reactive_only: false

// Current CompactionPipelineConfig has:
compact_model: Option<String>       // ✅ covers summarizer_model
summary_max_tokens: u32            // ✅ partial (only max, no min/ratio)
keep_recent_messages: usize        // ❌ message-count, not token-based
max_ptl_retries: u32               // ✅ covers PTL retry count
// ❌ MISSING: proactive_threshold_pct, tail_protect_tokens, summary_ratio, reactive_only
```

---

## Section 5. Tail-protection audit

### Current split logic (compaction_pipeline.rs:128-135):
```rust
let keep_count = self.config.keep_recent_messages;  // ← message count, not tokens
let boundary = messages.len().saturating_sub(keep_count);
let to_summarize = &messages[..boundary];
let to_keep = &messages[boundary..];
```
- **Message-count based**: keeps last 6 messages verbatim
- **NO token accounting** for tail: doesn't ensure ~20K token window as plan specifies

### Current preprocessing (compaction_pipeline.rs:213-264):
- ✅ Images → lightweight "[image]" placeholder
- ✅ Long tool results → truncate to 2000 chars with "[truncated, N chars total]" marker
- ❌ NO head protection: system prompt and first user/assistant pair NOT skipped in `to_summarize`

### Audit findings:
- **to_summarize** (sent to summarizer): messages[0..boundary] — includes system-like content if present
- **to_keep** (kept verbatim): messages[boundary..] — last 6 messages, no token budget logic
- Plan requires:
  - Head: skip system prompt + first (user, assistant) pair from summarization
  - Tail: token-based ~20K window (not message count)
  - Middle: everything else gets summarized

### Missing implementation:
- `fn find_head_boundary()` to identify system + first exchange
- `fn estimate_tail_tokens()` to find boundary ensuring ~20K remaining
- Update `CompactionPipelineConfig` with `tail_protect_tokens: usize`

---

## Section 6. Iterative summary reuse

### SessionSummaryStore present:
- ✅ Location: `memory/session_summary_store.rs:1-200`
- ✅ API: `save(session_id, summary, event_count, topics, memory_count) -> Result`
- ✅ API: `recent(limit) -> Result<Vec<SessionSummary>>`
- ✅ Storage: SQLite table `session_summaries(session_id, summary, event_count, ...)`

### Current pipeline integration:
- ✅ `CompactionContext` has `pub session_summary_store: Option<Arc<SessionSummaryStore>>` at compaction_pipeline.rs:87
- ✅ Passed from harness.rs:958 when invoking compact()
- ❌ **NEVER READ** in `compact()` implementation
- ❌ **NEVER WRITTEN** after compaction completes

### Plan requirement:
- On first compaction: summarize from scratch, save to SessionSummaryStore
- On second compaction in same session: fetch prior summary from store, **include as input to next summarizer call**
- "下次压缩时复用上次 summary，不是从头总结" (reuse previous summary, don't re-summarize from scratch)

### Current behavior (FALSE per plan):
- Every compaction starts fresh from raw messages
- No cross-compaction summary reuse
- SessionSummaryStore sits unused in the context

### Missing implementation:
- `SessionSummaryStore::get_latest(session_id) -> Result<Option<SessionSummary>>` method
- In `generate_summary()`: fetch prior summary, prepend to `to_summarize` messages
- After `compact()` success: call `session_summary_store.save(...)` with new summary + metrics

---

## Section 7. Cross-compaction token budget

### TokenEscalation (AR-T1):
- ✅ Location: `agent/token_escalation.rs:1-100`
- ✅ Tiers: [4096, 8192, 16384, 32768, 65536]
- ✅ Created at harness.rs:421
- ✅ Escalates on MaxTokens error, survives loop iterations
- ⚠️ **Resets per turn at harness.rs:2200** (inside loop, not across compaction)

### Actual token tracking:
- ✅ `ContextBudgetManager::update_actual_usage(input_tokens, msg_count)` at budget.rs:52-56
- ✅ Called when API response received (implicit in usage tracking, not shown in grep)
- ❌ NO `finalContextTokensFromLastResponse()` equivalent
- ❌ Task budget (`taskBudgetRemaining` in plan) — **NOT FOUND** in codebase

### Plan requirement (§S3.T1, 跨压缩 token 预算):
- `finalContextTokensFromLastResponse()` should read from `usage.iterations[-1]` (last turn's actual context tokens)
- `taskBudgetRemaining` is loop-local, **does NOT reset on compaction**
- Compaction reduces messages but shouldn't reset the remaining task budget

### Current state:
- ❌ NO `taskBudgetRemaining` tracking in harness loop
- ❌ NO explicit "don't reset budget on compaction" logic
- ⚠️ TokenEscalation escalates per turn, could use up budget before compaction even runs

### Gap summary:
- Token budget is **not persistent across compaction boundary** in way plan describes
- No mechanism to track "how many tokens can this remaining task still use"

---

## Section 8. Summarizer model config

### Current structure:
```rust
pub struct CompactionPipelineConfig {
    pub compact_model: Option<String>,  // ← model override
    pub summary_max_tokens: u32,
    pub keep_recent_messages: usize,
    pub max_ptl_retries: u32,
}

// Default
compact_model: None,  // ← defaults to session model
summary_max_tokens: 2000,
keep_recent_messages: 6,
max_ptl_retries: 3,
```

### Model resolution (compaction_pipeline.rs:157):
```rust
let compact_model = self.config.compact_model.as_deref().unwrap_or(model);
```
- If `compact_model` is set, use it; else use session model

### Where is it set?
- ❌ NO YAML config binding found
- ❌ NO `AgentLoopConfig` field for compaction config
- ❌ Hardcoded at harness.rs via inline `CompactionPipelineConfig::default()`

### Plan config (§S3.T1):
```yaml
context_compression:
  summarizer_model: gpt-4o-mini  # or anthropic haiku
```

### Gap:
- NO way to override `compact_model` from session/agent YAML config
- Always defaults to session model (usually Claude)
- Plan specifies "haiku / gpt-4o-mini" (cheaper models for summarization)

---

## Section 9. Python mirror (claude-code-runtime)

### Checked files:
- `lang/claude-code-runtime-python/src/claude_code_runtime/`: 13 .py modules found
- Grep for "compact" / "compaction": NO matches in any .py file
- Summary: **claude-code-runtime proxies to Anthropic's official SDK** (claude-agent-sdk subprocess)

### Implications:
- claude-code-runtime does NOT implement its own compaction logic
- Relies on Anthropic's native prompt caching + API-level token limits
- **Plan scope (§S3.T1 涉及文件): Rust-only** — no Python work needed
- If claude-code-runtime hits context limit, it's API-level, not runtime-level

---

## Section 10. Test coverage of current state

### Existing tests:
- **compaction_pipeline.rs**: 8 test functions (L107-262)
  - `test_compact_basic_flow()` — full happy path
  - `test_compact_too_few_messages()` — boundary condition
  - `test_compact_ptl_all_retries_fail()` — PTL exhaustion
  - `test_compact_with_custom_instructions()` — prompt customization
  - `test_compact_message_reassembly()` — output reassembly
  - 3 unit tests for `format_summary()` helper

- **auto_compact.rs**: test file exists (referenced in file list)

### Untested behaviors (gaps):
- ❌ Proactive trigger (threshold check before LLM call)
- ❌ Iterative summary reuse (read prior summary from store)
- ❌ Reactive guard (hasAttemptedReactiveCompact equivalent)
- ❌ Cross-compaction budget survival
- ❌ Tail-protected split (token-based window)
- ❌ Head protection (system prompt skip)
- ❌ PRE_COMPACT hook firing (before summarizer LLM)
- ❌ Error classifier integration (ContextOverflow → should_compress routing)
- ❌ Summarizer model config from YAML

---

## Revised Delta Task List (replaces plan §S3.T1 実施内容)

### T1.A — CompactionPipelineConfig extension + YAML binding
**Concrete task**: Add config fields for proactive trigger, tail protection, summary scaling; bind to AgentLoopConfig.
- **Files**: 
  - `crates/grid-engine/src/context/compaction_pipeline.rs` — config struct (add 5 fields)
  - `crates/grid-engine/src/agent/loop_config.rs` — AgentLoopConfig (add compaction_config field)
  - `crates/grid-engine/src/agent/harness.rs` — instantiation (read from loop_config)
- **LOC estimate**: small (50 lines config, 30 lines wiring)
- **Depends on**: none

### T1.B — Token-based tail protection + head protection split
**Concrete task**: Replace message-count-based `keep_recent_messages` with token-aware split; skip system prompt + first exchange from summarization.
- **Files**:
  - `crates/grid-engine/src/context/compaction_pipeline.rs` — `compact()` method (L128-135), add `find_head_boundary()` + `find_tail_boundary()` helpers
- **LOC estimate**: medium (80 lines new logic)
- **Depends on**: T1.A (needs `tail_protect_tokens`, `summary_ratio` config fields)

### T1.C — Proactive threshold monitoring + preemptive compaction trigger
**Concrete task**: Before LLM call in main loop, check context usage % against `proactive_threshold_pct`; if exceeded, trigger compaction without waiting for 413.
- **Files**:
  - `crates/grid-engine/src/agent/harness.rs` — main loop (L700 vicinity), add threshold check
  - `crates/grid-engine/src/context/budget.rs` — add `usage_pct()` helper
- **LOC estimate**: small (40 lines threshold logic + helper)
- **Depends on**: T1.A (needs `proactive_threshold_pct` config)

### T1.D — PRE_COMPACT hook (before summarizer LLM)
**Concrete task**: Add `HookPoint::PreCompact` enum variant; fire hook before `generate_summary()` call; adjust proto hook.proto if needed.
- **Files**:
  - `crates/grid-engine/src/hooks/mod.rs` — add PreCompact variant
  - `crates/grid-engine/src/context/compaction_pipeline.rs` — fire hook before L159
  - `proto/eaasp/runtime/v2/hook.proto` — add PreCompactHook oneof if missing
- **LOC estimate**: small (20 lines code + 10 lines proto)
- **Depends on**: none (independent)

### T1.E — SessionSummaryStore integration (iterative reuse)
**Concrete task**: After successful compaction, save summary to store; on next compaction, prepend prior summary to input messages.
- **Files**:
  - `crates/grid-engine/src/memory/session_summary_store.rs` — add `get_latest(session_id)` method
  - `crates/grid-engine/src/context/compaction_pipeline.rs` — `compact()` call `session_summary_store.get_latest()` before `generate_summary()`, prepend result; after success, call `.save()`
- **LOC estimate**: medium (60 lines methods + prompt injection)
- **Depends on**: T1.A (session_id available in CompactionContext), T1.B (summary messaging)

### T1.F — Cross-compaction budget tracking
**Concrete task**: Introduce `task_budget_remaining` loop variable (not reset on compaction); update via actual token usage from API responses.
- **Files**:
  - `crates/grid-engine/src/agent/harness.rs` — main loop (L160+), add `task_budget_remaining` var, update on each API response, preserve across compaction boundary
  - `crates/grid-engine/src/context/budget.rs` — add helper to read actual usage
- **LOC estimate**: small (30 lines tracking)
- **Depends on**: none (new tracking, orthogonal)

### T1.G — Error classifier integration + reactive guard
**Concrete task**: Consult `error_classifier::FailoverReason` for compaction decisions; add `attempted_reactive_compact` flag to prevent repeated retries.
- **Files**:
  - `crates/grid-engine/src/agent/harness.rs` — L942 area, check `Failover::should_compress`, set `attempted_reactive_compact` guard
- **LOC estimate**: small (25 lines)
- **Depends on**: none (error_classifier already exists)

### Summary of deltas:
- **T1.A**: config extension (blocking for T1.B, T1.C, T1.E)
- **T1.B**: tail-protection logic (blocking for T1.E)
- **T1.C**: proactive check (independent, can be done in parallel with T1.A)
- **T1.D**: hook wiring (independent)
- **T1.E**: summary reuse (depends on T1.A, T1.B)
- **T1.F**: budget tracking (independent)
- **T1.G**: guard logic (independent)

**Dependency order**: A → {B, C, D, E, F, G}, B → E

---

## Modules to NOT create (would duplicate existing code)

- ❌ `context_compressor.rs` — CompactionPipeline already serves this role
- ❌ `SessionSummary` type — SessionSummaryStore already defines it
- ❌ `FailoverReason` classifier — error_classifier.rs already has it

---

## Modules/functions to extend

- **compaction_pipeline.rs**: 
  - `CompactionPipelineConfig` struct (add 5 config fields)
  - `compact()` method (integrate prior summary fetch, fire PRE_COMPACT hook, implement token-based split)
  - Add `find_head_boundary()`, `find_tail_boundary()` helpers
  
- **session_summary_store.rs**:
  - Add `get_latest(session_id) -> Result<Option<SessionSummary>>` method
  - Add `delete_old(before_timestamp)` for cleanup (nice-to-have)

- **harness.rs**:
  - Main loop (add proactive threshold check, cross-compaction budget tracking, reactive guard)
  - CompactionPipelineConfig instantiation (read from AgentLoopConfig)

- **budget.rs**:
  - Add `usage_pct()` helper to ContextBudgetManager
  - Add public getter for `last_actual_usage`

- **hooks/mod.rs**:
  - Add `PreCompact` variant to HookPoint enum

- **loop_config.rs**:
  - Add `compaction_config: Option<CompactionPipelineConfig>` field to AgentLoopConfig

---

## Test plan

### New test coverage to add:

1. **test_proactive_trigger** (harness integration test)
   - Setup: ContextBudgetManager with 100K context window, fill 80%
   - Verify: proactive compaction triggers before 413 error occurs
   - Assert: messages compacted, loop continues, no PTL attempt

2. **test_tail_protection_tokens** (compaction_pipeline.rs)
   - Setup: 20 messages totaling 100K tokens, tail_protect_tokens = 20000, summary_ratio = 0.2
   - Verify: head boundary found (system + first exchange), tail = ~20K tokens preserved, middle summarized
   - Assert: post_tokens < pre_tokens, tail messages verbatim in kept_messages

3. **test_head_protection_system_prompt** (compaction_pipeline.rs)
   - Setup: messages with system prompt prefix
   - Verify: system prompt NOT sent to summarizer (in to_summarize)
   - Assert: formatted summary is clean, no system prompt leaked

4. **test_iterative_summary_reuse** (compaction_pipeline.rs + session_summary_store integration)
   - Setup: first compaction, save summary; second compaction in same session
   - Verify: prior summary fetched from store, prepended to generate_summary input
   - Assert: second summary mentions "based on prior context" or similar

5. **test_pre_compact_hook_fires_before_lm** (harness integration)
   - Setup: mock hook that logs execution, CompactionPipeline ready
   - Trigger: PTL error
   - Verify: hook fired with `HookPoint::PreCompact` before summarizer LLM call
   - Assert: log shows hook timestamp < summarizer call timestamp

6. **test_reactive_guard_prevents_double_compact** (harness integration)
   - Setup: mock provider that always returns PTL
   - Verify: after first compact attempt fails (summarizer also 413s), second attempt not made
   - Assert: loop terminates with error after MAX_COMPACT_ATTEMPTS, not infinite retry

7. **test_cross_compaction_budget_survives** (harness integration)
   - Setup: task_budget_remaining = 10K tokens, consume 5K in first turn, compact, continue
   - Verify: task_budget_remaining stays at ~5K after compact (not reset)
   - Assert: loop can track remaining budget across compaction boundary

### Behaviors currently untested (leave for manual E2E):
- Proactive + reactive together (threshold triggers preemptive, then 413 triggers reactive)
- Summarizer model override from config
- SessionSummaryStore cross-session injection (requires multi-session setup)

---

## Risks & Open Questions

### Architectural decisions needing human input:

1. **Summary reuse collision**: If a session has 10 compactions, do we reuse the most recent summary or all prior summaries? Plan says "下次压缩时复用上次 summary" (reuse **previous** summary), implying 1:1 chain, not DAG. Confirm: linear reuse, not DAG?

2. **Token budget semantics**: Plan says `taskBudgetRemaining` is "loop-local" and "not reset on compaction". Does this mean:
   - (a) Total tokens available for entire task, decremented per turn, survives compaction?
   - (b) Tokens available in **this** loop iteration only, reset per turn, survives compaction of messages?
   - Recommend clarifying in ADR before implementation.

3. **Proactive vs. reactive interaction**: If proactive triggers at 75% and compacts, then (by chance) next turn hits 413 anyway:
   - Do we allow a **second** reactive compaction, or is the `reactive_only: false` config flag enough?
   - Recommend: allow both, but different strategies (proactive = aggressive ratio=0.2, reactive = ratio=0.5)?

4. **Head protection scope**: Should "first exchange" skip include ONLY the first user/assistant pair, or system + first N exchanges?
   - Plan says "system + 第一组 user/assistant"（system + first pair）— confirm singular pair?

5. **SessionSummaryStore table schema**: Current schema stores one summary per session_id (upsert on conflict). For iterative reuse to work, do we need:
   - (a) One row per session (current), reuse the latest?
   - (b) One row per (session_id, compaction_round) to track evolution?
   - Recommend: (a) simpler, align with "reuse previous" semantics.

6. **PRE_COMPACT hook decision mutations**: Can the hook MUTATE the prompt or messages before summarizer LLM, or just log/audit?
   - If mutate: need to thread decision back into `generate_summary()` call
   - Recommend: POST hook for audit, PRE hook is log-only for now (mutate added later)

7. **Summarizer model choice**: Plan suggests "haiku / gpt-4o-mini" for cheaper summarization. But session might be using Claude API. Should we:
   - (a) Allow cross-provider summarizer (use OpenAI API key for summary even if session is Claude)?
   - (b) Stick to session provider for summary (no extra credentials needed)?
   - Recommend: (b) for MVP, (a) as Phase 3 enhancement.

### Risks:

- **Risk**: PTL retry inside generate_summary() drops oldest messages without checking if they're important (head-protected). Mitigate: ensure head boundary is found first, don't drop from head.
- **Risk**: SessionSummaryStore upsert-on-conflict means old summaries lost. If we want to keep history for analysis, need schema change. Mitigate: accept for MVP, add retention policy in Phase 3.
- **Risk**: Cross-provider summarizer needs credential rotation. If OpenAI key runs out, summarization fails but session (Claude) could continue. Mitigate: graceful fallback to session model if summarizer fails.

---

## Implementation order (recommended)

1. **T1.A** (config) — unblocks everything
2. **T1.D** (hook) — independent, quick win
3. **T1.F** (budget) — independent, foundational
4. **T1.B** (tail-protection) — depends on T1.A, enables T1.E
5. **T1.C** (proactive) — depends on T1.A, small
6. **T1.E** (reuse) — depends on T1.A + T1.B
7. **T1.G** (guard) — independent, small

**Total LOC**: ~300 lines across 7 deltas
**Duration estimate**: 4-6 hours for careful implementation + test coverage

