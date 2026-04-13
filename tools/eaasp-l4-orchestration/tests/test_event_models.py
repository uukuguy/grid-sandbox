"""Tests for Event and EventMetadata data models."""

from __future__ import annotations

import time

from eaasp_l4_orchestration.event_models import Event, EventMetadata


def test_event_auto_assigns_event_id():
    e = Event(session_id="s1", event_type="STOP", payload={"reason": "done"})
    assert e.event_id
    assert len(e.event_id) == 36  # UUID format


def test_event_auto_assigns_created_at():
    before = int(time.time())
    e = Event(session_id="s1", event_type="STOP")
    after = int(time.time())
    assert before <= e.created_at <= after


def test_event_preserves_explicit_values():
    e = Event(
        session_id="s1",
        event_type="PRE_TOOL_USE",
        event_id="custom-id",
        created_at=1000,
        cluster_id="c-001",
    )
    assert e.event_id == "custom-id"
    assert e.created_at == 1000
    assert e.cluster_id == "c-001"


def test_event_metadata_defaults():
    m = EventMetadata()
    assert m.source == ""
    assert m.trace_id == ""
    assert m.parent_event_id == ""
    assert m.extra == {}


def test_event_with_metadata():
    m = EventMetadata(source="interceptor:grid-runtime", trace_id="t-1")
    e = Event(session_id="s1", event_type="STOP", metadata=m)
    assert e.metadata.source == "interceptor:grid-runtime"
    assert e.metadata.trace_id == "t-1"


def test_metadata_to_dict_skips_empty():
    m = EventMetadata(source="test-src")
    d = m.to_dict()
    assert d == {"source": "test-src"}
    assert "trace_id" not in d


def test_metadata_to_dict_full():
    m = EventMetadata(
        trace_id="t1",
        span_id="s1",
        parent_event_id="p1",
        source="src",
        extra={"k": "v"},
    )
    d = m.to_dict()
    assert d == {
        "trace_id": "t1",
        "span_id": "s1",
        "parent_event_id": "p1",
        "source": "src",
        "extra": {"k": "v"},
    }
