# EAASP v2.0 Phase 0.5 工作日志

> **Phase**: MVP — 全层贯通
> **日期**: 2026-04-12 ~ 2026-04-13
> **状态**: 🟢 Completed

---

## 概述

Phase 0.5 将 Phase 0 的 Infrastructure Foundation 升级为可运行的 MVP：用户通过 `eaasp-cli` 发起 session，agent 实时调用 SCADA 工具、写入 memory、触发 hook，跨 session 读取历史记忆。

---

## 完成的 12 个任务

### Stage S1 — L4→L1 gRPC 真通路
- **S1.T1**: Proto → Python 生成管线（L4 gRPC stubs）
- **S1.T2**: L1RuntimeClient 抽象 + Initialize/Send 真调用
- **S1.T3**: Session 状态机完善（created→active→closed→failed + close endpoint）

### Stage S2 — LLM Provider 集成
- **S2.T1**: Provider 配置通路（proto llm_provider/llm_model + env passthrough）
- **S2.T2**: grid-runtime Agent Loop 启用 LLM（capabilities dynamic model）

### Stage S3 — MCP + Hook 执行器
- **S3.T1**: L1 Runtime connectMCP 对接 mock-scada（auto-wiring from skill deps）
- **S3.T2**: Scoped-Hook 执行器（ScopedHookHandler + command hook execution）
- **S3.T3**: Agent Tool Call → Memory Write 通路（L2 MemoryWriteHook）

### Stage S4 — 流式输出管线
- **S4.T1**: L1→L4 Streaming Response（SSE endpoint）
- **S4.T2**: CLI 流式显示（stream_sse + --stream default）

### Stage S5 — 人工验收 + 收尾
- **S5.T1**: `make dev-eaasp` 一键启动（8 服务编排）
- **S5.T2**: 人工验收 + hermes 容器化 + memory/hook 修复 + 文档收尾

---

## 关键技术变更

### Memory Write 修复（2026-04-13）
**根因**: 三个 runtime 的 memory hook 只写 `anchors` 表（`write_anchor`），不写 `memory_files` 表（`write_file`）。`memory_search` 的 FTS5 索引只在 `memory_files` 上，导致搜索永远返回空。

**修复**: 每个 runtime 在 `write_anchor` 后追加 `write_file`：
- `grid-runtime`: `memory_write_hook.rs` — anchor 写入后追加 memory_file
- `claude-code-runtime`: `service.py` OnToolResult — 同上
- `hermes-runtime`: 新建 `l2_memory_client.py` + OnToolResult 写入（从零添加）

### Hermes L2 Memory 工具注入
**问题**: hermes-agent 从 skill prose 读到 memory_* 工具名试图调用，但这些工具没注入。
**方案**: `L2MemoryToolProxy` — REST proxy，把 L2 的 6 个 MCP tools 注册为 agent 可调用工具。
**Deferred**: D67/D68 — Phase 1 统一为 L2 SSE MCP transport + L4 ConnectMCP 下发。

### Skill Prose 自主工作流指示
**问题**: grid-runtime agent 生成校准建议后停下等用户确认写入。
**修复**: SKILL.md 加 `IMPORTANT: This is an autonomous workflow...` 指示。

### CLI Memory Search 显示修复
**问题**: `eaasp-cli memory search` 显示空列（memory_id/scope/category）。
**根因**: L2 SearchHit 结构嵌套 `{"memory": {...}, "score": ...}`，CLI 从顶层取字段。
**修复**: `cmd_memory.py` flatten nested structure + 显示 content 截断。

### Hook Deny 单元验证
新增 2 个集成测试用真实 `block_write_scada.sh` 脚本：
- `block_write_scada_hook_denies_scada_write`: scada_write → Block (exit 2)
- `block_write_scada_hook_allows_non_scada_tools`: bash → Continue (exit 0)

### ADR-V2-005 Tool Sandbox Container
设计文档（Phase 1+ 实现）：Session 级工具容器隔离，sibling container + network MCP (SSE)。

### Hermes Runtime 容器化 + MCP SSE Bridge
- Dockerfile 重建（v2 proto stubs + mcp 依赖）
- mock-scada 新增 SSE transport（`--transport sse --port 18090`）
- `McpBridge` SSE 客户端 + `inject_mcp_tools` monkey-patch

---

## 验收结果

| 验证项 | grid-runtime | claude-code-runtime | hermes-runtime |
|--------|:-:|:-:|:-:|
| Skill prose 注入 | ✅ | ✅ | ✅ |
| MCP tool call (scada_read) | ✅ | ✅ | ✅ (SSE) |
| Scoped hooks 注册 | ✅ | ✅ | — |
| Memory write (anchor + file) | ✅ | ✅ | ✅ |
| Streaming output | ✅ | ✅ | ✅ |
| Memory search (FTS5) | ✅ | ✅ | 待重建容器 |
| Hook deny (scada_write) | ✅ (单元测试) | ✅ (单元测试) | — |

### Memory Search 验证
```
make eaasp-memory-search Q="xfmr"
→ 1 hit, score 0.9999 (tool_evidence category)
```

### Hook Deny 验证
```
cargo test -p grid-runtime block_write_scada -- 2 passed
```

---

## 测试计数

| 组件 | 测试数 | 变化 |
|------|--------|------|
| grid-runtime | 97 pass + 2 ignored | +2 (hook deny 集成测试) |
| claude-code-runtime | 77 pass | 无变化 |
| hermes-runtime | 25 pass | 无变化 |
| L4 orchestration | 61 pass | 无变化 |
| L2 memory engine | 47 pass | 无变化 |
| skill-registry | 23 pass | 无变化 |

---

## 新增 Deferred

| ID | 内容 | Phase |
|----|------|-------|
| D62 | Per-session tool-sandbox container lifecycle (L4) | Phase 1+ |
| D63 | Tool-sandbox 通用基础镜像 + OCI artifact 分发 | Phase 1+ |
| D64 | T0/T1 runtime 的工具容器化 | Phase 1+ |
| D65 | MCP server 多实例/连接池 | Phase 1+ |
| D66 | hermes-agent 内置工具与 MCP monkey-patch 叠加修复 | Phase 1 |
| D67 | L2 Memory Engine 实现 SSE MCP transport | Phase 1 |
| D68 | L4 统一下发所有 MCP server 配置（含 L2） | Phase 1 |

---

## 下一步

Phase 1: Event-driven foundation — 先解 ADR-V2-001/002/003，再实现 L4 Event Engine。
