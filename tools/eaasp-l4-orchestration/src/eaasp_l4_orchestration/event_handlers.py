"""Event Engine handler implementations — ADR-V2-003.

4 handler types forming the Event Engine pipeline:
  Ingestor → Deduplicator → Clusterer → Indexer

Each handler processes a single Event and returns Event | None.
Returning None drops the event from downstream processing (e.g. dedup).
"""

from __future__ import annotations

import uuid
from typing import Protocol, runtime_checkable

from .event_models import Event


# ── Handler Protocol ─────────────────────────────────────────────────────────


@runtime_checkable
class EventHandler(Protocol):
    """Base handler interface for Event Engine pipeline."""

    @property
    def name(self) -> str: ...

    async def handle(self, event: Event) -> Event | None: ...


# ── DefaultIngestor ──────────────────────────────────────────────────────────


class DefaultIngestor:
    """标准化原始事件：分配 event_id、归一化 timestamp。"""

    name = "default-ingestor"

    async def handle(self, event: Event) -> Event:
        if not event.event_id:
            event.event_id = str(uuid.uuid4())
        if not event.metadata.source:
            event.metadata.source = "unknown"
        return event


# ── TimeWindowDeduplicator ───────────────────────────────────────────────────


class TimeWindowDeduplicator:
    """时间窗口去重。

    相同 (session_id, event_type, tool_name) 在 window_seconds 内只保留第一条。
    """

    name = "time-window-deduplicator"

    def __init__(self, window_seconds: float = 2.0) -> None:
        self.window_seconds = window_seconds
        self._seen: dict[str, float] = {}  # dedup_key → last_seen_timestamp

    async def handle(self, event: Event) -> Event | None:
        tool_name = event.payload.get("tool_name", "")
        key = f"{event.session_id}:{event.event_type}:{tool_name}"
        now = float(event.created_at)
        last = self._seen.get(key)
        if last is not None and (now - last) < self.window_seconds:
            return None  # duplicate within window — drop
        self._seen[key] = now
        return event


# ── TimeWindowClusterer ──────────────────────────────────────────────────────


class TimeWindowClusterer:
    """时间窗口聚类（Phase 1 最简实现）。

    同一 session 内 window_seconds 秒内的连续事件归入同一 cluster。
    Phase 5 替换为 TopologyAwareClusterer（需 ontology）。
    """

    name = "time-window-clusterer"

    def __init__(self, window_seconds: float = 30.0) -> None:
        self.window_seconds = window_seconds
        # session_id → (cluster_id, last_event_time)
        self._clusters: dict[str, tuple[str, float]] = {}

    async def handle(self, event: Event) -> Event:
        session_id = event.session_id
        now = float(event.created_at)

        current = self._clusters.get(session_id)
        if current is None or (now - current[1]) > self.window_seconds:
            cluster_id = f"c-{uuid.uuid4().hex[:8]}"
        else:
            cluster_id = current[0]

        self._clusters[session_id] = (cluster_id, now)
        event.cluster_id = cluster_id
        return event


# ── FTS5Indexer ──────────────────────────────────────────────────────────────


class FTS5Indexer:
    """FTS5 全文索引（委托给 SQLite trigger 机制）。

    Phase 1: Indexer 本身不做额外工作，FTS5 同步由 SQLite trigger 完成。
    Future: add vector embedding, causal graph indexing here.
    """

    name = "fts5-indexer"

    async def handle(self, event: Event) -> Event:
        return event
