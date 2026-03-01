# octo-engine 增强方案 (Brainstorming)

> 日期: 2026-03-01
> 对标: openfang, pi_agent_rust

---

## 1. 现状分析

### 1.1 octo-engine 当前能力

| 特性 | 当前状态 | 评分 |
|------|----------|------|
| 流式响应 | ✅ 支持 | ⭐⭐⭐⭐ |
| 循环检测 | 基础版 (重复+乒乓+断路器) | ⭐⭐⭐ |
| 上下文管理 | 4级降级 | ⭐⭐⭐⭐ |
| 并发工具 | 串行 (1个) | ⭐⭐ |
| 消息队列 | 无 | ⭐ |
| Extension 系统 | 无 | ⭐ |
| 向量记忆 | 无 | ⭐ |
| 安全策略 | 无 | ⭐ |

### 1.2 差距分析

```
octo-engine vs openfang/pi_agent_rust:

缺失的关键特性:
1. LoopGuard 增强版 (结果感知、轮询处理、警告机制)
2. 并发工具执行 (8个)
3. 向量记忆系统
4. Extension 插件系统
5. 消息队列 (Steering/FollowUp)
6. 安全策略
7. 可配置的最大迭代次数
```

---

## 2. 增强方案

### 2.1 优先级 P0: LoopGuard 增强 (来自 openfang)

**目标**: 增强循环检测能力，避免 agent 陷入重复调用

**当前代码位置**: `crates/octo-engine/src/agent/loop_guard.rs`

**新增特性**:

| 特性 | 描述 | 来自 |
|------|------|------|
| 结果感知检测 | 跟踪 (call_hash + result_hash) 对 | openfang |
| 乒乓检测增强 | 支持 A-B-A-B 和 A-B-C-A-B-C 模式 | openfang |
| 轮询工具处理 | shell_exec 等享受 3x 阈值 | openfang |
| 警告升级机制 | 超过 max_warnings 后升级为阻止 | openfang |
| 统计快照 | 暴露 LoopGuardStats 供调试 | openfang |

**实现方案**:

```rust
// 新增配置
pub struct LoopGuardConfig {
    pub warn_threshold: u32,           // 默认 3
    pub block_threshold: u32,          // 默认 5
    pub global_circuit_breaker: u32,  // 默认 30
    pub poll_multiplier: u32,         // 默认 3 (轮询工具)
    pub outcome_warn_threshold: u32,   // 默认 2
    pub outcome_block_threshold: u32, // 默认 3
    pub ping_pong_min_repeats: u32,  // 默认 3
    pub max_warnings_per_call: u32,  // 默认 3
}

// 新增数据结构
pub struct LoopGuard {
    call_counts: HashMap<String, u32>,
    outcome_counts: HashMap<String, u32>,  // 新增: 结果感知
    blocked_outcomes: HashSet<String>,     // 新增: 结果阻塞
    recent_calls: Vec<String>,             // 环形缓冲区 (30)
    warnings_emitted: HashMap<String, u32>, // 新增: 警告桶
    poll_counts: HashMap<String, u32>,    // 新增: 轮询计数
    blocked_calls: u32,
    hash_to_tool: HashMap<String, String>, // 新增: 统计
}

// 新增返回值
pub enum LoopGuardVerdict {
    Allow,
    Warn(String),      // 新增: 警告但允许
    Block(String),     // 阻止执行
    CircuitBreak(String),
}
```

---

### 2.2 优先级 P0: 向量记忆系统 (来自 openfang/pi_agent_rust)

**目标**: 支持语义搜索，召回相关历史记忆

**当前状态**: 只有 XML 编译的工作记忆

**新增组件**:

```
crates/octo-engine/src/memory/
    ├── vector.rs          # 新增: 向量存储
    ├── embeddings.rs      # 新增: 嵌入生成
    └── semantic.rs        # 新增: 语义搜索
```

**实现方案**:

```rust
// 新增 trait
#[async_trait]
pub trait SemanticMemory: Send + Sync {
    async fn store(&self, key: &str, content: &str, metadata: JsonValue) -> Result<()>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>>;
    async fn delete(&self, key: &str) -> Result<()>;
}

// 新增结构
pub struct SqliteVectorMemory {
    conn: Connection,
    embedding_model: String,
}

impl SemanticMemory for SqliteVectorMemory {
    async fn store(&self, key: &str, content: &str, metadata: JsonValue) -> Result<()> {
        // 1. 生成嵌入向量
        let embedding = self.generate_embedding(content).await?;
        // 2. 存储到向量数据库 (SQLite + ANN)
        self.upsert(key, content, embedding, metadata).await
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        // 1. 生成查询嵌入
        let query_embedding = self.generate_embedding(query).await?;
        // 2. ANN 搜索
        self.ann_search(query_embedding, limit).await
    }
}
```

**依赖**:
- `sqlite-vss` 或 `vectordb` crate
- 嵌入模型: OpenAI ada-002 / Ollama

---

### 2.3 优先级 P1: 并发工具执行 (来自 pi_agent_rust)

**目标**: 支持最多 8 个工具并发执行

**当前状态**: 串行执行

**修改位置**: `crates/octo-engine/src/agent/loop_.rs`

**实现方案**:

```rust
const MAX_CONCURRENT_TOOLS: usize = 8;

async fn execute_tools_concurrent(
    tool_calls: Vec<ToolCall>,
    tools: &ToolRegistry,
    ctx: &ToolContext,
) -> Vec<(ToolCall, ToolResult)> {
    // 并发执行
    let futures = tool_calls.into_iter().map(|call| {
        async move {
            let result = tools.execute(&call.name, call.input, ctx).await;
            (call, result)
        }
    });

    // 使用 futures::stream::StreamExt::buffer_unordered
    futures::stream::iter(futures)
        .buffer_unordered(MAX_CONCURRENT_TOOLS)
        .collect()
        .await
}
```

---

### 2.4 优先级 P1: 可配置最大迭代次数

**当前**: 硬编码 30

**修改**:

```rust
// AgentLoop 配置新增
pub struct AgentConfig {
    pub max_iterations: usize,  // 默认 50, 可配置
    pub max_concurrent_tools: usize, // 默认 1
    pub enable_vector_memory: bool,   // 默认 false
    // ...
}

// 从配置或 manifest 读取
let max_iterations = config.max_iterations.unwrap_or(50);
```

---

### 2.5 优先级 P2: 消息队列系统 (来自 pi_agent_rust)

**目标**: 支持 Steering/FollowUp 消息队列

**新增组件**:

```rust
// crates/octo-engine/src/agent/queue.rs

pub enum QueueKind {
    Steering,   // 引导消息 (优先级高)
    FollowUp,   // 跟进消息
}

pub struct MessageQueue {
    steering: VecDeque<Message>,
    follow_up: VecDeque<Message>,
    mode: QueueMode,
}

pub enum QueueMode {
    All,        // 一次处理所有
    OneAtATime, // 一次处理一个
}

// 在 AgentLoop 中使用
while has_more_tool_calls || !pending_messages.is_empty() {
    // 处理队列消息
    let pending = self.drain_queue().await;
    for msg in pending {
        self.messages.push(msg);
    }
    // 继续主循环
}
```

---

### 2.6 优先级 P2: 安全策略 (来自 zeroclaw)

**目标**: 添加安全策略，防止危险操作

**新增组件**:

```rust
// crates/octo-engine/src/security/mod.rs

pub struct SecurityPolicy {
    allowed_commands: Vec<String>,
    blocked_paths: Vec<PathBuf>,
    max_file_size: u64,
    require_confirmation: Vec<String>,
}

impl SecurityPolicy {
    pub fn from_config(config: &SecurityConfig) -> Self {
        // 从配置文件加载
    }

    pub fn check_command(&self, cmd: &str) -> CheckResult {
        // 检查命令是否允许
    }

    pub fn check_path(&self, path: &Path) -> CheckResult {
        // 检查路径是否安全
    }
}

// Tool 执行前检查
fn execute_tool_safely(tool: &dyn Tool, params: JsonValue, policy: &SecurityPolicy) -> Result<ToolResult> {
    // 1. 检查命令白名单/黑名单
    // 2. 检查路径访问权限
    // 3. 检查文件大小限制
}
```

---

### 2.7 优先级 P3: Extension 插件系统 (来自 pi_agent_rust)

**目标**: 支持运行时扩展

**设计**:

```
crates/octo-engine/src/extension/
    ├── mod.rs
    ├── loader.rs      # 动态加载
    ├── registry.rs   # 插件注册表
    ├── context.rs    # 扩展上下文
    └── hooks.rs      # 钩子点
```

**Hook 点**:

| 钩子 | 时机 |
|------|------|
| `before_agent_start` | Agent 启动前 |
| `after_agent_end` | Agent 完成后 |
| `before_tool_call` | 工具调用前 |
| `after_tool_call` | 工具调用后 |
| `before_compaction` | 上下文压缩前 |
| `after_compaction` | 上下文压缩后 |

---

## 3. 实施路线图

### Phase 1: LoopGuard 增强 (1周)

- [ ] 增强 LoopGuard 配置
- [ ] 添加结果感知检测
- [ ] 添加警告机制
- [ ] 添加统计快照

### Phase 2: 向量记忆 (2周)

- [ ] 设计 SemanticMemory trait
- [ ] 实现 SqliteVectorMemory
- [ ] 集成嵌入模型
- [ ] 记忆召回集成到 Agent Loop

### Phase 3: 并发工具 (1周)

- [ ] 修改工具执行逻辑
- [ ] 添加并发控制
- [ ] 测试并发安全性

### Phase 4: 消息队列 (2周)

- [ ] 实现 MessageQueue
- [ ] 集成到 Agent Loop
- [ ] 支持 Steering/FollowUp

### Phase 5: 安全策略 (1周)

- [ ] 设计 SecurityPolicy
- [ ] 命令/路径检查
- [ ] 集成到工具执行

### Phase 6: Extension 系统 (3周)

- [ ] 设计 Extension 架构
- [ ] 实现 Loader
- [ ] 定义 Hook 接口
- [ ] 集成到 Agent Loop

---

## 4. 依赖更新

```toml
# Cargo.toml 新增依赖

[dependencies]
# 向量搜索
vectordb = "0.2"
sqlite-vss = "0.2"

# 并发
futures-util = "0.3"

# 嵌入模型
reqwest = { version = "0.11", features = "json" }

# 插件加载
libloading = "0.8"

# 安全
regex = "1"
```

---

## 5. 总结

| 优先级 | 特性 | 工作量 | 收益 |
|--------|------|--------|------|
| P0 | LoopGuard 增强 | 1周 | 高 |
| P0 | 向量记忆 | 2周 | 高 |
| P1 | 并发工具 | 1周 | 中 |
| P1 | 可配置迭代 | 0.5周 | 中 |
| P2 | 消息队列 | 2周 | 中 |
| P2 | 安全策略 | 1周 | 高 |
| P3 | Extension | 3周 | 高 |

**推荐实施顺序**: P0 → P1 → P2 → P3

总工作量: **约 10 周**
