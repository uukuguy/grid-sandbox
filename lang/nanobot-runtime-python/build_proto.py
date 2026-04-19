"""Build gRPC Python stubs from EAASP v2 proto files for nanobot-runtime.

Mirrors ``lang/claude-code-runtime-python/build_proto.py`` but targets the
``nanobot_runtime._proto`` package namespace.

Usage:
    cd lang/nanobot-runtime-python
    uv run python build_proto.py
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

PROTO_ROOT = Path(os.getenv("PROTO_ROOT", Path(__file__).parent.parent.parent / "proto"))
OUT_DIR = Path(__file__).parent / "src" / "nanobot_runtime" / "_proto"

PROTO_FILES = [
    "eaasp/runtime/v2/common.proto",
    "eaasp/runtime/v2/runtime.proto",
    "eaasp/runtime/v2/hook.proto",
]

PKG_PREFIX = "nanobot_runtime._proto"


def build() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "__init__.py").touch()

    for proto in PROTO_FILES:
        proto_path = Path(proto)
        out_subdir = OUT_DIR / proto_path.parent
        out_subdir.mkdir(parents=True, exist_ok=True)

        parts = proto_path.parent.parts
        for i in range(len(parts)):
            init_path = OUT_DIR / Path(*parts[: i + 1]) / "__init__.py"
            init_path.touch()

        cmd = [
            sys.executable,
            "-m",
            "grpc_tools.protoc",
            f"--proto_path={PROTO_ROOT}",
            f"--python_out={OUT_DIR}",
            f"--grpc_python_out={OUT_DIR}",
            f"--pyi_out={OUT_DIR}",
            str(PROTO_ROOT / proto),
        ]
        print(f"Compiling {proto}...")
        subprocess.check_call(cmd)

    _fix_imports(OUT_DIR)
    print("Proto build complete.")


def _fix_imports(out_dir: Path) -> None:
    """Rewrite bare ``from eaasp.`` imports to use our package namespace."""
    for py_file in out_dir.rglob("*.py"):
        content = py_file.read_text()
        fixed = content.replace(
            "from eaasp.", f"from {PKG_PREFIX}.eaasp."
        )
        if fixed != content:
            py_file.write_text(fixed)
            print(f"  Fixed imports in {py_file.relative_to(out_dir)}")


if __name__ == "__main__":
    build()
