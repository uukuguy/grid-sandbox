"""Contract v1 — chunk_type whitelist gate across 7 runtimes (ADR-V2-021).

Every ``SendResponse`` emitted by a conformant L1 runtime MUST carry a
``ChunkType`` enum value drawn from the ADR-V2-021 §4 whitelist, and the
UNSPECIFIED (=0) default is forbidden on the wire. A DONE chunk MUST
appear exactly once per turn as the terminal signal.

This suite is invoked per-runtime via ``pytest --runtime=<name>``. The
existing ``runtime_launcher`` / ``runtime_grpc_stub`` fixtures in
``tests/contract/conftest.py`` handle subprocess launch + skip-if-deps-
missing; no new fixtures needed.

Phase 3.5 S3.T1.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from claude_code_runtime._proto.eaasp.runtime.v2 import (
    common_pb2,
    runtime_pb2,
)

if TYPE_CHECKING:
    from claude_code_runtime._proto.eaasp.runtime.v2 import runtime_pb2_grpc

pytestmark = pytest.mark.contract_v1


# ---------------------------------------------------------------------------
# ADR-V2-021 §4 whitelist — mirrors the table in the ADR. UNSPECIFIED is
# deliberately NOT in the set; every other ChunkType enum member IS.
# Keeping it frozen locally (not imported from L4 / CLI) makes this test
# the contract SSOT — if an ADR amendment adds a new wire name, both the
# consumer whitelist and this set must update in lockstep.
# ---------------------------------------------------------------------------

ALLOWED_WIRE = frozenset(
    {
        "text_delta",
        "thinking",
        "tool_start",
        "tool_result",
        "done",
        "error",
        "workflow_continuation",
    }
)


def _wire_name(enum_int: int) -> str:
    """Convert a ChunkType enum int to its ADR wire-string form.

    ``common_pb2.ChunkType.Name(5)`` returns ``"CHUNK_TYPE_DONE"``; the
    ADR specifies lowercase without the ``CHUNK_TYPE_`` prefix
    (``"done"``). Raises ``ValueError`` if ``enum_int`` is not a known
    ChunkType member — that IS a contract violation worth surfacing.
    """
    raw = common_pb2.ChunkType.Name(enum_int)
    assert raw.startswith("CHUNK_TYPE_"), (
        f"unexpected enum name shape: {raw!r}"
    )
    return raw[len("CHUNK_TYPE_") :].lower()


# ---------------------------------------------------------------------------
# Guard tests — lock the ADR table and the proto invariant. These do NOT
# launch a runtime and run unconditionally (as long as --runtime supplies
# any value to satisfy the session fixture chain — see module note).
# ---------------------------------------------------------------------------


def test_whitelist_matches_adr() -> None:
    """Lock ADR-V2-021 §4 table against accidental edits.

    If someone adds / removes / renames a wire value here without updating
    ADR-V2-021, this test fails and forces the ADR amendment back in scope.
    The whitelist is ALL non-UNSPECIFIED ChunkType enum members.
    """
    enum_values = common_pb2.ChunkType.DESCRIPTOR.values_by_number
    expected = frozenset(
        _wire_name(i) for i in enum_values if i != 0
    )
    assert ALLOWED_WIRE == expected, (
        f"whitelist drifted from ChunkType enum. "
        f"whitelist={sorted(ALLOWED_WIRE)} enum_non_zero={sorted(expected)}"
    )


def test_unspecified_is_zero() -> None:
    """ChunkType.UNSPECIFIED MUST be the proto3 default (0).

    Proto3 implicitly uses 0 as the field default; if the UNSPECIFIED
    sentinel ever moves off 0 we lose the ability to detect a runtime
    that forgets to set ``chunk_type``.
    """
    assert common_pb2.CHUNK_TYPE_UNSPECIFIED == 0


# ---------------------------------------------------------------------------
# Live-runtime contract test — one turn, drain stream, assert every chunk.
# ---------------------------------------------------------------------------


def test_chunk_type_contract(
    runtime_launcher,  # noqa: ARG001 — required to keep subprocess alive
    runtime_grpc_stub: "runtime_pb2_grpc.RuntimeServiceStub",
    runtime_name: str,
) -> None:
    """ADR-V2-021: chunks carry ChunkType enum, wire strings in whitelist.

    Per-turn invariants:

    1. No chunk may be ``CHUNK_TYPE_UNSPECIFIED`` (proto default sentinel).
    2. The wire-string form of every chunk_type is in :data:`ALLOWED_WIRE`.
    3. At least one ``CHUNK_TYPE_DONE`` chunk MUST be emitted (terminal).

    THINKING / TOOL_START / TOOL_RESULT are all optional — a pure text
    reply to "hello" may legitimately emit only TEXT_DELTA + DONE, and
    that is conformant.
    """
    payload = common_pb2.SessionPayload(
        session_id="chunk-type-contract-1",
        user_id="u",
        runtime_id=f"{runtime_name}-contract-test",
    )
    init_resp = runtime_grpc_stub.Initialize(
        runtime_pb2.InitializeRequest(payload=payload)
    )
    sid = init_resp.session_id
    assert sid, f"Initialize MUST return non-empty session_id; got {sid!r}"

    msg = runtime_pb2.UserMessage(content="hello", message_type="text")
    stream = runtime_grpc_stub.Send(
        runtime_pb2.SendRequest(session_id=sid, message=msg)
    )

    observed: list[int] = []
    violations: list[str] = []
    try:
        for chunk in stream:
            observed.append(chunk.chunk_type)

            if chunk.chunk_type == common_pb2.CHUNK_TYPE_UNSPECIFIED:
                violations.append(
                    "CHUNK_TYPE_UNSPECIFIED emitted — proto default sentinel "
                    "is forbidden per ADR-V2-021 §1"
                )
                continue

            try:
                wire = _wire_name(chunk.chunk_type)
            except ValueError as exc:
                violations.append(
                    f"chunk_type={chunk.chunk_type!r} is not a known "
                    f"ChunkType enum member: {exc}"
                )
                continue

            if wire not in ALLOWED_WIRE:
                violations.append(
                    f"chunk_type wire={wire!r} (enum int={chunk.chunk_type}) "
                    f"not in ADR-V2-021 whitelist {sorted(ALLOWED_WIRE)}"
                )
    finally:
        # Clean teardown even on assertion failure — keeps the session
        # fixture reusable and prevents cross-test state bleed.
        try:
            runtime_grpc_stub.Terminate(common_pb2.Empty())
        except Exception:  # noqa: BLE001
            # Double-terminate / missing-session errors are out of
            # scope for this test (D139 owns that).
            pass

    # Observability: include the full wire-name trace on any failure so
    # the reader can diff against the ADR table without re-running.
    wire_trace = [
        common_pb2.ChunkType.Name(i) if i in common_pb2.ChunkType.values() else f"?({i})"
        for i in observed
    ]

    assert not violations, (
        f"runtime={runtime_name!r} emitted non-conformant chunks:\n  - "
        + "\n  - ".join(violations)
        + f"\nobserved wire trace: {wire_trace}"
    )
    assert observed, (
        f"runtime={runtime_name!r} emitted zero SendResponse chunks; "
        f"contract requires ≥1 DONE chunk per turn"
    )
    assert common_pb2.CHUNK_TYPE_DONE in observed, (
        f"runtime={runtime_name!r} did not emit CHUNK_TYPE_DONE; "
        f"observed wire trace: {wire_trace}"
    )
