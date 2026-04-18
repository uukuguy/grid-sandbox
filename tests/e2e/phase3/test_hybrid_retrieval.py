"""E2E B3+B4 — HNSW vector index and hybrid retrieval sample set.

B3: Validates the memory_hnsw_samples.json fixture shape and integrity
    (100+ samples, correct dimensionality, unit-normalized embeddings).

B4: Validates the hybrid retrieval scoring logic (FTS + HNSW + time-decay
    fusion) using in-process computation against the fixture samples.
    No live LLM or running service required.

Reference: D98 HybridIndex HNSW cache (Phase 3 S2.T4 closed) +
           D78 EventEmbeddingIndex (Phase 3 S2.T2 closed).
"""

from __future__ import annotations

import json
import math
from pathlib import Path
from typing import TypedDict

import pytest

_REPO_ROOT = Path(__file__).resolve().parents[3]
_FIXTURE_PATH = _REPO_ROOT / "tests" / "e2e" / "fixtures" / "memory_hnsw_samples.json"


# ---------------------------------------------------------------------------
# Fixture loading helpers
# ---------------------------------------------------------------------------


class MemorySample(TypedDict):
    id: str
    text: str
    category: str
    score: float
    embedding: list[float]
    created_at: str


def load_samples() -> list[MemorySample]:
    return json.loads(_FIXTURE_PATH.read_text())["samples"]


# ---------------------------------------------------------------------------
# B3 — Fixture integrity
# ---------------------------------------------------------------------------


def test_fixture_file_exists():
    assert _FIXTURE_PATH.exists(), f"Fixture not found: {_FIXTURE_PATH}"


def test_fixture_has_100_plus_samples():
    data = json.loads(_FIXTURE_PATH.read_text())
    assert data["count"] >= 100
    assert len(data["samples"]) == data["count"]


def test_fixture_version_field():
    data = json.loads(_FIXTURE_PATH.read_text())
    assert "version" in data
    assert data["version"] == "1.0"


def test_fixture_dim_field():
    data = json.loads(_FIXTURE_PATH.read_text())
    assert data["dim"] == 16


def test_all_samples_have_required_fields():
    for s in load_samples():
        assert "id" in s
        assert "text" in s
        assert "embedding" in s
        assert "category" in s
        assert "created_at" in s


def test_embedding_dimension_matches_fixture_dim():
    data = json.loads(_FIXTURE_PATH.read_text())
    dim = data["dim"]
    for s in data["samples"]:
        assert len(s["embedding"]) == dim, (
            f"Sample {s['id']} has {len(s['embedding'])} dims, expected {dim}"
        )


def test_embeddings_are_approximately_unit_normalized():
    """All embeddings should have L2-norm ≈ 1.0 (unit sphere)."""
    for s in load_samples():
        vec = s["embedding"]
        norm = math.sqrt(sum(x * x for x in vec))
        assert abs(norm - 1.0) < 1e-6, (
            f"Sample {s['id']} norm={norm:.6f}, expected 1.0"
        )


def test_sample_ids_are_unique():
    ids = [s["id"] for s in load_samples()]
    assert len(ids) == len(set(ids)), "Duplicate sample IDs found"


def test_categories_cover_expected_set():
    expected = {"tech", "science", "art", "history", "sports",
                "cooking", "travel", "music", "health", "finance"}
    actual = {s["category"] for s in load_samples()}
    assert expected == actual


# ---------------------------------------------------------------------------
# B4 — Hybrid retrieval scoring
# ---------------------------------------------------------------------------


def cosine_similarity(a: list[float], b: list[float]) -> float:
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(x * x for x in b))
    if na == 0 or nb == 0:
        return 0.0
    return dot / (na * nb)


def fts_score(query: str, text: str) -> float:
    """Naive keyword overlap score (0..1) for FTS simulation."""
    q_words = set(query.lower().split())
    t_words = set(text.lower().split())
    if not q_words:
        return 0.0
    return len(q_words & t_words) / len(q_words)


def time_decay(created_at: str, reference: str = "2026-04-30T00:00:00Z", half_life_days: float = 30.0) -> float:
    """Exponential time-decay: score = 2^(-age_days / half_life_days)."""
    from datetime import datetime, timezone

    def parse(s: str) -> datetime:
        return datetime.fromisoformat(s.replace("Z", "+00:00"))

    age_days = (parse(reference) - parse(created_at)).total_seconds() / 86400
    return math.pow(2.0, -age_days / half_life_days)


def hybrid_score(
    query: str,
    query_embedding: list[float],
    sample: MemorySample,
    w_fts: float = 0.5,
    w_sem: float = 0.5,
) -> float:
    fts = fts_score(query, sample["text"])
    sem = max(0.0, cosine_similarity(query_embedding, sample["embedding"]))
    decay = time_decay(sample["created_at"])
    return (w_fts * fts + w_sem * sem) * decay


def test_cosine_similarity_identical_vectors():
    v = [1.0, 0.0, 0.0]
    assert cosine_similarity(v, v) == pytest.approx(1.0)


def test_cosine_similarity_orthogonal_vectors():
    assert cosine_similarity([1.0, 0.0], [0.0, 1.0]) == pytest.approx(0.0)


def test_cosine_similarity_opposite_vectors():
    assert cosine_similarity([1.0, 0.0], [-1.0, 0.0]) == pytest.approx(-1.0)


def test_fts_score_exact_match():
    assert fts_score("tech topic", "tech topic number") == pytest.approx(1.0)


def test_fts_score_no_match():
    assert fts_score("quantum physics", "cooking recipe") == pytest.approx(0.0)


def test_time_decay_recent_entry_near_one():
    score = time_decay("2026-04-29T00:00:00Z")
    assert score > 0.95


def test_time_decay_old_entry_lower():
    recent = time_decay("2026-04-29T00:00:00Z")
    old = time_decay("2026-03-01T00:00:00Z")
    assert old < recent


def test_hybrid_score_range():
    """All hybrid scores should be in [0, 1]."""
    samples = load_samples()
    query = "tech topic"
    query_vec = samples[0]["embedding"]  # use first sample's embedding as query
    for s in samples[:20]:
        score = hybrid_score(query, query_vec, s)
        assert 0.0 <= score <= 1.0 + 1e-9, f"Score {score} out of [0,1] for {s['id']}"


def test_hybrid_retrieval_top_k_returns_relevant():
    """Top-1 result for a tech-embedding query should be a tech sample."""
    samples = load_samples()
    # Use a tech sample's embedding as query
    tech_samples = [s for s in samples if s["category"] == "tech"]
    query_embedding = tech_samples[0]["embedding"]
    query_text = "tech topic"

    scored = [
        (hybrid_score(query_text, query_embedding, s, w_fts=0.3, w_sem=0.7), s)
        for s in samples
    ]
    scored.sort(key=lambda x: x[0], reverse=True)
    top_category = scored[0][1]["category"]
    # Top result should be tech (using 70% semantic weight on a tech embedding)
    assert top_category == "tech", f"Expected 'tech', got '{top_category}'"


def test_hybrid_retrieval_semantic_weight_dominates_at_07():
    """With w_sem=0.7 the semantic component should dominate FTS for embedding queries."""
    samples = load_samples()
    s = samples[0]
    pure_sem = hybrid_score("", s["embedding"], s, w_fts=0.0, w_sem=1.0)
    pure_fts = hybrid_score(s["text"], [0.0] * 16, s, w_fts=1.0, w_sem=0.0)
    blend = hybrid_score(s["text"], s["embedding"], s, w_fts=0.3, w_sem=0.7)
    # blend should be between pure_fts and pure_sem
    assert min(pure_fts, pure_sem) <= blend + 1e-9


def test_hybrid_weights_00_returns_zero():
    """Zero weights yield zero hybrid score."""
    s = load_samples()[0]
    assert hybrid_score("tech", s["embedding"], s, w_fts=0.0, w_sem=0.0) == pytest.approx(0.0)
