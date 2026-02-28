# octo-workbench v1.0 详细实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 完成 octo-workbench v1.0，包含阻塞问题修复、工具扩展、MCP/Skills 集成、记忆系统完善、安全加固，通过 33 个测试案例验证。

**Architecture:** 分阶段实施：阶段 A 修复阻塞问题 (WebSocket/MCP/Skills)，阶段 B 扩展工具和功能，阶段 C 安全加固和发布。每个任务包含测试代码和实现代码。

**Tech Stack:** Rust (octo-engine, octo-server), React + TypeScript (web), SQLite, MCP (Model Context Protocol), Skills (SKILL.md)

---

## Phase A: 阻塞问题修复 (Day 1-2)

### Task A1: WebSocket 连接修复

**Files:**
- Modify: `crates/octo-server/src/ws.rs` (路由配置)
- Test: 手动测试 WebSocket 连接
- Verify: `curl ws://localhost:3001/ws`

**Step 1: 检查 WebSocket 路由配置**

```bash
grep -n "ws" crates/octo-server/src/router.rs
```

**Step 2: 修复路由**

在 router.rs 添加:
```rust
let ws_route = Route::new()
    .at("/ws")
    .get(ws_handler);
```

**Step 3: 测试连接**

使用前端测试: 访问 http://localhost:5180 ，检查 Console 无 WebSocket 错误

**Step 4: Commit**

```bash
git add crates/octo-server/src/ws.rs crates/octo-server/src/router.rs
git commit -m "fix(ws): add WebSocket route at /ws endpoint"
```

---

### Task A2: MCP Server 启动修复

**Files:**
- Modify: `crates/octo-engine/src/mcp/manager.rs`
- Test: 启动 MCP filesystem server
- Verify: `curl http://localhost:3001/api/mcp/servers`

**Step 1: 检查 McpManager::start_server 实现**

```bash
grep -n "start_server" crates/octo-engine/src/mcp/manager.rs
```

**Step 2: 检查进程管理逻辑**

在 manager.rs 中添加调试日志:
```rust
tracing::info!("Starting MCP server: {:?}", config.name);
tracing::info!("Command: {:?} {:?}", config.command, config.args);
```

**Step 3: 测试启动**

1. 启动后端服务
2. 访问 MCP 页面点击 Start
3. 检查状态变为 "running"

**Step 4: Commit**

```bash
git add crates/octo-engine/src/mcp/manager.rs
git commit -m "fix(mcp): add debug logging for server startup"
```

---

### Task A3: Skills 配置和加载

**Files:**
- Create: `skills/code-debugger/SKILL.md`
- Create: `skills/git-helper/SKILL.md`
- Create: `skills/readme-writer/SKILL.md`
- Create: `skills/test-generator/SKILL.md`
- Create: `skills/code-review/SKILL.md`
- Create: `skills/file-organizer/SKILL.md`
- Modify: `config.yaml` (添加 skills.dirs)
- Test: 验证 Skills 加载

**Step 1: 创建 skills 目录结构**

```bash
mkdir -p skills/code-debugger skills/git-helper skills/readme-writer
mkdir -p skills/test-generator skills/code-review skills/file-organizer
```

**Step 2: 创建 code-debugger SKILL.md**

```markdown
---
name: code-debugger
description: 帮助调试代码问题，提供错误分析和修复建议
capabilities: [FileRead, ShellExec]
triggers: [debug, error, fix, bug, 调试, 错误]
---

# Code Debugger Skill

## 触发条件
用户请求调试代码、修复错误时触发。

## 执行步骤
1. 读取相关代码文件
2. 分析错误信息
3. 提供修复建议
4. 可选择执行修复命令
```

**Step 3: 创建其他 Skills**

按相同格式创建 git-helper, readme-writer, test-generator, code-review, file-organizer

**Step 4: 配置 config.yaml**

```yaml
skills:
  dirs:
    - ./skills
```

**Step 5: 验证加载**

```bash
curl http://localhost:3001/api/skills
# 预期返回 6 个 skills
```

**Step 6: Commit**

```bash
git add skills/ config.yaml
git commit -m "feat(skills): add 6 skills (debugger, git-helper, readme-writer, test-generator, code-review, file-organizer)"
```

---

## Phase B: 核心功能扩展 (Day 3-7)

### Task B1: 网络工具实现

**Files:**
- Create: `crates/octo-engine/src/tools/web_fetch.rs`
- Create: `crates/octo-engine/src/tools/web_search.rs`
- Modify: `crates/octo-engine/src/tools/mod.rs`
- Test: T11 (MCP fetch 验证)

**Step 1: 实现 web_fetch 工具**

```rust
// crates/octo-engine/src/tools/web_fetch.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WebFetchInput {
    pub url: String,
    pub max_length: Option<usize>,
}

pub async fn web_fetch(url: &str, max_length: Option<usize>) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(url).await?;
    let mut content = response.text().await?;

    if let Some(max) = max_length {
        if content.len() > max {
            content.truncate(max);
            content.push_str("\n... (truncated)");
        }
    }

    Ok(content)
}
```

**Step 2: 注册到 mod.rs**

```rust
pub mod web_fetch;
pub mod web_search;
```

**Step 3: 测试**

```bash
cargo test -p octo-engine web_fetch
```

**Step 4: Commit**

```bash
git add crates/octo-engine/src/tools/
git commit -m "feat(tools): add web_fetch and web_search tools"
```

---

### Task B2: MCP API Stub 补全

**Files:**
- Modify: `crates/octo-server/src/api/mcp_tools.rs`
- Modify: `crates/octo-server/src/api/mcp_logs.rs`
- Test: 验证 MCP API

**Step 1: 实现 list_tools**

```rust
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
) -> Json<Vec<McpToolInfo>> {
    let manager = state.mcp_manager();
    let Some(manager) = manager else {
        return Json(vec![]);
    };

    if let Some(tools) = manager.get_tool_infos(&server_id) {
        Json(tools)
    } else {
        Json(vec![])
    }
}
```

**Step 2: 实现 call_tool**

```rust
pub async fn call_tool(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Json(req): Json<McpToolCallRequest>,
) -> Json<McpToolCallResponse> {
    let manager = match state.mcp_manager() {
        Some(m) => m,
        None => return Json(McpToolCallResponse { error: Some("No manager".into()), .. }),
    };

    // 调用工具并返回结果
}
```

**Step 3: 测试**

```bash
curl http://localhost:3001/api/mcp/servers/test-server/tools
```

**Step 4: Commit**

```bash
git add crates/octo-server/src/api/
git commit -m "feat(api): implement MCP list_tools and call_tool endpoints"
```

---

### Task B3: Semantic Memory 基础实现

**Files:**
- Create: `crates/octo-engine/src/memory/semantic.rs`
- Modify: `crates/octo-engine/src/memory/mod.rs`
- Test: T19-T22 (Memory 测试)

**Step 1: 创建 Semantic Memory 结构**

```rust
// crates/octo-engine/src/memory/semantic.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEntity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub properties: serde_json::Value,
    pub relations: Vec<EntityRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelation {
    pub target_id: String,
    pub relation_type: String,
}

pub struct SemanticMemory {
    entities: Vec<SemanticEntity>,
}

impl SemanticMemory {
    pub fn new() -> Self {
        Self { entities: vec![] }
    }

    pub fn add_entity(&mut self, entity: SemanticEntity) {
        self.entities.push(entity);
    }

    pub fn search(&self, query: &str) -> Vec<&SemanticEntity> {
        self.entities.iter()
            .filter(|e| e.name.contains(query))
            .collect()
    }
}
```

**Step 2: 注册到 mod.rs**

```rust
pub mod semantic;
pub use semantic::SemanticMemory;
```

**Step 3: 测试**

```bash
cargo test -p octo-engine semantic
```

**Step 4: Commit**

```bash
git add crates/octo-engine/src/memory/
git commit -m "feat(memory): add SemanticMemory for entity storage"
```

---

## Phase C: 安全加固 (Day 8-9)

### Task C1: Rate Limiting 实现

**Files:**
- Create: `crates/octo-server/src/middleware/rate_limit.rs`
- Modify: `crates/octo-server/src/router.rs`
- Test: 超过限制返回 429

**Step 1: 实现 Rate Limiter**

```rust
// crates/octo-server/src/middleware/rate_limit.rs

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<std::time::Instant>>>>,
    max_requests: usize,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window_secs,
        }
    }

    pub async fn check(&self, key: &str) -> bool {
        let mut requests = self.requests.write().await;
        let now = std::time::Instant::now();

        let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);
        timestamps.retain(|t| now.duration_since(*t).as_secs() < self.window_secs);

        if timestamps.len() >= self.max_requests {
            return false;
        }

        timestamps.push(now);
        true
    }
}
```

**Step 2: 应用到路由**

```rust
// router.rs
let rate_limiter = RateLimiter::new(100, 60); // 100 requests per minute
```

**Step 3: 测试**

```bash
# 发送超过 100 个请求，应该返回 429
```

**Step 4: Commit**

```bash
git add crates/octo-server/src/middleware/
git commit -m "feat(middleware): add rate limiting (100 req/min)"
```

---

### Task C2: 30 轮对话支持

**Files:**
- Modify: `crates/octo-engine/src/agent/loop_.rs`
- Test: T32 (30 轮对话)

**Step 1: 修改 MAX_ROUNDS**

```rust
// loop_.rs
const MAX_ROUNDS: usize = 30; // 原来是 10
```

**Step 2: 测试**

连续发送 30 轮对话，验证不中断

**Step 3: Commit**

```bash
git add crates/octo-engine/src/agent/loop_.rs
git commit -m "feat(agent): increase MAX_ROUNDS to 30"
```

---

## Phase D: 测试验证 (Day 10-12)

### Task D1: 33 个测试案例执行

**Files:**
- Test: 全部测试案例

**Step 1: 基础功能测试 (T01-T09)**

| 测试 | 命令 | 预期 |
|------|------|------|
| T01 | 发送 "你好" | AI 响应 |
| T02 | 发送 "ls /tmp" | 文件列表 |
| T03 | 发送 "读取 README.md" | 文件内容 |

**Step 2: MCP 测试 (T10-T14)**

| 测试 | 验证 |
|------|------|
| T10 | filesystem MCP 启动并列出文件 |
| T11 | fetch MCP 获取网页 |
| T12 | sqlite MCP 执行查询 |
| T13 | github MCP 列出 issues |
| T14 | brave-search MCP 搜索 |

**Step 3: Skills 测试 (T15-T18)**

| 测试 | 验证 |
|------|------|
| T15 | 6 个 Skills 加载成功 |
| T16 | code-debugger 触发 |
| T17 | git-helper 触发 |
| T18 | readme-writer 生成文档 |

**Step 4: Memory 测试 (T19-T22)**

| 测试 | 命令 | 预期 |
|------|------|------|
| T19 | "记住我的名字是小明" | 存储成功 |
| T20 | "我叫什么？" | "小明" |
| T21 | "搜索项目" | 相关记忆 |
| T22 | "忘记那个名字" | 删除成功 |

**Step 5: 安全测试 (T23-T29)**

| 测试 | 验证 |
|------|------|
| T23 | Session 持久化 |
| T24 | LoopGuard 重复检测 |
| T25 | LoopGuard 乒乓检测 |
| T26 | Context 70% 阈值 |
| T27 | Context 90% 阈值 |
| T28 | LLM 重试 |
| T29 | LLM 不可重试错误 |

**Step 6: 调试测试 (T30-T33)**

| 测试 | 验证 |
|------|------|
| T30 | TokenBudget 显示 |
| T31 | ToolExecution 记录 |
| T32 | 30 轮对话 |
| T33 | WebSocket 重连 |

---

## 测试数据准备

### /tmp/octo-test/ 目录

```bash
mkdir -p /tmp/octo-test/data /tmp/octo-test/logs

# 创建测试文件
echo "# Test Project" > /tmp/octo-test/README.md
echo 'def add(a, b):' > /tmp/octo-test/hello.py
echo 'console.log("hello");' > /tmp/octo-test/hello.js
echo '{"name": "test", "version": "1.0"}' > /tmp/octo-test/data/config.json
echo "2026-01-01 INFO starting" > /tmp/octo-test/logs/app.log
```

---

## 实施顺序

```
Day 1: A1 (WebSocket) + A2 (MCP 启动)
Day 2: A3 (Skills 配置)
Day 3: B1 (网络工具)
Day 4: B2 (MCP API)
Day 5: B3 (Semantic Memory)
Day 6: C1 (Rate Limit)
Day 7: C2 (30 轮)
Day 8-9: D1 (测试验证)
Day 10-12: Bug 修复 + 发布
```

---

## 验收标准

| 类别 | 测试数 | 通过率 |
|------|--------|--------|
| 基础功能 | 9 | 100% |
| MCP | 5 | 100% |
| Skills | 4 | 100% |
| Memory | 4 | 100% |
| 安全 | 7 | 100% |
| 调试 | 4 | 100% |
| **总计** | **33** | **100%** |

---

**Plan complete and saved to `docs/plans/2026-03-01-octo-workbench-v1-0-tasks.md`.**

## 执行选择

**1. Subagent-Driven (this session)** -  dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - open new session with executing-plans, batch execution with checkpoints

Which approach?
