#!/usr/bin/env python3
"""F4 — Conflict detection between Accepted ADRs claiming the same module.

Two Accepted ADRs whose `affected_modules` overlap AND neither supersedes the
other are reported as potential conflicts. We do NOT attempt to prove the
Decisions are actually contradictory — that would require NLP. Instead we
surface the overlap for human review. The `review_checklist` / supersede chain
typically resolves false positives.

CLI:
    adr_conflict_detect.py [--adr-root <path>] [--json]

Exported API:
    detect(config) -> list[ConflictReport]
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from adr_config import AdrConfig, load_config  # noqa: E402
from adr_frontmatter import AdrMeta, parse_file, parse_legacy  # noqa: E402


# ── Types ──────────────────────────────────────────────────────────────────


@dataclass
class ConflictReport:
    adr_a_id: str
    adr_b_id: str
    shared_modules: list[str] = field(default_factory=list)
    reason: str = ""


# ── Helpers ────────────────────────────────────────────────────────────────


def _load_one(path: Path) -> AdrMeta | None:
    """Best-effort load: frontmatter first, legacy fallback."""
    meta, _ = parse_file(path)
    if meta is not None:
        return meta
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return None
    return parse_legacy(text)


def _collect_accepted(adr_root: Path) -> list[AdrMeta]:
    metas: list[AdrMeta] = []
    if not adr_root.exists():
        return metas
    for p in sorted(adr_root.glob("ADR*.md")):
        if p.name == "ADR-TEMPLATE.md":
            continue
        meta = _load_one(p)
        if meta is None:
            continue
        # Fallback id from filename when frontmatter missing
        if not meta.id:
            meta.id = p.stem.split(" ")[0]
        if meta.status == "Accepted":
            metas.append(meta)
    return metas


def _normalize_module(m: str) -> str:
    """Collapse trailing slashes and whitespace for comparison."""
    return m.strip().rstrip("/")


def _in_supersede_chain(a: AdrMeta, b: AdrMeta) -> bool:
    """True if either ADR supersedes the other (directly)."""
    if b.id and b.id in (a.supersedes or []):
        return True
    if a.id and a.id in (b.supersedes or []):
        return True
    if a.superseded_by == b.id or b.superseded_by == a.id:
        return True
    return False


# ── Core ───────────────────────────────────────────────────────────────────


def detect(config: AdrConfig) -> list[ConflictReport]:
    """Find Accepted pairs sharing a module and not in supersede relation."""
    metas = _collect_accepted(config.adr_root)

    # Build module → list[meta]
    module_to_adrs: dict[str, list[AdrMeta]] = {}
    for m in metas:
        for raw_mod in m.affected_modules or []:
            norm = _normalize_module(str(raw_mod))
            if not norm:
                continue
            module_to_adrs.setdefault(norm, []).append(m)

    reports: list[ConflictReport] = []
    seen_pairs: set[tuple[str, str]] = set()

    for module, owners in module_to_adrs.items():
        if len(owners) < 2:
            continue
        # Pair-wise compare
        for i in range(len(owners)):
            for j in range(i + 1, len(owners)):
                a, b = owners[i], owners[j]
                if a.id == b.id:
                    continue
                if _in_supersede_chain(a, b):
                    continue
                key = tuple(sorted([a.id, b.id]))
                if key in seen_pairs:
                    # Accumulate shared modules onto existing report
                    for r in reports:
                        if (r.adr_a_id, r.adr_b_id) == key and module not in r.shared_modules:
                            r.shared_modules.append(module)
                    continue
                seen_pairs.add(key)
                reports.append(
                    ConflictReport(
                        adr_a_id=key[0],
                        adr_b_id=key[1],
                        shared_modules=[module],
                        reason=(
                            "both Accepted, neither supersedes the other, "
                            "and both claim same affected_module"
                        ),
                    )
                )
    return reports


# ── CLI ────────────────────────────────────────────────────────────────────


def _build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        description="F4 conflict detector across Accepted ADRs",
    )
    ap.add_argument("--adr-root", help="Override ADR root directory")
    ap.add_argument("--json", action="store_true", help="Emit JSON list of reports")
    return ap


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)

    cfg = load_config()
    if args.adr_root:
        cfg.adr_root = Path(args.adr_root).resolve()

    reports = detect(cfg)

    if args.json:
        print(json.dumps([asdict(r) for r in reports], ensure_ascii=False, indent=2))
        return 0

    if not reports:
        print(f"[F4 OK] no conflicts detected under {cfg.adr_root}")
        return 0

    for r in reports:
        modules = ", ".join(r.shared_modules)
        print(
            f"[F4] {r.adr_a_id} conflicts with {r.adr_b_id} on shared module {modules}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
