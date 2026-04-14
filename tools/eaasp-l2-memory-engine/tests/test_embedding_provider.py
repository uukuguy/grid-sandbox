"""Tests for embedding/provider.py — MockEmbedding + singleton config."""

from __future__ import annotations

import math
from collections.abc import Iterator

import pytest

from eaasp_l2_memory_engine.embedding import (
    MockEmbedding,
    OllamaEmbedding,
    get_embedding_provider,
    reset_embedding_provider,
)


pytestmark = pytest.mark.asyncio


@pytest.fixture(autouse=True)
def _reset_singleton() -> Iterator[None]:
    """Ensure every test starts with a clean singleton."""
    reset_embedding_provider()
    yield
    reset_embedding_provider()


async def test_mock_embedding_deterministic() -> None:
    mock = MockEmbedding()
    vec1 = await mock.embed("hello world")
    vec2 = await mock.embed("hello world")
    assert vec1 == vec2

    # Different input → different output.
    vec3 = await mock.embed("goodbye world")
    assert vec1 != vec3


async def test_mock_embedding_dimension() -> None:
    mock = MockEmbedding()
    vec = await mock.embed("test")
    assert len(vec) == 1024
    assert mock.dimension == 1024


async def test_mock_embedding_normalized() -> None:
    mock = MockEmbedding()
    vec = await mock.embed("normalize me")
    norm = math.sqrt(sum(v * v for v in vec))
    assert abs(norm - 1.0) < 1e-6


async def test_mock_embedding_model_id() -> None:
    mock = MockEmbedding()
    assert mock.model_id == "mock-bge-m3:fp16"


async def test_embed_batch_preserves_order() -> None:
    mock = MockEmbedding()
    texts = ["alpha", "beta", "gamma"]
    batch = await mock.embed_batch(texts)
    individual = [await mock.embed(t) for t in texts]
    assert batch == individual
    assert len(batch) == 3


async def test_get_embedding_provider_defaults_to_mock(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.delenv("EAASP_EMBEDDING_PROVIDER", raising=False)
    monkeypatch.delenv("EAASP_EMBEDDING_MODEL", raising=False)
    provider = get_embedding_provider()
    assert isinstance(provider, MockEmbedding)
    assert provider.model_id == "mock-bge-m3:fp16"


async def test_get_embedding_provider_ollama_env(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("EAASP_EMBEDDING_PROVIDER", "ollama")
    monkeypatch.delenv("EAASP_EMBEDDING_MODEL", raising=False)
    monkeypatch.delenv("EAASP_OLLAMA_URL", raising=False)
    provider = get_embedding_provider()
    assert isinstance(provider, OllamaEmbedding)
    assert provider.model_id.endswith("@ollama")
    assert provider.dimension == 1024


async def test_reset_embedding_provider(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("EAASP_EMBEDDING_PROVIDER", "mock")
    first = get_embedding_provider()
    assert isinstance(first, MockEmbedding)

    # Without reset, changing env has no effect.
    monkeypatch.setenv("EAASP_EMBEDDING_PROVIDER", "ollama")
    still_first = get_embedding_provider()
    assert still_first is first

    # After reset, new env takes effect.
    reset_embedding_provider()
    second = get_embedding_provider()
    assert isinstance(second, OllamaEmbedding)
