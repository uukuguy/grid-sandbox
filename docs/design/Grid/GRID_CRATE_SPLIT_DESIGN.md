# Grid Crate 拆分设计方案

> 文档版本: 1.0 | 日期: 2026-04-04
> 状态: 已批准 (Approved)
> 前置文档: [GRID_PRODUCT_DESIGN.md](./GRID_PRODUCT_DESIGN.md)

---

## 一、背景

根据 GRID_PRODUCT_DESIGN.md 的产品矩阵定义，当前 `octo-cli` crate 承载了两个定位完全不同的产品：

| 产品 | 场景 | 用户 |
|------|------|------|
| **Grid CLI** | CI/CD 管道、SSH 远程、脚本集成 | DevOps、后端工程师 |
| **Grid Studio** | 日常开发协作、调试优化、多会话管理 | 全栈开发者、AI 工程师 |

当前 `octo-cli` 将两者混在一个 crate 中，导致：
- CLI 二进制不必要地包含 ratatui/crossterm/axum 依赖（+10MB）
- TUI/Dashboard 代码不必要地包含 rustyline 依赖
- 产品边界模糊，难以独立演进

**本文档是新项目（未上线），无向后兼容需求，采用一步到位策略。**

---

## 二、现状分析

### 2.1 代码量分布

| 目录 | 行数 | 占比 | 依赖特征 |
|------|------|------|---------|
| `tui/` | 14,698 | 61% | ratatui, crossterm |
| `commands/` | 6,004 | 25% | 混合（需拆分） |
| `repl/` | 2,051 | 8% | rustyline |
| `ui/` | 487 | 2% | 无外部依赖（共享） |
| `output/` | 100 | <1% | 无外部依赖 |
| `dashboard/` | 4+assets | <1% | axum, tower-http |
| **合计** | **24,160** | **100%** | |

### 2.2 依赖关系分析

```
commands/state.rs (AppState) ←── 所有模块依赖
commands/types.rs            ←── 所有命令枚举定义
ui/table.rs                  ←── 6 个 commands（agent/session/mcp/tools/skill/memory）
ui/theme.rs                  ←── tui/theme.rs 依赖
ui/streaming.rs              ←── 依赖 output::OutputConfig
output/                      ←── 9 个 commands + ui/streaming.rs

tui/     ← 完全自封闭，无外部 `use crate::tui::`
repl/    ← 自封闭，仅被 commands/run.rs 调用
dashboard/ ← 被 commands/dashboard.rs + octo-desktop 消费
```

**关键发现**：
1. `tui/`（14,698 行）**零对外依赖**，可整体搬迁
2. `repl/`（2,051 行）仅通过 `commands/run.rs` 一个入口调用
3. `dashboard/` 被 `octo-desktop` 通过 `lib.rs` 的 `build_router()` 消费
4. `ui/` 是唯一的真正共享层（CLI commands 和 TUI 同时依赖）
5. `AppState`（state.rs）是全局核心，所有模块依赖

### 2.3 commands/ 归属分析

| 文件 | 归属 | 理由 |
|------|------|------|
| `state.rs` | **共享** | AppState 被所有模块依赖 |
| `types.rs` | **共享** | 子命令枚举定义 |
| `agent.rs` | **共享** | CLI 和 Studio 都需要管理 agent |
| `session.rs` | **共享** | 同上 |
| `memory.rs` | **共享** | 同上 |
| `mcp.rs` | **共享** | 同上 |
| `tools.rs` | **共享** | 同上 |
| `config.rs` | **共享** | 同上 |
| `auth.rs` | **共享** | 同上 |
| `skill.rs` | **共享** | 同上 |
| `eval_cmd.rs` | **共享** | CLI eval + Studio 可视化 |
| `ask.rs` | **grid-cli** | 单次调用，管道友好 |
| `run.rs` | **grid-cli** | 调用 repl |
| `init.rs` | **grid-cli** | 项目初始化 |
| `doctor.rs` | **grid-cli** | 健康检查 |
| `completions.rs` | **grid-cli** | Shell 补全生成 |
| `root.rs` | **grid-cli** | 路径管理 |
| `sandbox.rs` | **grid-cli** | 沙箱诊断 |
| `dashboard.rs` | **grid-studio** | Web Dashboard 服务 |
| `dashboard_auth.rs` | **grid-studio** | Dashboard 认证 |
| `dashboard_cert.rs` | **grid-studio** | TLS 证书生成 |
| `dashboard_security.rs` | **grid-studio** | CORS/安全头 |

---

## 三、目标 Crate 结构

### 3.1 新增 crate: `grid-cli-common`

共享层，被 grid-cli 和 grid-studio 共同依赖。

```
crates/grid-cli-common/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── state.rs              ← commands/state.rs (AppState)
    ├── types.rs              ← commands/types.rs (子命令枚举)
    ├── output/               ← output/ (OutputConfig, Json/Text/StreamJson)
    │   ├── mod.rs
    │   ├── json.rs
    │   ├── stream_json.rs
    │   └── text.rs
    ├── ui/                   ← ui/ (共享终端 UI 组件)
    │   ├── mod.rs
    │   ├── markdown.rs
    │   ├── spinner.rs
    │   ├── streaming.rs
    │   ├── table.rs
    │   └── theme.rs
    └── commands/             ← 共享命令 handlers
        ├── mod.rs
        ├── agent.rs
        ├── session.rs
        ├── memory.rs
        ├── mcp.rs
        ├── tools.rs
        ├── config.rs
        ├── auth.rs
        ├── skill.rs
        └── eval_cmd.rs
```

**Cargo.toml 依赖**：
```toml
[dependencies]
grid-types = { path = "../grid-types" }
grid-engine = { path = "../grid-engine" }
grid-eval = { path = "../grid-eval" }
# 终端渲染（轻量，非 TUI 框架）
termimad = "0.31"
indicatif = "0.17"
owo-colors = "4"
# CLI 解析
clap = { version = "4.5", features = ["derive", "env"] }
# 通用
serde, serde_json, serde_yaml, anyhow, tracing, chrono, uuid, rusqlite
```

**不包含**：ratatui, crossterm, rustyline, axum, tower-http

### 3.2 瘦 CLI: `grid-cli`

```
crates/grid-cli/
├── Cargo.toml
└── src/
    ├── main.rs               ← CLI 入口（无 Tui/Dashboard 命令）
    ├── lib.rs
    ├── repl/                 ← 交互式 REPL
    │   ├── mod.rs
    │   ├── context.rs
    │   ├── file_ref.rs
    │   ├── helper.rs
    │   ├── history.rs
    │   ├── hooks.rs
    │   └── slash.rs
    └── commands/             ← CLI 独有命令
        ├── mod.rs
        ├── ask.rs
        ├── run.rs
        ├── init.rs
        ├── doctor.rs
        ├── completions.rs
        ├── root.rs
        └── sandbox.rs
```

**Cargo.toml 依赖**：
```toml
[dependencies]
grid-cli-common = { path = "../grid-cli-common" }
grid-engine = { path = "../grid-engine" }
grid-types = { path = "../grid-types" }
# CLI 独有
rustyline = { version = "17", features = ["with-file-history"] }
dialoguer = "0.11"
clap = { version = "4.5", features = ["derive", "env"] }
clap_complete = "4.5"
```

**不包含**：ratatui, crossterm, axum, tower-http

**二进制名**：`grid`

**命令树**：
```
grid ask "message"
grid run [--continue] [--session] [--agent] [--dual]
grid agent list|info|create|start|stop|delete
grid session list|create|delete|export|import
grid memory search|list|get|add|edit|delete
grid mcp list|add|remove|logs|tools
grid tool list|info
grid skill list|info|install
grid config show|set|validate
grid auth login|logout|status
grid eval run|compare|benchmark|report|...
grid sandbox status|exec|logs
grid init
grid doctor [--repair]
grid root show|init
grid completions bash|zsh|fish
```

### 3.3 全功能工作台: `grid-studio`

```
crates/grid-studio/
├── Cargo.toml
└── src/
    ├── main.rs               ← Studio 入口（TUI 默认，--web 启动 Dashboard）
    ├── lib.rs                ← 暴露 dashboard::build_router 给 grid-desktop
    ├── tui/                  ← 全屏 TUI（47 文件原样搬迁）
    │   ├── mod.rs
    │   ├── app_state.rs
    │   ├── event.rs
    │   ├── event_handler.rs
    │   ├── key_handler.rs
    │   ├── render.rs
    │   ├── theme.rs
    │   ├── autocomplete/     ← 5 文件
    │   ├── formatters/       ← 10 文件
    │   ├── managers/         ← 5 文件
    │   ├── overlays/         ← 3 文件
    │   └── widgets/          ← 14 文件
    └── dashboard/            ← Web Dashboard
        ├── mod.rs
        ├── server.rs         ← dashboard.rs 重命名
        ├── auth.rs           ← dashboard_auth.rs
        ├── cert.rs           ← dashboard_cert.rs
        ├── security.rs       ← dashboard_security.rs
        └── assets/           ← 嵌入式 HTML/JS/CSS
```

**Cargo.toml 依赖**：
```toml
[dependencies]
grid-cli-common = { path = "../grid-cli-common" }
grid-engine = { path = "../grid-engine" }
grid-types = { path = "../grid-types" }
# TUI
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }
# Web Dashboard
axum = "0.8"
tower = "0.5"
tower-http = "0.6"
# TLS (optional)
axum-server = { version = "0.7", features = ["tls-rustls"], optional = true }
rcgen = { version = "0.13", optional = true }
# 文件遍历（autocomplete 用）
ignore = "0.4"
unicode-width = "0.2"
dirs = "6"

[features]
default = ["tui"]
tui = []
web = []
dashboard-tls = ["axum-server", "rcgen"]
full = ["tui", "web", "dashboard-tls"]
```

**不包含**：rustyline

**二进制名**：`grid-studio`

**命令**：
```
grid-studio                    # TUI 模式（默认）
grid-studio --theme indigo     # 指定主题
grid-studio --web              # 同时启动 Web Dashboard
grid-studio --web --port 8080  # 指定端口
```

### 3.4 Desktop 更新: `grid-desktop`

```toml
# crates/grid-desktop/Cargo.toml
[dependencies]
grid-studio = { path = "../grid-studio" }  # 原来是 octo-cli
```

代码变更：`octo_cli::commands::dashboard::build_router` → `grid_studio::dashboard::build_router`

---

## 四、依赖图总览

```
grid-types (0)                    ← 零依赖
    │
    ├── grid-sandbox (1)
    │
    ├── grid-engine (1)           ← 核心引擎
    │       │
    │       ├── grid-eval (2)
    │       │
    │       └── grid-cli-common (2) ← 共享命令层
    │               │
    │               ├── grid-cli (3)      ← 二进制: grid
    │               │     额外: rustyline
    │               │     不含: ratatui, crossterm, axum
    │               │
    │               └── grid-studio (3)   ← 二进制: grid-studio
    │                     额外: ratatui, crossterm, axum
    │                     不含: rustyline
    │
    ├── grid-runtime (2)          ← octo-server 重命名
    │
    ├── grid-platform (2)         ← octo-platform-server 重命名
    │
    └── grid-desktop (3)          ← 依赖 grid-studio
```

---

## 五、二进制产物

| Binary | Crate | 预估大小 | 用途 |
|--------|-------|---------|------|
| `grid` | grid-cli | ~20MB | CI/CD, SSH, 脚本 |
| `grid-studio` | grid-studio | ~30MB | 开发者工作台 |
| `grid-runtime` | grid-runtime | ~25MB | 容器部署 |
| `grid-platform` | grid-platform | ~30MB | 企业平台 |
| `grid-desktop` | grid-desktop | ~50MB | Tauri 桌面应用 |

---

## 六、全局重命名清单

### 6.1 Crate 重命名

| 当前 | 目标 |
|------|------|
| `octo-types` | `grid-types` |
| `octo-sandbox` | `grid-sandbox` |
| `octo-engine` | `grid-engine` |
| `octo-eval` | `grid-eval` |
| `octo-cli` | **拆分为** grid-cli-common + grid-cli + grid-studio |
| `octo-server` | `grid-runtime` |
| `octo-platform-server` | `grid-platform` |
| `octo-desktop` | `grid-desktop` |

### 6.2 环境变量

| 当前 | 目标 |
|------|------|
| `OCTO_HOST` | `GRID_HOST` |
| `OCTO_PORT` | `GRID_PORT` |
| `OCTO_DB_PATH` | `GRID_DB_PATH` |
| `OCTO_GLOBAL_ROOT` | `GRID_GLOBAL_ROOT` |
| `OCTO_PROJECT_ROOT` | `GRID_PROJECT_ROOT` |

### 6.3 路径

| 当前 | 目标 |
|------|------|
| `~/.octo/` | `~/.grid/` |
| `.octo/` | `.grid/` |
| `data/octo.db` | `data/grid.db` |

### 6.4 品牌

| 当前 | 目标 |
|------|------|
| ASCII Art "OCTO" + 🦑 | 圆角线条 "GRID" + ◆ (U+25C6) |
| 状态栏 "🦑 Octo" | "◆ Grid" |
| 品牌色 Cyan | Indigo #5E6AD2 |

---

## 七、风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 拆分后编译时间增加 | 中 | workspace 共享依赖，增量编译不受影响 |
| grid-cli-common 边界不清 | 高 | 严格规则：只放"两个产品都需要的代码" |
| `use crate::` 路径大面积修改 | 低 | 一次性完成，IDE rename 辅助 |
| tests 需要调整 | 中 | tui/ 内的测试随 crate 迁移，integration tests 需更新 import |

---

## 八、设计红线

| 规则 | 理由 |
|------|------|
| grid-cli **不依赖** ratatui/crossterm | CLI 场景不需要全屏终端 |
| grid-studio **不依赖** rustyline | TUI 有自己的输入框 |
| grid-cli-common **不依赖** ratatui/crossterm/rustyline/axum | 纯共享逻辑 |
| grid-cli-common **不包含** 产品特有逻辑 | 只放两个产品都需要的代码 |
| 共享 commands **不感知** UI 模式 | handlers 返回数据，展示由调用者决定 |
