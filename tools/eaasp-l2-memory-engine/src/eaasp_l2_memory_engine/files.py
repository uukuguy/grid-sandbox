"""Layer 2 — File-based Memory (versioned, with status state machine)."""

from __future__ import annotations

import json
import time
import uuid
from typing import Any, Literal

from pydantic import BaseModel, Field

from .db import connect

MemoryStatus = Literal["agent_suggested", "confirmed", "archived"]

_ALLOWED_TRANSITIONS: dict[str, set[str]] = {
    "agent_suggested": {"confirmed", "archived"},
    "confirmed": {"archived"},
    "archived": set(),
}


class MemoryFileIn(BaseModel):
    memory_id: str | None = None
    scope: str
    category: str
    content: str
    evidence_refs: list[str] = Field(default_factory=list)
    status: MemoryStatus = "agent_suggested"


class MemoryFileOut(BaseModel):
    memory_id: str
    version: int
    scope: str
    category: str
    content: str
    evidence_refs: list[str]
    status: MemoryStatus
    created_at: int
    updated_at: int


class InvalidStatusTransition(ValueError):
    pass


class MemoryFileStore:
    def __init__(self, db_path: str) -> None:
        self.db_path = db_path

    async def write(self, memory: MemoryFileIn) -> MemoryFileOut:
        """Insert new memory or bump version of an existing memory_id.

        Wrapped in BEGIN IMMEDIATE to avoid racy (SELECT MAX + INSERT) version
        collisions (C1). When memory_id is provided, status transition is
        validated against the latest version (M4).
        """
        now = int(time.time() * 1000)
        memory_id = memory.memory_id or f"mem_{uuid.uuid4().hex[:16]}"
        db = await connect(self.db_path)
        try:
            await db.execute("BEGIN IMMEDIATE")
            try:
                cur = await db.execute(
                    """
                    SELECT MAX(version) AS v, MIN(created_at) AS c,
                           (SELECT status FROM memory_files
                              WHERE memory_id = ?
                              ORDER BY version DESC LIMIT 1) AS latest_status
                      FROM memory_files WHERE memory_id = ?
                    """,
                    (memory_id, memory_id),
                )
                row = await cur.fetchone()
                latest_version = row["v"] if row and row["v"] is not None else 0
                created_at = row["c"] if row and row["c"] is not None else now
                latest_status = row["latest_status"] if row else None
                new_version = latest_version + 1

                # M4: enforce status transitions when bumping an existing memory_id.
                if latest_status is not None and memory.status != latest_status:
                    if memory.status not in _ALLOWED_TRANSITIONS[latest_status]:
                        raise InvalidStatusTransition(
                            f"Cannot transition {latest_status} → {memory.status} "
                            f"for {memory_id}"
                        )

                await db.execute(
                    """
                    INSERT INTO memory_files (
                        memory_id, version, scope, category, content, evidence_refs,
                        status, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        memory_id,
                        new_version,
                        memory.scope,
                        memory.category,
                        memory.content,
                        json.dumps(memory.evidence_refs),
                        memory.status,
                        created_at,
                        now,
                    ),
                )
                await db.execute(
                    """
                    INSERT INTO memory_fts (memory_id, version, content_text, category, scope)
                    VALUES (?, ?, ?, ?, ?)
                    """,
                    (memory_id, new_version, memory.content, memory.category, memory.scope),
                )
                await db.commit()
            except Exception:
                await db.rollback()
                raise
        finally:
            await db.close()

        return MemoryFileOut(
            memory_id=memory_id,
            version=new_version,
            scope=memory.scope,
            category=memory.category,
            content=memory.content,
            evidence_refs=memory.evidence_refs,
            status=memory.status,
            created_at=created_at,
            updated_at=now,
        )

    async def read_latest(self, memory_id: str) -> MemoryFileOut | None:
        db = await connect(self.db_path)
        try:
            cur = await db.execute(
                """
                SELECT * FROM memory_files
                WHERE memory_id = ?
                ORDER BY version DESC
                LIMIT 1
                """,
                (memory_id,),
            )
            row = await cur.fetchone()
        finally:
            await db.close()
        return _row_to_memory(row) if row else None

    async def list(
        self,
        scope: str | None = None,
        category: str | None = None,
        status: MemoryStatus | None = None,
        limit: int = 50,
    ) -> list[MemoryFileOut]:
        """Latest version of each memory_id matching filters."""
        where: list[str] = []
        params: list[Any] = []
        if scope is not None:
            where.append("scope = ?")
            params.append(scope)
        if category is not None:
            where.append("category = ?")
            params.append(category)
        if status is not None:
            where.append("status = ?")
            params.append(status)

        where_clause = ("WHERE " + " AND ".join(where)) if where else ""
        sql = f"""
            SELECT mf.*
            FROM memory_files mf
            INNER JOIN (
                SELECT memory_id, MAX(version) AS max_v
                FROM memory_files
                GROUP BY memory_id
            ) latest
              ON mf.memory_id = latest.memory_id AND mf.version = latest.max_v
            {where_clause}
            ORDER BY mf.updated_at DESC
            LIMIT ?
        """
        params.append(limit)

        db = await connect(self.db_path)
        try:
            cur = await db.execute(sql, params)
            rows = await cur.fetchall()
        finally:
            await db.close()
        return [_row_to_memory(r) for r in rows]

    async def archive(self, memory_id: str) -> MemoryFileOut:
        """Transition status → archived (creates new version)."""
        latest = await self.read_latest(memory_id)
        if latest is None:
            raise KeyError(f"memory_id not found: {memory_id}")
        if "archived" not in _ALLOWED_TRANSITIONS[latest.status]:
            raise InvalidStatusTransition(
                f"Cannot transition {latest.status} → archived for {memory_id}"
            )
        return await self.write(
            MemoryFileIn(
                memory_id=memory_id,
                scope=latest.scope,
                category=latest.category,
                content=latest.content,
                evidence_refs=latest.evidence_refs,
                status="archived",
            )
        )

    async def confirm(self, memory_id: str) -> MemoryFileOut:
        latest = await self.read_latest(memory_id)
        if latest is None:
            raise KeyError(f"memory_id not found: {memory_id}")
        if "confirmed" not in _ALLOWED_TRANSITIONS[latest.status]:
            raise InvalidStatusTransition(
                f"Cannot transition {latest.status} → confirmed for {memory_id}"
            )
        return await self.write(
            MemoryFileIn(
                memory_id=memory_id,
                scope=latest.scope,
                category=latest.category,
                content=latest.content,
                evidence_refs=latest.evidence_refs,
                status="confirmed",
            )
        )


def _row_to_memory(row: Any) -> MemoryFileOut:
    refs_raw = row["evidence_refs"]
    refs = json.loads(refs_raw) if refs_raw else []
    return MemoryFileOut(
        memory_id=row["memory_id"],
        version=row["version"],
        scope=row["scope"],
        category=row["category"],
        content=row["content"],
        evidence_refs=refs,
        status=row["status"],
        created_at=row["created_at"],
        updated_at=row["updated_at"],
    )
