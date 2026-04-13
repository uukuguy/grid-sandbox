"""SqliteWalBackend — SQLite WAL implementation of EventStreamBackend.

Phase 1 concrete backend per ADR-V2-002. Features:
- WAL mode for concurrent read/write
- FTS5 full-text search via trigger-synced virtual table
- subscribe() via polling (0.5s interval)
- event_id / source / metadata_json / cluster_id columns
"""

from __future__ import annotations

import asyncio
import json
import time
import uuid
from collections.abc import AsyncIterator
from typing import Any

from .db import connect


class SqliteWalBackend:
    """SQLite WAL-backed event stream with FTS5 search."""

    def __init__(self, db_path: str) -> None:
        self.db_path = db_path

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
        if not session_id:
            raise ValueError("session_id must be a non-empty string")
        if not event_type:
            raise ValueError("event_type must be a non-empty string")

        eid = event_id or str(uuid.uuid4())
        ts = int(time.time())
        payload_json = json.dumps(payload, sort_keys=True)
        metadata_json = json.dumps(metadata or {}, sort_keys=True)

        db = await connect(self.db_path)
        try:
            await db.execute("BEGIN IMMEDIATE")
            try:
                cur = await db.execute(
                    """
                    INSERT INTO session_events
                        (session_id, event_type, payload_json, created_at,
                         event_id, source, metadata_json)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (session_id, event_type, payload_json, ts, eid, source, metadata_json),
                )
                seq = cur.lastrowid
                await db.commit()
            except Exception:
                await db.rollback()
                raise
        finally:
            await db.close()

        assert seq is not None
        return int(seq), eid

    async def list_events(
        self,
        session_id: str,
        from_seq: int = 1,
        to_seq: int | None = None,
        limit: int = 500,
        event_types: list[str] | None = None,
    ) -> list[dict[str, Any]]:
        """查询事件列表（升序）。"""
        safe_limit = max(1, min(int(limit), 500))
        if from_seq < 1:
            from_seq = 1
        effective_to = to_seq if to_seq is not None else 2**31 - 1

        db = await connect(self.db_path)
        try:
            if event_types:
                placeholders = ",".join("?" for _ in event_types)
                cur = await db.execute(
                    f"""
                    SELECT seq, session_id, event_type, payload_json, created_at,
                           event_id, source, metadata_json, cluster_id
                    FROM session_events
                    WHERE session_id = ?
                      AND seq BETWEEN ? AND ?
                      AND event_type IN ({placeholders})
                    ORDER BY seq ASC
                    LIMIT ?
                    """,
                    (session_id, from_seq, effective_to, *event_types, safe_limit),
                )
            else:
                cur = await db.execute(
                    """
                    SELECT seq, session_id, event_type, payload_json, created_at,
                           event_id, source, metadata_json, cluster_id
                    FROM session_events
                    WHERE session_id = ?
                      AND seq BETWEEN ? AND ?
                    ORDER BY seq ASC
                    LIMIT ?
                    """,
                    (session_id, from_seq, effective_to, safe_limit),
                )
            rows = await cur.fetchall()
        finally:
            await db.close()

        return [_row_to_dict(r) for r in rows]

    async def subscribe(
        self,
        session_id: str,
        from_seq: int = 0,
    ) -> AsyncIterator[dict[str, Any]]:
        """Long-poll based subscription for follow mode."""
        last_seq = from_seq
        while True:
            events = await self.list_events(
                session_id, from_seq=last_seq + 1, limit=100
            )
            for event in events:
                yield event
                last_seq = event["seq"]
            if not events:
                await asyncio.sleep(0.5)

    async def count(self, session_id: str) -> int:
        """返回 session 的事件总数。"""
        db = await connect(self.db_path)
        try:
            cur = await db.execute(
                "SELECT COUNT(*) FROM session_events WHERE session_id = ?",
                (session_id,),
            )
            row = await cur.fetchone()
        finally:
            await db.close()
        return int(row[0]) if row else 0

    async def search(
        self,
        session_id: str,
        query: str,
        limit: int = 50,
    ) -> list[dict[str, Any]]:
        """FTS5 full-text search over event_type + payload_json."""
        safe_limit = max(1, min(int(limit), 500))
        db = await connect(self.db_path)
        try:
            cur = await db.execute(
                """
                SELECT se.seq, se.session_id, se.event_type, se.payload_json,
                       se.created_at, se.event_id, se.source, se.metadata_json,
                       se.cluster_id
                FROM session_events_fts fts
                JOIN session_events se ON se.seq = fts.rowid
                WHERE fts.session_events_fts MATCH ?
                  AND se.session_id = ?
                ORDER BY se.seq ASC
                LIMIT ?
                """,
                (query, session_id, safe_limit),
            )
            rows = await cur.fetchall()
        finally:
            await db.close()
        return [_row_to_dict(r) for r in rows]

    async def update_cluster(self, event_id: str, cluster_id: str) -> None:
        """回写 cluster_id。"""
        db = await connect(self.db_path)
        try:
            await db.execute("BEGIN IMMEDIATE")
            try:
                await db.execute(
                    "UPDATE session_events SET cluster_id = ? WHERE event_id = ?",
                    (cluster_id, event_id),
                )
                await db.commit()
            except Exception:
                await db.rollback()
                raise
        finally:
            await db.close()


def _row_to_dict(r: Any) -> dict[str, Any]:
    """Convert a sqlite Row to a dict with parsed payload."""
    payload_raw = r["payload_json"] if r["payload_json"] else "{}"
    metadata_raw = r["metadata_json"] if r["metadata_json"] else "{}"
    return {
        "seq": int(r["seq"]),
        "session_id": r["session_id"],
        "event_type": r["event_type"],
        "payload": json.loads(payload_raw),
        "created_at": int(r["created_at"]),
        "event_id": r["event_id"] or "",
        "source": r["source"] or "",
        "metadata": json.loads(metadata_raw),
        "cluster_id": r["cluster_id"],
    }
