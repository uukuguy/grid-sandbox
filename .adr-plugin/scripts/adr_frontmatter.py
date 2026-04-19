#!/usr/bin/env python3
"""Parse and serialize YAML frontmatter in ADR markdown files.

Stdlib only. Minimal YAML subset — sufficient for ADR governance needs.

Usage:
    from adr_frontmatter import parse_file, dump_frontmatter, AdrMeta
    meta, body = parse_file("ADR-V2-021.md")
    # or parse_text(text) for string input
    # or parse_legacy(text) for old-style ADRs without YAML
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any


REQUIRED_FIELDS = {"id", "title", "type", "status", "date"}
VALID_TYPES = {"contract", "strategy", "record"}
VALID_STATUSES = {"Proposed", "Accepted", "Superseded", "Deprecated", "Archived"}
VALID_ENFORCEMENT_LEVELS = {"physical", "contract-test", "review-only", "strategic"}


@dataclass
class EnforcementMeta:
    level: str = "review-only"
    trace: list = field(default_factory=list)
    review_checklist: str | None = None


@dataclass
class AdrMeta:
    id: str = ""
    title: str = ""
    type: str = "strategy"
    status: str = "Proposed"
    date: str = ""
    phase: str = ""
    author: str = ""
    supersedes: list = field(default_factory=list)
    superseded_by: str | None = None
    deprecated_at: str | None = None
    deprecated_reason: str | None = None
    enforcement: EnforcementMeta = field(default_factory=EnforcementMeta)
    affected_modules: list = field(default_factory=list)
    related: list = field(default_factory=list)
    raw: dict = field(default_factory=dict)

    def validate(self) -> list[str]:
        """Return list of validation error messages."""
        errors = []
        if not self.id or not self.id.startswith("ADR-"):
            errors.append(f"id missing or malformed: {self.id!r}")
        if self.type not in VALID_TYPES:
            errors.append(f"type must be one of {VALID_TYPES}, got {self.type!r}")
        if self.status not in VALID_STATUSES:
            errors.append(f"status must be one of {VALID_STATUSES}, got {self.status!r}")
        if not re.match(r"^\d{4}-\d{2}-\d{2}$", self.date):
            errors.append(f"date must be YYYY-MM-DD, got {self.date!r}")
        if self.enforcement.level not in VALID_ENFORCEMENT_LEVELS:
            errors.append(
                f"enforcement.level must be one of {VALID_ENFORCEMENT_LEVELS}, "
                f"got {self.enforcement.level!r}"
            )
        # Contract type must have trace
        if self.type == "contract" and not self.enforcement.trace:
            errors.append("contract type requires non-empty enforcement.trace")
        # Review-only must have checklist
        if (
            self.enforcement.level == "review-only"
            and not self.enforcement.review_checklist
            and self.type != "record"
        ):
            errors.append("review-only enforcement requires review_checklist path")
        # Superseded consistency
        if self.status == "Superseded" and not self.superseded_by:
            errors.append("status=Superseded requires superseded_by")
        if self.status == "Deprecated" and not self.deprecated_at:
            errors.append("status=Deprecated requires deprecated_at")
        return errors


def parse_text(text: str) -> tuple[AdrMeta | None, str]:
    """Parse ADR markdown text. Returns (meta, body).

    If no YAML frontmatter detected, returns (None, original_text).
    Caller can then use parse_legacy() to extract metadata from old-style **Status:** lines.
    """
    if not text.startswith("---\n") and not text.startswith("---\r\n"):
        return None, text

    # Find end of frontmatter
    m = re.search(r"^---\s*$", text[4:], re.MULTILINE)
    if not m:
        return None, text

    end_pos = 4 + m.start()
    fm_text = text[4:end_pos]
    # Skip the closing --- and the trailing newline
    body_start = end_pos + m.end() - m.start()
    body = text[body_start:].lstrip("\n")

    raw = _parse_yaml(fm_text)
    meta = _build_meta(raw)
    return meta, body


def parse_file(path: str | Path) -> tuple[AdrMeta | None, str]:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    return parse_text(text)


def parse_legacy(text: str) -> AdrMeta:
    """Extract metadata from old-style ADR that uses **Status:** **Date:** header lines.

    Only used during migration — not a long-term path.
    """
    meta = AdrMeta()
    # Title from first H1
    m = re.search(r"^#\s+(ADR-[A-Z0-9\-]+)\s*[—\-–:]?\s*(.*)$", text, re.MULTILINE)
    if m:
        meta.id = m.group(1).strip()
        meta.title = m.group(2).strip() if m.group(2) else ""

    patterns = {
        "status": r"^\*\*Status:\*\*\s*(.+?)$",
        "date": r"^\*\*Date:\*\*\s*(\d{4}-\d{2}-\d{2})",
        "phase": r"^\*\*Phase:\*\*\s*(.+?)$",
        "author": r"^\*\*Author:\*\*\s*(.+?)$",
    }
    for key, pat in patterns.items():
        m = re.search(pat, text, re.MULTILINE)
        if m:
            val = m.group(1).strip()
            # Status may have trailing commentary like "Accepted / Deferred"
            if key == "status":
                # Extract first valid status keyword
                for s in VALID_STATUSES:
                    if s in val:
                        val = s
                        break
                else:
                    # Try to extract from bold wrapping `**Accepted**`
                    bm = re.search(r"\*\*(\w+)\*\*", val)
                    if bm and bm.group(1) in VALID_STATUSES:
                        val = bm.group(1)
            setattr(meta, key, val)

    # related: find "ADR-V2-NNN" refs in **Related:** line
    m = re.search(r"^\*\*Related:\*\*\s*(.+?)$", text, re.MULTILINE)
    if m:
        refs = re.findall(r"ADR-[A-Z0-9\-]+", m.group(1))
        meta.related = list(set(refs))

    # supersedes: from **Supersedes...** line
    m = re.search(r"^\*\*Supersede[^:]*:\*\*\s*(.+?)$", text, re.MULTILINE | re.IGNORECASE)
    if m:
        refs = re.findall(r"ADR-[A-Z0-9\-]+", m.group(1))
        meta.supersedes = list(set(refs))

    return meta


def dump_frontmatter(meta: AdrMeta) -> str:
    """Serialize AdrMeta to YAML frontmatter text (with --- markers)."""
    lines = ["---"]
    lines.append(f"id: {meta.id}")
    lines.append(f'title: "{_esc(meta.title)}"')
    lines.append(f"type: {meta.type}")
    lines.append(f"status: {meta.status}")
    lines.append(f"date: {meta.date}")
    if meta.phase:
        lines.append(f'phase: "{_esc(meta.phase)}"')
    if meta.author:
        lines.append(f'author: "{_esc(meta.author)}"')
    lines.append(f"supersedes: {_dump_list(meta.supersedes)}")
    lines.append(f"superseded_by: {_dump_val(meta.superseded_by)}")
    lines.append(f"deprecated_at: {_dump_val(meta.deprecated_at)}")
    lines.append(f"deprecated_reason: {_dump_val(meta.deprecated_reason)}")
    lines.append("enforcement:")
    lines.append(f"  level: {meta.enforcement.level}")
    if meta.enforcement.trace:
        lines.append("  trace:")
        for t in meta.enforcement.trace:
            lines.append(f'    - "{_esc(t)}"')
    else:
        lines.append("  trace: []")
    lines.append(f"  review_checklist: {_dump_val(meta.enforcement.review_checklist)}")
    if meta.affected_modules:
        lines.append("affected_modules:")
        for m in meta.affected_modules:
            lines.append(f'  - "{_esc(m)}"')
    else:
        lines.append("affected_modules: []")
    lines.append(f"related: {_dump_list(meta.related)}")
    lines.append("---")
    return "\n".join(lines) + "\n"


def write_file(path: str | Path, meta: AdrMeta, body: str) -> None:
    p = Path(path)
    text = dump_frontmatter(meta) + "\n" + body.lstrip("\n")
    p.write_text(text, encoding="utf-8")


# ── Internal helpers ──────────────────────────────────────────────────────


def _parse_yaml(text: str) -> dict:
    """Minimal YAML parser for ADR frontmatter schema.

    Supports top-level keys, nested 'enforcement:' block (with 'trace:' list),
    inline lists, and nested dicts at one level.
    """
    result: dict = {}
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            i += 1
            continue

        indent = len(line) - len(line.lstrip())
        if indent == 0:
            if ":" not in stripped:
                i += 1
                continue
            key, _, val = stripped.partition(":")
            key = key.strip()
            val = val.strip()
            if val:
                result[key] = _coerce(val)
                i += 1
            else:
                # Nested — collect lines until dedent
                nested: dict | list = {}
                j = i + 1
                sub_list = None
                while j < len(lines):
                    nxt = lines[j]
                    nxt_s = nxt.strip()
                    if not nxt_s or nxt_s.startswith("#"):
                        j += 1
                        continue
                    nxt_indent = len(nxt) - len(nxt.lstrip())
                    if nxt_indent == 0:
                        break
                    if nxt_s.startswith("- "):
                        if sub_list is None:
                            sub_list = []
                            nested = sub_list
                        sub_list.append(_coerce(nxt_s[2:].strip()))
                    elif ":" in nxt_s and isinstance(nested, dict):
                        sk, _, sv = nxt_s.partition(":")
                        sk = sk.strip()
                        sv = sv.strip()
                        if sv:
                            nested[sk] = _coerce(sv)
                        else:
                            # Second-level nesting (e.g., enforcement.trace list)
                            sub_list_inner = []
                            k = j + 1
                            while k < len(lines):
                                inner = lines[k]
                                inner_s = inner.strip()
                                inner_indent = len(inner) - len(inner.lstrip())
                                if inner_indent <= nxt_indent or not inner_s:
                                    break
                                if inner_s.startswith("- "):
                                    sub_list_inner.append(_coerce(inner_s[2:].strip()))
                                k += 1
                            if sub_list_inner:
                                nested[sk] = sub_list_inner
                            else:
                                nested[sk] = None
                            j = k - 1
                    j += 1
                result[key] = nested
                i = j
        else:
            i += 1
    return result


def _coerce(val: str) -> Any:
    if val == "null" or val == "~" or val == "":
        return None
    if val == "true":
        return True
    if val == "false":
        return False
    if val.startswith('"') and val.endswith('"'):
        return val[1:-1]
    if val.startswith("'") and val.endswith("'"):
        return val[1:-1]
    if val.startswith("[") and val.endswith("]"):
        inner = val[1:-1].strip()
        if not inner:
            return []
        return [_coerce(x.strip()) for x in _split_list(inner)]
    try:
        return int(val)
    except ValueError:
        pass
    try:
        return float(val)
    except ValueError:
        pass
    return val


def _split_list(s: str) -> list[str]:
    """Split inline list respecting quoted strings."""
    parts = []
    current = ""
    quote = None
    for ch in s:
        if quote:
            current += ch
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
            current += ch
        elif ch == "," and not quote:
            parts.append(current.strip())
            current = ""
        else:
            current += ch
    if current.strip():
        parts.append(current.strip())
    return parts


def _build_meta(raw: dict) -> AdrMeta:
    meta = AdrMeta()
    meta.raw = raw
    for f in ("id", "title", "type", "status", "date", "phase", "author",
              "superseded_by", "deprecated_at", "deprecated_reason"):
        if f in raw and raw[f] is not None:
            setattr(meta, f, raw[f])
    for f in ("supersedes", "related", "affected_modules"):
        if f in raw and isinstance(raw[f], list):
            setattr(meta, f, raw[f])
    if "enforcement" in raw and isinstance(raw["enforcement"], dict):
        e = raw["enforcement"]
        em = EnforcementMeta(
            level=e.get("level", "review-only"),
            trace=e.get("trace", []) or [],
            review_checklist=e.get("review_checklist"),
        )
        meta.enforcement = em
    return meta


def _esc(s: str) -> str:
    if s is None:
        return ""
    return str(s).replace('\\', '\\\\').replace('"', '\\"')


def _dump_val(v: Any) -> str:
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    return f'"{_esc(str(v))}"'


def _dump_list(lst: list) -> str:
    if not lst:
        return "[]"
    items = ", ".join(_dump_val(x) for x in lst)
    return f"[{items}]"


# ── CLI ────────────────────────────────────────────────────────────────────


def _cli() -> int:
    import argparse
    import json

    ap = argparse.ArgumentParser(description="Parse ADR frontmatter")
    ap.add_argument("path", help="ADR markdown file")
    ap.add_argument("--legacy", action="store_true", help="Parse legacy **Status:** format")
    ap.add_argument("--json", action="store_true", help="Output as JSON")
    ap.add_argument("--validate", action="store_true", help="Print validation errors")
    args = ap.parse_args()

    if args.legacy:
        text = Path(args.path).read_text(encoding="utf-8")
        meta = parse_legacy(text)
        body = text
    else:
        meta, body = parse_file(args.path)
        if meta is None:
            print(f"[warn] No YAML frontmatter in {args.path}; try --legacy", file=__import__("sys").stderr)
            return 2

    if args.json:
        d = asdict(meta)
        d.pop("raw", None)
        print(json.dumps(d, ensure_ascii=False, indent=2))
    else:
        print(dump_frontmatter(meta))

    if args.validate:
        errs = meta.validate()
        if errs:
            print("\n[validation errors]", file=__import__("sys").stderr)
            for e in errs:
                print(f"  - {e}", file=__import__("sys").stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(_cli())
