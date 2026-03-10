# Octo-CLI 重新设计与实现计划

**创建日期**: 2026-03-10
**分支**: main
**基线**: 904 tests passing (commit 71dc7fc)
**设计文档**: docs/design/AGENT_CLI_DESIGN.md

---

## 第一部分：研究结论与技术决策

### 1.1 REPL 库选型：rustyline vs reedline

| 维度 | rustyline v17 | reedline v0.38 |
|------|--------------|----------------|
| **生态验证** | IronClaw + ZeroClaw 两个参考项目使用 | 无参考项目使用 |
| **成熟度** | 10+ 年历史，GNU Readline API 兼容 | 3 年历史，Nushell 团队维护 |
| **Trait 扩展** | Completer/Hinter/Highlighter/Validator/Helper | Completer/Validator/Highlighter |
| **vi/emacs 模式** | 内建支持（`EditMode::Vi`） | 内建支持（`EditMode::Vi`） |
| **历史记录** | `FileHistory` 内建，支持搜索 | `FileBackedHistory` 内建 |
| **async 兼容** | 阻塞式 `readline()`，需 spawn_blocking | 阻塞式 `read_line()`，需 spawn_blocking |
| **多行输入** | 通过 Validator 控制续行 | 原生多行支持 |
| **依赖大小** | 较小（核心库） | 较大（含 crossterm 子系统） |
| **文档质量** | 优秀，大量 examples | 良好，文档较少 |

**决策：选用 `rustyline v17`**

理由：
1. 两个参考项目（IronClaw、ZeroClaw）已验证其在智能体 CLI 场景的可用性
2. IronClaw 的 `ReplHelper` 实现（Completer + Hinter + Highlighter）是完整可参考的模式
3. ZeroClaw 的 `SlashCommandCompleter` 是 slash 命令补全的标准实现
4. rustyline 的 `custom-bindings` feature 支持自定义快捷键
5. 社区更成熟，遇到问题更容易找到解决方案

### 1.2 TUI 框架：确认 Ratatui 0.29

- OpenFang 使用 Ratatui 0.29 + crossterm，实现了 19 个 Tab 屏幕、2,437 行核心 App Shell
- ZeroClaw 同样使用 Ratatui 0.29
- **决策：直接复用 OpenFang 的 TUI 架构**，包括事件循环、状态机、主题系统

### 1.3 Web Dashboard：确认 Alpine.js 嵌入式方案

- OpenFang 的 `include_str!()` + Alpine.js 模式已验证
- 单二进制部署、零构建依赖
- **`web/` React 前端标记为 Deferred**，不在本计划范围

### 1.4 octo-engine API 集成点确认

研究确认 octo-engine 已暴露完整的 CLI 集成 API：

| 组件 | 关键 API | 文件 |
|------|---------|------|
| **AgentRuntime** | `start_primary()` → `AgentExecutorHandle` | `agent/runtime.rs` |
| **AgentExecutorHandle** | `send(AgentMessage)` + `subscribe()` → `broadcast::Receiver<AgentEvent>` | `agent/executor.rs` |
| **run_agent_loop** | `(config, messages) -> BoxStream<AgentEvent>` | `agent/harness.rs` |
| **AgentEvent** | TextDelta/ToolStart/ToolResult/Done/Completed 等 17 个变体 | `agent/events.rs` |
| **ToolRegistry** | `specs()`/`get()`/`names()` | `tools/mod.rs` |
| **McpManager** | `add_server()`/`remove_server()`/`bridge_tools()` | `mcp/manager.rs` |
| **SessionStore** | `create_session()`/`get_messages()`/`list_sessions()` | `session/mod.rs` |
| **MemoryStore** | `store()`/`search()`/`list()` | `memory/store_traits.rs` |
| **AgentCatalog** | `register()`/`list_all()`/`get()` | `agent/catalog.rs` |
| **SkillRegistry** | `list_all()`/`get()`/`invocable_skills()` | `skills/registry.rs` |

**需要新增的 Engine 接口**（详见第三部分）：

| 新增 API | 位置 | 说明 |
|---------|------|------|
| `AgentRuntime::send_message_streaming()` | `agent/runtime.rs` | CLI 友好的高层消息 API |
| `AgentRuntime::create_session_and_start()` | `agent/runtime.rs` | 一步完成 Session 创建 + Agent 启动 |
| `SessionStore::delete_session()` | `session/mod.rs` | Session 删除（当前缺失） |
| `SessionStore::most_recent_session()` | `session/mod.rs` | 支持 `--continue` 恢复最近会话 |

---

## 第二部分：模块结构设计

### 2.1 目标目录结构

```
crates/octo-cli/src/
├── main.rs                          # 入口：Clap 解析 + 全局选项 + 路由
├── commands/
│   ├── mod.rs                       # 命令路由 dispatch
│   ├── types.rs                     # Clap 命令/子命令枚举定义
│   ├── state.rs                     # AppState（复用 octo-engine）
│   ├── run.rs                       # octo run — REPL 启动入口
│   ├── ask.rs                       # octo ask — 单次查询（Headless）
│   ├── agent.rs                     # Agent 生命周期管理
│   ├── session.rs                   # Session CRUD + 导出
│   ├── memory.rs                    # Memory 搜索/列表/添加
│   ├── tool.rs                      # Tool 列表/详情/调用
│   ├── mcp.rs                       # MCP Server 管理 (NEW)
│   ├── config.rs                    # 配置管理（增强）
│   ├── doctor.rs                    # 健康诊断 (NEW)
│   └── completions.rs              # Shell 补全生成 (NEW)
├── repl/
│   ├── mod.rs                       # REPL 主循环
│   ├── helper.rs                    # ReplHelper: Completer + Hinter + Highlighter
│   ├── slash.rs                     # Slash 命令解析和执行
│   └── history.rs                   # 会话历史管理
├── ui/
│   ├── mod.rs                       # UI 抽象层
│   ├── streaming.rs                 # Token 流式渲染
│   ├── markdown.rs                  # Markdown 终端渲染（termimad）
│   ├── spinner.rs                   # Spinner 和进度（indicatif）
│   ├── table.rs                     # 表格格式化输出
│   └── theme.rs                     # 颜色主题管理
├── tui/                             # 全屏 TUI（基于 OpenFang fork）
│   ├── mod.rs                       # App 状态机 + Tab 导航 + Event Loop
│   ├── event.rs                     # 统一事件系统
│   ├── theme.rs                     # octo 品牌主题
│   ├── backend.rs                   # 后端抽象层（InProcess / HTTP）
│   ├── chat_runner.rs               # 独立 Chat TUI
│   ├── launcher.rs                  # 交互式启动菜单
│   └── screens/
│       ├── mod.rs                   # 屏幕路由
│       ├── welcome.rs               # 欢迎页 + 首次配置向导
│       ├── dashboard.rs             # 系统概览
│       ├── agents.rs                # Agent 管理
│       ├── chat.rs                  # Chat 交互（核心）
│       ├── sessions.rs              # Session 列表 + 恢复
│       ├── memory.rs                # 多层内存浏览器
│       ├── skills.rs                # Skills 管理
│       ├── mcp.rs                   # MCP Server 管理
│       ├── tools.rs                 # 工具列表 + 执行历史
│       ├── security.rs              # 安全策略 + 审计日志
│       ├── settings.rs              # 配置管理
│       └── logs.rs                  # 结构化日志查看
└── output/
    ├── mod.rs                       # 输出格式抽象
    ├── text.rs                      # 纯文本输出
    ├── json.rs                      # JSON 输出
    └── stream_json.rs               # Stream-JSON 输出（行分隔）
```

### 2.2 依赖清单更新

```toml
[dependencies]
# 核心
octo-engine = { workspace = true }
octo-types = { workspace = true }
octo-sandbox = { workspace = true }

# CLI 解析
clap = { version = "4.5", features = ["derive", "env"] }
clap_complete = "4.5"

# 异步运行时
tokio = { workspace = true, features = ["full"] }
futures-util = { workspace = true }

# REPL / 行编辑（参考 IronClaw + ZeroClaw）
rustyline = { version = "17", features = ["custom-bindings", "derive", "with-file-history"] }

# 终端渲染
termimad = "0.31"                    # Markdown 终端渲染
indicatif = "0.17"                   # Spinner、进度条
owo-colors = "4"                     # 零分配彩色输出
crossterm = "0.28"                   # 终端控制

# TUI 框架（参考 OpenFang）
ratatui = "0.29"                     # 终端 UI

# 配置 & 数据
rusqlite = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
directories = "5"                    # XDG 路径规范

# 错误 & 日志
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
dotenvy = { workspace = true }

# UUID & 时间
uuid = { workspace = true }
chrono = { workspace = true }
```

---

## 第三部分：octo-engine 接口增强

### 3.1 AgentRuntime 新增方法

```rust
// crates/octo-engine/src/agent/runtime.rs

impl AgentRuntime {
    /// CLI 友好的高层消息 API：创建/恢复 Session + 发送消息 + 返回事件流
    ///
    /// 封装了 start_primary() + AgentExecutorHandle::send() + subscribe() 的完整流程
    pub async fn send_message_streaming(
        &self,
        session_id: &SessionId,
        content: &str,
    ) -> Result<broadcast::Receiver<AgentEvent>, AgentError> {
        // 1. 获取或创建 primary handle
        let handle = self.primary().await
            .ok_or_else(|| AgentError::Internal("No active agent session".into()))?;

        // 2. 订阅事件流（必须在 send 之前订阅，避免丢失事件）
        let rx = handle.subscribe();

        // 3. 发送用户消息
        handle.send(AgentMessage::UserMessage {
            content: content.to_string(),
            channel_id: "cli".to_string(),
        }).await.map_err(|e| AgentError::Internal(e.to_string()))?;

        Ok(rx)
    }

    /// 一步完成 Session 创建 + Agent 启动
    pub async fn create_session_and_start(
        &self,
        user_id: UserId,
        agent_id: Option<&AgentId>,
        resume_session: Option<SessionId>,
    ) -> Result<(SessionId, AgentExecutorHandle), AgentError> {
        let session_store = self.session_store();

        // 恢复或创建 session
        let session = match resume_session {
            Some(sid) => {
                session_store.get_session(&sid).await
                    .ok_or_else(|| AgentError::Internal(format!("Session not found: {}", sid)))?
            }
            None => {
                session_store.create_session_with_user(&user_id).await
            }
        };

        // 加载历史消息
        let history = session_store.get_messages(&session.session_id).await
            .unwrap_or_default();

        // 启动 agent
        let handle = self.start_primary(
            session.session_id.clone(),
            user_id,
            session.sandbox_id.clone(),
            history,
            agent_id,
        ).await;

        Ok((session.session_id, handle))
    }
}
```

### 3.2 SessionStore 新增方法

```rust
// crates/octo-engine/src/session/mod.rs

#[async_trait]
pub trait SessionStore: Send + Sync {
    // ... 现有方法 ...

    /// 删除 Session 及其所有消息
    async fn delete_session(&self, session_id: &SessionId) -> bool;

    /// 获取最近的 Session（用于 --continue 功能）
    async fn most_recent_session(&self) -> Option<SessionData>;

    /// 获取用户最近的 Session
    async fn most_recent_session_for_user(&self, user_id: &UserId) -> Option<SessionData>;
}
```

### 3.3 SessionStore SQLite 实现

```rust
// crates/octo-engine/src/session/sqlite.rs — 新增方法

impl SessionStore for SqliteSessionStore {
    async fn delete_session(&self, session_id: &SessionId) -> bool {
        let sid = session_id.to_string();
        self.db.call(move |conn| {
            conn.execute("DELETE FROM session_messages WHERE session_id = ?", [&sid])?;
            let changes = conn.execute("DELETE FROM sessions WHERE session_id = ?", [&sid])?;
            Ok(changes > 0)
        }).await.unwrap_or(false)
    }

    async fn most_recent_session(&self) -> Option<SessionData> {
        self.db.call(|conn| {
            conn.query_row(
                "SELECT session_id, user_id, sandbox_id, created_at
                 FROM sessions ORDER BY created_at DESC LIMIT 1",
                [],
                |row| Ok(SessionData { /* ... */ }),
            ).optional()
        }).await.ok().flatten()
    }

    async fn most_recent_session_for_user(&self, user_id: &UserId) -> Option<SessionData> {
        let uid = user_id.to_string();
        self.db.call(move |conn| {
            conn.query_row(
                "SELECT session_id, user_id, sandbox_id, created_at
                 FROM sessions WHERE user_id = ? ORDER BY created_at DESC LIMIT 1",
                [&uid],
                |row| Ok(SessionData { /* ... */ }),
            ).optional()
        }).await.ok().flatten()
    }
}
```

---

## 第四部分：实现任务分解

### Phase 1：CLI 核心基础设施 (R1-R8)

> 目标：完整的命令结构 + Headless 模式 + 输出系统 + 所有子命令实现

| ID | 任务 | 文件 | 依赖 | 说明 |
|----|------|------|------|------|
| R1 | 重写命令结构 | `types.rs`, `main.rs`, `mod.rs` | 无 | 按设计文档 4.1 定义完整 Clap 枚举 |
| R2 | 输出格式系统 | `output/mod.rs`, `text.rs`, `json.rs`, `stream_json.rs` | 无 | text/json/stream-json 三种模式 |
| R3 | UI 基础组件 | `ui/theme.rs`, `ui/table.rs`, `ui/spinner.rs`, `ui/markdown.rs` | 无 | termimad + indicatif + owo-colors |
| R4 | AppState 增强 | `state.rs` | R1 | 加载 SkillRegistry/McpManager，支持工作目录配置 |
| R5 | Engine 接口增强 | `runtime.rs`, `session/mod.rs`, `session/sqlite.rs` | 无 | 3.1-3.3 新增方法 |
| R6 | Headless 模式 (ask) | `commands/ask.rs` | R2, R4, R5 | `octo ask "message"` 单次查询 |
| R7 | Agent 子命令 | `commands/agent.rs` | R3, R4 | list/info/create/start/pause/stop/delete |
| R8 | Session 子命令 | `commands/session.rs` | R3, R4, R5 | list/create/show/delete/export |

### Phase 2：REPL 交互模式 (R9-R14)

> 目标：完整的交互式 REPL + 流式渲染 + Slash 命令

| ID | 任务 | 文件 | 依赖 | 说明 |
|----|------|------|------|------|
| R9 | 流式渲染引擎 | `ui/streaming.rs` | R2, R3 | AgentEvent → 终端渲染（text/thinking/tool） |
| R10 | ReplHelper | `repl/helper.rs` | 无 | Completer/Hinter/Highlighter（参考 IronClaw） |
| R11 | REPL 主循环 | `repl/mod.rs`, `commands/run.rs` | R4, R5, R9, R10 | rustyline 集成 + Session 管理 + 流式输出 |
| R12 | Slash 命令系统 | `repl/slash.rs` | R11 | /help /compact /undo /cost /model /clear /save /exit |
| R13 | 历史管理 | `repl/history.rs` | R11 | 持久化历史 + 搜索（XDG 路径） |
| R14 | @file 引用展开 | `repl/mod.rs` | R11 | 用户消息中 @path 展开为文件内容 |

### Phase 3：管理子命令补全 (R15-R20)

> 目标：所有管理子命令完整实现 + 诊断工具

| ID | 任务 | 文件 | 依赖 | 说明 |
|----|------|------|------|------|
| R15 | Memory 子命令 | `commands/memory.rs` | R3, R4 | search/list/add/graph — 复用 MemoryStore/WorkingMemory |
| R16 | Tool 子命令 | `commands/tool.rs` | R3, R4 | list/info/invoke — 复用 ToolRegistry |
| R17 | MCP 子命令 | `commands/mcp.rs` | R3, R4 | list/add/remove/status/logs/test — 复用 McpManager |
| R18 | Config 增强 | `commands/config.rs` | R3 | init 向导 + get/set/paths |
| R19 | doctor 诊断 | `commands/doctor.rs` | R4 | 环境检查 + --repair 自动修复 |
| R20 | Shell 补全 | `commands/completions.rs` | R1 | clap_complete 生成 bash/zsh/fish |

### Phase 4：TUI 全屏模式 (T1-T8)

> 目标：基于 OpenFang fork 的全屏 TUI + 12 个屏幕

| ID | 任务 | 文件 | 依赖 | 说明 |
|----|------|------|------|------|
| T1 | App Shell + Event Loop | `tui/mod.rs`, `tui/event.rs` | R4 | 从 OpenFang fork，替换 Backend enum |
| T2 | TuiBackend trait | `tui/backend.rs` | R4 | InProcessBackend（包装 AgentRuntime）+ HttpBackend |
| T3 | Theme 系统 | `tui/theme.rs` | 无 | 从 OpenFang fork，替换品牌色（国网绿 #00843D） |
| T4 | Chat 屏幕 | `tui/screens/chat.rs` | T1, T2 | 从 OpenFang fork chat.rs + chat_runner.rs |
| T5 | Welcome + Dashboard | `tui/screens/welcome.rs`, `dashboard.rs` | T1, T3 | 引导 + 系统概览 |
| T6 | Launcher 菜单 | `tui/launcher.rs` | T1, T3 | 从 OpenFang fork launcher.rs |
| T7 | Agent/Session 屏幕 | `tui/screens/agents.rs`, `sessions.rs` | T1, T2 | Agent 管理 + Session 列表 |
| T8 | 功能屏幕 | `tui/screens/memory.rs`, `mcp.rs`, `tools.rs`, `skills.rs`, `security.rs`, `settings.rs`, `logs.rs` | T1, T2 | 7 个功能屏幕 |

### Phase 5：高级功能 (A1-A6)

> 目标：差异化功能 + 竞争力对齐

| ID | 任务 | 文件 | 依赖 | 说明 |
|----|------|------|------|------|
| A1 | Plan/Build 模式切换 | `repl/slash.rs` | R12 | /mode plan (只读) / /mode build (完整权限) |
| A2 | Cost/Token 追踪 | `ui/streaming.rs`, `repl/slash.rs` | R9, R12 | 每轮 + 累计 Token + 费用 + /cost 命令 |
| A3 | 上下文压缩 | `repl/slash.rs` | R12 | /compact 命令，复用 ContextPruner |
| A4 | Session 导出 | `commands/session.rs` | R8 | HTML/JSON/Markdown 格式导出 |
| A5 | Hook 系统集成 | `repl/mod.rs` | R11 | PreToolUse / PostToolUse 生命周期事件 |
| A6 | --add-dir 上下文 | `commands/run.rs` | R11 | 额外目录上下文注入 |

---

## 第五部分：REPL 核心循环详细设计

### 5.1 ReplHelper 实现（参考 IronClaw）

```rust
// crates/octo-cli/src/repl/helper.rs

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help",    "显示帮助信息"),
    ("/compact", "压缩上下文"),
    ("/undo",    "撤销最近工具操作"),
    ("/cost",    "显示 Token 用量和费用"),
    ("/model",   "切换模型"),
    ("/mode",    "切换模式 (plan/build)"),
    ("/clear",   "清除当前对话"),
    ("/save",    "保存会话"),
    ("/exit",    "退出"),
];

pub struct ReplHelper {
    pub is_streaming: Arc<AtomicBool>,
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        if !line.starts_with('/') {
            return Ok((0, vec![]));
        }

        let prefix = &line[..pos];
        let matches: Vec<Pair> = SLASH_COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(prefix))
            .map(|(cmd, desc)| Pair {
                display: format!("{cmd}  {desc}"),
                replacement: cmd.to_string(),
            })
            .collect();

        Ok((0, matches))
    }
}

impl Hinter for ReplHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        if line.starts_with('/') && pos == line.len() {
            SLASH_COMMANDS
                .iter()
                .find(|(cmd, _)| cmd.starts_with(line) && *cmd != line)
                .map(|(cmd, _)| cmd[line.len()..].to_string())
        } else {
            None
        }
    }
}

impl Highlighter for ReplHelper {}
impl Validator for ReplHelper {}
impl Helper for ReplHelper {}
```

### 5.2 REPL 主循环

```rust
// crates/octo-cli/src/repl/mod.rs

pub async fn run_repl(state: &AppState, opts: &RunOptions) -> Result<()> {
    let user_id = UserId::from_string("cli-user");

    // 1. 解析 Session：恢复或新建
    let (session_id, handle) = state.agent_runtime.create_session_and_start(
        user_id.clone(),
        opts.agent_id.as_ref(),
        resolve_resume_session(state, opts).await?,
    ).await?;

    // 2. 初始化 rustyline
    let is_streaming = Arc::new(AtomicBool::new(false));
    let helper = ReplHelper {
        is_streaming: is_streaming.clone(),
    };

    let config = rustyline::Config::builder()
        .max_history_size(1000)?
        .edit_mode(if opts.vi_mode {
            rustyline::EditMode::Vi
        } else {
            rustyline::EditMode::Emacs
        })
        .build();

    let mut rl = rustyline::Editor::with_config(config)?;
    rl.set_helper(Some(helper));

    // 加载历史
    let history_path = history_file_path(&session_id)?;
    let _ = rl.load_history(&history_path);

    // 3. 显示欢迎信息
    show_welcome(&session_id, state).await;

    // 4. 主循环
    loop {
        // 等待流式输出完成
        while is_streaming.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let prompt = build_prompt(state);
        match rl.readline(&prompt) {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) if line.starts_with('/') => {
                let action = handle_slash_command(&line, state, &session_id).await?;
                if action == SlashAction::Exit {
                    show_resume_hint(&session_id);
                    break;
                }
            }
            Ok(line) => {
                rl.add_history_entry(&line)?;

                // 展开 @file 引用
                let expanded = expand_file_refs(&line)?;

                // 发送消息，获取事件流
                is_streaming.store(true, Ordering::Relaxed);
                let rx = state.agent_runtime
                    .send_message_streaming(&session_id, &expanded).await?;

                // 流式渲染响应
                render_streaming_response(rx, &state.output_config).await?;

                is_streaming.store(false, Ordering::Relaxed);
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                // Ctrl+C: 中断当前操作
                handle.send(AgentMessage::Cancel).await.ok();
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                // Ctrl+D: 退出
                show_resume_hint(&session_id);
                break;
            }
            Err(e) => return Err(e.into()),
        }
    }

    // 保存历史
    rl.save_history(&history_path)?;
    Ok(())
}
```

### 5.3 流式渲染引擎

```rust
// crates/octo-cli/src/ui/streaming.rs

pub async fn render_streaming_response(
    mut rx: broadcast::Receiver<AgentEvent>,
    config: &OutputConfig,
) -> Result<()> {
    match config.format {
        OutputFormat::Text => render_text_stream(&mut rx).await,
        OutputFormat::Json => render_json_stream(&mut rx).await,
        OutputFormat::StreamJson => render_stream_json(&mut rx).await,
    }
}

async fn render_text_stream(rx: &mut broadcast::Receiver<AgentEvent>) -> Result<()> {
    let mut stdout = std::io::stdout();
    let mut spinner: Option<ProgressBar> = None;
    let mut total_input_tokens = 0u64;
    let mut total_output_tokens = 0u64;

    loop {
        match rx.recv().await {
            Ok(AgentEvent::TextDelta { text }) => {
                // 停止 spinner（如果有）
                if let Some(sp) = spinner.take() {
                    sp.finish_and_clear();
                }
                write!(stdout, "{}", text)?;
                stdout.flush()?;
            }
            Ok(AgentEvent::ThinkingDelta { text }) => {
                // 灰色显示思考过程
                write!(stdout, "{}", text.style(owo_colors::Style::new().dimmed()))?;
                stdout.flush()?;
            }
            Ok(AgentEvent::ToolStart { tool_name, input, .. }) => {
                // 显示工具调用 Spinner
                let sp = ProgressBar::new_spinner();
                sp.set_style(ProgressStyle::with_template(
                    "  {spinner:.cyan} {msg}"
                )?.tick_chars("⣿⠿⢟⠯⠷ "));
                sp.set_message(format!("{}({})", tool_name, truncate_json(&input, 60)));
                sp.enable_steady_tick(Duration::from_millis(100));
                spinner = Some(sp);
            }
            Ok(AgentEvent::ToolResult { tool_id: _, output, success }) => {
                if let Some(sp) = spinner.take() {
                    sp.finish_and_clear();
                }
                let icon = if success { "✔" } else { "✘" };
                let color = if success { "green" } else { "red" };
                eprintln!("  {} {}", icon.color(color), truncate(&output, 120));
            }
            Ok(AgentEvent::TokenBudgetUpdate { budget }) => {
                // 静默更新（状态栏显示）
            }
            Ok(AgentEvent::Error { message }) => {
                if let Some(sp) = spinner.take() {
                    sp.finish_and_clear();
                }
                eprintln!("\n{}: {}", "Error".red().bold(), message);
            }
            Ok(AgentEvent::Done) => {
                println!(); // 换行
                break;
            }
            Ok(AgentEvent::Completed(result)) => {
                // 显示 Token 用量
                eprintln!(
                    "\n{} rounds, {} tool calls",
                    result.rounds, result.tool_calls,
                );
                break;
            }
            Ok(_) => {} // 忽略其他事件
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Skipped {} events", n);
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }

    Ok(())
}
```

---

## 第六部分：TUI 迁移策略

### 6.1 从 OpenFang 直接 fork 的组件

| 组件 | OpenFang 源文件 | 预计行数 | 改动量 |
|------|----------------|---------|--------|
| Event Loop 骨架 | `tui/mod.rs` (2,437 行) | ~1,500 | 删除 OpenFang 特有 Tab，替换 Backend enum |
| Event System | `tui/event.rs` (500 行) | ~400 | 替换 StreamEvent → AgentEvent 映射 |
| Theme | `tui/theme.rs` (200 行) | ~200 | 替换品牌色（orange → 国网绿） |
| Chat Screen | `screens/chat.rs` (887 行) | ~800 | 适配 AgentEvent 类型 |
| Chat Runner | `tui/chat_runner.rs` (400 行) | ~350 | 适配 Backend 初始化 |
| Launcher | `launcher.rs` (605 行) | ~400 | 替换菜单选项和品牌 |
| Dashboard | `screens/dashboard.rs` (278 行) | ~250 | 适配 octo metrics 数据源 |
| Sessions | `screens/sessions.rs` (313 行) | ~300 | 适配 SessionStore 类型 |
| Settings | `screens/settings.rs` | ~400 | 适配 octo config 系统 |
| Logs | `screens/logs.rs` | ~300 | 适配 EventBus |

### 6.2 需要新建的组件

| 组件 | 说明 | 预计行数 |
|------|------|---------|
| `tui/backend.rs` | TuiBackend trait + InProcessBackend + HttpBackend | ~500 |
| `screens/memory.rs` | 多层内存浏览器（L0/L1/L2 + KG） | ~800 |
| `screens/mcp.rs` | MCP Server 管理（启停/工具列表/日志） | ~800 |
| `screens/tools.rs` | 工具列表 + 执行历史 | ~600 |
| `screens/skills.rs` | Skills 管理（适配 SkillRegistry） | ~600 |
| `screens/security.rs` | 安全策略 + 审计日志 | ~600 |

### 6.3 TuiBackend trait 设计

```rust
// crates/octo-cli/src/tui/backend.rs

#[async_trait]
pub trait TuiBackend: Send + Sync {
    // Agent 操作
    async fn list_agents(&self) -> Result<Vec<AgentEntry>>;
    async fn send_message(&self, session_id: &SessionId, msg: &str)
        -> Result<broadcast::Receiver<AgentEvent>>;
    async fn cancel(&self, session_id: &SessionId) -> Result<()>;

    // Session 操作
    async fn list_sessions(&self, limit: usize) -> Result<Vec<SessionSummary>>;
    async fn create_session(&self) -> Result<SessionData>;
    async fn delete_session(&self, session_id: &SessionId) -> Result<bool>;

    // Memory 操作
    async fn search_memory(&self, query: &str, limit: usize) -> Result<Vec<MemoryResult>>;
    async fn list_memory(&self, filter: MemoryFilter) -> Result<Vec<MemoryEntry>>;

    // MCP 操作
    async fn list_mcp_servers(&self) -> Result<HashMap<String, ServerRuntimeState>>;
    async fn add_mcp_server(&self, config: McpServerConfig) -> Result<Vec<McpToolInfo>>;
    async fn remove_mcp_server(&self, name: &str) -> Result<()>;

    // Tool 操作
    async fn list_tools(&self) -> Result<Vec<ToolSpec>>;

    // Skill 操作
    async fn list_skills(&self) -> Result<Vec<SkillDefinition>>;

    // Config / Metrics
    async fn get_metrics(&self) -> Result<MeteringSnapshot>;
}

/// InProcess 后端 — CLI 直接持有 octo-engine
pub struct InProcessBackend {
    runtime: Arc<AgentRuntime>,
}

/// HTTP 后端 — 连接远程 octo-server
pub struct HttpBackend {
    base_url: String,
    client: reqwest::Client,
}
```

---

## 第七部分：执行计划与依赖关系图

```
Phase 1 (CLI 基础设施)
┌─────────────────────────────────────────────┐
│ R1: 命令结构    R2: 输出系统    R3: UI 组件  │ ← 并行
│ R5: Engine 增强                              │ ← 并行
├─────────────────────────────────────────────┤
│ R4: AppState 增强                            │ ← 依赖 R1
├─────────────────────────────────────────────┤
│ R6: ask (Headless)                           │ ← 依赖 R2, R4, R5
│ R7: agent 子命令   R8: session 子命令        │ ← 依赖 R3, R4
└─────────────────────────────────────────────┘

Phase 2 (REPL 交互)
┌─────────────────────────────────────────────┐
│ R9: 流式渲染     R10: ReplHelper            │ ← 并行
├─────────────────────────────────────────────┤
│ R11: REPL 主循环                             │ ← 依赖 R4, R5, R9, R10
├─────────────────────────────────────────────┤
│ R12: Slash 命令   R13: 历史管理              │ ← 依赖 R11
│ R14: @file 引用                              │ ← 依赖 R11
└─────────────────────────────────────────────┘

Phase 3 (管理子命令)
┌─────────────────────────────────────────────┐
│ R15: memory    R16: tool    R17: mcp        │ ← 并行
│ R18: config    R19: doctor  R20: completions│ ← 并行
└─────────────────────────────────────────────┘

Phase 4 (TUI 全屏)
┌─────────────────────────────────────────────┐
│ T1: App Shell   T3: Theme                   │ ← 并行
├─────────────────────────────────────────────┤
│ T2: TuiBackend                               │ ← 依赖 T1
├─────────────────────────────────────────────┤
│ T4: Chat   T5: Welcome   T6: Launcher      │ ← 依赖 T1, T2
├─────────────────────────────────────────────┤
│ T7: Agent/Session   T8: 功能屏幕            │ ← 依赖 T1, T2
└─────────────────────────────────────────────┘

Phase 5 (高级功能)
┌─────────────────────────────────────────────┐
│ A1-A6: 差异化功能                            │ ← 依赖 Phase 2
└─────────────────────────────────────────────┘
```

### 任务总计

| Phase | 任务数 | 预计新增行数 | 说明 |
|-------|--------|-------------|------|
| Phase 1 | 8 | ~2,500 | CLI 骨架 + Engine 接口 |
| Phase 2 | 6 | ~2,000 | REPL 核心 |
| Phase 3 | 6 | ~1,500 | 管理命令 |
| Phase 4 | 8 | ~7,000 | TUI（含 OpenFang fork） |
| Phase 5 | 6 | ~1,000 | 高级功能 |
| **总计** | **34** | **~14,000** | |

---

## 第八部分：Deferred 项目

以下功能不在本计划范围，记录为后续工作：

| ID | 功能 | 说明 | 前置条件 |
|----|------|------|----------|
| D1 | `web/` React 前端 | 保持现状，不删除不维护 | — |
| D2 | 嵌入式 Web Dashboard | Alpine.js SPA（设计文档 Part 6） | Phase 4 完成后 |
| D3 | Tauri Desktop 桌面应用 | 设计文档 6.8 备选方案 | D2 完成后 |
| D4 | Auto-memory 跨 Session | 自动学习项目记忆 | Phase 5 完成后 |
| D5 | 双 Agent 模式 | Plan + Build Agent Tab 切换 | Phase 5 A1 完成后 |

---

## 第九部分：内置配色方案设计

### 9.1 配色方案总览

octo-cli/TUI/Web Dashboard 内置 12 种配色方案，覆盖冷色、暖色、渐变、无色系四大类别，满足不同用户审美偏好。

详细预览文件：`docs/design/color_comparison.html`（`open docs/design/color_comparison.html` 查看）

| # | 名称 | 主色 | 色系 | 灵感来源 |
|---|------|------|------|----------|
| 1 | 国网绿 | `#00843D` | 冷色-绿 | 能源行业、官方稳重 |
| 2 | 道奇蓝 | `#3B82F6` | 冷色-蓝 | 经典科技蓝、信任感 |
| 3 | 海洋青 (推荐) | `#06B6D4` | 冷色-青 | 终端感、章鱼/海洋主题 |
| 4 | 深海靛蓝 | `#6366F1` | 冷色-靛 | VS Code / GitHub IDE 风格 |
| 5 | 墨紫 | `#8B5CF6` | 冷色-紫 | 神秘感、章鱼墨汁 |
| 6 | 翡翠绿 | `#10B981` | 冷色-翠 | Matrix 风格、终端经典 |
| 7 | 琥珀金 | `#F59E0B` | 暖色-金 | Warp Terminal、暗色下醒目 |
| 8 | 珊瑚橙 | `#F97316` | 暖色-橙 | Rust 品牌色、OpenFang 原色 |
| 9 | 玫瑰红 | `#F43F5E` | 暖色-红 | Vercel/v0、Zed Editor |
| 10 | 薄荷蓝 | `#14B8A6` | 冷色-蓝绿 | Tailwind Teal、Mintlify |
| 11 | 日落渐变 | `#EC4899->#F59E0B` | 双色渐变 | GitHub Copilot、Cursor |
| 12 | 月光银 | `#94A3B8` | 无色系 | Ghostty、极简主义 |

### 9.2 CSS 变量规范

每个主题定义 4 个核心 CSS 变量（渐变主题额外定义 `--accent2`/`--accent2-text`）：

```css
--accent       /* 主色（按钮、进度条填充、状态指示灯） */
--accent-dim   /* 暗色变体（边框、hover 背景） */
--accent-glow  /* 半透明发光（消息气泡背景、活跃项高亮） */
--accent-text  /* 文本色（标题、链接、数值强调） */
```

### 9.3 终端 TUI 适配

- 所有颜色为 24-bit true color，Ratatui `Color::Rgb()` 原生支持
- 在 256 色终端自动降级到最近 ANSI 颜色
- 渐变主题（日落）在 TUI 中使用 accent + accent2 双色近似
- 兼容终端：iTerm2, Alacritty, Windows Terminal, Kitty, WezTerm

### 9.4 实现位置

| 组件 | 文件 | 说明 |
|------|------|------|
| TUI Theme | `crates/octo-cli/src/tui/theme.rs` | Ratatui Style 定义 |
| REPL 颜色 | `crates/octo-cli/src/ui/theme.rs` | owo-colors 样式 |
| Web Dashboard | 嵌入 HTML CSS 变量 | Alpine.js 主题切换 |
| 配色预览 | `docs/design/color_comparison.html` | 设计参考 |

### 9.5 配置方式

```yaml
# config.yaml
cli:
  theme: cyan          # 默认主题：海洋青
  # 可选值: sgcc, openfang, cyan, indigo, violet, emerald,
  #         amber, coral, rose, teal, sunset, slate
```

命令行覆盖：
```bash
octo run --theme amber
octo tui --theme sunset
```

REPL 运行时切换：
```
/theme slate
/theme list
```

---

## 附录：参考项目 REPL 库对比

| 项目 | 库 | 版本 | 特色功能 | 与 octo 相关度 |
|------|-----|------|---------|---------------|
| **IronClaw** | rustyline | v17 | ReplHelper 5 trait 完整实现、22 个 Slash 命令、termimad Markdown | **高** — 直接参考 |
| **ZeroClaw** | rustyline | v17 | SlashCommandCompleter + dialoguer 确认 + Ratatui TUI | **高** — REPL + TUI 共存 |
| **Pi Agent** | charmed-rust | — | BubbleTea/Lipgloss/Glamour Rust 绑定 | 低 — Go 生态移植 |
| **Goose** | 无 REPL | — | HTTP API、Recipe 系统 | 低 — 无 REPL |
| **OpenFang** | 无 REPL | — | Ratatui TUI + Alpine.js Web | **高** — TUI 架构参考 |
| **LocalGPT** | 无 REPL | — | Server-based | 低 |
| **Moltis** | 无 REPL | — | Gateway-first Web | 低 |
