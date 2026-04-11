"""Layer 3 — Hybrid Retrieval Index (keyword via FTS5 + time-decay weighting).

MVP scoring model (semantic deferred to Phase 2):

    score = fts_score * time_decay

where:
    fts_score   = normalized BM25 from FTS5 (higher = better match)
    time_decay  = exp(-age_days / HALF_LIFE_DAYS), age derived from memory updated_at
"""

from __future__ import annotations

import math
import re
import time
from typing import Any

from pydantic import BaseModel

from .db import connect
from .files import MemoryFileOut, _row_to_memory

HALF_LIFE_DAYS = 30.0
_MS_PER_DAY = 86_400_000.0
MAX_TOP_K = 100

# Allow letters, digits, whitespace, and common punctuation inside a phrase.
# Everything else (FTS5 operators, quotes, special chars) is stripped.
_FTS_SAFE = re.compile(r"[^\w\s\-_.]", flags=re.UNICODE)


class SearchHit(BaseModel):
    memory: MemoryFileOut
    score: float
    fts_score: float
    time_decay: float


def _sanitize_query(query: str) -> str | None:
    """Return an FTS5-safe phrase query, or None for empty/unusable input.

    C2: strips all FTS5 operator characters (`*`, `^`, `:`, `"`, `(`, `)`, etc.)
    before wrapping in a phrase, preventing syntax errors and DoS via adversarial
    queries.
    """
    cleaned = _FTS_SAFE.sub(" ", query).strip()
    cleaned = " ".join(cleaned.split())  # collapse whitespace
    if not cleaned:
        return None
    return '"' + cleaned + '"'


def _time_decay(updated_at_ms: int, now_ms: int) -> float:
    age_days = max(0.0, (now_ms - updated_at_ms) / _MS_PER_DAY)
    return math.exp(-age_days / HALF_LIFE_DAYS)


class HybridIndex:
    def __init__(self, db_path: str) -> None:
        self.db_path = db_path

    async def search(
        self,
        query: str,
        top_k: int = 10,
        scope: str | None = None,
        category: str | None = None,
    ) -> list[SearchHit]:
        # C3: bound top_k (unbounded causes memory blow-up on crafted input).
        top_k = max(1, min(int(top_k), MAX_TOP_K))

        fts_query = _sanitize_query(query)
        if fts_query is None:
            return []

        # M2: join against (memory_id, MAX(version)) inside the query instead
        # of doing one _is_latest() call per candidate row.
        sql = """
            SELECT mf.*, bm25(memory_fts) AS rank
            FROM memory_fts
            JOIN memory_files mf
              ON memory_fts.memory_id = mf.memory_id
             AND memory_fts.version = mf.version
            JOIN (
                SELECT memory_id, MAX(version) AS mv
                FROM memory_files
                GROUP BY memory_id
            ) latest
              ON mf.memory_id = latest.memory_id AND mf.version = latest.mv
            WHERE memory_fts MATCH ?
        """
        params: list[Any] = [fts_query]
        if scope is not None:
            sql += " AND mf.scope = ?"
            params.append(scope)
        if category is not None:
            sql += " AND mf.category = ?"
            params.append(category)
        # Oversample by 4x so time-decay reranking has candidates to reorder.
        sql += " ORDER BY rank ASC LIMIT ?"
        params.append(top_k * 4)

        db = await connect(self.db_path)
        try:
            try:
                cur = await db.execute(sql, params)
                rows = await cur.fetchall()
            except Exception:
                # Defense in depth against any residual FTS5 parse error.
                return []
        finally:
            await db.close()

        now_ms = int(time.time() * 1000)
        hits: list[SearchHit] = []
        # BM25 in sqlite returns lower = better (negative or small positive).
        # Normalize: fts_score = 1 / (1 + rank) so larger = better.
        for row in rows:
            memory = _row_to_memory(row)
            bm25 = row["rank"]
            fts_score = 1.0 / (1.0 + max(bm25, 0.0))
            decay = _time_decay(memory.updated_at, now_ms)
            hits.append(
                SearchHit(
                    memory=memory,
                    score=fts_score * decay,
                    fts_score=fts_score,
                    time_decay=decay,
                )
            )

        hits.sort(key=lambda h: h.score, reverse=True)
        return hits[:top_k]
