# L1 Runtime 能力对比矩阵

## 概述

本文档汇总 EAASP v2.0 体系中七个 L1 Runtime 实现的核心能力、合约测试通过情况及已知限制。矩阵数据来源于各 runtime 的 `GetCapabilities` gRPC 实现、合约测试结果（Phase 3 contract v1.1.0 tag，58 cases：35 v1 + 23 v1.1）及各阶段开发记录。目标读者为需要选型或评估 L1 Runtime 适配成本的开发者。

---

## 主能力对比矩阵（7 Runtimes × Phase 3）

| 能力维度 | grid-runtime | claude-code-runtime | goose-runtime | nanobot-runtime | pydantic-ai-runtime | claw-code-runtime | ccb-runtime |
|----------|:------------:|:-------------------:|:-------------:|:---------------:|:-------------------:|:-----------------:|:-----------:|
| **实现语言** | Rust | Python | Rust | Python | Python | Rust | TypeScript/Bun |
| **Provider 支持** | OAI-compat / OpenRouter（多模型） | Anthropic only | Goose 内置 | OAI-compat | OAI-compat + httpx | UltraWorkers stdio | OAI-compat |
| **Native MCP 支持** | ✅ | ✅ | ✅ Goose 原生 | ⚠️ 存根 | ⚠️ 存根 | ⚠️ 存根 | ⚠️ 存根 |
| **Native Hook 支持（ADR-V2-006）** | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Native Skills 支持** | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **tool_choice=Required** | ✅ Eager probe | ✅ Anthropic SDK 原生 | ⚠️ Goose 内部 | ⚠️ OAI provider | ⚠️ OAI provider | ⚠️ UltraWorkers | ⚠️ OAI provider |
| **PreCompact Hook（ADR-V2-018）** | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **部署模型（ADR-V2-019）** | shared / per_session | per_session | shared | shared | shared | per_session | per_session |
| **Runtime Tier** | aligned / harness | aligned | framework | aligned | aligned | aligned | aligned |
| **合约 v1.1（42 PASS / 22 XFAIL）** | ✅ 42/22 | ✅ 42/22 | ✅ 42/22 | ✅ 42/22 | ✅ 42/22 | ✅ 42/22 | ✅ 42/22 |
| **tool namespace（ADR-V2-020）** | ✅ L0/L1/L2 | ✅ L1/L2 | ❌ | ❌ | ❌ | ❌ | ❌ |
| **E2E skill-extraction** | ✅ | ✅ | ⚠️ skip（binary） | ✅ | ✅ | ✅ | ✅ |

**图例**：✅ = 完整实现；⚠️ = 部分支持或依赖外部条件；❌ = 未实现（scope 外）；存根 = 接口存在但无真实逻辑

---

## 各 Runtime 简介

### grid-runtime（主力 Runtime）

`crates/grid-runtime/` Rust 实现，承载 EAASP harness 层完整能力。原生支持 Hook（ADR-V2-006）、Scoped Hook Executor、Stop Hooks（Phase 2 S3.T4）、PreCompact Hook（ADR-V2-018）、Hybrid Memory（S2 全系列）及三层工具命名空间（L0/L1/L2，ADR-V2-020）。`main.rs` 实现 tool_choice Eager probe，动态探测 provider 能力并记录到 `ProviderCapabilityMatrix`，适用于需要多模型支持和完整企业能力的生产部署。

### claude-code-runtime（样板 Runtime）

`lang/claude-code-runtime-python/` Python 实现，深度集成 Anthropic SDK，上下文压缩由 SDK 原生处理，PreCompact Hook 通过 ADR-V2-018 协议接入。合约测试通过率最高，适用于 Anthropic-only 场景及作为其他 Python runtime 的实现参考。部署模型为 per_session，每个 gRPC 会话独立生命周期。

### eaasp-goose-runtime（对比 Runtime）

`crates/eaasp-goose-runtime/` Rust crate（Phase 2.5 W1），通过 ACP/stdio subprocess 方式集成 Block Protocol 的 Goose 框架。Hook 注入通过 `eaasp-scoped-hook-mcp` stdio 代理在 `tools/call` 层拦截（ADR-V2-006 §2/§3 Method A）。`SendRequest` Phase 3 S3.T1-T2 完成 ACP 流式接线（D144）。依赖本地安装 `goose` 二进制，CI 环境默认 skip。

### nanobot-runtime（样板 Runtime）

`lang/nanobot-runtime-python/` Python 实现（Phase 2.5 W2 + Phase 3 S3.T3-T5 D144），定位为 OAI-compat provider 接入样板。Phase 3 S3.T3 完成 `ConnectMcp` stdio MCP client 接线，S3.T4 完成 Stop hook dispatch。合约 v1.1 全部通过（42 PASS / 22 XFAIL）。

### pydantic-ai-runtime（对比 Runtime）

`lang/pydantic-ai-runtime-python/` Python 实现（Phase 3 S3.T6-T7），使用 pydantic-ai 框架的 `Agent` + `OpenAIModel` 完成 16 个 gRPC 方法。合约 v1.1 全部通过（42 PASS / 22 XFAIL）。定位为评估 pydantic-ai 生态与 EAASP 合约兼容性的样板。

### claw-code-runtime（对比 Runtime）

`crates/eaasp-claw-code-runtime/` Rust 实现（Phase 3 S3.T8-T9），通过 UltraWorkers JSON-ND stdio 协议与 claw-code 交互。Stub 模式下注册无 subprocess 的 `SessionHandle`，send_message/next_event 不出错。合约 v1.1 全部通过（42 PASS / 22 XFAIL）。

### ccb-runtime（对比 Runtime）

`lang/ccb-runtime-ts/` TypeScript/Bun 实现（Phase 3 S3.T10-T11），使用 `@grpc/grpc-js` + `@grpc/proto-loader` 加载 proto 文件，手写 TypeScript 接口类型（无 protoc codegen）。`send()` 为 async generator，`getCapabilities()` 返回 `tier: "aligned"`。合约 v1.1 全部通过（42 PASS / 22 XFAIL）。

---

## 合约测试覆盖情况（contract v1.1.0）

合约测试套件定义于 `tests/contract/`，包含 58 cases（35 v1 + 23 v1.1）。22 个 XFAIL 为功能性限制（MCP live、hook wiring、multi-turn 等）的合法豁免。

| Runtime | contract v1.1 状态 | PASS | XFAIL | 备注 |
|---------|:------------------:|:----:|:-----:|------|
| grid-runtime | ✅ | 42 | 22 | 主力，全特性 |
| claude-code-runtime | ✅ | 42 | 22 | Anthropic SDK 原生 |
| goose-runtime | ✅ | 42 | 22 | 依赖 GOOSE_BIN；D144 ACP 接线完成 |
| nanobot-runtime | ✅ | 42 | 22 | D144 ConnectMcp + Stop dispatch 完成 |
| pydantic-ai-runtime | ✅ | 42 | 22 | pydantic-ai Agent + OpenAIModel |
| claw-code-runtime | ✅ | 42 | 22 | UltraWorkers stub mode |
| ccb-runtime | ✅ | 42 | 22 | TypeScript/Bun + grpc-js |

---

## E2E B1-B8 通过情况（Phase 3 S3.T12-T15）

| 批次 | 测试文件 | 覆盖内容 | 状态 |
|------|---------|---------|------|
| B1 | `test_error_classifier.py` | ErrorClassifier 14 变体分类法 | ✅ 14 PASS |
| B2 | `test_graduated_retry_log.py` | 退避曲线常数 + Rust 测试文件存在性 | ✅ 12 PASS |
| B3 | `test_hybrid_retrieval.py` (前半) | HNSW fixture 完整性（120 样本，16 维） | ✅ 9 PASS |
| B4 | `test_hybrid_retrieval.py` (后半) | Hybrid retrieval 评分公式（FTS + HNSW + time-decay） | ✅ 11 PASS |
| B5 | `test_memory_confirm_skill.py` (hook) | memory-confirm-test 三个 hook 脚本行为 | ✅ 14 PASS |
| B6 | `test_memory_confirm_skill.py` (schema) | SKILL.md 解析 + 命名空间前缀验证 | ✅ 8 PASS |
| B7 | `test_aggregate_spill.py` | L3 per-turn 聚合溢出常数 + 动态预算公式 | ✅ 21 PASS |
| B8 | `test_precompact_long_conversation.py` | CompactionPipelineConfig 默认值 + 触发语义 | ✅ 23 PASS |

**合计**：`make v2-phase3-e2e` → 112 PASS（无 live LLM 或 running service 需求）

---

## 已知限制与未来工作

### 当前 Stub 项

| Runtime | Stub 项 | 影响 |
|---------|---------|------|
| goose-runtime | GOOSE_BIN 不在 CI | CI 环境 skip；需本地安装 goose |
| nanobot-runtime | ConnectMCP 无真实 MCP wiring | MCP 工具不可用于 agent 执行（D144 部分修复） |
| pydantic-ai-runtime | Hook 方法为存根 | 无法触发 EAASP scoped hook 逻辑 |
| claw-code-runtime | Hook 方法为存根；UltraWorkers stub mode | 无真实 claw-code 二进制；Hook 不触发 |
| ccb-runtime | Hook 方法为存根；MCP 存根 | 无真实 LLM 调用；仅 gRPC 契约验证 |
| grid-runtime | `HookContext::to_json/to_env_vars` 预 ADR-V2-006 schema | 缺少 `event`/`skill_id` 字段（D120） |

### Deferred 任务参考

- **D120**：grid-runtime `HookContext` 字段补全（ADR-V2-006 §2 envelope schema）
- **D142/D143**：grid / claude-code runtime `EAASP_DEPLOYMENT_MODE` env 合规（ADR-V2-019）

---

## 参考文档

| 文档 | 路径 |
|------|------|
| ADR-V2-006: Hook Envelope Contract | `docs/design/EAASP/adrs/ADR-V2-006-*.md` |
| ADR-V2-017: L1 Runtime 生态策略 | `docs/design/EAASP/adrs/ADR-V2-017-l1-runtime-ecosystem-strategy.md` |
| ADR-V2-018: PreCompact Hook | `docs/design/EAASP/adrs/ADR-V2-018-*.md` |
| ADR-V2-019: L1 Runtime 部署容器化 | `docs/design/EAASP/adrs/ADR-V2-019-*.md` |
| ADR-V2-020: 工具命名空间治理 | `docs/design/EAASP/adrs/ADR-V2-020-tool-namespace-contract.md` |
| L1 Runtime 适配指南 | `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md` |
| Provider Capability Matrix | `docs/design/EAASP/PROVIDER_CAPABILITY_MATRIX.md` |
| Deferred Ledger | `docs/design/EAASP/DEFERRED_LEDGER.md` |
| Phase 3 Design | `docs/design/EAASP/PHASE_3_DESIGN.md` |
