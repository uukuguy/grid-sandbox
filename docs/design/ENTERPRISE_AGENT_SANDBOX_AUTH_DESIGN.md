# 企业级 Agent 沙箱与认证架构设计方案

**版本**: v1.0
**创建日期**: 2026-03-01
**状态**: 正式设计

---

## 一、项目背景与目标

### 1.1 当前状态

octo-engine 当前已实现：
- ✅ Agent Loop（10 轮，可配置）
- ✅ Provider Trait（Anthropic + OpenAI）
- ✅ Tool Trait + 12 个内置工具
- ✅ Subprocess 沙箱（白名单 + env_clear + 路径检查）
- ✅ 记忆系统（3 层）
- ✅ MCP 支持（Stdio + SSE）
- ✅ Skill 系统（热重载）

### 1.2 目标：对标 Claude Code / OpenClaw

| 能力维度 | 当前 | 目标 |
|----------|------|------|
| 沙箱隔离 | Subprocess (不安全) | WASM + Docker |
| 企业安全 | 无 | RBAC + 认证 |
| 工具数 | 12 | 30+ |
| Provider | 2 (无 Failover) | 5+ (Chain) |

---

## 二、对标分析：与 Claude Code / OpenClaw 的差距

### 2.1 竞品能力对比

根据 `docs/design/COMPETITIVE_ANALYSIS.md` 的深度代码分析：

| 能力维度 | octo-engine | Claude Code / OpenClaw | 差距等级 |
|----------|-------------|----------------------|---------|
| **沙箱隔离** | Subprocess 白名单 | Docker + WASM | 🔴 大 |
| **Agent Loop** | 10 轮 | 50 轮 / 无限 | 🟠 中 |
| **工具数** | 12 | 30-54 | 🟠 中 |
| **Provider** | 2 (无 Failover) | 5+ / Chain | 🟠 中 |
| **企业安全** | 空白 | RBAC + 审计 | 🔴 大 |
| **定时任务** | 空白 | Cron + Workflow | 🔴 大 |
| **多渠道** | WebSocket | CLI + API + Telegram | 🟠 中 |

### 2.2 详细差距分析

#### 沙箱隔离（最大差距）

| 项目 | 当前实现 | 差距 |
|------|---------|------|
| 隔离方式 | Subprocess (env_clear + 白名单) | Docker (OS 级隔离) |
| 资源限制 | 超时控制 | CPU/内存/网络完整限制 |
| 挂载安全 | 基础路径检查 | 白名单 + 符号链接防护 |
| 审计日志 | 无 | 完整操作审计 |

#### 企业安全（完全空白）

| 项目 | 当前状态 | Claude Code / OpenClaw |
|------|---------|----------------------|
| 认证 | 无 | OAuth2 / API Key |
| RBAC | 无 | Admin/Developer/Viewer |
| 审计日志 | 无 | 完整记录 |
| 加密存储 | 无 | AES-256-GCM |

#### Agent Loop 深度

| 项目 | 当前 | 对标 |
|------|------|------|
| 最大迭代 | 10 轮 | 50 轮 |
| Loop Guard | 3 层 | 5 层 |
| 重试策略 | 基础 | 5s→80s 指数退避 |
| Provider Failover | 无 | Chain 切换 |

### 2.3 各维度评级（当前 vs 目标）

| 维度 | 当前评级 | 目标评级 |
|------|---------|---------|
| **Agent Loop 核心** | ★★☆☆☆ (40%) | ★★★★☆ (80%) |
| **工具系统** | ★★☆☆☆ (40%) | ★★★★☆ (75%) |
| **记忆与上下文** | ★★★★☆ (80%) | ★★★★★ (95%) |
| **MCP集成** | ★★★☆☆ (65%) | ★★★★☆ (85%) |
| **沙箱隔离** | ★☆☆☆☆ (10%) | ★★★★☆ (80%) |
| **企业安全** | ★☆☆☆☆ (0%) | ★★★★☆ (80%) |
| **LLM Provider** | ★★☆☆☆ (35%) | ★★★★☆ (75%) |

---

## 三、沙箱架构设计

### 2.1 三沙箱体系

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
        │                 │                 │
        ▼                 ▼                 ▼
   ┌─────────┐      ┌─────────┐      ┌─────────┐
   │  WASM   │      │ Docker  │      │Subprocess│
   │  沙箱   │      │  沙箱  │      │ (仅测试) │
   └─────────┘      └─────────┘      └─────────┘
      ▲                 ▲
      │                 │
      └──── 生产环境 ────┘
```

### 2.2 WASM 沙箱 vs Docker 沙箱

| 特性 | WASM 沙箱 | Docker 沙箱 |
|------|-----------|------------|
| **启动时间** | 毫秒级 | 3-10 秒 |
| **资源消耗** | 1-16 MB | 512 MB - 2 GB |
| **隔离边界** | 语言级 (WebAssembly) | 操作系统级 |
| **适用任务** | 纯计算、第三方 Skill | 完整 Agent、复杂环境 |
| **实现复杂度** | 中等 (~500 LOC) | 较高 (~800 LOC) |
| **Rust 集成** | 强项 | 依赖 bollard |

### 2.3 工具类型与沙箱选择

| 工具类型 | 推荐的沙箱 | 示例 |
|----------|-----------|------|
| Shell 命令 | **禁用** | `bash`, `grep`, `git` |
| 无状态计算 | **WASM** | JSON 解析、正则匹配、Skill 计算 |
| 完整环境 | **Docker** | Claude Code、Python 项目、Node 项目 |
| 开发测试 | Subprocess | 快速调试（仅开发环境） |

### 2.4 安全级别定义

| 沙箱 | 用途 | 安全级别 | 生产可用 |
|------|------|---------|---------|
| **Docker** | 生产环境首选 | ⭐⭐⭐⭐⭐ | ✅ |
| **WASM** | 生产环境首选 | ⭐⭐⭐⭐ | ✅ |
| **Subprocess** | 仅开发/测试 | ⭐⭐ | ❌ |

---

## 四、WASM 沙箱实现方案

### 3.1 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                   WasmSandbox                               │
├─────────────────────────────────────────────────────────────┤
│  • Fuel Metering (指令计数，防止死循环)                    │
│  • Epoch 中断 (时间炸弹防护)                               │
│  • 内存限制 (默认 16MB)                                    │
│  • WASI 兼容接口                                           │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 核心代码结构

```rust
// crates/octo-engine/src/sandbox/wasm.rs

use wasmtime::{Engine, Module, Instance, Store, Limits};
use wasmtime_wasi::WasiCtx;

pub struct WasmAdapter {
    engine: Engine,
    max_memory: usize,      // 默认 16MB
    max_fuel: u64,          // 默认 1M 指令
    max_duration: Duration,  // 默认 30s
}

#[async_trait]
impl RuntimeAdapter for WasmAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Wasm
    }

    async fn create(&self, config: SandboxConfig) -> Result<SandboxId> {
        // 编译 WASM 模块
    }

    async fn execute(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult> {
        // Fuel 计量 + Epoch 中断执行
    }
}
```

### 3.3 实施任务

| 任务 | 估算 | 依赖 |
|------|------|------|
| RuntimeAdapter Trait 抽象 | 200 LOC | - |
| WasmAdapter 实现 | 350 LOC | wasmtime |
| Fuel Metering 配置 | 50 LOC | - |
| Epoch 超时机制 | 50 LOC | - |
| WASI 接口集成 | 100 LOC | wasmtime-wasi |
| **总计** | **~750 LOC** | |

---

## 五、Docker 沙箱实现方案

### 4.1 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                   DockerSandbox                             │
├─────────────────────────────────────────────────────────────┤
│  • 容器生命周期管理 (创建/启动/停止/销毁)                  │
│  • 资源限制 (CPU/内存/网络)                                │
│  • 挂载安全 (白名单 + 符号链接防护)                        │
│  • 网络隔离 (独立网络)                                     │
│  • 并发控制 (GroupQueue)                                   │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 核心代码结构

```rust
// crates/octo-engine/src/sandbox/docker.rs

use bollard::Docker;

pub struct DockerAdapter {
    client: Docker,
    max_containers: usize,
    image: String,
    workspace_mount: PathBuf,
}

#[async_trait]
impl RuntimeAdapter for DockerAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Docker
    }

    async fn create(&self, config: SandboxConfig) -> Result<SandboxId> {
        // 1. 并发配额检查
        // 2. 创建容器 (内存/CPU 限制)
        // 3. 挂载安全验证
    }

    async fn execute(&self, id: &SandboxId, cmd: &str) -> Result<ExecResult> {
        // 在容器中执行命令
    }
}
```

### 4.3 挂载安全策略

```rust
pub struct MountSecurity {
    allowed_paths: Vec<PathBuf>,
    system_paths_blacklist: Vec<&'static str>,
}

impl MountSecurity {
    pub fn validate(&self, path: &Path) -> Result<()> {
        // 1. 解析符号链接，防止逃逸
        let canonical = path.canonicalize()?;

        // 2. 检查是否在允许路径内
        // 3. 检查系统路径黑名单 (/etc, /usr, /proc...)
        // 4. 检查敏感文件 (.env, .ssh, .aws...)
    }
}
```

### 4.4 实施任务

| 任务 | 估算 | 依赖 |
|------|------|------|
| DockerAdapter 实现 | 400 LOC | bollard |
| 并发控制 (GroupQueue) | 150 LOC | - |
| 挂载安全验证 | 150 LOC | - |
| 网络隔离 | 50 LOC | Docker network |
| 资源限制 (CPU/内存) | 50 LOC | - |
| **总计** | **~800 LOC** | |

---

## 六、RBAC 与认证方案

### 5.1 三角色 RBAC

| 角色 | 用户管理 | 系统配置 | 所有沙箱 | 自有沙箱 | MCP/Skills | 只读访问 |
|------|---------|---------|---------|---------|-----------|---------|
| **Admin** | Yes | Yes | Yes | Yes | Yes | Yes |
| **Developer** | No | No | No | Yes | Yes | Yes |
| **Viewer** | No | No | No | No | No | 被授权的沙箱 |

### 5.2 双层权限模型

#### 系统层：RBAC 角色控制
控制用户对系统资源的访问权限。

#### Session 层：三级权限模式

| 模式 | 说明 | 适用场景 |
|------|------|---------|
| **ReadOnly** | 工具不可写入，仅可读 | 只读调试、安全审计 |
| **Interactive** | 危险操作需用户确认 | 默认模式 |
| **AutoApprove** | 自动批准所有操作 | 需 Admin 或明确授权 |

### 5.3 认证架构

```
┌─────────────────────────────────────────────────────────────┐
│                    AuthService                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐    │
│  │  Login     │  │  Register   │  │   Session       │    │
│  │  Handler   │  │  Handler    │  │   Manager       │    │
│  └─────────────┘  └─────────────┘  └─────────────────┘    │
│         │                │                    │             │
│         └────────────────┼────────────────────┘             │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              UserStore (SQLite)                      │   │
│  │  - users (id, username, password_hash, role)        │   │
│  │  - sessions (id, user_id, token, expires_at)       │   │
│  │  - invitations (code, created_by, used_at)          │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 5.4 数据模型

```rust
// crates/octo-engine/src/auth/mod.rs

/// 用户角色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Admin,
    Developer,
    Viewer,
}

/// 用户
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

/// 会话
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,        // HMAC 签名
    pub expires_at: DateTime<Utc>,
}
```

### 5.5 认证方案

| 特性 | 方案 | 说明 |
|------|------|------|
| 密码存储 | bcrypt-12 | 12 轮哈希 |
| 会话管理 | HMAC Cookie | 签名验证 |
| 注册方式 | 邀请码 | Admin 生成邀请码 |
| 后期扩展 | OAuth2 / LDAP | Phase 4 |

### 5.6 实施任务

| 任务 | 估算 | 依赖 |
|------|------|------|
| 用户/会话数据模型 | 150 LOC | - |
| AuthService 核心 | 200 LOC | - |
| 密码 bcrypt 哈希 | 依赖 bcrypt | - |
| HMAC Session | 50 LOC | - |
| RBAC 权限检查 | 150 LOC | 用户模型 |
| Auth Middleware | 100 LOC | AuthService |
| 邀请码系统 | 100 LOC | 用户模型 |
| **总计** | **~750 LOC** | |

---

## 七、Per-User 隔离设计

### 6.1 目录结构

```
data/users/{user_id}/
  memory/
    blocks/           # Working Memory 块（JSON）
    memories.db       # 用户记忆库
    archive/          # 归档 JSONL
  sessions/           # 会话数据
  sandboxes/         # 沙箱定义
  credentials/        # 凭据（AES-256-GCM 加密）
  skills/             # 私有 Skills/MCP

data/shared/
  memory/
    system_facts.db   # 系统级共享知识
  skills/             # 全局共享 Skills
  mcp-servers/        # 全局共享 MCP Servers
  templates/          # 沙箱模板

data/system/
  octo.db             # 主数据库（SQLite WAL）
  audit.log           # 审计日志
  config.toml         # 系统配置
```

### 6.2 隔离规则

- 每个用户的记忆完全隔离（`user_id` 过滤）
- 沙箱级记忆可选隔离（`sandbox_id` 过滤）
- 系统级知识（如 MCP 工具文档）所有用户共享
- Admin 可查看所有用户记忆（审计需求）
- Viewer 不可访问记忆系统

---

## 八、实施计划

### 7.1 Phase 3: 完整 MVP

| 阶段 | 内容 | 工作量 | 优先级 |
|------|------|--------|--------|
| **3.1** | WASM 沙箱 | ~750 LOC | P0 |
| **3.2** | Docker 沙箱 | ~800 LOC | P0 |
| **3.3** | 认证系统 | ~450 LOC | P1 |
| **3.4** | RBAC 权限 | ~300 LOC | P1 |

### 7.2 总体工作量

| 模块 | 预估 LOC |
|------|---------|
| WASM 沙箱 | 750 |
| Docker 沙箱 | 800 |
| 认证 + RBAC | 750 |
| **总计** | **~2,300 LOC** |

---

## 九、参考来源

- **架构设计文档**: `docs/design/ARCHITECTURE_DESIGN.md`
- **竞品分析**: `docs/design/COMPETITIVE_ANALYSIS.md`
- **OpenFang 参考**:
  - `openfang-kernel/src/sandbox/` (沙箱实现)
  - `openfang-kernel/src/auth/rbac.rs` (316 LOC)
- **HappyClaw 参考**:
  - Docker 容器管理
  - RBAC 实现

---

## 十、决策记录

| 决策 | 内容 | 日期 |
|------|------|------|
| D-01 | Subprocess 仅作为开发测试备用，不用于生产 | 2026-03-01 |
| D-02 | WASM + Docker 作为生产环境双沙箱 | 2026-03-01 |
| D-03 | 认证采用 bcrypt-12 + HMAC Session | 2026-03-01 |
| D-04 | RBAC 采用三角色 (Admin/Developer/Viewer) | 2026-03-01 |
