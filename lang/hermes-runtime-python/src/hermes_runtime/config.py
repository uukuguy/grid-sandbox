"""Configuration for hermes-runtime."""

import os
from dataclasses import dataclass
from pathlib import Path

from dotenv import load_dotenv


@dataclass
class HermesRuntimeConfig:
    """Runtime configuration from environment variables."""

    grpc_port: int = 50053
    runtime_id: str = "hermes-runtime"
    runtime_name: str = "Hermes Agent Runtime"
    tier: str = "aligned"  # T2

    # hermes-agent config
    hermes_model: str = "anthropic/claude-sonnet-4-20250514"
    hermes_base_url: str = ""
    hermes_api_key: str = ""
    hermes_provider: str = ""
    hermes_max_iterations: int = 50
    hermes_toolsets: str = ""  # comma-separated, empty = all

    # HookBridge sidecar
    hook_bridge_url: str = ""  # e.g. "http://localhost:50054"

    # Deployment
    deployment_mode: str = "shared"  # "shared" or "per_session"

    @classmethod
    def from_env(cls, env_file: str | Path | None = None) -> "HermesRuntimeConfig":
        if env_file:
            load_dotenv(env_file)
        return cls(
            grpc_port=int(os.getenv("HERMES_RUNTIME_PORT", "50053")),
            runtime_id=os.getenv("HERMES_RUNTIME_ID", "hermes-runtime"),
            runtime_name=os.getenv("HERMES_RUNTIME_NAME", "Hermes Agent Runtime"),
            hermes_model=os.getenv("HERMES_MODEL", "anthropic/claude-sonnet-4-20250514"),
            hermes_base_url=os.getenv("HERMES_BASE_URL", ""),
            hermes_api_key=os.getenv("HERMES_API_KEY", os.getenv("OPENROUTER_API_KEY", "")),
            hermes_provider=os.getenv("HERMES_PROVIDER", ""),
            hermes_max_iterations=int(os.getenv("HERMES_MAX_ITERATIONS", "50")),
            hermes_toolsets=os.getenv("HERMES_TOOLSETS", ""),
            hook_bridge_url=os.getenv("HOOK_BRIDGE_URL", ""),
            deployment_mode=os.getenv("HERMES_DEPLOYMENT_MODE", "shared"),
        )
