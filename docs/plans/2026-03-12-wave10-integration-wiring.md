# Wave 10: Integration Wiring Plan

> 目标：将 Wave 7-9 已实现但未接线的模块接入运行时，一波完成。
> 预期：实际可用评分从 ~7.3 → 8.5

---

## 执行批次

### Batch A — harness.rs 核心接线（1 个 agent，顺序修改）

修改 `crates/octo-engine/src/agent/harness.rs`，以下 4 项按行号从上到下依次插入，无冲突：

| # | 接线项 | 插入位置 | 改动 |
|---|--------|---------|------|
| A1 | E-Stop 检查 | loop 顶部 (~L229) | `if let Some(es) = &config.estop { if es.is_triggered() { emit EmergencyStopped; break; } }` |
| A2 | Canary Rotation | LLM 调用前 (~L424) | `if let Some(cg) = &config.canary_guard { let token = cg.rotate(); /* embed in context */ }` |
| A3 | Text Tool Call Recovery | stream 消费后 (~L542) | `if tool_uses.is_empty() && !full_text.is_empty() { tool_uses = parse_tool_calls_from_text(&full_text); }` 并移除 `#[allow(dead_code)]` |
| A4 | Self-Repair | tool 执行失败后 (~L1008) | `if let Some(sr) = &mut config.self_repair { match sr.check_and_repair(tool_name, success) { Repaired(hint) => /* inject hint */, Unrecoverable => /* emit event, break */ } }` |

附带修改：
- `loop_config.rs`: 添加 `canary_guard: Option<CanaryGuardLayer>` 字段 + builder
- `runtime.rs`: 构造 LoopConfig 时传入 canary_guard

### Batch B — 查询/上下文接线（3 个 agent 并行）

| # | 接线项 | 文件 | 改动 |
|---|--------|------|------|
| B1 | Reranker → HybridQueryEngine | `memory/sqlite_store.rs` hybrid_search() | RRF 后，若 config 有 `RerankStrategy::Llm`，调用 `reranker.rerank()` |
| B2 | PromptParts dynamic_context | `context/system_prompt.rs` build_separated() | 将时间戳/MCP 服务器列表/session 状态路由到 `dynamic_context` 字段 |
| B3 | RetryPolicy → Provider Pipeline | `providers/pipeline.rs` | 在 HTTP 错误处理中用 `RetryInfo::from_response()` 解析，按 `RetryPolicy` 决策 backoff/failover |

### Batch C — 注册/中间件/数据（5 个 agent 并行）

| # | 接线项 | 文件 | 改动 |
|---|--------|------|------|
| C1 | KG Tools 注册 | `agent/runtime.rs` | ToolRegistry setup 时调用 `register_kg_tools()` |
| C2 | Rate Limit 中间件 | `agent/harness.rs` tool 执行前 (~L959) | 检查 `tool.rate_limit()`，简单计数器拦截 |
| C3 | OAuth PKCE 修复 | `mcp/oauth.rs` L116 | hex → base64url 编码，更新测试 |
| C4 | Metering 定价表扩充 | `metering/pricing.rs` | 添加 ~20 模型，注意 specific-first 排序 |
| C5 | Provider 默认表扩充 | `providers/defaults.rs` | 添加 ~10 providers（openrouter, cohere, google 等） |

---

## 依赖关系

```
Batch A (顺序) ─── 无外部依赖
Batch B (并行) ─── 无外部依赖
Batch C (并行) ─── C3 需要 Cargo.toml 加 base64 非 optional

A, B, C 三个 batch 互相独立，可全部并行启动。
```

## 验证

完成后统一运行：
```bash
cargo test --workspace -- --test-threads=1
```

预期：1761 + 新增接线测试 ≈ 1780+ tests passing
