# octo-sandbox 下一会话指南

**最后更新**: 2026-03-23 09:30 GMT+8
**当前分支**: `main`
**当前状态**: Phase AB 完成，待选择下一阶段

---

## 项目状态

```
Phase AB: 智能体工具执行环境 (10/10) → COMPLETE @ 282d3f6
Phase AA: Octo 部署配置架构 (6/6+D2) → COMPLETE @ 4fbc30d
Phase Z:  Landmine Scan & Fix (2/2)   → COMPLETE @ 81fa923
Phase Y:  Playbook Skill SubAgent (1/1)→ COMPLETE @ c0f92b4
Phase X:  TUI 运行状态增强 (4/4)       → COMPLETE
Phase W:  OctoRoot 统一目录管理 (10/10) → COMPLETE
Phase V:  Agent Skills 完整实现 (11/12) → COMPLETE @ 19d3f30
Phase U:  TUI Production Hardening     → COMPLETE @ 77c2297
Phase T-A: 评估框架+TUI+基准           → ALL COMPLETE
Wave 1-10: Core Engine + CLI          → COMPLETE @ 675155d
```

### 基线数据

- **Tests**: octo-cli 472, 全量需确认
- **测试命令**: `cargo test --workspace -- --test-threads=1`

---

## Phase AB 完成摘要

- **SandboxProfile**: dev/stg/prod/custom 一行切换
- **OctoRunMode**: 自动检测容器/主机环境
- **ExecutionTargetResolver**: 路由引擎 RunMode × Profile × ToolCategory → Local|Sandbox
- **BashTool**: with_sandbox() + profile-aware env 过滤
- **SkillRuntime**: shell/python/node 尊重 profile timeout
- **ExternalSandboxProvider**: E2B/Modal/Firecracker 接口定义
- **CLI**: `octo sandbox status/dry-run/list-backends`
- **StatusBar**: 沙箱 profile 徽章显示

---

## Deferred 未清项

| 来源 | ID | 内容 | 前置条件 | 状态 |
|------|----|----|---------|------|
| Phase AB | AB-D1 | Octo 沙箱 Docker 镜像 | 基础串联完成 ✅ | 🟢 可实施 |
| Phase AB | AB-D2 | E2B provider 完整实现 | External trait 稳定 ✅ | 🟢 可实施 |
| Phase AB | AB-D3 | WASM 插件加载框架 | WASM 路由激活 | ⏳ |
| Phase AB | AB-D4 | Session Sandbox 持久化 | BashTool 沙箱集成 ✅ | 🟢 可实施 |
| Phase AB | AB-D5 | CredentialResolver → 沙箱 env 注入 | Z-D1 完成 | ⏳ |
| Phase AB | AB-D6 | gVisor / Firecracker provider | External trait 稳定 ✅ | 🟢 可实施 |
| Phase AA | AA-D1 | `octo auth login/status/logout` | UX 设计 | ⏳ |
| Phase AA | AA-D3 | XDG Base Directory 支持 | 低优先级 | ⏳ |
| Phase AA | AA-D4 | Config 热重载 | 未来增强 | ⏳ |
| Phase Z | Z-D1 | CredentialResolver → provider chain | Config 加载稳定 | 🟡 |
| Phase U | U-D1 | Agent Debug Panel 重设计 | 前置已满足 | 🟢 可实施 |
| Phase S | S-D1 | Agent Skills 规范研究 | 前置已满足 | 🟢 可实施 |

---

## 关键代码路径

| 文件 | 作用 |
|------|------|
| `crates/octo-engine/src/sandbox/profile.rs` | SandboxProfile 枚举 + resolve() |
| `crates/octo-engine/src/sandbox/run_mode.rs` | OctoRunMode 自动检测 |
| `crates/octo-engine/src/sandbox/target.rs` | ExecutionTargetResolver 路由引擎 |
| `crates/octo-engine/src/sandbox/external.rs` | ExternalSandboxProvider trait |
| `crates/octo-engine/src/tools/bash.rs` | BashTool 沙箱路由集成 |
| `crates/octo-cli/src/commands/sandbox.rs` | CLI 诊断命令 |

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

# 沙箱诊断
cargo run -p octo-cli -- sandbox status
cargo run -p octo-cli -- sandbox dry-run
```
