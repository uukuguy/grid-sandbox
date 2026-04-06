# Phase BF — L2 统一资产层 + L1 抽象机制 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 构建 EAASP L2 统一资产层（Skill Registry + MCP Orchestrator），扩展 L1 Runtime 协议支持 L2 资产拉取，在 certifier 中实现 Mock L3 RuntimeSelector + 盲盒对比。

**Architecture:**
- L2 Skill Registry：REST API（Axum），SQLite 元数据 + 文件系统内容 + Git 版本追溯，独立部署
- L2 MCP Orchestrator：YAML 配置驱动，Shared 模式子进程管理，REST API
- L1 协议扩展：SessionPayload 新增 skill_ids + skill_registry_url，Runtime initialize 时从 L2 REST 拉取 Skill 内容
- Mock L3 RuntimeSelector：certifier 扩展，运行时池管理 + 盲盒并行执行 + 用户评分

**Tech Stack:** Rust, Axum 0.8, SQLite (rusqlite), git2, tonic 0.12, tokio

---

## 设计决策汇总

| # | 决策 | 理由 |
|---|------|------|
| BF-KD1 | L2 存储：SQLite 元数据 + 文件系统内容 + Git 版本追溯 | 与 grid-engine 统一技术栈，三层各司其职 |
| BF-KD2 | L1 ↔ L2 Skill 通信：REST（L1 拉取内容）| Agent 不直连 L2，L3 下发 skill_ids，L1 从 L2 REST 拉取 |
| BF-KD3 | L2 实现语言：Rust（独立 binary） | 复用 grid-types，rmcp 已验证，单 binary 部署 |
| BF-KD4 | RuntimeSelector 属于 L3，BF 在 certifier mock | L3 未来用 Python/TS 实现，certifier mock 是验证基线 |
| BF-KD5 | 盲盒模式：用户主动开启，并行执行，匿名评分 | BF 实验性功能，成本翻倍，用户自愿触发 |
| BF-KD6 | L2 三个独立服务：Skill Registry / MCP Orchestrator / Ontology Service | 三种资产本质不同，物理分离 |
| BF-KD7 | MCP Orchestrator 对 L1 不直连 | L3 从 Orchestrator 获取连接信息，筛选后下发给 L1 |
| BF-KD8 | Agent 不需要 skill_search | L3 下发可用 skill 列表，L1 从 L2 拉取内容，Agent 内部使用 |
| BF-KD9 | L2 Skill Registry = REST only（去掉 MCP Server 接口） | Agent 不直连 L2，纯 REST 足够 |
| BF-KD10 | MCP Server 运行模式：Shared/PerSession/OnDemand | BF 只实现 Shared（子进程），PerSession 留 BH |
| BF-KD11 | Skill 获取 = 预加载 + 按需发现（C 方案） | 但 BF 阶段只实现预加载，按需发现留 L3 治理后 |
| BF-KD12 | L1 Skill 转换是 Runtime 内部的事 | Grid → SkillDefinition；CC → .claude/skills/；L2 只交付 SKILL.md |

## 数据流全景

```
┌──────────────────────────────────────────────────────────────────┐
│ L2 统一资产层（逻辑统一，物理分离）                                  │
│                                                                  │
│  Skill Registry (REST)        MCP Orchestrator (REST)            │
│  ┌─────────────────┐          ┌──────────────────────┐          │
│  │ GET /skills/{id} │          │ GET /mcp-servers     │          │
│  │ → SKILL.md 内容  │          │ POST /mcp-servers/   │          │
│  │                  │          │   start/stop/health  │          │
│  │ SQLite + fs + git│          │                      │          │
│  └────────┬─────────┘          │  ┌──────┐ ┌──────┐  │          │
│           │                    │  │erp   │ │crm   │  │          │
│           │                    │  │mcp   │ │mcp   │  │          │
│           │                    │  └──┬───┘ └──┬───┘  │          │
│           │                    └─────┼────────┼──────┘          │
└───────────┼──────────────────────────┼────────┼─────────────────┘
            │ REST                     │        │ MCP 直连
            │                          │        │
┌───────────┼──────────────────────────┼────────┼─────────────────┐
│ certifier mock-L3                    │        │                 │
│ ┌─────────┴──────┐                   │        │                 │
│ │ RuntimeSelector │ ← 策略筛选        │        │                 │
│ │ skill_ids      │                   │        │                 │
│ │ mcp_servers    │                   │        │                 │
│ └────────┬───────┘                   │        │                 │
└──────────┼───────────────────────────┼────────┼─────────────────┘
           │ gRPC Initialize           │        │
           │ (SessionPayload)          │        │
           ▼                           │        │
┌──────────────────────────────────────┼────────┼─────────────────┐
│ L1 Runtime                           │        │                 │
│ ┌────────────────┐                   │        │                 │
│ │ initialize():  │                   │        │                 │
│ │  skill_ids →   │─ REST GET ────────┘        │                 │
│ │  拉取 SKILL.md │                            │                 │
│ │  → load_skill()│                            │                 │
│ │                │                            │                 │
│ │ connect_mcp(): │                            │                 │
│ │  mcp_servers → │─ MCP connect ──────────────┘                 │
│ │  直连 MCP Srv  │                                              │
│ └────────────────┘                                              │
│ ┌────────────────┐                                              │
│ │ Agent (内部)    │ ← 使用 skills + MCP tools 自主执行            │
│ └────────────────┘                                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## 项目结构变更

```
新增:
  tools/eaasp-skill-registry/       # L2 Skill Registry (REST API server)
  tools/eaasp-mcp-orchestrator/     # L2 MCP Orchestrator (REST API + 子进程管理)
  tools/eaasp-certifier/src/runtime_pool.rs   # 运行时池管理
  tools/eaasp-certifier/src/blindbox.rs       # 盲盒对比
  tools/eaasp-certifier/src/selector.rs       # Mock RuntimeSelector

修改:
  proto/eaasp/runtime/v1/runtime.proto        # SessionPayload 新增字段
  crates/grid-runtime/src/contract.rs         # Rust SessionPayload 同步
  crates/grid-runtime/src/service.rs          # gRPC type conversion 同步
  crates/grid-runtime/src/harness.rs          # initialize 拉取 L2 skill 内容
  Cargo.toml                                  # workspace members 新增
  Makefile                                    # 新增 targets
```

---

## Wave 分解

| Wave | 内容 | 产出 | 依赖 |
|------|------|------|------|
| **W1** | 协议扩展 + SessionPayload L2 字段 | proto v1.3 + contract.rs + service.rs 同步 | 无 |
| **W2** | L2 Skill Registry crate | REST API server + SQLite + 文件系统 + Git | W1（SkillContent 类型） |
| **W3** | L2 MCP Orchestrator crate | YAML 配置 + Shared 子进程管理 + REST API | 无 |
| **W4** | L1 Runtime L2 集成 | GridHarness initialize 从 L2 拉取 + load_skill | W1, W2 |
| **W5** | Mock L3 RuntimeSelector + 运行时池 | certifier 扩展：pool + selector | W1 |
| **W6** | 盲盒对比 | certifier blindbox: 并行执行 + 匿名展示 + 评分 | W5 |
| **W7** | 集成验证 + 文档 | 端到端测试 + 设计文档 + Makefile targets | W1-W6 |

### 并行策略

```
W1 ──────────→ W1 完成
  │
  ├── W2 ────→ W2 完成 ──┐
  │                       ├── W4 ──→ W4 完成
  ├── W3 ────→ W3 完成    │
  │                       │
  └── W5 ────→ W5 完成 ──┤
                          ├── W6 ──→ W6 完成
                          │
                          └── W7 ──→ W7 完成（需 W2+W3+W4+W5+W6）
```

---

## Task 1: Wave 1 — 协议扩展 (SessionPayload L2 字段)

**Files:**
- Modify: `proto/eaasp/runtime/v1/runtime.proto:75-84`
- Modify: `crates/grid-runtime/src/contract.rs:100-116`
- Modify: `crates/grid-runtime/src/service.rs:29-52`
- Test: `crates/grid-runtime/tests/contract_l2_fields.rs`

### Step 1: 写 proto SessionPayload 新增字段的测试

```rust
// crates/grid-runtime/tests/contract_l2_fields.rs
use grid_runtime::contract::SessionPayload;

#[test]
fn session_payload_l2_fields_roundtrip() {
    let payload = SessionPayload {
        user_id: "user-1".into(),
        user_role: "developer".into(),
        org_unit: "engineering".into(),
        managed_hooks_json: None,
        quotas: Default::default(),
        context: Default::default(),
        hook_bridge_url: None,
        telemetry_endpoint: None,
        // New L2 fields
        skill_ids: vec!["order-management".into(), "logistics".into()],
        skill_registry_url: Some("http://l2-skill:8080".into()),
        allowed_skill_search: false,
        skill_search_scope: vec![],
    };
    let json = serde_json::to_string(&payload).unwrap();
    let restored: SessionPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.skill_ids.len(), 2);
    assert_eq!(restored.skill_registry_url.as_deref(), Some("http://l2-skill:8080"));
    assert!(!restored.allowed_skill_search);
}

#[test]
fn session_payload_l2_fields_default_empty() {
    let payload = SessionPayload {
        user_id: "u".into(),
        user_role: "r".into(),
        org_unit: "o".into(),
        managed_hooks_json: None,
        quotas: Default::default(),
        context: Default::default(),
        hook_bridge_url: None,
        telemetry_endpoint: None,
        skill_ids: vec![],
        skill_registry_url: None,
        allowed_skill_search: false,
        skill_search_scope: vec![],
    };
    assert!(payload.skill_ids.is_empty());
    assert!(payload.skill_registry_url.is_none());
}
```

### Step 2: 运行测试确认失败

Run: `cargo test -p grid-runtime contract_l2_fields -- --test-threads=1`
Expected: FAIL — `skill_ids` field does not exist

### Step 3: 更新 proto

在 `proto/eaasp/runtime/v1/runtime.proto` 的 `SessionPayload` message 末尾新增:

```protobuf
message SessionPayload {
  // ... existing fields 1-8 ...
  repeated string skill_ids = 9;          // L3-selected skill IDs to preload
  string skill_registry_url = 10;         // L2 Skill Registry REST endpoint
  bool allowed_skill_search = 11;         // Whether agent can search for more skills
  repeated string skill_search_scope = 12; // Allowed search scope patterns (e.g. "org/erp/*")
}
```

### Step 4: 更新 contract.rs SessionPayload

在 `crates/grid-runtime/src/contract.rs` `SessionPayload` struct 新增:

```rust
pub struct SessionPayload {
    // ... existing fields ...
    /// L3-selected skill IDs to preload from L2 Skill Registry.
    pub skill_ids: Vec<String>,
    /// L2 Skill Registry REST endpoint URL.
    pub skill_registry_url: Option<String>,
    /// Whether the agent is allowed to search for additional skills at runtime.
    pub allowed_skill_search: bool,
    /// Allowed search scope patterns (e.g. "org/erp/*").
    pub skill_search_scope: Vec<String>,
}
```

### Step 5: 更新 service.rs type conversion

在 `crates/grid-runtime/src/service.rs` `to_session_payload` 函数新增字段映射:

```rust
fn to_session_payload(p: proto::SessionPayload) -> contract::SessionPayload {
    contract::SessionPayload {
        // ... existing fields ...
        skill_ids: p.skill_ids,
        skill_registry_url: if p.skill_registry_url.is_empty() {
            None
        } else {
            Some(p.skill_registry_url)
        },
        allowed_skill_search: p.allowed_skill_search,
        skill_search_scope: p.skill_search_scope,
    }
}
```

### Step 6: 修复所有使用 SessionPayload 的地方

在 harness.rs、verifier.rs、mock_l3.rs 等文件中所有构造 SessionPayload 的地方补上新字段默认值:
```rust
skill_ids: vec![],
skill_registry_url: None,
allowed_skill_search: false,
skill_search_scope: vec![],
```

### Step 7: 运行测试确认通过

Run: `cargo test -p grid-runtime -- --test-threads=1`
Expected: ALL PASS

### Step 8: Commit

```bash
git add proto/eaasp/runtime/v1/runtime.proto crates/grid-runtime/
git commit -m "feat(eaasp): add L2 skill fields to SessionPayload (proto v1.3)"
```

---

## Task 2: Wave 2 — L2 Skill Registry crate

**Files:**
- Create: `tools/eaasp-skill-registry/Cargo.toml`
- Create: `tools/eaasp-skill-registry/src/main.rs`
- Create: `tools/eaasp-skill-registry/src/lib.rs`
- Create: `tools/eaasp-skill-registry/src/store.rs`
- Create: `tools/eaasp-skill-registry/src/git_backend.rs`
- Create: `tools/eaasp-skill-registry/src/promotion.rs`
- Create: `tools/eaasp-skill-registry/src/routes.rs`
- Create: `tools/eaasp-skill-registry/src/models.rs`
- Test: `tools/eaasp-skill-registry/tests/store_test.rs`
- Test: `tools/eaasp-skill-registry/tests/api_test.rs`
- Modify: `Cargo.toml` (workspace members)

### Step 1: 创建 crate 骨架 + 写 store 测试

Cargo.toml:
```toml
[package]
name = "eaasp-skill-registry"
edition.workspace = true
version.workspace = true
description = "EAASP L2 Skill Registry — REST API for skill asset management"

[dependencies]
axum = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = "0.9"
rusqlite = { workspace = true }
tokio-rusqlite = { workspace = true }
git2 = "0.19"
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
clap = { version = "4", features = ["derive"] }
tower-http = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
reqwest = { workspace = true }
tempfile = "3"
```

models.rs — 核心数据模型:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Skill promotion status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillStatus {
    Draft,
    Tested,
    Reviewed,
    Production,
}

/// Skill metadata stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub id: String,          // e.g. "org/order-management"
    pub name: String,
    pub description: String,
    pub version: String,     // semver
    pub status: SkillStatus,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Full skill content (metadata + SKILL.md body).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContent {
    pub meta: SkillMeta,
    pub frontmatter_yaml: String,
    pub prose: String,
}

/// Skill version entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub version: String,
    pub status: SkillStatus,
    pub created_at: DateTime<Utc>,
    pub git_commit: Option<String>,
}

/// Draft submission request.
#[derive(Debug, Deserialize)]
pub struct SubmitDraftRequest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub frontmatter_yaml: String,
    pub prose: String,
}

/// Promotion request.
#[derive(Debug, Deserialize)]
pub struct PromoteRequest {
    pub target_status: SkillStatus,
}

/// Search query parameters.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub tags: Option<String>,  // comma-separated
    pub status: Option<String>,
    pub limit: Option<usize>,
}
```

store.rs 测试:
```rust
// tools/eaasp-skill-registry/tests/store_test.rs
use eaasp_skill_registry::models::*;
use eaasp_skill_registry::store::SkillStore;
use tempfile::TempDir;

#[tokio::test]
async fn store_submit_and_read() {
    let tmp = TempDir::new().unwrap();
    let store = SkillStore::open(tmp.path()).await.unwrap();

    let draft = SubmitDraftRequest {
        id: "org/order-mgmt".into(),
        name: "Order Management".into(),
        description: "Order processing skill".into(),
        version: "1.0.0".into(),
        author: Some("test".into()),
        tags: vec!["erp".into(), "order".into()],
        frontmatter_yaml: "name: order-mgmt\n".into(),
        prose: "# Order Management\nProcess orders...".into(),
    };

    store.submit_draft(draft).await.unwrap();

    let content = store.read_skill("org/order-mgmt", None).await.unwrap();
    assert_eq!(content.meta.name, "Order Management");
    assert_eq!(content.meta.status, SkillStatus::Draft);
    assert!(content.prose.contains("Process orders"));
}

#[tokio::test]
async fn store_search_by_tags() {
    let tmp = TempDir::new().unwrap();
    let store = SkillStore::open(tmp.path()).await.unwrap();

    // Submit two skills
    store.submit_draft(SubmitDraftRequest {
        id: "org/skill-a".into(), name: "A".into(), description: "Desc A".into(),
        version: "1.0.0".into(), author: None, tags: vec!["erp".into()],
        frontmatter_yaml: String::new(), prose: "A".into(),
    }).await.unwrap();

    store.submit_draft(SubmitDraftRequest {
        id: "org/skill-b".into(), name: "B".into(), description: "Desc B".into(),
        version: "1.0.0".into(), author: None, tags: vec!["crm".into()],
        frontmatter_yaml: String::new(), prose: "B".into(),
    }).await.unwrap();

    let results = store.search(Some("erp"), None, 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "org/skill-a");
}

#[tokio::test]
async fn store_promote_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let store = SkillStore::open(tmp.path()).await.unwrap();

    store.submit_draft(SubmitDraftRequest {
        id: "org/skill-c".into(), name: "C".into(), description: "Desc C".into(),
        version: "1.0.0".into(), author: None, tags: vec![],
        frontmatter_yaml: String::new(), prose: "C".into(),
    }).await.unwrap();

    // draft → tested
    store.promote("org/skill-c", "1.0.0", SkillStatus::Tested).await.unwrap();
    let content = store.read_skill("org/skill-c", None).await.unwrap();
    assert_eq!(content.meta.status, SkillStatus::Tested);

    // tested → reviewed
    store.promote("org/skill-c", "1.0.0", SkillStatus::Reviewed).await.unwrap();

    // reviewed → production
    store.promote("org/skill-c", "1.0.0", SkillStatus::Production).await.unwrap();
    let content = store.read_skill("org/skill-c", None).await.unwrap();
    assert_eq!(content.meta.status, SkillStatus::Production);
}
```

### Step 2: 运行测试确认失败

Run: `cargo test -p eaasp-skill-registry -- --test-threads=1`
Expected: FAIL — module not found

### Step 3: 实现 store.rs

```rust
// tools/eaasp-skill-registry/src/store.rs

use std::path::{Path, PathBuf};
use anyhow::{Context, Result, bail};
use chrono::Utc;
use rusqlite::params;
use tokio_rusqlite::Connection;
use tracing::info;

use crate::models::*;

pub struct SkillStore {
    db: Connection,
    base_dir: PathBuf,
}

impl SkillStore {
    pub async fn open(base_dir: &Path) -> Result<Self> {
        let db_path = base_dir.join("registry.db");
        let skills_dir = base_dir.join("skills");
        std::fs::create_dir_all(&skills_dir)?;

        let db = Connection::open(&db_path).await?;

        db.call(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS skills (
                    id TEXT NOT NULL,
                    version TEXT NOT NULL,
                    name TEXT NOT NULL,
                    description TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'draft',
                    author TEXT,
                    tags TEXT NOT NULL DEFAULT '[]',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY (id, version)
                );
                CREATE INDEX IF NOT EXISTS idx_skills_tags ON skills(tags);
                CREATE INDEX IF NOT EXISTS idx_skills_status ON skills(status);",
            )?;
            Ok(())
        })
        .await?;

        Ok(Self {
            db,
            base_dir: base_dir.to_path_buf(),
        })
    }

    pub async fn submit_draft(&self, req: SubmitDraftRequest) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(&req.tags)?;

        // Write SKILL.md to filesystem
        let skill_dir = self.base_dir.join("skills").join(&req.id).join(&req.version);
        std::fs::create_dir_all(&skill_dir)?;
        let skill_md = if req.frontmatter_yaml.is_empty() {
            req.prose.clone()
        } else {
            format!("---\n{}---\n\n{}", req.frontmatter_yaml, req.prose)
        };
        std::fs::write(skill_dir.join("SKILL.md"), &skill_md)?;

        // Insert metadata into SQLite
        let id = req.id.clone();
        let version = req.version.clone();
        let name = req.name.clone();
        let description = req.description.clone();
        let author = req.author.clone();
        let now2 = now.clone();

        self.db
            .call(move |conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO skills (id, version, name, description, status, author, tags, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 'draft', ?5, ?6, ?7, ?8)",
                    params![id, version, name, description, author, tags_json, now, now2],
                )?;
                Ok(())
            })
            .await?;

        info!(id = %req.id, version = %req.version, "Skill draft submitted");
        Ok(())
    }

    pub async fn read_skill(&self, id: &str, version: Option<&str>) -> Result<SkillContent> {
        let id_owned = id.to_string();
        let version_owned = version.map(|v| v.to_string());

        let meta = self
            .db
            .call(move |conn| {
                let row = if let Some(ver) = &version_owned {
                    conn.query_row(
                        "SELECT id, version, name, description, status, author, tags, created_at, updated_at
                         FROM skills WHERE id = ?1 AND version = ?2",
                        params![id_owned, ver],
                        |row| row_to_meta(row),
                    )
                } else {
                    conn.query_row(
                        "SELECT id, version, name, description, status, author, tags, created_at, updated_at
                         FROM skills WHERE id = ?1 ORDER BY created_at DESC LIMIT 1",
                        params![id_owned],
                        |row| row_to_meta(row),
                    )
                };
                row.map_err(|e| e.into())
            })
            .await?;

        // Read SKILL.md from filesystem
        let skill_path = self
            .base_dir
            .join("skills")
            .join(&meta.id)
            .join(&meta.version)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&skill_path)
            .with_context(|| format!("Reading {}", skill_path.display()))?;

        let (frontmatter_yaml, prose) = parse_skill_md(&content);

        Ok(SkillContent {
            meta,
            frontmatter_yaml,
            prose,
        })
    }

    pub async fn search(&self, tag: Option<&str>, query: Option<&str>, limit: usize) -> Result<Vec<SkillMeta>> {
        let tag_owned = tag.map(|t| t.to_string());
        let query_owned = query.map(|q| q.to_string());

        self.db
            .call(move |conn| {
                let mut sql = "SELECT id, version, name, description, status, author, tags, created_at, updated_at FROM skills WHERE 1=1".to_string();
                let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

                if let Some(ref tag) = tag_owned {
                    sql.push_str(" AND tags LIKE ?");
                    param_values.push(Box::new(format!("%\"{}\"%", tag)));
                }
                if let Some(ref q) = query_owned {
                    sql.push_str(" AND (name LIKE ? OR description LIKE ?)");
                    let pattern = format!("%{}%", q);
                    param_values.push(Box::new(pattern.clone()));
                    param_values.push(Box::new(pattern));
                }
                sql.push_str(&format!(" ORDER BY updated_at DESC LIMIT {}", limit));

                let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt
                    .query_map(params_refs.as_slice(), |row| row_to_meta(row))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
            .map_err(Into::into)
    }

    pub async fn promote(&self, id: &str, version: &str, target: SkillStatus) -> Result<()> {
        let id_owned = id.to_string();
        let version_owned = version.to_string();
        let status_str = serde_json::to_value(&target)?.as_str().unwrap().to_string();
        let now = Utc::now().to_rfc3339();

        self.db
            .call(move |conn| {
                let affected = conn.execute(
                    "UPDATE skills SET status = ?1, updated_at = ?2 WHERE id = ?3 AND version = ?4",
                    params![status_str, now, id_owned, version_owned],
                )?;
                if affected == 0 {
                    return Err(rusqlite::Error::QueryReturnedNoRows.into());
                }
                Ok(())
            })
            .await?;

        info!(id = %id, version = %version, status = %status_str, "Skill promoted");
        Ok(())
    }

    pub async fn list_versions(&self, id: &str) -> Result<Vec<SkillVersion>> {
        let id_owned = id.to_string();
        self.db
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT version, status, created_at FROM skills WHERE id = ?1 ORDER BY created_at DESC",
                )?;
                let rows = stmt
                    .query_map(params![id_owned], |row| {
                        Ok(SkillVersion {
                            version: row.get(0)?,
                            status: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(SkillStatus::Draft),
                            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now()),
                            git_commit: None,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
            .map_err(Into::into)
    }
}

fn row_to_meta(row: &rusqlite::Row) -> rusqlite::Result<SkillMeta> {
    Ok(SkillMeta {
        id: row.get(0)?,
        name: row.get(2)?,
        description: row.get(3)?,
        version: row.get(1)?,
        status: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(SkillStatus::Draft),
        author: row.get(5)?,
        tags: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

fn parse_skill_md(content: &str) -> (String, String) {
    if content.starts_with("---\n") {
        if let Some(end) = content[4..].find("\n---\n") {
            let frontmatter = content[4..4 + end].to_string();
            let prose = content[4 + end + 5..].trim_start().to_string();
            return (frontmatter, prose);
        }
    }
    (String::new(), content.to_string())
}
```

### Step 4: 实现 routes.rs (REST API)

```rust
// tools/eaasp-skill-registry/src/routes.rs

use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};

use crate::models::*;
use crate::store::SkillStore;

pub fn router(store: Arc<SkillStore>) -> Router {
    Router::new()
        .route("/skills/{id}/content", get(read_skill))
        .route("/skills/{id}/versions", get(list_versions))
        .route("/skills/search", get(search_skills))
        .route("/skills/draft", post(submit_draft))
        .route("/skills/{id}/promote/{version}", post(promote))
        .route("/health", get(health))
        .with_state(store)
}

async fn read_skill(
    State(store): State<Arc<SkillStore>>,
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<SkillContent>, StatusCode> {
    let version = params.get("version").map(|v| v.as_str());
    store
        .read_skill(&id, version)
        .await
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn list_versions(
    State(store): State<Arc<SkillStore>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<SkillVersion>>, StatusCode> {
    store
        .list_versions(&id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn search_skills(
    State(store): State<Arc<SkillStore>>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<SkillMeta>>, StatusCode> {
    let limit = q.limit.unwrap_or(20);
    store
        .search(q.tags.as_deref(), q.q.as_deref(), limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn submit_draft(
    State(store): State<Arc<SkillStore>>,
    Json(req): Json<SubmitDraftRequest>,
) -> Result<StatusCode, StatusCode> {
    store
        .submit_draft(req)
        .await
        .map(|_| StatusCode::CREATED)
        .map_err(|_| StatusCode::BAD_REQUEST)
}

async fn promote(
    State(store): State<Arc<SkillStore>>,
    Path((id, version)): Path<(String, String)>,
    Json(req): Json<PromoteRequest>,
) -> Result<StatusCode, StatusCode> {
    store
        .promote(&id, &version, req.target_status)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|_| StatusCode::BAD_REQUEST)
}

async fn health() -> &'static str {
    "ok"
}
```

### Step 5: 实现 main.rs

```rust
// tools/eaasp-skill-registry/src/main.rs

use std::sync::Arc;
use clap::Parser;
use tracing::info;

#[derive(Parser)]
#[command(name = "eaasp-skill-registry")]
#[command(about = "EAASP L2 Skill Registry — REST API server")]
struct Cli {
    /// Data directory (SQLite + skills files)
    #[arg(short, long, default_value = "./data/skill-registry")]
    data_dir: String,

    /// Listen port
    #[arg(short, long, default_value = "8081")]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("eaasp_skill_registry=info")
        .init();

    let cli = Cli::parse();
    let store = Arc::new(
        eaasp_skill_registry::store::SkillStore::open(std::path::Path::new(&cli.data_dir)).await?,
    );

    let app = eaasp_skill_registry::routes::router(store);
    let addr = format!("0.0.0.0:{}", cli.port);
    info!(addr = %addr, "Skill Registry starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### Step 6: 实现 lib.rs

```rust
pub mod models;
pub mod routes;
pub mod store;
pub mod git_backend;
```

### Step 7: 实现 git_backend.rs (基础版)

```rust
// tools/eaasp-skill-registry/src/git_backend.rs

use std::path::Path;
use anyhow::Result;
use git2::{Repository, Signature};
use tracing::info;

pub struct GitBackend {
    repo: Repository,
}

impl GitBackend {
    pub fn open_or_init(path: &Path) -> Result<Self> {
        let repo = if path.join(".git").exists() {
            Repository::open(path)?
        } else {
            let repo = Repository::init(path)?;
            // Initial commit
            let sig = Signature::now("eaasp-registry", "registry@eaasp.local")?;
            let tree_id = repo.index()?.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;
            repo
        };
        Ok(Self { repo })
    }

    pub fn commit_change(&self, message: &str) -> Result<String> {
        let sig = Signature::now("eaasp-registry", "registry@eaasp.local")?;
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let head = self.repo.head()?.peel_to_commit()?;
        let oid = self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])?;
        let short = &oid.to_string()[..8];
        info!(commit = %short, message = %message, "Git commit created");
        Ok(short.to_string())
    }
}
```

### Step 8: 运行测试

Run: `cargo test -p eaasp-skill-registry -- --test-threads=1`
Expected: ALL PASS

### Step 9: Commit

```bash
git add -f tools/eaasp-skill-registry/ Cargo.toml
git commit -m "feat(eaasp): add L2 Skill Registry crate (REST API + SQLite + Git)"
```

---

## Task 3: Wave 3 — L2 MCP Orchestrator crate

**Files:**
- Create: `tools/eaasp-mcp-orchestrator/Cargo.toml`
- Create: `tools/eaasp-mcp-orchestrator/src/main.rs`
- Create: `tools/eaasp-mcp-orchestrator/src/lib.rs`
- Create: `tools/eaasp-mcp-orchestrator/src/config.rs`
- Create: `tools/eaasp-mcp-orchestrator/src/manager.rs`
- Create: `tools/eaasp-mcp-orchestrator/src/routes.rs`
- Test: `tools/eaasp-mcp-orchestrator/tests/manager_test.rs`

### Step 1: 写 manager 测试

```rust
// tools/eaasp-mcp-orchestrator/tests/manager_test.rs
use eaasp_mcp_orchestrator::config::{McpServerDef, RunMode};
use eaasp_mcp_orchestrator::manager::McpManager;

#[tokio::test]
async fn manager_load_config_and_list() {
    let config = vec![
        McpServerDef {
            name: "erp-mcp".into(),
            command: "echo".into(),
            args: vec!["hello".into()],
            transport: "streamable-http".into(),
            port: 8090,
            mode: RunMode::Shared,
            tags: vec!["erp".into()],
            env: Default::default(),
            health_endpoint: "/health".into(),
        },
    ];
    let mgr = McpManager::new(config);
    let servers = mgr.list_servers().await;
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "erp-mcp");
    assert!(!servers[0].running);
}

#[tokio::test]
async fn manager_start_stop_shared() {
    let config = vec![
        McpServerDef {
            name: "test-echo".into(),
            command: "sleep".into(),
            args: vec!["30".into()],
            transport: "stdio".into(),
            port: 0,
            mode: RunMode::Shared,
            tags: vec![],
            env: Default::default(),
            health_endpoint: String::new(),
        },
    ];
    let mgr = McpManager::new(config);
    mgr.start("test-echo").await.unwrap();
    let servers = mgr.list_servers().await;
    assert!(servers[0].running);

    mgr.stop("test-echo").await.unwrap();
    let servers = mgr.list_servers().await;
    assert!(!servers[0].running);
}

#[tokio::test]
async fn manager_filter_by_tags() {
    let config = vec![
        McpServerDef {
            name: "a".into(), command: "echo".into(), args: vec![],
            transport: "stdio".into(), port: 0, mode: RunMode::Shared,
            tags: vec!["erp".into()], env: Default::default(),
            health_endpoint: String::new(),
        },
        McpServerDef {
            name: "b".into(), command: "echo".into(), args: vec![],
            transport: "stdio".into(), port: 0, mode: RunMode::Shared,
            tags: vec!["crm".into()], env: Default::default(),
            health_endpoint: String::new(),
        },
    ];
    let mgr = McpManager::new(config);
    let filtered = mgr.list_by_tags(&["erp"]).await;
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "a");
}
```

### Step 2-7: 实现 config.rs, manager.rs, routes.rs, main.rs

(实现细节与 Skill Registry 类似 — YAML 配置加载、子进程管理、REST 路由。
config.rs 定义 `McpServerDef` + `RunMode`；
manager.rs 管理 `HashMap<String, Child>` 子进程；
routes.rs 提供 `/mcp-servers`, `/mcp-servers/{name}/start`, `/mcp-servers/{name}/stop`, `/mcp-servers/{name}/info`, `/health`)

### Step 8: 运行测试

Run: `cargo test -p eaasp-mcp-orchestrator -- --test-threads=1`
Expected: ALL PASS

### Step 9: Commit

```bash
git add -f tools/eaasp-mcp-orchestrator/ Cargo.toml
git commit -m "feat(eaasp): add L2 MCP Orchestrator crate (YAML config + subprocess)"
```

---

## Task 4: Wave 4 — L1 Runtime L2 集成

**Files:**
- Modify: `crates/grid-runtime/src/harness.rs`
- Create: `crates/grid-runtime/src/l2_client.rs`
- Test: `crates/grid-runtime/tests/l2_integration.rs`

### Step 1: 写 L2 client 测试

```rust
// crates/grid-runtime/tests/l2_integration.rs
use grid_runtime::l2_client::L2SkillClient;

#[tokio::test]
async fn l2_client_fetch_skill_content() {
    // This test requires eaasp-skill-registry running on localhost:8081
    // In CI, skip if not available
    let client = L2SkillClient::new("http://localhost:8081");
    // Will fail gracefully in unit test mode
    match client.fetch_skill("org/test-skill").await {
        Ok(content) => assert!(!content.prose.is_empty()),
        Err(_) => eprintln!("Skill registry not running, skipping integration test"),
    }
}
```

### Step 2: 实现 l2_client.rs

```rust
// crates/grid-runtime/src/l2_client.rs

use anyhow::Result;
use serde::Deserialize;
use tracing::{info, warn};

/// Minimal L2 Skill content for L1 consumption.
#[derive(Debug, Clone, Deserialize)]
pub struct L2SkillContent {
    pub meta: L2SkillMeta,
    pub frontmatter_yaml: String,
    pub prose: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct L2SkillMeta {
    pub id: String,
    pub name: String,
    pub version: String,
}

/// HTTP client for L2 Skill Registry.
pub struct L2SkillClient {
    base_url: String,
    http: reqwest::Client,
}

impl L2SkillClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Fetch skill content by ID from L2 Skill Registry.
    pub async fn fetch_skill(&self, skill_id: &str) -> Result<L2SkillContent> {
        let url = format!("{}/skills/{}/content", self.base_url, skill_id);
        info!(skill_id = %skill_id, url = %url, "Fetching skill from L2");
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("L2 Skill Registry returned {}: {}", resp.status(), skill_id);
        }
        let content: L2SkillContent = resp.json().await?;
        Ok(content)
    }

    /// Fetch multiple skills in batch.
    pub async fn fetch_skills(&self, skill_ids: &[String]) -> Vec<(String, Result<L2SkillContent>)> {
        let mut results = Vec::new();
        for id in skill_ids {
            let result = self.fetch_skill(id).await;
            results.push((id.clone(), result));
        }
        results
    }
}
```

### Step 3: 修改 harness.rs initialize 方法

在 `GridHarness` 的 `initialize` 实现中，添加 L2 skill 拉取逻辑:

```rust
// 在 initialize 方法内，SessionPayload 处理后:
if !payload.skill_ids.is_empty() {
    if let Some(ref url) = payload.skill_registry_url {
        let client = crate::l2_client::L2SkillClient::new(url);
        for (id, result) in client.fetch_skills(&payload.skill_ids).await {
            match result {
                Ok(content) => {
                    let skill = SkillContent {
                        skill_id: content.meta.id,
                        name: content.meta.name,
                        frontmatter_yaml: content.frontmatter_yaml,
                        prose: content.prose,
                    };
                    if let Err(e) = self.load_skill(&handle, skill).await {
                        warn!(skill_id = %id, error = %e, "Failed to load skill from L2");
                    }
                }
                Err(e) => {
                    warn!(skill_id = %id, error = %e, "Failed to fetch skill from L2");
                }
            }
        }
    } else {
        warn!("skill_ids provided but no skill_registry_url in SessionPayload");
    }
}
```

### Step 4: 运行测试

Run: `cargo test -p grid-runtime -- --test-threads=1`
Expected: ALL PASS (integration test skipped if registry not running)

### Step 5: Commit

```bash
git add crates/grid-runtime/src/l2_client.rs crates/grid-runtime/src/harness.rs crates/grid-runtime/src/lib.rs
git commit -m "feat(grid-runtime): L2 Skill Registry integration (REST client + initialize)"
```

---

## Task 5: Wave 5 — Mock L3 RuntimeSelector + 运行时池

**Files:**
- Create: `tools/eaasp-certifier/src/runtime_pool.rs`
- Create: `tools/eaasp-certifier/src/selector.rs`
- Modify: `tools/eaasp-certifier/src/lib.rs`
- Modify: `tools/eaasp-certifier/src/mock_l3.rs`
- Test: in-file `#[cfg(test)]`

### Step 1: 写 runtime_pool 测试

```rust
// tools/eaasp-certifier/src/runtime_pool.rs (tests at bottom)
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_register_and_list() {
        let pool = RuntimePool::new();
        pool.register(RuntimeEntry {
            id: "grid-harness".into(),
            name: "Grid".into(),
            endpoint: "http://localhost:50051".into(),
            tier: "harness".into(),
            healthy: true,
        });
        pool.register(RuntimeEntry {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            endpoint: "http://localhost:50052".into(),
            tier: "harness".into(),
            healthy: true,
        });
        let all = pool.list();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn pool_healthy_only() {
        let pool = RuntimePool::new();
        pool.register(RuntimeEntry {
            id: "a".into(), name: "A".into(), endpoint: "x".into(),
            tier: "harness".into(), healthy: true,
        });
        pool.register(RuntimeEntry {
            id: "b".into(), name: "B".into(), endpoint: "y".into(),
            tier: "harness".into(), healthy: false,
        });
        let healthy = pool.healthy();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].id, "a");
    }
}
```

### Step 2: 实现 runtime_pool.rs

```rust
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEntry {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub tier: String,
    pub healthy: bool,
}

pub struct RuntimePool {
    entries: Arc<RwLock<Vec<RuntimeEntry>>>,
}

impl RuntimePool {
    pub fn new() -> Self {
        Self { entries: Arc::new(RwLock::new(Vec::new())) }
    }

    pub fn register(&self, entry: RuntimeEntry) {
        let mut entries = self.entries.write().unwrap();
        entries.retain(|e| e.id != entry.id);
        entries.push(entry);
    }

    pub fn list(&self) -> Vec<RuntimeEntry> {
        self.entries.read().unwrap().clone()
    }

    pub fn healthy(&self) -> Vec<RuntimeEntry> {
        self.entries.read().unwrap().iter().filter(|e| e.healthy).cloned().collect()
    }

    pub fn get(&self, id: &str) -> Option<RuntimeEntry> {
        self.entries.read().unwrap().iter().find(|e| e.id == id).cloned()
    }
}
```

### Step 3: 实现 selector.rs

```rust
// tools/eaasp-certifier/src/selector.rs
use crate::runtime_pool::{RuntimeEntry, RuntimePool};

pub enum SelectionStrategy {
    /// User explicitly chose a runtime.
    UserPreference(String),
    /// Blindbox: pick 2 random runtimes for comparison.
    Blindbox,
    /// Default: cheapest healthy runtime.
    Default,
}

pub struct RuntimeSelector;

impl RuntimeSelector {
    /// Select runtime(s) based on strategy.
    pub fn select(pool: &RuntimePool, strategy: &SelectionStrategy) -> Vec<RuntimeEntry> {
        let healthy = pool.healthy();
        if healthy.is_empty() {
            return vec![];
        }

        match strategy {
            SelectionStrategy::UserPreference(id) => {
                healthy.into_iter().filter(|e| e.id == *id).collect()
            }
            SelectionStrategy::Blindbox => {
                // Pick up to 2 distinct runtimes
                if healthy.len() >= 2 {
                    vec![healthy[0].clone(), healthy[1].clone()]
                } else {
                    healthy
                }
            }
            SelectionStrategy::Default => {
                // First healthy (cheapest-first ordering is future work)
                vec![healthy[0].clone()]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_pool::RuntimeEntry;

    fn make_pool() -> RuntimePool {
        let pool = RuntimePool::new();
        pool.register(RuntimeEntry {
            id: "grid".into(), name: "Grid".into(), endpoint: "a".into(),
            tier: "harness".into(), healthy: true,
        });
        pool.register(RuntimeEntry {
            id: "cc".into(), name: "Claude Code".into(), endpoint: "b".into(),
            tier: "harness".into(), healthy: true,
        });
        pool
    }

    #[test]
    fn select_user_preference() {
        let pool = make_pool();
        let selected = RuntimeSelector::select(&pool, &SelectionStrategy::UserPreference("cc".into()));
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "cc");
    }

    #[test]
    fn select_blindbox_two() {
        let pool = make_pool();
        let selected = RuntimeSelector::select(&pool, &SelectionStrategy::Blindbox);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn select_default_first_healthy() {
        let pool = make_pool();
        let selected = RuntimeSelector::select(&pool, &SelectionStrategy::Default);
        assert_eq!(selected.len(), 1);
    }
}
```

### Step 4: 运行测试

Run: `cargo test -p eaasp-certifier -- --test-threads=1`
Expected: ALL PASS

### Step 5: Commit

```bash
git add tools/eaasp-certifier/src/
git commit -m "feat(certifier): add RuntimePool + RuntimeSelector (mock L3)"
```

---

## Task 6: Wave 6 — 盲盒对比

**Files:**
- Create: `tools/eaasp-certifier/src/blindbox.rs`
- Modify: `tools/eaasp-certifier/src/main.rs` (新增 blindbox 子命令)
- Modify: `tools/eaasp-certifier/src/lib.rs`

### Step 1: 实现 blindbox.rs

```rust
// tools/eaasp-certifier/src/blindbox.rs

use std::time::Instant;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tonic::transport::Channel;
use tracing::info;

use crate::common_proto;
use crate::runtime_proto;
use crate::runtime_proto::runtime_service_client::RuntimeServiceClient;
use crate::runtime_pool::RuntimeEntry;

/// A single runtime's execution result (anonymized).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindboxResult {
    /// Anonymous label: "A" or "B"
    pub label: String,
    /// Collected response text
    pub response_text: String,
    /// Execution time
    pub duration_ms: u64,
    /// Hidden: actual runtime ID (revealed after voting)
    #[serde(skip_serializing)]
    pub runtime_id: String,
}

/// User vote for blindbox comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlindboxVote {
    AWins,
    BWins,
    Tie,
}

/// Complete blindbox comparison record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindboxRecord {
    pub prompt: String,
    pub result_a: BlindboxResult,
    pub result_b: BlindboxResult,
    pub vote: Option<BlindboxVote>,
    pub revealed: bool,
}

impl BlindboxRecord {
    /// Reveal which runtime produced which result.
    pub fn reveal(&mut self) -> (String, String) {
        self.revealed = true;
        (
            format!("A = {}", self.result_a.runtime_id),
            format!("B = {}", self.result_b.runtime_id),
        )
    }
}

/// Execute the same prompt on two runtimes, collect responses.
pub async fn execute_blindbox(
    runtimes: &[RuntimeEntry; 2],
    prompt: &str,
) -> Result<BlindboxRecord> {
    info!("Starting blindbox comparison: {} vs {}", runtimes[0].id, runtimes[1].id);

    // Randomize order
    let (first, second, label_first, label_second) = if rand_bool() {
        (&runtimes[0], &runtimes[1], "A", "B")
    } else {
        (&runtimes[1], &runtimes[0], "A", "B")
    };

    // Execute in parallel
    let (result_a, result_b) = tokio::join!(
        execute_single(first, prompt, label_first),
        execute_single(second, prompt, label_second),
    );

    Ok(BlindboxRecord {
        prompt: prompt.to_string(),
        result_a: result_a?,
        result_b: result_b?,
        vote: None,
        revealed: false,
    })
}

async fn execute_single(
    runtime: &RuntimeEntry,
    prompt: &str,
    label: &str,
) -> Result<BlindboxResult> {
    let start = Instant::now();

    let mut client = RuntimeServiceClient::connect(runtime.endpoint.clone()).await?;

    // Initialize session
    let init_resp = client
        .initialize(tonic::Request::new(runtime_proto::InitializeRequest {
            payload: Some(runtime_proto::SessionPayload {
                user_id: "blindbox-user".into(),
                user_role: "tester".into(),
                org_unit: "qa".into(),
                ..Default::default()
            }),
        }))
        .await?;

    let session_id = init_resp.into_inner().session_id;

    // Send prompt
    let mut stream = client
        .send(tonic::Request::new(runtime_proto::SendRequest {
            session_id: session_id.clone(),
            message: Some(runtime_proto::UserMessage {
                content: prompt.into(),
                message_type: "text".into(),
                metadata: Default::default(),
            }),
        }))
        .await?
        .into_inner();

    // Collect response
    let mut text = String::new();
    while let Some(chunk) = stream.message().await? {
        if chunk.chunk_type == "text_delta" {
            text.push_str(&chunk.content);
        }
    }

    // Terminate
    let _ = client
        .terminate(tonic::Request::new(runtime_proto::TerminateRequest {
            session_id,
        }))
        .await;

    let duration = start.elapsed().as_millis() as u64;

    Ok(BlindboxResult {
        label: label.into(),
        response_text: text,
        duration_ms: duration,
        runtime_id: runtime.id.clone(),
    })
}

fn rand_bool() -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        % 2
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blindbox_record_reveal() {
        let mut record = BlindboxRecord {
            prompt: "test".into(),
            result_a: BlindboxResult {
                label: "A".into(),
                response_text: "hello".into(),
                duration_ms: 100,
                runtime_id: "grid".into(),
            },
            result_b: BlindboxResult {
                label: "B".into(),
                response_text: "world".into(),
                duration_ms: 200,
                runtime_id: "cc".into(),
            },
            vote: None,
            revealed: false,
        };

        assert!(!record.revealed);
        let (a, b) = record.reveal();
        assert!(record.revealed);
        assert!(a.contains("grid") || a.contains("cc"));
        assert!(b.contains("grid") || b.contains("cc"));
    }

    #[test]
    fn blindbox_vote_serialization() {
        let vote = BlindboxVote::AWins;
        let json = serde_json::to_string(&vote).unwrap();
        assert!(json.contains("AWins"));
    }
}
```

### Step 2: 更新 main.rs 新增 blindbox 子命令

在 `tools/eaasp-certifier/src/main.rs` 的 `Commands` enum 中新增:

```rust
/// Run blindbox comparison between two runtimes.
Blindbox {
    /// First runtime endpoint
    #[arg(long)]
    runtime_a: String,
    /// Second runtime endpoint
    #[arg(long)]
    runtime_b: String,
    /// Prompt to send
    #[arg(short, long)]
    prompt: String,
},
```

### Step 3: 运行测试

Run: `cargo test -p eaasp-certifier -- --test-threads=1`
Expected: ALL PASS

### Step 4: Commit

```bash
git add tools/eaasp-certifier/src/
git commit -m "feat(certifier): add blindbox comparison (parallel execution + anonymous vote)"
```

---

## Task 7: Wave 7 — 集成验证 + 文档 + Makefile

**Files:**
- Create: `docs/design/Grid/EAASP_L2_ASSET_LAYER_DESIGN.md`
- Modify: `Makefile`
- Modify: `docs/dev/NEXT_SESSION_GUIDE.md`
- Modify: `docs/design/Grid/EAASP_ROADMAP.md`

### Step 1: 更新 Makefile

新增 targets:

```makefile
# ── L2 Skill Registry ──
skill-registry-build:
	cargo build -p eaasp-skill-registry

skill-registry-start:
	cargo run -p eaasp-skill-registry -- --data-dir ./data/skill-registry --port 8081

skill-registry-test:
	cargo test -p eaasp-skill-registry -- --test-threads=1

# ── L2 MCP Orchestrator ──
mcp-orch-build:
	cargo build -p eaasp-mcp-orchestrator

mcp-orch-start:
	cargo run -p eaasp-mcp-orchestrator -- --config ./config/mcp-servers.yaml --port 8082

mcp-orch-test:
	cargo test -p eaasp-mcp-orchestrator -- --test-threads=1

# ── Blindbox ──
blindbox:
	cargo run -p eaasp-certifier -- blindbox \
		--runtime-a http://localhost:50051 \
		--runtime-b http://localhost:50052 \
		--prompt "$(PROMPT)"
```

### Step 2: 更新 EAASP_ROADMAP.md Phase BF 状态

将 BF 状态从 "pending" 更新为 "in progress"，标记已确认的设计决策。

### Step 3: 编写设计文档

创建 `docs/design/Grid/EAASP_L2_ASSET_LAYER_DESIGN.md`（中文），包含:
- L2 统一资产层架构
- Skill Registry REST API 规范
- MCP Orchestrator 运行模式设计
- L1 ↔ L2 通信流程
- 盲盒对比设计
- 所有 BF-KD 决策记录

### Step 4: 更新 NEXT_SESSION_GUIDE.md

### Step 5: Commit

```bash
git add Makefile docs/
git commit -m "docs: Phase BF design document + Makefile targets + session guide"
```

---

## Deferred Items (BF)

| ID | 内容 | 前置条件 |
|----|------|---------|
| BF-D1 | Git 版本追溯集成到 submit_draft/promote | git2 crate 基础实现就绪后 |
| BF-D2 | MCP Orchestrator PerSession 模式 | Docker API 集成 (BH) |
| BF-D3 | MCP Orchestrator OnDemand 模式 | 连接计数 + idle 超时 (BH) |
| BF-D4 | Agent 按需 skill 发现 (allowed_skill_search) | L3 治理层 (BH) |
| BF-D5 | Ontology Service | BH+ |
| BF-D6 | Skill Registry RBAC 访问控制 | L3 认证体系 (BH) |
| BF-D7 | 盲盒 ELO/win-rate 统计聚合 | 足够评分数据积累后 |
| BF-D8 | RuntimeSelector 成本排序 (cheapest-first) | CostEstimate 数据收集 |
| BF-D9 | L2 MCP Orchestrator 容器化管理 | Docker API (BH) |
| BF-D10 | Skill Registry streamable-http 生产模式 | 部署架构确认 |

## 验收标准

- [ ] `cargo test --workspace -- --test-threads=1` 全部通过（包含 BF 新增测试）
- [ ] `eaasp-skill-registry` 可启动，REST API 可 CRUD skill
- [ ] `eaasp-mcp-orchestrator` 可启动，Shared 模式子进程可启停
- [ ] `grid-runtime` initialize 可从 L2 Skill Registry 拉取 skill 内容
- [ ] `eaasp-certifier blindbox` 可并行执行两个 runtime + 输出匿名结果
- [ ] SessionPayload proto v1.3 新增 L2 字段
- [ ] 设计文档 `EAASP_L2_ASSET_LAYER_DESIGN.md` 完成
