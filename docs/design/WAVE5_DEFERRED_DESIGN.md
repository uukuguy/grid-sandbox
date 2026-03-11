# Wave 5 暂缓项设计与实现方案

> **基于代码实现的深度分析** — 2026-03-11
>
> **基线**: 1501 tests passing @ commit `6d41b7a`（Wave 4 完成后）
>
> **前置完成**: Wave 3 (D2+D7+D3) + Wave 4 (D1-P1+D1-P2+D5)

---

## 总览

| 暂缓项 | 内容 | 复杂度 | 前置条件 | 实施建议 |
|--------|------|--------|---------|---------|
| **D1-P3** | Byzantine 共识持久化 + 前端可视化 | 中 | D1-P2 ✅ 已完成 | **可立即开始** |
| **D6** | 离线模式 SQLite 同步 (LWW) | 高 | D1+D5 ✅ + 协议设计 | 需先完成 D1-P3 |
| **D4** | Let's Encrypt ACME 自动证书 | 中 | 公网域名 + 生产部署 | 基础设施就绪后实施 |

---

## D1-P3: Byzantine 共识持久化

### 1. 现状分析

#### 已实现（D1-P1 + D1-P2）

| 组件 | 文件 | 状态 | 说明 |
|------|------|------|------|
| ConsensusPhase 状态机 | `collaboration/consensus.rs` | ✅ 完整 | PrePrepare→Prepare→Commit→Finalized/Failed |
| ByzantineProposal | `collaboration/consensus.rs` | ✅ 完整 | 仲裁阈值 `2f+1`，投票去重，自动 phase 推进 |
| ED25519 签名 | `collaboration/crypto.rs` | ✅ 完整 | ConsensusKeypair，SignedMessage，投票签名/验证 |
| ViewChange 协议 | `collaboration/consensus.rs` | ✅ 完整 | ViewState 轮询选举，ViewChangeTracker 累积请求 |
| 消息类型 | `collaboration/channel.rs` | ✅ 完整 | 含 Signed 变体 + ViewChange 消息 |
| 测试覆盖 | `tests/byzantine_consensus.rs` | ✅ 48 tests | 共识、签名、视图变更、序列化全覆盖 |

#### 未实现（D1-P3 目标）

| 缺失组件 | 当前状态 | 影响 |
|----------|---------|------|
| SQLite 共识表 | 无（DB migration 仅到 v6） | 共识状态重启后丢失 |
| ByzantineProposal 持久化 | 仅 InMemoryCollaborationStore | 无法恢复未完成的共识 |
| ViewChange 持久化 | 纯内存 | 重启后视图号归零 |
| 签名公钥注册表 | 无 | 无法跨会话验证历史投票 |
| 前端可视化 | 无 | 用户无法观察共识进度 |

### 2. 架构差距分析

**当前持久化层**:

```
CollaborationStore trait (persistence.rs)
    ├── save_collaboration(session_id, collab_id, shared_state, events, proposals)
    ├── load_collaboration(session_id, collab_id) → Option<CollaborationSnapshot>
    ├── list_collaborations(session_id)
    └── delete_collaboration(session_id, collab_id)

CollaborationSnapshot {
    collaboration_id, shared_state, events, proposals, saved_at
}
```

**关键问题**: `CollaborationSnapshot` 只存简单 `Proposal`（非 Byzantine），不包含：
- `ByzantineProposal`（phase、prepare_votes、commit_votes）
- `ViewState` / `ViewChangeTracker`
- `SignedMessage` 签名链
- `ConsensusKeypair` 公钥注册

### 3. 实现方案

#### 3.1 数据库 Migration v7

新增三张表，复用现有 `tokio-rusqlite` 异步模式：

```sql
-- Migration v7: Byzantine consensus persistence

-- 共识提案表
CREATE TABLE IF NOT EXISTS byzantine_proposals (
    id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    collaboration_id TEXT NOT NULL,
    proposer TEXT NOT NULL,
    action TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    phase TEXT NOT NULL DEFAULT 'PrePrepare',
    prepare_votes TEXT NOT NULL DEFAULT '[]',    -- JSON: Vec<ConsensusVote>
    commit_votes TEXT NOT NULL DEFAULT '[]',     -- JSON: Vec<ConsensusVote>
    total_agents INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    finalized_at TEXT,
    PRIMARY KEY (id, session_id)
);

CREATE INDEX IF NOT EXISTS idx_byzantine_proposals_session
    ON byzantine_proposals(session_id, collaboration_id);
CREATE INDEX IF NOT EXISTS idx_byzantine_proposals_phase
    ON byzantine_proposals(session_id, phase);

-- 视图状态表（每个 session+collaboration 一条记录）
CREATE TABLE IF NOT EXISTS consensus_view_state (
    session_id TEXT NOT NULL,
    collaboration_id TEXT NOT NULL,
    view_number INTEGER NOT NULL DEFAULT 0,
    leader TEXT NOT NULL,
    agents TEXT NOT NULL DEFAULT '[]',           -- JSON: Vec<String>
    timeout_ms INTEGER NOT NULL DEFAULT 5000,
    pending_requests TEXT NOT NULL DEFAULT '[]', -- JSON: Vec<ViewChangeRequest>
    updated_at TEXT NOT NULL,
    PRIMARY KEY (session_id, collaboration_id)
);

-- 签名审计日志（不可变，用于事后验证）
CREATE TABLE IF NOT EXISTS consensus_signatures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    proposal_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    phase TEXT NOT NULL,                         -- 'prepare' | 'commit'
    approve INTEGER NOT NULL,                    -- boolean
    signature BLOB NOT NULL,                     -- 64 bytes ED25519
    public_key BLOB NOT NULL,                    -- 32 bytes ED25519
    payload TEXT NOT NULL,                        -- 原始签名内容
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_consensus_sigs_proposal
    ON consensus_signatures(session_id, proposal_id);
```

**设计决策**:
- `prepare_votes`/`commit_votes` 用 JSON 存储而非关系表 — 投票数量有限（通常 < 15），JSON 查询足够
- `consensus_signatures` 独立存储签名链 — 不可变审计日志，支持事后验证
- 复合主键 `(id, session_id)` — 与现有表设计一致

#### 3.2 SQLiteCollaborationStore 实现

扩展 `CollaborationStore` trait，新增 Byzantine 相关方法：

```rust
// 新增 trait 方法（保持向后兼容）
#[async_trait]
pub trait ByzantineStore: Send + Sync {
    // 提案 CRUD
    async fn save_proposal(
        &self, session_id: &SessionId, collab_id: &str,
        proposal: &ByzantineProposal,
    ) -> Result<()>;

    async fn load_proposal(
        &self, session_id: &SessionId, proposal_id: &str,
    ) -> Result<Option<ByzantineProposal>>;

    async fn list_proposals(
        &self, session_id: &SessionId, collab_id: &str,
        phase_filter: Option<ConsensusPhase>,
    ) -> Result<Vec<ByzantineProposal>>;

    async fn update_proposal_phase(
        &self, session_id: &SessionId, proposal_id: &str,
        proposal: &ByzantineProposal,
    ) -> Result<()>;

    // 视图状态
    async fn save_view_state(
        &self, session_id: &SessionId, collab_id: &str,
        tracker: &ViewChangeTracker,
    ) -> Result<()>;

    async fn load_view_state(
        &self, session_id: &SessionId, collab_id: &str,
    ) -> Result<Option<ViewChangeTracker>>;

    // 签名审计
    async fn log_signature(
        &self, session_id: &SessionId, proposal_id: &str,
        signed_msg: &SignedMessage, phase: &str, approve: bool,
    ) -> Result<()>;

    async fn get_signatures(
        &self, session_id: &SessionId, proposal_id: &str,
    ) -> Result<Vec<SignatureRecord>>;
}
```

**实现策略**: 新建 `SqliteCollaborationStore`，同时实现 `CollaborationStore`（已有 trait）和 `ByzantineStore`（新 trait），复用同一个 `tokio_rusqlite::Connection`。

#### 3.3 前端可视化方案

**后端 API 端点**（添加到 `octo-server/src/api/`）:

```
GET  /api/collaborations/:session_id/byzantine
     → 返回该 session 所有 ByzantineProposal 列表

GET  /api/collaborations/:session_id/byzantine/:proposal_id
     → 返回单个提案详情（含投票、签名、phase 历史）

GET  /api/collaborations/:session_id/view-state
     → 返回当前 ViewState（leader、view_number）

WS   /ws (扩展现有 WebSocket)
     → 新增事件类型: ConsensusPhaseChanged, VoteReceived, ViewChanged
```

**前端组件**（添加到 `web/src/pages/` 或 `web/src/components/`）:

| 组件 | 功能 |
|------|------|
| `ProposalTimeline` | 时间轴显示提案 phase 推进过程 |
| `VoteMatrix` | 矩阵视图：agents × phases，显示投票状态 |
| `ViewStateIndicator` | 当前 view number + leader 标识 |
| `ConsensusProgress` | 进度条：当前投票数 / 仲裁阈值 |

### 4. 任务分解

| Task | 内容 | 文件 | 估计 LOC |
|------|------|------|---------|
| D1-P3-1 | DB Migration v7: 三张共识表 | `db/migrations.rs` | ~80 |
| D1-P3-2 | `ByzantineStore` trait 定义 | `collaboration/persistence.rs` | ~60 |
| D1-P3-3 | `SqliteCollaborationStore` 实现 | `collaboration/sqlite_store.rs`（新建） | ~350 |
| D1-P3-4 | 集成到 `CollaborationManager` | `collaboration/manager.rs` | ~50 |
| D1-P3-5 | REST API 端点 | `octo-server/src/api/collaboration.rs`（新建） | ~150 |
| D1-P3-6 | WebSocket 事件扩展 | `octo-server/src/ws.rs` | ~40 |
| D1-P3-7 | 测试（15+ tests） | `tests/byzantine_persistence.rs` | ~300 |

**总计**: ~1030 LOC，预计 2-3 天

### 5. 风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| JSON 投票序列化性能 | 低 | 投票数量受限于 agent 数（< 15） |
| Migration 与现有 v6 兼容 | 低 | 新增表，不修改已有表 |
| ConsensusKeypair 私钥不持久化 | 设计选择 | 每次启动重新生成，公钥通过签名审计表关联 |
| 前端组件与现有 Debug 页面集成 | 中 | 作为 Debug 页面新 Tab 集成 |

---

## D6: 离线模式 SQLite 同步 (LWW)

### 1. 现状分析

#### 数据库层现状

| 层级 | 表 | 存储方式 | 变更追踪 | 同步就绪 |
|------|---|---------|---------|---------|
| L0 Working Memory | `memory_blocks` | SQLite | 无 | ❌ |
| L1 Session | `sessions` + `session_messages` | SQLite + DashMap 缓存 | 无 | ❌ |
| L2 Persistent Memory | `memories` + `memories_fts` | SQLite + FTS5 | 无 | ❌ |
| Tools | `tool_executions` | SQLite | 无 | ❌ |
| MCP | `mcp_servers` + `mcp_executions` + `mcp_logs` | SQLite | 无 | ❌ |
| Scheduler | `scheduled_tasks` + `task_executions` | SQLite | 无 | ❌ |
| Audit | `audit_logs` | SQLite（同步 rusqlite） | 无 | ❌ |
| Collaboration | 内存 only | `InMemoryCollaborationStore` | 无 | ❌ |

**核心缺失**: 所有表均无变更追踪机制（no `updated_at` trigger, no changelog, no vector clock）。

#### 连接管理模式

```
异步路径: tokio_rusqlite::Connection
  → SessionStore, MemoryStore, McpStorage, etc.
  → .call(|conn| { ... }).await

同步路径: rusqlite::Connection
  → GraphStore, AuditStorage
  → 直接阻塞调用
```

**WAL 模式已启用**: `PRAGMA journal_mode = WAL` — 有利于并发读写，但不支持跨设备同步。

#### Desktop 端现状

- Tauri 2.0 集成完成（D5）
- 内嵌 `octo-server` 运行在随机 localhost 端口
- 自动更新已就绪（`tauri-plugin-updater`）
- **无任何离线/同步逻辑**

### 2. LWW (Last-Write-Wins) 同步方案设计

#### 2.1 设计原则

1. **LWW 优先** — 最简单的冲突解决策略，适合单用户多设备场景
2. **增量同步** — 只传输上次同步后的变更
3. **幂等操作** — 同一变更重复应用不会产生副作用
4. **离线容忍** — 设备可长时间离线后同步

#### 2.2 变更追踪层（Change Tracking）

**Migration v8**: 为所有需要同步的表添加变更追踪

```sql
-- Migration v8: Change tracking for offline sync

-- 全局同步元数据表
CREATE TABLE IF NOT EXISTS sync_metadata (
    device_id TEXT PRIMARY KEY,              -- 本设备唯一 ID (UUID)
    last_sync_at TEXT,                       -- 上次成功同步时间
    sync_version INTEGER NOT NULL DEFAULT 0, -- 单调递增的同步版本号
    server_url TEXT,                         -- 远程服务器 URL
    created_at TEXT NOT NULL
);

-- 变更日志表（所有同步表共用）
CREATE TABLE IF NOT EXISTS sync_changelog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,                -- 来源表名
    row_id TEXT NOT NULL,                    -- 被修改行的主键
    operation TEXT NOT NULL,                 -- 'INSERT' | 'UPDATE' | 'DELETE'
    changed_columns TEXT,                    -- JSON: 变更的列名列表
    old_values TEXT,                         -- JSON: 变更前的值（UPDATE/DELETE）
    new_values TEXT,                         -- JSON: 变更后的值（INSERT/UPDATE）
    device_id TEXT NOT NULL,                 -- 产生变更的设备
    timestamp TEXT NOT NULL,                 -- ISO 8601 高精度时间戳
    sync_version INTEGER NOT NULL,           -- 本地同步版本号
    synced INTEGER NOT NULL DEFAULT 0        -- 0=未同步, 1=已同步
);

CREATE INDEX IF NOT EXISTS idx_changelog_unsynced
    ON sync_changelog(synced, sync_version);
CREATE INDEX IF NOT EXISTS idx_changelog_table
    ON sync_changelog(table_name, row_id);

-- 为需要同步的表添加 LWW 时间戳列
ALTER TABLE sessions ADD COLUMN lww_updated_at TEXT;
ALTER TABLE session_messages ADD COLUMN lww_updated_at TEXT;
ALTER TABLE memories ADD COLUMN lww_updated_at TEXT;
ALTER TABLE memory_blocks ADD COLUMN lww_updated_at TEXT;
ALTER TABLE mcp_servers ADD COLUMN lww_updated_at TEXT;
ALTER TABLE scheduled_tasks ADD COLUMN lww_updated_at TEXT;

-- Trigger 模板：自动记录变更到 changelog
-- (为每张同步表创建 INSERT/UPDATE/DELETE triggers)
```

**Trigger 策略**: 每张需要同步的表创建三个 trigger（AFTER INSERT / AFTER UPDATE / AFTER DELETE），自动写入 `sync_changelog`。

#### 2.3 同步协议

```
Desktop (Client)                          Server
    |                                        |
    |--- GET /api/sync/status -------------->|
    |<-- { server_version, last_client_sync }|
    |                                        |
    |--- POST /api/sync/pull --------------->|
    |    { since_version: N }                |
    |<-- { changes: [...], version: M }      |
    |                                        |
    |    [apply remote changes locally]      |
    |    [LWW: compare timestamps]           |
    |                                        |
    |--- POST /api/sync/push --------------->|
    |    { changes: [...], device_id }       |
    |<-- { applied: N, conflicts: [...] }    |
    |                                        |
    |--- POST /api/sync/ack ---------------->|
    |    { synced_version: M }               |
    |<-- { ok }                              |
```

**同步请求/响应格式**:

```rust
// 变更记录（传输格式）
pub struct SyncChange {
    pub table_name: String,
    pub row_id: String,
    pub operation: SyncOperation,        // Insert | Update | Delete
    pub data: serde_json::Value,         // 行数据 JSON
    pub timestamp: String,               // LWW 时间戳
    pub device_id: String,
    pub sync_version: u64,
}

// Pull 响应
pub struct SyncPullResponse {
    pub changes: Vec<SyncChange>,
    pub server_version: u64,
    pub has_more: bool,                  // 分页标记
}

// Push 请求
pub struct SyncPushRequest {
    pub changes: Vec<SyncChange>,
    pub device_id: String,
    pub client_version: u64,
}

// Push 响应
pub struct SyncPushResponse {
    pub applied: usize,
    pub conflicts: Vec<SyncConflict>,
    pub server_version: u64,
}

// 冲突记录
pub struct SyncConflict {
    pub table_name: String,
    pub row_id: String,
    pub client_value: serde_json::Value,
    pub server_value: serde_json::Value,
    pub resolution: ConflictResolution,  // ServerWins | ClientWins（LWW 按时间戳）
}
```

#### 2.4 冲突解决策略 (LWW)

```
对于同一行的并发修改：
  if client.timestamp > server.timestamp:
      server 接受 client 的值
  else:
      client 接受 server 的值（返回 conflict 通知）

特殊情况处理：
  - DELETE vs UPDATE: DELETE wins（墓碑机制）
  - INSERT 冲突（same PK）: LWW 按时间戳
  - session_messages: 追加 only，不修改，用 ID 去重
  - audit_logs: 不同步（各设备独立记录）
```

#### 2.5 同步范围决策

| 表 | 同步 | 理由 |
|----|------|------|
| `sessions` | ✅ | 跨设备访问会话 |
| `session_messages` | ✅ | 会话内容同步 |
| `memories` | ✅ | 长期记忆跨设备共享 |
| `memory_blocks` | ✅ | 工作记忆同步 |
| `mcp_servers` | ✅ | MCP 配置跨设备 |
| `scheduled_tasks` | ✅ | 定时任务配置 |
| `tool_executions` | ❌ | 执行记录是本地的 |
| `mcp_executions` | ❌ | 执行记录是本地的 |
| `mcp_logs` | ❌ | 日志是本地的 |
| `task_executions` | ❌ | 执行记录是本地的 |
| `audit_logs` | ❌ | 审计日志是本地的 |
| `byzantine_proposals` | ⚠️ | D1-P3 完成后考虑 |

### 3. 模块架构

```
crates/octo-engine/src/sync/
├── mod.rs                  # 模块导出
├── changelog.rs            # 变更日志管理（trigger 注册、changelog 读写）
├── protocol.rs             # 同步协议定义（SyncChange, SyncPullResponse 等）
├── lww.rs                  # LWW 冲突解决逻辑
├── client.rs               # 同步客户端（Desktop 端使用）
├── server.rs               # 同步服务端（集成到 octo-server）
└── config.rs               # 同步配置

crates/octo-server/src/api/sync.rs  # REST API 端点
crates/octo-desktop/src/sync.rs     # Desktop 端同步调度
```

### 4. 任务分解

| Task | 内容 | 估计 LOC |
|------|------|---------|
| D6-1 | DB Migration v8: sync_metadata + sync_changelog + LWW 列 + triggers | ~200 |
| D6-2 | `sync/changelog.rs`: 变更日志读写 + trigger 注册 | ~250 |
| D6-3 | `sync/protocol.rs`: 同步协议类型定义 | ~150 |
| D6-4 | `sync/lww.rs`: LWW 冲突解决 + 合并逻辑 | ~200 |
| D6-5 | `sync/server.rs`: 同步服务端逻辑 | ~300 |
| D6-6 | `sync/client.rs`: 同步客户端逻辑 | ~250 |
| D6-7 | `octo-server/src/api/sync.rs`: REST 端点 | ~150 |
| D6-8 | `octo-desktop/src/sync.rs`: Desktop 同步调度 | ~200 |
| D6-9 | 测试（20+ tests） | ~500 |

**总计**: ~2200 LOC，预计 5-8 天

### 5. 前置依赖

| 依赖 | 状态 | 说明 |
|------|------|------|
| D1 (Byzantine 共识) | ✅ D1-P1+P2 完成 | 共识数据可作为同步对象 |
| D5 (Tauri 自动更新) | ✅ 完成 | Desktop 分发机制就绪 |
| D1-P3 (共识持久化) | ⏳ 建议先完成 | 持久化后才有数据可同步 |
| 同步协议设计文档 | 📝 本文档 | 已包含协议设计 |

### 6. 风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| LWW 时间戳漂移 | 中 | 使用 NTP 同步 + 逻辑时钟混合方案 |
| 大量离线变更一次性同步 | 中 | 分页 pull/push（每次最多 1000 条） |
| FTS5 虚拟表不可直接同步 | 中 | 同步 memories 表后本地重建 FTS 索引 |
| DashMap 缓存与同步数据不一致 | 高 | 同步完成后清空缓存 + 重新加载 |
| 墓碑记录无限增长 | 低 | 定期清理已确认同步的 changelog（> 30 天） |

---

## D4: Let's Encrypt ACME 自动证书

### 1. 现状分析

#### TLS 基础设施现状

| 组件 | 状态 | 详情 |
|------|------|------|
| octo-server (Workbench) | ❌ 纯 HTTP | `TcpListener::bind()` 无 TLS |
| octo-platform-server | ❌ 纯 HTTP | 同上 |
| octo-cli Dashboard | ⚠️ 可选 TLS | `axum-server` + `rcgen` 自签名证书 |
| ACME 依赖 | ❌ 无 | 无 `acme` 或 `instant-acme` crate |
| 证书存储 | ❌ 无 | 无 certificates 表 |
| Secret 管理 | ❌ 未实现 | CLAUDE.md 描述了 `secret/` 模块但代码不存在 |

#### 现有加密依赖

```toml
# octo-engine 已有
sha2 = "0.10"           # SHA-256
hmac = "0.12"            # HMAC
aes-gcm = "0.10"         # AES-GCM 加密
argon2 = "0.5"           # 密码哈希
ed25519-dalek = "2"      # ED25519 签名
jsonwebtoken = "9"       # JWT
keyring = { version = "3", optional = true }  # 系统密钥链

# octo-cli 已有（可选）
axum-server = { version = "0.7", features = ["tls-rustls"], optional = true }
rcgen = { version = "0.13", optional = true }
```

#### 配置系统现状

```rust
// octo-server: ServerConfig
pub struct ServerConfig {
    pub host: String,           // 默认 127.0.0.1
    pub port: u16,              // 默认 3001
    pub cors_origins: Vec<String>,
    // 无 TLS 相关字段
}

// octo-platform-server: PlatformConfig
pub struct PlatformConfig {
    pub host: String,           // 默认 127.0.0.1
    pub port: u16,              // 默认 3002
    pub data_dir: PathBuf,
    // 无 TLS 相关字段
}
```

### 2. 架构决策分析

#### 方案对比

| 方案 | 优点 | 缺点 | 推荐度 |
|------|------|------|--------|
| **A: 内置 ACME** | 零外部依赖，一键部署 | 复杂度高 (~1200 LOC)，需维护 | ⭐⭐⭐ |
| **B: 反向代理** | 成熟稳定，TLS 卸载 | 额外组件（nginx/Caddy） | ⭐⭐⭐⭐ |
| **C: 混合方案** | 开发用自签名 + 生产用反向代理 | 两套配置 | ⭐⭐⭐⭐⭐ |

**推荐方案 C**: 混合方案

- **开发/本地**: 复用 `rcgen` 自签名证书（已在 octo-cli 实现）
- **生产部署**: 提供 Caddy/nginx 配置模板 + 内置 ACME 作为可选 feature
- **渐进式**: 先实现基础 TLS 支持，ACME 作为独立 feature flag

#### Challenge 类型选择

| 类型 | 适用场景 | 复杂度 | 推荐 |
|------|---------|--------|------|
| HTTP-01 | 公网 HTTP 可达 | 低 | ✅ 默认 |
| DNS-01 | 通配符证书，无公网 HTTP | 高（需 DNS API） | ⚠️ 可选 |
| TLS-ALPN-01 | 只有 443 端口 | 中 | ❌ 暂不支持 |

### 3. 实现方案

#### 3.1 TLS 配置扩展

```rust
// 扩展 ServerConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,                          // 是否启用 TLS
    pub cert_path: Option<PathBuf>,             // PEM 证书路径
    pub key_path: Option<PathBuf>,              // PEM 私钥路径
    pub domain: Option<String>,                 // 域名（ACME 用）
    pub auto_cert: bool,                        // 启用 ACME 自动证书
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcmeConfig {
    pub enabled: bool,
    pub email: String,                          // 联系邮箱
    pub directory_url: String,                  // ACME 目录 URL
    pub challenge_type: ChallengeType,          // Http01 | Dns01
    pub storage_path: PathBuf,                  // 证书存储目录
    pub renewal_days_before: u32,               // 过期前 N 天续期（默认 30）
}
```

#### 3.2 数据库 Migration v9（如 D1-P3 用 v7，D6 用 v8）

```sql
-- Migration v9: Certificate management

CREATE TABLE IF NOT EXISTS certificates (
    id TEXT PRIMARY KEY,
    domain TEXT NOT NULL UNIQUE,
    cert_pem TEXT NOT NULL,
    key_pem_encrypted TEXT NOT NULL,          -- AES-GCM 加密存储
    chain_pem TEXT,                           -- 中间证书链
    issued_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_renewed_at TEXT,
    renewal_attempts INTEGER DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',    -- active | expired | pending | failed
    acme_account_id TEXT
);

CREATE TABLE IF NOT EXISTS acme_accounts (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    account_url TEXT,
    private_key_encrypted TEXT NOT NULL,      -- AES-GCM 加密
    created_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
);

CREATE INDEX IF NOT EXISTS idx_certificates_domain
    ON certificates(domain);
CREATE INDEX IF NOT EXISTS idx_certificates_expires
    ON certificates(expires_at);
```

#### 3.3 模块架构

```
crates/octo-engine/src/tls/
├── mod.rs                  # 模块导出 + feature gate
├── config.rs               # TlsConfig, AcmeConfig
├── self_signed.rs          # 自签名证书生成（从 octo-cli 提取）
├── acme_client.rs          # ACME 协议客户端（feature = "acme"）
├── challenge.rs            # HTTP-01 challenge handler
├── cert_store.rs           # 证书存储（SQLite + 文件系统）
└── auto_renew.rs           # 自动续期任务（集成 scheduler）
```

**Feature Gate**:
```toml
[features]
tls = ["rustls", "rustls-pemfile"]
acme = ["tls", "instant-acme", "x509-parser"]
```

#### 3.4 Server 集成

```rust
// main.rs 修改
async fn start_server(cfg: &AppConfig) -> Result<()> {
    let app = build_router(state);

    if cfg.tls.enabled {
        let tls_config = load_tls_config(&cfg.tls)?;
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        // 使用 rustls + axum 集成
        axum_serve_tls(listener, app, tls_config).await?;
    } else {
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    }
}
```

### 4. 任务分解

| Task | 内容 | 估计 LOC |
|------|------|---------|
| D4-1 | `tls/config.rs`: TLS + ACME 配置结构 | ~100 |
| D4-2 | `tls/self_signed.rs`: 从 octo-cli 提取自签名逻辑 | ~120 |
| D4-3 | 扩展 `ServerConfig`: 添加 TLS 字段 + 环境变量 | ~80 |
| D4-4 | Server TLS 集成: rustls + axum | ~150 |
| D4-5 | DB Migration v9: certificates + acme_accounts 表 | ~60 |
| D4-6 | `tls/cert_store.rs`: 证书 CRUD + 加密存储 | ~200 |
| D4-7 | `tls/acme_client.rs`: ACME 协议实现（feature） | ~400 |
| D4-8 | `tls/challenge.rs`: HTTP-01 challenge handler | ~150 |
| D4-9 | `tls/auto_renew.rs`: 自动续期（集成 scheduler） | ~100 |
| D4-10 | 部署配置模板: Caddy/nginx 反向代理 | ~50 |
| D4-11 | 测试（12+ tests） | ~300 |

**总计**: ~1710 LOC，预计 3-5 天

### 5. 前置依赖

| 依赖 | 状态 | 说明 |
|------|------|------|
| 公网域名 | ❌ 需要 | ACME 验证需要域名解析 |
| 公网 IP | ❌ 需要 | HTTP-01 challenge 需要公网可达 |
| 生产部署环境 | ❌ 需要 | 本地开发环境无法测试 ACME |
| Secret 管理模块 | ❌ 需要 | 证书私钥加密存储 |

### 6. 风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| ACME 速率限制 | 中 | 使用 staging 环境测试，生产环境缓存证书 |
| 证书续期失败 | 高 | 到期前 30 天开始尝试，失败后指数退避重试 |
| 私钥安全 | 高 | AES-GCM 加密存储，可选 keyring 集成 |
| axum 0.8 TLS 兼容性 | 中 | 使用 `axum-server` 0.7（已在 CLI 验证） |
| DNS-01 API 多样性 | 低 | 初期仅支持 HTTP-01，DNS-01 作为后续迭代 |

---

## 实施优先级建议

### 推荐执行顺序

```
Phase 1 (可立即开始):
  D1-P3 — Byzantine 共识持久化
  ├── 前置条件全部满足
  ├── 复杂度中等 (~1030 LOC)
  └── 为 D6 提供可同步的共识数据

Phase 2 (D1-P3 完成后):
  D6-基础 — 变更追踪 + 同步协议
  ├── Migration v8 + changelog 机制
  ├── LWW 冲突解决核心
  └── 先做 server 端，Desktop 集成后续

Phase 3 (需基础设施):
  D4-基础 — TLS 支持
  ├── 自签名证书（开发环境）
  ├── 配置扩展 + Server 集成
  └── ACME 作为独立 feature 后续添加
```

### 可并行的工作

```
D1-P3 (共识持久化)  ←→  D4-基础 (TLS 配置 + 自签名)
         ↓
D6 (离线同步)       ←→  D4-ACME (ACME 自动证书)
```

- D1-P3 和 D4-基础 可以并行（无代码依赖）
- D6 依赖 D1-P3（需要持久化的共识数据才有同步意义）
- D4-ACME 可以独立于 D6 开发（但需要生产环境验证）

---

## 附录: 关键代码引用

### 现有核心接口

| 接口 | 文件 | 说明 |
|------|------|------|
| `CollaborationStore` trait | `collaboration/persistence.rs` | 持久化抽象层 |
| `InMemoryCollaborationStore` | `collaboration/persistence.rs` | 当前唯一实现 |
| `ByzantineProposal` | `collaboration/consensus.rs` | 共识数据结构 |
| `ViewChangeTracker` | `collaboration/consensus.rs` | 视图变更追踪 |
| `ConsensusKeypair` / `SignedMessage` | `collaboration/crypto.rs` | ED25519 签名 |
| `CollaborationMessage` | `collaboration/channel.rs` | 消息协议（含 Signed 变体） |
| `Database::migrate()` | `db/connection.rs` | Migration 执行框架 |
| `SessionStore` trait | `session/mod.rs` | 会话存储抽象 |
| `MemoryStore` trait | `memory/mod.rs` | 记忆存储抽象 |
| `ServerConfig` | `octo-server/src/config.rs` | 服务器配置 |
| `PlatformConfig` | `octo-platform-server/src/lib.rs` | 平台配置 |

### 数据库 Migration 版本规划

| 版本 | 内容 | 状态 |
|------|------|------|
| v1-v6 | 现有 schema（sessions, memories, tools, MCP, scheduler, audit） | ✅ 已部署 |
| **v7** | Byzantine consensus tables（D1-P3） | 📋 规划中 |
| **v8** | Sync changelog + LWW tracking（D6） | 📋 规划中 |
| **v9** | Certificate management（D4） | 📋 规划中 |
