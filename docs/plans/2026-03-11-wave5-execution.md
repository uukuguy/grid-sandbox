# Wave 5 执行计划 — 共识持久化 + 离线同步 + TLS 支持

> **目标**: 实现 Wave 5 的三个暂缓项，完成 octo-sandbox 数据持久化和安全通信基础设施。
>
> **基线**: 1501 tests passing @ commit `6d41b7a`（Wave 4 完成后）
>
> **前置完成**: Wave 3 (D2+D7+D3) + Wave 4 (D1-P1+D1-P2+D5)
>
> **设计文档**: `docs/design/WAVE5_DEFERRED_DESIGN.md`

---

## 设计决策记录

| 决策点 | 选项 | 决策 | 理由 |
|--------|------|------|------|
| D1-P3 私钥持久化 | A)不持久化 B)SQLite加密 C)Keyring | **B) AES-GCM 加密存储到 SQLite** | Agent 需要持久身份，跨会话验证签名 |
| D6 时间戳方案 | A)物理时钟 B)HLC C)Lamport | **B) Hybrid Logical Clock** | 保证因果序，仅多 ~50 LOC |
| D4 范围缩减 | A)内置ACME B)反向代理 C)混合 | **缩减为 TLS 配置 + 自签名 + Caddy 模板** | 内置 ACME 使用率低，反向代理更实用 |

---

## 总览

| Wave | 主题 | Tasks | 估计 LOC | 预估工期 |
|------|------|-------|---------|---------|
| **Wave 5a** | 共识持久化 | D1-P3 (7 subtasks) | ~1100 | 2-3 天 |
| **Wave 5b** | 离线同步 | D6 (9 subtasks) | ~2250 | 5-8 天 |
| **Wave 5c** | TLS 支持 | D4-lite (6 subtasks) | ~550 | 1-2 天 |

---

## Wave 5a: D1-P3 — Byzantine 共识持久化

### 前置条件: ✅ 全部满足
- D1-P1 (PBFT-lite) ✅ @ `0f1b0a7`
- D1-P2 (ED25519 + ViewChange) ✅ @ `e28225c`

### 任务分解

#### D1-P3-T1: DB Migration v7 — 共识持久化表

**文件**: `crates/octo-engine/src/db/migrations.rs`

新增三张表:

```sql
-- byzantine_proposals: 共识提案持久化
CREATE TABLE IF NOT EXISTS byzantine_proposals (
    id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    collaboration_id TEXT NOT NULL,
    proposer TEXT NOT NULL,
    action TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    phase TEXT NOT NULL DEFAULT 'PrePrepare',
    prepare_votes TEXT NOT NULL DEFAULT '[]',    -- JSON
    commit_votes TEXT NOT NULL DEFAULT '[]',     -- JSON
    total_agents INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    finalized_at TEXT,
    PRIMARY KEY (id, session_id)
);
CREATE INDEX idx_bp_session ON byzantine_proposals(session_id, collaboration_id);
CREATE INDEX idx_bp_phase ON byzantine_proposals(session_id, phase);

-- consensus_view_state: 视图状态持久化
CREATE TABLE IF NOT EXISTS consensus_view_state (
    session_id TEXT NOT NULL,
    collaboration_id TEXT NOT NULL,
    view_number INTEGER NOT NULL DEFAULT 0,
    leader TEXT NOT NULL,
    agents TEXT NOT NULL DEFAULT '[]',
    timeout_ms INTEGER NOT NULL DEFAULT 5000,
    pending_requests TEXT NOT NULL DEFAULT '[]',
    updated_at TEXT NOT NULL,
    PRIMARY KEY (session_id, collaboration_id)
);

-- consensus_signatures: 签名审计日志 (不可变)
CREATE TABLE IF NOT EXISTS consensus_signatures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    proposal_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    phase TEXT NOT NULL,
    approve INTEGER NOT NULL,
    signature BLOB NOT NULL,
    public_key BLOB NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_cs_proposal ON consensus_signatures(session_id, proposal_id);

-- consensus_keypairs: Agent 密钥持久化 (AES-GCM 加密)
CREATE TABLE IF NOT EXISTS consensus_keypairs (
    agent_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    public_key BLOB NOT NULL,
    private_key_encrypted BLOB NOT NULL,       -- AES-GCM 加密
    encryption_nonce BLOB NOT NULL,            -- 12 bytes nonce
    created_at TEXT NOT NULL,
    PRIMARY KEY (agent_id, session_id)
);
```

**估计**: ~100 LOC

---

#### D1-P3-T2: ByzantineStore trait 定义

**文件**: `crates/octo-engine/src/agent/collaboration/persistence.rs` (扩展)

```rust
#[async_trait]
pub trait ByzantineStore: Send + Sync {
    // 提案 CRUD
    async fn save_proposal(&self, session_id: &SessionId, collab_id: &str,
        proposal: &ByzantineProposal) -> Result<()>;
    async fn load_proposal(&self, session_id: &SessionId,
        proposal_id: &str) -> Result<Option<ByzantineProposal>>;
    async fn list_proposals(&self, session_id: &SessionId, collab_id: &str,
        phase_filter: Option<ConsensusPhase>) -> Result<Vec<ByzantineProposal>>;
    async fn update_proposal(&self, session_id: &SessionId,
        proposal: &ByzantineProposal) -> Result<()>;
    async fn delete_proposals(&self, session_id: &SessionId,
        collab_id: &str) -> Result<usize>;

    // 视图状态
    async fn save_view_state(&self, session_id: &SessionId, collab_id: &str,
        tracker: &ViewChangeTracker) -> Result<()>;
    async fn load_view_state(&self, session_id: &SessionId,
        collab_id: &str) -> Result<Option<ViewChangeTracker>>;

    // 签名审计
    async fn log_signature(&self, session_id: &SessionId, proposal_id: &str,
        signed_msg: &SignedMessage, phase: &str, approve: bool) -> Result<()>;
    async fn get_signatures(&self, session_id: &SessionId,
        proposal_id: &str) -> Result<Vec<SignatureRecord>>;

    // 密钥持久化
    async fn save_keypair(&self, session_id: &SessionId, agent_id: &str,
        keypair: &ConsensusKeypair, encryption_key: &[u8; 32]) -> Result<()>;
    async fn load_keypair(&self, session_id: &SessionId, agent_id: &str,
        encryption_key: &[u8; 32]) -> Result<Option<ConsensusKeypair>>;
}
```

**估计**: ~80 LOC

---

#### D1-P3-T3: SqliteByzantineStore 实现

**文件**: `crates/octo-engine/src/agent/collaboration/sqlite_store.rs` (新建)

实现 `ByzantineStore` trait:
- 使用 `tokio_rusqlite::Connection` 异步访问
- `save_proposal`: INSERT OR REPLACE，votes 序列化为 JSON
- `load_proposal`: SELECT + JSON 反序列化
- `list_proposals`: 支持 phase 过滤
- `update_proposal`: UPDATE phase + votes
- `save_keypair`: AES-GCM 加密私钥后存储
- `load_keypair`: 读取 + AES-GCM 解密

同时实现 `CollaborationStore`（已有 trait），替代 `InMemoryCollaborationStore`。

**估计**: ~400 LOC

---

#### D1-P3-T4: 密钥加密工具

**文件**: `crates/octo-engine/src/agent/collaboration/crypto.rs` (扩展)

```rust
// 新增方法
impl ConsensusKeypair {
    /// 加密私钥用于持久化存储
    pub fn encrypt_private_key(&self, key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>)>;

    /// 从加密数据恢复密钥对
    pub fn decrypt_and_restore(
        agent_id: &str, public_key: &[u8],
        encrypted_private: &[u8], nonce: &[u8], key: &[u8; 32],
    ) -> Result<Self>;
}
```

复用已有 `aes-gcm` 依赖。

**估计**: ~80 LOC

---

#### D1-P3-T5: CollaborationManager 集成

**文件**: `crates/octo-engine/src/agent/collaboration/manager.rs` (修改)

- 添加 `store: Option<Arc<dyn ByzantineStore>>` 字段
- `with_store(store)` builder 方法
- 在 `add_proposal` / `vote` / `view_change` 时自动持久化
- 在 `new()` / `with_context()` 时自动从 store 恢复状态

**估计**: ~80 LOC

---

#### D1-P3-T6: mod.rs 导出更新

**文件**: `crates/octo-engine/src/agent/collaboration/mod.rs` (修改)

- 导出 `sqlite_store` 模块
- 导出 `ByzantineStore` trait
- 导出 `SignatureRecord` 类型

**估计**: ~15 LOC

---

#### D1-P3-T7: 测试 (15+ tests)

**文件**: `crates/octo-engine/tests/byzantine_persistence.rs` (新建)

| 测试 | 内容 |
|------|------|
| `test_save_load_proposal` | 提案序列化/反序列化 roundtrip |
| `test_proposal_with_votes` | 带投票的提案持久化 |
| `test_list_proposals_by_phase` | Phase 过滤查询 |
| `test_update_proposal_phase` | Phase 推进后更新 |
| `test_delete_proposals` | 批量删除 |
| `test_save_load_view_state` | 视图状态 roundtrip |
| `test_view_state_with_requests` | 带 pending requests 的视图状态 |
| `test_log_and_get_signatures` | 签名审计日志 |
| `test_signature_ordering` | 签名时间序 |
| `test_keypair_encrypt_decrypt` | 密钥加密/解密 roundtrip |
| `test_keypair_wrong_key` | 错误加密密钥拒绝 |
| `test_collaboration_store_save_load` | CollaborationSnapshot roundtrip |
| `test_collaboration_store_list` | 列表查询 |
| `test_collaboration_store_delete` | 删除操作 |
| `test_full_consensus_lifecycle` | 完整共识流程: 创建→投票→终结→持久化→恢复 |

**估计**: ~350 LOC

---

### Wave 5a 提交策略

```
commit: "feat(collaboration): D1-P3 — Byzantine consensus persistence with encrypted keypairs"
checkpoint: "checkpoint: Wave 5a COMPLETE — D1-P3"
```

### Wave 5a 验收标准

- [ ] `cargo check --workspace` 无错误
- [ ] `cargo test --workspace -- --test-threads=1` 全部通过
- [ ] Migration v7 正确创建 4 张新表
- [ ] ByzantineProposal 持久化/恢复 roundtrip 正确
- [ ] ViewState 持久化/恢复正确
- [ ] 签名审计日志可查询
- [ ] ConsensusKeypair AES-GCM 加密/解密正确
- [ ] 错误加密密钥被拒绝

---

## Wave 5b: D6 — 离线模式 SQLite 同步 (LWW + HLC)

### 前置条件
- D1-P3 ⏳ 建议先完成（共识数据需要持久化后才能同步）
- D5 ✅ Tauri 自动更新已就绪

### 任务分解

#### D6-T1: HLC (Hybrid Logical Clock) 实现

**文件**: `crates/octo-engine/src/sync/hlc.rs` (新建)

```rust
/// Hybrid Logical Clock: 物理时钟 + 逻辑计数器
pub struct HybridClock {
    physical: AtomicI64,     // 物理时间 (ms since epoch)
    logical: AtomicU32,       // 逻辑计数器
    node_id: String,          // 设备唯一 ID
}

pub struct HlcTimestamp {
    pub physical_ms: i64,
    pub logical: u32,
    pub node_id: String,
}

impl HybridClock {
    pub fn now(&self) -> HlcTimestamp;          // 生成新时间戳
    pub fn update(&self, remote: &HlcTimestamp); // 收到远程时间戳后更新
}

impl Ord for HlcTimestamp { ... }              // 全序比较
```

**估计**: ~100 LOC

---

#### D6-T2: DB Migration v8 — 同步基础设施

**文件**: `crates/octo-engine/src/db/migrations.rs` (扩展)

```sql
-- sync_metadata: 设备同步元数据
CREATE TABLE IF NOT EXISTS sync_metadata (
    device_id TEXT PRIMARY KEY,
    last_sync_at TEXT,
    sync_version INTEGER NOT NULL DEFAULT 0,
    server_url TEXT,
    created_at TEXT NOT NULL
);

-- sync_changelog: 变更日志
CREATE TABLE IF NOT EXISTS sync_changelog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    row_id TEXT NOT NULL,
    operation TEXT NOT NULL,             -- INSERT | UPDATE | DELETE
    changed_columns TEXT,                -- JSON
    old_values TEXT,                      -- JSON
    new_values TEXT,                      -- JSON
    device_id TEXT NOT NULL,
    hlc_timestamp TEXT NOT NULL,          -- HLC 序列化格式
    sync_version INTEGER NOT NULL,
    synced INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_cl_unsynced ON sync_changelog(synced, sync_version);
CREATE INDEX idx_cl_table ON sync_changelog(table_name, row_id);

-- 为同步表添加 HLC 列
ALTER TABLE sessions ADD COLUMN hlc_updated TEXT;
ALTER TABLE session_messages ADD COLUMN hlc_updated TEXT;
ALTER TABLE memories ADD COLUMN hlc_updated TEXT;
ALTER TABLE memory_blocks ADD COLUMN hlc_updated TEXT;
ALTER TABLE mcp_servers ADD COLUMN hlc_updated TEXT;
ALTER TABLE scheduled_tasks ADD COLUMN hlc_updated TEXT;
```

注意: SQLite triggers 在 Rust 代码中注册（非 migration SQL），因为需要访问 HLC。

**估计**: ~120 LOC

---

#### D6-T3: 变更追踪器

**文件**: `crates/octo-engine/src/sync/changelog.rs` (新建)

```rust
pub struct ChangeTracker {
    conn: tokio_rusqlite::Connection,
    clock: Arc<HybridClock>,
    device_id: String,
}

impl ChangeTracker {
    pub async fn record_change(&self, table: &str, row_id: &str,
        op: SyncOperation, old: Option<Value>, new: Option<Value>) -> Result<()>;
    pub async fn get_unsynced_changes(&self, limit: usize) -> Result<Vec<SyncChange>>;
    pub async fn mark_synced(&self, change_ids: &[i64]) -> Result<()>;
    pub async fn get_changes_since(&self, version: u64) -> Result<Vec<SyncChange>>;
    pub async fn cleanup_old_changes(&self, older_than_days: u32) -> Result<usize>;
}
```

**估计**: ~250 LOC

---

#### D6-T4: 同步协议类型定义

**文件**: `crates/octo-engine/src/sync/protocol.rs` (新建)

```rust
pub enum SyncOperation { Insert, Update, Delete }

pub struct SyncChange {
    pub id: Option<i64>,                 // changelog ID (本地)
    pub table_name: String,
    pub row_id: String,
    pub operation: SyncOperation,
    pub data: serde_json::Value,
    pub hlc_timestamp: HlcTimestamp,
    pub device_id: String,
    pub sync_version: u64,
}

pub struct SyncPullRequest { pub since_version: u64, pub limit: usize }
pub struct SyncPullResponse { pub changes: Vec<SyncChange>, pub server_version: u64, pub has_more: bool }
pub struct SyncPushRequest { pub changes: Vec<SyncChange>, pub device_id: String, pub client_version: u64 }
pub struct SyncPushResponse { pub applied: usize, pub conflicts: Vec<SyncConflict>, pub server_version: u64 }
pub struct SyncConflict {
    pub table_name: String, pub row_id: String,
    pub client_value: Value, pub server_value: Value,
    pub resolution: ConflictResolution,
}
pub enum ConflictResolution { ClientWins, ServerWins }
```

**估计**: ~150 LOC

---

#### D6-T5: LWW 冲突解决引擎

**文件**: `crates/octo-engine/src/sync/lww.rs` (新建)

```rust
pub struct LwwResolver;

impl LwwResolver {
    /// 合并两个变更: HLC 较大者胜出
    pub fn resolve(local: &SyncChange, remote: &SyncChange) -> ConflictResolution;

    /// 批量应用远程变更到本地数据库
    pub async fn apply_remote_changes(
        conn: &tokio_rusqlite::Connection,
        changes: &[SyncChange],
        clock: &HybridClock,
    ) -> Result<Vec<SyncConflict>>;
}
```

DELETE vs UPDATE: DELETE wins（墓碑语义）。

**估计**: ~200 LOC

---

#### D6-T6: 同步服务端

**文件**: `crates/octo-engine/src/sync/server.rs` (新建)

```rust
pub struct SyncServer {
    conn: tokio_rusqlite::Connection,
    clock: Arc<HybridClock>,
}

impl SyncServer {
    pub async fn handle_pull(&self, req: SyncPullRequest) -> Result<SyncPullResponse>;
    pub async fn handle_push(&self, req: SyncPushRequest) -> Result<SyncPushResponse>;
    pub async fn get_status(&self, device_id: &str) -> Result<SyncStatus>;
}
```

**估计**: ~250 LOC

---

#### D6-T7: 同步客户端

**文件**: `crates/octo-engine/src/sync/client.rs` (新建)

```rust
pub struct SyncClient {
    server_url: String,
    device_id: String,
    http: reqwest::Client,
    tracker: Arc<ChangeTracker>,
    clock: Arc<HybridClock>,
}

impl SyncClient {
    pub async fn sync(&self) -> Result<SyncReport>;  // 完整同步流程
    async fn pull(&self) -> Result<SyncPullResponse>;
    async fn push(&self) -> Result<SyncPushResponse>;
    async fn invalidate_caches(&self);  // 同步后清空 DashMap 缓存
}
```

**估计**: ~250 LOC

---

#### D6-T8: REST API 端点

**文件**: `crates/octo-server/src/api/sync.rs` (新建)

```
GET  /api/sync/status?device_id=xxx     → SyncStatus
POST /api/sync/pull                      → SyncPullResponse
POST /api/sync/push                      → SyncPushResponse
```

**估计**: ~150 LOC

---

#### D6-T9: 测试 (20+ tests)

**文件**: `crates/octo-engine/tests/offline_sync.rs` (新建)

| 测试 | 内容 |
|------|------|
| `test_hlc_monotonic` | HLC 单调递增 |
| `test_hlc_causality` | 收到远程时间戳后更新 |
| `test_hlc_ordering` | 全序比较正确 |
| `test_hlc_serialization` | HLC 序列化/反序列化 |
| `test_record_insert_change` | INSERT 变更记录 |
| `test_record_update_change` | UPDATE 变更记录 |
| `test_record_delete_change` | DELETE 变更记录 |
| `test_get_unsynced_changes` | 未同步变更查询 |
| `test_mark_synced` | 标记已同步 |
| `test_cleanup_old_changes` | 过期变更清理 |
| `test_lww_client_wins` | Client 时间戳更新 → Client wins |
| `test_lww_server_wins` | Server 时间戳更新 → Server wins |
| `test_lww_delete_wins` | DELETE vs UPDATE → DELETE wins |
| `test_lww_insert_conflict` | 相同 PK INSERT → LWW |
| `test_apply_remote_inserts` | 应用远程 INSERT |
| `test_apply_remote_updates` | 应用远程 UPDATE |
| `test_apply_remote_deletes` | 应用远程 DELETE |
| `test_pull_protocol` | Pull 请求/响应 roundtrip |
| `test_push_protocol` | Push 请求/响应 roundtrip |
| `test_full_sync_cycle` | 完整同步流程: 本地变更→push→remote变更→pull→合并 |
| `test_sync_after_offline` | 模拟离线后重连同步 |

**估计**: ~500 LOC

---

### Wave 5b 提交策略

```
commit 1: "feat(sync): D6-core — HLC + change tracking + LWW conflict resolution"
commit 2: "feat(sync): D6-protocol — Sync server/client + REST API"
checkpoint: "checkpoint: Wave 5b COMPLETE — D6 offline sync"
```

### Wave 5b 验收标准

- [ ] `cargo check --workspace` 无错误
- [ ] `cargo test --workspace -- --test-threads=1` 全部通过
- [ ] HLC 保证因果序（concurrent 变更可比较）
- [ ] 变更追踪器正确记录 INSERT/UPDATE/DELETE
- [ ] LWW 冲突解决: 较新时间戳胜出
- [ ] DELETE vs UPDATE: DELETE wins
- [ ] Pull/Push 协议 roundtrip 正确
- [ ] 模拟离线→重连→同步场景通过

---

## Wave 5c: D4-lite — TLS 支持（缩减版）

### 范围说明

**不包含**: 内置 ACME 协议、Let's Encrypt 自动证书
**包含**: TLS 配置、自签名证书生成、反向代理部署模板

### 任务分解

#### D4-T1: TLS 配置结构

**文件**: `crates/octo-server/src/config.rs` (修改)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub self_signed: bool,              // 自动生成自签名证书
    pub self_signed_dir: Option<PathBuf>,
}
```

环境变量: `OCTO_TLS_ENABLED`, `OCTO_TLS_CERT_PATH`, `OCTO_TLS_KEY_PATH`, `OCTO_TLS_SELF_SIGNED`

**估计**: ~80 LOC

---

#### D4-T2: 自签名证书生成

**文件**: `crates/octo-engine/src/tls/mod.rs` (新建)

从 `octo-cli/src/commands/dashboard_cert.rs` 提取并泛化:

```rust
pub fn generate_self_signed_cert(
    domain: &str,
    output_dir: &Path,
) -> Result<(PathBuf, PathBuf)>;  // (cert_path, key_path)
```

添加 `rcgen` 为 `octo-engine` 可选依赖（feature = "tls"）。

**估计**: ~100 LOC

---

#### D4-T3: Server TLS 集成

**文件**: `crates/octo-server/src/main.rs` (修改)

```rust
if cfg.tls.enabled {
    let (cert, key) = if cfg.tls.self_signed {
        tls::generate_self_signed_cert("localhost", &data_dir)?
    } else {
        (cfg.tls.cert_path.unwrap(), cfg.tls.key_path.unwrap())
    };
    let tls_config = RustlsConfig::from_pem_file(cert, key).await?;
    axum_server::bind_rustls(addr, tls_config).serve(app).await?;
} else {
    axum::serve(listener, app).await?;
}
```

添加 `axum-server` + `rustls` 为可选依赖。

**估计**: ~80 LOC

---

#### D4-T4: Platform Server TLS 集成

**文件**: `crates/octo-platform-server/src/main.rs` (修改)

同 D4-T3 模式，复用 `octo-engine` 的 TLS 模块。

**估计**: ~60 LOC

---

#### D4-T5: 反向代理部署模板

**文件**: `deploy/caddy/Caddyfile` (新建), `deploy/nginx/nginx.conf` (新建)

```
# Caddy (自动 ACME)
api.example.com {
    reverse_proxy localhost:3001
}

# Nginx + Certbot
server {
    listen 443 ssl;
    server_name api.example.com;
    ssl_certificate /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;
    location / { proxy_pass http://127.0.0.1:3001; }
}
```

**估计**: ~50 LOC

---

#### D4-T6: 测试 (6+ tests)

**文件**: `crates/octo-engine/tests/tls_config.rs` (新建)

| 测试 | 内容 |
|------|------|
| `test_tls_config_default_disabled` | 默认 TLS 禁用 |
| `test_tls_config_from_env` | 环境变量解析 |
| `test_self_signed_cert_generation` | 自签名证书生成 |
| `test_self_signed_cert_valid` | 生成的证书可解析 |
| `test_tls_config_missing_cert_path` | 缺少证书路径报错 |
| `test_tls_config_serialization` | 配置序列化/反序列化 |

**估计**: ~180 LOC

---

### Wave 5c 提交策略

```
commit: "feat(tls): D4-lite — TLS configuration + self-signed certs + deploy templates"
checkpoint: "checkpoint: Wave 5c COMPLETE — D4-lite TLS"
```

### Wave 5c 验收标准

- [ ] `cargo check --workspace` 无错误
- [ ] `cargo test --workspace -- --test-threads=1` 全部通过
- [ ] TLS 配置可通过 env/config.yaml 设置
- [ ] 自签名证书可生成并被 rustls 加载
- [ ] Server 可选择 HTTP 或 HTTPS 模式启动
- [ ] Caddy/Nginx 部署模板可用

---

## 执行波次与依赖

```
Wave 5a: D1-P3 (共识持久化)     ←→  Wave 5c: D4-lite (TLS 支持)
         ↓                              [可并行，无代码依赖]
Wave 5b: D6 (离线同步)
         [依赖 D1-P3: 共识数据需先持久化]
```

### 并行策略

```
Agent-A: D1-P3 (共识持久化)      ← 2-3 天
Agent-B: D4-lite (TLS 支持)      ← 1-2 天 (可与 D1-P3 并行)
         ↓ (D1-P3 完成后)
Agent-C: D6 (离线同步)            ← 5-8 天
```

---

## 提交策略汇总

```
Wave 5a (D1-P3):
  commit 7: "feat(collaboration): D1-P3 — Byzantine consensus persistence with encrypted keypairs"
  checkpoint: "checkpoint: Wave 5a COMPLETE — D1-P3, N tests"

Wave 5c (D4-lite):
  commit 8: "feat(tls): D4-lite — TLS configuration + self-signed certs + deploy templates"
  checkpoint: "checkpoint: Wave 5a+5c COMPLETE — D1-P3+D4-lite, N tests"

Wave 5b (D6):
  commit 9: "feat(sync): D6-core — HLC + change tracking + LWW conflict resolution"
  commit 10: "feat(sync): D6-protocol — Sync server/client + REST API"
  checkpoint: "checkpoint: Wave 5 COMPLETE — D1-P3+D4-lite+D6, N tests"
```

---

## Deferred（仍暂缓）

| ID | 内容 | 前置条件 | 状态 |
|----|------|---------|------|
| D4-ACME | 内置 ACME 自动证书 | 公网域名 + 生产部署 | ⏳ 用 Caddy 反向代理代替 |
| D6-V2 | CRDT 离线同步 | D6-LWW 完成 + 需求验证 | ⏳ |
| D6-Desktop | Desktop 端同步集成 | D6 核心完成 | ⏳ |

---

## 风险总览

| 风险 | 等级 | 影响范围 | 缓解措施 |
|------|------|---------|---------|
| AES-GCM 密钥管理 | 中 | D1-P3 | 从环境变量加载加密密钥，不硬编码 |
| Migration v7/v8 与 v6 兼容 | 低 | D1-P3, D6 | 新增表，不修改已有表 |
| HLC 精度跨平台差异 | 低 | D6 | 使用 `SystemTime::now()` + 逻辑计数器兜底 |
| DashMap 缓存失效 | 中 | D6 | 同步完成后显式清空缓存 |
| FTS5 虚拟表同步 | 中 | D6 | 同步 memories 表后触发 FTS 重建 |
| axum-server TLS 兼容性 | 低 | D4-lite | 已在 octo-cli 验证过 |
