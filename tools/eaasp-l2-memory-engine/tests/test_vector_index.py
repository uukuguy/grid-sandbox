"""S2.T1 — HNSWVectorIndex contract tests (ADR-V2-015)."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import pytest

from eaasp_l2_memory_engine.vector_index import (
    DimensionMismatchError,
    HNSWVectorIndex,
    ModelIdMismatchError,
    model_id_to_safe_dirname,
)


# NOTE: we rely on ``asyncio_mode = "auto"`` (set in pyproject.toml) so that
# every ``async def test_*`` is auto-marked. The single sync test below stays
# un-decorated; applying a module-level ``pytestmark`` here would mark it
# too and emit a spurious warning.


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _rand_vec(rng: np.random.Generator, dim: int = 1024) -> list[float]:
    """Generate a deterministic random unit-ish vector as list[float]."""
    return rng.standard_normal(dim).astype("float32").tolist()


# ---------------------------------------------------------------------------
# 1. model_id → safe directory name
# ---------------------------------------------------------------------------


def test_model_id_to_safe_dirname() -> None:
    assert model_id_to_safe_dirname("bge-m3:fp16@ollama") == "bge-m3-fp16-ollama"
    # Path separator and dot both collapse to dashes.
    assert model_id_to_safe_dirname("namespace/model.v1") == "namespace-model-v1"
    # Already-safe name is a no-op.
    assert model_id_to_safe_dirname("plain-name") == "plain-name"


# ---------------------------------------------------------------------------
# 2. Add + search round-trip
# ---------------------------------------------------------------------------


async def test_hnsw_add_and_search_round_trip(tmp_path: Path) -> None:
    idx = HNSWVectorIndex(model_id="bge-m3:fp16@ollama", octo_root=tmp_path)
    rng = np.random.default_rng(42)
    vecs = [_rand_vec(rng) for _ in range(5)]

    for i, v in enumerate(vecs):
        await idx.add(f"id-{i}", v)

    assert idx.count() == 5

    # Searching the exact vector of id-2 should return id-2 as the top hit.
    hits = await idx.search(vecs[2], top_k=3)
    assert len(hits) == 3
    assert hits[0].id == "id-2"
    # Cosine similarity of a vector with itself is 1.0 (allow float slack).
    assert hits[0].score == pytest.approx(1.0, abs=1e-4)


# ---------------------------------------------------------------------------
# 3 & 4. Dimension mismatch on add / search
# ---------------------------------------------------------------------------


async def test_hnsw_dimension_mismatch_on_add_raises(tmp_path: Path) -> None:
    idx = HNSWVectorIndex(model_id="bge-m3:fp16@ollama", octo_root=tmp_path, dim=1024)
    wrong_dim = [0.1] * 512
    with pytest.raises(DimensionMismatchError):
        await idx.add("x", wrong_dim)


async def test_hnsw_dimension_mismatch_on_search_raises(
    tmp_path: Path,
) -> None:
    idx = HNSWVectorIndex(model_id="bge-m3:fp16@ollama", octo_root=tmp_path, dim=1024)
    # Populate so search actually would run.
    rng = np.random.default_rng(0)
    await idx.add("a", _rand_vec(rng))
    wrong_dim = [0.2] * 768
    with pytest.raises(DimensionMismatchError):
        await idx.search(wrong_dim, top_k=1)


# ---------------------------------------------------------------------------
# 5. Empty index → []
# ---------------------------------------------------------------------------


async def test_hnsw_empty_search_returns_empty_list(tmp_path: Path) -> None:
    idx = HNSWVectorIndex(model_id="bge-m3:fp16@ollama", octo_root=tmp_path)
    rng = np.random.default_rng(0)
    hits = await idx.search(_rand_vec(rng), top_k=5)
    assert hits == []


# ---------------------------------------------------------------------------
# 6. Delete removes id from search results
# ---------------------------------------------------------------------------


async def test_hnsw_delete_then_search_excludes(tmp_path: Path) -> None:
    idx = HNSWVectorIndex(model_id="bge-m3:fp16@ollama", octo_root=tmp_path)
    rng = np.random.default_rng(7)
    vecs = [_rand_vec(rng) for _ in range(3)]
    for i, v in enumerate(vecs):
        await idx.add(f"id-{i}", v)

    await idx.delete("id-1")
    assert idx.count() == 2

    # Querying with vecs[1] (the deleted item's vector) must not return id-1.
    hits = await idx.search(vecs[1], top_k=5)
    returned_ids = {h.id for h in hits}
    assert "id-1" not in returned_ids
    # The other two ids are still there.
    assert returned_ids == {"id-0", "id-2"}


# ---------------------------------------------------------------------------
# 7. Per-model_id directory isolation
# ---------------------------------------------------------------------------


async def test_hnsw_per_model_id_directory_isolation(tmp_path: Path) -> None:
    idx_a = HNSWVectorIndex(model_id="model-a:v1", octo_root=tmp_path)
    idx_b = HNSWVectorIndex(model_id="model-b:v1", octo_root=tmp_path)

    # Different directories on disk.
    assert idx_a.index_dir != idx_b.index_dir
    assert idx_a.index_dir.exists()
    assert idx_b.index_dir.exists()

    rng = np.random.default_rng(13)
    vec = _rand_vec(rng)
    await idx_a.add("only-in-a", vec)

    # Model B has no entries, so even searching A's vector returns [].
    hits_b = await idx_b.search(vec, top_k=5)
    assert hits_b == []
    # Model A sees the entry.
    hits_a = await idx_a.search(vec, top_k=5)
    assert any(h.id == "only-in-a" for h in hits_a)


# ---------------------------------------------------------------------------
# 8. Save + reload preserves entries
# ---------------------------------------------------------------------------


async def test_hnsw_save_and_reload(tmp_path: Path) -> None:
    idx = HNSWVectorIndex(model_id="bge-m3:fp16@ollama", octo_root=tmp_path)
    rng = np.random.default_rng(99)
    vecs = [_rand_vec(rng) for _ in range(3)]
    for i, v in enumerate(vecs):
        await idx.add(f"id-{i}", v)
    await idx.save()

    # Fresh instance with the same model_id should load from disk.
    reloaded = HNSWVectorIndex(model_id="bge-m3:fp16@ollama", octo_root=tmp_path)
    assert reloaded.count() == 3

    hits = await reloaded.search(vecs[0], top_k=1)
    assert len(hits) == 1
    assert hits[0].id == "id-0"
    assert hits[0].score == pytest.approx(1.0, abs=1e-4)


# ---------------------------------------------------------------------------
# 9. Reload with wrong model_id raises
# ---------------------------------------------------------------------------


async def test_hnsw_reload_model_id_mismatch_raises(tmp_path: Path) -> None:
    """Simulate a corrupted / wrong-target reload by writing a meta.json for
    model A but instantiating with model B in the same directory."""
    idx_a = HNSWVectorIndex(model_id="model-a:v1", octo_root=tmp_path)
    rng = np.random.default_rng(5)
    await idx_a.add("x", _rand_vec(rng))
    await idx_a.save()

    # Now manually point model B at model A's directory: the safe names
    # collide only if we pick colliding ids, so instead we construct model B
    # with the *same* model_id-safe dirname would require crafting the path.
    # Simplest path: load, then rewrite the meta.json to declare model B,
    # then load as model A (mismatch).
    import json as _json

    meta = _json.loads(idx_a.meta_path.read_text())
    meta["model_id"] = "foreign-model:v2"
    idx_a.meta_path.write_text(_json.dumps(meta))

    with pytest.raises(ModelIdMismatchError):
        HNSWVectorIndex(model_id="model-a:v1", octo_root=tmp_path)


# ---------------------------------------------------------------------------
# 10. Growable max_elements
# ---------------------------------------------------------------------------


async def test_hnsw_growth_resize(tmp_path: Path) -> None:
    """Start with a tiny capacity and confirm that surpassing it triggers
    resize without crashing, and that every insert remains queryable."""
    idx = HNSWVectorIndex(
        model_id="bge-m3:fp16@ollama",
        octo_root=tmp_path,
        max_elements=5,
    )
    rng = np.random.default_rng(21)
    vecs = [_rand_vec(rng) for _ in range(6)]

    for i, v in enumerate(vecs):
        await idx.add(f"id-{i}", v)

    assert idx.count() == 6
    # Capacity should have grown (at least double of the original 5).
    assert idx._max_elements >= 10

    # Every id must still be retrievable as top-1 of its own vector.
    for i, v in enumerate(vecs):
        hits = await idx.search(v, top_k=1)
        assert hits, f"id-{i} query returned no hits"
        assert hits[0].id == f"id-{i}"
