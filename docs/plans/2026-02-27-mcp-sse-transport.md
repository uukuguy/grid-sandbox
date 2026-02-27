# MCP SSE Transport 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标**：为 octo-engine MCP 客户端增加 Streamable HTTP（SSE）transport 支持，使 octo 能连接远程 MCP 服务器（如通过 URL 暴露的服务），同时保持与现有 Stdio transport 的完全兼容。

**架构**：采用方案 A——在 `McpServerConfigV2` 添加 `transport` 字段（`stdio` / `sse`），`McpManager::add_server_v2()` 根据该字段选择创建 `StdioMcpClient` 或 `SseMcpClient`。两者都实现相同的 `McpClient` trait，上层调用无需感知差异。同时完善 `mcp_servers.rs` REST API（当前全部为 TODO stub）。

**技术栈**：Rust async/tokio、`rmcp 0.16`（feature: `transport-streamable-http-client-reqwest`）、`reqwest 0.12`（已有依赖）、Axum 0.8

---

## 背景：当前代码状态

- `crates/octo-engine/src/mcp/traits.rs` — `McpClient` trait（transport-agnostic）、`McpServerConfigV2`（无 transport 字段）
- `crates/octo-engine/src/mcp/stdio.rs` — `StdioMcpClient`，用 `rmcp::transport::TokioChildProcess`
- `crates/octo-engine/src/mcp/manager.rs` — `add_server()` 硬编码创建 `StdioMcpClient`
- `crates/octo-server/src/api/mcp_servers.rs` — 所有 handlers 都是 TODO stub，不与 McpManager/McpStorage 交互

**注意**：`mcp_servers.rs` REST API 是 stub，本计划顺带实现 `list_servers` 和 `create_server` 两个最重要的端点。其他端点（update/delete/start/stop）保持 TODO，不在本计划范围内（YAGNI）。

---

## 文件索引

### 新增文件
| 文件 | 任务 | 说明 |
|------|------|------|
| `crates/octo-engine/src/mcp/sse.rs` | Task 2 | SseMcpClient 实现 |

### 修改文件
| 文件 | 任务 | 说明 |
|------|------|------|
| `Cargo.toml`（workspace） | Task 1 | 添加 rmcp SSE feature |
| `crates/octo-engine/Cargo.toml` | Task 1 | 添加 rmcp SSE feature |
| `crates/octo-engine/src/mcp/traits.rs` | Task 1 | `McpTransport` 枚举 + `McpServerConfigV2` 新增 transport 字段 |
| `crates/octo-engine/src/mcp/mod.rs` | Task 2 | 导出 `SseMcpClient` |
| `crates/octo-engine/src/mcp/manager.rs` | Task 3 | `add_server_v2()` 根据 transport 分发 |
| `crates/octo-server/src/api/mcp_servers.rs` | Task 4 | 实现 `list_servers` + `create_server`（去掉 TODO） |

---

## Task 1：添加 transport 字段 + 依赖

**目标**：在配置类型中建立 transport 枚举，并启用 rmcp SSE feature。

**文件**：
- 修改：`Cargo.toml`（workspace root）
- 修改：`crates/octo-engine/Cargo.toml`
- 修改：`crates/octo-engine/src/mcp/traits.rs`

### Step 1: 在 workspace Cargo.toml 更新 rmcp feature

打开 `Cargo.toml`（workspace root），找到：

```toml
rmcp = { version = "0.16", features = ["client", "transport-child-process"] }
```

替换为：

```toml
rmcp = { version = "0.16", features = ["client", "transport-child-process", "transport-streamable-http-client-reqwest"] }
```

### Step 2: 验证 feature 名称存在

```bash
cargo fetch 2>&1 | tail -5
```

预期：无错误（feature 名称正确则直接成功）。如果 feature 不存在会报 `Package 'rmcp' does not have feature 'transport-streamable-http-client-reqwest'`，此时改为 `transport-sse-client`（查 `cargo metadata` 确认）。

实际验证方式：

```bash
cargo metadata --no-deps --format-version 1 | python3 -c "
import json, sys
d = json.load(sys.stdin)
for p in d['packages']:
    if p['name'] == 'rmcp':
        print('Features:', list(p['features'].keys()))
"
```

### Step 3: 在 traits.rs 添加 McpTransport 枚举和更新 McpServerConfigV2

打开 `crates/octo-engine/src/mcp/traits.rs`，在文件顶部的 use 语句之后、`McpToolInfo` 之前，插入：

```rust
/// MCP 服务器传输方式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    /// 本地进程 stdin/stdout（默认）
    #[default]
    Stdio,
    /// Streamable HTTP / SSE（远程服务器）
    Sse,
}
```

然后在 `McpServerConfigV2` struct 末尾（`pub enabled: bool,` 之后）添加字段：

```rust
    #[serde(default)]
    pub transport: McpTransport,
    /// SSE transport 专用：服务器 URL（如 "http://localhost:8080/mcp"）
    #[serde(default)]
    pub url: Option<String>,
```

同时更新 `From<McpServerConfigV2> for McpServerConfig` 的 impl（无需变动，`McpServerConfig` 是 Stdio 专用，保持不变）。

### Step 4: 验证编译

```bash
cargo check -p octo-engine 2>&1 | grep "^error" | head -20
```

预期：0 errors（trait 和 enum 变化不影响现有代码，因为 `McpTransport` 有 `#[default]`）。

### Step 5: Commit

```bash
cd /Users/sujiangwen/sandbox/LLM/speechless.ai/Autonomous-Agents/octo-sandbox
git add Cargo.toml crates/octo-engine/Cargo.toml crates/octo-engine/src/mcp/traits.rs
git commit -m "feat(mcp): add McpTransport enum (stdio/sse) + transport/url fields to McpServerConfigV2"
```

---

## Task 2：实现 SseMcpClient

**目标**：创建 `SseMcpClient`，使用 `rmcp::transport::StreamableHttpClientTransport` 连接远程 MCP 服务器。

**文件**：
- 新增：`crates/octo-engine/src/mcp/sse.rs`
- 修改：`crates/octo-engine/src/mcp/mod.rs`

### Step 1: 创建 sse.rs

创建文件 `crates/octo-engine/src/mcp/sse.rs`，内容如下：

```rust
use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::info;

use rmcp::model::{CallToolRequestParams, RawContent};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{RoleClient, ServiceExt};

use super::traits::{McpClient, McpToolInfo};

/// MCP client using Streamable HTTP (SSE) transport.
/// Connects to a remote MCP server via HTTP URL.
pub struct SseMcpClient {
    name: String,
    url: String,
    service: Option<RunningService<RoleClient, ()>>,
}

impl SseMcpClient {
    pub fn new(name: String, url: String) -> Self {
        Self {
            name,
            url,
            service: None,
        }
    }
}

#[async_trait]
impl McpClient for SseMcpClient {
    fn name(&self) -> &str {
        &self.name
    }

    async fn connect(&mut self) -> Result<()> {
        info!(
            name = %self.name,
            url = %self.url,
            "Connecting to remote MCP server via SSE"
        );

        let transport = StreamableHttpClientTransport::from_uri(self.url.clone());

        let service = ()
            .serve(transport)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to MCP server at {}: {e}", self.url))?;

        let peer_info = service.peer_info();
        info!(
            name = %self.name,
            server = ?peer_info,
            "Remote MCP server connected"
        );

        self.service = Some(service);
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<McpToolInfo>> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SSE MCP client not connected"))?;

        let tools = service
            .list_all_tools()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list SSE MCP tools: {e}"))?;

        Ok(tools
            .into_iter()
            .map(|t| McpToolInfo {
                name: t.name.to_string(),
                description: t.description.map(|d| d.to_string()),
                input_schema: serde_json::Value::Object(t.input_schema.as_ref().clone()),
            })
            .collect())
    }

    async fn call_tool(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SSE MCP client not connected"))?;

        let arguments = if args.is_object() {
            Some(args.as_object().unwrap().clone())
        } else {
            None
        };

        let result = service
            .call_tool(CallToolRequestParams {
                meta: None,
                name: name.to_string().into(),
                arguments,
                task: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to call SSE MCP tool '{name}': {e}"))?;

        let content_strs: Vec<String> = result
            .content
            .into_iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(text) => Some(text.text.clone()),
                _ => None,
            })
            .collect();

        Ok(serde_json::json!({
            "content": content_strs.join("\n"),
            "isError": result.is_error.unwrap_or(false),
        }))
    }

    fn is_connected(&self) -> bool {
        self.service.is_some()
    }

    async fn shutdown(&mut self) -> Result<()> {
        if let Some(service) = self.service.take() {
            info!(name = %self.name, "Disconnecting remote MCP server");
            service
                .cancel()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to cancel SSE MCP service: {e}"))?;
        }
        Ok(())
    }
}
```

### Step 2: 在 mod.rs 导出 SseMcpClient

打开 `crates/octo-engine/src/mcp/mod.rs`，添加：

```rust
pub mod sse;
pub use sse::SseMcpClient;
```

（在现有 `pub mod stdio;` 和 `pub use stdio::StdioMcpClient;` 旁边）

### Step 3: 验证编译

```bash
cargo check -p octo-engine 2>&1 | grep "^error" | head -20
```

预期：0 errors。

如果报 `StreamableHttpClientTransport` 不存在，说明 feature 名称需要调整——检查 Task 1 Step 2 的 cargo metadata 输出，确认正确的 feature 名。

### Step 4: Commit

```bash
git add crates/octo-engine/src/mcp/sse.rs crates/octo-engine/src/mcp/mod.rs
git commit -m "feat(mcp): add SseMcpClient using StreamableHttpClientTransport"
```

---

## Task 3：McpManager 支持 transport 分发

**目标**：`McpManager` 新增 `add_server_v2()` 方法，根据 `McpTransport` 创建不同的 client 实现。

**文件**：
- 修改：`crates/octo-engine/src/mcp/manager.rs`

### Step 1: 在 manager.rs 添加 import

在 `manager.rs` 顶部，在现有 `use super::stdio::StdioMcpClient;` 后面添加：

```rust
use super::sse::SseMcpClient;
use super::traits::McpTransport;
```

### Step 2: 添加 add_server_v2() 方法

在 `McpManager` impl 块中，在 `add_server()` 方法之后，添加新方法：

```rust
/// Add and connect a new MCP server, supporting both Stdio and SSE transports.
pub async fn add_server_v2(&mut self, config: McpServerConfigV2) -> Result<Vec<McpToolInfo>> {
    let name = config.name.clone();

    let mut client: Box<dyn McpClient> = match config.transport {
        McpTransport::Stdio => {
            Box::new(StdioMcpClient::new(McpServerConfig {
                name: config.name.clone(),
                command: config.command.clone(),
                args: config.args.clone(),
                env: config.env.clone(),
            }))
        }
        McpTransport::Sse => {
            let url = config.url.clone().ok_or_else(|| {
                anyhow::anyhow!("SSE transport requires 'url' field for server '{name}'")
            })?;
            Box::new(SseMcpClient::new(config.name.clone(), url))
        }
    };

    client.connect().await?;
    let tools = client.list_tools().await?;

    info!(
        server = %name,
        transport = ?config.transport,
        tool_count = tools.len(),
        "MCP server connected with tools"
    );

    let client: Arc<RwLock<Box<dyn McpClient>>> = Arc::new(RwLock::new(client));
    self.clients.insert(name.clone(), client);
    self.tool_infos.insert(name, tools.clone());
    Ok(tools)
}
```

### Step 3: 验证编译

```bash
cargo check -p octo-engine 2>&1 | grep "^error" | head -20
```

预期：0 errors。

### Step 4: Commit

```bash
git add crates/octo-engine/src/mcp/manager.rs
git commit -m "feat(mcp): McpManager::add_server_v2() dispatches stdio/sse transport by config"
```

---

## Task 4：实现 REST API list_servers + create_server

**目标**：将 `mcp_servers.rs` 中最重要的两个 handler（list 和 create）从 TODO stub 改为真实实现，并把 `transport` 字段暴露给前端。

**文件**：
- 修改：`crates/octo-server/src/api/mcp_servers.rs`

**背景**：`AppState` 目前是否持有 `McpManager` 和 `McpStorage`，需要先确认。

### Step 1: 检查 AppState 结构

```bash
grep -n "McpManager\|McpStorage\|mcp" crates/octo-server/src/state.rs | head -20
grep -n "McpManager\|McpStorage\|mcp" crates/octo-server/src/main.rs | head -20
```

根据输出决定：
- 若 `AppState` 已有 `mcp_manager` 字段 → 直接使用
- 若没有 → 在 Step 2 中添加（见下方两种情况的代码）

### Step 2a: 若 AppState 已有 McpManager，更新 McpServerConfigRequest

在 `mcp_servers.rs` 中，在 `McpServerConfigRequest` 里添加 transport 字段：

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerConfigRequest {
    pub name: String,
    pub source: Option<String>,
    // Stdio transport fields
    pub command: Option<String>,     // 改为 Option（SSE 时可省略）
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    // SSE transport fields
    pub transport: Option<String>,   // "stdio" | "sse"，默认 "stdio"
    pub url: Option<String>,         // SSE 专用
    pub enabled: Option<bool>,
}
```

在 `McpServerResponse` 中添加 transport 字段：

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerResponse {
    pub id: String,
    pub name: String,
    pub source: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub transport: String,   // 新增
    pub url: Option<String>, // 新增
    pub enabled: bool,
    pub runtime_status: String,
    pub tool_count: usize,
    pub created_at: String,
    pub updated_at: String,
}
```

### Step 2b: 若 AppState 没有 McpManager，先跳过 manager 集成

只做 request/response 结构扩展（transport/url 字段），保留 TODO 逻辑（不连接实际 server），避免引入过多依赖。这样 API 结构对前端正确，后续再接 manager。

### Step 3: 实现 create_server handler（不连接 manager，仅返回正确结构）

找到当前的 `create_server` 函数，将其改为：

```rust
pub async fn create_server(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<McpServerConfigRequest>,
) -> Json<McpServerResponse> {
    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    let transport = req.transport.as_deref().unwrap_or("stdio").to_string();

    Json(McpServerResponse {
        id,
        name: req.name,
        source: req.source.unwrap_or_else(|| "manual".to_string()),
        command: req.command.unwrap_or_default(),
        args: req.args.unwrap_or_default(),
        env: req.env.unwrap_or_default(),
        transport: transport.clone(),
        url: req.url,
        enabled: req.enabled.unwrap_or(true),
        runtime_status: "stopped".to_string(),
        tool_count: 0,
        created_at: now.clone(),
        updated_at: now,
    })
}
```

### Step 4: 验证编译

```bash
cargo check --workspace 2>&1 | grep "^error" | head -20
```

预期：0 errors。

### Step 5: Commit

```bash
git add crates/octo-server/src/api/mcp_servers.rs
git commit -m "feat(api): add transport/url fields to MCP server API request/response"
```

---

## Task 5：全量构建验证

### Step 1: 完整编译检查

```bash
cargo check --workspace 2>&1 | tail -5
```

预期：0 errors，若干 warnings 可忽略。

### Step 2: TypeScript 类型检查

```bash
cd web && npx tsc --noEmit 2>&1 | tail -10 && cd ..
```

预期：0 errors。

### Step 3: 前端构建

```bash
cd web && npx vite build 2>&1 | tail -5 && cd ..
```

预期：dist/ 构建成功。

### Step 4: 更新文档

在 `docs/dev/NEXT_SESSION_GUIDE.md` 中，将 `MCP SSE Transport` 的状态从 `⏳ 待实施` 改为 `✅ 已实施`：

找到这行：
```
| MCP SSE Transport | P1 | ⏳ 待实施 |
```

改为：
```
| MCP SSE Transport ✅ | P1 | **已实施** |
```

在 `docs/dev/MEMORY_INDEX.md` 的 `[Active Work]` 部分追加：

```
- {当前时间} | MCP SSE Transport 完成: SseMcpClient + add_server_v2() + transport/url API 字段
```

### Step 5: Commit

```bash
git add docs/dev/NEXT_SESSION_GUIDE.md docs/dev/MEMORY_INDEX.md
git commit -m "docs: MCP SSE Transport complete - SseMcpClient + transport dispatch + API fields"
```

---

## 完成标准

| 检查项 | 验收标准 |
|--------|---------|
| 编译 | `cargo check --workspace` 0 errors |
| SseMcpClient | 实现 `McpClient` trait 所有方法，使用 `StreamableHttpClientTransport::from_uri()` |
| McpTransport 枚举 | `McpServerConfigV2.transport` 字段 serde 序列化为 `"stdio"` / `"sse"` |
| add_server_v2() | Stdio 路径创建 `StdioMcpClient`；SSE 路径创建 `SseMcpClient`；SSE 无 url 时返回明确错误 |
| REST API | `McpServerResponse` 含 `transport` + `url` 字段；`create_server` 能返回 SSE 类型 server |
| 前端兼容 | `npx tsc --noEmit` 0 errors（前端未修改，不应有新错误） |

---

## 提交历史预期

```
feat(mcp): add McpTransport enum (stdio/sse) + transport/url fields to McpServerConfigV2
feat(mcp): add SseMcpClient using StreamableHttpClientTransport
feat(mcp): McpManager::add_server_v2() dispatches stdio/sse transport by config
feat(api): add transport/url fields to MCP server API request/response
docs: MCP SSE Transport complete - SseMcpClient + transport dispatch + API fields
```
