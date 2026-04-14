"""Phase 2 S2.T1 — MemoryFileStore dual-write integration tests.

Verifies the write path now:
    1. Embeds content via the configured provider.
    2. Persists embedding_model_id / embedding_dim / embedding_vec atomically
       alongside the base row.
    3. Best-effort adds the vector to the per-model HNSW index.
    4. Does NOT abort the DB write when either the embedding provider or
       the HNSW backend fails.
"""

from __future__ import annotations

import struct
from pathlib import Path
from typing import Any

import aiosqlite
import pytest

from eaasp_l2_memory_engine.db import init_db
from eaasp_l2_memory_engine.files import MemoryFileIn, MemoryFileStore

pytestmark = pytest.mark.asyncio


@pytest.fixture(autouse=True)
def _mock_embedding_env(monkeypatch: pytest.MonkeyPatch) -> None:
    """Force every test in this file to use the deterministic MockEmbedding
    provider and start with a fresh singleton."""
    monkeypatch.setenv("EAASP_EMBEDDING_PROVIDER", "mock")
    monkeypatch.setenv("EAASP_EMBEDDING_MODEL", "mock-bge-m3:fp16")
    from eaasp_l2_memory_engine.embedding import reset_embedding_provider

    reset_embedding_provider()
    yield
    reset_embedding_provider()


async def _make_store(tmp_path: Path) -> MemoryFileStore:
    db_file = str(tmp_path / "mem.db")
    await init_db(db_file)
    return MemoryFileStore(db_file, octo_root=str(tmp_path))


async def _fetch_embedding_cols(db_path: str, memory_id: str) -> dict[str, Any]:
    async with aiosqlite.connect(db_path) as db:
        db.row_factory = aiosqlite.Row
        cur = await db.execute(
            """
            SELECT embedding_model_id, embedding_dim, embedding_vec
            FROM memory_files WHERE memory_id = ? AND version = 1
            """,
            (memory_id,),
        )
        row = await cur.fetchone()
    assert row is not None, f"row not found for {memory_id}"
    return {
        "model_id": row["embedding_model_id"],
        "dim": row["embedding_dim"],
        "vec": row["embedding_vec"],
    }


async def test_memory_write_with_mock_embedding(tmp_path: Path) -> None:
    """Happy path: mock provider fills all three embedding columns."""
    store = await _make_store(tmp_path)
    out = await store.write(
        MemoryFileIn(
            scope="test",
            category="threshold",
            content="salary_floor=50000",
        )
    )

    cols = await _fetch_embedding_cols(store.db_path, out.memory_id)
    assert cols["model_id"] == "mock-bge-m3:fp16"
    assert cols["dim"] == 1024
    assert isinstance(cols["vec"], (bytes, memoryview))
    assert len(bytes(cols["vec"])) == 1024 * 4  # f32 × 1024

    # Sanity: unpack round-trips to a finite list of length 1024.
    recovered = list(struct.unpack("1024f", bytes(cols["vec"])))
    assert len(recovered) == 1024


async def test_memory_write_hnsw_add_succeeds(tmp_path: Path) -> None:
    """HNSW index should be populated with ``{memory_id}:v{version}``."""
    from eaasp_l2_memory_engine.embedding import get_embedding_provider
    from eaasp_l2_memory_engine.vector_index import HNSWVectorIndex

    store = await _make_store(tmp_path)
    out = await store.write(
        MemoryFileIn(scope="s", category="c", content="hello world")
    )

    # Re-hydrate the HNSW index from disk the same way a downstream search
    # would. Must use identical model_id + octo_root + dim.
    embedder = get_embedding_provider()
    idx = HNSWVectorIndex(
        model_id=embedder.model_id,
        octo_root=str(tmp_path),
        dim=embedder.dimension,
    )

    # Some index backends load lazily on first query — give them a chance.
    # Searching by the same content should return our id at rank 1.
    query_vec = await embedder.embed("hello world")
    hits = await idx.search(query_vec, top_k=5)
    assert len(hits) >= 1
    expected_id = f"{out.memory_id}:v1"
    assert any(h.id == expected_id for h in hits), (
        f"expected {expected_id} in hits {[h.id for h in hits]}"
    )


async def test_memory_write_embedding_provider_crash_does_not_block(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """If the embedder raises, the row must still be inserted with NULL
    embedding columns and no exception surfaced to the caller."""
    import eaasp_l2_memory_engine.embedding as emb_mod

    class _BoomEmbedding:
        model_id = "boom-model"
        dimension = 1024

        async def embed(self, text: str) -> list[float]:
            raise RuntimeError("simulated embed failure")

        async def embed_batch(self, texts: list[str]) -> list[list[float]]:
            raise RuntimeError("simulated embed failure")

    # Force the provider resolver to return our exploding stub.
    monkeypatch.setattr(emb_mod, "get_embedding_provider", lambda: _BoomEmbedding())

    store = await _make_store(tmp_path)
    out = await store.write(
        MemoryFileIn(scope="s", category="c", content="still writes")
    )
    assert out.memory_id.startswith("mem_")
    assert out.version == 1

    cols = await _fetch_embedding_cols(store.db_path, out.memory_id)
    assert cols["model_id"] is None
    assert cols["dim"] is None
    assert cols["vec"] is None


async def test_memory_write_hnsw_failure_does_not_block(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """If the HNSW backend raises, the DB must still have embedding columns
    populated and the call must return normally."""
    import eaasp_l2_memory_engine.vector_index as vi_mod

    class _BoomIndex:
        def __init__(self, *args: Any, **kwargs: Any) -> None:
            raise RuntimeError("simulated HNSW failure")

    monkeypatch.setattr(vi_mod, "HNSWVectorIndex", _BoomIndex)

    store = await _make_store(tmp_path)
    out = await store.write(
        MemoryFileIn(scope="s", category="c", content="db still gets embedding")
    )

    cols = await _fetch_embedding_cols(store.db_path, out.memory_id)
    # DB embedding columns must be populated (embedding computed pre-txn,
    # HNSW failure is post-commit and non-fatal).
    assert cols["model_id"] == "mock-bge-m3:fp16"
    assert cols["dim"] == 1024
    assert cols["vec"] is not None
    assert len(bytes(cols["vec"])) == 1024 * 4
