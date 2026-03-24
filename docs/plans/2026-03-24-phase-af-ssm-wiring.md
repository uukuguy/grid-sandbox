# Phase AF — SessionSandboxManager Wiring

**创建时间**: 2026-03-24
**状态**: 待执行
**基线**: 2467 tests @ `ee4986f`
**前置**: Phase AE (Agent Workspace Architecture) COMPLETE

---

## 目标

将 `SessionSandboxManager` (SSM) 从 `None` 占位符改为真实实例注入到 `AgentExecutor`，使容器沙箱执行路径端到端可用。

### 当前状态

```
AgentRuntime::start_or_get_primary()
  └→ AgentExecutor::new(..., None)  // ← AC-T7 占位 None
       └→ self.session_sandbox = None
            └→ BashTool 永远走 Local 路径
```

### 目标状态

```
AgentRuntime (持有 Option<Arc<SSM>>)
  └→ start_or_get_primary()
       └→ AgentExecutor::new(..., self.ssm.clone())
            └→ BashTool::with_session_sandbox(...)
                 └→ 根据 Profile 路由到容器执行
```

---

## 影响范围

| 文件 | 改动类型 | 说明 |
|------|----------|------|
| `crates/octo-engine/src/agent/runtime.rs` | 修改 | 添加 `session_sandbox` 字段，构造 SSM，传入 executor |
| `crates/octo-engine/src/sandbox/mod.rs` | 无改动 | SSM 已完整实现，直接使用 |
| `crates/octo-engine/src/sandbox/docker.rs` | 无改动 | DockerAdapter 已完整 |
| `crates/octo-engine/src/agent/executor.rs` | 无改动 | 已接受 `Option<Arc<SSM>>`，无需修改 |
| `crates/octo-engine/src/tools/bash.rs` | 无改动 | `with_session_sandbox()` 构造器已存在 |

---

## 任务清单

### G1: AgentRuntime 注入 SSM

**AF-T1**: AgentRuntime 添加 `session_sandbox` 字段

- 文件: `crates/octo-engine/src/agent/runtime.rs`
- 在 `AgentRuntime` struct 添加 `pub(crate) session_sandbox: Option<Arc<SessionSandboxManager>>`
- 在 `AgentRuntime::new()` 中：
  - 检测 `OctoRunMode`
  - 如果 Host 模式且 `SandboxProfile != Development`，尝试创建 `DockerAdapter` → `SessionSandboxManager`
  - 如果 Docker 不可用或 Development 模式，保持 `None`
- 将 `self.session_sandbox.clone()` 传入 `AgentExecutor::new()` 替换 `None`

**AF-T2**: AgentRuntimeConfig 添加沙箱配置

- 文件: `crates/octo-engine/src/agent/runtime.rs`
- 在 `AgentRuntimeConfig` 添加：
  - `pub sandbox_profile: Option<String>` — 允许通过配置指定 profile
- `SandboxProfile::resolve()` 已支持 env var 和 config fallback，只需传入

### G2: 验证 + 测试

**AF-T3**: 编译验证 + 现有测试不回归

- `cargo check --workspace`
- `cargo test --workspace -- --test-threads=1` 确认 2467 tests 全部通过
- Development profile 下 SSM = None，行为不变

---

## Deferred Items

| ID | 描述 | 原因 |
|----|------|------|
| AF-D1 | BashTool 自动从 AgentExecutor 获取 SSM 并构建 `with_session_sandbox` | 当前 BashTool 在 ToolRegistry 中是全局共享的，per-session wiring 需要 ToolRegistry per-session 化 | ✅ 已补 @ 6d99361 — AgentExecutor.tools_snapshot 中 per-session 替换 BashTool |

---

## 执行顺序

```
AF-T1 + AF-T2 (同一文件) → AF-T3 (验证)
```

单线性依赖，3 个 tasks，预计 30 分钟。
