#!/usr/bin/env python3
"""Download official SWE-bench Lite dataset from HuggingFace and convert to octo-eval JSONL format.

Usage:
    python3 scripts/download_swebench_lite.py

Downloads the parquet file directly from HuggingFace Hub and converts to JSONL.
Requires: pandas, pyarrow (will attempt pip install if missing).
"""

import json
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

PARQUET_URL = (
    "https://huggingface.co/datasets/princeton-nlp/SWE-bench_Lite"
    "/resolve/main/data/test-00000-of-00001.parquet"
)


def ensure_dependencies():
    """Ensure pandas and pyarrow are available."""
    try:
        import pandas  # noqa: F401
        import pyarrow  # noqa: F401
    except ImportError:
        print("Installing pandas and pyarrow...")
        subprocess.check_call(
            [sys.executable, "-m", "pip", "install", "-q", "pandas", "pyarrow"],
            stderr=subprocess.DEVNULL,
        )


def main():
    ensure_dependencies()
    import pandas as pd

    output_dir = Path("crates/octo-eval/datasets")
    output_file = output_dir / "swe_bench_lite.jsonl"
    backup_file = output_dir / "swe_bench_lite.synthetic.jsonl.bak"

    # Backup existing file if present
    if output_file.exists():
        print(f"Backing up existing dataset to {backup_file}")
        output_file.rename(backup_file)

    # Download parquet to a temp file
    with tempfile.NamedTemporaryFile(suffix=".parquet", delete=False) as tmp:
        tmp_path = tmp.name
    print(f"Downloading SWE-bench Lite parquet from HuggingFace...")
    urllib.request.urlretrieve(PARQUET_URL, tmp_path)

    df = pd.read_parquet(tmp_path)
    print(f"Loaded {len(df)} instances")
    print(f"Columns: {list(df.columns)}")

    # Convert to JSONL
    count = 0
    with open(output_file, "w") as f:
        for _, row in df.iterrows():
            record = {
                "instance_id": str(row.get("instance_id", "")),
                "repo": str(row.get("repo", "")),
                "base_commit": str(row.get("base_commit", "")),
                "patch": str(row.get("patch", "")),
                "test_patch": str(row.get("test_patch", "")),
                "problem_statement": str(row.get("problem_statement", "")),
                "hints_text": str(row.get("hints_text", "")),
                "fail_to_pass": str(row.get("FAIL_TO_PASS", "[]")),
                "pass_to_pass": str(row.get("PASS_TO_PASS", "[]")),
                "version": str(row.get("version", "")),
                "environment_setup_commit": str(row.get("environment_setup_commit", "")),
                "created_at": str(row.get("created_at", "")),
            }
            f.write(json.dumps(record) + "\n")
            count += 1

    print(f"Wrote {count} records to {output_file}")

    # Validate
    with open(output_file) as f:
        lines = [l.strip() for l in f if l.strip()]
    for i, line in enumerate(lines):
        try:
            obj = json.loads(line)
            assert "instance_id" in obj, f"Line {i}: missing instance_id"
            assert "problem_statement" in obj, f"Line {i}: missing problem_statement"
            assert "patch" in obj, f"Line {i}: missing patch"
        except Exception as e:
            print(f"Validation error on line {i}: {e}")
            sys.exit(1)

    print(f"Validation passed: {len(lines)} records OK")

    # Clean up temp file
    Path(tmp_path).unlink(missing_ok=True)

    # Print stats
    repos: dict[str, int] = {}
    for line in lines:
        obj = json.loads(line)
        repo = obj["repo"]
        repos[repo] = repos.get(repo, 0) + 1
    print(f"\nRepository distribution ({len(repos)} repos):")
    for repo, cnt in sorted(repos.items(), key=lambda x: -x[1]):
        print(f"  {repo}: {cnt}")


if __name__ == "__main__":
    main()
