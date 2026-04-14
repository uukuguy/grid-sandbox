# Agent Loop 多轮工具调用根因分析

**创建日期**: 2026-04-14
**关联**: D87（grid-engine agent loop 过早退出）、ADR-V2-016、Phase 2 S0.T2 / S1.T1

---

## 背景

Phase 1 E2E 验证（2026-04-14）发现：threshold-calibration skill 定义 6 步工作流（scada_read → memory_search → memory_read → memory_write_anchor → memory_write_file → final JSON），但三个 runtime 表现差异巨大：

| Runtime | PRE_TOOL_USE 计数 | workflow 完成度 |
|---------|-----------------|--------------|
| claude-code-runtime | ≥4 | ✅ 完整跑完 |
| hermes-runtime | ≥4 | ✅（MCP stdio 缺失，有幻觉但循环本身通） |
| grid-runtime | 1 | ❌ 只执行 step 1 就停 |

最初定位到 `crates/grid-engine/src/agent/harness.rs:1169` 的退出条件 `stop_reason != ToolUse || tool_uses.is_empty()`，以为是 bug。但深入调研后发现**根因并非此处的代码逻辑**。

---

## 三个 Loop 实现的客观对比

### 终止条件对比（源码逐一核对）

| 系统 | 文件位置 | 终止条件 | 对 text-only 响应的反应 |
|------|---------|---------|----------------------|
| **claude-code** | `CCB/src/query.ts:341, 1100` | `needsFollowUp == false`（= 本轮**无 tool_use 块**） | **终止**，返回 `reason: 'completed'` |
| **hermes** | `run_agent.py:7866`, `agent_loop.py:204` | `assistant_msg.tool_calls` 为空 **且** 内容校验通过 **且** 不是 thinking-only **且** 重试次数用光 | **不终止**——继续几轮 retry / prefill / 注入 "Continue now" |
| **grid-engine** | `harness.rs:1169` | `stop_reason != ToolUse \|\| tool_uses.is_empty()` | **立刻终止**（唯一例外是 MaxTokens 有 escalation/continuation） |

### 关键代码引用

**claude-code `CCB/src/query.ts:589-590`**（精确注释原文）：
> "Set during streaming whenever a tool_use block arrives — the sole loop-exit signal. If false after streaming, we're done (modulo stop-hook retry)."

**claude-code 和 grid-engine 的终止规则本质上是同一个**——都是"本轮无 tool_use → terminate"。

**hermes `run_agent.py:10049-10074`** intermediate-ack detection：
```python
# 检测 LLM 只是在打招呼 / ack，没真干活
if is_intermediate_ack(assistant_text):
    messages.append({
        "role": "system",
        "content": "[System: Continue now. Execute the required tool calls ...]"
    })
    continue  # 不 break，逼 LLM 继续
```

**hermes 有、claude-code 和 grid-engine 都没有**的关键逻辑。

---

## 真正的根因：LLM 模型能力差异，不是 loop bug

### 事实一：三个 runtime 用的 LLM 不同

| Runtime | 模型 | API | Env |
|---------|-----|-----|-----|
| **claude-code-runtime** | `claude-sonnet-4-20250514` | Anthropic 官方（通过 claude-agent-sdk → CLI subprocess） | `ANTHROPIC_API_KEY` |
| **hermes-runtime** | GPT-4 等 OpenAI 家族 / OpenRouter 路由的模型 | OpenAI HTTP | `OPENAI_API_KEY` |
| **grid-runtime** | GPT-4 等 OpenAI 家族 / OpenRouter 路由的模型 | OpenAI HTTP | `OPENAI_API_KEY` |

源码证据：
- `lang/claude-code-runtime-python/src/claude_code_runtime/config.py:22,47`: 默认 `claude-sonnet-4-20250514` + `ANTHROPIC_API_KEY`
- `crates/grid-runtime/src/config.rs:44-50`: `LLM_PROVIDER="openai"` → `OPENAI_*` 三连

**两个 runtime 根本不用同一个 LLM。**

### 事实二：不同模型的"自主行为"差异

同一个 skill prompt（"校准 Transformer-001 的温度阈值，用 SCADA 读快照 → memory 查历史 → 写 anchor 和 file → 返回 JSON"），丢给：

- **Claude Sonnet 4**：**一轮连续产出多个 tool_use 块**（read_data + memory_search + ... 在同一个 assistant message 里），或者在下一轮继续产出 tool_use。**不会中途问"要不要继续"**。
- **GPT-4o / OpenRouter 开源模型**：调完第一个 tool 后，**往往返回 text "我已经读到了数据，是否需要继续 1/2/3?"**，`stop_reason=stop`（OpenAI）映射到 grid 的 `EndTurn`，**没有 tool_use 块**。

### 事实三：D87 regression test mock 已明示

`crates/grid-engine/tests/d87_multi_step_workflow_regression.rs:42-48` 注释原文：
> "real-world E2E scenario (grid-runtime + threshold-calibration skill, 2026-04-14): LLM calls `scada_read_snapshot` (step 1), receives tool result, then **emits text asking user "是否需要我：1/2/3?"** with stop_reason=EndTurn WITHOUT emitting another tool_use block."

这是观察到的 **GPT** 行为，不是 Claude。Test mock 精确复现了这个场景。

### 事实四：hermes 在同样 GPT 上不会卡住

hermes-runtime 和 grid-runtime 用同样的 `OPENAI_*` 配置，但 hermes 能多轮。原因：**hermes 的 loop 里有 `intermediate-ack detection`**，识别"LLM 只是在打招呼/ack，没真干活"，主动注入 `[System: Continue now. Execute the required tool calls...]` 再 `continue`。

这等于**让 hermes 自己扮演用户角色**去推动 LLM 继续。

---

## 架构错配

| 使用模式 | claude-code 合适吗 | hermes 合适吗 | grid-engine 现状 |
|---------|------------------|--------------|----------------|
| **交互式 REPL**（有人盯着） | ✅ 完美（SDK 用 Claude） | ✅ 也行 | ⚠️ 能工作，但 EAASP 不是 REPL |
| **non-interactive skill 执行**（API/服务模式） | ❌ 会停在 text-only（只是 Claude 自己很少触发） | ✅ intermediate-ack 补救 | ❌ 停在 text-only |

**grid-engine harness.rs 的 loop 是为 REPL 模式设计的**（和 claude-code 等价），**但 EAASP 场景是 non-interactive skill 执行**——没有人在 REPL 对面接"继续"。

**claude-code MVP 能过**是因为 **Claude 模型自己主动连续调 tool，根本没走到"text-only terminate"分支**——换句话说，**loop 的缺陷被模型掩盖了**，不是 loop 本身好。

---

## 一句话根因

> **grid-engine harness.rs 的 loop 是为"REPL 交互模式"设计的（和 claude-code 等价），被用在"non-interactive skill 执行"场景下，缺少 hermes-style 的"LLM 中间响应 → 自动注入 continue"机制。同样用 OpenAI 模型，hermes 补上了，grid 没补。**

---

## 修复方向

**不是修 harness.rs:1169 的退出条件**（它对 REPL 是对的）。
**是在退出条件满足前，加一个 `intermediate-ack detection + continuation 注入`**（照抄 hermes 的 `run_agent.py:10049-10074`）。

最小实现（伪代码）：

```rust
// harness.rs 在 L1168 退出分支入口前插入
if stop_reason == StopReason::EndTurn
    && tool_uses.is_empty()
    && total_tool_calls > 0  // 之前已经调过 tool，说明在 workflow 中途
    && workflow_continuation_count < MAX_WORKFLOW_CONTINUATIONS  // 防死循环（默认 3）
{
    // 照抄 hermes: 注入 system message 逼 LLM 继续
    workflow_continuation_count += 1;
    messages.push(ChatMessage::assistant(&full_text));
    messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: "[System: Continue now. Execute the required tool calls \
                   to complete the workflow. Do not ask for confirmation.]".into(),
        }],
    });
    let _ = tx.send(AgentEvent::IterationEnd { round, ... }).await;
    continue;  // 不 break，重入 loop
}

// ↓ 保持原有 L1169 退出分支不变
if tool_uses.is_empty() {
    ...
}
```

### 触发条件设计

- `stop_reason == EndTurn`：LLM 主动结束（不是 MaxTokens 等）
- `tool_uses.is_empty()`：本轮没调 tool
- `total_tool_calls > 0`：之前有过 tool，说明已经在 workflow 中途（不是一开始就 text-only 的纯对话场景）
- `workflow_continuation_count < MAX_WORKFLOW_CONTINUATIONS`：3 次上限，避免 LLM 死循环问用户

### 为什么不用 goose 的 `final_output` tool gate

goose 的方案要求每个 skill 注册一个特殊的 `final_output` tool。这对已有 skill（threshold-calibration 等）不向后兼容，**所有 skill 都要改**。hermes 的 intermediate-ack 是**纯 loop 层改动，skill 无需任何修改**，更适合 EAASP 已有生态。

---

## 验证

完成修复后的验收用 D87 regression test（mock 已经精确匹配这个场景）：

```bash
cargo test -p grid-engine d87_multi_step_workflow_no_early_exit \
  -- --test-threads=1 --nocapture
```

**预期**：测试通过（3 个 tool 全部被调用）。

E2E 验证则是真跑 grid-runtime + threshold-calibration，观察 `PRE_TOOL_USE ≥ 4`。

---

## 关联文档

- `docs/design/EAASP/AGENT_LOOP_PATTERNS_TO_ADOPT.md` — 从四个开源 runtime（hermes/claude-code/goose/agno）提取的可吸收 loop 优点清单，分批次落地
- `docs/design/EAASP/adrs/ADR-V2-016-agent-loop-generic-principle.md` — ADR 草稿（Proposed）
- `docs/plans/2026-04-14-v2-phase2-plan.md` — Phase 2 S0.T2（ADR-V2-016）和 S1.T1（D87 修复）
- `crates/grid-engine/tests/d87_multi_step_workflow_regression.rs` — 锁定 regression test
- 废弃：`docs/plans/2026-04-14-s1t1-d87-fix-implementation.md` —— 基于旧根因假设（改 L1169 退出条件）的计划，被本文档取代
