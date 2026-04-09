"""Tests for RuntimeServiceImpl and TelemetryCollector."""

from hermes_runtime.config import HermesRuntimeConfig
from hermes_runtime.service import RuntimeServiceImpl
from hermes_runtime.telemetry import TelemetryCollector


def test_telemetry_collector():
    tc = TelemetryCollector(session_id="s1", runtime_id="hermes-runtime", user_id="u1")
    tc.record("session_start")
    tc.record("send", payload={"content_len": 42})
    entries = tc.peek()
    assert len(entries) == 2
    assert entries[0].event_type == "session_start"
    flushed = tc.flush()
    assert len(flushed) == 2
    assert len(tc.peek()) == 0


def test_service_init():
    """RuntimeServiceImpl 可正确构建。"""
    config = HermesRuntimeConfig()
    service = RuntimeServiceImpl(config)
    assert service.session_mgr.count == 0
    assert service.config.runtime_id == "hermes-runtime"
