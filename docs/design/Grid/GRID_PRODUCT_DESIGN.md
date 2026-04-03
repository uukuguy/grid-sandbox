# Grid 产品设计方案

> 文档版本: 1.0 | 日期: 2026-04-03
> 状态: 草案 (Draft)

---

## 一、产品愿景

**Grid** 是一个分层的自主智能体平台，从开发者命令行工具到企业级多租户平台，覆盖智能体开发、测试、部署、运营的全生命周期。

**核心理念**：一个引擎，四种交付形态。

```
                        ┌─────────────────────────────────┐
                        │        Grid Platform            │  ← 企业级多租户平台
                        │   (认证·计费·租户·审计)          │
                        └──────────────┬──────────────────┘
                                       │ 编排
                        ┌──────────────┴──────────────────┐
                        │        Grid Runtime             │  ← 智能体运行时服务
                        │   (REST/WS API · 容器部署)      │
                        └──────────────┬──────────────────┘
                                       │ 共享
              ┌────────────────────────┼────────────────────────┐
              │                        │                        │
   ┌──────────┴──────────┐  ┌─────────┴─────────┐  ┌──────────┴──────────┐
   │     Grid CLI        │  │    Grid Studio     │  │    Grid SDK         │
   │  (ask · run · eval) │  │  (TUI · Web UI)    │  │  (Rust/Python/TS)   │
   │  开发者命令行工具    │  │  交互式工作台       │  │  嵌入式集成         │
   └─────────────────────┘  └────────────────────┘  └─────────────────────┘
```

---

## 二、产品矩阵

### 2.1 四个产品层级

| 产品 | 命名 | 定位 | 用户画像 | 交付形态 |
|------|------|------|---------|---------|
| **Grid CLI** | `grid` | 开发者命令行工具 | 后端工程师、DevOps、CI/CD | 单二进制 |
| **Grid Studio** | `grid-studio` | 本地交互式工作台 | 全栈开发者、AI 工程师 | 单二进制 + Web |
| **Grid Runtime** | `grid-runtime` | 智能体运行时服务 | 平台工程师、SRE | Docker 容器 |
| **Grid Platform** | `grid-platform` | 企业多租户平台 | 企业 IT、平台团队 | Kubernetes 部署 |

### 2.2 产品关系

```
Grid CLI ──────── 独立运行，本地直接调用 Engine
                  不需要 Runtime 进程

Grid Studio ───── 本地模式: 直接调用 Engine（与 CLI 相同）
                  远程模式: 连接 Runtime API

Grid Runtime ──── 无状态服务进程，对外提供 REST/WS API
                  可被 Studio、Platform、SDK 调用

Grid Platform ─── 编排层，管理 Runtime 实例池
                  不直接操作 Engine，通过 Runtime API
```

---

## 三、Grid CLI — 开发者命令行工具

### 3.1 产品定位

**一句话**：像 `curl` 一样简单地与 AI 智能体交互。

**核心场景**：
- CI/CD 管道中自动执行智能体任务
- SSH 远程环境中快速问答
- 脚本集成（管道输入/JSON 输出）
- 智能体配置和资源管理

### 3.2 命令结构

```bash
grid                              # 产品根命令
├── ask "message"                 # 单条消息，执行完退出
│   ├── --session <id>            #   指定会话
│   ├── --agent <id>              #   指定智能体
│   ├── --output json|text        #   输出格式
│   └── --max-rounds <n>          #   最大轮次限制
│
├── run                           # 交互式 REPL
│   ├── --continue                #   恢复上次会话
│   ├── --session <id>            #   指定会话
│   ├── --agent <id>              #   指定智能体
│   ├── --dual                    #   双智能体模式 (Plan+Build)
│   └── --theme <name>            #   颜色主题
│
├── eval                          # 评估套件
│   ├── run --suite <name>        #   运行评估
│   ├── compare --suite <name>    #   多模型对比
│   ├── benchmark                 #   基准测试
│   └── report <run_id>           #   查看报告
│
├── agent                         # 智能体管理
│   ├── list / info / create      #
│   └── start / stop / delete     #
│
├── session                       # 会话管理
│   ├── list / create / delete    #
│   └── export / import           #
│
├── memory                        # 记忆管理
│   ├── search / list / get       #
│   └── add / edit / delete       #
│
├── mcp                           # MCP 服务器管理
│   ├── list / add / remove       #
│   └── logs / tools              #
│
├── tool list / info              # 工具查看
├── skill list / info / install   # 技能管理
├── config show / set / validate  # 配置管理
├── auth login / logout / status  # 认证管理
├── sandbox status / exec / logs  # 沙箱诊断
├── init                          # 项目初始化
├── doctor [--repair]             # 健康检查
└── completions bash|zsh|fish     # Shell 补全
```

### 3.3 设计原则

| 原则 | 说明 |
|------|------|
| **管道友好** | `grid ask "..." --output json \| jq .result` |
| **零守护进程** | 不需要后台服务，直接调用 Engine |
| **静默可控** | `--quiet` 只输出结果，`--verbose` 输出调试信息 |
| **退出码语义** | 0=成功, 1=错误, 2=超时, 3=被拒绝 |
| **幂等安全** | `ask` 和管理命令可安全重试 |

### 3.4 技术实现

```toml
# Cargo.toml
[package]
name = "grid-cli"

[features]
default = ["cli"]
cli = []  # ask + run + 管理命令 — 轻量，无 TUI 依赖
```

**依赖**：`grid-engine`, `grid-types`, `grid-sandbox`, `grid-eval`
**不依赖**：ratatui, crossterm, tower-http（这些属于 Studio）

**二进制大小目标**：< 30MB（release, stripped）

---

## 四、Grid Studio — 交互式工作台

### 4.1 产品定位

**一句话**：开发者的 AI 智能体控制中心。

**核心场景**：
- 日常开发中与智能体长时间协作
- 实时观察智能体的思考过程、工具调用、上下文消耗
- 调试和优化智能体行为
- 管理多个会话和智能体

### 4.2 双模式架构

```
┌─────────────────────────────────────────────────────┐
│                  Grid Studio                         │
│                                                      │
│   ┌─────────────────┐    ┌────────────────────────┐ │
│   │    TUI 模式      │    │     Web UI 模式        │ │
│   │  (grid-studio)   │    │  (grid-studio --web)   │ │
│   │                  │    │                         │ │
│   │  ratatui 全屏    │    │  React + Vite          │ │
│   │  终端原生体验    │    │  浏览器中运行           │ │
│   │  键盘驱动       │    │  鼠标友好               │ │
│   └────────┬─────────┘    └───────────┬─────────────┘ │
│            │                          │               │
│            └──────────┬───────────────┘               │
│                       │                               │
│              ┌────────┴────────┐                      │
│              │  AgentRuntime   │  ← 共享同一个运行时   │
│              │  (本地 Engine)  │                       │
│              └─────────────────┘                       │
└─────────────────────────────────────────────────────────┘
```

### 4.3 TUI 模式功能

```bash
grid-studio                       # 启动 TUI（默认）
grid-studio --theme indigo         # 指定主题
grid-studio --web --port 8080      # 同时启动 Web UI
```

#### 界面布局

```
╭─ ◆ Grid Studio ────────────────────────────────────╮
│                                                      │
│  ╭───╮  ╭───╮  ╶──┬──╴  ╭───╮                      │
│  │   │  │   │     │     │   │    Tier 3 Welcome     │
│  │   │  ├───╯     │     │   │                       │
│  │   │  │  ╲      │     │   │                       │
│  ╰───╯  ╵   ╲  ╶──╯     ╰───╯                      │
│                                                      │
│          Autonomous AI Workbench                     │
│                                                      │
│  ╔══════════════════════════════════════════════╗    │
│  ║  Enter: send │ /help: commands │ Ctrl+C: quit ║   │
│  ╚══════════════════════════════════════════════╝    │
│                                                      │
│──────────────────────────────────────────────────────│
│ ❯ █                                                  │
│──────────────────────────────────────────────────────│
│ ◆ Grid  │  claude-sonnet-4  │  ▸0 ▾0  │  ━━━━━ 100%│
│ …/project  │  ⏇ main                                │
╰──────────────────────────────────────────────────────╯
```

#### 核心功能清单

| 功能 | 说明 | 快捷键 |
|------|------|--------|
| **对话** | Markdown 渲染、流式输出、代码高亮 | Enter 发送 |
| **工具结果** | 可折叠/展开、Diff 自动着色 | Ctrl+O |
| **上下文监控** | 实时 token 消耗、context % 进度条 | 状态栏 |
| **Git 集成** | 分支、改动状态实时显示 | 状态栏 |
| **会话管理** | 多会话切换、恢复、导出 | Ctrl+S |
| **Agent Debug** | 运行状态、记忆、工具调用追踪 | F5 |
| **命令补全** | / 命令、@ 文件引用自动补全 | Tab |
| **工具审批** | 风险分级弹窗审批 | Y/N/A |
| **主题切换** | 12 套预设主题 | — |

### 4.4 Web UI 模式功能

基于 `web/` 前端，连接本地 AgentRuntime API：

| 页面 | 功能 |
|------|------|
| **Chat** | 流式对话、Markdown、代码块 |
| **Tools** | 工具执行历史、输入/输出查看 |
| **Memory** | 三层记忆浏览器（Working/Session/Persistent） |
| **Debug** | Token Budget 可视化、EventBus 实时流 |
| **MCP** | MCP 服务器管理、工具调用、日志 |
| **Tasks** | 任务列表、状态追踪 |
| **Schedule** | 定时任务管理 |

### 4.5 品牌视觉系统

#### 品牌色

| Token | 色值 | 用途 |
|-------|------|------|
| `--brand` | `RGB(94, 106, 210)` #5E6AD2 | 品牌主色 (Indigo) |
| `--brand-dim` | `RGB(47, 53, 105)` | 品牌暗色（边框、不活跃） |
| `--brand-glow` | `rgba(94, 106, 210, 0.15)` | 发光效果 |

#### TUI 色彩系统

```
背景层级:
  --bg-deep:      #0a0a0f    最深底层
  --surface-1:    #111118    卡片/面板
  --surface-2:    #1a1a24    悬浮层
  --surface-3:    #242430    输入框

文字层级:
  --fg:           #EDEDEF    主文字
  --fg-muted:     #8A8F98    次要文字
  --fg-faint:     #4E5158    占位符

边框:
  --border:       rgba(255, 255, 255, 0.08)
  --border-hover: rgba(255, 255, 255, 0.15)

语义色:
  --success:      #22C55E
  --warning:      #F59E0B
  --error:        #EF4444
  --info:         #3B82F6
```

#### 品牌图标

| 场景 | 图标 | Unicode |
|------|------|---------|
| TUI 状态栏 | ◆ | U+25C6 实心菱形 |
| Welcome ASCII Art | 圆角线条 "GRID" | 方案 A |
| Web favicon | ◆ 矢量 SVG | — |

#### 字体

| 用途 | 字体 | 备注 |
|------|------|------|
| UI 文字 | Inter | 正文 14px，标题 600 weight |
| 代码/数据 | JetBrains Mono | 等宽，代码块和 token 数字 |
| TUI | 终端字体 | 无自定义要求 |

### 4.6 技术实现

```toml
[package]
name = "grid-studio"

[features]
default = ["tui"]
tui = ["dep:ratatui", "dep:crossterm"]
web = ["dep:tower-http", "dep:axum"]
full = ["tui", "web"]
```

**二进制大小目标**：< 40MB（含 TUI + Web 静态资源）

---

## 五、Grid Runtime — 智能体运行时服务

### 5.1 产品定位

**一句话**：容器化的智能体执行引擎，通过 API 提供智能体能力。

**核心场景**：
- 容器内后台运行，接受 API 调用
- 被 Grid Studio（远程模式）、Grid Platform、第三方应用调用
- 横向扩展，每个实例服务多个会话
- 企业内部智能体基础设施

### 5.2 架构定位

```
Grid Runtime 不是"应用服务器"，而是"智能体运行时"。
它只负责：执行智能体逻辑、管理工具和 MCP、维护记忆。
它不负责：认证、计费、用户管理、租户隔离（这些是 Platform 的职责）。
```

### 5.3 API 设计

```
REST API:
  POST   /api/sessions                    创建会话
  GET    /api/sessions                    列出会话
  GET    /api/sessions/:id                获取会话详情
  DELETE /api/sessions/:id                删除会话
  POST   /api/sessions/:id/messages       发送消息（返回流式响应）

  GET    /api/agents                      列出智能体
  GET    /api/agents/:id                  获取智能体详情
  POST   /api/agents                      创建智能体

  GET    /api/tools                       列出工具
  POST   /api/tools/:name/execute         执行工具

  GET    /api/memory/search?q=...         搜索记忆
  POST   /api/memory                      写入记忆

  GET    /api/mcp/servers                 列出 MCP 服务器
  POST   /api/mcp/servers                 添加 MCP 服务器

  GET    /api/metrics/prometheus           Prometheus 指标
  GET    /healthz                         健康检查
  GET    /readyz                          就绪检查

WebSocket:
  WS     /ws?session_id=...               实时流式通信
```

### 5.4 部署模式

```yaml
# docker-compose.yml
services:
  grid-runtime:
    image: ghcr.io/grid/runtime:latest
    ports:
      - "3001:3001"
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - GRID_DB_PATH=/data/grid.db
      - GRID_LOG_FORMAT=json
      - GRID_SANDBOX_PROFILE=development
    volumes:
      - grid-data:/data
      - ./config.yaml:/etc/grid/config.yaml
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3001/healthz"]
      interval: 30s
```

### 5.5 安全模型

| 层 | 机制 | 说明 |
|----|------|------|
| **网络** | 内部网络，不暴露公网 | 由 Platform/Ingress 代理 |
| **认证** | API Key（Bearer Token） | 简单，适合内部服务间调用 |
| **沙箱** | SandboxProfile（dev/stg/prod） | 工具执行环境隔离 |
| **审计** | AuditStorage（SQLite） | 所有操作可追溯 |
| **加密** | AES-GCM（Secret Manager） | API Key、凭证加密存储 |

### 5.6 技术实现

```toml
[package]
name = "grid-runtime"
# 当前: octo-server，重命名

[dependencies]
grid-engine = { path = "../grid-engine" }
grid-types = { path = "../grid-types" }
grid-sandbox = { path = "../grid-sandbox" }
axum = "0.8"
tokio = "1.42"
```

**容器镜像大小目标**：< 100MB（Alpine base, release build）

---

## 六、Grid Platform — 企业多租户平台

### 6.1 产品定位

**一句话**：企业级自主智能体管理平台，支持多团队、多智能体的安全协作。

**核心场景**：
- 企业内部 AI 智能体的统一管理
- 多团队共享智能体能力，按需分配资源
- 合规审计（谁在什么时候用了什么智能体做了什么）
- 成本控制（按租户/项目计量 token 消耗）

### 6.2 架构

```
┌─────────────────────────────────────────────────────────┐
│                    Grid Platform                         │
│                                                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐ │
│  │ Auth     │ │ Tenant   │ │ Billing  │ │ Admin      │ │
│  │ Gateway  │ │ Manager  │ │ Metering │ │ Console    │ │
│  │          │ │          │ │          │ │            │ │
│  │ JWT/OIDC │ │ 租户隔离  │ │ Token 计│ │ Web 管理台 │ │
│  │ OAuth2   │ │ 配额管理  │ │ 费模型  │ │            │ │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────────────┘ │
│       │            │            │                        │
│  ┌────┴────────────┴────────────┴─────────────────────┐ │
│  │              Runtime Pool Manager                   │ │
│  │                                                      │ │
│  │   ┌─────────┐  ┌─────────┐  ┌─────────┐           │ │
│  │   │Runtime-1│  │Runtime-2│  │Runtime-N│  ← 弹性伸缩│ │
│  │   │Tenant-A │  │Tenant-B │  │Tenant-C │           │ │
│  │   └─────────┘  └─────────┘  └─────────┘           │ │
│  └──────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

### 6.3 核心能力

| 能力 | 说明 |
|------|------|
| **多租户隔离** | 每个租户独立的 Runtime 实例、数据库、MCP 配置 |
| **身份认证** | JWT + OAuth2/OIDC，支持企业 SSO（SAML, LDAP） |
| **RBAC 权限** | 平台管理员 / 租户管理员 / 开发者 / 查看者 |
| **配额管理** | 按租户设置 token 上限、并发会话数、工具白名单 |
| **成本计量** | 按模型/租户/项目 的 token 消耗统计和报表 |
| **合规审计** | 所有操作日志、数据留存策略、可导出审计报告 |
| **智能体市场** | 共享智能体模板、技能包、MCP 服务器配置 |
| **弹性伸缩** | 基于负载自动扩缩 Runtime 实例池 |

### 6.4 与 Runtime 的交互

```
Platform API → Runtime Pool Manager → 选择/创建 Runtime 实例 → 转发请求

- Platform 维护 Tenant → Runtime 映射表
- 新租户: 分配新 Runtime 实例（或共享实例 + namespace 隔离）
- 请求路由: Platform 认证后，附加 tenant_id 头转发给 Runtime
- 指标采集: 从 Runtime /metrics 端点拉取，聚合到 Platform 监控
```

### 6.5 技术实现

```toml
[package]
name = "grid-platform"
# 当前: octo-platform-server

[dependencies]
grid-engine = { path = "../grid-engine" }
grid-types = { path = "../grid-types" }
grid-sandbox = { path = "../grid-sandbox" }
axum = "0.8"
jsonwebtoken = "9"  # JWT
```

---

## 七、共享核心：Grid Engine

### 7.1 定位

Grid Engine 是**纯库 crate**，不感知任何部署模式（CLI/Server/Platform）。

### 7.2 模块划分

```
grid-engine/
├── agent/          AgentRuntime, AgentExecutor, AgentLoop, Catalog, Store
├── context/        SystemPromptBuilder, ContextBudget, Compaction, Collapse
├── memory/         WorkingMemory(L0), SessionMemory(L1), MemoryStore(L2), KG
├── mcp/            McpManager, McpClient, McpToolBridge
├── providers/      Anthropic, OpenAI, ProviderChain
├── tools/          ToolRegistry, 内置工具, Tool trait
├── skills/         SkillLoader, SkillRegistry, SkillRuntime
├── session/        SessionStore (SQLite/InMemory)
├── security/       SecurityPolicy, PermissionEngine, ActionTracker
├── hooks/          HookRegistry, HookHandler, HookAction
├── event/          EventBus (pub/sub)
├── db/             SQLite wrapper + migrations
├── auth/           API Key, Role-based auth
├── audit/          AuditEvent, AuditRecord, AuditStorage
├── secret/         SecretManager (AES-GCM)
├── sandbox/        SandboxManager
├── extension/      WASM plugin host
├── metrics/        Counter, Gauge, Histogram
├── metering/       Token usage metering
└── root/           OctoRoot path management
```

### 7.3 设计红线

| 规则 | 理由 |
|------|------|
| Engine **不包含** HTTP/CLI/TUI 代码 | 纯业务逻辑库 |
| Engine **不感知** 部署模式 | 通过 config 注入差异 |
| Engine **不管理** 进程生命周期 | 由上层 binary 管理 |
| Engine **API 稳定** | 版本变更需 semver |

---

## 八、Crate 依赖图

```
grid-types (0)                    ← 零依赖类型定义
    │
    ├── grid-sandbox (1)          ← 沙箱运行时适配器
    │
    ├── grid-engine (1)           ← 核心引擎（智能体/记忆/工具/MCP）
    │       │
    │       ├── grid-eval (2)     ← 评估框架
    │       │
    │       ├── grid-cli (2)      ← CLI 工具 (ask + run + 管理命令)
    │       │
    │       ├── grid-studio (2)   ← TUI + Web 工作台
    │       │
    │       ├── grid-runtime (2)  ← 运行时服务 (REST/WS API)
    │       │
    │       └── grid-platform (2) ← 多租户平台
    │
    └── (直接依赖 grid-types 的其他 crate)
```

### 当前 → 目标映射

| 当前 Crate | 目标 Crate | 变更 |
|-----------|-----------|------|
| `octo-types` | `grid-types` | 重命名 |
| `octo-sandbox` | `grid-sandbox` | 重命名 |
| `octo-engine` | `grid-engine` | 重命名 |
| `octo-eval` | `grid-eval` | 重命名 |
| `octo-cli` | `grid-cli` + `grid-studio` | **拆分** |
| `octo-server` | `grid-runtime` | 重命名 + 定位调整 |
| `octo-platform-server` | `grid-platform` | 重命名 |
| `octo-desktop` | `grid-desktop` | 重命名（Tauri 壳） |

---

## 九、拆分策略：octo-cli → grid-cli + grid-studio

### 9.1 当前结构

```
octo-cli/src/
├── main.rs              → 分发 ask/run/tui/dashboard
├── commands/
│   ├── ask.rs           → CLI
│   ├── run.rs           → CLI
│   ├── state.rs         → 共享（两者都需要）
│   ├── dashboard*.rs    → Studio
│   └── [其他管理命令]   → CLI
├── repl/                → CLI（REPL 引擎）
├── tui/                 → Studio（TUI 引擎，47 个文件）
├── ui/                  → 共享（streaming 渲染）
├── output/              → CLI（格式化输出）
└── dashboard/           → Studio（Web UI 路由）
```

### 9.2 拆分方案

**阶段一（最小变更）**：Cargo features 软拆分

```toml
# grid-cli/Cargo.toml
[features]
default = ["cli"]
cli = []                                    # ask + run + 管理命令
studio = ["dep:ratatui", "dep:crossterm"]   # TUI 模式
web = ["dep:tower-http"]                    # Web UI 模式
full = ["cli", "studio", "web"]             # 全功能
```

```bash
# 轻量 CLI（CI 容器用）— 不含 TUI 依赖
cargo build -p grid-cli --no-default-features --features cli
# → 产出: grid

# 完整工作台
cargo build -p grid-cli --features full
# → 产出: grid-studio (或 grid --tui / grid --web)
```

**阶段二（完全拆分）**：独立 crate

```
grid-cli/     → 只含 ask + run + 管理命令
grid-studio/  → 只含 TUI + Web + 共享 state
```

**建议**：先做阶段一，验证产品边界后再决定是否需要阶段二。

---

## 十、实施路线图

### Phase 1：品牌重塑 + 软拆分（1-2 周）

| 任务 | 说明 | 优先级 |
|------|------|--------|
| Cargo feature 拆分 | cli / studio / web features | P0 |
| 品牌替换 | OCTO → GRID（ASCII Art、状态栏、Welcome） | P0 |
| 品牌色统一 | Indigo #5E6AD2 + 双色系统（brand + accent） | P0 |
| 品牌图标 | 🦑 → ◆ (U+25C6) | P0 |
| README 更新 | 产品定位说明 | P1 |

### Phase 2：TUI 视觉升级（1-2 周）

| 任务 | 说明 | 优先级 |
|------|------|--------|
| 色彩系统统一 | style_tokens ↔ theme 合并 | P0 |
| Autocomplete 主题化 | 硬编码色 → theme tokens | P0 |
| Welcome ASCII Art | 圆角线条 GRID + Tier 分级 | P0 |
| Context 进度条升级 | 5 段 → 8 段 + 精细字符 | P1 |
| 消息间距优化 | 空行分隔 + 角色区分 | P1 |
| 动效优化 | 页面切换 fadeIn + 卡片 hover | P1 |

### Phase 3：Web UI 升级（2-3 周）

| 任务 | 说明 | 优先级 |
|------|------|--------|
| 色彩系统升级 | 4 层背景 + 语义色 | P0 |
| 字体引入 | Inter + JetBrains Mono | P0 |
| 导航重构 | TabBar → 侧边栏 | P1 |
| Dashboard 实化 | stub → 连接 AgentRuntime | P1 |
| 骨架屏 + 动效 | Skeleton、fadeIn、hover | P2 |

### Phase 4：Runtime 定位调整（1 周）

| 任务 | 说明 | 优先级 |
|------|------|--------|
| 重命名 octo-server → grid-runtime | Cargo + Docker | P0 |
| 添加 /healthz /readyz | 容器编排支持 | P0 |
| Dockerfile 优化 | 多阶段构建, Alpine base | P1 |
| OpenAPI spec 生成 | 从代码自动生成 | P2 |

### Phase 5：Platform 增强（持续）

| 任务 | 说明 | 优先级 |
|------|------|--------|
| Runtime Pool Manager | 管理多 Runtime 实例 | P1 |
| 计费/计量集成 | Token 消耗按租户统计 | P2 |
| Admin Console | 管理端 Web UI | P2 |
| SSO 集成 | SAML/OIDC | P2 |

---

## 十一、竞品对标

| 维度 | Grid CLI | Claude Code | Cursor | Devin |
|------|---------|------------|--------|-------|
| **命令行** | ✅ ask+run | ✅ claude | ❌ | ❌ |
| **TUI** | ✅ 全屏 | ✅ 全屏 | ❌ | ❌ |
| **Web UI** | ✅ 嵌入式 | ❌ | ✅ IDE 插件 | ✅ Web |
| **评估框架** | ✅ 内置 | ❌ | ❌ | ✅ |
| **多模型** | ✅ ProviderChain | ❌ Anthropic only | ✅ | ❌ |
| **MCP 支持** | ✅ stdio+SSE | ✅ | ✅ | ❌ |
| **沙箱执行** | ✅ Docker+WASM | ❌ 本地 | ❌ | ✅ |
| **企业部署** | ✅ Platform | ❌ | ✅ Business | ❌ |
| **自主模式** | ✅ 规划中 | ✅ | ❌ | ✅ |
| **开源** | ✅ | ✅ | ❌ | ❌ |

**Grid 的差异化优势**：
1. **全栈覆盖**：从 CLI 到企业平台一条产品线
2. **引擎级可控**：Rust 编写，可深度定制
3. **多模型中立**：ProviderChain 支持任意 LLM
4. **评估内置**：开发-评估闭环
5. **沙箱安全**：企业级隔离

---

## 十二、命名规范总结

| 层级 | 面向用户的名称 | 二进制名 | Crate 名 |
|------|--------------|---------|----------|
| 品牌 | **Grid** | — | — |
| CLI | Grid CLI | `grid` | `grid-cli` |
| 工作台 | Grid Studio | `grid-studio` | `grid-studio` |
| 运行时 | Grid Runtime | `grid-runtime` | `grid-runtime` |
| 平台 | Grid Platform | `grid-platform` | `grid-platform` |
| 引擎 | Grid Engine | — (库) | `grid-engine` |
| 类型 | — | — (库) | `grid-types` |
| 沙箱 | — | — (库) | `grid-sandbox` |
| 评估 | — | — (库) | `grid-eval` |
| 桌面 | Grid Desktop | `grid-desktop` | `grid-desktop` |

---

## 附录 A：Web UI 设计系统（Grid Studio Web 模式）

详见本次会话前序分析。核心要点：

- 4 层背景深度系统
- Inter + JetBrains Mono 双字体
- 侧边栏导航替代顶部 TabBar
- 骨架屏 + 微交互动效
- 消灭 emoji 图标，统一使用 Lucide

## 附录 B：TUI 视觉改进清单

详见本次会话前序分析。核心要点：

- 统一 style_tokens ↔ theme 色值
- Autocomplete/Approval 弹窗用主题色
- 光标颜色跟随主题
- Context 进度条 8 段精细化
- Welcome 面板 GRID 圆角线条 ASCII Art
