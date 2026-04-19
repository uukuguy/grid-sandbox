#!/usr/bin/env python3
"""Aggregated ADR health report.

Imports adr_classify / adr_lint / adr_conflict_detect if available and
produces either a human-readable dashboard or a structured JSON dict.

Never crashes when siblings are missing or raise — each subsystem is optional.

Usage:
    python adr_review.py              # human text
    python adr_review.py --json       # machine JSON
    python adr_review.py --health     # abbreviated health summary
    python adr_review.py --adr-root <path> --config <path>
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

# Make sibling scripts importable whether run from anywhere.
_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

from adr_config import AdrConfig, load_config  # noqa: E402
from adr_frontmatter import (  # noqa: E402
    VALID_ENFORCEMENT_LEVELS,
    VALID_STATUSES,
    VALID_TYPES,
    AdrMeta,
    parse_file,
    parse_legacy,
)


def _safe_import(name: str):
    try:
        return __import__(name)
    except Exception:
        return None


def _call_any(fn, *arg_variants):
    """Try calling fn with each argument tuple in order; return first success."""
    last_exc: Exception | None = None
    for args, kwargs in arg_variants:
        try:
            return fn(*args, **kwargs)
        except TypeError as e:
            last_exc = e
            continue
    raise last_exc if last_exc else TypeError("no argument variant matched")


def _iter_adrs(cfg: AdrConfig):
    """Yield (path, meta, has_yaml) for every ADR-*.md in adr_root."""
    if not cfg.adr_root.exists():
        return
    for p in sorted(cfg.adr_root.glob("ADR-*.md")):
        # Skip template
        if p.name.startswith("ADR-TEMPLATE"):
            continue
        try:
            text = p.read_text(encoding="utf-8")
        except OSError:
            continue
        meta, _ = parse_file(p)
        if meta is None:
            meta = parse_legacy(text)
            yield p, meta, False
        else:
            yield p, meta, True


def _count_by(items, key_fn):
    counts: dict[str, int] = {}
    for it in items:
        k = key_fn(it) or "unknown"
        counts[k] = counts.get(k, 0) + 1
    return counts


def _try_classify(cfg: AdrConfig) -> dict[str, Any] | None:
    """Invoke adr_classify API and roll the per-ADR results up into counts.

    Supports both a batch `classify_all` (preferred) and the per-ADR
    `classify_adr` + `discover_adrs` pair exposed by the current siblings.
    """
    mod = _safe_import("adr_classify")
    if mod is None:
        return None

    try:
        if hasattr(mod, "classify_all"):
            raw = _call_any(
                mod.classify_all,
                ((cfg,), {}),
                ((), {"cfg": cfg}),
                ((), {"config": cfg}),
                ((), {}),
            )
            if isinstance(raw, dict):
                return raw

        # Fallback: classify_adr(path) + discover_adrs(root)
        if hasattr(mod, "classify_adr") and hasattr(mod, "discover_adrs"):
            paths = mod.discover_adrs(cfg.adr_root)
            per: list[dict[str, Any]] = []
            by_type: dict[str, int] = {}
            for p in paths:
                res = mod.classify_adr(p)
                t = getattr(res, "type", None) or "unknown"
                by_type[t] = by_type.get(t, 0) + 1
                per.append(
                    {
                        "adr_id": getattr(res, "adr_id", str(p.stem)),
                        "type": t,
                        "confidence": getattr(res, "confidence", None),
                        "rationale": getattr(res, "rationale", ""),
                    }
                )
            return {"by_type": by_type, "results": per}
    except Exception as e:
        return {"error": f"classify failed: {e}"}
    return None


def _try_lint(cfg: AdrConfig) -> dict[str, Any] | None:
    mod = _safe_import("adr_lint")
    if mod is None:
        return None

    try:
        if hasattr(mod, "lint_all"):
            raw = _call_any(
                mod.lint_all,
                ((cfg,), {}),
                ((), {"cfg": cfg}),
                ((), {"config": cfg}),
                ((), {}),
            )
            if isinstance(raw, dict):
                return raw
            if isinstance(raw, list):
                return _rollup_findings(raw)

        # Fallback: run_checks(adrs, cfg, checks_enabled)
        if hasattr(mod, "run_checks"):
            # discover_adrs lives in adr_classify; replicate inline if missing
            classify_mod = _safe_import("adr_classify")
            if classify_mod and hasattr(classify_mod, "discover_adrs"):
                paths = classify_mod.discover_adrs(cfg.adr_root)
            else:
                paths = [
                    p
                    for p in sorted(cfg.adr_root.glob("ADR-*.md"))
                    if not p.name.startswith("ADR-TEMPLATE")
                ]
            findings = mod.run_checks(paths, cfg, {"F1", "F2", "F3", "F4", "F5"})
            return _rollup_findings(findings)
    except Exception as e:
        return {"error": f"lint failed: {e}"}
    return None


def _rollup_findings(findings: list) -> dict[str, Any]:
    """Count LintFinding-shaped items by check code, treating non-PASS as issues."""
    failures: dict[str, int] = {"F1": 0, "F2": 0, "F3": 0, "F4": 0, "F5": 0}
    total = 0
    for f in findings:
        check = getattr(f, "check", None) or (f.get("check") if isinstance(f, dict) else None)
        level = getattr(f, "level", None) or (f.get("level") if isinstance(f, dict) else None)
        if not check:
            continue
        total += 1
        if level and str(level).upper() == "PASS":
            continue
        if check in failures:
            failures[check] += 1
    return {"failures": failures, "total_findings": total}


def _try_conflict(cfg: AdrConfig) -> dict[str, Any] | None:
    mod = _safe_import("adr_conflict_detect")
    if mod is None or not hasattr(mod, "detect"):
        return None
    try:
        raw = _call_any(
            mod.detect,
            ((cfg,), {}),
            ((), {"config": cfg}),
            ((), {"cfg": cfg}),
            ((), {}),
        )
    except Exception as e:
        return {"error": f"detect failed: {e}"}

    if isinstance(raw, dict):
        return raw
    if isinstance(raw, list):
        return {"conflicts": raw, "count": len(raw)}
    return {"conflicts": [], "count": 0}


def _get(d: Any, key: str, default: Any = None) -> Any:
    if isinstance(d, dict):
        return d.get(key, default)
    return default


def build_report(cfg: AdrConfig) -> dict[str, Any]:
    """Collect the full report as a structured dict."""
    adrs = list(_iter_adrs(cfg))
    total = len(adrs)

    metas = [m for _, m, _ in adrs]
    legacy_count = sum(1 for _, _, has_yaml in adrs if not has_yaml)

    # Type bucket — unknown when metadata doesn't declare it
    type_counts = _count_by(metas, lambda m: m.type if m.type in VALID_TYPES else "unknown")

    # Status bucket
    status_counts = _count_by(metas, lambda m: m.status if m.status in VALID_STATUSES else "unknown")

    # Enforcement bucket
    enforcement_counts = _count_by(
        metas,
        lambda m: m.enforcement.level if m.enforcement.level in VALID_ENFORCEMENT_LEVELS else "unknown",
    )

    # For contract ADRs, count those with/without trace
    contract_with_trace = 0
    contract_missing_trace = 0
    for m in metas:
        if m.type == "contract":
            if m.enforcement.trace:
                contract_with_trace += 1
            else:
                contract_missing_trace += 1

    # Invoke sibling subsystems defensively
    classify_result = _try_classify(cfg)
    lint_result = _try_lint(cfg)
    conflict_result = _try_conflict(cfg)

    # Extract F1-F5 counts from lint if present
    f_counts = {"F1": None, "F2": None, "F3": None, "F4": None, "F5": None}
    if isinstance(lint_result, dict):
        # Common shapes: {"F1": [...], ...} or {"failures": {"F1": n, ...}}
        if "failures" in lint_result and isinstance(lint_result["failures"], dict):
            for k, v in lint_result["failures"].items():
                if k in f_counts:
                    f_counts[k] = v if isinstance(v, int) else len(v) if hasattr(v, "__len__") else None
        else:
            for k in f_counts:
                v = lint_result.get(k)
                if v is not None:
                    f_counts[k] = v if isinstance(v, int) else len(v) if hasattr(v, "__len__") else None

    # F1 fallback: count ADRs without YAML frontmatter (legacy count is a proxy)
    if f_counts["F1"] is None:
        f_counts["F1"] = legacy_count

    # F4 fallback: from conflict_detect output
    if f_counts["F4"] is None and isinstance(conflict_result, dict):
        cl = conflict_result.get("conflicts")
        if isinstance(cl, list):
            f_counts["F4"] = len(cl)
        elif isinstance(cl, int):
            f_counts["F4"] = cl

    # Classify type counts — prefer classifier output over declared frontmatter
    classify_types = None
    if isinstance(classify_result, dict):
        # Common shape: {"by_type": {"contract": n, ...}} or {"contract": n, ...}
        by_type = classify_result.get("by_type")
        if isinstance(by_type, dict):
            classify_types = by_type
        else:
            maybe = {
                t: classify_result[t]
                for t in VALID_TYPES
                if t in classify_result and isinstance(classify_result[t], int)
            }
            if maybe:
                classify_types = maybe

    # Build recommended actions
    actions: list[str] = []
    if legacy_count:
        actions.append(
            f"Backfill frontmatter on {legacy_count} ADR{'s' if legacy_count != 1 else ''} "
            "-> /adr:audit --backfill"
        )
    if classify_types and classify_types.get("record", 0) > 0:
        n = classify_types["record"]
        actions.append(f"Downgrade {n} record-type ADR{'s' if n != 1 else ''} -> /adr:downgrade")
    if contract_missing_trace:
        actions.append(
            f"Add CI trace for {contract_missing_trace} contract ADR"
            f"{'s' if contract_missing_trace != 1 else ''}"
        )
    f4 = f_counts.get("F4")
    if isinstance(f4, int) and f4 > 0:
        actions.append(f"Resolve {f4} ADR conflict{'s' if f4 != 1 else ''} -> /adr:reconcile")
    f5 = f_counts.get("F5")
    if isinstance(f5, int) and f5 > 0:
        actions.append(f"Review {f5} stale ADR{'s' if f5 != 1 else ''} (F5 warn)")

    return {
        "generated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "adr_root": str(cfg.adr_root),
        "total": total,
        "by_type_declared": type_counts,
        "by_type_classified": classify_types,
        "by_status": status_counts,
        "by_enforcement": enforcement_counts,
        "contract_trace": {
            "with_trace": contract_with_trace,
            "missing_trace": contract_missing_trace,
        },
        "failures": f_counts,
        "legacy_count": legacy_count,
        "actions": actions,
        "subsystems": {
            "classify": classify_result is not None and "error" not in (classify_result or {}),
            "lint": lint_result is not None and "error" not in (lint_result or {}),
            "conflict": conflict_result is not None and "error" not in (conflict_result or {}),
        },
        "subsystem_errors": {
            k: v.get("error")
            for k, v in {
                "classify": classify_result or {},
                "lint": lint_result or {},
                "conflict": conflict_result or {},
            }.items()
            if isinstance(v, dict) and "error" in v
        },
    }


def render_text(report: dict[str, Any], health_only: bool = False) -> str:
    out: list[str] = []
    date_short = report["generated_at"].split()[0]
    out.append(f"ADR Governance Health Report  -- {date_short}")
    out.append("=" * 60)
    out.append("")
    out.append(f"Total ADRs: {report['total']}")

    # Type section — prefer classifier, fall back to declared
    types_src = report.get("by_type_classified") or report.get("by_type_declared") or {}
    contract_n = types_src.get("contract", 0)
    strategy_n = types_src.get("strategy", 0)
    record_n = types_src.get("record", 0)
    unknown_n = types_src.get("unknown", 0)

    ct = report.get("contract_trace", {})
    with_trace = ct.get("with_trace", 0)
    missing_trace = ct.get("missing_trace", 0)
    trace_suffix = f"  ({with_trace} with CI trace, {missing_trace} missing)" if contract_n else ""

    out.append(f"  [C] contract:  {contract_n}{trace_suffix}")
    out.append(f"  [S] strategy:  {strategy_n}")
    record_suffix = "  (-> should be downgraded)" if record_n else ""
    out.append(f"  [R] record:    {record_n}{record_suffix}")
    if unknown_n:
        out.append(f"  [?] unknown:   {unknown_n}")
    out.append("")

    # Status
    status = report.get("by_status", {})
    out.append("Status:")
    out.append(
        f"  Proposed:   {status.get('Proposed', 0):<4} "
        f"Accepted: {status.get('Accepted', 0)}"
    )
    out.append(
        f"  Superseded: {status.get('Superseded', 0):<4} "
        f"Deprecated: {status.get('Deprecated', 0):<4} "
        f"Archived: {status.get('Archived', 0)}"
    )
    out.append("")

    # Enforcement
    enf = report.get("by_enforcement", {})
    out.append("Enforcement:")
    out.append(
        f"  physical:       {enf.get('physical', 0):<4} "
        f"contract-test: {enf.get('contract-test', 0)}"
    )
    out.append(
        f"  review-only:    {enf.get('review-only', 0):<4} "
        f"strategic:     {enf.get('strategic', 0)}"
    )
    out.append("")

    # Issues
    f = report.get("failures", {})
    out.append("Issues:")
    for key, label in [
        ("F1", "F1 failures"),
        ("F2", "F2 failures"),
        ("F3", "F3 warnings"),
        ("F4", "F4 conflicts"),
        ("F5", "F5 stale"),
    ]:
        val = f.get(key)
        shown = "n/a" if val is None else str(val)
        extra = ""
        if key == "F1" and val is not None and val > 0 and report.get("legacy_count", 0) == val:
            extra = "  (missing YAML frontmatter -- legacy)"
        out.append(f"  {label}:  {shown}{extra}")
    out.append("")

    # Subsystems
    subs = report.get("subsystems", {})
    missing = [k for k, v in subs.items() if not v]
    if missing:
        out.append(f"Subsystems unavailable: {', '.join(missing)}")
        for name, err in (report.get("subsystem_errors") or {}).items():
            out.append(f"  {name}: {err}")
        out.append("")

    # Actions
    actions = report.get("actions", [])
    if actions:
        out.append("Actions recommended:")
        for a in actions:
            out.append(f"  - {a}")
    else:
        out.append("Actions recommended: none")

    if health_only:
        # compact view — still print everything above since health is the summary,
        # but drop a blank line
        pass

    return "\n".join(out) + "\n"


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Aggregated ADR health report")
    ap.add_argument("--json", action="store_true", help="Emit structured JSON")
    ap.add_argument("--health", action="store_true", help="Health summary mode")
    ap.add_argument("--adr-root", help="Override ADR root directory")
    ap.add_argument("--config", help="Override project root for config loading")
    args = ap.parse_args(argv)

    cfg = load_config(project_root=args.config) if args.config else load_config()
    if args.adr_root:
        cfg.adr_root = Path(args.adr_root).resolve()

    report = build_report(cfg)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print(render_text(report, health_only=args.health), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
