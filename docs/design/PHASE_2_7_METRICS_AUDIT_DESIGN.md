# Phase 2.7 Metrics + Audit 可观测性设计

## 概述

Phase 2.7 实现完整的可观测性系统，包括 Metrics 指标收集和 Audit 审计日志。

## 架构设计

### Metrics 指标系统

采用 Prometheus 风格指标模型：

- **Counter**: 只能递增的计数器 (如请求总数)
- **Gauge**: 可增可减的值 (如活跃连接数)
- **Histogram**: 统计数据分布 (如延迟分布)

核心组件：
- `MetricsRegistry`: 全局单例注册表
- `MetricFamily`: 指标族 (name + labels)
- `Reporter`: REST API 报告器

### Audit 审计系统

采用结构化日志 + SQLite 持久化：

- `AuditEvent`: 审计事件结构
- `AuditStorage`: SQLite 存储层
- `AuditMiddleware`: Axum 中间件自动记录

### 与现有系统集成

1. **EventBus**: 自动发布 OctoEvent 时记录 Metrics
2. **Middleware**: HTTP 请求自动记录 Audit
3. **DB Migration v6**: 新增 audit_logs 表

## 数据结构

### MetricsRegistry

```rust
pub struct MetricsRegistry {
    counters: DashMap<String, Arc<AtomicU64>>,
    gauges: DashMap<String, Arc<AtomicI64>>,
    histograms: DashMap<String, Histogram>,
}

impl MetricsRegistry {
    pub fn counter(&self, name: &str) -> Counter
    pub fn gauge(&self, name: &str) -> Gauge
    pub fn histogram(&self, name: &str, buckets: Vec<f64>) -> Histogram
    pub fn snapshot(&self) -> MetricsSnapshot
}
```

### AuditStorage

```rust
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
```

## API 设计

| 端点 | 方法 | 功能 |
|------|------|------|
| `/api/v1/metrics` | GET | 获取指标快照 |
| `/api/v1/audit` | GET | 审计日志列表 (分页) |
| `/api/v1/audit/export` | GET | 导出 CSV |

## 实施顺序

1. Metrics 核心 (Registry + Types)
2. Audit 存储层 + Migration
3. REST API 端点
4. Middleware 集成
5. 测试验证

## 估算工作量

| 模块 | 估算 |
|------|------|
| Metrics 核心 | ~350 LOC |
| Audit 存储 | ~250 LOC |
| REST API | ~150 LOC |
| Middleware | ~80 LOC |
| 测试 | ~150 LOC |
| **总计** | **~980 LOC** |
