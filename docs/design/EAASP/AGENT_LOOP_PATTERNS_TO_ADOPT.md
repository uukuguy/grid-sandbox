# Agent Loop 可吸收优点清单（从 4 个开源 runtime 提取）

**创建日期**: 2026-04-14
**基于调研**: hermes-agent / claude-code (CCB) / goose / agno
**关联**: `AGENT_LOOP_ROOT_CAUSE_ANALYSIS.md`、Phase 2 Plan

---

## 概述

在 D87 根因调研过程中，对 4 个开源 agent loop 做了 Explore 反查。本文档汇总**除了"多轮继续"本身之外**值得 grid-engine 吸收的设计细节，按价值和落地时机分三批次。

---

## Tier 1：立刻应该做（Phase 2 S1 / S2）

### 1. Tool result 三层截断 + 溢出落盘 ⭐⭐⭐

**来源**: hermes `tool_result_storage.py:1-80`

**三层防御**：
- Per-tool cap：工具自身预截断
- Per-result：超阈值 spill 到 `/tmp/hermes-results/{tool_use_id}.txt`，只喂 preview + 文件引用给 LLM
- Per-turn aggregate：所有 tool_result 总和 > 200K 时，从最大的开始继续 spill 到 under budget

**grid-engine 现状**：无 tool 输出预算控制，单次大 result 能爆上下文。

**吸收**：新建 `crates/grid-engine/src/agent/tool_result_storage.rs`，在 tool 执行完、生成 tool_result ChatMessage 前过一道预算检查。落盘目录复用 OctoRoot。

**建议落地**: Phase 2 **S2.T5**（新任务，L2 memory 增强时一起做，因为溢出的文件天然是 memory 的候选）

---

### 2. ErrorClassifier + 恢复动作矩阵 ⭐⭐⭐

**来源**: hermes `error_classifier.py:25-80`

**核心设计**：
```rust
enum FailoverReason {
    Auth, AuthPermanent, Billing, RateLimit, Overloaded,
    ServerError, Timeout, ContextOverflow, PayloadTooLarge,
    ModelNotFound, FormatError, ThinkingSignature,
    LongContextTier, Unknown,
}

struct RecoveryActions {
    retryable: bool,
    should_compress: bool,
    should_rotate_credential: bool,
    should_fallback: bool,
}
```

每个错误独立映射到 4 个动作布尔。配合 `withRetry` 使用。

**grid-engine 现状**：错误处理散在 harness.rs 各处，字符串匹配 + 临时分支。

**吸收**：新建 `crates/grid-engine/src/providers/error_classifier.rs`，所有 provider 层 error 经过此 classifier。

**建议落地**: Phase 2 **S1.T6**（新任务，与 D87 修复同批次，把错误处理一起现代化）

---

### 3. Graduated retry with backoff ⭐⭐⭐

**来源**: claude-code `CCB/src/services/api/withRetry.ts:50-100`

**规则**：
- 429 rate limit → 指数退避 + rotate credential（1 hr cooldown）
- 529 overloaded → 跳过 non-foreground sources
- auth → clear cache + OAuth refresh
- 配合 ErrorClassifier 使用

**grid-engine 现状**：重试逻辑不成体系。

**吸收**：新建 `crates/grid-engine/src/providers/retry.rs`，wrap 所有 provider call。

**建议落地**: Phase 2 **S1.T7**（和 S1.T6 同批次）

---

### 4. Stop hooks（loop 结束前的扩展点）⭐⭐

**来源**: claude-code `CCB/src/query/stopHooks.ts:65-150`

**设计精妙之处**：
- 在 loop 完成、return 之前跑一批 hook
- 每个 hook 可以往 messages 里 push 东西、让 loop 再转一圈
- **API 错误时跳过**避免 death spiral（错误 → hook 触发 → 又错 → 又触发）

用途：
- 记忆提取（这轮里的有用事实自动 write anchor）
- 模板分类
- 提示建议
- 权限追踪

**grid-engine 现状**：只有 `fire_post_task_hooks`，能力有限（hook 不能触发 loop 再跑）。

**吸收**：扩展现有 hook 系统，允许 hook 返回 "inject_and_continue" 决策。

**建议落地**: Phase 2 **S3.T4**（新任务，和 skill extraction meta-skill 一起做，因为 skill extraction 本身就是一个完美的 stop hook 用例）

---

## Tier 2：中期值得做（Phase 3 或 Phase 2 末尾）

### 5. Tail-protected + iterative context 压缩 ⭐⭐⭐

**来源**: hermes `context_compressor.py:60-70`、`trajectory_compressor.py:54+`

**策略**：
- 保护头部（system + 第一组 user/assistant 交换）
- 保护尾部（最近 ~20K token）
- 中间用廉价模型总结（"summarizer provider"，建议 haiku / GPT-3.5）
- **下一次压缩时复用上次 summary，不是从头总结**（关键创新——节省 token 10x）
- Summary 长度自适应：原内容 20%，2K-12K ceil
- Tool output pre-pruning 在 LLM 调用前先过一遍

**grid-engine 现状**：无上下文压缩，长会话必崩。

**建议落地**: Phase 2 **S3.T1 PreCompact 接入** → 作为"compaction 策略"的实现

---

### 6. Reactive compaction（触发式压缩）⭐⭐

**来源**: claude-code `CCB/src/services/compact/reactiveCompact.ts`

**策略**：413 payload-too-large 或 context overflow 时才触发，不是 proactive。单次限制（`hasAttemptedReactiveCompact` guard）避免死循环。

**与 #5 的关系**：hermes 侧重"策略"，claude-code 侧重"触发时机"。**两个合用**：平时用 hermes 的 tail-protected 策略；413 / context overflow 时降级到 reactive（allow more aggressive truncation）。

**建议落地**: Phase 2 **S3.T1** 一起做

---

### 7. Final-output tool gate（可选 opt-in）⭐⭐

**来源**: goose `agent.rs:1714-1725`

**策略**：注册一个特殊的 `final_output` tool，**LLM 只有调用它才能真正退出**。否则 text-only 响应被注入 `FINAL_OUTPUT_CONTINUATION_MESSAGE` 让 loop 继续。

**与 hermes intermediate-ack 的对比**：
| 维度 | hermes intermediate-ack | goose final_output gate |
|------|-----------------------|----------------------|
| 向后兼容 | ✅ 已有 skill 零修改 | ❌ 每个 skill 要声明 |
| 语义精准度 | 启发式（可能误触发） | 显式（LLM 必须调 tool） |
| 落地难度 | 低 | 中（需改 skill 和 LLM prompt） |

**建议落地**: 作为 skill-level opt-in，Phase 3 或更后。Phase 2 用 intermediate-ack 作为默认。

---

### 8. 并行 tool 执行（read-only 并发，write serial）⭐⭐

**来源**: claude-code `CCB/src/services/tools/StreamingToolExecutor.ts`

**策略**：
- 一轮里多个 tool_use 分成：read-only 并行、有副作用的 serial
- 取消信号传播到每个 tool
- **被取消的 queue 里的 tool 自动生成 synthetic tool_result**（避免 LLM 下一轮看到 tool_use 无 tool_result 而崩）

**grid-engine 现状**：tool 串行执行。

**建议落地**: Phase 3（性能优化 stage）。Phase 2 焦点是 correctness 不是 performance。

---

### 9. 跨压缩 token 预算 ⭐⭐

**来源**: claude-code `CCB/src/query.ts:314, 1175-1184`

**策略**：用 `finalContextTokensFromLastResponse()` 从 `usage.iterations[-1]` 取真实 context 占用，不是累计输入输出 token。压缩后预算**不重置**（`taskBudgetRemaining` 是 loop-local 变量）。

**grid-engine 现状**：有 TokenEscalation，但不跨压缩持续。

**建议落地**: 和 #5 + #6 同期（Phase 2 S3）

---

### 10. Thread-scoped interrupt 传播 ⭐

**来源**: hermes `interrupt.py:24-49`

**策略**：Per-thread interrupt flag + `_ThreadAwareEventProxy`。多 session 同进程时，cancel 只影响目标 session。

**grid-engine 现状**：有 `cancel_token`，但未细化到"避免跨 session 污染"。

**建议落地**: Phase 2 **S4.T4**（新任务，和 E2E 验收一起做，确保多 session 不互相干扰）

---

## Tier 3：知道就好，不紧急

### 11. Recipe + success_checks 自动 retry（goose）

Recipe 带 shell 命令作为 success check，agent loop 跑完执行 check，失败 → **重置对话到初始状态**重跑。适合 CI 场景。

**grid-engine 不急做**，EAASP 目前没有这类场景。Phase 5+ 再考虑。

---

### 12. Credential pool（hermes）

多个 OpenAI key 轮换 + per-key 状态（ok/exhausted + TTL）。

**grid-engine 不急做**，单 key 够用。企业部署时再提。

---

### 13. Ephemeral system prompt（hermes）

系统提示词分"持久的"（缓存）和"临时的"（每次注入）。不破坏 prompt cache 的同时塞临时指令。

**grid-engine 已有 prompt caching**，但没区分 persistent/ephemeral。Phase 4 可做。

---

### 14. CacheSafeParams for forked agent（claude-code）

子 agent snapshot 父的 cache-safe params，跑完 merge 回主 loop。sub-agent / task 委派。

**grid-engine 现状**：有 sub-agent 机制，但 cache 复用不够好。Phase 4 可做。

---

### 15. Agno 的 pause 机制（反面参考，有部分可借鉴）

Agno 没 loop（单次 call），但 `requires_confirmation` / `requires_user_input` / `external_execution_required` 作为一等公民 pause 信号值得借鉴——**tool 可以主动声明"这次需要人类批准"**，loop 暂停存状态，调用方用 `continue_run()` 恢复。

**grid-engine 现状**：HITL 是 hook decision 实现的，不是一等公民。

**建议落地**: Phase 3+，需要改状态机。

---

## 落地批次规划（映射到 Phase 2）

### 批次 A（Phase 2 S1 — 紧跟 D87 修复）

| 任务编号 | 模式 | 来源 | 备注 |
|---------|-----|------|------|
| S1.T1 | hermes intermediate-ack | hermes | D87 修复主体（见根因文档） |
| S1.T6（新） | ErrorClassifier | hermes | 为后续错误处理铺路 |
| S1.T7（新） | withRetry graduated | claude-code | 和 ErrorClassifier 配合 |

### 批次 B（Phase 2 S2 / S3 — 记忆和压缩）

| 任务编号 | 模式 | 来源 | 备注 |
|---------|-----|------|------|
| S2.T5（新） | Tool result 三层截断 | hermes | 溢出文件入 L2 memory，复用 embedding |
| S3.T1 升级 | Tail-protected compaction | hermes | 升级原 PreCompact 任务为完整实现 |
| S3.T1 增补 | Reactive compaction | claude-code | 和 #5 互补 |
| S3.T1 增补 | 跨压缩 token 预算 | claude-code | 和压缩一起做 |
| S3.T4（新） | Stop hooks 扩展 | claude-code | 配合 skill extraction meta-skill |

### 批次 C（Phase 2 S4 E2E 验收时）

| 任务编号 | 模式 | 来源 | 备注 |
|---------|-----|------|------|
| S4.T4（新） | Thread-scoped interrupt | hermes | 多 session 隔离验证 |

### 批次 D（Phase 3+）

| 模式 | 来源 | 备注 |
|-----|------|------|
| 并行 tool 执行 | claude-code | 性能优化 |
| Final-output tool gate | goose | skill-level opt-in |
| Ephemeral system prompt | hermes | prompt cache 优化 |
| CacheSafeParams subagent | claude-code | sub-agent cache 复用 |
| Pause 一等公民 | agno | HITL 改状态机 |
| Recipe + success_checks | goose | CI 场景 |
| Credential pool | hermes | 企业部署 |

---

## 验收检查

每个批次落地后应该有：
1. 新建或修改的模块有单元测试
2. E2E 场景能触发（特别是批次 A 的 intermediate-ack，要有 regression test）
3. ADR 记录设计决策（引用本文档对应章节）
4. WORK_LOG 更新

---

## 关联文档

- `AGENT_LOOP_ROOT_CAUSE_ANALYSIS.md` — 为什么 claude-code 能多轮而 grid 不能的根因
- `docs/design/EAASP/adrs/ADR-V2-016-agent-loop-generic-principle.md` — agent loop 设计原则 ADR
- `docs/plans/2026-04-14-v2-phase2-plan.md` — Phase 2 实施计划
