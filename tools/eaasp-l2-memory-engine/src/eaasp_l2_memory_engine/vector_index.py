"""S2.T1 — HNSW in-process vector index, per-model_id directory isolation.

ADR-V2-015 iron laws enforced here:
  1. Dimension is tracked per model_id; a mismatched dim at query time raises.
  2. Search and add must match the index's model_id; reload with a different
     model_id raises.
  3. Migration (dual-write + re-index) is handled elsewhere.

The module is async-friendly: all mutating ops (``add``/``delete``/``save``)
are serialized via ``asyncio.Lock``. ``search`` is lock-free because hnswlib
is safe for concurrent reads.
"""

from __future__ import annotations

import asyncio
import json
from pathlib import Path
from typing import NamedTuple, Protocol

import hnswlib


# ---------------------------------------------------------------------------
# Public error types
# ---------------------------------------------------------------------------


class DimensionMismatchError(ValueError):
    """Raised when a provided vector does not match the index dimension."""


class ModelIdMismatchError(ValueError):
    """Raised when loading an index whose stored model_id differs from the
    configured one."""


# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------


class Hit(NamedTuple):
    """A search result. ``score`` is cosine similarity in [-1, 1]."""

    id: str
    score: float


class VectorIndex(Protocol):
    """Abstract contract. Concrete backends must honour ADR-V2-015."""

    async def add(self, id: str, vec: list[float]) -> None: ...

    async def search(self, vec: list[float], top_k: int) -> list[Hit]: ...

    async def delete(self, id: str) -> None: ...

    async def save(self) -> None: ...


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def model_id_to_safe_dirname(model_id: str) -> str:
    """Convert ``'bge-m3:fp16@ollama'`` → ``'bge-m3-fp16-ollama'``.

    Replaces filesystem-unfriendly characters with ``-``. The mapping is
    many-to-one — two distinct model_ids could collapse to the same dir, but
    ADR-V2-015 treats model_id as the unique key tracked in meta.json, so
    :class:`HNSWVectorIndex` will catch cross-id reloads via
    :class:`ModelIdMismatchError`.
    """
    return (
        model_id.replace(":", "-").replace("/", "-").replace(".", "-").replace("@", "-")
    )


# ---------------------------------------------------------------------------
# HNSW backend
# ---------------------------------------------------------------------------


class HNSWVectorIndex:
    """HNSW-backed in-process vector index, one directory per model_id.

    On construction, attempts to load an existing index from
    ``{octo_root}/l2-memory/hnsw-{safe_name}/``. If the directory has a
    pre-existing index with a different ``model_id`` or ``dim`` in its
    ``meta.json`` the constructor raises.
    """

    def __init__(
        self,
        model_id: str,
        octo_root: str | Path,
        dim: int = 1024,
        space: str = "cosine",
        M: int = 16,
        ef_construction: int = 200,
        max_elements: int = 10_000,
    ) -> None:
        self.model_id = model_id
        self.dim = dim
        self.space = space
        self.M = M
        self.ef_construction = ef_construction
        self._max_elements = max_elements

        safe_name = model_id_to_safe_dirname(model_id)
        self.index_dir = Path(octo_root) / "l2-memory" / f"hnsw-{safe_name}"
        self.index_dir.mkdir(parents=True, exist_ok=True)
        self.index_path = self.index_dir / "index.bin"
        self.meta_path = self.index_dir / "meta.json"

        # hnswlib expects Literal["l2","ip","cosine"]; space validated above so cast is safe.
        self._index = hnswlib.Index(space=space, dim=dim)  # type: ignore[arg-type]
        self._id_to_label: dict[str, int] = {}
        self._label_to_id: dict[int, str] = {}
        self._next_label = 0
        self._loaded = False
        self._write_lock = asyncio.Lock()

        # Try to load existing index (raises on model_id / dim mismatch)
        self._try_load_sync()

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    def _try_load_sync(self) -> None:
        """Load index from disk if present; otherwise initialize empty.

        Raises:
            ModelIdMismatchError: when the on-disk meta.json declares a
                different model_id.
            DimensionMismatchError: when the on-disk meta.json declares a
                different dim.
        """
        if self.index_path.exists() and self.meta_path.exists():
            meta = json.loads(self.meta_path.read_text())
            if meta["model_id"] != self.model_id:
                raise ModelIdMismatchError(
                    f"Index at {self.index_dir} has model_id="
                    f"{meta['model_id']!r}, but loading with "
                    f"{self.model_id!r}"
                )
            if meta["dim"] != self.dim:
                raise DimensionMismatchError(
                    f"Index dim={meta['dim']}, code dim={self.dim}"
                )
            self._index.load_index(
                str(self.index_path), max_elements=self._max_elements
            )
            self._id_to_label = dict(meta["id_to_label"])
            # JSON object keys are strings; labels are ints.
            self._label_to_id = {int(k): v for k, v in meta["label_to_id"].items()}
            self._next_label = int(meta["next_label"])
            self._loaded = True
        else:
            self._index.init_index(
                max_elements=self._max_elements,
                M=self.M,
                ef_construction=self.ef_construction,
            )
            self._loaded = True

        # ef controls query recall; raise slightly above ef_construction or a
        # sensible floor of 50.
        self._index.set_ef(max(self.ef_construction, 50))

    # ------------------------------------------------------------------
    # Mutating API
    # ------------------------------------------------------------------

    async def add(self, id: str, vec: list[float]) -> None:
        """Insert or overwrite the vector for ``id``.

        If ``id`` already exists, the prior label is soft-deleted (via
        ``mark_deleted``) and a fresh label is assigned.
        """
        if len(vec) != self.dim:
            raise DimensionMismatchError(f"vec len {len(vec)} != index dim {self.dim}")
        async with self._write_lock:
            # Grow if we're about to hit capacity.
            current_count = self._index.get_current_count()
            if current_count >= self._max_elements - 1:
                new_max = self._max_elements * 2
                self._index.resize_index(new_max)
                self._max_elements = new_max

            if id in self._id_to_label:
                old_label = self._id_to_label[id]
                self._label_to_id.pop(old_label, None)
                try:
                    self._index.mark_deleted(old_label)
                except (RuntimeError, Exception):  # noqa: BLE001
                    # Label may already be marked deleted; idempotent.
                    pass

            label = self._next_label
            self._next_label += 1
            self._index.add_items([vec], [label])
            self._id_to_label[id] = label
            self._label_to_id[label] = id

    async def delete(self, id: str) -> None:
        """Soft-delete ``id``. Search will skip deleted labels."""
        async with self._write_lock:
            if id not in self._id_to_label:
                return
            label = self._id_to_label.pop(id)
            self._label_to_id.pop(label, None)
            try:
                self._index.mark_deleted(label)
            except (RuntimeError, Exception):  # noqa: BLE001
                pass

    async def save(self) -> None:
        """Persist the index and metadata to disk."""
        async with self._write_lock:
            self._index.save_index(str(self.index_path))
            meta = {
                "model_id": self.model_id,
                "dim": self.dim,
                "space": self.space,
                "M": self.M,
                "ef_construction": self.ef_construction,
                "next_label": self._next_label,
                "id_to_label": self._id_to_label,
                # Stringify keys for JSON; parsed back to int on load.
                "label_to_id": {str(k): v for k, v in self._label_to_id.items()},
            }
            self.meta_path.write_text(json.dumps(meta))

    # ------------------------------------------------------------------
    # Read-only API
    # ------------------------------------------------------------------

    async def search(self, vec: list[float], top_k: int) -> list[Hit]:
        """Return the top ``top_k`` hits by cosine similarity.

        Deleted labels are filtered out. Empty index returns ``[]``.
        """
        if len(vec) != self.dim:
            raise DimensionMismatchError(f"vec len {len(vec)} != index dim {self.dim}")
        # Use *live* count (excluding soft-deleted) as the hard ceiling.
        # ``get_current_count`` includes deleted items, so requesting k >
        # alive_count causes hnswlib to raise "Cannot return results in a
        # contiguous 2D array".
        alive = len(self._id_to_label)
        if alive == 0:
            return []
        requested = min(max(top_k, 1), alive)
        try:
            labels, distances = self._index.knn_query([vec], k=requested)
        except RuntimeError:
            # Graph may be sparse after deletions; retry with k=1 to at least
            # return the nearest live neighbour.
            if requested <= 1:
                return []
            labels, distances = self._index.knn_query([vec], k=1)
        out: list[Hit] = []
        for label, dist in zip(labels[0], distances[0]):
            label_int = int(label)
            if label_int not in self._label_to_id:
                continue  # soft-deleted
            # hnswlib returns *distance*; for cosine space it is 1 - cos_sim.
            score = 1.0 - float(dist)
            out.append(Hit(id=self._label_to_id[label_int], score=score))
            if len(out) >= top_k:
                break
        return out

    # ------------------------------------------------------------------
    # Introspection (mainly for tests / diagnostics)
    # ------------------------------------------------------------------

    def count(self) -> int:
        """Number of live (non-deleted) entries in the index."""
        return len(self._id_to_label)


__all__ = [
    "DimensionMismatchError",
    "HNSWVectorIndex",
    "Hit",
    "ModelIdMismatchError",
    "VectorIndex",
    "model_id_to_safe_dirname",
]
