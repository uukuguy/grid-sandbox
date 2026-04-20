#!/usr/bin/env python3
"""ADR lint — F1-F5 checks.

- F1 Frontmatter compliance (via AdrMeta.validate()) — FAIL blocks CI
- F2 Trace existence: contract ADRs' enforcement.trace paths must exist — FAIL blocks CI
- F3 Trace in CI: contract ADRs' trace paths must appear in a workflow YAML — WARN only
- F4 Conflict detection (delegated to adr_conflict_detect.detect()) — WARN only
        (detector is advisory: module overlap ≠ real conflict; needs human review)
- F5 Staleness: affected_modules paths that no longer exist — WARN only

CLI:
    adr_lint.py [--id ADR-V2-021 | --all] [--check F1,F2,F3,F4,F5] [--ci]
                [--adr-root <path>]

Exit codes:
    --ci mode:   1 on any F1 or F2 failure, 0 otherwise (F3/F4/F5 are WARN only)
    human mode:  0 always (informational)
"""
from __future__ import annotations

import argparse
import os
import sys
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from adr_config import AdrConfig, load_config  # noqa: E402
from adr_conflict_detect import detect as detect_conflicts  # noqa: E402
from adr_frontmatter import AdrMeta, parse_file, parse_legacy  # noqa: E402


# ── ANSI ───────────────────────────────────────────────────────────────────

_TTY = sys.stdout.isatty()


def _c(text: str, color: str, ci: bool) -> str:
    if ci or not _TTY:
        return text
    codes = {"green": "32", "yellow": "33", "red": "31", "cyan": "36", "dim": "2"}
    code = codes.get(color, "0")
    return f"\x1b[{code}m{text}\x1b[0m"


# ── Result model ───────────────────────────────────────────────────────────


@dataclass
class LintFinding:
    check: str  # F1..F5
    level: str  # PASS | FAIL | WARN
    adr_id: str
    message: str

    @property
    def is_failure(self) -> bool:
        return self.level == "FAIL"


# ── ADR discovery ──────────────────────────────────────────────────────────


def _load(path: Path) -> tuple[AdrMeta, bool]:
    """Return (meta, used_legacy)."""
    meta, _ = parse_file(path)
    if meta is not None:
        return meta, False
    text = path.read_text(encoding="utf-8")
    return parse_legacy(text), True


def _discover(adr_root: Path) -> list[Path]:
    if not adr_root.exists():
        return []
    return sorted(
        p for p in adr_root.glob("ADR*.md")
        if p.is_file() and p.name != "ADR-TEMPLATE.md"
    )


def _find_by_id(adr_root: Path, adr_id: str) -> Path | None:
    for p in _discover(adr_root):
        if p.stem.startswith(adr_id) or adr_id in p.stem:
            return p
    return None


# ── F1: Frontmatter compliance ─────────────────────────────────────────────


def check_f1(meta: AdrMeta, used_legacy: bool, adr_id: str) -> list[LintFinding]:
    if used_legacy:
        # No frontmatter at all → single FAIL finding
        return [
            LintFinding(
                "F1",
                "FAIL",
                adr_id,
                "no YAML frontmatter — run /adr:new or backfill manually",
            )
        ]
    errs = meta.validate()
    if not errs:
        return [LintFinding("F1", "PASS", adr_id, "frontmatter compliant")]
    return [LintFinding("F1", "FAIL", adr_id, e) for e in errs]


# ── F2: Trace existence (contract only) ────────────────────────────────────


def check_f2(meta: AdrMeta, cfg: AdrConfig, adr_id: str) -> list[LintFinding]:
    if meta.type != "contract":
        return [LintFinding("F2", "PASS", adr_id, "n/a (non-contract)")]
    if not meta.enforcement.trace:
        return [
            LintFinding(
                "F2", "FAIL", adr_id, "contract type with empty enforcement.trace"
            )
        ]
    findings = []
    for tr in meta.enforcement.trace:
        tr_path = cfg.resolve(str(tr))
        if tr_path.exists():
            findings.append(LintFinding("F2", "PASS", adr_id, f"trace exists: {tr}"))
        else:
            findings.append(LintFinding("F2", "FAIL", adr_id, f"trace missing: {tr}"))
    return findings


# ── F3: Trace in CI (contract only) ────────────────────────────────────────


def _workflow_texts(cfg: AdrConfig) -> dict[str, str]:
    """Return {path_str: content} for every workflow YAML referenced in cfg."""
    texts: dict[str, str] = {}
    for wf in cfg.ci_workflows:
        wf_path = cfg.resolve(str(wf))
        if not wf_path.exists():
            continue
        if wf_path.is_dir():
            for p in sorted(wf_path.glob("*.yml")) + sorted(wf_path.glob("*.yaml")):
                try:
                    texts[str(p)] = p.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    continue
        elif wf_path.is_file():
            try:
                texts[str(wf_path)] = wf_path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
    return texts


def check_f3(
    meta: AdrMeta, cfg: AdrConfig, adr_id: str, workflow_texts: dict[str, str]
) -> list[LintFinding]:
    if meta.type != "contract":
        return [LintFinding("F3", "PASS", adr_id, "n/a (non-contract)")]
    if not meta.enforcement.trace:
        return [LintFinding("F3", "FAIL", adr_id, "no trace to check against CI")]

    findings = []
    for tr in meta.enforcement.trace:
        tr_str = str(tr)
        # Substring match on filename or full relative path
        needle_full = tr_str
        needle_base = Path(tr_str).name
        hit_file = None
        for wf_path, content in workflow_texts.items():
            if needle_full in content or needle_base in content:
                hit_file = wf_path
                break
        if hit_file:
            findings.append(
                LintFinding(
                    "F3",
                    "PASS",
                    adr_id,
                    f"trace referenced in CI: {needle_base} in {Path(hit_file).name}",
                )
            )
        else:
            findings.append(
                LintFinding(
                    "F3",
                    "WARN",
                    adr_id,
                    f"trace not found in any CI workflow: {tr_str}",
                )
            )
    return findings


# ── F4: Conflict detection (delegated, project-wide) ───────────────────────


def check_f4(cfg: AdrConfig) -> list[LintFinding]:
    """F4 is advisory: detector surfaces Accepted-pair module overlaps for
    human review (see adr_conflict_detect.py module docstring). Overlap ≠
    contradiction — two ADRs can legitimately touch the same crate from
    different concern layers (e.g. contract freeze vs ecosystem strategy).
    Emit WARN, not FAIL — never block CI."""
    conflicts = detect_conflicts(cfg)
    if not conflicts:
        return [LintFinding("F4", "PASS", "*", "no Accepted-pair conflicts detected")]
    findings = []
    for c in conflicts:
        findings.append(
            LintFinding(
                "F4",
                "WARN",
                f"{c.adr_a_id}↔{c.adr_b_id}",
                f"module overlap (advisory, not a blocker): {', '.join(c.shared_modules)}",
            )
        )
    return findings


# ── F5: Staleness (advisory only) ──────────────────────────────────────────


def check_f5(meta: AdrMeta, cfg: AdrConfig, adr_id: str) -> list[LintFinding]:
    if meta.status != "Accepted":
        return [LintFinding("F5", "PASS", adr_id, "n/a (not Accepted)")]
    if not meta.affected_modules:
        return [LintFinding("F5", "PASS", adr_id, "no affected_modules declared")]

    findings = []
    for mod in meta.affected_modules:
        mod_str = str(mod)
        p = cfg.resolve(mod_str)
        if p.exists():
            findings.append(
                LintFinding("F5", "PASS", adr_id, f"module present: {mod_str}")
            )
        else:
            findings.append(
                LintFinding(
                    "F5",
                    "WARN",
                    adr_id,
                    f"affected_module missing on disk: {mod_str}",
                )
            )
    # 180-day no-reference check is deferred — requires git log; skip silently
    # if git unavailable (we don't even attempt unless the user opts in later).
    return findings


# ── Runner ─────────────────────────────────────────────────────────────────


def run_checks(
    adrs: list[Path],
    cfg: AdrConfig,
    checks_enabled: set[str],
) -> list[LintFinding]:
    findings: list[LintFinding] = []
    workflow_texts = _workflow_texts(cfg) if "F3" in checks_enabled else {}

    # F4 is project-wide, run once
    f4_done = False

    for p in adrs:
        meta, used_legacy = _load(p)
        adr_id = meta.id or p.stem

        if "F1" in checks_enabled:
            findings.extend(check_f1(meta, used_legacy, adr_id))
        if "F2" in checks_enabled:
            findings.extend(check_f2(meta, cfg, adr_id))
        if "F3" in checks_enabled:
            findings.extend(check_f3(meta, cfg, adr_id, workflow_texts))
        if "F5" in checks_enabled:
            findings.extend(check_f5(meta, cfg, adr_id))

        if "F4" in checks_enabled and not f4_done:
            findings.extend(check_f4(cfg))
            f4_done = True

    # Edge case: F4 only run when at least one ADR processed; run standalone
    # if needed
    if "F4" in checks_enabled and not f4_done:
        findings.extend(check_f4(cfg))

    return findings


# ── CLI output ─────────────────────────────────────────────────────────────


def _fmt_finding(f: LintFinding, ci: bool) -> str:
    color = {"PASS": "green", "FAIL": "red", "WARN": "yellow"}.get(f.level, "dim")
    tag = _c(f"[{f.check} {f.level}]", color, ci)
    return f"{tag} {f.adr_id} {f.message}"


def _build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        description="ADR F1-F5 lint checker",
    )
    sel = ap.add_mutually_exclusive_group()
    sel.add_argument("--id", dest="adr_id", help="Lint one ADR by ID")
    sel.add_argument("--all", action="store_true", help="Lint every ADR (default)")
    ap.add_argument(
        "--check",
        default="F1,F2,F3,F4,F5",
        help="Comma-separated checks to run (default: F1,F2,F3,F4,F5)",
    )
    ap.add_argument(
        "--ci",
        action="store_true",
        help="CI mode — no ANSI, exit 1 on any F1 or F2 FAIL (F3/F4/F5 are WARN only)",
    )
    ap.add_argument("--adr-root", help="Override ADR root directory")
    return ap


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)

    cfg = load_config()
    if args.adr_root:
        cfg.adr_root = Path(args.adr_root).resolve()

    checks_enabled = {c.strip().upper() for c in args.check.split(",") if c.strip()}
    unknown = checks_enabled - {"F1", "F2", "F3", "F4", "F5"}
    if unknown:
        print(f"[error] unknown checks: {sorted(unknown)}", file=sys.stderr)
        return 2

    if args.adr_id:
        p = _find_by_id(cfg.adr_root, args.adr_id)
        if not p:
            print(f"[error] ADR not found: {args.adr_id}", file=sys.stderr)
            return 2
        adrs = [p]
    else:
        adrs = _discover(cfg.adr_root)
        if not adrs:
            print(f"[warn] no ADRs under {cfg.adr_root}", file=sys.stderr)
            return 0

    # Disable color if explicitly non-tty
    use_ci_fmt = args.ci or bool(os.environ.get("NO_COLOR"))

    findings = run_checks(adrs, cfg, checks_enabled)

    # Sort: group by ADR then by check
    findings.sort(key=lambda f: (f.adr_id, f.check, f.level))

    for f in findings:
        print(_fmt_finding(f, use_ci_fmt))

    # Summary
    counts = {"PASS": 0, "FAIL": 0, "WARN": 0}
    for f in findings:
        counts[f.level] = counts.get(f.level, 0) + 1
    summary = (
        f"\nSummary: {counts['PASS']} PASS | "
        f"{counts['FAIL']} FAIL | {counts['WARN']} WARN"
    )
    print(_c(summary, "cyan", use_ci_fmt))

    if args.ci:
        # Only F1/F2 FAIL is a hard blocker. F3 (trace in CI) and F4
        # (module overlap) are advisory — they surface candidates for
        # human review, not provable contradictions. Emitting them as
        # CI-blocking errors causes legitimate, ship-ready ADRs to
        # halt merges because two Accepted ADRs happen to touch the
        # same crate from different concern layers.
        hard_fail = any(
            f.is_failure and f.check in {"F1", "F2"} for f in findings
        )
        return 1 if hard_fail else 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
