# Phase AC — 沙箱容器（Sandbox Container）

> 将 container.sample 参考实现转化为正式 Octo 沙箱 Docker 镜像，并实现 per-session 容器复用。
> 对应 Deferred: AB-D1（Docker 镜像构建）+ AB-D4（Session Sandbox 持久化）

## 背景

Phase AB 建立了完整的沙箱路由基础设施：SandboxProfile / OctoRunMode / ExecutionTargetResolver / BashTool sandbox routing。但当前 Docker 后端依赖硬编码的 `octo-sandbox/*` 镜像（ImageRegistry），镜像本身不存在；`container.sample/` 只是参考模板，不被 octo-engine 使用。

同时，当前所有沙箱执行都是 **Ephemeral** 模式（每次 create → execute → destroy），`SandboxRef::Session` 路径从未被使用。这导致 Staging/Production 模式下每次工具调用都有容器启动开销（~2-5s）。

本阶段目标：
1. **AB-D1**: 将 `container.sample/` 升级为正式 `container/` 目录，产出可用的 `octo-sandbox:base` Docker 镜像
2. **AB-D4**: 实现 SessionSandbox — per-session 容器复用，一个 session 内所有工具调用共享同一容器

## 设计约束

1. **container.sample 是起点，不是终点** — 参考镜像为 Claude Code 定制，Octo 镜像需要精简（去掉 claude-code CLI / agent-runner / plugins）
2. **SessionData.sandbox_id 已存在** — `session/mod.rs:14` 已有 `sandbox_id: SandboxId`，需要接通
3. **DockerAdapter 已有完整 Bollard 集成** — create/execute/destroy 已实现，只需增加 session 生命周期管理
4. **SandboxRef::Session 已定义** — `target.rs:41` 已有类型，ExecutionTargetResolver 当前只产出 Ephemeral
5. **不改变 Tool/SkillRuntime trait 接口** — 所有改动在 sandbox 模块和 BashTool 内部

## 基线

- **Tests**: 2446 @ commit `d813625`
- **Branch**: `main`

---

## Task 分组

### G1: Octo 沙箱 Docker 镜像（AB-D1）

#### AC-T1: 创建 `container/` 正式目录结构

**目标**: 从 `container.sample/` 派生 Octo 专用沙箱镜像目录

**产出文件**: `container/Dockerfile`, `container/.dockerignore`, `container/scripts/`

**设计**:
- 基于 `container.sample/Dockerfile.base` 精简：
  - **保留**: Stage 1 (base-system: OS packages + Python + CLI tools) — 这是通用工具执行环境
  - **去掉**: Stage 2 (claude-code, bun, AI CLI tools, playwright) — Octo 不需要
  - **去掉**: Stage 3 (agent-runner, plugins, mcp config) — Octo 有自己的 agent runtime
  - **新增**: Octo 特定标记 `LABEL octo.sandbox=true`，用于容器识别
  - **新增**: `/workspace/project` + `/workspace/session` 目录结构（沿用 container.sample 约定）
  - **新增**: 健康检查脚本 (`HEALTHCHECK`)
- 镜像名称: `octo-sandbox:base`（与 ImageRegistry 默认值对齐）
- 用户: 非 root (`sandbox` user, UID 1000)
- Entrypoint: `["sleep", "infinity"]` — 保持容器运行，等待 exec 调用

**验证**: `docker build -t octo-sandbox:base container/` 成功

#### AC-T2: ImageRegistry 动态化 + 镜像可用性检查

**目标**: ImageRegistry 从硬编码升级为可配置，增加镜像存在性校验

**修改文件**: `crates/octo-engine/src/sandbox/docker.rs`

**设计**:
- `ImageRegistry::new()` 增加 `custom_images: HashMap<String, String>` 参数
- 新增 `ImageRegistry::from_config(config: &SandboxImageConfig)` — 从配置文件加载
- 新增 `SandboxImageConfig` 结构体（在 `octo-types/src/sandbox.rs`）：
  ```rust
  pub struct SandboxImageConfig {
      pub default_image: String,  // "octo-sandbox:base"
      pub language_images: HashMap<String, String>,  // language -> image 覆盖
  }
  ```
- `DockerAdapter::is_image_available(image: &str) -> bool` — Bollard inspect_image 检查
- 启动时 log warning 如果默认镜像不存在

**验证**: 单元测试验证 custom image 覆盖 + 默认 fallback

#### AC-T3: `octo sandbox build` CLI 命令

**目标**: 一键构建 Octo 沙箱镜像

**修改文件**: `crates/octo-cli/src/commands/sandbox.rs`, `crates/octo-cli/src/commands/types.rs`

**设计**:
- 新增 `SandboxCommands::Build` 子命令：
  ```
  octo sandbox build [--tag TAG] [--no-cache] [--dev]
  ```
  - `--tag`: 镜像标签，默认 `octo-sandbox:base`
  - `--no-cache`: 无缓存构建
  - `--dev`: 使用 Dockerfile.dev（含 Rust 工具链）
- 实现: 调用 `docker build -t <tag> container/`（通过 `std::process::Command`，不用 Bollard — build 是一次性操作）
- 构建前检查 `container/Dockerfile` 是否存在
- 输出构建进度（流式打印 docker build 输出）

**验证**: 命令解析测试 + 参数验证测试

### G2: Session Sandbox 生命周期（AB-D4）

#### AC-T4: SessionSandboxManager — 容器池管理

**目标**: 管理 per-session Docker 容器的创建、复用、销毁

**新增文件**: `crates/octo-engine/src/sandbox/session_sandbox.rs`

**设计**:
```rust
pub struct SessionSandboxManager {
    docker: Arc<DockerAdapter>,
    // session_id -> (container_id, last_used, created_at)
    containers: Arc<RwLock<HashMap<String, SessionContainer>>>,
    config: SessionSandboxConfig,
}

pub struct SessionContainer {
    pub container_id: String,
    pub sandbox_id: SandboxId,
    pub created_at: Instant,
    pub last_used: Instant,
    pub execution_count: u64,
}

pub struct SessionSandboxConfig {
    pub image: String,                // 默认 "octo-sandbox:base"
    pub idle_timeout: Duration,       // 空闲超时，默认 30min
    pub max_lifetime: Duration,       // 最大生命周期，默认 4h
    pub max_containers: usize,        // 最大并发容器数，默认 5
    pub working_dir: String,          // 容器内工作目录，默认 "/workspace/session"
    pub mount_project: bool,          // 是否挂载项目目录（只读），默认 true
}
```

**核心方法**:
- `get_or_create(session_id: &str) -> Result<SandboxId>` — 复用已有容器或创建新容器
- `execute(session_id: &str, command: &str) -> Result<ExecResult>` — 在 session 容器中执行
- `release(session_id: &str) -> Result<()>` — 主动释放容器
- `cleanup_idle() -> usize` — 清理超时容器（由定时器调用）
- `shutdown() -> Result<()>` — 优雅关闭所有容器

**容器创建配置**:
- Image: 从 SessionSandboxConfig.image
- Volumes: `$PWD:/workspace/project:ro`（如果 mount_project=true）
- Labels: `octo-sandbox=true`, `octo-session-id=<session_id>`
- Cmd: `["sleep", "infinity"]`
- AutoRemove: false（由 SessionSandboxManager 管理生命周期）

**验证**: 单元测试（mock DockerAdapter）+ 集成测试标记 `#[cfg(feature = "sandbox-docker")]`

#### AC-T5: ExecutionTargetResolver 支持 Session 路由

**目标**: Staging/Production 模式支持 Session 容器复用

**修改文件**: `crates/octo-engine/src/sandbox/target.rs`

**设计**:
- `ExecutionTargetResolver` 新增 `session_id: Option<String>` 字段
- `new()` 签名不变（向后兼容），新增 `with_session(session_id: String) -> Self`
- 路由逻辑变更：
  - 当 `session_id` 存在 + Docker 后端可用 → 返回 `SandboxRef::Session { id }` 替代 `Ephemeral`
  - 当 `session_id` 为 None → 行为不变（Ephemeral）
  - Wasm / External 仍用 Ephemeral（这些后端自带轻量隔离）
- `RoutingPreview` 增加 `session_id: Option<String>` 字段

**验证**: 扩展现有测试 + 新增 session 路由测试

#### AC-T6: BashTool 接入 SessionSandboxManager

**目标**: BashTool 在 Staging/Production 模式下使用 session 容器执行

**修改文件**: `crates/octo-engine/src/tools/bash.rs`

**设计**:
- `BashTool` 新增 `session_sandbox: Option<Arc<SessionSandboxManager>>` 字段
- `with_sandbox()` 签名扩展: `with_sandbox(run_mode, profile, router, session_sandbox)`
  - `session_sandbox` 为 `Option<Arc<SessionSandboxManager>>`
  - 当有 session_sandbox 时，将 session_id 传入 ExecutionTargetResolver
- `execute()` 流程变更：
  ```
  1. resolver.resolve(Shell) → ExecutionTarget
  2. if Sandbox(Session { id }) → session_sandbox.execute(id, command)
  3. if Sandbox(Ephemeral { .. }) → 现有 execute_via_sandbox 流程
  4. if Local → 现有 execute_local 流程
  ```
- 向后兼容：`session_sandbox = None` 时行为与 Phase AB 完全一致

**验证**: 单元测试 + 现有 BashTool 测试不回归

### G3: 集成与可观测

#### AC-T7: AgentExecutor 注入 SessionSandboxManager

**目标**: 将 SessionSandboxManager 生命周期与 AgentExecutor（per-session）对齐

**修改文件**: `crates/octo-engine/src/agent/executor.rs`（或相关初始化路径）

**设计**:
- AgentExecutor 创建时，如果 SandboxProfile != Development：
  - 检查 Docker 是否可用（`DockerAdapter::is_available()`）
  - 如果可用 → 创建 `SessionSandboxManager`，传入 session_id
  - 将 `SessionSandboxManager` 传给 BashTool.with_sandbox()
- AgentExecutor drop / session 结束时 → 调用 `session_sandbox.release(session_id)`
- 优雅降级：Docker 不可用 → 不创建 SessionSandboxManager，BashTool 走 Ephemeral/Local

**验证**: 集成测试验证 session 生命周期

#### AC-T8: 容器清理定时器 + `octo sandbox cleanup` 命令

**目标**: 自动清理空闲容器 + 手动清理命令

**修改文件**:
- `crates/octo-engine/src/sandbox/session_sandbox.rs`（定时器）
- `crates/octo-cli/src/commands/sandbox.rs`（CLI）

**设计**:
- 定时器: `SessionSandboxManager::start_cleanup_timer()` — 每 5 分钟检查 idle_timeout
- CLI 命令:
  ```
  octo sandbox cleanup [--force] [--session SESSION_ID]
  ```
  - 无参数: 清理所有超时容器
  - `--force`: 强制清理所有 Octo 沙箱容器（通过 label 过滤）
  - `--session`: 清理指定 session 的容器
- 实现: 通过 Bollard `list_containers` + label filter `octo-sandbox=true`

**验证**: 清理逻辑单元测试 + CLI 参数测试

#### AC-T9: ToolExecution 记录 session container 信息

**目标**: 执行遥测记录 session 容器复用情况

**修改文件**: `crates/octo-types/src/execution.rs`, `crates/octo-engine/src/tools/recorder.rs`

**设计**:
- `ToolExecution` 新增:
  ```rust
  pub sandbox_session_id: Option<String>,    // session container ID
  pub sandbox_container_reused: Option<bool>, // 是否复用已有容器
  ```
- `ToolExecutionRecorder` 填充这两个字段
- TUI ExecutionDetail 面板显示容器复用状态

**验证**: recorder 测试验证新字段填充

---

## Deferred（暂缓项）

> 本阶段已知但暂未实现的功能点。每次开始新 Task 前先检查此列表。

| ID | 内容 | 前置条件 | 状态 |
|----|------|---------|------|
| AC-D1 | CI/CD 镜像构建流水线（GitHub Actions） | AC-T1 Dockerfile 稳定 | ⏳ |
| AC-D2 | 容器资源限制（memory/cpu cgroup 配额） | AC-T4 基础功能验证 | ✅ 已补 @ f452f5c |
| AC-D3 | 容器网络隔离（自定义 Docker network） | 安全需求评估 | ✅ 已补 @ f452f5c |
| AC-D4 | 多镜像支持（按 language/tool 选择不同镜像） | AC-T2 ImageRegistry 稳定 | ⏳ |
| AC-D5 | 容器文件系统快照/恢复 | session sandbox 生产验证 | ⏳ |
| AC-D6 | Docker Compose 编排（多容器协同） | 多 Agent 协作场景 | ⏳ |

---

## 风险与注意事项

1. **Docker daemon 依赖**: 开发环境可能没有 Docker → Development profile 不受影响，测试用 `#[cfg(feature = "sandbox-docker")]` 隔离
2. **container.sample 保留**: 不删除 `container.sample/`，它是 Claude Code 集成的参考实现；`container/` 是 Octo 专用
3. **容器启动延迟**: 首次创建 ~2-5s，复用后 exec 延迟 ~100ms — Session 模式大幅改善体验
4. **资源泄露**: 进程崩溃可能留下孤儿容器 → AC-T8 cleanup 命令 + label 过滤兜底
5. **向后兼容**: 所有新增字段为 Option<T>，所有 with_* 构造器保留无参版本

## 验证标准

- [ ] `docker build -t octo-sandbox:base container/` 成功
- [ ] `cargo check --workspace` 通过
- [ ] `cargo test --workspace -- --test-threads=1` 全部通过，无回归
- [ ] `octo sandbox build` 能构建镜像
- [ ] `octo sandbox status` 显示镜像可用状态
- [ ] `octo sandbox cleanup` 能清理孤儿容器
- [ ] Development profile 行为与 Phase AB 完全一致（零回归）
- [ ] Staging profile 下工具调用复用 session 容器

## 依赖关系

```
AC-T1 (Dockerfile) ─────────────────────────────────┐
AC-T2 (ImageRegistry) ──┐                           │
AC-T3 (CLI build) ──────┤                           │
                        │                           │
AC-T4 (SessionSandboxManager) ──┐                   │
AC-T5 (Resolver session) ───────┤                   │
                                ↓                   │
AC-T6 (BashTool session) ───────┤                   │
                                ↓                   ↓
AC-T7 (AgentExecutor inject) ──────────────────────→ 依赖 T4+T6
AC-T8 (Cleanup timer+CLI) ─────────────────────────→ 依赖 T4
AC-T9 (Telemetry) ─────────────────────────────────→ 依赖 T6
```

**并行可能性**:
- G1 (T1, T2, T3) 和 G2 (T4, T5) 可并行
- T6 依赖 T4 + T5
- T7, T8, T9 依赖 T6
