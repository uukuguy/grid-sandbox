# Phase 1 — Event-driven Foundation 设计实施方案

> **权威参考**：ADR-V2-001 / ADR-V2-002 / ADR-V2-003（`docs/design/EAASP/adrs/`）
> **执行计划**：`docs/plans/2026-04-13-v2-phase1-plan.md`

---

## 一、架构总览

Phase 1 在 Phase 0.75 的可运行 agent 基础上增加**事件可观测性**。核心变更集中在 L4 Orchestration 层，L1 Runtime 改动最小化（拦截器策略）。

### 数据流架构

```
┌───────────────────────────────────────────────────────────────────┐
│                     L4 Orchestration 层                          │
│                                                                   │
│  ┌─── 事件来源 ──────────────────────────────────────────────┐   │
│  │                                                            │   │
│  │  来源 1: 平台拦截器（零改造，自动从现有 RPC 提取）         │   │
│  │    session_orchestrator.send_message()                      │   │
│  │      ├─ OnToolCall    → PRE_TOOL_USE 事件                  │   │
│  │      ├─ OnToolResult  → POST_TOOL_USE / _FAILURE 事件      │   │
│  │      ├─ OnStop        → STOP 事件                          │   │
│  │      ├─ Initialize OK → SESSION_START 事件                 │   │
│  │      └─ Terminate     → POST_SESSION_END 事件              │   │
│  │                                                            │   │
│  │  来源 2: L1 EmitEvent REST（T1 runtime 主动 emit）         │   │
│  │    POST /v1/events/ingest                                   │   │
│  │      ├─ USER_PROMPT_SUBMIT                                  │   │
│  │      ├─ PRE_COMPACT                                         │   │
│  │      ├─ SUBAGENT_STOP                                       │   │
│  │      └─ PERMISSION_REQUEST                                  │   │
│  └────────────────────────────────────────────────────────────┘   │
│                            │                                      │
│                            ▼                                      │
│  ┌─── Event Engine ──────────────────────────────────────────┐   │
│  │                                                            │   │
│  │  EventStreamBackend.append()  ←── 先持久化（保证不丢）     │   │
│  │            │                                               │   │
│  │            ▼ (async queue)                                 │   │
│  │  ┌──────────────────────────────────────────────────┐     │   │
│  │  │ Handler Pipeline (后台 worker)                    │     │   │
│  │  │                                                   │     │   │
│  │  │  [DefaultIngestor]                                │     │   │
│  │  │    → 分配 event_id, 标注 source, 归一化 timestamp  │     │   │
│  │  │                                                   │     │   │
│  │  │  [TimeWindowDeduplicator]                         │     │   │
│  │  │    → (session_id, event_type, tool_name) 2s 窗口   │     │   │
│  │  │                                                   │     │   │
│  │  │  [TimeWindowClusterer]                            │     │   │
│  │  │    → 同 session 30s 窗口归入同一 cluster           │     │   │
│  │  │                                                   │     │   │
│  │  │  [FTS5Indexer]                                    │     │   │
│  │  │    → SQLite trigger 自动同步 FTS5 索引             │     │   │
│  │  └──────────────────────────────────────────────────┘     │   │
│  │            │                                               │   │
│  │            ▼                                               │   │
│  │  Backend.update_cluster()  ←── 回写 cluster_id             │   │
│  └────────────────────────────────────────────────────────────┘   │
│                            │                                      │
│                            ▼                                      │
│  ┌─── 事件消费 ──────────────────────────────────────────────┐   │
│  │                                                            │   │
│  │  GET /v1/sessions/{id}/events          → 事件列表          │   │
│  │  GET /v1/sessions/{id}/events?follow=1 → SSE 实时流        │   │
│  │  GET /v1/events/clusters/{cluster_id}  → 聚类详情          │   │
│  │                                                            │   │
│  │  eaasp-cli session events <id>          → 一次性输出       │   │
│  │  eaasp-cli session events <id> --follow → 实时追踪         │   │
│  └────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────┘
```

---

## 二、模块设计

### 2.1 新增文件清单

```
tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/
├── event_backend.py          # EventStreamBackend Protocol 定义
├── event_backend_sqlite.py   # SQLite WAL 实现
├── event_models.py           # Event / EventMetadata 数据模型
├── event_engine.py           # EventEngine 编排器 + handler pipeline
├── event_handlers.py         # 4 个默认 handler 实现
├── event_interceptor.py      # L4 拦截器（从现有 RPC 提取事件）
│
│  # 已有文件需修改：
├── event_stream.py           # → 委托给 EventStreamBackend（保持兼容）
├── session_orchestrator.py   # → 注入 EventEngine + 拦截器
├── api.py                    # → 新增 event ingest/follow/cluster 端点
└── db.py                     # → 扩展 schema（新列 + FTS5）

tools/eaasp-cli-v2/src/eaasp_cli_v2/
├── cmd_session.py            # → 新增 events 子命令 + follow mode
└── client.py                 # → 新增 SSE 客户端方法
```

### 2.2 event_models.py — 数据模型

```python
"""标准化事件数据模型，对齐 proto EventStreamEntry。"""

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any
import uuid
import time


@dataclass
class EventMetadata:
    """事件追踪元数据。"""
    trace_id: str = ""             # 跨事件追踪 ID
    span_id: str = ""              # 当前事件 span
    parent_event_id: str = ""      # 因果父事件（构建事件 DAG）
    source: str = ""               # "runtime:grid-runtime" | "interceptor" | "orchestrator"
    extra: dict[str, Any] = field(default_factory=dict)


@dataclass
class Event:
    """标准化事件结构。"""
    session_id: str
    event_type: str                # HookEventType 枚举值名
    payload: dict[str, Any] = field(default_factory=dict)
    event_id: str = ""             # UUID, Ingestor 分配
    metadata: EventMetadata = field(default_factory=EventMetadata)
    created_at: int = 0            # unix epoch
    cluster_id: str | None = None  # Clusterer 分配
    seq: int | None = None         # DB 自增序号（append 后回填）

    def __post_init__(self):
        if not self.event_id:
            self.event_id = str(uuid.uuid4())
        if not self.created_at:
            self.created_at = int(time.time())
```

### 2.3 event_backend.py — 可插拔后端接口

```python
"""EventStreamBackend Protocol — 可插拔的事件持久化接口。

Phase 1: SqliteWalBackend
Phase 6+: NatsJetstreamBackend / KafkaBackend
"""

from __future__ import annotations
from typing import Any, Protocol, AsyncIterator


class EventStreamBackend(Protocol):
    async def append(
        self,
        session_id: str,
        event_type: str,
        payload: dict[str, Any],
        *,
        event_id: str | None = None,
        source: str = "",
        metadata: dict[str, Any] | None = None,
    ) -> tuple[int, str]:
        """追加事件。返回 (seq, event_id)。"""
        ...

    async def list_events(
        self,
        session_id: str,
        from_seq: int = 1,
        to_seq: int | None = None,
        limit: int = 500,
        event_types: list[str] | None = None,
    ) -> list[dict[str, Any]]:
        """查询事件列表（升序）。"""
        ...

    async def subscribe(
        self,
        session_id: str,
        from_seq: int = 0,
    ) -> AsyncIterator[dict[str, Any]]:
        """订阅事件流（用于 follow mode）。"""
        ...

    async def count(self, session_id: str) -> int:
        """返回 session 的事件总数。"""
        ...

    async def search(
        self,
        session_id: str,
        query: str,
        limit: int = 50,
    ) -> list[dict[str, Any]]:
        """全文搜索事件。"""
        ...

    async def update_cluster(self, event_id: str, cluster_id: str) -> None:
        """回写 cluster_id（Event Engine Clusterer 调用）。"""
        ...
```

### 2.4 event_backend_sqlite.py — SQLite WAL 实现

关键变更点（相对于现有 `event_stream.py`）：

| 变更 | 说明 |
|------|------|
| 新增列 `event_id` | UUID TEXT, 分布式友好 |
| 新增列 `source` | 事件来源标识 |
| 新增列 `metadata_json` | 追踪元数据 JSON |
| 新增列 `cluster_id` | Clusterer 分配的聚类 ID |
| FTS5 虚拟表 | `session_events_fts(event_type, payload_json)` |
| FTS5 同步 trigger | `AFTER INSERT → INSERT INTO fts` |
| `subscribe()` 方法 | 基于 polling（0.5s 间隔）的 AsyncIterator |
| `search()` 方法 | FTS5 MATCH 查询 |
| `update_cluster()` 方法 | `UPDATE session_events SET cluster_id=? WHERE event_id=?` |

### 2.5 event_interceptor.py — 平台拦截器

```python
"""L4 平台拦截器 — 从现有 session_orchestrator 调用中自动提取事件。

无需修改 proto 或 L1 runtime。拦截点在 session_orchestrator 的
send_message / stream_message 方法中，每个 L1 response chunk 被
检查是否包含 tool_call / tool_result / stop 信息。
"""

from __future__ import annotations
from .event_models import Event, EventMetadata


class EventInterceptor:
    """从 L1 response chunks 中提取 HookEventType 事件。"""

    def extract_from_chunk(
        self,
        session_id: str,
        chunk: dict,
        *,
        runtime_id: str = "",
    ) -> Event | None:
        """检查 chunk 是否对应一个可提取的事件。

        返回 Event 或 None（chunk 不对应任何事件类型）。
        """
        chunk_type = chunk.get("chunk_type", "")

        if chunk_type == "tool_call_start":
            return Event(
                session_id=session_id,
                event_type="PRE_TOOL_USE",
                payload={
                    "tool_name": chunk.get("tool_name", ""),
                    "arguments": chunk.get("arguments", {}),
                },
                metadata=EventMetadata(source=f"interceptor:{runtime_id}"),
            )

        if chunk_type == "tool_result":
            is_error = chunk.get("is_error", False)
            return Event(
                session_id=session_id,
                event_type="POST_TOOL_USE_FAILURE" if is_error else "POST_TOOL_USE",
                payload={
                    "tool_name": chunk.get("tool_name", ""),
                    "result": chunk.get("content", ""),
                    "is_error": is_error,
                },
                metadata=EventMetadata(source=f"interceptor:{runtime_id}"),
            )

        if chunk_type == "done":
            return Event(
                session_id=session_id,
                event_type="STOP",
                payload={
                    "reason": "complete",
                    "response_text": chunk.get("response_text", ""),
                },
                metadata=EventMetadata(source=f"interceptor:{runtime_id}"),
            )

        return None

    def create_session_start(
        self, session_id: str, runtime_id: str
    ) -> Event:
        """在 Initialize 成功后调用。"""
        return Event(
            session_id=session_id,
            event_type="SESSION_START",
            payload={"runtime_id": runtime_id},
            metadata=EventMetadata(source=f"interceptor:{runtime_id}"),
        )

    def create_session_end(
        self, session_id: str
    ) -> Event:
        """在 close_session / Terminate 时调用。"""
        return Event(
            session_id=session_id,
            event_type="POST_SESSION_END",
            payload={},
            metadata=EventMetadata(source="interceptor:orchestrator"),
        )
```

### 2.6 event_engine.py — 编排器

核心逻辑（简化版）：

```python
class EventEngine:
    def __init__(
        self,
        backend: EventStreamBackend,
        handlers: list[EventHandler] | None = None,
        queue_size: int = 1000,
    ):
        self.backend = backend
        self.handlers = handlers or _default_handlers()
        self._queue: asyncio.Queue[Event] = asyncio.Queue(maxsize=queue_size)
        self._running = False

    async def ingest(self, event: Event) -> tuple[int, str]:
        """接收事件：先持久化，再异步投递到 pipeline 队列。"""
        seq, event_id = await self.backend.append(
            session_id=event.session_id,
            event_type=event.event_type,
            payload=event.payload,
            event_id=event.event_id,
            source=event.metadata.source,
            metadata=_serialize_metadata(event.metadata),
        )
        event.seq = seq
        event.event_id = event_id
        # 非阻塞：队列满时丢弃（fire-and-forget 语义）
        try:
            self._queue.put_nowait(event)
        except asyncio.QueueFull:
            logger.warning("Event pipeline queue full, dropping event %s", event_id)
        return seq, event_id

    async def _worker(self):
        """后台 worker：取事件 → 执行 handler chain → 回写 cluster_id。"""
        while self._running:
            try:
                event = await asyncio.wait_for(self._queue.get(), timeout=1.0)
            except asyncio.TimeoutError:
                continue
            for handler in self.handlers:
                result = await handler.handle(event)
                if result is None:
                    break
                event = result
            if event is not None and event.cluster_id:
                await self.backend.update_cluster(event.event_id, event.cluster_id)
```

### 2.7 session_orchestrator.py 集成改造

**最小侵入原则**：不重写 `session_orchestrator`，只在关键节点注入 `EventEngine.ingest()` 和 `EventInterceptor`。

```python
# session_orchestrator.py 变更点

class SessionOrchestrator:
    def __init__(
        self,
        ...,
        event_engine: EventEngine | None = None,     # 新增
        event_interceptor: EventInterceptor | None = None,  # 新增
    ):
        ...
        self.event_engine = event_engine
        self.event_interceptor = event_interceptor or EventInterceptor()

    async def create_session(self, ...):
        ...
        # Step 5 — Initialize 成功后，通过拦截器注入 SESSION_START
        if self.event_engine:
            start_event = self.event_interceptor.create_session_start(
                session_id, runtime_pref
            )
            await self.event_engine.ingest(start_event)
        ...

    async def stream_message(self, ...):
        ...
        async for chunk in l1.send(l1_sid, content):
            ...
            # 拦截器：从 chunk 提取事件
            if self.event_engine:
                extracted = self.event_interceptor.extract_from_chunk(
                    session_id, chunk, runtime_id=runtime_id
                )
                if extracted:
                    await self.event_engine.ingest(extracted)
            ...
```

### 2.8 api.py 新增端点

| 端点 | 方法 | 用途 |
|------|------|------|
| `/v1/events/ingest` | POST | 接收 L1 EmitEvent REST fallback |
| `/v1/sessions/{id}/events` | GET (+follow=1) | 增强：支持 SSE follow mode |
| `/v1/events/clusters/{cluster_id}` | GET | 查看聚类详情 |

SSE follow mode 实现：

```python
@app.get("/v1/sessions/{session_id}/events")
async def list_events(
    session_id: str,
    follow: bool = Query(default=False),
    ...
):
    if follow:
        return StreamingResponse(
            _sse_event_stream(session_id, from_seq),
            media_type="text/event-stream",
        )
    else:
        # 现有逻辑不变
        ...

async def _sse_event_stream(session_id, from_seq):
    backend = app.state.event_engine.backend
    async for event in backend.subscribe(session_id, from_seq):
        yield f"data: {json.dumps(event)}\n\n"
```

### 2.9 db.py Schema 扩展

```sql
-- Phase 1 扩展（增量 migration，不删改现有列）
ALTER TABLE session_events ADD COLUMN event_id TEXT;
ALTER TABLE session_events ADD COLUMN source TEXT DEFAULT '';
ALTER TABLE session_events ADD COLUMN metadata_json TEXT DEFAULT '{}';
ALTER TABLE session_events ADD COLUMN cluster_id TEXT;

-- FTS5 全文索引
CREATE VIRTUAL TABLE IF NOT EXISTS session_events_fts USING fts5(
    event_type, payload_json,
    content='session_events', content_rowid='seq'
);

CREATE TRIGGER IF NOT EXISTS session_events_ai AFTER INSERT ON session_events BEGIN
    INSERT INTO session_events_fts(rowid, event_type, payload_json)
    VALUES (new.seq, new.event_type, new.payload_json);
END;

-- cluster_id 索引（聚类查询）
CREATE INDEX IF NOT EXISTS idx_session_events_cluster
    ON session_events(cluster_id) WHERE cluster_id IS NOT NULL;
```

**Migration 策略**：SQLite `ALTER TABLE ADD COLUMN` 是在线操作，不需要重建表。`init_db()` 在 `executescript(SCHEMA)` 后追加 `ALTER` 语句（使用 `try/except` 忽略 "duplicate column name" 错误）。

### 2.10 CLI 事件观察

```python
# cmd_session.py 新增子命令

@session_app.command("events")
def session_events(
    session_id: str,
    follow: bool = typer.Option(False, "--follow", "-f", help="实时追踪事件流"),
    format_: str = typer.Option("table", "--format", help="输出格式: table/json"),
    search: str = typer.Option("", "--search", "-s", help="FTS 搜索"),
):
    """查看 session 事件列表或实时追踪。"""
    if follow:
        _follow_events(session_id, format_)
    elif search:
        _search_events(session_id, search, format_)
    else:
        _list_events(session_id, format_)

def _follow_events(session_id: str, format_: str):
    """SSE 长连接实时显示事件。"""
    import httpx
    with httpx.stream(
        "GET",
        f"{config.l4_url}/v1/sessions/{session_id}/events?follow=true",
        timeout=None,
    ) as response:
        for line in response.iter_lines():
            if line.startswith("data:"):
                event = json.loads(line[5:])
                _print_event(event, format_)

def _print_event(event: dict, format_: str):
    """格式化输出单条事件。"""
    if format_ == "json":
        typer.echo(json.dumps(event, ensure_ascii=False))
        return
    # table format with color coding
    ts = _format_ts(event.get("created_at", 0))
    etype = event.get("event_type", "")
    color = EVENT_COLORS.get(etype, "white")
    payload_summary = _summarize_payload(event.get("payload", {}))
    cluster = event.get("cluster_id", "")
    typer.echo(
        f"[{ts}] {typer.style(etype.ljust(22), fg=color)} "
        f"{payload_summary}"
        + (f"  cluster={cluster}" if cluster else "")
    )

EVENT_COLORS = {
    "SESSION_START": "green",
    "PRE_TOOL_USE": "cyan",
    "POST_TOOL_USE": "blue",
    "POST_TOOL_USE_FAILURE": "red",
    "STOP": "yellow",
    "POST_SESSION_END": "magenta",
    "USER_PROMPT_SUBMIT": "white",
    "CLUSTER_FORMED": "bright_green",
}
```

---

## 三、grid-runtime EmitEvent 实装方案（S2.T1）

Phase 1 中 grid-runtime 是唯一需要修改的 L1 runtime（T1 示范）。

### 变更点

| 文件 | 变更 |
|------|------|
| `crates/grid-runtime/src/contract.rs` | `emit_event()` 从 placeholder 改为 HTTP POST 实现 |
| `crates/grid-runtime/src/harness.rs` | agent loop 关键节点调用 `emit_event()` |
| `crates/grid-runtime/src/event_emitter.rs` | 新增：封装 HTTP POST 到 L4 |

### event_emitter.rs 设计

```rust
/// Fire-and-forget event emitter.
/// 
/// 发送失败时 log warning 但不阻塞 agent loop。
/// 内部维护一个 bounded channel (capacity=100)，
/// 后台 tokio task 消费并 POST 到 L4。
pub struct EventEmitter {
    tx: mpsc::Sender<EventStreamEntry>,
    l4_url: String,
}

impl EventEmitter {
    pub fn new(l4_url: &str) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let url = format!("{}/v1/events/ingest", l4_url);
        tokio::spawn(Self::worker(rx, url.clone()));
        Self { tx, l4_url: url }
    }

    pub fn emit(&self, entry: EventStreamEntry) {
        // try_send: 队列满时 drop（fire-and-forget）
        let _ = self.tx.try_send(entry);
    }

    async fn worker(mut rx: mpsc::Receiver<EventStreamEntry>, url: String) {
        let client = reqwest::Client::new();
        while let Some(entry) = rx.recv().await {
            let body = serde_json::json!({
                "session_id": entry.session_id,
                "event_id": entry.event_id,
                "event_type": entry.event_type,
                "payload_json": entry.payload_json,
                "timestamp": entry.timestamp,
            });
            if let Err(e) = client.post(&url).json(&body).send().await {
                tracing::warn!("EmitEvent POST failed: {}", e);
            }
        }
    }
}
```

### harness.rs 集成点

```rust
// GridHarness agent loop 中的事件发射点：

// 1. Initialize 成功后
emitter.emit(EventStreamEntry {
    event_type: "SESSION_START",
    session_id: sid.clone(),
    ..
});

// 2. tool call 前
emitter.emit(EventStreamEntry {
    event_type: "PRE_TOOL_USE",
    payload_json: json!({"tool_name": name, "arguments": args}).to_string(),
    ..
});

// 3. tool call 后
emitter.emit(EventStreamEntry {
    event_type: "POST_TOOL_USE",
    payload_json: json!({"tool_name": name, "result": result}).to_string(),
    ..
});

// 4. agent 完成
emitter.emit(EventStreamEntry {
    event_type: "STOP",
    payload_json: json!({"reason": "complete"}).to_string(),
    ..
});
```

**注意**：claude-code-runtime 和 hermes-runtime **不改动**——它们的核心事件由 L4 拦截器覆盖。

---

## 四、向后兼容策略

| 组件 | 兼容方式 |
|------|---------|
| `SessionEventStream` | 保留类名和 `append()` / `list_events()` 签名，内部委托给 `SqliteWalBackend` |
| `SessionOrchestrator` | `event_engine` 参数可选（None 时退化为现有行为） |
| `session_events` 表 | 新列用 `ALTER TABLE ADD COLUMN`，旧数据新列为 NULL/空 |
| 现有测试 | 无需修改——`EventEngine` 注入为 None 时完全透明 |
| API 端点 | 现有 `GET /v1/sessions/{id}/events` 保持不变，`follow` 和 `search` 是新参数 |

---

## 五、测试策略

| Stage | 测试文件 | 预期数量 |
|-------|---------|---------|
| S1.T1 | `tests/test_event_backend_sqlite.py` | 6-8 |
| S1.T2 | `tests/test_event_models.py` | 4-5 |
| S2.T1 | `crates/grid-runtime/tests/emit_event_integration.rs` | 5-6 |
| S3.T1 | `tests/test_event_engine.py` | 6-8 |
| S3.T2 | `tests/test_event_handlers.py` | 6-8 |
| S3.T3 | `tests/test_event_api.py` + `tests/test_event_interceptor.py` | 6-8 |
| S4.T1 | `tests/test_cli_events.py` | 4-5 |
| **合计** | | **~40-50 个新测试** |

**不改动现有测试** — EventEngine 为可选注入，不影响 Phase 0.5/0.75 的 216 个测试。

---

## 六、风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 拦截器提取的 chunk_type 与实际 L1 输出不匹配 | 事件丢失 | S3.T3 端到端测试覆盖三 runtime 的 chunk 格式 |
| SQLite WAL subscribe polling 延迟 (0.5s) | follow mode 不够实时 | Phase 1 可接受；Phase 6 切 NATS push |
| FTS5 trigger 在高频写入时性能下降 | 事件写入变慢 | 监控 append 延迟；Phase 6 可异步更新 FTS |
| Event Engine worker 单线程瓶颈 | pipeline 积压 | queue_size=1000 缓冲；Phase 6 多 worker |
| grid-runtime EmitEvent HTTP POST 到 L4 | 需要知道 L4 URL | 通过环境变量 `EAASP_L4_URL` 注入 |

---

## 七、实施顺序

```
S1.T1 + S1.T2 (并行)
  │
  ├─ S1.T1: EventStreamBackend + SqliteWalBackend + subscribe + FTS5
  └─ S1.T2: Event + EventMetadata 数据模型 + DB migration
  │
  ▼
S2.T1 + S2.T2 + S2.T3 + S3.T1 (并行)
  │
  ├─ S2.T1: grid-runtime EventEmitter + harness 集成
  ├─ S2.T2: claude-code-runtime (拦截器覆盖，无改动)
  ├─ S2.T3: hermes-runtime (拦截器覆盖，无改动)
  └─ S3.T1: EventEngine pipeline 框架 + handler 接口
  │
  ▼
S3.T2 + S3.T3 (串行)
  │
  ├─ S3.T2: Deduplicator + Clusterer 默认实现
  └─ S3.T3: API 端点 + 拦截器 + session_orchestrator 集成
  │
  ▼
S4.T1 → S4.T2
  │
  ├─ S4.T1: CLI events 子命令 + follow mode
  └─ S4.T2: 端到端验证 + dev-eaasp.sh 更新
```

**预计新增代码量**：~2000-2500 行 Python + ~300 行 Rust
**预计新增测试**：~40-50 个

---

## 八、决策索引

| ID | 决策 | ADR/来源 |
|---|------|---------|
| D73 | Event Room → Phase 4 | ADR-V2-001 |
| D74 | Pipeline 异步队列 | ADR-V2-003 |
| D75 | Clusterer 时间窗口 only | ADR-V2-003 |
| D76 | EmitEvent OPTIONAL, T1 必须 | ADR-V2-001 |
| D77 | SQLite WAL + 接口抽象 | ADR-V2-002 |
| D78 | REST fallback | ADR-V2-001 |
