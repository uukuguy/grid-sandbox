"""Embedding provider abstractions for L2 Memory Engine.

Supports:
- MockEmbedding: deterministic hash-based, for tests (default).
- OllamaEmbedding: POSTs to local Ollama (dev), bge-m3:fp16 @ 1024 dims.

Configuration via environment variables:
- EAASP_EMBEDDING_PROVIDER: "mock" (default) | "ollama"
- EAASP_EMBEDDING_MODEL: model name (defaults vary by provider)
- EAASP_OLLAMA_URL: Ollama base URL (default "http://localhost:11434")

Note on Ollama/macOS proxy: httpx.AsyncClient uses `trust_env=False` to bypass
system proxies (e.g. Clash) which otherwise turn 127.0.0.1 calls into 502. See
MEMORY.md "Ollama 已知问题" for precedent in reqwest / grid-engine OpenAIProvider.
"""

from __future__ import annotations

import hashlib
import math
import os
import random
from typing import Protocol

import httpx

# bge-m3 family embedding dimension (fixed).
BGE_M3_DIMENSION = 1024


class EmbeddingProvider(Protocol):
    """Interface for embedding providers (dev/test/prod)."""

    async def embed(self, text: str) -> list[float]:
        """Embed a single text string."""
        ...

    async def embed_batch(self, texts: list[str]) -> list[list[float]]:
        """Embed multiple texts in batch (may be sequential for simple providers)."""
        ...

    @property
    def dimension(self) -> int:
        """Embedding dimension (e.g., 1024 for bge-m3:fp16)."""
        ...

    @property
    def model_id(self) -> str:
        """Model identifier (e.g., 'bge-m3:fp16@ollama')."""
        ...


class OllamaEmbedding:
    """Ollama embedding provider (dev environment).

    POSTs to {ollama_url}/api/embeddings with body {"model": ..., "prompt": text},
    reads response["embedding"] as list[float].
    """

    def __init__(
        self,
        model: str = "bge-m3:fp16",
        ollama_url: str = "http://localhost:11434",
    ) -> None:
        self.model = model
        self.ollama_url = ollama_url.rstrip("/")
        # Fixed for bge-m3 family. Extend if adding other models.
        self._dim = BGE_M3_DIMENSION

    async def embed(self, text: str) -> list[float]:
        # trust_env=False: bypass macOS proxy (Clash/etc) which breaks localhost.
        async with httpx.AsyncClient(timeout=30.0, trust_env=False) as client:
            resp = await client.post(
                f"{self.ollama_url}/api/embeddings",
                json={"model": self.model, "prompt": text},
            )
            resp.raise_for_status()
            data = resp.json()
            return data["embedding"]

    async def embed_batch(self, texts: list[str]) -> list[list[float]]:
        vecs: list[list[float]] = []
        for text in texts:
            vecs.append(await self.embed(text))
        return vecs

    @property
    def dimension(self) -> int:
        return self._dim

    @property
    def model_id(self) -> str:
        return f"{self.model}@ollama"


class MockEmbedding:
    """Deterministic mock embedding for tests.

    Uses SHA-256(text) as the seed for a PRNG that generates `dimension` gaussian
    samples, then L2-normalizes to unit length. Output values are in [-1, 1]
    (signed), suitable for cosine similarity.
    """

    def __init__(self, model: str = "mock-bge-m3:fp16") -> None:
        self.model = model
        self._dim = BGE_M3_DIMENSION

    async def embed(self, text: str) -> list[float]:
        digest = hashlib.sha256(text.encode("utf-8")).digest()
        seed = int.from_bytes(digest[:8], "little", signed=False)
        rng = random.Random(seed)
        samples = [rng.gauss(0.0, 1.0) for _ in range(self._dim)]
        norm = math.sqrt(sum(v * v for v in samples))
        if norm == 0.0:
            # Degenerate; return uniform non-zero vector normalized.
            return [1.0 / math.sqrt(self._dim)] * self._dim
        return [v / norm for v in samples]

    async def embed_batch(self, texts: list[str]) -> list[list[float]]:
        return [await self.embed(text) for text in texts]

    @property
    def dimension(self) -> int:
        return self._dim

    @property
    def model_id(self) -> str:
        return self.model


# Module-level singleton (reset via reset_embedding_provider() in tests).
_PROVIDER_INSTANCE: EmbeddingProvider | None = None


def get_embedding_provider() -> EmbeddingProvider:
    """Get or create singleton embedding provider from env config.

    Env:
        EAASP_EMBEDDING_PROVIDER: "mock" (default) | "ollama"
        EAASP_EMBEDDING_MODEL: model name (defaults to provider-specific)
        EAASP_OLLAMA_URL: Ollama base URL (default http://localhost:11434)
    """
    global _PROVIDER_INSTANCE
    if _PROVIDER_INSTANCE is not None:
        return _PROVIDER_INSTANCE

    provider_type = os.getenv("EAASP_EMBEDDING_PROVIDER", "mock").lower()

    if provider_type == "ollama":
        model = os.getenv("EAASP_EMBEDDING_MODEL", "bge-m3:fp16")
        ollama_url = os.getenv("EAASP_OLLAMA_URL", "http://localhost:11434")
        _PROVIDER_INSTANCE = OllamaEmbedding(model=model, ollama_url=ollama_url)
    else:
        model = os.getenv("EAASP_EMBEDDING_MODEL", "mock-bge-m3:fp16")
        _PROVIDER_INSTANCE = MockEmbedding(model=model)

    return _PROVIDER_INSTANCE


def reset_embedding_provider() -> None:
    """For tests: drop the singleton so env changes take effect on next get()."""
    global _PROVIDER_INSTANCE
    _PROVIDER_INSTANCE = None
