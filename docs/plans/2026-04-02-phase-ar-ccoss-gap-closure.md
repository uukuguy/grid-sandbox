# Phase AR — CC-OSS 缺口补齐（7 deferred 解锁）

> 目标：解锁 7 个追赶 CC-OSS 的必选 deferred 项，补齐自动升级、会话抄本、会话分叉、Blob GC、语义搜索、Webhook/MQ 触发。
> 日期：2026-04-02
> 依据：Phase AP+AQ deferred 分析 + CC-OSS 源码对标
> 执行策略：3 Wave，每 Wave 独立 commit/验证

---

## 一、设计决策

| 决策点 | 选项 | 决定 | 理由 |
|--------|------|------|------|
| max_tokens 升级 | A:继续说 / **B:参数升级** | B | 省一轮 LLM 调用，CC-OSS 做法 |
| Transcript 格式 | A:纯文本 / **B:JSONL** | B | 结构化可查询，CC-OSS 兼容 |
| Blob GC 策略 | A:引用计数 / **B:TTL+容量** | B | 无需追踪引用，简单可靠 |
| Fork 实现 | **A:复用 ThreadStore** / B:全新 | A | fork_thread() 已实现 |
| MQ 触发 | A:硬绑 Redis / **B:Trait 抽象** | B | 可扩展，框架只提供 channel 实现 |
| 语义搜索 | A:独立 embedding / **B:复用 VectorIndex** | B | 已有基础设施 |

---

## 二、依赖图

```
T1 TokenEscalation ─────────────── 零依赖
T2 SessionTranscript ────────────── 零依赖
T3 BlobStore GC ─────────────────── 零依赖（依赖 BlobStore 已完成）
        │
T4 Session Fork/Rewind ─────────── 零依赖（ThreadStore 已存在）
        │
T5 Webhook 触发 ─────────────────── 依赖 start_autonomous (T5 自建)
T6 MessageQueue 触发 ────────────── 依赖 T5 的 TriggerSource trait
T7 tool_search 语义搜索 ─────────── 零依赖（VectorIndex 已存在）
```

---

## 三、Wave 执行顺序

```
时间 →
─────────────────────────────────────────────────
Wave 1  │ T1 + T2 + T3                │ 零依赖，可并行
────────┤                              ├────
Wave 2  │ T4                           │ 独立
────────┤                              ├────
Wave 3  │ T5 + T6 + T7                │ T5→T6 顺序，T7 可并行
─────────────────────────────────────────────────
```

---

## 四、任务详细清单

### Wave 1：基础设施增强（零依赖）

#### T1 — max_output_tokens 自动升级 (~50 行)

- **Deferred**: AP-D2
- **依赖**: 无
- **新增文件**: `crates/octo-engine/src/agent/token_escalation.rs` (~40 行)
- **修改文件**:
  - `agent/mod.rs` (模块声明)
  - `agent/harness.rs` (~10 行，max_tokens 升级分支)

**TokenEscalation 设计**:

```rust
/// 阶梯式 max_tokens 升级器。
/// 当 LLM 因 max_tokens 截断时，升级到下一档再重试，
/// 避免浪费一轮 ContinuationTracker 调用。
pub struct TokenEscalation {
    tiers: Vec<u32>,
    current_tier: usize,
}

impl TokenEscalation {
    pub fn new() -> Self {
        Self {
            tiers: vec![4096, 8192, 16384, 32768, 65536],
            current_tier: 0,
        }
    }

    /// 自定义 tier 列表
    pub fn with_tiers(tiers: Vec<u32>) -> Self {
        Self { tiers, current_tier: 0 }
    }

    /// 当前 max_tokens 值
    pub fn current(&self) -> u32 {
        self.tiers[self.current_tier]
    }

    /// 尝试升级到下一档。返回 Some(new_max) 或 None（已到顶）
    pub fn escalate(&mut self) -> Option<u32> {
        if self.current_tier + 1 < self.tiers.len() {
            self.current_tier += 1;
            Some(self.tiers[self.current_tier])
        } else {
            None
        }
    }

    /// 重置到初始档位（新 turn 开始时调用）
    pub fn reset(&mut self) {
        self.current_tier = 0;
    }
}
```

**Harness 集成逻辑** (在 `stop_reason == MaxTokens` 分支):

```rust
// 现有逻辑：ContinuationTracker 检测到 max_tokens 后注入"请继续"
// 新增：先尝试 TokenEscalation，成功则直接用更大的 max_tokens 重试
if stop_reason == StopReason::MaxTokens {
    if let Some(new_max) = token_escalation.escalate() {
        debug!(old = config.max_tokens, new = new_max, "TokenEscalation: upgrading max_tokens");
        config.max_tokens = new_max;
        // 不注入 continuation prompt，直接用更大的 buffer 重试
        messages.push(ChatMessage::assistant(&full_text));
        continue;
    }
    // 已到顶，fallback 到 ContinuationTracker
    if continuation_tracker.should_continue("max_tokens") { ... }
}
```

**测试** (~4 tests):
- escalate 升级链 4096→8192→...→65536
- escalate 到顶返回 None
- reset 重置
- current 返回当前值

---

#### T2 — Session Transcript 写入 (~80 行)

- **Deferred**: AP-D6
- **依赖**: 无
- **新增文件**: `crates/octo-engine/src/session/transcript.rs` (~60 行)
- **修改文件**:
  - `session/mod.rs` (模块声明)
  - `agent/loop_config.rs` (+transcript_writer 字段)
  - `agent/harness.rs` (~15 行，消息追加后写入)
  - `agent/executor.rs` (~5 行，创建 TranscriptWriter 并传入)

**TranscriptWriter 设计**:

```rust
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// JSONL transcript 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub content_preview: String,  // 前 500 字符
    pub blob_ref: Option<String>, // 大内容的 blob 引用
    pub tool_name: Option<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

/// 追加式 JSONL 抄本写入器
pub struct TranscriptWriter {
    file_path: PathBuf,
}

impl TranscriptWriter {
    pub fn new(session_dir: PathBuf, session_id: &str) -> Self {
        let file_path = session_dir.join(format!("{}.transcript.jsonl", session_id));
        Self { file_path }
    }

    /// 追加一条记录
    pub fn append(&self, entry: &TranscriptEntry) -> anyhow::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true).append(true).open(&self.file_path)?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// 读取完整 transcript
    pub fn read_all(&self) -> anyhow::Result<Vec<TranscriptEntry>> {
        let content = fs::read_to_string(&self.file_path)?;
        content.lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).map_err(Into::into))
            .collect()
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}
```

**Harness 集成**：每次 `messages.push()` 后追加 transcript entry。

**测试** (~4 tests):
- append + read_all 往返
- blob_ref 写入
- 空文件 read_all 返回空 vec
- 多条追加顺序

---

#### T3 — BlobStore GC (~100 行)

- **Deferred**: AQ-D2
- **依赖**: BlobStore (AQ-T3, 已完成)
- **新增文件**: `crates/octo-engine/src/storage/blob_gc.rs` (~80 行)
- **修改文件**:
  - `storage/mod.rs` (模块声明 + re-export)

**BlobGc 设计**:

```rust
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use anyhow::Result;
use tracing::{debug, info};

/// Blob garbage collector — TTL + 容量双重策略。
pub struct BlobGc {
    base_dir: PathBuf,
    max_age: Duration,
    max_total_bytes: u64,
}

impl BlobGc {
    pub fn new(base_dir: PathBuf, max_age: Duration, max_total_bytes: u64) -> Self {
        Self { base_dir, max_age, max_total_bytes }
    }

    /// 默认配置：7 天 TTL + 1GB 容量上限
    pub fn with_defaults(base_dir: PathBuf) -> Self {
        Self::new(base_dir, Duration::from_secs(7 * 86400), 1_073_741_824)
    }

    /// 执行 GC，返回 (deleted_count, freed_bytes)
    pub fn collect(&self) -> Result<(usize, u64)> {
        let mut deleted = 0;
        let mut freed = 0u64;
        let mut entries = Vec::new();

        // 1. 遍历两级目录，收集所有 blob 文件
        self.walk_blobs(&mut entries)?;

        // 2. TTL 清理
        let now = SystemTime::now();
        for entry in &entries {
            if let Ok(age) = now.duration_since(entry.modified) {
                if age > self.max_age {
                    if std::fs::remove_file(&entry.path).is_ok() {
                        deleted += 1;
                        freed += entry.size;
                    }
                }
            }
        }

        // 3. 容量清理（按 mtime 从旧到新删除，直到总量 < max）
        let remaining: Vec<_> = entries.iter()
            .filter(|e| e.path.exists())
            .collect();
        let total: u64 = remaining.iter().map(|e| e.size).sum();
        if total > self.max_total_bytes {
            let mut sorted = remaining;
            sorted.sort_by_key(|e| e.modified);
            let mut current_total = total;
            for entry in sorted {
                if current_total <= self.max_total_bytes { break; }
                if std::fs::remove_file(&entry.path).is_ok() {
                    deleted += 1;
                    freed += entry.size;
                    current_total -= entry.size;
                }
            }
        }

        info!(deleted, freed_bytes = freed, "BlobGc: collection complete");
        Ok((deleted, freed))
    }

    fn walk_blobs(&self, entries: &mut Vec<BlobEntry>) -> Result<()> { ... }
}

struct BlobEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}
```

**测试** (~4 tests):
- TTL 过期清理
- 容量上限清理
- 空目录不报错
- 混合清理

---

### Wave 2：会话管理增强

#### T4 — Session Fork/Rewind (~150 行)

- **Deferred**: AP-D7
- **依赖**: ThreadStore (已存在)
- **修改文件**:
  - `agent/executor.rs` (~40 行，+Rewind/Fork 消息处理)
  - `agent/harness.rs` (~20 行，+rewind_messages 辅助函数)
  - `crates/octo-server/src/api/sessions.rs` (~60 行，+2 REST 端点)
  - `crates/octo-server/src/router.rs` (~5 行，注册路由)

**新增 AgentMessage 变体**:

```rust
pub enum AgentMessage {
    // ...existing...
    /// Rewind conversation to turn N
    Rewind { to_turn: usize },
    /// Fork conversation at turn N into new session
    Fork { at_turn: usize, new_session_id: SessionId },
}
```

**Harness 辅助函数**:

```rust
/// Rewind messages to the specified turn.
/// A "turn" is a (user, assistant) pair. Turn 0 = first pair.
/// Keeps system messages + first N+1 turn pairs.
pub fn rewind_messages(messages: &mut Vec<ChatMessage>, to_turn: usize) {
    let mut turn_count = 0;
    let mut keep_until = 0;
    for (i, msg) in messages.iter().enumerate() {
        if msg.role == MessageRole::Assistant {
            if turn_count >= to_turn {
                keep_until = i + 1;
                break;
            }
            turn_count += 1;
        }
        keep_until = i + 1;
    }
    messages.truncate(keep_until);
}
```

**Server API**:

```
POST /api/v1/sessions/{id}/rewind  { "to_turn": 3 }
  → 200 { "message_count": N }

POST /api/v1/sessions/{id}/fork    { "at_turn": 3 }
  → 201 { "new_session_id": "..." }
```

**Executor 处理**:
- `Rewind`: 调用 `rewind_messages(&mut self.history, to_turn)`, 持久化
- `Fork`: clone history 截至 at_turn, 创建新 session, 写入 SessionStore

**测试** (~5 tests):
- rewind 截断正确
- rewind turn=0 保留首轮
- fork 创建新 session 且历史正确
- rewind 空 session
- API 端点 400 参数校验

---

### Wave 3：外部集成

#### T5 — Webhook 触发自主模式 (~100 行)

- **Deferred**: AQ-D4
- **依赖**: 无
- **新增文件**: `agent/autonomous_trigger.rs` (~60 行)
- **修改文件**:
  - `agent/mod.rs` (模块声明)
  - `agent/runtime.rs` (~30 行，+start_autonomous 方法)
  - `crates/octo-server/src/api/autonomous.rs` (~40 行，+trigger 端点)
  - `crates/octo-server/src/router.rs` (~3 行)

**TriggerSource trait + ChannelTrigger**:

```rust
use async_trait::async_trait;
use tokio::sync::mpsc;
use octo_types::SessionId;
use super::autonomous::AutonomousConfig;

#[derive(Debug, Clone)]
pub struct TriggerEvent {
    pub session_id: Option<SessionId>,
    pub config_override: Option<AutonomousConfig>,
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait TriggerSource: Send + Sync {
    async fn next_trigger(&mut self) -> anyhow::Result<TriggerEvent>;
    fn name(&self) -> &str;
}

/// Channel-based trigger (用于 Webhook HTTP → 内部调度)
pub struct ChannelTriggerSource {
    rx: mpsc::Receiver<TriggerEvent>,
    name: String,
}

impl ChannelTriggerSource {
    pub fn new(name: &str) -> (Self, mpsc::Sender<TriggerEvent>) {
        let (tx, rx) = mpsc::channel(32);
        (Self { rx, name: name.to_string() }, tx)
    }
}

#[async_trait]
impl TriggerSource for ChannelTriggerSource {
    async fn next_trigger(&mut self) -> anyhow::Result<TriggerEvent> {
        self.rx.recv().await.ok_or_else(|| anyhow::anyhow!("channel closed"))
    }
    fn name(&self) -> &str { &self.name }
}
```

**AgentRuntime 扩展**:

```rust
impl AgentRuntime {
    pub async fn start_autonomous(
        &self,
        session_id: SessionId,
        config: AutonomousConfig,
    ) -> Result<AgentExecutorHandle> {
        // 复用 start_session 逻辑，但注入 autonomous config
        // 发送初始 tick 消息启动自主循环
    }
}
```

**Server Webhook 端点**:

```
POST /api/v1/autonomous/trigger
Body: { "session_id": "optional", "config": { "max_autonomous_rounds": 5 } }
→ 200 { "session_id": "...", "status": "started" }
```

**测试** (~3 tests):
- ChannelTriggerSource 往返
- start_autonomous 创建 handle
- trigger 端点参数验证

---

#### T6 — MessageQueue 触发 (~100 行)

- **Deferred**: AQ-D5
- **依赖**: T5 (TriggerSource trait)
- **新增文件**: 无（扩展 `autonomous_trigger.rs` ~40 行）
- **修改文件**:
  - `agent/autonomous_trigger.rs` (+PollingTriggerSource)
  - `agent/runtime.rs` (~20 行，+register_trigger_source + listener loop)

**PollingTriggerSource** (通用轮询适配器):

```rust
/// 通用轮询式触发源 — 定期调用闭包检查新消息。
/// 适用于 Redis LPOP、NATS subscribe、文件队列等。
pub struct PollingTriggerSource {
    name: String,
    interval: Duration,
    poll_fn: Box<dyn Fn() -> Option<TriggerEvent> + Send + Sync>,
}

#[async_trait]
impl TriggerSource for PollingTriggerSource {
    async fn next_trigger(&mut self) -> anyhow::Result<TriggerEvent> {
        loop {
            if let Some(event) = (self.poll_fn)() {
                return Ok(event);
            }
            tokio::time::sleep(self.interval).await;
        }
    }
    fn name(&self) -> &str { &self.name }
}
```

**TriggerListener** (后台统一监听):

```rust
pub struct TriggerListener {
    sources: Vec<Box<dyn TriggerSource>>,
}

impl TriggerListener {
    pub fn new() -> Self { Self { sources: Vec::new() } }

    pub fn register(&mut self, source: Box<dyn TriggerSource>) {
        self.sources.push(source);
    }

    /// 启动后台监听，触发时调用 callback
    pub fn start(self, runtime: Arc<AgentRuntime>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            // select! 监听所有 sources，触发 runtime.start_autonomous()
        })
    }
}
```

**测试** (~3 tests):
- PollingTriggerSource 轮询触发
- TriggerListener 多 source 注册
- Listener 回调正确

---

#### T7 — tool_search 语义搜索 (~80 行)

- **Deferred**: AQ-D3
- **依赖**: VectorIndex (已存在)
- **修改文件**:
  - `tools/tool_search.rs` (~50 行，+semantic_search 方法 + 混合搜索)
  - `tools/mod.rs` (~10 行，ToolRegistry +build_search_index)
  - `agent/loop_config.rs` (~5 行，可选 tool_search_index 字段)

**混合搜索策略**:

```rust
impl ToolSearchTool {
    /// 混合搜索：先子串匹配，结果不足时 fallback 到语义搜索
    async fn hybrid_search(
        &self, query: &str, limit: usize,
    ) -> Vec<ToolSearchResult> {
        let registry = self.registry.read().await;
        let mut results = search_tools(&registry, query, limit);

        // 子串结果足够则直接返回
        if results.len() >= limit {
            return results;
        }

        // Fallback: 语义搜索（需要 index）
        if let Some(ref index) = *self.search_index.read().await {
            let semantic = index.search(query, limit - results.len()).await;
            for hit in semantic {
                if !results.iter().any(|r| r.name == hit.key) {
                    results.push(ToolSearchResult {
                        name: hit.key,
                        description: hit.metadata.unwrap_or_default(),
                        score: (hit.score * 60.0) as u32, // normalize to 0-60 range
                    });
                }
            }
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(limit);
        results
    }
}
```

**Index 构建** (惰性，首次搜索时触发):

```rust
impl ToolRegistry {
    pub async fn build_search_index(
        &self, provider: &dyn Provider,
    ) -> Result<VectorIndex> {
        let mut index = VectorIndex::new(VectorIndexConfig::default());
        for (name, tool) in self.iter() {
            let desc = tool.spec().description;
            let embedding = provider.embed(&desc).await?;
            index.insert(&name, embedding, Some(desc));
        }
        Ok(index)
    }
}
```

**测试** (~3 tests):
- 混合搜索：子串足够时不走语义
- 混合搜索：子串不足时 fallback 到语义
- 去重（子串和语义同时匹配同一工具）

---

## 五、工作量总结

| Wave | 任务 | 新增/修改代码 | 累计 |
|------|------|-------------|------|
| W1 | T1(升级) + T2(抄本) + T3(GC) | ~230 行 | 230 |
| W2 | T4(fork/rewind) | ~150 行 | 380 |
| W3 | T5(webhook) + T6(MQ) + T7(语义) | ~280 行 | 660 |
| **总计** | **7 任务** | **~660 行** | |

---

## 六、Deferred 状态变更

| ID | 内容 | Phase AR 后状态 |
|----|------|----------------|
| AP-D2 | max_output_tokens 自动升级 | ✅ TokenEscalation (T1) |
| AP-D6 | 会话抄本 | ✅ TranscriptWriter (T2) |
| AP-D7 | 会话 fork/rewind | ✅ Rewind/Fork API (T4) |
| AQ-D2 | BlobStore GC | ✅ BlobGc (T3) |
| AQ-D3 | tool_search 语义搜索 | ✅ 混合搜索 (T7) |
| AQ-D4 | 自主模式 Webhook 触发 | ✅ TriggerSource + Webhook (T5) |
| AQ-D5 | 自主模式 MQ 触发 | ✅ PollingTriggerSource (T6) |

---

## Deferred（暂缓项）

> 本阶段已知但暂未实现的功能点。

| ID | 内容 | 前置条件 | 状态 |
|----|------|---------|------|
| AR-D1 | TranscriptWriter 压缩归档（gzip 老 transcript） | T2 完成 + 存储策略确定 | ⏳ |
| AR-D2 | Fork API 前端 UI（分支可视化） | T4 完成 + 前端 thread 组件 | ⏳ |
| AR-D3 | TriggerSource Redis/NATS 具体实现 | T6 完成 + 消息队列部署 | ⏳ |
| AR-D4 | 语义搜索 index 持久化（避免每次重建） | T7 完成 + index 序列化 | ⏳ |
