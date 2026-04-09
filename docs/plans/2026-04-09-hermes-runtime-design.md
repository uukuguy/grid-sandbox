# Hermes Runtime — Grid L1 T2 Aligned Runtime 设计实施方案

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 hermes-agent v0.8.0 (NousResearch) 作为 Grid EAASP L1 执行层的第三个 Runtime，通过 T2 Aligned 模式（Plugin + HookBridge gRPC）接入治理体系。

**Architecture:** 不 fork hermes-agent 源码。写一个 Python gRPC wrapper（hermes-runtime）包装 `AIAgent`，实现 EAASP 16-method RuntimeContract。治理拦截通过 hermes 原生 plugin 系统的 `pre_tool_call`/`post_tool_call` hooks + monkey-patch `handle_function_call` 实现 deny/modify 语义。Plugin 内部通过 gRPC 调用已有的 `grid-hook-bridge` sidecar。

**Tech Stack:** Python 3.11+, grpcio, hermes-agent 0.8.0 (pip), EAASP proto stubs (复用)

---

## 〇、设计决策记录

| ID | 决策 | 原因 |
|----|------|------|
| HR-KD1 | **T2 Aligned**（非 T1/T3） | hermes 是成熟独立 harness (9660 行 agent loop)，T2 薄适配零 fork |
| HR-KD2 | **Shared 部署模式** | hermes gateway 天然支持多会话复用，启动成本低 |
| HR-KD3 | **Plugin + monkey-patch 混合拦截** | hermes `pre_tool_call` hook 存在但返回值被忽略 → plugin 做遥测，monkey-patch 做 deny/modify |
| HR-KD4 | **hermes-agent v0.8.0** @ `268ee6b` | 当前 3th-party/harnesses/hermes-agent 的固定版本 |
| HR-KD5 | **gRPC EvaluateHook（单次模式）** | 优先使用简单 RPC 而非 StreamHooks 双向流，降低 Python 侧复杂度 |
| HR-KD6 | **容器基于官方 Dockerfile 叠加** | 复用 hermes 的完整依赖（playwright/ffmpeg/node 等） |

---

## 一、整体架构

```
┌──────────────────────────────────────────────────────────┐
│  L3 治理层                                               │
│  ┌──────────────────┐                                    │
│  │ HookBridge       │←─── gRPC EvaluateHook ────┐       │
│  │ Sidecar (Rust)   │                           │       │
│  │ :50054           │                           │       │
│  └──────────────────┘                           │       │
└─────────────────────────────────────────────────┼───────┘
                                                  │
┌─────────────────────────────────────────────────┼───────┐
│  hermes-runtime (Python, :50053)                │       │
│  ┌──────────────────┐   ┌──────────────────────┐│       │
│  │ RuntimeService   │   │ governance_plugin    ││       │
│  │ (gRPC 16 方法)   │   │ ├── __init__.py      ││       │
│  │                  │   │ │   pre_tool_call ────┘│       │
│  │ adapter.py       │   │ │   post_tool_call     │       │
│  │ ┌──────────────┐ │   │ │   monkey-patch       │       │
│  │ │ HermesAdapter│ │   │ └── hook_bridge.py     │       │
│  │ │  AIAgent ────┼─┼───┤     (gRPC client)      │       │
│  │ │  wrapper     │ │   └──────────────────────┘│       │
│  │ └──────────────┘ │                           │       │
│  └──────────────────┘                           │       │
│                                                 │       │
│  ┌──────────────────────────────────────────────┘       │
│  │ hermes-agent v0.8.0 (pip install)                    │
│  │ AIAgent.run_conversation() ← 同步 agent loop         │
│  │ model_tools.handle_function_call() ← 工具分发         │
│  │ tools/registry.py ← 47 tools, 40 toolsets            │
│  │ tools/mcp_tool.py ← MCP 客户端                       │
│  └──────────────────────────────────────────────────────┘
└─────────────────────────────────────────────────────────┘
```

### 数据流

```
L3 Initialize(SessionPayload)
  → hermes-runtime service.Initialize()
    → HermesAdapter.create_session()
      → AIAgent.__init__(session_id, model, toolsets, ...)
      → governance_plugin 注入（如果有 hook_bridge_url）
    → return session_id

L4 Send(session_id, message)
  → hermes-runtime service.Send()
    → HermesAdapter.send()
      → AIAgent.run_conversation(message, conversation_history)
        → LLM API call
        → tool_calls?
          → monkey-patched handle_function_call()
            → governance_plugin.pre_hook() → gRPC EvaluateHook
              → Allow: registry.dispatch()
              → Deny: return error JSON to LLM
              → Modify: dispatch with modified args
            → governance_plugin.post_hook() → gRPC report
        → final response
      → yield ResponseChunk stream
```

---

## 二、目录结构

```
lang/hermes-runtime-python/
├── pyproject.toml
├── Dockerfile
├── Makefile                    # 开发命令
├── src/hermes_runtime/
│   ├── __init__.py
│   ├── __main__.py             # gRPC server 启动入口
│   ├── _proto/                 # 符号链接 → claude-code-runtime 的 proto stubs
│   ├── config.py               # HermesRuntimeConfig
│   ├── service.py              # RuntimeServiceImpl (16 方法)
│   ├── adapter.py              # HermesAdapter — AIAgent 包装层
│   ├── mapper.py               # hermes 消息 ↔ proto 转换
│   ├── telemetry.py            # TelemetryCollector (复用 claude-code-runtime 的)
│   ├── session.py              # SessionManager (复用模式)
│   └── governance_plugin/      # hermes plugin — grid 治理接入
│       ├── plugin.yaml
│       ├── __init__.py         # register(ctx) — hooks + monkey-patch
│       └── hook_bridge.py      # gRPC HookBridge 客户端 (Python)
└── tests/
    └── test_hermes_runtime.py  # 单元测试
```

---

## 三、实施任务

### Task 1: 项目骨架 + Config

**Files:**
- Create: `lang/hermes-runtime-python/pyproject.toml`
- Create: `lang/hermes-runtime-python/src/hermes_runtime/__init__.py`
- Create: `lang/hermes-runtime-python/src/hermes_runtime/config.py`
- Create: `lang/hermes-runtime-python/tests/__init__.py`
- Test: `lang/hermes-runtime-python/tests/test_config.py`

**Step 1: Write pyproject.toml**

```toml
[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[project]
name = "hermes-runtime"
version = "0.1.0"
description = "Grid EAASP L1 Runtime — hermes-agent T2 Aligned adapter"
requires-python = ">=3.11"
dependencies = [
  "hermes-agent>=0.8.0",
  "grpcio>=1.62.0,<2",
  "grpcio-tools>=1.62.0,<2",
  "protobuf>=4.25.0,<6",
  "python-dotenv>=1.0.0,<2",
]

[project.optional-dependencies]
dev = ["pytest>=9.0.0,<10", "pytest-asyncio>=1.0.0,<2"]

[tool.setuptools.packages.find]
where = ["src"]

[tool.pytest.ini_options]
testpaths = ["tests"]
asyncio_mode = "auto"
```

**Step 2: Write config.py**

```python
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
```

**Step 3: Write failing test**

```python
# tests/test_config.py
import os
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
```

**Step 4: Run tests**

```bash
cd lang/hermes-runtime-python
uv venv .venv --python 3.11
source .venv/bin/activate
uv pip install -e ".[dev]"
pytest tests/test_config.py -xvs
```

**Step 5: Commit**

```bash
git add lang/hermes-runtime-python/
git commit -m "feat(hermes-runtime): W1 project skeleton + HermesRuntimeConfig (2 tests)"
```

---

### Task 2: Proto Stubs + SessionManager

**Files:**
- Create: `lang/hermes-runtime-python/src/hermes_runtime/_proto/` (symlink)
- Create: `lang/hermes-runtime-python/src/hermes_runtime/session.py`
- Test: `lang/hermes-runtime-python/tests/test_session.py`

**Step 1: Link proto stubs**

复用 claude-code-runtime 已编译的 proto stubs：

```bash
cd lang/hermes-runtime-python/src/hermes_runtime
ln -s ../../../claude-code-runtime-python/src/claude_code_runtime/_proto _proto
```

验证 import：

```python
from hermes_runtime._proto.eaasp.runtime.v1 import runtime_pb2
from hermes_runtime._proto.eaasp.common.v1 import common_pb2
```

**Step 2: Write session.py**

```python
"""Session manager for hermes-runtime — tracks active AIAgent instances."""

import time
import uuid
from dataclasses import dataclass, field


@dataclass
class HermesSession:
    session_id: str
    user_id: str
    user_role: str
    org_unit: str
    managed_hooks_json: str = ""
    context: dict = field(default_factory=dict)
    hook_bridge_url: str = ""
    telemetry_endpoint: str = ""
    skills: list = field(default_factory=list)
    mcp_servers: list = field(default_factory=list)
    conversation_history: list = field(default_factory=list)
    created_at: str = field(default_factory=lambda: time.strftime("%Y-%m-%dT%H:%M:%SZ"))
    paused: bool = False


class SessionManager:
    def __init__(self):
        self._sessions: dict[str, HermesSession] = {}

    @property
    def count(self) -> int:
        return len(self._sessions)

    def create(self, **kwargs) -> HermesSession:
        sid = f"hermes-{uuid.uuid4().hex[:12]}"
        session = HermesSession(session_id=sid, **kwargs)
        self._sessions[sid] = session
        return session

    def get(self, session_id: str) -> HermesSession | None:
        return self._sessions.get(session_id)

    def terminate(self, session_id: str) -> HermesSession | None:
        return self._sessions.pop(session_id, None)

    def pause(self, session_id: str) -> bool:
        s = self._sessions.get(session_id)
        if s:
            s.paused = True
            return True
        return False

    def resume(self, session_id: str) -> bool:
        s = self._sessions.get(session_id)
        if s and s.paused:
            s.paused = False
            return True
        return False

    def restore(self, data: dict) -> HermesSession:
        session = HermesSession(**data)
        self._sessions[session.session_id] = session
        return session
```

**Step 3: Write failing test**

```python
# tests/test_session.py
from hermes_runtime.session import SessionManager


def test_session_lifecycle():
    mgr = SessionManager()
    assert mgr.count == 0

    s = mgr.create(user_id="u1", user_role="developer", org_unit="eng")
    assert s.session_id.startswith("hermes-")
    assert mgr.count == 1

    assert mgr.get(s.session_id) is s
    assert mgr.get("nonexistent") is None


def test_session_pause_resume():
    mgr = SessionManager()
    s = mgr.create(user_id="u1", user_role="dev", org_unit="eng")
    assert not s.paused
    assert mgr.pause(s.session_id)
    assert s.paused
    assert mgr.resume(s.session_id)
    assert not s.paused


def test_session_terminate():
    mgr = SessionManager()
    s = mgr.create(user_id="u1", user_role="dev", org_unit="eng")
    sid = s.session_id
    terminated = mgr.terminate(sid)
    assert terminated is s
    assert mgr.count == 0
    assert mgr.get(sid) is None
```

**Step 4: Run tests**

```bash
pytest tests/test_session.py -xvs
```

**Step 5: Commit**

```bash
git add lang/hermes-runtime-python/
git commit -m "feat(hermes-runtime): W2 proto stubs symlink + SessionManager (3 tests)"
```

---

### Task 3: Governance Plugin — HookBridge gRPC 客户端

**Files:**
- Create: `lang/hermes-runtime-python/src/hermes_runtime/governance_plugin/plugin.yaml`
- Create: `lang/hermes-runtime-python/src/hermes_runtime/governance_plugin/__init__.py`
- Create: `lang/hermes-runtime-python/src/hermes_runtime/governance_plugin/hook_bridge.py`
- Test: `lang/hermes-runtime-python/tests/test_governance_plugin.py`

**Step 1: Write plugin.yaml**

```yaml
name: grid-governance
version: "0.1.0"
description: "Grid EAASP L3 governance integration — HookBridge relay for tool-call interception"
author: "Grid Platform"
provides_hooks:
  - pre_tool_call
  - post_tool_call
  - on_session_start
  - on_session_end
```

**Step 2: Write hook_bridge.py (gRPC client)**

```python
"""gRPC client for Grid HookBridge sidecar — EvaluateHook one-shot mode."""

import json
import logging
import grpc

logger = logging.getLogger(__name__)

# Proto stubs — shared with hermes-runtime
_common_pb2 = None
_hook_pb2 = None
_hook_pb2_grpc = None


def _lazy_import():
    global _common_pb2, _hook_pb2, _hook_pb2_grpc
    if _common_pb2 is not None:
        return
    from hermes_runtime._proto.eaasp.common.v1 import common_pb2
    from hermes_runtime._proto.eaasp.hook.v1 import hook_pb2, hook_pb2_grpc
    _common_pb2 = common_pb2
    _hook_pb2 = hook_pb2
    _hook_pb2_grpc = hook_pb2_grpc


class HookBridgeClient:
    """Synchronous gRPC client for HookBridge EvaluateHook."""

    def __init__(self, url: str):
        self._url = url
        self._channel: grpc.Channel | None = None
        self._stub = None

    def _ensure_connected(self):
        if self._channel is None:
            _lazy_import()
            self._channel = grpc.insecure_channel(self._url)
            self._stub = _hook_pb2_grpc.HookBridgeServiceStub(self._channel)

    def evaluate_pre_tool_call(
        self, session_id: str, tool_name: str, tool_id: str, input_json: str
    ) -> tuple[str, str, str]:
        """Returns (decision, reason, modified_input). decision: 'allow'|'deny'|'modify'."""
        try:
            self._ensure_connected()
            request = _hook_pb2.HookEvaluateRequest(
                session_id=session_id,
                hook_type="pre_tool_call",
                tool_name=tool_name,
                tool_id=tool_id,
                input_json=input_json,
            )
            response = self._stub.EvaluateHook(request, timeout=5.0)
            return response.decision, response.reason, response.modified_input
        except Exception as e:
            logger.warning("HookBridge pre_tool_call failed (allow-on-error): %s", e)
            return "allow", "", ""

    def evaluate_post_tool_result(
        self, session_id: str, tool_name: str, tool_id: str, output: str, is_error: bool
    ) -> tuple[str, str, str]:
        try:
            self._ensure_connected()
            request = _hook_pb2.HookEvaluateRequest(
                session_id=session_id,
                hook_type="post_tool_result",
                tool_name=tool_name,
                tool_id=tool_id,
                output=output,
                is_error=is_error,
            )
            response = self._stub.EvaluateHook(request, timeout=5.0)
            return response.decision, response.reason, response.modified_input
        except Exception as e:
            logger.warning("HookBridge post_tool_result failed: %s", e)
            return "allow", "", ""

    def close(self):
        if self._channel:
            self._channel.close()
            self._channel = None
            self._stub = None
```

**Step 3: Write plugin __init__.py (register + monkey-patch)**

```python
"""Grid Governance plugin for hermes-agent — HookBridge relay."""

import functools
import json
import logging
import os

logger = logging.getLogger(__name__)

# Module-level state — set during register(), used by hooks
_hook_bridge = None
_session_id = ""  # set per-session by hermes-runtime adapter
_original_handle_function_call = None


def set_session_id(sid: str):
    """Called by hermes-runtime adapter to set current session context."""
    global _session_id
    _session_id = sid


def register(ctx):
    """Hermes plugin registration — called by PluginManager."""
    bridge_url = os.getenv("HOOK_BRIDGE_URL", "")
    if not bridge_url:
        logger.info("grid-governance: HOOK_BRIDGE_URL not set, running in audit-only mode")

    # Register hooks for observability/telemetry
    ctx.register_hook("pre_tool_call", _on_pre_tool_call)
    ctx.register_hook("post_tool_call", _on_post_tool_call)
    ctx.register_hook("on_session_start", _on_session_start)
    ctx.register_hook("on_session_end", _on_session_end)

    # Monkey-patch handle_function_call for deny/modify support
    if bridge_url:
        from .hook_bridge import HookBridgeClient
        global _hook_bridge
        _hook_bridge = HookBridgeClient(bridge_url)
        _install_tool_call_interceptor()
        logger.info("grid-governance: HookBridge connected at %s", bridge_url)


def _install_tool_call_interceptor():
    """Wrap model_tools.handle_function_call to check HookBridge decisions."""
    import model_tools
    global _original_handle_function_call
    _original_handle_function_call = model_tools.handle_function_call

    @functools.wraps(_original_handle_function_call)
    def _intercepted_handle_function_call(
        function_name, function_args, task_id=None, **kwargs
    ):
        if _hook_bridge is None:
            return _original_handle_function_call(
                function_name, function_args, task_id=task_id, **kwargs
            )

        # Pre-tool-call governance check
        input_json = json.dumps(function_args, ensure_ascii=False)
        tool_id = kwargs.get("tool_call_id", "") or ""
        decision, reason, modified_input = _hook_bridge.evaluate_pre_tool_call(
            session_id=_session_id,
            tool_name=function_name,
            tool_id=tool_id,
            input_json=input_json,
        )

        if decision == "deny":
            logger.warning(
                "grid-governance DENIED tool call: %s reason=%s", function_name, reason
            )
            return json.dumps(
                {"error": f"[Grid Governance] Tool call denied: {reason}"},
                ensure_ascii=False,
            )

        if decision == "modify" and modified_input:
            try:
                function_args = json.loads(modified_input)
            except json.JSONDecodeError:
                pass

        return _original_handle_function_call(
            function_name, function_args, task_id=task_id, **kwargs
        )

    model_tools.handle_function_call = _intercepted_handle_function_call


# ── Hook callbacks (observability only, return value ignored by hermes) ──

def _on_pre_tool_call(**kwargs):
    logger.debug("grid-governance pre_tool_call: %s", kwargs.get("tool_name"))


def _on_post_tool_call(**kwargs):
    logger.debug("grid-governance post_tool_call: %s", kwargs.get("tool_name"))


def _on_session_start(**kwargs):
    logger.info("grid-governance session_start")


def _on_session_end(**kwargs):
    logger.info("grid-governance session_end")
    global _hook_bridge
    if _hook_bridge:
        _hook_bridge.close()
```

**Step 4: Write tests**

```python
# tests/test_governance_plugin.py
import json
from unittest.mock import MagicMock, patch


def test_hook_bridge_client_allow_on_error():
    """HookBridge 连接失败时 fallback 到 allow。"""
    from hermes_runtime.governance_plugin.hook_bridge import HookBridgeClient

    client = HookBridgeClient("http://localhost:99999")
    decision, reason, modified = client.evaluate_pre_tool_call(
        "s1", "terminal", "t1", '{"command": "ls"}'
    )
    assert decision == "allow"


def test_interceptor_deny(monkeypatch):
    """Monkey-patch 拦截器在 deny 时返回 error JSON。"""
    from hermes_runtime.governance_plugin import (
        _install_tool_call_interceptor,
        set_session_id,
    )
    import hermes_runtime.governance_plugin as gp

    mock_bridge = MagicMock()
    mock_bridge.evaluate_pre_tool_call.return_value = ("deny", "blocked by policy", "")

    # Mock original handle_function_call
    mock_original = MagicMock(return_value='{"ok": true}')

    gp._hook_bridge = mock_bridge
    gp._original_handle_function_call = mock_original
    set_session_id("test-session")

    # Simulate the intercepted call
    import model_tools
    with patch.object(model_tools, "handle_function_call", mock_original):
        _install_tool_call_interceptor()
        result = model_tools.handle_function_call("terminal", {"command": "rm -rf /"})

    parsed = json.loads(result)
    assert "denied" in parsed["error"].lower()
    mock_original.assert_not_called()  # 原始函数不应被调用


def test_interceptor_modify(monkeypatch):
    """Monkey-patch 拦截器在 modify 时替换参数。"""
    from hermes_runtime.governance_plugin import set_session_id
    import hermes_runtime.governance_plugin as gp

    mock_bridge = MagicMock()
    modified_args = json.dumps({"command": "ls -la"})
    mock_bridge.evaluate_pre_tool_call.return_value = ("modify", "", modified_args)

    mock_original = MagicMock(return_value='{"ok": true}')

    gp._hook_bridge = mock_bridge
    gp._original_handle_function_call = mock_original
    set_session_id("test-session")

    import model_tools
    with patch.object(model_tools, "handle_function_call", mock_original):
        gp._install_tool_call_interceptor()
        model_tools.handle_function_call("terminal", {"command": "rm -rf /"})

    # 验证原始函数被调用时参数已被替换
    call_args = mock_original.call_args
    assert call_args[0][1] == {"command": "ls -la"}
```

**Step 5: Run tests**

```bash
pytest tests/test_governance_plugin.py -xvs
```

**Step 6: Commit**

```bash
git add lang/hermes-runtime-python/src/hermes_runtime/governance_plugin/
git add lang/hermes-runtime-python/tests/test_governance_plugin.py
git commit -m "feat(hermes-runtime): W3 governance plugin — HookBridge gRPC client + monkey-patch interceptor (3 tests)"
```

---

### Task 4: HermesAdapter — AIAgent 包装层

**Files:**
- Create: `lang/hermes-runtime-python/src/hermes_runtime/adapter.py`
- Create: `lang/hermes-runtime-python/src/hermes_runtime/mapper.py`
- Test: `lang/hermes-runtime-python/tests/test_adapter.py`

**Step 1: Write mapper.py**

```python
"""hermes-agent 消息格式 ↔ EAASP proto 转换。"""

from hermes_runtime._proto.eaasp.runtime.v1 import runtime_pb2
from hermes_runtime._proto.eaasp.common.v1 import common_pb2


def chunk_to_proto(chunk_type: str, content: str, **kwargs) -> runtime_pb2.ResponseChunk:
    """Convert a hermes response fragment to EAASP ResponseChunk proto."""
    return runtime_pb2.ResponseChunk(
        chunk_type=chunk_type,
        content=content,
        tool_name=kwargs.get("tool_name", ""),
        tool_id=kwargs.get("tool_id", ""),
        is_error=kwargs.get("is_error", False),
    )


def telemetry_to_proto(event: dict) -> common_pb2.TelemetryEvent:
    """Convert telemetry dict to proto."""
    return common_pb2.TelemetryEvent(
        session_id=event.get("session_id", ""),
        runtime_id=event.get("runtime_id", ""),
        event_type=event.get("event_type", ""),
        timestamp=event.get("timestamp", ""),
        payload_json=str(event.get("payload", {})),
    )
```

**Step 2: Write adapter.py**

```python
"""HermesAdapter — wraps hermes-agent AIAgent for EAASP RuntimeContract."""

import json
import logging
import queue
import threading
from typing import Generator

from hermes_runtime.config import HermesRuntimeConfig
from hermes_runtime.governance_plugin import set_session_id

logger = logging.getLogger(__name__)


class HermesAdapter:
    """Manages one AIAgent instance per session, adapts sync→async streaming."""

    def __init__(self, config: HermesRuntimeConfig):
        self._config = config
        self._agents: dict[str, object] = {}  # session_id → AIAgent

    def create_agent(self, session_id: str, **session_kwargs) -> None:
        """Create and store an AIAgent for this session."""
        from run_agent import AIAgent

        enabled_toolsets = None
        if self._config.hermes_toolsets:
            enabled_toolsets = [t.strip() for t in self._config.hermes_toolsets.split(",") if t.strip()]

        agent = AIAgent(
            base_url=self._config.hermes_base_url or None,
            api_key=self._config.hermes_api_key or None,
            provider=self._config.hermes_provider or None,
            model=self._config.hermes_model,
            max_iterations=self._config.hermes_max_iterations,
            enabled_toolsets=enabled_toolsets,
            session_id=session_id,
            quiet_mode=True,
            skip_context_files=True,
            skip_memory=True,
        )
        self._agents[session_id] = agent

    def get_agent(self, session_id: str):
        return self._agents.get(session_id)

    def remove_agent(self, session_id: str):
        return self._agents.pop(session_id, None)

    def send_message(
        self,
        session_id: str,
        content: str,
        conversation_history: list | None = None,
    ) -> Generator[dict, None, None]:
        """Run conversation synchronously, yield chunks via thread bridge.

        hermes AIAgent.run_conversation() is synchronous and blocking.
        We run it in a background thread and bridge results via a queue.
        """
        agent = self._agents.get(session_id)
        if agent is None:
            yield {"chunk_type": "error", "content": f"No agent for session {session_id}"}
            return

        set_session_id(session_id)

        result_queue: queue.Queue = queue.Queue()

        def _stream_delta(text: str):
            result_queue.put({"chunk_type": "text_delta", "content": text})

        def _tool_start(tool_name: str, args_preview: str):
            result_queue.put({
                "chunk_type": "tool_start",
                "content": args_preview,
                "tool_name": tool_name,
            })

        def _tool_complete(tool_name: str, result_preview: str):
            result_queue.put({
                "chunk_type": "tool_result",
                "content": result_preview,
                "tool_name": tool_name,
            })

        agent.stream_delta_callback = _stream_delta
        agent.tool_start_callback = _tool_start
        agent.tool_complete_callback = _tool_complete

        def _run():
            try:
                result = agent.run_conversation(
                    user_message=content,
                    conversation_history=conversation_history or [],
                )
                final = result.get("final_response", "") if isinstance(result, dict) else str(result)
                result_queue.put({"chunk_type": "done", "content": final})
            except Exception as e:
                result_queue.put({"chunk_type": "error", "content": str(e), "is_error": True})
            finally:
                result_queue.put(None)  # sentinel

        thread = threading.Thread(target=_run, daemon=True)
        thread.start()

        while True:
            item = result_queue.get()
            if item is None:
                break
            yield item
```

**Step 3: Write tests**

```python
# tests/test_adapter.py
from hermes_runtime.adapter import HermesAdapter
from hermes_runtime.config import HermesRuntimeConfig


def test_adapter_create_remove():
    """Adapter 可以创建和移除 agent 占位（不实际初始化 AIAgent）。"""
    config = HermesRuntimeConfig()
    adapter = HermesAdapter(config)
    # 直接操作内部 dict 模拟（避免实际导入 hermes AIAgent）
    adapter._agents["test-1"] = "mock-agent"
    assert adapter.get_agent("test-1") == "mock-agent"
    assert adapter.get_agent("nonexistent") is None
    removed = adapter.remove_agent("test-1")
    assert removed == "mock-agent"
    assert adapter.get_agent("test-1") is None


def test_send_message_no_agent():
    """send_message 对不存在的 session 返回 error chunk。"""
    config = HermesRuntimeConfig()
    adapter = HermesAdapter(config)
    chunks = list(adapter.send_message("nonexistent", "hello"))
    assert len(chunks) == 1
    assert chunks[0]["chunk_type"] == "error"
```

**Step 4: Run tests**

```bash
pytest tests/test_adapter.py -xvs
```

**Step 5: Commit**

```bash
git add lang/hermes-runtime-python/src/hermes_runtime/adapter.py
git add lang/hermes-runtime-python/src/hermes_runtime/mapper.py
git add lang/hermes-runtime-python/tests/test_adapter.py
git commit -m "feat(hermes-runtime): W4 HermesAdapter + mapper — AIAgent wrapper with thread bridge (2 tests)"
```

---

### Task 5: RuntimeServiceImpl — 16 方法 gRPC 服务

**Files:**
- Create: `lang/hermes-runtime-python/src/hermes_runtime/service.py`
- Create: `lang/hermes-runtime-python/src/hermes_runtime/telemetry.py`
- Create: `lang/hermes-runtime-python/src/hermes_runtime/__main__.py`
- Test: `lang/hermes-runtime-python/tests/test_service.py`

**Step 1: Write telemetry.py**（复用 claude-code-runtime 模式）

```python
"""Telemetry collector for hermes-runtime."""

import time
import uuid
from dataclasses import dataclass, field


@dataclass
class TelemetryEntry:
    event_type: str
    session_id: str
    runtime_id: str
    user_id: str = ""
    timestamp: str = field(default_factory=lambda: time.strftime("%Y-%m-%dT%H:%M:%SZ"))
    payload: dict = field(default_factory=dict)


class TelemetryCollector:
    def __init__(self, session_id: str, runtime_id: str, user_id: str = ""):
        self.session_id = session_id
        self.runtime_id = runtime_id
        self.user_id = user_id
        self._entries: list[TelemetryEntry] = []

    def record(self, event_type: str, payload: dict | None = None):
        self._entries.append(TelemetryEntry(
            event_type=event_type,
            session_id=self.session_id,
            runtime_id=self.runtime_id,
            user_id=self.user_id,
            payload=payload or {},
        ))

    def peek(self) -> list[TelemetryEntry]:
        return list(self._entries)

    def flush(self) -> list[TelemetryEntry]:
        entries = list(self._entries)
        self._entries.clear()
        return entries
```

**Step 2: Write service.py**

```python
"""gRPC RuntimeService — EAASP L1 16-method contract for hermes-agent."""

import logging
import time

import grpc

from hermes_runtime._proto.eaasp.common.v1 import common_pb2
from hermes_runtime._proto.eaasp.runtime.v1 import runtime_pb2, runtime_pb2_grpc
from hermes_runtime.adapter import HermesAdapter
from hermes_runtime.config import HermesRuntimeConfig
from hermes_runtime.mapper import chunk_to_proto
from hermes_runtime.session import SessionManager
from hermes_runtime.telemetry import TelemetryCollector

logger = logging.getLogger(__name__)


class RuntimeServiceImpl(runtime_pb2_grpc.RuntimeServiceServicer):
    """EAASP L1 RuntimeService — Hermes Agent T2 Aligned."""

    def __init__(self, config: HermesRuntimeConfig):
        self.config = config
        self.adapter = HermesAdapter(config)
        self.session_mgr = SessionManager()
        self._telemetry: dict[str, TelemetryCollector] = {}
        self._start_time = time.time()

    def _get_or_404(self, session_id: str, context):
        session = self.session_mgr.get(session_id)
        if session is None:
            context.set_code(grpc.StatusCode.NOT_FOUND)
            context.set_details(f"Session {session_id} not found")
        return session

    # ── 1. Health ──

    async def Health(self, request, context):
        return runtime_pb2.HealthStatus(
            healthy=True,
            runtime_id=self.config.runtime_id,
            checks={
                "hermes": "ok",
                "sessions": str(self.session_mgr.count),
                "uptime": f"{time.time() - self._start_time:.0f}s",
            },
        )

    # ── 2. GetCapabilities ──

    async def GetCapabilities(self, request, context):
        return runtime_pb2.CapabilityManifest(
            runtime_id=self.config.runtime_id,
            runtime_name=self.config.runtime_name,
            tier=self.config.tier,
            model=self.config.hermes_model,
            context_window=200000,
            supported_tools=[
                "terminal", "read_file", "write_file", "patch", "search_files",
                "web_search", "web_extract", "browser_navigate", "execute_code",
                "delegate_task", "memory", "todo", "skills_list", "skill_view",
            ],
            native_hooks=False,  # T2 — uses HookBridge
            native_mcp=True,     # hermes has native MCP
            native_skills=True,  # hermes has native skills
            requires_hook_bridge=True,
            deployment_mode=self.config.deployment_mode,
            cost=runtime_pb2.CostEstimate(
                input_cost_per_1k=0.0,  # depends on model choice
                output_cost_per_1k=0.0,
            ),
        )

    # ── 3. Initialize ──

    async def Initialize(self, request, context):
        payload = request.payload
        session = self.session_mgr.create(
            user_id=payload.user_id,
            user_role=payload.user_role,
            org_unit=payload.org_unit,
            managed_hooks_json=payload.managed_hooks_json,
            context=dict(payload.context) if payload.context else {},
            hook_bridge_url=payload.hook_bridge_url or self.config.hook_bridge_url,
            telemetry_endpoint=payload.telemetry_endpoint,
        )
        sid = session.session_id

        # Create AIAgent instance for this session
        try:
            self.adapter.create_agent(sid)
        except Exception as e:
            logger.error("Failed to create AIAgent for %s: %s", sid, e)
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(str(e))
            self.session_mgr.terminate(sid)
            return runtime_pb2.InitializeResponse(session_id="")

        self._telemetry[sid] = TelemetryCollector(
            session_id=sid,
            runtime_id=self.config.runtime_id,
            user_id=payload.user_id,
        )
        self._telemetry[sid].record("session_start")

        logger.info("Session initialized: %s (user=%s, model=%s)",
                     sid, payload.user_id, self.config.hermes_model)
        return runtime_pb2.InitializeResponse(session_id=sid)

    # ── 4. Send (streaming) ──

    async def Send(self, request, context):
        session = self._get_or_404(request.session_id, context)
        if session is None:
            return

        sid = session.session_id
        message = request.message
        logger.info("Send: session=%s content=%s", sid, message.content[:80])

        tc = self._telemetry.get(sid)
        if tc:
            tc.record("send", payload={"content_len": len(message.content)})

        for chunk in self.adapter.send_message(
            session_id=sid,
            content=message.content,
            conversation_history=session.conversation_history,
        ):
            yield chunk_to_proto(**chunk)

        # Persist conversation entry
        session.conversation_history.append({"role": "user", "content": message.content})

    # ── 5. LoadSkill ──

    async def LoadSkill(self, request, context):
        session = self._get_or_404(request.session_id, context)
        if session is None:
            return runtime_pb2.LoadSkillResponse(success=False, error="session not found")

        skill = request.skill
        session.skills.append({"skill_id": skill.skill_id, "name": skill.name})

        tc = self._telemetry.get(session.session_id)
        if tc:
            tc.record("skill_loaded", payload={"skill_id": skill.skill_id})

        return runtime_pb2.LoadSkillResponse(success=True)

    # ── 6. OnToolCall ──

    async def OnToolCall(self, request, context):
        # T2: 治理拦截已在 governance_plugin monkey-patch 中完成
        # 这里直接返回 allow（避免双重检查）
        return common_pb2.HookDecision(decision="allow", reason="", modified_input="")

    # ── 7. OnToolResult ──

    async def OnToolResult(self, request, context):
        return common_pb2.HookDecision(decision="allow", reason="", modified_input="")

    # ── 8. OnStop ──

    async def OnStop(self, request, context):
        return common_pb2.StopDecision(decision="complete", feedback="")

    # ── 9. ConnectMcp ──

    async def ConnectMcp(self, request, context):
        session = self._get_or_404(request.session_id, context)
        if session is None:
            return runtime_pb2.ConnectMcpResponse(success=False)
        # hermes-agent 的 MCP 连接在 config.yaml 中管理，此处记录
        connected = [s.name for s in request.servers]
        session.mcp_servers.extend(connected)
        return runtime_pb2.ConnectMcpResponse(success=True, connected=connected, failed=[])

    # ── 10. DisconnectMcp ──

    async def DisconnectMcp(self, request, context):
        session = self.session_mgr.get(request.session_id)
        if session and request.server_name in session.mcp_servers:
            session.mcp_servers.remove(request.server_name)
        return runtime_pb2.DisconnectMcpResponse(success=True)

    # ── 11. EmitTelemetry ──

    async def EmitTelemetry(self, request, context):
        tc = self._telemetry.get(request.session_id)
        if tc:
            entries = tc.peek()
            return common_pb2.TelemetryBatch(events=[])  # TODO: full mapping
        return common_pb2.TelemetryBatch(events=[])

    # ── 12. GetState ──

    async def GetState(self, request, context):
        session = self._get_or_404(request.session_id, context)
        if session is None:
            return runtime_pb2.SessionState()
        import json
        state_data = json.dumps({
            "session_id": session.session_id,
            "user_id": session.user_id,
            "conversation_history": session.conversation_history,
            "skills": session.skills,
        }).encode()
        return runtime_pb2.SessionState(
            session_id=session.session_id,
            state_data=state_data,
            runtime_id=self.config.runtime_id,
            created_at=session.created_at,
            state_format="hermes-json-v1",
        )

    # ── 13. RestoreState ──

    async def RestoreState(self, request, context):
        import json
        try:
            data = json.loads(request.state_data)
            session = self.session_mgr.restore(data)
            self.adapter.create_agent(session.session_id)
            sid = session.session_id
            self._telemetry[sid] = TelemetryCollector(
                session_id=sid, runtime_id=self.config.runtime_id, user_id=session.user_id,
            )
            return runtime_pb2.InitializeResponse(session_id=sid)
        except Exception as e:
            context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
            context.set_details(str(e))
            return runtime_pb2.InitializeResponse(session_id="")

    # ── 14. PauseSession ──

    async def PauseSession(self, request, context):
        success = self.session_mgr.pause(request.session_id)
        return runtime_pb2.PauseResponse(success=success)

    # ── 15. ResumeSession ──

    async def ResumeSession(self, request, context):
        success = self.session_mgr.resume(request.session_id)
        if success:
            return runtime_pb2.ResumeResponse(success=True, session_id=request.session_id)
        context.set_code(grpc.StatusCode.NOT_FOUND)
        return runtime_pb2.ResumeResponse(success=False, session_id="")

    # ── 16. Terminate ──

    async def Terminate(self, request, context):
        sid = request.session_id
        tc = self._telemetry.pop(sid, None)
        if tc:
            tc.record("session_end")
            tc.flush()
        self.adapter.remove_agent(sid)
        session = self.session_mgr.terminate(sid)
        return runtime_pb2.TerminateResponse(success=session is not None)
```

**Step 3: Write __main__.py**

```python
"""hermes-runtime gRPC server entry point."""

import asyncio
import logging
import grpc
from grpc import aio

from hermes_runtime._proto.eaasp.runtime.v1 import runtime_pb2_grpc
from hermes_runtime.config import HermesRuntimeConfig
from hermes_runtime.service import RuntimeServiceImpl

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(levelname)s %(message)s")
logger = logging.getLogger("hermes-runtime")


async def serve():
    config = HermesRuntimeConfig.from_env()
    server = aio.server()
    service = RuntimeServiceImpl(config)
    runtime_pb2_grpc.add_RuntimeServiceServicer_to_server(service, server)
    addr = f"[::]:{config.grpc_port}"
    server.add_insecure_port(addr)
    logger.info("hermes-runtime starting on %s (model=%s, tier=%s)",
                addr, config.hermes_model, config.tier)
    await server.start()
    await server.wait_for_termination()


def main():
    asyncio.run(serve())


if __name__ == "__main__":
    main()
```

**Step 4: Write tests**

```python
# tests/test_service.py
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
```

**Step 5: Run tests**

```bash
pytest tests/ -xvs
```

**Step 6: Commit**

```bash
git add lang/hermes-runtime-python/
git commit -m "feat(hermes-runtime): W5 RuntimeServiceImpl 16 方法 + telemetry + gRPC server entry (2 tests)"
```

---

### Task 6: Dockerfile + Makefile + Certifier 验证

**Files:**
- Create: `lang/hermes-runtime-python/Dockerfile`
- Modify: `Makefile` — 添加 hermes-runtime targets
- Test: eaasp-certifier verify

**Step 1: Write Dockerfile**

```dockerfile
FROM python:3.11-slim

# System deps for hermes-agent
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential nodejs npm ripgrep git && \
    rm -rf /var/lib/apt/lists/*

# Install hermes-agent from local source
COPY 3th-party/harnesses/hermes-agent /opt/hermes
WORKDIR /opt/hermes
RUN pip install --no-cache-dir -e ".[all]"

# Install hermes-runtime
COPY lang/hermes-runtime-python /opt/hermes-runtime
WORKDIR /opt/hermes-runtime
RUN pip install --no-cache-dir -e ".[dev]"

# Link governance plugin into hermes plugin directory
ENV HERMES_HOME=/opt/data
RUN mkdir -p /opt/data/plugins && \
    ln -s /opt/hermes-runtime/src/hermes_runtime/governance_plugin /opt/data/plugins/grid-governance

EXPOSE 50053

CMD ["python", "-m", "hermes_runtime"]
```

**Step 2: Add Makefile targets**

在项目根 `Makefile` 中添加：

```makefile
# ── Hermes Runtime ──
hermes-runtime-setup:
	cd lang/hermes-runtime-python && uv venv .venv --python 3.11 && \
	source .venv/bin/activate && uv pip install -e ".[dev]"

hermes-runtime-test:
	cd lang/hermes-runtime-python && source .venv/bin/activate && \
	pytest tests/ -xvs

hermes-runtime-start:
	cd lang/hermes-runtime-python && source .venv/bin/activate && \
	python -m hermes_runtime

hermes-runtime-build:
	docker build -f lang/hermes-runtime-python/Dockerfile -t hermes-runtime:latest .
```

**Step 3: Certifier 验证（手动，需 API Key）**

```bash
# 终端 1: 启动 hermes-runtime
make hermes-runtime-start

# 终端 2: certifier 验证
cargo run -p eaasp-certifier --release -- verify --endpoint http://localhost:50053
```

**Step 4: Commit**

```bash
git add lang/hermes-runtime-python/Dockerfile Makefile
git commit -m "feat(hermes-runtime): W6 Dockerfile + Makefile targets + certifier integration"
```

---

## 四、Deferred Items

| ID | 内容 | 触发条件 |
|----|------|---------|
| HR-D1 | StreamHooks 双向流模式（替代 EvaluateHook） | 延迟敏感场景需要长连接 |
| HR-D2 | Hermes Skills ↔ L2 Skill Registry 双向同步 | L2 Registry REST API 稳定后 |
| HR-D3 | Hermes Memory ↔ L2 Memory Engine 集成 | L2 Memory Engine 实现后 |
| HR-D4 | 会话状态序列化完善（含 hermes 内部 todo/memory） | 跨 runtime 会话迁移需求 |
| HR-D5 | hermes MCP 服务器列表 ↔ L2 MCP Orchestrator 同步 | L2 MCP Orchestrator 完善后 |
| HR-D6 | hermes 多平台 gateway 接入 L5 协作层 | L5 协作层设计完成后 |
| HR-D7 | 容器化自动构建 CI (GitHub Actions) | 发布流程需要时 |
| HR-D8 | hermes context_compressor 遥测上报 | 精细化 token 用量追踪 |
| HR-D9 | hermes subagent (delegate_task) 的治理透传 | 子 agent 也需治理拦截时 |
| HR-D10 | PerSession 部署模式支持 | 强隔离场景需求 |

---

## 五、测试矩阵

| 层级 | 测试内容 | 命令 | 预期 |
|------|---------|------|------|
| 单元 | Config/Session/Telemetry | `pytest tests/ -xvs` | 10+ tests pass |
| 单元 | Governance plugin (deny/modify/allow) | `pytest tests/test_governance_plugin.py` | 3 tests pass |
| 集成 | gRPC Health/GetCapabilities | certifier verify (无需 API Key) | 2/16 pass |
| 集成 | Initialize + Send + Terminate | certifier verify (需 API Key) | 16/16 pass |
| E2E | L3 → hermes-runtime → HookBridge → deny | 手动 | deny 结果正确传播 |

---

## 六、里程碑

| Week | 内容 | 产出 |
|------|------|------|
| W1 | 项目骨架 + Config | pyproject.toml, config.py (2 tests) |
| W2 | Proto stubs + SessionManager | session.py (3 tests) |
| W3 | Governance Plugin + HookBridge client | plugin + hook_bridge.py (3 tests) |
| W4 | HermesAdapter + mapper | adapter.py, mapper.py (2 tests) |
| W5 | RuntimeServiceImpl 16 方法 | service.py, __main__.py (2 tests) |
| W6 | Dockerfile + Makefile + certifier | 容器构建 + 集成验证 |

**总计：~12 tests, ~6 个文件核心代码（~800 行 Python）**
