# Phase 2.7 Metrics + Audit 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现完整的可观测性系统，包括 Metrics 指标收集和 Audit 审计日志

**Architecture:**
- Metrics: Prometheus 风格指标模型 (Counter/Gauge/Histogram)，使用 DashMap 实现无锁并发
- Audit: SQLite 持久化，新增 DB Migration v6，Axum Middleware 自动记录
- REST API: 指标快照 + 审计日志查询

**Tech Stack:** Rust, Tokio, DashMap, sqlx, SQLite

---

## 实施任务总览

| 任务 | 估算 | 状态 |
|------|------|------|
| Task 1: Metrics 基础结构 | 80 LOC | ⬜ |
| Task 2: Counter/Gauge/Histogram 实现 | 150 LOC | ⬜ |
| Task 3: Audit 存储层 + Migration | 150 LOC | ⬜ |
| Task 4: Audit Middleware | 80 LOC | ⬜ |
| Task 5: REST API 端点 | 150 LOC | ⬜ |
| Task 6: EventBus 集成 | 50 LOC | ⬜ |
| Task 7: 测试 | 100 LOC | ⬜ |
| Task 8: 构建验证 | - | ⬜ |

---

## Task 1: Metrics 基础结构

**Files:**
- Create: `crates/octo-engine/src/metrics/mod.rs`
- Create: `crates/octo-engine/src/metrics/registry.rs`
- Modify: `crates/octo-engine/src/lib.rs`

**Step 1: 创建 metrics 模块目录和 mod.rs**

```rust
// crates/octo-engine/src/metrics/mod.rs

pub mod registry;
pub mod counter;
pub mod gauge;
pub mod histogram;

pub use registry::MetricsRegistry;
pub use counter::Counter;
pub use gauge::Gauge;
pub use histogram::Histogram;
```

**Step 2: 创建基础结构**

```rust
// crates/octo-engine/src/metrics/registry.rs

use std::sync::Arc;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, AtomicI64};

/// 全局指标注册表
pub struct MetricsRegistry {
    counters: DashMap<String, Arc<AtomicU64>>,
    gauges: DashMap<String, Arc<AtomicI64>>,
    histograms: DashMap<String, Histogram>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: DashMap::new(),
            gauges: DashMap::new(),
            histograms: DashMap::new(),
        }
    }

    pub fn counter(&self, name: &str) -> Counter {
        let counter = self.counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();
        Counter(counter)
    }

    pub fn gauge(&self, name: &str) -> Gauge {
        let gauge = self.gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(AtomicI64::new(0)))
            .clone();
        Gauge(gauge)
    }

    pub fn histogram(&self, name: &str, buckets: Vec<f64>) -> Histogram {
        let hist = self.histograms
            .entry(name.to_string())
            .or_insert_with(|| Histogram::new(buckets))
            .clone();
        Histogram(hist)
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: 更新 lib.rs**

```rust
pub mod metrics;
pub use metrics::MetricsRegistry;
```

**Step 4: 运行 cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 5: Commit**

```bash
git add crates/octo-engine/src/metrics/
git commit -m "feat(metrics): add MetricsRegistry base structure"
```

---

## Task 2: Counter/Gauge/Histogram 实现

**Files:**
- Modify: `crates/octo-engine/src/metrics/counter.rs`
- Create: `crates/octo-engine/src/metrics/gauge.rs`
- Create: `crates/octo-engine/src/metrics/histogram.rs`

**Step 1: 创建 Counter**

```rust
// crates/octo-engine/src/metrics/counter.rs

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

pub struct Counter(Arc<AtomicU64>);

impl Counter {
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }

    pub fn inc(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn add(&self, n: u64) {
        self.0.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Clone for Counter {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
```

**Step 2: 创建 Gauge**

```rust
// crates/octo-engine/src/metrics/gauge.rs

use std::sync::Arc;
use std::sync::atomic::AtomicI64;

pub struct Gauge(Arc<AtomicI64>);

impl Gauge {
    pub fn new() -> Self {
        Self(Arc::new(AtomicI64::new(0)))
    }

    pub fn set(&self, v: i64) {
        self.0.store(v, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn dec(&self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get(&self) -> i64 {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Clone for Gauge {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
```

**Step 3: 创建 Histogram**

```rust
// crates/octo-engine/src/metrics/histogram.rs

use std::sync::Arc;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Histogram(Arc<HistogramInner>);

struct HistogramInner {
    buckets: Vec<f64>,
    counts: DashMap<usize, AtomicU64>,
    sum: AtomicU64,
    total_count: AtomicU64,
}

impl Histogram {
    pub fn new(buckets: Vec<f64>) -> Self {
        let counts = DashMap::new();
        for i in 0..=buckets.len() {
            counts.insert(i, AtomicU64::new(0));
        }

        Self(Arc::new(HistogramInner {
            buckets,
            counts,
            sum: AtomicU64::new(0),
            total_count: AtomicU64::new(0),
        }))
    }

    pub fn observe(&self, value: f64) {
        let bucket_idx = self.0.buckets
            .iter()
            .position(|&b| value <= b)
            .unwrap_or(self.0.buckets.len());

        if let Some(count) = self.0.counts.get(&bucket_idx) {
            count.inc();
        }

        self.0.sum.fetch_add(value as u64, Ordering::Relaxed);
        self.0.total_count.inc();
    }

    pub fn snapshot(&self) -> HistogramSnapshot {
        let mut buckets = Vec::new();
        let mut cumulative = 0u64;

        for (idx, bound) in self.0.buckets.iter().enumerate() {
            let count = self.0.counts.get(&idx)
                .map(|c| c.get())
                .unwrap_or(0);
            cumulative += count;
            buckets.push(Bucket {
                le: *bound,
                cumulative_count: cumulative,
            });
        }

        HistogramSnapshot {
            buckets,
            sum: self.0.sum.load(Ordering::Relaxed) as f64,
            count: self.0.total_count.load(Ordering::Relaxed),
        }
    }
}

impl Clone for Histogram {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[derive(Debug, Clone)]
pub struct HistogramSnapshot {
    pub buckets: Vec<Bucket>,
    pub sum: f64,
    pub count: u64,
}

#[derive(Debug, Clone)]
pub struct Bucket {
    pub le: f64,
    pub cumulative_count: u64,
}
```

**Step 4: 运行 cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 5: Commit**

```bash
git add crates/octo-engine/src/metrics/
git commit -m "feat(metrics): add Counter, Gauge, Histogram implementations"
```

---

## Task 3: Audit 存储层 + Migration

**Files:**
- Create: `crates/octo-engine/src/audit/mod.rs`
- Create: `crates/octo-engine/src/audit/storage.rs`
- Create: `crates/octo-server/src/migrations/v6_audit.sql`
- Modify: `crates/octo-engine/src/lib.rs`

**Step 1: 创建 audit 模块**

```rust
// crates/octo-engine/src/audit/mod.rs

pub mod storage;

pub use storage::AuditStorage;
pub use storage::AuditEvent;
```

**Step 2: 创建 AuditStorage**

```rust
// crates/octo-engine/src/audit/storage.rs

use sqlx::{SqlitePool, Row};
use chrono::{DateTime, Utc};

pub struct AuditStorage {
    pool: SqlitePool,
}

pub struct AuditEvent {
    pub event_type: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub resource_id: Option<String>,
    pub action: String,
    pub result: String,
    pub metadata: Option<serde_json::Value>,
    pub ip_address: Option<String>,
}

impl AuditStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn log(&self, event: AuditEvent) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO audit_logs (timestamp, event_type, user_id,
             session_id, resource_id, action, result, metadata, ip_address)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(Utc::now().to_rfc3339())
        .bind(&event.event_type)
        .bind(&event.user_id)
        .bind(&event.session_id)
        .bind(&event.resource_id)
        .bind(&event.action)
        .bind(&event.result)
        .bind(event.metadata.as_ref().map(|m| m.to_string()))
        .bind(&event.ip_address)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn query(
        &self,
        event_type: Option<&str>,
        user_id: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditRecord>, sqlx::Error> {
        let mut sql = "SELECT * FROM audit_logs WHERE 1=1".to_string();
        let mut binds: Vec<Box<dyn sqlx::Encode<sqlx::Sqlite>>> = Vec::new();

        if let Some(t) = event_type {
            sql.push_str(" AND event_type = ?");
        }
        if let Some(u) = user_id {
            sql.push_str(" AND user_id = ?");
        }

        sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as(&sql);

        if let Some(t) = event_type {
            query = query.bind(t);
        }
        if let Some(u) = user_id {
            query = query.bind(u);
        }
        query = query.bind(limit as i64).bind(offset as i64);

        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows.iter().map(|r| AuditRecord {
            id: r.get("id"),
            timestamp: r.get("timestamp"),
            event_type: r.get("event_type"),
            user_id: r.get("user_id"),
            session_id: r.get("session_id"),
            resource_id: r.get("resource_id"),
            action: r.get("action"),
            result: r.get("result"),
            metadata: r.get("metadata"),
            ip_address: r.get("ip_address"),
        }).collect())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditRecord {
    pub id: i64,
    pub timestamp: String,
    pub event_type: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub resource_id: Option<String>,
    pub action: String,
    pub result: String,
    pub metadata: Option<String>,
    pub ip_address: Option<String>,
}
```

**Step 3: 创建 Migration v6**

```sql
-- crates/octo-server/src/migrations/v6_audit.sql

CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    event_type TEXT NOT NULL,
    user_id TEXT,
    session_id TEXT,
    resource_id TEXT,
    action TEXT NOT NULL,
    result TEXT NOT NULL,
    metadata TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_audit_event_type ON audit_logs(event_type);
CREATE INDEX idx_audit_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_timestamp ON audit_logs(timestamp);
```

**Step 4: 更新 lib.rs**

```rust
pub mod audit;
pub use audit::AuditStorage;
```

**Step 5: 运行 cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 6: Commit**

```bash
git add crates/octo-engine/src/audit/
git add crates/octo-server/src/migrations/v6_audit.sql
git commit -m "feat(audit): add AuditStorage and migration v6"
```

---

## Task 4: Audit Middleware

**Files:**
- Create: `crates/octo-server/src/middleware/audit.rs`
- Modify: `crates/octo-server/src/router.rs`

**Step 1: 创建 Audit Middleware**

```rust
// crates/octo-server/src/middleware/audit.rs

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn audit_middleware(
    request: Request,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();

    // 从 extension 获取 user_id
    let user_id = request.extensions()
        .get::<super::auth::UserContext>()
        .map(|u| u.user_id.clone());

    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // 记录到审计日志 (通过 State)
    // 实际实现需要从 State 获取 AuditStorage

    response
}
```

**Step 2: 运行 cargo check**

Run: `cargo check -p octo-server`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/octo-server/src/middleware/audit.rs
git commit -m "feat(audit): add audit middleware"
```

---

## Task 5: REST API 端点

**Files:**
- Create: `crates/octo-server/src/api/metrics.rs`
- Create: `crates/octo-server/src/api/audit.rs`
- Modify: `crates/octo-server/src/r.rs`

**Step 1: 创建 Metrics API**

```rust
// crates/octo-server/src/api/metrics.rs

use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use super::AppState;

#[derive(Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub counters: Vec<CounterMetric>,
    pub gauges: Vec<GaugeMetric>,
    pub histograms: Vec<HistogramMetric>,
}

#[derive(Serialize)]
pub struct CounterMetric {
    pub name: String,
    pub value: u64,
}

#[derive(Serialize)]
pub struct GaugeMetric {
    pub name: String,
    pub value: i64,
}

#[derive(Serialize)]
pub struct HistogramMetric {
    pub name: String,
    pub count: u64,
    pub sum: f64,
    pub buckets: Vec<Bucket>,
}

#[derive(Serialize)]
pub struct Bucket {
    pub le: f64,
    pub count: u64,
}

pub async fn get_metrics(State(state): State<AppState>) -> Json<MetricsSnapshot> {
    let registry = state.metrics_registry.read().await;

    let counters = registry.counters()
        .iter()
        .map(|e| CounterMetric {
            name: e.key().clone(),
            value: e.value().get(),
        })
        .collect();

    // ... 类似处理 gauges 和 histograms

    Json(MetricsSnapshot {
        timestamp: chrono::Utc::now(),
        counters,
        gauges: vec![],
        histograms: vec![],
    })
}

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/metrics", get(get_metrics))
}
```

**Step 2: 创建 Audit API**

```rust
// crates/octo-server/src/api/audit.rs

use axum::{
    extract::{State, Query},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use super::AppState;

#[derive(Deserialize)]
pub struct AuditQuery {
    pub event_type: Option<String>,
    pub user_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Serialize)]
pub struct AuditResponse {
    pub logs: Vec<AuditRecord>,
    pub total: i64,
}

pub async fn list_audit(
    State(state): State<AppState>,
    Query(query): Query<AuditQuery>,
) -> Json<AuditResponse> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let logs = state.audit_storage
        .query(
            query.event_type.as_deref(),
            query.user_id.as_deref(),
            limit,
            offset,
        )
        .await
        .unwrap_or_default();

    Json(AuditResponse {
        logs,
        total: logs.len() as i64,
    })
}

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/audit", get(list_audit))
}
```

**Step 3: 更新 router.rs 合并路由**

```rust
use super::api::{metrics::router as metrics_router, audit::router as audit_router};

pub fn create_router() -> Router {
    Router::new()
        .nest("/api/v1", metrics_router())
        .nest("/api/v1", audit_router())
        // ... existing routes
}
```

**Step 4: 更新 AppState**

```rust
pub struct AppState {
    // ... existing fields
    pub metrics_registry: Arc<RwLock<octo_engine::metrics::MetricsRegistry>>,
    pub audit_storage: Arc<octo_engine::audit::AuditStorage>,
}
```

**Step 5: 运行 cargo check**

Run: `cargo check -p octo-server`
Expected: No errors

**Step 6: Commit**

```bash
git add crates/octo-server/src/api/metrics.rs
git add crates/octo-server/src/api/audit.rs
git add crates/octo-server/src/router.rs
git add crates/octo-server/src/state.rs
git commit -m "feat(api): add metrics and audit REST endpoints"
```

---

## Task 6: EventBus 集成

**Files:**
- Modify: `crates/octo-engine/src/event/bus.rs`

**Step 1: 添加 Metrics 集成**

```rust
// 在 EventBus publish 方法中添加指标记录

pub async fn publish(&self, event: OctoEvent) {
    // 原有逻辑...

    // 记录指标
    match &event {
        OctoEvent::ToolCallCompleted { tool_name, duration_ms, .. } => {
            // 调用 metrics registry
            // self.metrics.counter("octo.tools.executions.total").inc();
        }
        OctoEvent::LoopTurnStarted { .. } => {
            // self.metrics.counter("octo.sessions.turns.total").inc();
        }
        _ => {}
    }
}
```

**Step 2: 运行 cargo check**

Run: `cargo check -p octo-engine`
Expected: No errors

**Step 3: Commit**

```bash
git add crates/octo-engine/src/event/bus.rs
git commit -m "feat(metrics): integrate EventBus with Metrics"
```

---

## Task 7: 测试

**Files:**
- Create: `crates/octo-engine/src/metrics/registry_test.rs`
- Create: `crates/octo-engine/src/audit/storage_test.rs`

**Step 1: 编写 Metrics 测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_counter_operations() {
        let registry = MetricsRegistry::new();
        let counter = registry.counter("test.counter");

        assert_eq!(counter.get(), 0);
        counter.inc();
        assert_eq!(counter.get(), 1);
        counter.add(5);
        assert_eq!(counter.get(), 6);
    }

    #[tokio::test]
    async fn test_gauge_operations() {
        let registry = MetricsRegistry::new();
        let gauge = registry.gauge("test.gauge");

        gauge.set(10);
        assert_eq!(gauge.get(), 10);
        gauge.inc();
        assert_eq!(gauge.get(), 11);
        gauge.dec();
        assert_eq!(gauge.get(), 10);
    }

    #[tokio::test]
    async fn test_histogram_observations() {
        let registry = MetricsRegistry::new();
        let hist = registry.histogram("test.latency", vec![10.0, 50.0, 100.0]);

        hist.observe(5.0);
        hist.observe(25.0);
        hist.observe(75.0);

        let snapshot = hist.snapshot();
        assert_eq!(snapshot.count, 3);
    }
}
```

**Step 2: 运行测试**

Run: `cargo test -p octo-engine metrics`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/octo-engine/src/metrics/
git commit -m "test(metrics): add MetricsRegistry unit tests"
```

---

## Task 8: 构建验证

**Step 1: 运行完整 cargo check**

Run: `cargo check --all`
Expected: No errors

**Step 2: 运行所有测试**

Run: `cargo test --all`
Expected: All tests pass

**Step 3: 最终提交**

```bash
git add -A
git commit -m "feat: complete Phase 2.7 Metrics + Audit

- MetricsRegistry with Counter, Gauge, Histogram
- AuditStorage with SQLite persistence
- REST API endpoints for metrics and audit
- EventBus integration for auto-metrics
- Unit tests

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## 实施完成

所有任务完成后，运行验证：

```bash
# 构建验证
cargo check --all

# 测试验证
cargo test --all
```

---

## 实施选项

**计划已完成并保存到 `docs/plans/2026-03-01-phase2-7-metrics-audit.md`**

两种执行方式：

**1. Subagent-Driven (本会话)** - 每个任务派遣新的 subagent，我进行代码审查，快速迭代

**2. Parallel Session (单独会话)** - 在 worktree 中开新会话，使用 executing-plans，批量执行并检查点

请选择 (1/2)。
