# Wave 9 修订实施方案

> 基于可行性评审结果修订。原方案 9 个任务 → 修订为 10 个核心任务 + 1 个前置任务 + 1 个延伸任务。
> 基线：1727 tests @ `fa2ab42` (Wave 8 COMPLETE)
> 当前数据库迁移版本：CURRENT_VERSION = 9

---

## 一、总览表

| 编号 | 任务 | 类型 | 修订 LOC | 依赖 | 风险 | 迁移 |
|------|------|------|----------|------|------|------|
| **前置** W8-T9 | rmcp 升级 (0.16 → 1.x) | 升级 | ~200 | 无 | **高** | 无 |
| T1 | RRF 融合 | 修改 | ~100 | 无 | 低 | 无 |
| T2 | Merkle 链审计 | 修改+迁移 | ~130 | 无 | 低 | **v10** |
| T3 | 消息优先级队列 | 修改 | ~150 | 无 | 低 | 无 |
| T4a | 计量持久化 + 定价表 | 新建 | ~350 | 无 | 中 | **v11** |
| T4b | 计量签名重构 | 修改 | ~200 | T4a | 中 | 无 |
| T5 | Canary per-turn rotation | 修改 | ~80 | 无 | 低 | 无 |
| **延伸** T6 | MCP Server 角色 | 新建 | ~400 | W8-T9 | 高 | 无 |
| T7 | 图片 Token 固定估算 | 修改 | ~50 | 无 | 低 | 无 |
| T8 | ToolProgress 事件 | 修改 | ~100 | 无 | 低 | 无 |
| T9 | Schema Token 结构化建模 | 修改 | ~120 | 无 | 低 | 无 |

**修订总 LOC**: ~1,880 (核心任务 ~1,280 + 前置 ~200 + 延伸 ~400)
**预计新增测试**: ~35 个

---

## 二、迁移版本分配

| 迁移版本 | 任务 | 表变更 |
|----------|------|--------|
| v10 | T2 Merkle 链审计 | `audit_logs` 增加 `prev_hash TEXT`, `hash TEXT` |
| v11 | T4a 计量持久化 | 新建 `metering_records` 表 |

**规则**: 迁移版本严格递增，不可跳号。T2 和 T4a 如并行开发，需约定 v10/v11 归属后再实施。

---

## 三、前置任务：W8-T9 rmcp 升级

### 背景

当前依赖 `rmcp = 0.16`，仅支持 client 角色。T6 (MCP Server 角色) 需要 server 端 API，而 rmcp 0.16 不提供 `RoleServer`。需升级到 rmcp 1.x。

### 设计

**修改文件**:
- `Cargo.toml` (workspace) — rmcp 版本升级
- `crates/octo-engine/src/mcp/stdio.rs` — 适配新 API
- `crates/octo-engine/src/mcp/sse.rs` — 适配新 API (如有 breaking change)
- `crates/octo-engine/src/mcp/bridge.rs` — 工具桥接适配
- `crates/octo-engine/src/mcp/convert.rs` — 类型转换适配

**步骤**:

1. **调研 rmcp 1.x API 变更** — 确认 `RoleClient` / `RoleServer` trait 签名
2. **升级 Cargo.toml** — `rmcp = { version = "1", features = ["client", "server", "transport-child-process", "transport-streamable-http-client-reqwest"] }`
3. **适配 client 代码** — 修复编译错误，保持现有功能不变
4. **验证现有 MCP 测试通过**

**风险**: rmcp 0.16 → 1.x 可能有较大 API 变更。需先 `cargo add rmcp@1 --dry-run` 确认最新稳定版本。若 1.x 尚未发布或 API 不稳定，则：
- **备选方案 A**: 使用 rmcp 的 git 依赖指向 main 分支
- **备选方案 B**: 不依赖 rmcp server API，T6 使用自建 JSON-RPC server (基于 axum)

**预估**: ~200 LOC, 0 新测试 (现有测试应全部通过)

**验收标准**:
- `cargo check --workspace` 通过
- 所有现有 MCP 测试通过 (`cargo test --workspace -p octo-engine -- mcp --test-threads=1`)
- rmcp server feature flag 可用

---

## 四、核心任务设计

### T1: RRF 融合替代硬编码权重 (0.5 天, ~100 LOC)

**状态**: 设计完备，可直接执行

**修改文件**: `crates/octo-engine/src/memory/sqlite_store.rs`

**问题**: 当前第 354-387 行使用硬编码 `0.3 * fts_norm + 0.7 * vec_norm` 做分数融合，权重选择缺乏理论依据，且对结果排序位置不敏感。

**修订设计**: 替换为 Reciprocal Rank Fusion (RRF, k=60)。RRF 的核心优势是不依赖原始分数的归一化，只看排名。

```rust
/// Reciprocal Rank Fusion — 基于排名的融合算法
/// k=60 是 Cormack et al. (2009) 的标准推荐值
fn rrf_fuse(
    fts_results: &[(MemoryEntry, f32)],
    vec_results: &[(MemoryEntry, f32)],
    k: f64,
) -> Vec<(MemoryEntry, f64)> {
    let mut scores: HashMap<String, (MemoryEntry, f64)> = HashMap::new();

    // FTS 结果按原始分数降序排列（已排序），取排名
    for (rank, (entry, _score)) in fts_results.iter().enumerate() {
        let rrf_score = 1.0 / (k + rank as f64 + 1.0);
        scores
            .entry(entry.id.clone())
            .and_modify(|(_, s)| *s += rrf_score)
            .or_insert_with(|| (entry.clone(), rrf_score));
    }

    // Vector 结果同理
    for (rank, (entry, _score)) in vec_results.iter().enumerate() {
        let rrf_score = 1.0 / (k + rank as f64 + 1.0);
        scores
            .entry(entry.id.clone())
            .and_modify(|(_, s)| *s += rrf_score)
            .or_insert_with(|| (entry.clone(), rrf_score));
    }

    let mut results: Vec<_> = scores.into_values().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}
```

**集成点**: 替换 `sqlite_store.rs` 第 354-387 行的 "Step 3: Score fusion" 代码块。fusion 后的 `f64` 分数转为 `f32` 传入后续的 time_decay + importance 加权流程。

**向后兼容**: 纯内部变更，不影响任何公开 API。

**测试**: 3 个测试
- `test_rrf_fuse_basic` — 两个列表有交集，验证交集项分数更高
- `test_rrf_fuse_disjoint` — 两个列表无交集，验证所有项都出现
- `test_rrf_fuse_single_source` — 仅 FTS 无 vector，退化为单源排名

---

### T2: Merkle 链审计防篡改 (0.5 天, ~130 LOC)

**状态**: 设计完备，可直接执行

**修改文件**: `crates/octo-engine/src/audit/storage.rs`, `crates/octo-engine/src/db/migrations.rs`

**问题**: 当前 `AuditStorage` 是简单的 INSERT + SELECT，无防篡改机制。攻击者可直接修改 SQLite 中的审计记录。

**修订设计**: `AuditRecord` 增加 `prev_hash` 和 `hash` 字段。每条记录的 hash = SHA-256(prev_hash + timestamp + event_type + action + result)。`sha2` 已在 workspace 依赖中。

**迁移 v10**:

```sql
-- Migration v10: audit_logs hash chain
ALTER TABLE audit_logs ADD COLUMN prev_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE audit_logs ADD COLUMN hash TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_audit_hash ON audit_logs(hash);
```

**AuditRecord 变更**:

```rust
#[derive(Debug, Clone)]
pub struct AuditRecord {
    // ... 现有字段不变 ...
    pub prev_hash: String,
    pub hash: String,
}
```

**AuditStorage 新增方法**:

```rust
use sha2::{Sha256, Digest};

impl AuditStorage {
    /// 获取最后一条记录的 hash（链头）
    fn last_hash(&self) -> rusqlite::Result<String> {
        self.conn
            .query_row(
                "SELECT hash FROM audit_logs ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .or_else(|_| Ok(String::new())) // 空链返回空字符串
    }

    /// 计算记录哈希
    fn compute_hash(prev_hash: &str, timestamp: &str, event_type: &str, action: &str, result: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(timestamp.as_bytes());
        hasher.update(event_type.as_bytes());
        hasher.update(action.as_bytes());
        hasher.update(result.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 插入带哈希链的审计记录（替代原 log 方法的内部实现）
    pub fn log_chained(&self, event: AuditEvent) -> rusqlite::Result<i64> {
        let prev_hash = self.last_hash()?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let hash = Self::compute_hash(&prev_hash, &timestamp, &event.event_type, &event.action, &event.result);
        // INSERT 包含 prev_hash 和 hash
        // ...
    }

    /// 验证审计链完整性
    pub fn verify_chain(&self, from_id: i64, to_id: i64) -> rusqlite::Result<ChainVerifyResult> {
        // SELECT id, prev_hash, hash, timestamp, event_type, action, result
        // 逐条重算 hash，与存储的 hash 比对
        // 返回 ChainVerifyResult { valid: bool, broken_at: Option<i64>, records_checked: usize }
    }
}

pub struct ChainVerifyResult {
    pub valid: bool,
    pub broken_at: Option<i64>,
    pub records_checked: usize,
}
```

**向后兼容**: 原 `log()` 方法保持签名不变，内部调用 `log_chained()`。`query()` 返回的 `AuditRecord` 增加了两个字段，但这是 struct 扩展，不是 breaking change。

**测试**: 4 个测试
- `test_chained_insert` — 连续插入 3 条，验证 hash 链连贯
- `test_verify_chain_valid` — 正常链验证通过
- `test_verify_chain_tampered` — 篡改中间记录后验证失败，返回正确的 broken_at
- `test_empty_chain` — 空链验证通过

---

### T3: 消息优先级队列 (0.5 天, ~150 LOC)

**状态**: 设计完备，可直接执行

**修改文件**: `crates/octo-engine/src/agent/queue.rs`

**问题**: 当前 `MessageQueue` 仅区分 Steering/FollowUp 两种类型，无优先级概念。E-Stop 消息和普通 steering 消息同等对待。

**修订设计**: 在现有 `QueueEntry` 上增加 `MessagePriority` 字段，drain 时按优先级排序。

```rust
/// 消息优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,       // 用户中断
    Critical = 3,   // 系统 steering / E-Stop
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// QueueEntry 增加 priority 字段
pub struct QueueEntry {
    pub content: String,
    pub kind: QueueKind,
    pub priority: MessagePriority,
    pub timestamp: std::time::Instant,
}

impl QueueEntry {
    pub fn new(content: String, kind: QueueKind) -> Self {
        Self {
            content,
            kind,
            priority: MessagePriority::Normal,
            timestamp: std::time::Instant::now(),
        }
    }

    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
}
```

**MessageQueue 新增方法**:

```rust
impl MessageQueue {
    /// 带优先级入队 — Critical 消息插入队列前端
    pub fn push_with_priority(&mut self, kind: QueueKind, message: String, priority: MessagePriority) {
        let entry = QueueEntry::new(message, kind).with_priority(priority);
        let queue = match kind {
            QueueKind::Steering => &mut self.steering,
            QueueKind::FollowUp => &mut self.follow_up,
        };
        if priority >= MessagePriority::High {
            queue.push_front(entry);
        } else {
            queue.push_back(entry);
        }
    }
}
```

**向后兼容**: 现有 `push_steering()` / `push_followup()` / `push()` 方法保持不变，默认使用 `Normal` 优先级。`push_with_priority` 是新增方法。

**测试**: 3 个测试
- `test_critical_priority_front` — Critical 消息插入队列前端
- `test_normal_priority_back` — Normal 消息追加到队列末尾
- `test_mixed_priorities_drain_order` — 混合优先级的 drain 顺序正确

---

### T4a: 计量持久化 + 模型定价表 (1 天, ~350 LOC)

**状态**: 原 T4 需拆分。T4a 聚焦存储和定价，不触碰 Provider 签名。

**新建文件**:
- `crates/octo-engine/src/metering/storage.rs` — SQLite 持久化
- `crates/octo-engine/src/metering/pricing.rs` — 模型定价表

**问题 (评审发现)**:
1. 原方案 ~300 LOC 低估，实际需拆分
2. session 级隔离缺失 — `Metering` 只有全局计数器
3. model 名称未传递 — `record_request()` 不接收 model 参数
4. streaming output 未追踪 — `MeteringProvider::stream()` 硬编码 output_tokens=0

**T4a 设计**: 仅做持久化层和定价表，不重构 Provider 签名。

**迁移 v11**:

```sql
-- Migration v11: metering records
CREATE TABLE IF NOT EXISTS metering_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    is_error INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_metering_session ON metering_records(session_id);
CREATE INDEX IF NOT EXISTS idx_metering_model ON metering_records(model);
CREATE INDEX IF NOT EXISTS idx_metering_created ON metering_records(created_at);
```

**MeteringStorage**:

```rust
/// 计量持久化存储
pub struct MeteringStorage {
    db: Database,  // tokio-rusqlite Database
}

impl MeteringStorage {
    pub async fn new(db: Database) -> Self { ... }

    /// 记录单次请求
    pub async fn record(&self, record: MeteringRecord) -> Result<()> { ... }

    /// 按 session 汇总
    pub async fn summary_by_session(&self, session_id: &str) -> Result<MeteringSummary> { ... }

    /// 按 model 汇总
    pub async fn summary_by_model(&self, model: &str) -> Result<MeteringSummary> { ... }

    /// 全局汇总（指定时间范围）
    pub async fn summary_global(&self, since: Option<&str>) -> Result<MeteringSummary> { ... }

    /// 计算成本
    pub fn estimate_cost(&self, summary: &MeteringSummary) -> f64 {
        let pricing = ModelPricing::lookup(&summary.model);
        let input_cost = summary.input_tokens as f64 * pricing.input_per_million / 1_000_000.0;
        let output_cost = summary.output_tokens as f64 * pricing.output_per_million / 1_000_000.0;
        input_cost + output_cost
    }
}

#[derive(Debug, Clone)]
pub struct MeteringRecord {
    pub session_id: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub duration_ms: u64,
    pub is_error: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MeteringSummary {
    pub model: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_duration_ms: u64,
}
```

**ModelPricing** (定价表):

```rust
pub struct ModelPricing {
    pub model_pattern: &'static str,
    pub input_per_million: f64,   // USD per 1M input tokens
    pub output_per_million: f64,  // USD per 1M output tokens
}

impl ModelPricing {
    /// 模型名称模糊匹配定价
    pub fn lookup(model: &str) -> Self {
        PRICING_TABLE.iter()
            .find(|p| model.contains(p.model_pattern))
            .cloned()
            .unwrap_or(Self::default_pricing())
    }

    fn default_pricing() -> Self {
        Self { model_pattern: "unknown", input_per_million: 3.0, output_per_million: 15.0 }
    }
}

static PRICING_TABLE: &[ModelPricing] = &[
    // Anthropic
    ModelPricing { model_pattern: "claude-opus",    input_per_million: 15.0,  output_per_million: 75.0 },
    ModelPricing { model_pattern: "claude-sonnet",  input_per_million: 3.0,   output_per_million: 15.0 },
    ModelPricing { model_pattern: "claude-haiku",   input_per_million: 0.25,  output_per_million: 1.25 },
    // OpenAI
    ModelPricing { model_pattern: "gpt-4o",         input_per_million: 2.5,   output_per_million: 10.0 },
    ModelPricing { model_pattern: "gpt-4-turbo",    input_per_million: 10.0,  output_per_million: 30.0 },
    ModelPricing { model_pattern: "gpt-4",          input_per_million: 30.0,  output_per_million: 60.0 },
    ModelPricing { model_pattern: "gpt-3.5",        input_per_million: 0.5,   output_per_million: 1.5 },
    ModelPricing { model_pattern: "o1",             input_per_million: 15.0,  output_per_million: 60.0 },
    ModelPricing { model_pattern: "o3",             input_per_million: 10.0,  output_per_million: 40.0 },
    // 更多模型可后续追加
];
```

**集成点**: `MeteringProvider::complete()` 成功后，额外调用 `MeteringStorage::record()`（如果 storage 可用）。model 名称从 `CompletionRequest.model` 获取。session_id 暂留空字符串（T4b 解决）。

**测试**: 5 个测试
- `test_record_and_summary_by_model` — 记录多条、按 model 汇总
- `test_summary_by_session` — 按 session 汇总
- `test_pricing_lookup_anthropic` — Anthropic 模型定价匹配
- `test_pricing_lookup_openai` — OpenAI 模型定价匹配
- `test_pricing_lookup_unknown` — 未知模型使用默认定价

---

### T4b: 计量签名重构 (0.5 天, ~200 LOC)

**状态**: 依赖 T4a 完成

**修改文件**:
- `crates/octo-engine/src/metering/mod.rs` — `record_request()` 签名扩展
- `crates/octo-engine/src/providers/metering_provider.rs` — 传递 model + session_id

**问题**: 当前 `Metering::record_request(input, output, duration_ms)` 不接收 model 和 session_id 参数。`MeteringProvider::stream()` 的 output_tokens 硬编码为 0。

**设计**:

1. **扩展 `Metering::record_request` 签名**:

```rust
impl Metering {
    /// 扩展版本：接收 model 和 session_id，同时写入持久化存储
    pub fn record_request_ext(
        &self,
        input: usize,
        output: usize,
        duration_ms: u64,
        model: &str,
        session_id: &str,
    ) {
        // 更新原子计数器（保持向后兼容）
        self.record_request(input, output, duration_ms);
        // 异步写入持久化存储（通过 channel 发送，不阻塞调用方）
        if let Some(tx) = &self.persist_tx {
            let _ = tx.try_send(MeteringRecord {
                session_id: session_id.to_string(),
                model: model.to_string(),
                input_tokens: input as u64,
                output_tokens: output as u64,
                duration_ms,
                is_error: false,
            });
        }
    }
}
```

2. **`MeteringProvider` 传递 model**:

```rust
#[async_trait]
impl Provider for MeteringProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let model = request.model.clone();
        let start = Instant::now();
        let input_tokens = estimate_token_count(&request);
        let result = self.inner.complete(request).await;
        let duration = start.elapsed().as_millis() as u64;

        match &result {
            Ok(response) => {
                let output_tokens = response.usage.output_tokens as usize;
                self.metering.record_request_ext(
                    input_tokens, output_tokens, duration,
                    &model, "",  // session_id 由上层设置
                );
            }
            Err(_) => self.metering.record_error(),
        }
        result
    }
}
```

3. **Streaming output 追踪**: 暂不在 T4b 范围内。stream() 仍记录 output_tokens=0，在 WORK_LOG 中标记为已知限制，后续可通过包装 Stream 来追踪 `Usage` 事件。

**向后兼容**: `record_request()` 保持原签名不变。`record_request_ext()` 是新增方法。`persist_tx` 为 `Option<mpsc::Sender<MeteringRecord>>`，默认 None（不做持久化时无开销）。

**测试**: 3 个测试
- `test_record_request_ext_basic` — 扩展记录同时更新原子计数器
- `test_persist_channel_receive` — persist channel 正确接收记录
- `test_no_persist_when_disabled` — persist_tx 为 None 时不 panic

---

### T5: Per-turn Canary Rotation (0.5 天, ~80 LOC)

**状态**: 设计完备，可直接执行

**修改文件**: `crates/octo-engine/src/security/pipeline.rs`

**问题**: 当前 `CanaryGuardLayer` (第 266-324 行) 使用固定 canary 字符串。多轮对话中，如果 LLM 在某轮"记住"了 canary，后续轮次可能绕过检测。

**修订设计**: 为 `CanaryGuardLayer` 增加 `rotate()` 方法。canary 字段从 `String` 改为 `Arc<Mutex<String>>`（因为 `SafetyLayer` trait 的方法是 `&self`）。

```rust
use std::sync::Mutex;
use uuid::Uuid;

pub struct CanaryGuardLayer {
    canary: Mutex<String>,
}

impl CanaryGuardLayer {
    pub fn new(canary: impl Into<String>) -> Self {
        Self {
            canary: Mutex::new(canary.into()),
        }
    }

    pub fn with_default_canary() -> Self {
        Self::new("__CANARY_7f3a9b2e-4d1c-8e5f-a0b6-c3d2e1f09876__")
    }

    /// 生成新的 canary token 并返回（调用方需嵌入到 system prompt）
    pub fn rotate(&self) -> String {
        let new_canary = format!("__CANARY_{}__", &Uuid::new_v4().to_string()[..12]);
        let mut guard = self.canary.lock().unwrap();
        *guard = new_canary.clone();
        new_canary
    }

    /// 获取当前 canary 值
    pub fn canary(&self) -> String {
        self.canary.lock().unwrap().clone()
    }
}
```

**SafetyLayer 实现调整**: `check_output` 和 `check_tool_result` 中从 `Mutex` 读取当前 canary：

```rust
async fn check_output(&self, response: &str) -> SafetyDecision {
    let canary = self.canary.lock().unwrap().clone();
    if response.contains(&canary) {
        SafetyDecision::Block("...".into())
    } else {
        SafetyDecision::Allow
    }
}
```

**集成点**: `AgentLoop` 每轮开始时调用 `canary_guard.rotate()`，将返回的 canary 嵌入 system prompt 的尾部。

**测试**: 2 个测试
- `test_canary_rotate_changes_value` — rotate 后旧 canary 不再匹配
- `test_canary_rotate_new_detection` — rotate 后新 canary 被正确检测

---

### T7: 图片 Token 固定估算 (简化版) (0.5 天, ~50 LOC)

**状态**: 原设计需修订。原方案依赖 `ContentBlock::Image` 中不存在的 `width/height/detail` 字段。

**评审发现**: 当前 `ContentBlock::Image` 仅有 `source_type`, `media_type`, `data` 三个字段。添加 `width/height` 是 breaking change（需修改所有构造 Image 的调用点）。

**修订设计 (简化方案)**: 不修改 `ContentBlock::Image` 结构体。使用固定估算值替代 `data.len() / CHARS_PER_TOKEN`：

```rust
// 在 budget.rs 的 estimate_messages_tokens 中
ContentBlock::Image { data, media_type, .. } => {
    // Anthropic 官方文档：单张图片约 1600 tokens (标准分辨率)
    // 比 base64(data).len() / 4 准确得多
    // base64 数据本身不计入 context，图片按固定 token 值计算
    estimate_image_tokens_fixed(data.len())
}

/// 基于 base64 数据大小的图片 token 固定估算
/// - base64 大小 < 50KB → low-res: ~85 tokens
/// - base64 大小 < 500KB → standard: ~1600 tokens (单 tile)
/// - base64 大小 >= 500KB → high-res: ~3200 tokens (多 tile)
fn estimate_image_tokens_fixed(base64_len: usize) -> usize {
    match base64_len {
        0..=50_000 => 85,          // 低分辨率缩略图
        50_001..=500_000 => 1600,  // 标准分辨率
        _ => 3200,                  // 高分辨率
    }
}
```

**向后兼容**: 纯内部估算变更。`ContentBlock::Image` 结构体不变。

**未来改进**: 当需要精确估算时，可在 `ContentBlock::Image` 中增加 `Option<ImageMeta>` 字段（含 width/height/detail），这是向后兼容的添加。

**测试**: 2 个测试
- `test_image_token_estimation_small` — 小图片返回 85
- `test_image_token_estimation_large` — 大图片返回 3200

---

### T8: ToolProgress 事件 (0.5 天, ~100 LOC)

**状态**: 设计完备，可直接执行

**修改文件**: `crates/octo-engine/src/agent/events.rs`

**问题**: 长时间工具执行（如 bash 命令、大文件读取）无进度反馈，前端用户只能等待。

**修订设计**: `AgentEvent` 增加 `ToolProgress` variant。复用 `octo-types::ToolProgress` struct（已存在于 `tool.rs:123-177`）。

```rust
// events.rs — AgentEvent 增加 variant
pub enum AgentEvent {
    // ... 现有 variants 不变 ...

    /// 工具执行进度更新
    ToolProgress {
        tool_id: String,
        tool_name: String,
        progress: octo_types::ToolProgress,
    },
}
```

**集成点**: 在 `AgentLoop` 的工具执行路径中，通过 `EventBus` 发送 `AgentEvent::ToolProgress`。BashTool 执行时可按 stdout 行数估算进度。

**前端适配**: `ws.rs` WebSocket handler 需将 `ToolProgress` 事件序列化并推送给前端。前端 `MessageBubble` 可展示进度条。

**测试**: 2 个测试
- `test_tool_progress_event_serialization` — JSON 序列化格式正确
- `test_tool_progress_complete_flag` — fraction=1.0 时 is_complete 返回 true

---

### T9: Schema Token 结构化建模 (简化版) (0.5 天, ~120 LOC)

**状态**: 原设计需修订。原方案引用了不存在的 `t.parameters` 字段。

**评审发现**: `ToolSpec.input_schema` 是 `serde_json::Value`（JSON-Schema 格式），不是结构化的 `parameters` 列表。原设计中的 `t.parameters.iter()` 无法编译。

**修订设计**: 解析 `input_schema` 这个 `serde_json::Value` 来计算 token 开销。

```rust
/// 基于 Anthropic 文档的工具 Schema token 估算
/// 参考: https://docs.anthropic.com/en/docs/build-with-claude/tool-use#token-usage
///
/// FUNC_INIT = 7 tokens (function header boilerplate)
/// name + description 按字符估算
/// 每个 property: KEY_OVERHEAD(3) + name_tokens + desc_tokens + type_token(1)
pub fn estimate_tool_schema_tokens(tools: &[ToolSpec]) -> u64 {
    tools.iter().map(|t| estimate_single_tool_tokens(t)).sum()
}

fn estimate_single_tool_tokens(tool: &ToolSpec) -> u64 {
    const FUNC_INIT: u64 = 7;
    const PROP_OVERHEAD: u64 = 3;
    const CHARS_PER_TOKEN: u64 = 4;

    let name_tokens = (tool.name.len() as u64) / CHARS_PER_TOKEN + 1;
    let desc_tokens = (tool.description.len() as u64) / CHARS_PER_TOKEN + 1;

    // 解析 input_schema 中的 properties
    let prop_tokens = if let Some(props) = tool.input_schema.get("properties").and_then(|v| v.as_object()) {
        props.iter().map(|(key, value)| {
            let key_tokens = (key.len() as u64) / CHARS_PER_TOKEN + 1;
            let desc_tokens = value.get("description")
                .and_then(|d| d.as_str())
                .map(|d| (d.len() as u64) / CHARS_PER_TOKEN + 1)
                .unwrap_or(0);
            let enum_tokens = value.get("enum")
                .and_then(|e| e.as_array())
                .map(|arr| arr.len() as u64 * 2) // 每个 enum 值约 2 tokens
                .unwrap_or(0);
            PROP_OVERHEAD + key_tokens + desc_tokens + 1 + enum_tokens // +1 for type
        }).sum::<u64>()
    } else {
        // fallback: 按 JSON 字符数粗估
        (tool.input_schema.to_string().len() as u64) / CHARS_PER_TOKEN
    };

    FUNC_INIT + name_tokens + desc_tokens + prop_tokens
}
```

**集成点**: 替换 `ContextBudgetManager::estimate_tool_specs_tokens()` 中的实现（当前第 86-92 行直接用 `name.len() + description.len() + input_schema.to_string().len()` 除以 4）。

**向后兼容**: 纯内部估算变更。`estimate_tool_specs_tokens` 签名不变。

**测试**: 3 个测试
- `test_schema_tokens_simple_tool` — 简单工具 (2 parameters) 估算合理
- `test_schema_tokens_complex_tool` — 复杂工具 (多 enum, nested) 估算合理
- `test_schema_tokens_no_properties` — 无参数工具走 fallback

---

## 五、延伸任务：T6 MCP Server 角色

### 前置条件

W8-T9 (rmcp 升级) 必须完成，且 rmcp 1.x 提供可用的 `RoleServer` trait。

### 设计

**新建文件**: `crates/octo-engine/src/mcp/server.rs`

**核心**: 将 `ToolRegistry` 中注册的工具暴露为 MCP Server，供外部 agent 通过 stdio 或 SSE 调用。

```rust
use rmcp::server::{RoleServer, ServerConfig};  // rmcp 1.x API (待确认)

pub struct OctoMcpServer {
    tool_registry: Arc<ToolRegistry>,
    config: McpServerConfig,
}

impl OctoMcpServer {
    pub fn new(tool_registry: Arc<ToolRegistry>, config: McpServerConfig) -> Self { ... }

    /// 启动 stdio MCP server
    pub async fn serve_stdio(&self) -> Result<()> { ... }

    /// 启动 SSE MCP server (绑定 HTTP 端口)
    pub async fn serve_sse(&self, addr: SocketAddr) -> Result<()> { ... }
}

/// 实现 rmcp RoleServer trait — 将 tools/list 和 tools/call 映射到 ToolRegistry
impl RoleServer for OctoMcpServer {
    async fn list_tools(&self) -> Vec<ToolInfo> {
        self.tool_registry.list().into_iter().map(|t| /* convert */).collect()
    }

    async fn call_tool(&self, name: &str, params: Value) -> Result<Value> {
        self.tool_registry.execute(name, params).await
    }
}
```

**注意**: 以上代码是概念设计，具体 API 取决于 rmcp 1.x 的实际 trait 定义。W8-T9 完成后需根据实际 API 调整。

**备选方案** (如 rmcp 1.x 不可用): 基于 axum 自建 JSON-RPC 2.0 server，实现 MCP 协议的 `tools/list` 和 `tools/call` 方法。

**测试**: 5 个测试
- `test_mcp_server_list_tools` — 列出工具
- `test_mcp_server_call_tool` — 调用工具
- `test_mcp_server_unknown_tool` — 未知工具返回错误
- `test_mcp_server_tool_error` — 工具执行失败正确返回
- `test_mcp_server_capabilities` — capabilities 声明正确

---

## 六、并行执行策略

### Phase 0: 前置任务 (单独执行)

```
Agent-0: W8-T9 rmcp 升级 (~1 天)
         ↓ 完成后解锁 T6
```

### Phase 1: 核心任务 (3 个并行 Agent)

```
Agent-1: T1 RRF (0.5天) → T2 Merkle (0.5天) → T3 Priority Queue (0.5天)
         [串行：共享 octo-engine 文件，避免冲突]

Agent-2: T4a Metering 存储 (1天) → T4b Metering 签名 (0.5天)
         [串行：T4b 依赖 T4a]

Agent-3: T5 Canary (0.5天) → T7 Image Token (0.5天) → T8 ToolProgress (0.5天) → T9 Schema Token (0.5天)
         [串行：轻量任务链，文件无冲突]
```

**Phase 1 预计 Wall Time**: 2 天

### Phase 2: 延伸任务 (Phase 0 + Phase 1 完成后)

```
Agent-4: T6 MCP Server 角色 (2-3 天)
```

### 总 Wall Time

- 不含 T6: ~3 天 (Phase 0: 1天 + Phase 1: 2天)
- 含 T6: ~5-6 天

### 冲突避免规则

| Agent | 独占文件 | 共享只读 |
|-------|----------|----------|
| Agent-1 | sqlite_store.rs, storage.rs (audit), queue.rs | migrations.rs (v10) |
| Agent-2 | metering/mod.rs, metering/storage.rs, metering/pricing.rs, metering_provider.rs | migrations.rs (v11) |
| Agent-3 | pipeline.rs (security), budget.rs, events.rs | tool.rs (只读) |

**migrations.rs 协调**: Agent-1 写 v10，Agent-2 写 v11。两者在文件末尾追加，不修改已有代码。但为安全起见，建议 Agent-1 先完成 T2 (v10)，Agent-2 再执行 T4a (v11)。或者约定各自的 migration 函数名和版本号后并行开发，最后统一合并。

---

## 七、验收标准

### 通用标准

1. `cargo check --workspace` 通过
2. `cargo test --workspace -- --test-threads=1` 全部通过
3. 测试基线：1727 + ~35 = **1762+** tests
4. `cargo clippy --workspace -- -D warnings` 无 warning
5. 无新增 `unwrap()` 在非测试代码中（使用 `?` 或 `expect()`)

### 任务级验收

| 任务 | 验收标准 |
|------|----------|
| W8-T9 | rmcp 版本 >= 1.0, 现有 MCP 测试全部通过, `features = ["server"]` 可用 |
| T1 | 混合搜索结果排序与旧版不同但更合理, RRF 3 个测试通过 |
| T2 | 审计记录包含 hash 链, `verify_chain()` 能检测篡改, 4 个测试通过 |
| T3 | Critical 消息排在 Normal 前面, 向后兼容现有 API, 3 个测试通过 |
| T4a | `metering_records` 表可写可读, 定价计算准确, 5 个测试通过 |
| T4b | `record_request_ext()` 可用, model 名称被记录, 3 个测试通过 |
| T5 | `rotate()` 生成不同 canary, 旧 canary 不再触发检测, 2 个测试通过 |
| T6 | (延伸) MCP Server 可列出工具/调用工具, 5 个测试通过 |
| T7 | 图片 token 估算不再依赖 base64 长度 / 4, 2 个测试通过 |
| T8 | `AgentEvent::ToolProgress` 可序列化, 2 个测试通过 |
| T9 | Schema token 估算解析 JSON-Schema properties, 3 个测试通过 |

### 集成验收

完成所有核心任务后执行集成验证:

1. **内存搜索端到端**: 写入多条 memory → 混合搜索 → 确认 RRF 融合生效
2. **审计链端到端**: 写入 10 条审计 → verify_chain → 篡改第 5 条 → verify 报告 broken_at=5
3. **计量端到端**: 通过 MeteringProvider 发送请求 → 检查 metering_records 表 → 计算成本
4. **安全端到端**: 设置 canary → rotate → 检查旧 canary 不再触发 block

---

## 八、风险登记

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| rmcp 1.x 未发布或 API 大幅变动 | 中 | 高 (T6 阻塞) | 备选方案: 自建 JSON-RPC server |
| Migration v10/v11 并行开发冲突 | 低 | 中 | 约定版本号后并行，合并时检查 |
| `CanaryGuardLayer` 加 Mutex 影响性能 | 低 | 低 | canary 检查是字符串 contains，Mutex 锁时间极短 |
| T4b persist_tx channel 背压 | 低 | 中 | 使用 bounded channel + try_send，满时丢弃不阻塞 |
| 图片 token 固定估算不够精确 | 中 | 低 | 三档估算已比 base64/4 准确很多，后续可加 ImageMeta |

---

## 九、文件变更清单

### 新建文件 (4)
- `crates/octo-engine/src/metering/storage.rs`
- `crates/octo-engine/src/metering/pricing.rs`
- `crates/octo-engine/src/mcp/server.rs` (延伸任务)
- 测试文件 (在各 crate 现有 tests 目录中)

### 修改文件 (9)
- `Cargo.toml` — rmcp 版本 (前置任务)
- `crates/octo-engine/src/memory/sqlite_store.rs` — T1 RRF
- `crates/octo-engine/src/audit/storage.rs` — T2 Merkle
- `crates/octo-engine/src/db/migrations.rs` — T2 v10 + T4a v11
- `crates/octo-engine/src/agent/queue.rs` — T3 Priority
- `crates/octo-engine/src/metering/mod.rs` — T4b 签名扩展
- `crates/octo-engine/src/providers/metering_provider.rs` — T4b model 传递
- `crates/octo-engine/src/security/pipeline.rs` — T5 Canary rotation
- `crates/octo-engine/src/context/budget.rs` — T7 图片 + T9 Schema
- `crates/octo-engine/src/agent/events.rs` — T8 ToolProgress
