# octo-sandbox 下一会话指南

**最后更新**: 2026-03-22 19:41 GMT+8
**当前分支**: `main`
**当前状态**: Phase U COMPLETE + Post-Polish，已合并至 main

---

## 项目状态

从 Wave 1 到 Phase U，所有计划阶段均已完成。TUI 已完成从 12-Tab 到对话中心布局的全面重构。

```
Phase U:  TUI Production Hardening (10/10+3)→ COMPLETE @ 77c2297 (merged to main)
Phase T:  TUI OpenDev 整合 (24/24)          → COMPLETE @ 74464b9
CLI/Server Fixes: Usability hardening       → COMPLETE @ b4ebcbe
Phase S:  Agent Capability Boost (13/13)    → COMPLETE @ 68ad13e
Phase R:  GAIA Filtered Eval (8/8)          → COMPLETE @ 50df5e6
Phase Q:  GAIA & SWE-bench (15/15)          → COMPLETE @ 1ce10e5
Phase P:  Baseline Eval R2 (16/16)          → COMPLETE @ b0ba059
Phase O:  Deferred 暂缓项全解锁 (15/15)     → COMPLETE @ 9da42de
Phase N:  Agent Debug Panel (7/7)           → COMPLETE @ 3ba3351
Phase M-b: TUI Dual-View + Eval (8/8)      → COMPLETE @ 76bc12e
Phase M-a: Eval CLI Unification (12/12)     → COMPLETE @ e2b505b
Phase L:  Eval Whitebox (18/18)             → COMPLETE @ f28ad6c
Phase K:  Model Benchmark (11/12)           → COMPLETE @ 07f7ae9
Phase J:  Sandbox Security (16/16)          → COMPLETE @ 45a7342
Phase I:  External Benchmarks (13/13)       → COMPLETE @ 57ca310
Phase H:  Eval Capstone (10/10)             → COMPLETE @ 37680ec
Phase A-G: Eval Framework (85/85)           → COMPLETE @ ca5c898
Wave 1-10: Core Engine + CLI               → COMPLETE @ 675155d
```

### 基线数据

- **Tests**: 2329 passing (workspace), 456 (octo-cli)
- **评估任务**: ~297 个 (内部 167 + 外部 130)
- **GAIA 结果**: MiniMax-M2.1 41.6%, Qwen3.5-27B 39.2%
- **测试命令**: `cargo test --workspace -- --test-threads=1`

---

## Deferred 未清项（下次 session 启动时必查）

| 来源 | ID | 内容 | 前置条件 | 状态 |
|------|----|----|---------|------|
| Phase U | U-D1 | Agent Debug Panel 重设计 — 信息整合到 StatusBar 后需深入调整 | Phase U G3 完成 | 前置已满足 |
| Phase S | S-D1 | Agent Skills 系统是否符合标准规范 — 专题研究 | Phase S 完成 | 前置已满足 |

---

## 下一步建议

### 方向 1: U-D1 Agent Debug Panel 重设计

StatusBar 已整合 brand/tokens/elapsed/context%/git 信息，原有 Debug Panel（Phase N 实现的 dev_agent overlay）需要重新设计以避免信息重复。

### 方向 2: S-D1 Agent Skills 规范研究

对比 Agent Skills 系统与行业标准规范（如 OpenAI function calling、MCP tool spec），评估兼容性。

### 方向 3: 更强模型评估

使用 Claude/GPT-4o 级模型跑 GAIA 对比，突破 Qwen3.5 瓶颈。

### 方向 4: Agent 工具链增强

更多内置工具、更好的搜索策略、文件解析能力。

---

## TUI 快捷键参考

| 快捷键 | 功能 |
|--------|------|
| Enter | 发送消息 |
| Shift+Enter | 换行 |
| ESC | 取消当前任务（保留已完成内容） |
| Ctrl+O | 循环展开/折叠最近工具结果 |
| Alt+O | 全局切换工具折叠/展开 |
| Ctrl+Shift+O | 折叠所有工具结果 |
| Y/N/A | 工具审批（Approve/Deny/Always） |
| PageUp/PageDown | 快速滚动 |
| Tab | 自动完成 |
| Ctrl+D | 退出 |

---

## 快速启动

```bash
# 编译检查
cargo check --workspace

# 全量测试
cargo test --workspace -- --test-threads=1

# TUI 模式
make cli-tui

# CLI 交互模式
make cli-run

# 启动 server + web
make dev

# Server 单独启动
make server
```
