"""Runtime pool — selects L1 runtime from configured pool.

MVP: in-memory pool loaded from runtimes.yaml.
Future: dynamic registration, health checks, load balancing.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from pathlib import Path

import yaml

logger = logging.getLogger(__name__)


@dataclass
class RuntimeEntry:
    """A single L1 runtime in the pool."""

    id: str
    name: str
    endpoint: str
    tier: str = "harness"  # harness | aligned | framework
    protocol: str = "grpc"
    healthy: bool = True
    tags: list[str] = field(default_factory=list)


class RuntimePool:
    """Manages available L1 runtimes for session assignment."""

    def __init__(self) -> None:
        self._runtimes: dict[str, RuntimeEntry] = {}

    def load_config(self, config_path: str | Path) -> int:
        """Load runtime pool from YAML config. Returns count loaded."""
        path = Path(config_path)
        if not path.exists():
            logger.warning("Runtime config not found: %s", path)
            return 0

        data = yaml.safe_load(path.read_text())
        runtimes = data.get("runtimes", [])
        for r in runtimes:
            entry = RuntimeEntry(
                id=r["id"],
                name=r.get("name", r["id"]),
                endpoint=r["endpoint"],
                tier=r.get("tier", "harness"),
                protocol=r.get("protocol", "grpc"),
                tags=r.get("tags", []),
            )
            self._runtimes[entry.id] = entry

        logger.info("Loaded %d runtimes", len(runtimes))
        return len(runtimes)

    def add(self, entry: RuntimeEntry) -> None:
        """Register a runtime entry."""
        self._runtimes[entry.id] = entry

    def select(self, preferred: str | None = None) -> RuntimeEntry | None:
        """Select a runtime by preference or first healthy one."""
        if preferred and preferred in self._runtimes:
            entry = self._runtimes[preferred]
            if entry.healthy:
                return entry

        # Fallback: first healthy runtime
        for entry in self._runtimes.values():
            if entry.healthy:
                return entry
        return None

    def list_all(self) -> list[RuntimeEntry]:
        return list(self._runtimes.values())

    @property
    def count(self) -> int:
        return len(self._runtimes)
