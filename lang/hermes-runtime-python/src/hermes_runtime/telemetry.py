"""Telemetry collector for hermes-runtime."""

import time
from dataclasses import dataclass, field


@dataclass
class TelemetryEntry:
    event_type: str
    session_id: str
    runtime_id: str
    user_id: str = ""
    timestamp: str = field(default_factory=lambda: time.strftime("%Y-%m-%dT%H:%M:%SZ"))
    payload: dict = field(default_factory=dict)


class TelemetryCollector:
    def __init__(self, session_id: str, runtime_id: str, user_id: str = ""):
        self.session_id = session_id
        self.runtime_id = runtime_id
        self.user_id = user_id
        self._entries: list[TelemetryEntry] = []

    def record(self, event_type: str, payload: dict | None = None):
        self._entries.append(TelemetryEntry(
            event_type=event_type,
            session_id=self.session_id,
            runtime_id=self.runtime_id,
            user_id=self.user_id,
            payload=payload or {},
        ))

    def peek(self) -> list[TelemetryEntry]:
        return list(self._entries)

    def flush(self) -> list[TelemetryEntry]:
        entries = list(self._entries)
        self._entries.clear()
        return entries
