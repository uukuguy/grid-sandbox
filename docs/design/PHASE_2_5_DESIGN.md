# Phase 2.5 设计实施方案

**版本**: v1.0
**创建日期**: 2026-03-01
**目标**: octo-engine 对标 Claude Code / OpenClaw

---

## 一、当前状态与目标差距

### 1.1 octo-engine 现有能力

| 模块 | 状态 | 说明 |
|------|------|------|
| Agent Loop | ✅ | 10轮，可配置，Loop Guard |
| Provider | ✅ | Anthropic + OpenAI |
| Tools | ✅ | 12个内置工具 |
| 沙箱 | ⚠️ | Subprocess（仅开发） |
| 认证 | ❌ | 无 |
| RBAC | ❌ | 无 |
| 记忆系统 | ✅ | 3层（Working/Session/Persistent） |
| MCP | ✅ | Stdio + SSE |
| Skills | ✅ | 热重载 |
| EventBus | ✅ | 事件广播 |
| Security | ✅ | ActionTracker + 路径检查 |
| Extension | ✅ | 生命周期钩子 |
| Queue | ✅ | Steering/FollowUp模式 |

### 1.2 对标 Claude Code / OpenClaw 差距

| 能力维度 | 当前 | 目标 | 差距等级 |
|----------|------|------|----------|
| **沙箱隔离** | Subprocess | WASM + Docker | 🔴 大 |
| **认证** | 无 | API Key（可选） | 🔴 大 |
| **用户隔离** | 无 | User ID 隔离 | 🔴 大 |
| **LLM 多实例** | 单实例 | 多实例故障切换 | 🟠 中 |
| **Provider Chain** | 无 | 自动 failover | 🟠 中 |
| **定时任务** | 空白 | Cron + Queue | 🔴 大 |
| **Metrics** | 空白 | Prometheus | 🟠 中 |
| **Agent Loop** | 10轮 | 50轮/无限 | 🟠 中 |

---

## 二、octo-engine vs octo-platform 能力区分

### 2.1 核心原则

- **octo-engine**: 独立智能体，可运行在客户端或企业服务器
- **octo-platform**: 多用户平台，以 octo-engine 为底层智能体

### 2.2 能力矩阵

| 能力 | octo-engine | octo-platform | 说明 |
|------|-------------|---------------|------|
| **运行模式** | 单机 / 集群 | 集群 | engine 默认单机 |
| **认证** | 可选 API Key | 必须 OAuth2/API Key | engine 默认无 |
| **用户隔离** | 可选 User ID | 必须 | engine 可禁用 |
| **沙箱** | WASM + Docker | 同上 + 多租户隔离 | 共享 |
| **Provider 多实例** | 可选 | 必须 | engine 可单实例 |
| **Scheduler** | 可选 | 必须 | engine 可禁用 |
| **Metrics** | 本地 | 聚合 + 多用户 | engine 简化版 |
| **RBAC** | 无 | Admin/Developer/Viewer | engine 无 |

### 2.3 认证设计

```
┌─────────────────────────────────────────────────────────────┐
│                    octo-engine 认证模式                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   模式 A: 无认证（默认）                                      │
│   ┌─────────────────┐                                       │
│   │   单机模式       │  无用户隔离，无 API Key 检查          │
│   │   Local Only    │                                       │
│   └─────────────────┘                                       │
│                                                             │
│   模式 B: 简化认证（企业服务器）                              │
│   ┌─────────────────┐                                       │
│   │   API Key 模式   │  User ID 隔离，Key 验证              │
│   │   Enterprise    │  可选 RBAC（企业级单租户）            │
│   └─────────────────┘                                       │
│                                                             │
│   模式 C: 完整认证（octo-platform）                          │
│   ┌─────────────────┐                                       │
│   │  OAuth2/API Key │  多用户，完整 RBAC                     │
│   │  Multi-Tenant   │  租户隔离，完整审计                    │
│   └─────────────────┘                                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 三、Phase 2.5 详细设计

### 3.1 阶段拆分

由于任务量大，将 Phase 2.5 拆分为 **4 个子阶段**：

| 阶段 | 主题 | 核心模块 | 估算 | 参考项目 |
|------|------|----------|------|----------|
| **Phase 2.5** | 核心基础设施 | 沙箱 + 认证 + 用户隔离 | ~1800 LOC | openfang |
| **Phase 2.6** | Provider + Scheduler | 多实例 + 定时任务 | ~800 LOC | openfang |
| **Phase 2.7** | 可观测性 | Metrics + 审计 | ~500 LOC | openfang |
| **Phase 2.8** | Agent 增强 | Loop 增强 + Secret | ~400 LOC | openclaw |

### 3.2 模块总览

| 阶段 | 模块 | 目标 | 参考项目 |
|------|------|------|----------|
| **2.5** | 沙箱系统 (WASM + Docker) | 替换不安全 Subprocess | openfang: `runtime/sandbox.rs`, `docker_sandbox.rs` |
| **2.5** | 简化认证系统 | API Key 模式 | openfang: `kernel/auth.rs` |
| **2.5** | 用户隔离 | User ID 过滤 | openfang |
| **2.6** | LLM Provider 多实例 | (base_url, key, model) 故障切换 | openfang: `runtime/provider_health.rs`, `drivers/fallback.rs` |
| **2.6** | Cron Scheduler | 定时任务 | openfang: `kernel/scheduler.rs`, `kernel/cron.rs` |
| **2.7** | Metrics | Prometheus | openfang: `kernel/metering.rs` |
| **2.7** | 增强审计 | 完整日志 | openfang: `runtime/audit.rs` |
| **2.8** | Agent Loop 增强 | 50轮/无限 | openclaw: `runtime/agent_loop.rs` |
| **2.8** | Secret Manager | 加密存储 | openfang |

---

### 3.3 Phase 2.5: 核心基础设施

#### 3.3.1 设计目标

- 替代当前不安全的 Subprocess 沙箱
- WASM：轻量级计算任务（毫秒级启动）
- Docker：完整 Agent 运行环境（生产环境首选）

**参考**: openfang `crates/openfang-runtime/src/sandbox.rs`, `docker_sandbox.rs`, `subprocess_sandbox.rs`

#### 3.3.2 架构

```
                    工具调用请求
                          │
                          ▼
              ┌─────────────────────────┐
              │     Tool Registry       │
              │   (判断工具类型)        │
              └───────────┬─────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
   ┌─────────┐      ┌─────────┐      ┌─────────┐
   │  WASM   │      │ Docker  │      │Subprocess│
   │  沙箱   │      │  沙箱  │      │ (仅测试) │
   └─────────┘      └─────────┘      └─────────┘
      ▲                 ▲
      │                 │
      └──── 生产环境 ────┘
```

#### 3.2.3 RuntimeAdapter Trait

```rust
// crates/octo-engine/src/sandbox/mod.rs

/// 沙箱类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxType {
    /// WASM 沙箱 - 轻量计算
    Wasm,
    /// Docker 沙箱 - 完整环境
    Docker,
    /// Subprocess - 仅开发/测试
    Subprocess,
}

/// 沙箱配置
pub struct SandboxConfig {
    pub sandbox_type: SandboxType,
    pub max_memory_mb: usize,      // 默认 16MB (WASM) / 512MB (Docker)
    pub max_fuel: u64,             // WASM 指令限制
    pub max_duration_secs: u64,    // 超时时间
    pub allowed_paths: Vec<PathBuf>,  // 允许访问的路径
    pub env_vars: HashMap<String, String>,  // 环境变量
}

/// 执行结果
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// 沙箱运行时适配器 Trait
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    fn runtime_type(&self) -> SandboxType;

    async fn create(&self, config: &SandboxConfig) -> Result<SandboxId>;

    async fn execute(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult>;

    async fn destroy(&self, id: &SandboxId) -> Result<()>;
}
```

#### 3.2.4 WASM 沙箱实现

```rust
// crates/octo-engine/src/sandbox/wasm.rs

pub struct WasmAdapter {
    engine: Engine,
    max_memory: usize,      // 默认 16MB
    max_fuel: u64,          // 默认 1M 指令
    max_duration: Duration,  // 默认 30s
}

#[async_trait]
impl RuntimeAdapter for WasmAdapter {
    fn runtime_type(&self) -> SandboxType {
        SandboxType::Wasm
    }

    async fn create(&self, config: &SandboxConfig) -> Result<SandboxId> {
        // 编译 WASM 模块
    }

    async fn execute(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult> {
        // Fuel 计量 + Epoch 中断执行
    }
}
```

#### 3.2.5 Docker 沙箱实现

```rust
// crates/octo-engine/src/sandbox/docker.rs

pub struct DockerAdapter {
    client: Docker,
    max_containers: usize,
    image: String,
    workspace_mount: PathBuf,
}

#[async_trait]
impl RuntimeAdapter for DockerAdapter {
    fn runtime_type(&self) -> SandboxType {
        SandboxType::Docker
    }

    async fn create(&self, config: &SandboxConfig) -> Result<SandboxId> {
        // 1. 并发配额检查
        // 2. 创建容器 (内存/CPU 限制)
        // 3. 挂载安全验证
    }

    async fn execute(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult> {
        // 在容器中执行命令
    }
}
```

#### 3.2.6 配置示例

```yaml
# config.yaml
sandbox:
  # 默认沙箱类型
  default_type: "wasm"  # wasm / docker / subprocess

  wasm:
    max_memory_mb: 16
    max_fuel: 1000000    # 1M 指令
    max_duration_secs: 30

  docker:
    image: "octo-sandbox:latest"
    max_memory_mb: 512
    max_cpu_shares: 256
    max_containers: 10
    workspace_mount: "/workspace"

  # 工具类型映射到沙箱
  tool_mapping:
    bash: "docker"        # 需要完整环境
    file_read: "wasm"     # 只读计算
    file_write: "docker"  # 需要写入
    glob: "wasm"          # 只读计算
    grep: "wasm"          # 只读计算
    web_fetch: "wasm"     # 网络请求
```

#### 3.2.7 实施任务

| 任务 | 估算 | 依赖 |
|------|------|------|
| RuntimeAdapter Trait 抽象 | 150 LOC | - |
| WasmAdapter 实现 | 350 LOC | wasmtime |
| Fuel Metering | 50 LOC | - |
| DockerAdapter 实现 | 400 LOC | bollard |
| 并发控制 | 100 LOC | - |
| 挂载安全 | 100 LOC | - |
| Tool -> Sandbox 路由 | 50 LOC | - |
| **总计** | **~1,200 LOC** | |

---

---

## Phase 2.6: Provider + Scheduler

### 3.4 P0: LLM Provider 多实例

**参考**: openfang `crates/openfang-runtime/src/provider_health.rs`, `drivers/fallback.rs`

#### 3.4.1 设计目标

- 同一 Provider 类型配置多个 API Key
- 主实例故障时自动切换到备用实例
- 支持按成本/延迟选择实例

#### 3.4.2 数据结构

```rust
// crates/octo-engine/src/providers/mod.rs

/// LLM 实例配置
///
/// Instance 由 (provider, base_url, api_key, model) 四元组唯一确定
/// 不同 base_url + model 组合是不同的实例（如 Anthropic API vs Vertex）
pub struct LlmInstance {
    pub id: String,
    pub provider: ProviderType,        // anthropic / openai
    pub api_key: String,              // 明文或 Secret 引用
    pub base_url: Option<String>,      // 自定义端点 (如 Anthropic Vertex)
    pub model: String,                // 模型名 (如 claude-3-opus)
    pub priority: u8,                // 优先级 (0 最高)
    pub max_rpm: Option<u32>,         // 请求速率限制
    pub max_tpm: Option<u32>,         // token 速率限制
}

impl LlmInstance {
    /// 生成实例的唯一标识
    pub fn identity(&self) -> String {
        format!(
            "{}:{}+{}+{}",
            self.provider.as_str(),
            self.base_url.as_deref().unwrap_or("api"),
            self.model,
            // 不包含 key，只用于标识
            "***"
        )
    }
}

/// LLM Provider 链
pub struct ProviderChain {
    instances: Vec<LlmInstance>,
    health_check_interval: Duration,
}

impl ProviderChain {
    /// 获取可用的最高优先级实例
    pub async fn get_available(&self) -> Result<&LlmInstance>;

    /// 标记实例不健康，切换到备用
    pub async fn mark_unhealthy(&mut self, instance_id: &str);

    /// 健康检查定时任务
    pub async fn health_check(&mut self);
}
```

#### 3.2.3 故障切换逻辑

```
请求到达
    │
    ▼
选择最高优先级实例
    │
    ▼
执行请求
    │
    ├── 成功 ──▶ 返回结果
    │
    └── 失败 ──▶ 检查错误类型
                    │
                    ├── 可重试错误 (429, 500, 503)
                    │       │
                    │       ▼
                    │   标记当前实例不健康
                    │       │
                    │       ▼
                    │   切换到下一优先级实例
                    │       │
                    │       ▼
                    │   重试 (最多 3 次)
                    │
                    └── 不可重试错误 (401, 403)
                            │
                            ▼
                        直接返回错误
```

#### 3.2.4 配置示例

```yaml
# config.yaml

# Instance = (provider, base_url, api_key, model) 唯一确定
# 同一个 provider 可以有不同 base_url（如 API vs Vertex）

providers:
  anthropic:
    instances:
      # 实例 1: 标准 API + opus
      - id: "claude-opus-primary"
        api_key: "${ANTHROPIC_API_KEY_1}"
        base_url: "https://api.anthropic.com"  # 默认
        model: "claude-3-opus-20240229"
        priority: 0
        max_rpm: 50

      # 实例 2: Vertex + sonnet（不同 base_url + model = 不同实例）
      - id: "claude-sonnet-vertex"
        api_key: "${GOOGLE_APPLICATION_CREDENTIALS}"
        base_url: "https://anthropic-vertex.googleapis.com"
        model: "claude-3-5-sonnet-20241022"
        priority: 1
        max_rpm: 100

      # 实例 3: 另一个 API Key 备份
      - id: "claude-opus-backup"
        api_key: "${ANTHROPIC_API_KEY_2}"
        base_url: "https://api.anthropic.com"
        model: "claude-3-opus-20240229"
        priority: 2
        max_rpm: 50

  openai:
    instances:
      # 实例 1: 标准 API
      - id: "gpt4-primary"
        api_key: "${OPENAI_API_KEY}"
        base_url: "https://api.openai.com/v1"
        model: "gpt-4-turbo"
        priority: 0

      # 实例 2: Azure OpenAI
      - id: "gpt4-azure"
        api_key: "${AZURE_OPENAI_KEY}"
        base_url: "${AZURE_OPENAI_ENDPOINT}"
        model: "gpt-4-turbo"
        priority: 1

# 故障切换配置
failover:
  max_retries: 3
  retry_delay_ms: 1000
  health_check_interval_sec: 30
```

---

### 3.5 P0: 简化认证系统

**参考**: openfang `crates/openfang-kernel/src/auth.rs`

#### 3.3.1 设计目标

- 可选的 API Key 认证模式
- 向后兼容：无认证模式仍可用
- 企业级单租户支持

#### 3.3.2 认证模式

```rust
// crates/octo-engine/src/auth/mod.rs

/// 认证模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// 无认证（默认）
    None,
    /// API Key 模式
    ApiKey,
    /// 完整认证（保留给 octo-platform）
    Full,
}

/// 认证配置
pub struct AuthConfig {
    pub mode: AuthMode,
    pub api_keys: Vec<ApiKey>,        // 预配置的 API Key
    pub require_user_id: bool,        // 是否要求用户隔离
}

pub struct ApiKey {
    pub key: String,                  // sha256 哈希存储
    pub user_id: Option<String>,      // 可选用户绑定
    pub permissions: Vec<Permission>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
    Admin,
}
```

#### 3.3.3 认证中间件

```rust
// crates/octo-engine/src/auth/middleware.rs

/// 认证中间件
pub async fn auth_middleware(
    req: Request,
    next: Next,
    config: &AuthConfig,
) -> Result<Response, StatusCode> {
    match config.mode {
        AuthMode::None => {
            // 无认证模式，直接放行
            next.run(req).await
        }
        AuthMode::ApiKey => {
            // 验证 API Key
            let key = req.headers()
                .get("X-API-Key")
                .and_then(|v| v.to_str().ok());

            match key {
                Some(k) if config.validate_key(k) => {
                    // 验证通过，注入 user_id 到请求
                    let user_id = config.get_user_id(k);
                    let req = req.with_extension(user_id);
                    next.run(req).await
                }
                _ => Err(StatusCode::UNAUTHORIZED),
            }
        }
        AuthMode::Full => {
            // 完整认证（octo-platform 实现）
            unimplemented!("Use octo-platform for full auth")
        }
    }
}
```

#### 3.3.4 配置示例

```yaml
# config.yaml
auth:
  mode: "api_key"                    # none / api_key / full
  require_user_id: true              # 是否要求用户隔离

  api_keys:
    - key: "${API_KEY_HASH}"         # sha256 哈希
      user_id: "user-001"
      permissions: ["read", "write"]
      expires_at: null               # 永不过期

    - key: "${ADMIN_API_KEY_HASH}"
      user_id: "admin"
      permissions: ["read", "write", "admin"]
```

---

### 3.6 P0: 用户隔离

#### 3.4.1 设计目标

- 按 User ID 隔离数据
- 沙箱、记忆、MCP 配置等资源按用户隔离
- 支持资源级别的跨用户共享（可选）

#### 3.4.2 隔离范围

| 资源 | 隔离方式 | 可共享 |
|------|----------|--------|
| Session | User ID 过滤 | ❌ |
| Memory | User ID 过滤 | 可选 |
| MCP Server | User ID 关联 | ❌ |
| Skills | User ID 关联 | 可选 |
| Sandbox | User ID 关联 | ❌ |
| Metrics | User ID 标签 | ✅ (聚合) |

#### 3.4.3 实现

```rust
// crates/octo-engine/src/auth/context.rs

/// 用户上下文
pub struct UserContext {
    pub user_id: String,
    pub permissions: Vec<Permission>,
}

/// 带用户上下文的请求
pub trait UserScoped {
    fn user_id(&self) -> &Option<String>;
    fn set_user_id(&mut self, user_id: String);
}

/// 数据访问控制
pub fn filter_by_user<T>(items: Vec<T>, user_id: &Option<String>) -> Vec<T>
where
    T: UserScoped,
{
    match user_id {
        Some(uid) => items.into_iter()
            .filter(|item| item.user_id() == Some(uid))
            .collect(),
        None => items,  // 无用户上下文，返回全部（仅无认证模式）
    }
}
```

---

---

## Phase 2.6: Provider + Scheduler

### 3.7 P1: Cron Scheduler

**参考**: openfang `crates/openfang-kernel/src/scheduler.rs`, `kernel/cron.rs`

#### 3.5.1 设计目标

- 定时触发 Agent 执行
- 支持 Cron 表达式
- 任务持久化（支持重启恢复）

#### 3.5.2 核心结构

```rust
// crates/octo-engine/src/scheduler/mod.rs

/// 定时任务
pub struct ScheduledTask {
    pub id: String,
    pub user_id: Option<String>,
    pub cron: String,                    // Cron 表达式
    pub agent_config: AgentConfig,        // Agent 配置
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
}

/// 调度器
pub struct Scheduler {
    tasks: HashMap<String, ScheduledTask>,
    runtime: Runtime,                     // Tokio 运行时
}

impl Scheduler {
    /// 添加定时任务
    pub async fn add_task(&mut self, task: ScheduledTask) -> Result<()>;

    /// 移除任务
    pub async fn remove_task(&mut self, task_id: &str) -> Result<()>;

    /// 执行调度循环
    pub async fn run(&mut self) -> !;
}
```

#### 3.5.3 配置示例

```yaml
# config.yaml
scheduler:
  enabled: true

  tasks:
    - id: "daily-report"
      cron: "0 9 * * *"                 # 每天 9 点
      agent:
        system_prompt: "生成昨日报告"
        input: "请分析昨日数据并生成报告"
      enabled: true

    - id: "hourly-health"
      cron: "0 * * * *"                 # 每小时
      agent:
        system_prompt: "健康检查"
        input: "执行系统健康检查"
      enabled: false
```

---

---

## Phase 2.7: 可观测性

### 3.8 P1: Metrics

**参考**: openfang `crates/openfang-kernel/src/metering.rs`

#### 3.6.1 设计目标

- Prometheus 格式指标导出
- 关键指标：请求延迟、Token 消耗、错误率
- 可选：按 User ID 聚合

#### 3.6.2 指标定义

```rust
// crates/octo-engine/src/metrics/mod.rs

/// 指标定义
pub struct Metrics {
    // 请求指标
    pub requests_total: Counter,
    pub request_duration: Histogram,

    // Token 消耗
    pub tokens_used: Counter,

    // 错误指标
    pub errors_total: Counter,

    // Agent 指标
    pub agent_rounds_total: Counter,
    pub tool_executions_total: Counter,

    // 可选：用户维度
    pub user_requests: Counter,
}

impl Metrics {
    /// 记录请求
    pub fn record_request(&self, user_id: Option<&str>, duration_ms: u64);

    /// 记录 Token 消耗
    pub fn record_tokens(&self, user_id: Option<&str>, input: u64, output: u64);

    /// 记录错误
    pub fn record_error(&self, error_type: &str);
}
```

#### 3.6.3 导出端点

```rust
// crates/octo-server/src/api/metrics.rs

/// Prometheus 指标端点
pub async fn metrics() -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder.encode(&metric_families).unwrap()
}
```

---

---

## Phase 2.8: Agent 增强

### 3.9 P1: Agent Loop 增强

**参考**: openclaw `packages/agent/src/agent_loop.ts`

#### 3.7.1 目标

- 支持最多 50 轮对话（可配置）
- 支持无限轮模式（需配置）
- 增强 Loop Guard（更多检测维度）

#### 3.7.2 配置

```rust
// crates/octo-engine/src/agent/config.rs

pub struct AgentConfig {
    /// 最大对话轮数
    pub max_rounds: u32,          // 默认 50，0 表示无限

    /// 是否启用无限模式警告
    pub infinite_mode_warning: bool,

    /// 高级 Loop Guard 配置
    pub loop_guard: LoopGuardConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_rounds: 50,
            infinite_mode_warning: true,
            loop_guard: LoopGuardConfig::default(),
        }
    }
}
```

---

### 3.10 P2: 增强审计

**参考**: openfang `crates/openfang-runtime/src/audit.rs`

#### 3.8.1 目标

- 完整操作审计链
- JSONL 格式导出
- 可选：实时告警

#### 3.8.2 审计事件

```rust
// crates/octo-engine/src/audit/mod.rs

pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub action: AuditAction,
    pub resource: String,
    pub details: Value,
    pub result: AuditResult,
}

pub enum AuditAction {
    SessionCreate,
    SessionDelete,
    ToolExecute,
    MemoryWrite,
    MemoryRead,
    McpServerStart,
    McpServerStop,
    ConfigChange,
    AuthAttempt,
}

pub enum AuditResult {
    Success,
    Failure(String),
}
```

---

### 3.11 P2: Secret Manager

#### 3.9.1 目标

- 敏感信息加密存储
- 支持环境变量引用
- 支持用户级隔离

#### 3.9.2 设计

```rust
// crates/octo-engine/src/secret/mod.rs

/// 密钥管理器
pub struct SecretManager {
    store: encryption::EncryptedStore,
    master_key: [u8; 32],
}

impl SecretManager {
    /// 存储密钥
    pub async fn set(&self, key: &str, value: &str, user_id: Option<&str>) -> Result<()>;

    /// 获取密钥
    pub async fn get(&self, key: &str, user_id: Option<&str>) -> Result<String>;

    /// 解析配置中的密钥引用
    pub fn resolve(&self, config: &str) -> String {
        // ${SECRET:api_key} -> 解密后的值
    }
}
```

---

## 四、参考项目利用

### 4.1 openfang（主要参考）

| 模块 | 路径 | 可复用程度 |
|------|------|------------|
| Auth | `openfang-kernel/src/auth/` | 高（需简化） |
| RBAC | `openfang-kernel/src/auth/rbac.rs` | 中（需调整） |
| Scheduler | `openfang-kernel/src/scheduler/` | 高 |
| Workflow | `openfang-kernel/src/workflow/` | 中（参考设计） |
| Sandbox | `openfang-kernel/src/sandbox/` | 高 |
| Metrics | `openfang-kernel/src/metrics/` | 高 |

### 4.2 nanoclaw

| 模块 | 路径 | 可复用程度 |
|------|------|------------|
| Provider Chain | 参考其 Provider 切换逻辑 | 高 |
| MCP Bridge | `nanoclaw/src/mcp/` | 中 |

### 4.3 openclaw

| 模块 | 路径 | 可复用程度 |
|------|------|------------|
| Agent Loop | 参考其无限轮实现 | 中 |
| Context 管理 | `openclaw/packages/agent/src/` | 中 |

---

## 五、对 octo-platform 的影响与调整

### 5.1 架构调整

```
                    ┌─────────────────────────────────────────────┐
                    │              octo-platform                   │
                    │  ┌─────────────────────────────────────┐   │
                    │  │  Auth Layer (OAuth2)                │   │
                    │  │  - Multi-tenant                     │   │
                    │  │  - RBAC (Admin/Developer/Viewer)    │   │
                    │  └─────────────────────────────────────┘   │
                    │                    │                          │
                    │                    ▼                          │
                    │  ┌─────────────────────────────────────┐   │
                    │  │  Tenant Manager                     │   │
                    │  │  - 租户隔离                         │   │
                    │  │  - 配额管理                         │   │
                    │  └─────────────────────────────────────┘   │
                    │                    │                          │
                    └────────────────────┼──────────────────────────┘
                                         │
                    ┌────────────────────┼──────────────────────────┐
                    │                    ▼                          │
                    │  ┌─────────────────────────────────────┐      │
                    │  │       octo-engine (多实例)          │      │
                    │  │  ┌───────────────────────────────┐  │      │
                    │  │  │  Auth (API Key)              │  │      │
                    │  │  │  - User ID 隔离               │  │      │
                    │  │  │  - 可选认证                   │  │      │
                    │  │  └───────────────────────────────┘  │      │
                    │  │  ┌───────────────────────────────┐  │      │
                    │  │  │  Provider Chain              │  │      │
                    │  │  │  - 多实例故障切换            │  │      │
                    │  │  │  - 按租户分配                │  │      │
                    │  │  └───────────────────────────────┘  │      │
                    │  │  ┌───────────────────────────────┐  │      │
                    │  │  │  Scheduler                   │  │      │
                    │  │  │  - 多租户任务隔离            │  │      │
                    │  │  └───────────────────────────────┘  │      │
                    │  └─────────────────────────────────────┘      │
                    └─────────────────────────────────────────────┘
```

### 5.2 octo-platform 需要扩展的功能

| 功能 | octo-engine 提供 | octo-platform 扩展 |
|------|-----------------|-------------------|
| 认证 | API Key 模式 | OAuth2 / LDAP |
| RBAC | 无 | 三角色 + 租户 |
| 用户管理 | User ID 隔离 | 用户注册 / 邀请 |
| Provider | 多实例 | 租户配额 / 计费 |
| Scheduler | 单机任务 | 分布式任务 |
| Metrics | 基础指标 | 多租户聚合 |
| 审计 | 本地日志 | 集中式审计 |

### 5.3 数据层调整

#### 5.3.1 User ID 字段

所有表添加 `user_id` 字段（可选）：

```sql
-- sessions 表
ALTER TABLE sessions ADD COLUMN user_id TEXT;

-- memories 表
ALTER TABLE memories ADD COLUMN user_id TEXT;

-- mcp_servers 表
ALTER TABLE mcp_servers ADD COLUMN user_id TEXT;
```

#### 5.3.2 多租户隔离查询

```rust
fn get_sessions(user_id: &Option<String>) -> Vec<Session> {
    match user_id {
        Some(uid) => query!("SELECT * FROM sessions WHERE user_id = ?", uid),
        None => query!("SELECT * FROM sessions"),
    }
}
```

---

## 六、实施计划

### 6.1 任务拆分

| 阶段 | 任务 | 估算 | 优先级 |
|------|------|------|--------|
| 2.5.1 | **沙箱系统 (WASM + Docker)** | ~1200 LOC | **P0** |
| 2.5.2 | Provider Chain 多实例 | 300 LOC | P0 |
| 2.5.3 | 简化认证系统 | 400 LOC | P0 |
| 2.5.4 | 用户隔离 | 200 LOC | P0 |
| 2.5.5 | Cron Scheduler | 500 LOC | P1 |
| 2.5.6 | Metrics | 200 LOC | P1 |
| 2.5.7 | Agent Loop 增强 | 150 LOC | P1 |
| 2.5.8 | 增强审计 | 300 LOC | P2 |
| 2.5.9 | Secret Manager | 250 LOC | P2 |

### 6.2 实施顺序

```
Phase 2.5: 核心基础设施 (~1800 LOC)
├── 2.5.1 沙箱系统 (最高优先级)
│   ├── RuntimeAdapter Trait 抽象
│   ├── WasmAdapter 实现 (wasmtime)
│   │   ├── Fuel Metering
│   │   └── Epoch 中断
│   ├── DockerAdapter 实现 (bollard)
│   │   ├── 容器生命周期
│   │   ├── 资源限制
│   │   └── 挂载安全
│   └── Tool -> Sandbox 路由
│
├── 2.5.2 简化认证
│   ├── AuthConfig + AuthMode
│   ├── ApiKey 验证中间件
│   └── 用户上下文注入
│
└── 2.5.3 用户隔离
    ├── 数据库 user_id 字段
    ├── 查询过滤
    └── 资源关联


Phase 2.6: Provider + Scheduler (~800 LOC)
├── 2.6.1 LLM Provider 多实例
│   ├── Instance 配置 (base_url, key, model)
│   └── 故障切换逻辑
│
└── 2.6.2 Cron Scheduler
    ├── Cron 解析
    ├── 任务持久化
    └── 执行引擎


Phase 2.7: 可观测性 (~500 LOC)
├── 2.7.1 Metrics
│   ├── Prometheus 指标定义
│   ├── /metrics 端点
│   └── 用户维度聚合
│
└── 2.7.2 增强审计
    ├── 审计事件定义
    ├── JSONL 导出


Phase 2.8: Agent 增强 (~400 LOC)
├── 2.8.1 Agent Loop 增强
│   ├── max_rounds = 50
│   ├── 无限模式支持
│   └── Loop Guard 增强
│
└── 2.8.2 Secret Manager
    ├── 加密存储
    └── 配置引用解析
```

### 6.3 验收标准

| 模块 | 验收条件 |
|------|----------|
| **沙箱** | WASM 执行计算任务正常；Docker 创建容器成功 |
| **Provider Chain** | 3个实例（不同 base_url + model），主实例故障自动切换 |
| 认证 | 无认证/API Key模式可切换 |
| 用户隔离 | 相同资源不同用户不可互访 |
| Scheduler | Cron 表达式正确触发任务 |
| Metrics | /metrics 端点返回 Prometheus 格式 |
| Agent Loop | 支持 50 轮对话 |
| 审计 | 关键操作记录可查询 |
| Secret | 配置中的密钥加密存储 |

---

## 七、决策记录

| 编号 | 决策 | 内容 | 日期 |
|------|------|------|------|
| D-01 | LLM 多实例 | 同一 Provider 类型的多个 API Key，用于故障切换 | 2026-03-01 |
| D-02 | 认证模式 | octo-engine 默认无认证，可选 API Key | 2026-03-01 |
| D-03 | 用户隔离 | 通过 User ID 过滤实现资源隔离 | 2026-03-01 |
| D-04 | Scheduler | 单机 Cron，支持任务持久化 | 2026-03-01 |
| D-05 | Metrics | Prometheus 格式，支持用户维度 | 2026-03-01 |

---

## 八、相关文档

- `docs/design/ENTERPRISE_AGENT_SANDBOX_AUTH_DESIGN.md` - 沙箱与认证设计
- `docs/design/ARCHITECTURE_DESIGN.md` - 整体架构设计
- `docs/design/COMPETITIVE_ANALYSIS.md` - 竞品分析
- `docs/design/OPENFANG_ARCHITECTURE_ROADMAP.md` - OpenFang 整合路线图
