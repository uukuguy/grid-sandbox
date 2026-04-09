"""Tests for HermesRuntimeConfig."""

from hermes_runtime.config import HermesRuntimeConfig


def test_config_defaults():
    config = HermesRuntimeConfig()
    assert config.grpc_port == 50053
    assert config.runtime_id == "hermes-runtime"
    assert config.tier == "aligned"
    assert config.deployment_mode == "shared"
    assert config.hermes_max_iterations == 50


def test_config_from_env(monkeypatch):
    monkeypatch.setenv("HERMES_RUNTIME_PORT", "60053")
    monkeypatch.setenv("HERMES_MODEL", "openrouter/qwen-3-235b")
    monkeypatch.setenv("HOOK_BRIDGE_URL", "http://localhost:50054")
    config = HermesRuntimeConfig.from_env()
    assert config.grpc_port == 60053
    assert config.hermes_model == "openrouter/qwen-3-235b"
    assert config.hook_bridge_url == "http://localhost:50054"
