# Phase 2 Batch 3 设计文档

**日期**: 2026-02-26
**范围**: Skill Loader + MCP Client + Tool Execution 记录 + REST API + 最小 Debug UI
**前置**: Phase 2 Batch 2 完成（SQLite 持久化 + Memory 系统 + 混合检索）

---

## 1. 总体范围

### 1.1 本批次交付

| 组件 | 说明 |
|------|------|
| **Skill Loader** | SKILL.md 解析 + 系统提示注入 + user-invocable 工具化 + 热重载 |
| **MCP Client** | rmcp 薄封装 + McpClient trait + stdio transport + ToolRegistry 桥接 |
| **MCP Manager** | 多 MCP Server 管理 + .octo/mcp.json 配置 |
| **Tool Execution 记录** | SQLite 存储 + AgentLoop 集成 + WebSocket 实时事件 |
| **REST API** | 8 个端点（sessions, executions, tools, memories, budget） |
| **最小 Debug UI** | 3 Tab（Chat \| Tools \| Debug）+ 执行列表 + Token 预算仪表盘 |

### 1.2 延迟到 Batch 4

- MCP Server 管理 UI（ServerList, ToolCallForm, LogStream）
- Skills 编辑器 UI（SkillEditor, SkillPreview）
- Debug Interceptor（请求/响应拦截）
- Cross-Agent Comparison
- Memory Explorer UI
- Terminal 组件（xterm.js）
- Diff 视图（Monaco Editor）

---

## 2. Skill Loader

### 2.1 SKILL.md 格式规范

```yaml
---
name: skill-name                    # 必需，小写字母+数字+连字符，最多 64 字符
description: |                      # 必需，最多 1024 字符
  Skill 的功能描述和触发条件。
  以第三人称编写，注入系统提示供 Agent 发现。
version: "1.0.0"                    # 可选，语义版本号
user-invocable: true                # 可选，默认 false。允许用户手动调用
allowed-tools:                      # 可选，Skill 可使用的工具白名单
  - Read
  - Write
  - Bash
---

# Skill 标题

Markdown 指令正文...

支持模板变量: ${baseDir} 替换为 SKILL.md 所在目录的绝对路径。
```

### 2.2 文件发现顺序

```
优先级从高到低:
1. {project}/.octo/skills/*/SKILL.md    — 项目级 Skills
2. ~/.octo/skills/*/SKILL.md            — 用户级 Skills
3. (Phase 4) 插件内 skills              — 插件级 Skills
```

同名 Skill 项目级覆盖用户级。

### 2.3 核心类型

```rust
// crates/octo-types/src/skill.rs

/// Skill 定义（从 SKILL.md 解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub user_invocable: bool,
    pub allowed_tools: Option<Vec<String>>,
    pub body: String,                    // Markdown 指令正文（已替换模板变量）
    pub base_dir: PathBuf,               // SKILL.md 所在目录
    pub source_path: PathBuf,            // SKILL.md 完整路径
}
```

### 2.4 SkillLoader

```rust
// crates/octo-engine/src/skills/loader.rs

pub struct SkillLoader {
    search_dirs: Vec<PathBuf>,           // 搜索目录列表
}

impl SkillLoader {
    pub fn new(project_dir: Option<&Path>, user_dir: Option<&Path>) -> Self;

    /// 扫描所有目录，解析 SKILL.md 文件
    pub fn load_all(&self) -> Result<Vec<SkillDefinition>>;

    /// 解析单个 SKILL.md 文件
    pub fn parse_skill(path: &Path) -> Result<SkillDefinition>;
}
```

**解析流程**:
1. 读取文件内容
2. 分离 YAML frontmatter（`---` 分隔符）和 Markdown body
3. 解析 YAML 提取字段（serde_yaml）
4. 验证必需字段（name, description）
5. 替换模板变量 `${baseDir}` → 文件所在目录绝对路径
6. 构造 `SkillDefinition`

### 2.5 SkillRegistry

```rust
// crates/octo-engine/src/skills/registry.rs

pub struct SkillRegistry {
    skills: HashMap<String, SkillDefinition>,
    watcher: Option<notify::RecommendedWatcher>,  // 文件系统监视器
}

impl SkillRegistry {
    pub fn new() -> Self;

    /// 从 SkillLoader 加载并注册所有 Skills
    pub fn load_from(&mut self, loader: &SkillLoader) -> Result<()>;

    /// 获取所有 Skill 描述（用于系统提示注入）
    pub fn prompt_section(&self) -> String;

    /// 获取所有 user-invocable Skills（用于注册为工具）
    pub fn invocable_skills(&self) -> Vec<&SkillDefinition>;

    /// 按名称查找
    pub fn get(&self, name: &str) -> Option<&SkillDefinition>;

    /// 启动文件监视，变更时自动重新加载
    pub fn start_watching(&mut self, loader: SkillLoader) -> Result<()>;

    /// 重新加载所有 Skills
    pub fn reload(&mut self, loader: &SkillLoader) -> Result<()>;
}
```

**系统提示注入**: `prompt_section()` 生成如下格式:

```xml
<available_skills>
## planning-with-files (v2.1.2)
Implements Manus-style file-based planning for complex tasks...
Use: /planning-with-files

## code-review (v1.0.0)
Performs thorough code review of recent changes...
Use: /code-review
</available_skills>
```

### 2.6 SkillTool（user-invocable 工具封装）

```rust
// crates/octo-engine/src/skills/tool.rs

/// 将 user-invocable Skill 包装为可调用的 Tool
pub struct SkillTool {
    skill: SkillDefinition,
}

impl Tool for SkillTool {
    fn name(&self) -> &str;       // 返回 skill.name
    fn description(&self) -> &str; // 返回 skill.description
    fn parameters(&self) -> Value; // 无参数或可选 args 参数

    async fn execute(&self, _args: Value, _ctx: ToolContext) -> Result<ToolResult> {
        // 返回 skill.body 作为指令文本
        // Agent 收到后将按照指令执行
        Ok(ToolResult::success(self.skill.body.clone()))
    }
}
```

### 2.7 热重载

使用 `notify` crate 监视 skills 目录:
- 文件创建/修改/删除时触发 `reload()`
- debounce 300ms（防止编辑器多次保存触发）
- 重载后更新 SkillRegistry + ToolRegistry
- 通过日志通知（无需 WebSocket 事件，Phase 2 足够）

### 2.8 与 SystemPromptBuilder 集成

修改 `crates/octo-engine/src/context/builder.rs`:
- `SystemPromptBuilder::new()` 接受 `&SkillRegistry`
- `build()` 时调用 `skill_registry.prompt_section()` 插入系统提示

### 2.9 新增文件

| 文件 | 说明 |
|------|------|
| `crates/octo-types/src/skill.rs` | SkillDefinition 类型 |
| `crates/octo-engine/src/skills/mod.rs` | 模块入口 |
| `crates/octo-engine/src/skills/loader.rs` | SkillLoader（SKILL.md 解析） |
| `crates/octo-engine/src/skills/registry.rs` | SkillRegistry + 热重载 |
| `crates/octo-engine/src/skills/tool.rs` | SkillTool（工具封装） |

### 2.10 新增依赖

| Crate | 用途 |
|-------|------|
| `serde_yaml` | YAML frontmatter 解析 |
| `notify` | 文件系统监视（热重载） |

---

## 3. MCP Client

### 3.1 架构

```
.octo/mcp.json → McpManager → StdioMcpClient(rmcp) → MCP Server 进程
                       ↓
                  McpToolBridge → ToolRegistry
```

rmcp 处理 MCP 协议细节（JSON-RPC 2.0、stdio transport），我们在上面加一层薄封装。

### 3.2 McpClient Trait

```rust
// crates/octo-engine/src/mcp/traits.rs

#[async_trait]
pub trait McpClient: Send + Sync {
    /// 服务器名称
    fn name(&self) -> &str;

    /// 连接到 MCP Server（启动子进程 + 初始化握手）
    async fn connect(&mut self) -> Result<()>;

    /// 列出 Server 提供的工具
    async fn list_tools(&self) -> Result<Vec<McpToolInfo>>;

    /// 调用 Server 上的工具
    async fn call_tool(&self, name: &str, args: serde_json::Value)
        -> Result<serde_json::Value>;

    /// 检查连接状态
    fn is_connected(&self) -> bool;

    /// 优雅关闭
    async fn shutdown(&mut self) -> Result<()>;
}

/// MCP 工具信息（从 tools/list 获取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,  // JSON Schema
}
```

### 3.3 StdioMcpClient（rmcp 封装）

```rust
// crates/octo-engine/src/mcp/stdio.rs

use rmcp::{ServiceExt, transport::TokioChildProcess};

pub struct StdioMcpClient {
    config: McpServerConfig,
    // rmcp client handle（connect 后填充）
    client: Option<rmcp::service::RunningService<rmcp::RoleClient, ()>>,
}

impl StdioMcpClient {
    pub fn new(config: McpServerConfig) -> Self;
}

#[async_trait]
impl McpClient for StdioMcpClient {
    async fn connect(&mut self) -> Result<()> {
        // 1. 用 TokioChildProcess 启动子进程
        // 2. 通过 rmcp ServiceExt::serve() 建立连接
        // 3. initialize 握手
        // 4. 存储 client handle
    }

    async fn list_tools(&self) -> Result<Vec<McpToolInfo>> {
        // client.list_tools() → 转换为我们的 McpToolInfo
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        // client.call_tool(name, args) → 转换结果
    }

    async fn shutdown(&mut self) -> Result<()> {
        // client.cancel() → 等待子进程退出
    }
}
```

### 3.4 McpServerConfig

```rust
// crates/octo-engine/src/mcp/traits.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,                      // 服务器名称
    pub command: String,                   // 可执行文件 (npx/uvx/node/python)
    pub args: Vec<String>,                 // 命令行参数
    #[serde(default)]
    pub env: HashMap<String, String>,      // 额外环境变量
}
```

### 3.5 McpToolBridge

```rust
// crates/octo-engine/src/mcp/bridge.rs

/// 将 MCP Server 的工具桥接为本地 Tool
pub struct McpToolBridge {
    client: Arc<tokio::sync::RwLock<dyn McpClient>>,
    server_name: String,
    tool_info: McpToolInfo,
}

impl Tool for McpToolBridge {
    fn name(&self) -> &str {
        &self.tool_info.name
    }

    fn description(&self) -> &str {
        self.tool_info.description.as_deref().unwrap_or("")
    }

    fn parameters(&self) -> Value {
        self.tool_info.input_schema.clone()
    }

    fn source(&self) -> ToolSource {
        ToolSource::Mcp(self.server_name.clone())
    }

    async fn execute(&self, args: Value, _ctx: ToolContext) -> Result<ToolResult> {
        let client = self.client.read().await;
        let result = client.call_tool(&self.tool_info.name, args).await?;
        Ok(ToolResult::success(result.to_string()))
    }
}
```

### 3.6 McpManager

```rust
// crates/octo-engine/src/mcp/manager.rs

pub struct McpManager {
    clients: HashMap<String, Arc<tokio::sync::RwLock<Box<dyn McpClient>>>>,
}

impl McpManager {
    pub fn new() -> Self;

    /// 从配置文件加载所有 MCP Server
    pub async fn load_config(config_path: &Path) -> Result<Vec<McpServerConfig>>;

    /// 添加并连接一个 MCP Server
    pub async fn add_server(&mut self, config: McpServerConfig) -> Result<Vec<McpToolInfo>>;

    /// 移除并关闭一个 MCP Server
    pub async fn remove_server(&mut self, name: &str) -> Result<()>;

    /// 将所有 MCP 工具桥接到 ToolRegistry
    pub async fn bridge_tools(&self, registry: &mut ToolRegistry) -> Result<()>;

    /// 关闭所有 Server
    pub async fn shutdown_all(&mut self) -> Result<()>;
}
```

### 3.7 配置文件格式

```json
// .octo/mcp.json
{
  "servers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
      "env": {}
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "ghp_xxx"
      }
    }
  }
}
```

### 3.8 新增文件

| 文件 | 说明 |
|------|------|
| `crates/octo-engine/src/mcp/mod.rs` | 模块入口 |
| `crates/octo-engine/src/mcp/traits.rs` | McpClient trait + McpServerConfig + McpToolInfo |
| `crates/octo-engine/src/mcp/stdio.rs` | StdioMcpClient（rmcp 封装） |
| `crates/octo-engine/src/mcp/bridge.rs` | McpToolBridge（Tool 实现） |
| `crates/octo-engine/src/mcp/manager.rs` | McpManager |

### 3.9 新增依赖

| Crate | Features | 用途 |
|-------|----------|------|
| `rmcp` | `client`, `transport-child-process` | MCP 协议客户端 |

---

## 4. Tool Execution 记录

### 4.1 数据模型

```rust
// crates/octo-types/src/execution.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: String,                    // ULID
    pub session_id: String,
    pub tool_name: String,
    pub source: ToolSource,            // BuiltIn / Mcp(server_name) / Skill(skill_name)
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: ExecutionStatus,
    pub started_at: i64,               // Unix 毫秒
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Running,
    Success,
    Failed,
    Timeout,
}
```

### 4.2 SQLite Schema

在现有 `migrations.rs` 中新增迁移:

```sql
CREATE TABLE IF NOT EXISTS tool_executions (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    source TEXT NOT NULL,
    input TEXT NOT NULL,
    output TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    started_at INTEGER NOT NULL,
    duration_ms INTEGER,
    error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tool_executions_session
    ON tool_executions(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_executions_tool
    ON tool_executions(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_executions_started
    ON tool_executions(started_at DESC);
```

### 4.3 ToolExecutionRecorder

```rust
// crates/octo-engine/src/tools/recorder.rs

pub struct ToolExecutionRecorder {
    db: Database,
}

impl ToolExecutionRecorder {
    pub fn new(db: Database) -> Self;

    /// 记录工具调用开始
    pub async fn record_start(
        &self, session_id: &str, tool_name: &str, source: &ToolSource,
        input: &Value
    ) -> Result<String>;  // 返回 execution_id

    /// 记录工具调用完成
    pub async fn record_complete(
        &self, id: &str, output: &Value, duration_ms: u64
    ) -> Result<()>;

    /// 记录工具调用失败
    pub async fn record_failed(
        &self, id: &str, error: &str, duration_ms: u64
    ) -> Result<()>;

    /// 查询 Session 的执行记录
    pub async fn list_by_session(
        &self, session_id: &str, limit: usize, offset: usize
    ) -> Result<Vec<ToolExecution>>;

    /// 查询单条执行记录
    pub async fn get(&self, id: &str) -> Result<Option<ToolExecution>>;
}
```

### 4.4 AgentLoop 集成

修改 `crates/octo-engine/src/agent/loop_.rs`:
- 添加 `recorder: Option<ToolExecutionRecorder>` 字段
- 在工具调用前调用 `recorder.record_start()`
- 在工具调用后调用 `recorder.record_complete()` 或 `record_failed()`
- 发出 `AgentEvent::ToolExecution(ToolExecution)` 事件

### 4.5 WebSocket 新事件

```rust
// 服务端 → 客户端
ServerMessage::ToolExecution {
    execution: ToolExecution,  // 完整执行记录
}

ServerMessage::TokenBudgetUpdate {
    budget: TokenBudgetSnapshot,  // 当前预算快照
}
```

```rust
pub struct TokenBudgetSnapshot {
    pub total: usize,           // 总 Token 数
    pub system_prompt: usize,   // 区域 A
    pub dynamic_context: usize, // 区域 B
    pub history: usize,         // 区域 C
    pub free: usize,            // 剩余
    pub usage_percent: f32,     // 使用率
    pub degradation_level: u8,  // 0-3
}
```

### 4.6 新增文件

| 文件 | 说明 |
|------|------|
| `crates/octo-types/src/execution.rs` | ToolExecution + ExecutionStatus 类型 |
| `crates/octo-engine/src/tools/recorder.rs` | ToolExecutionRecorder |

---

## 5. REST API

### 5.1 端点列表

| Method | Path | 说明 | 响应 |
|--------|------|------|------|
| `GET` | `/api/sessions` | Session 列表 | `Vec<SessionSummary>` |
| `GET` | `/api/sessions/:id` | Session 详情 + 消息历史 | `SessionDetail` |
| `GET` | `/api/sessions/:id/executions` | Session 的工具执行列表 | `Vec<ToolExecution>` |
| `GET` | `/api/executions/:id` | 单条执行详情 | `ToolExecution` |
| `GET` | `/api/tools` | 已注册工具列表 | `Vec<ToolInfo>` |
| `GET` | `/api/memories?q=xxx` | 搜索持久记忆 | `Vec<MemoryEntry>` |
| `GET` | `/api/memories/working` | 当前 Working Memory 块 | `Vec<MemoryBlock>` |
| `GET` | `/api/budget` | Token 预算状态 | `TokenBudgetSnapshot` |

### 5.2 响应类型

```rust
// Session 列表项
pub struct SessionSummary {
    pub id: String,
    pub created_at: String,
    pub message_count: usize,
    pub last_active: Option<String>,
}

// Session 详情
pub struct SessionDetail {
    pub id: String,
    pub created_at: String,
    pub messages: Vec<ChatMessage>,
    pub execution_count: usize,
}

// 工具信息
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub source: ToolSource,
    pub parameters: Value,
}
```

### 5.3 分页

所有列表端点支持查询参数:
- `limit` — 每页数量（默认 50，最大 200）
- `offset` — 偏移量（默认 0）

### 5.4 新增文件

| 文件 | 说明 |
|------|------|
| `crates/octo-server/src/api/mod.rs` | API 模块入口 + 路由注册 |
| `crates/octo-server/src/api/sessions.rs` | Session 端点 |
| `crates/octo-server/src/api/executions.rs` | Execution 端点 |
| `crates/octo-server/src/api/tools.rs` | Tools 端点 |
| `crates/octo-server/src/api/memories.rs` | Memory 端点 |
| `crates/octo-server/src/api/budget.rs` | Budget 端点 |

### 5.5 路由注册

修改 `crates/octo-server/src/router.rs`:

```rust
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        // 新增 REST API
        .nest("/api", api::routes())
        .route("/ws", get(ws_handler))
        .layer(cors)
        .layer(trace)
        .with_state(state)
}
```

---

## 6. 前端 Debug UI

### 6.1 Tab 布局

从当前单一 Chat 视图扩展为三 Tab:

```
┌──────────────────────────────────────────┐
│  Chat  │  Tools  │  Debug               │
├──────────────────────────────────────────┤
│                                          │
│        (Active Tab Content)              │
│                                          │
└──────────────────────────────────────────┘
```

- **Chat** — 现有对话界面（不变）
- **Tools** — 工具执行列表 + 执行详情
- **Debug** — Token 预算仪表盘 + 系统信息

### 6.2 Tools Tab

**ExecutionList 组件**:

```
┌─────────────────────────────────────────────────────┐
│ Tool Executions                          ↻ Refresh  │
├─────────┬────────┬─────────┬──────────┬────────────┤
│ Tool    │ Source │ Status  │ Duration │ Time       │
├─────────┼────────┼─────────┼──────────┼────────────┤
│ bash    │ BuiltIn│ ✅ ok   │ 1.2s     │ 14:30:01   │
│ grep    │ BuiltIn│ ✅ ok   │ 0.3s     │ 14:30:05   │
│ fs_read │ MCP    │ ❌ fail │ 0.1s     │ 14:30:08   │
│ bash    │ BuiltIn│ ⏳ run  │ —        │ 14:30:10   │
└─────────┴────────┴─────────┴──────────┴────────────┘
```

**ExecutionDetail 组件**（点击行展开）:

```
┌─ bash (BuiltIn) ──────────────────────────┐
│ Status: ✅ Success  Duration: 1.2s        │
│                                            │
│ ▼ Input                                    │
│ ┌────────────────────────────────────────┐ │
│ │ { "command": "ls -la /tmp" }           │ │
│ └────────────────────────────────────────┘ │
│                                            │
│ ▼ Output                                   │
│ ┌────────────────────────────────────────┐ │
│ │ "total 48\ndrwxrwxrwt  12 root..."     │ │
│ └────────────────────────────────────────┘ │
└────────────────────────────────────────────┘
```

### 6.3 Debug Tab

**TokenBudgetBar 组件**:

```
Context Window Usage (78%)                    L1
┌──────────────────────────────────────────────┐
│ ██████ System ██████ Dynamic ██████ History ░░│
│  12K (12%)    │ 28K (28%)  │ 38K (38%) │Free │
└──────────────────────────────────────────────┘

System Prompt:  12,480 tokens (12%)
Dynamic Context: 28,200 tokens (28%)
Conversation:   38,500 tokens (38%)
Free:           20,820 tokens (22%)

Degradation Level: L1 (Soft Trim)
```

颜色方案:
- < 60%: 绿色
- 60-80%: 黄色
- 80-90%: 橙色
- \> 90%: 红色

### 6.4 状态管理

```typescript
// web/src/atoms/debug.ts
import { atom } from 'jotai'

export interface ToolExecution {
  id: string
  session_id: string
  tool_name: string
  source: string
  input: unknown
  output: unknown | null
  status: 'running' | 'success' | 'failed' | 'timeout'
  started_at: number
  duration_ms: number | null
  error: string | null
}

export interface TokenBudget {
  total: number
  system_prompt: number
  dynamic_context: number
  history: number
  free: number
  usage_percent: number
  degradation_level: number
}

export const executionsAtom = atom<ToolExecution[]>([])
export const tokenBudgetAtom = atom<TokenBudget | null>(null)
export const selectedExecutionAtom = atom<string | null>(null)
```

### 6.5 WebSocket 事件处理

扩展 `web/src/ws/events.ts`:

```typescript
case 'tool_execution':
  // 更新或添加 execution
  set(executionsAtom, prev => {
    const idx = prev.findIndex(e => e.id === data.execution.id)
    if (idx >= 0) {
      const next = [...prev]
      next[idx] = data.execution
      return next
    }
    return [...prev, data.execution]
  })
  break

case 'token_budget_update':
  set(tokenBudgetAtom, data.budget)
  break
```

### 6.6 新增前端文件

| 文件 | 说明 |
|------|------|
| `web/src/atoms/debug.ts` | 执行和预算 atoms |
| `web/src/pages/Tools.tsx` | Tools 页面 |
| `web/src/pages/Debug.tsx` | Debug 页面 |
| `web/src/components/tools/ExecutionList.tsx` | 执行列表表格 |
| `web/src/components/tools/ExecutionDetail.tsx` | 执行详情展开面板 |
| `web/src/components/debug/TokenBudgetBar.tsx` | Token 预算可视化 |

---

## 7. 修改现有文件清单

| 文件 | 变更 |
|------|------|
| `Cargo.toml` | 添加 serde_yaml, notify, rmcp workspace 依赖 |
| `crates/octo-types/Cargo.toml` | 添加 serde_yaml |
| `crates/octo-types/src/lib.rs` | 添加 skill, execution 模块 re-exports |
| `crates/octo-engine/Cargo.toml` | 添加 serde_yaml, notify, rmcp |
| `crates/octo-engine/src/lib.rs` | 添加 skills, mcp 模块 |
| `crates/octo-engine/src/tools/mod.rs` | SkillTool 注册 |
| `crates/octo-engine/src/tools/traits.rs` | Tool trait 添加 source() 方法 |
| `crates/octo-engine/src/context/builder.rs` | 集成 SkillRegistry 提示注入 |
| `crates/octo-engine/src/agent/loop_.rs` | ToolExecutionRecorder + TokenBudgetSnapshot 事件 |
| `crates/octo-engine/src/db/migrations.rs` | 添加 tool_executions 表迁移 |
| `crates/octo-server/Cargo.toml` | 如需额外依赖 |
| `crates/octo-server/src/main.rs` | SkillLoader + McpManager 初始化 |
| `crates/octo-server/src/router.rs` | 添加 REST API 路由 |
| `crates/octo-server/src/state.rs` | 添加 SkillRegistry + McpManager + Recorder |
| `crates/octo-server/src/ws.rs` | 新增 tool_execution + token_budget_update 事件 |
| `web/src/ws/types.ts` | 新增事件类型定义 |
| `web/src/ws/events.ts` | 新增事件处理 |
| `web/src/atoms/ui.ts` | 扩展 Tab 定义 |
| `web/src/components/layout/TabBar.tsx` | 3 Tab 支持 |
| `web/src/components/layout/AppLayout.tsx` | Tab 路由 |

---

## 8. 任务拆分（初步）

| # | 任务 | 依赖 | 预估新文件 |
|---|------|------|-----------|
| 1 | SkillDefinition 类型 + SKILL.md 解析器 | 无 | 2 |
| 2 | SkillRegistry + SkillTool + SystemPromptBuilder 集成 | 1 | 3 |
| 3 | Skill 热重载（notify watcher） | 2 | 0（修改 registry.rs） |
| 4 | McpClient trait + McpServerConfig 类型 | 无 | 2 |
| 5 | StdioMcpClient（rmcp 封装） | 4 | 1 |
| 6 | McpToolBridge + McpManager | 4, 5 | 2 |
| 7 | ToolExecution 类型 + SQLite schema | 无 | 2 |
| 8 | ToolExecutionRecorder + AgentLoop 集成 | 7 | 1 |
| 9 | TokenBudgetSnapshot + WebSocket 事件 | 8 | 0（修改现有） |
| 10 | REST API 端点 (sessions + executions) | 7, 8 | 3 |
| 11 | REST API 端点 (tools + memories + budget) | 10 | 3 |
| 12 | 前端 atoms + WebSocket 事件处理 | 9 | 2 |
| 13 | Tools Tab (ExecutionList + ExecutionDetail) | 12 | 3 |
| 14 | Debug Tab (TokenBudgetBar) | 12 | 2 |
| 15 | AppState + main.rs 集成 + 全量编译验证 | 1-14 | 0 |

**三条并行开发路径**:
- **Skill 链**: 1 → 2 → 3
- **MCP 链**: 4 → 5 → 6
- **Debug 链**: 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14
- **集成**: 15（所有链完成后）

---

## 9. 关键技术决策

| 决策 | 选择 | 原因 |
|------|------|------|
| Skill 集成模型 | 混合：系统提示注入 + user-invocable 工具 | 与 Claude Code 模型一致 |
| MCP 实现 | rmcp 薄封装 | 官方 SDK 处理协议细节，我们控制接口 |
| YAML 解析 | serde_yaml | Rust 生态标准选择 |
| 文件监视 | notify crate | 跨平台，成熟稳定 |
| REST 框架 | Axum（已有） | 复用现有 web 框架 |
| 前端 Tab | 3 Tab (Chat/Tools/Debug) | 关注点分离 |
