"""EAASP v2.0 L2 Memory Engine — Embedding provider abstractions."""

from __future__ import annotations

from .provider import (
    EmbeddingProvider,
    MockEmbedding,
    OllamaEmbedding,
    get_embedding_provider,
    reset_embedding_provider,
)

__all__ = [
    "EmbeddingProvider",
    "OllamaEmbedding",
    "MockEmbedding",
    "get_embedding_provider",
    "reset_embedding_provider",
]
