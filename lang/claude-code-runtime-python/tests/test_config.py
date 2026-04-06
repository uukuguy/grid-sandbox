"""Tests for RuntimeConfig."""

import os

from claude_code_runtime.config import RuntimeConfig


def test_default_config():
    config = RuntimeConfig()
    assert config.grpc_port == 50052
    assert config.runtime_id == "claude-code-runtime"
    assert config.tier == "harness"
    assert config.permission_mode == "acceptEdits"


def test_from_env(monkeypatch):
    monkeypatch.setenv("CLAUDE_RUNTIME_PORT", "50099")
    monkeypatch.setenv("ANTHROPIC_MODEL_NAME", "claude-haiku-4-5-20251001")
    monkeypatch.setenv("ANTHROPIC_BASE_URL", "http://proxy:8080")
    monkeypatch.setenv("CLAUDE_MAX_TURNS", "5")

    config = RuntimeConfig.from_env()
    assert config.grpc_port == 50099
    assert config.anthropic_model_name == "claude-haiku-4-5-20251001"
    assert config.anthropic_base_url == "http://proxy:8080"
    assert config.max_turns == 5


def test_from_env_defaults(monkeypatch, tmp_path):
    # Use an empty env file to avoid loading root .env
    empty_env = tmp_path / ".env"
    empty_env.write_text("")

    for key in [
        "CLAUDE_RUNTIME_PORT",
        "CLAUDE_RUNTIME_ID",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_MODEL_NAME",
    ]:
        monkeypatch.delenv(key, raising=False)

    config = RuntimeConfig.from_env(env_file=empty_env)
    assert config.grpc_port == 50052
    assert config.anthropic_api_key == ""
    assert config.anthropic_base_url == ""
