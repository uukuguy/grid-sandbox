#!/usr/bin/env python3
"""Hybrid rule-based ADR classifier.

Assigns type ∈ {contract, strategy, record, unknown} to each ADR with a
confidence score ∈ [0.0, 1.0] and a short rationale string. Rules apply in
priority order — first strong match wins. Priority: record → contract →
strategy → unknown (fallback).

CLI:
    adr_classify.py [--id ADR-V2-017 | --all] [--adr-root <path>] [--json]

Default `--all`. Exit code is always 0 (informational tool).

Implementation notes
--------------------
We operate on the Markdown text, not just frontmatter, because legacy ADRs
lack YAML frontmatter — we still want to classify them. Decision section
extraction falls back to the whole text when no `## Decision` header exists.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

# Local siblings — scripts/ must be on sys.path (direct invocation does this)
HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from adr_config import load_config  # noqa: E402
from adr_frontmatter import (  # noqa: E402
    AdrMeta,
    parse_file,
    parse_legacy,
)


# ── Rule vocabulary ────────────────────────────────────────────────────────

RECORD_TITLE_MARKERS = ("task", "sign-off", "signoff", "collapse", "收尾", "打点")
RECORD_DECISION_PREFIXES = ("实施", "implementation", "implement")
RECORD_PHASE_PATTERN = re.compile(
    r"\b([SW]\d+(?:\.[TW]\d+[a-z]?)?)\b",  # S3.T5, W1.T2, S4 …
    re.IGNORECASE,
)

CONTRACT_KEYWORDS = {
    "MUST",
    "必须",
    "禁止",
    "Frozen",
    "enum",
    "schema",
    "契约",
    "contract freeze",
}
CONTRACT_MODULE_PATTERNS = (
    re.compile(r"\.proto\b"),
    re.compile(r"\bproto/"),
    re.compile(r"\bmigration", re.IGNORECASE),
    re.compile(r"\bschema\b", re.IGNORECASE),
    re.compile(r"\bdb/", re.IGNORECASE),
)

STRATEGY_KEYWORDS = {
    "策略",
    "strategy",
    "ecosystem",
    "三轨",
    "principle",
    "原则",
    "tier",
    "生态",
    "选型",
}


# ── Data model ─────────────────────────────────────────────────────────────


@dataclass
class ClassifyResult:
    adr_id: str
    path: str
    type: str  # contract | strategy | record | unknown
    confidence: float
    rationale: str
    title: str = ""
    status: str = ""
    used_fallback_parser: bool = False


# ── Core ──────────────────────────────────────────────────────────────────


def _load_meta(path: Path) -> tuple[AdrMeta, str, bool]:
    """Return (meta, body, used_fallback). For legacy ADRs, parse_legacy fills meta."""
    meta, body = parse_file(path)
    if meta is not None:
        return meta, body, False
    raw_text = path.read_text(encoding="utf-8")
    legacy_meta = parse_legacy(raw_text)
    return legacy_meta, raw_text, True


def _extract_decision_section(body: str) -> str:
    """Pull the '## Decision' body if present; else return the whole text.

    Matches both English '## Decision' and Chinese '## 决定' (as seen in the
    meta-ADR template). Stops at the next H2.
    """
    m = re.search(
        r"^##\s+(?:Decision|决定|Decision\s*/\s*决定)\b(.*?)(?=^##\s+|\Z)",
        body,
        re.MULTILINE | re.DOTALL,
    )
    if m:
        return m.group(1)
    return body


def _first_non_empty_line(text: str) -> str:
    for ln in text.splitlines():
        s = ln.strip()
        if s and not s.startswith("#") and not s.startswith("---"):
            return s
    return ""


def _module_mentions_contract(affected_modules: list[str], body: str) -> list[str]:
    """Return the subset of affected_modules that look like contract-level files."""
    hits = []
    for m in affected_modules or []:
        for pat in CONTRACT_MODULE_PATTERNS:
            if pat.search(m):
                hits.append(m)
                break
    # Also scan body text for inline proto mentions — legacy ADRs lack
    # affected_modules but may discuss proto/*.proto in prose.
    if not hits:
        for pat in CONTRACT_MODULE_PATTERNS:
            bm = pat.search(body)
            if bm:
                hits.append(bm.group(0))
                break
    return hits


def _matched_keywords(text: str, vocab: set[str]) -> list[str]:
    """Case-sensitive for ALL-CAPS English markers (MUST), case-insensitive otherwise."""
    hits = []
    for kw in vocab:
        if kw.isupper() and kw.isascii():
            if re.search(rf"\b{re.escape(kw)}\b", text):
                hits.append(kw)
        else:
            if re.search(re.escape(kw), text, re.IGNORECASE):
                hits.append(kw)
    return hits


def classify_one(meta: AdrMeta, body: str) -> tuple[str, float, str]:
    """Return (type, confidence, rationale)."""
    decision_text = _extract_decision_section(body)
    title = meta.title or ""
    phase = meta.phase or ""

    # ── Rule 1: RECORD ──────────────────────────────────────────────────
    # a) phase field names a specific task/wave (S3.T5, W1.T2a, …)
    # b) decision body's first sentence starts with 实施 / Implementation
    # c) title contains marker words (task, sign-off, collapse, 收尾 …)
    record_reasons = []
    if phase:
        pm = RECORD_PHASE_PATTERN.search(phase)
        # Guard: "Phase 3" / "Phase 2.5" alone is NOT record-worthy — need
        # an S-prefix or explicit task code.
        if pm and re.search(r"[STW]\d", pm.group(1), re.IGNORECASE):
            record_reasons.append(f"phase names task: {pm.group(1)}")
    first_line = _first_non_empty_line(decision_text).lower()
    for pref in RECORD_DECISION_PREFIXES:
        if first_line.startswith(pref):
            record_reasons.append(f"Decision starts with '{pref}'")
            break
    lowered_title = title.lower()
    for marker in RECORD_TITLE_MARKERS:
        if marker in lowered_title:
            record_reasons.append(f"title contains '{marker}'")
            break

    if record_reasons:
        # Strong signals if ≥2 reasons or phase-task match; otherwise medium
        conf = 0.85 if len(record_reasons) >= 2 else 0.7
        return "record", conf, "; ".join(record_reasons)

    # ── Rule 2: CONTRACT ────────────────────────────────────────────────
    # Keywords AND module evidence both required.
    contract_kws = _matched_keywords(decision_text, CONTRACT_KEYWORDS)
    module_hits = _module_mentions_contract(meta.affected_modules or [], body)
    if contract_kws and module_hits:
        # Confidence scales with number of matched keywords (cap 0.95)
        conf = min(0.6 + 0.1 * len(contract_kws), 0.95)
        rationale = (
            f"keywords: {', '.join(sorted(set(contract_kws))[:4])}; "
            f"modules: {', '.join(module_hits[:3])}"
        )
        return "contract", conf, rationale

    # ── Rule 3: STRATEGY ────────────────────────────────────────────────
    strat_kws = _matched_keywords(decision_text, STRATEGY_KEYWORDS)
    # Guard: strategy ADRs describe selection/ecosystem — no field-level
    # module coupling to proto/schema.
    if strat_kws and not module_hits:
        conf = min(0.55 + 0.1 * len(strat_kws), 0.9)
        rationale = f"keywords: {', '.join(sorted(set(strat_kws))[:4])}; no field-level specs"
        return "strategy", conf, rationale

    # Mixed: contract keywords without module evidence → weak strategy signal
    if contract_kws and not module_hits:
        return (
            "strategy",
            0.5,
            f"contract-like keywords {contract_kws[:3]} but no proto/schema modules",
        )

    # ── Fallback: UNKNOWN ───────────────────────────────────────────────
    return (
        "unknown",
        0.3,
        "no strong contract/strategy/record signals — manual review suggested",
    )


def classify_adr(path: Path) -> ClassifyResult:
    meta, body, used_fallback = _load_meta(path)
    adr_type, conf, rationale = classify_one(meta, body)
    # If frontmatter already declares a type, report classifier view vs declared
    # but do not override — classification is advisory.
    declared = meta.raw.get("type") if meta.raw else None
    if declared and declared != adr_type:
        rationale = f"declared={declared}; classifier says {adr_type} — {rationale}"
    return ClassifyResult(
        adr_id=meta.id or path.stem,
        path=str(path),
        type=adr_type,
        confidence=round(conf, 2),
        rationale=rationale,
        title=meta.title,
        status=meta.status,
        used_fallback_parser=used_fallback,
    )


def discover_adrs(adr_root: Path) -> list[Path]:
    if not adr_root.exists():
        return []
    return sorted(
        p
        for p in adr_root.glob("ADR*.md")
        if p.is_file() and p.name != "ADR-TEMPLATE.md"
    )


def find_adr_by_id(adr_root: Path, adr_id: str) -> Path | None:
    for p in discover_adrs(adr_root):
        if p.stem.startswith(adr_id) or adr_id in p.stem:
            return p
    return None


# ── CLI ────────────────────────────────────────────────────────────────────


def _build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        description="Hybrid ADR classifier (contract | strategy | record | unknown)",
    )
    sel = ap.add_mutually_exclusive_group()
    sel.add_argument("--id", dest="adr_id", help="Classify one ADR by ID (e.g. ADR-V2-017)")
    sel.add_argument("--all", action="store_true", help="Classify every ADR (default)")
    ap.add_argument("--adr-root", help="Override ADR root directory")
    ap.add_argument("--json", action="store_true", help="Emit JSON list of dicts")
    return ap


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)

    cfg = load_config()
    adr_root = Path(args.adr_root).resolve() if args.adr_root else cfg.adr_root

    if args.adr_id:
        p = find_adr_by_id(adr_root, args.adr_id)
        if not p:
            print(f"[error] ADR not found: {args.adr_id} in {adr_root}", file=sys.stderr)
            return 0  # informational — still exit 0
        results = [classify_adr(p)]
    else:
        results = [classify_adr(p) for p in discover_adrs(adr_root)]

    if args.json:
        print(json.dumps([asdict(r) for r in results], ensure_ascii=False, indent=2))
        return 0

    if not results:
        print(f"[info] no ADRs found under {adr_root}")
        return 0

    # Human-readable table
    id_w = max((len(r.adr_id) for r in results), default=12)
    type_w = max((len(r.type) for r in results), default=8)
    for r in results:
        print(
            f"{r.adr_id:<{id_w}} | {r.type:<{type_w}} | {r.confidence:.2f} | {r.rationale}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
