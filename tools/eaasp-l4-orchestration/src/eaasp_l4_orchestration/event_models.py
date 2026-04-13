"""标准化事件数据模型，对齐 proto EventStreamEntry。

Phase 1 Event Engine 的核心数据结构。所有事件（拦截器提取 / L1 EmitEvent）
统一为 Event 结构后进入 handler pipeline。
"""

from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from typing import Any


@dataclass
class EventMetadata:
    """事件追踪元数据。"""

    trace_id: str = ""
    span_id: str = ""
    parent_event_id: str = ""
    source: str = ""  # "runtime:grid-runtime" | "interceptor" | "orchestrator"
    extra: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {}
        if self.trace_id:
            d["trace_id"] = self.trace_id
        if self.span_id:
            d["span_id"] = self.span_id
        if self.parent_event_id:
            d["parent_event_id"] = self.parent_event_id
        if self.source:
            d["source"] = self.source
        if self.extra:
            d["extra"] = self.extra
        return d


@dataclass
class Event:
    """标准化事件结构。"""

    session_id: str
    event_type: str  # HookEventType 枚举值名
    payload: dict[str, Any] = field(default_factory=dict)
    event_id: str = ""  # UUID, Ingestor 分配
    metadata: EventMetadata = field(default_factory=EventMetadata)
    created_at: int = 0  # unix epoch
    cluster_id: str | None = None  # Clusterer 分配
    seq: int | None = None  # DB 自增序号（append 后回填）

    def __post_init__(self) -> None:
        if not self.event_id:
            self.event_id = str(uuid.uuid4())
        if not self.created_at:
            self.created_at = int(time.time())
