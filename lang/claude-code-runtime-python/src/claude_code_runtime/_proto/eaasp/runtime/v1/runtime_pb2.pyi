from eaasp.common.v1 import common_pb2 as _common_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class InitializeRequest(_message.Message):
    __slots__ = ("payload",)
    PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    payload: SessionPayload
    def __init__(self, payload: _Optional[_Union[SessionPayload, _Mapping]] = ...) -> None: ...

class InitializeResponse(_message.Message):
    __slots__ = ("session_id",)
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    def __init__(self, session_id: _Optional[str] = ...) -> None: ...

class SessionPayload(_message.Message):
    __slots__ = ("user_id", "user_role", "org_unit", "managed_hooks_json", "quotas", "context", "hook_bridge_url", "telemetry_endpoint")
    class QuotasEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    class ContextEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    USER_ID_FIELD_NUMBER: _ClassVar[int]
    USER_ROLE_FIELD_NUMBER: _ClassVar[int]
    ORG_UNIT_FIELD_NUMBER: _ClassVar[int]
    MANAGED_HOOKS_JSON_FIELD_NUMBER: _ClassVar[int]
    QUOTAS_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    HOOK_BRIDGE_URL_FIELD_NUMBER: _ClassVar[int]
    TELEMETRY_ENDPOINT_FIELD_NUMBER: _ClassVar[int]
    user_id: str
    user_role: str
    org_unit: str
    managed_hooks_json: str
    quotas: _containers.ScalarMap[str, str]
    context: _containers.ScalarMap[str, str]
    hook_bridge_url: str
    telemetry_endpoint: str
    def __init__(self, user_id: _Optional[str] = ..., user_role: _Optional[str] = ..., org_unit: _Optional[str] = ..., managed_hooks_json: _Optional[str] = ..., quotas: _Optional[_Mapping[str, str]] = ..., context: _Optional[_Mapping[str, str]] = ..., hook_bridge_url: _Optional[str] = ..., telemetry_endpoint: _Optional[str] = ...) -> None: ...

class SendRequest(_message.Message):
    __slots__ = ("session_id", "message")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    message: UserMessage
    def __init__(self, session_id: _Optional[str] = ..., message: _Optional[_Union[UserMessage, _Mapping]] = ...) -> None: ...

class UserMessage(_message.Message):
    __slots__ = ("content", "message_type", "metadata")
    class MetadataEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_TYPE_FIELD_NUMBER: _ClassVar[int]
    METADATA_FIELD_NUMBER: _ClassVar[int]
    content: str
    message_type: str
    metadata: _containers.ScalarMap[str, str]
    def __init__(self, content: _Optional[str] = ..., message_type: _Optional[str] = ..., metadata: _Optional[_Mapping[str, str]] = ...) -> None: ...

class ResponseChunk(_message.Message):
    __slots__ = ("chunk_type", "content", "tool_name", "tool_id", "is_error")
    CHUNK_TYPE_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    TOOL_NAME_FIELD_NUMBER: _ClassVar[int]
    TOOL_ID_FIELD_NUMBER: _ClassVar[int]
    IS_ERROR_FIELD_NUMBER: _ClassVar[int]
    chunk_type: str
    content: str
    tool_name: str
    tool_id: str
    is_error: bool
    def __init__(self, chunk_type: _Optional[str] = ..., content: _Optional[str] = ..., tool_name: _Optional[str] = ..., tool_id: _Optional[str] = ..., is_error: bool = ...) -> None: ...

class LoadSkillRequest(_message.Message):
    __slots__ = ("session_id", "skill")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    SKILL_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    skill: SkillContent
    def __init__(self, session_id: _Optional[str] = ..., skill: _Optional[_Union[SkillContent, _Mapping]] = ...) -> None: ...

class SkillContent(_message.Message):
    __slots__ = ("skill_id", "name", "frontmatter_yaml", "prose")
    SKILL_ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    FRONTMATTER_YAML_FIELD_NUMBER: _ClassVar[int]
    PROSE_FIELD_NUMBER: _ClassVar[int]
    skill_id: str
    name: str
    frontmatter_yaml: str
    prose: str
    def __init__(self, skill_id: _Optional[str] = ..., name: _Optional[str] = ..., frontmatter_yaml: _Optional[str] = ..., prose: _Optional[str] = ...) -> None: ...

class LoadSkillResponse(_message.Message):
    __slots__ = ("success", "error")
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    success: bool
    error: str
    def __init__(self, success: bool = ..., error: _Optional[str] = ...) -> None: ...

class GetStateRequest(_message.Message):
    __slots__ = ("session_id",)
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    def __init__(self, session_id: _Optional[str] = ...) -> None: ...

class SessionState(_message.Message):
    __slots__ = ("session_id", "state_data", "runtime_id", "created_at", "state_format")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    STATE_DATA_FIELD_NUMBER: _ClassVar[int]
    RUNTIME_ID_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    STATE_FORMAT_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    state_data: bytes
    runtime_id: str
    created_at: str
    state_format: str
    def __init__(self, session_id: _Optional[str] = ..., state_data: _Optional[bytes] = ..., runtime_id: _Optional[str] = ..., created_at: _Optional[str] = ..., state_format: _Optional[str] = ...) -> None: ...

class ConnectMcpRequest(_message.Message):
    __slots__ = ("session_id", "servers")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    SERVERS_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    servers: _containers.RepeatedCompositeFieldContainer[McpServerConfig]
    def __init__(self, session_id: _Optional[str] = ..., servers: _Optional[_Iterable[_Union[McpServerConfig, _Mapping]]] = ...) -> None: ...

class McpServerConfig(_message.Message):
    __slots__ = ("name", "transport", "command", "args", "url", "env")
    class EnvEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    NAME_FIELD_NUMBER: _ClassVar[int]
    TRANSPORT_FIELD_NUMBER: _ClassVar[int]
    COMMAND_FIELD_NUMBER: _ClassVar[int]
    ARGS_FIELD_NUMBER: _ClassVar[int]
    URL_FIELD_NUMBER: _ClassVar[int]
    ENV_FIELD_NUMBER: _ClassVar[int]
    name: str
    transport: str
    command: str
    args: _containers.RepeatedScalarFieldContainer[str]
    url: str
    env: _containers.ScalarMap[str, str]
    def __init__(self, name: _Optional[str] = ..., transport: _Optional[str] = ..., command: _Optional[str] = ..., args: _Optional[_Iterable[str]] = ..., url: _Optional[str] = ..., env: _Optional[_Mapping[str, str]] = ...) -> None: ...

class ConnectMcpResponse(_message.Message):
    __slots__ = ("success", "connected", "failed")
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    CONNECTED_FIELD_NUMBER: _ClassVar[int]
    FAILED_FIELD_NUMBER: _ClassVar[int]
    success: bool
    connected: _containers.RepeatedScalarFieldContainer[str]
    failed: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, success: bool = ..., connected: _Optional[_Iterable[str]] = ..., failed: _Optional[_Iterable[str]] = ...) -> None: ...

class EmitTelemetryRequest(_message.Message):
    __slots__ = ("session_id",)
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    def __init__(self, session_id: _Optional[str] = ...) -> None: ...

class CapabilityManifest(_message.Message):
    __slots__ = ("runtime_id", "runtime_name", "tier", "model", "context_window", "supported_tools", "native_hooks", "native_mcp", "native_skills", "cost", "metadata", "requires_hook_bridge")
    class MetadataEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    RUNTIME_ID_FIELD_NUMBER: _ClassVar[int]
    RUNTIME_NAME_FIELD_NUMBER: _ClassVar[int]
    TIER_FIELD_NUMBER: _ClassVar[int]
    MODEL_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_WINDOW_FIELD_NUMBER: _ClassVar[int]
    SUPPORTED_TOOLS_FIELD_NUMBER: _ClassVar[int]
    NATIVE_HOOKS_FIELD_NUMBER: _ClassVar[int]
    NATIVE_MCP_FIELD_NUMBER: _ClassVar[int]
    NATIVE_SKILLS_FIELD_NUMBER: _ClassVar[int]
    COST_FIELD_NUMBER: _ClassVar[int]
    METADATA_FIELD_NUMBER: _ClassVar[int]
    REQUIRES_HOOK_BRIDGE_FIELD_NUMBER: _ClassVar[int]
    runtime_id: str
    runtime_name: str
    tier: str
    model: str
    context_window: int
    supported_tools: _containers.RepeatedScalarFieldContainer[str]
    native_hooks: bool
    native_mcp: bool
    native_skills: bool
    cost: CostEstimate
    metadata: _containers.ScalarMap[str, str]
    requires_hook_bridge: bool
    def __init__(self, runtime_id: _Optional[str] = ..., runtime_name: _Optional[str] = ..., tier: _Optional[str] = ..., model: _Optional[str] = ..., context_window: _Optional[int] = ..., supported_tools: _Optional[_Iterable[str]] = ..., native_hooks: bool = ..., native_mcp: bool = ..., native_skills: bool = ..., cost: _Optional[_Union[CostEstimate, _Mapping]] = ..., metadata: _Optional[_Mapping[str, str]] = ..., requires_hook_bridge: bool = ...) -> None: ...

class CostEstimate(_message.Message):
    __slots__ = ("input_cost_per_1k", "output_cost_per_1k")
    INPUT_COST_PER_1K_FIELD_NUMBER: _ClassVar[int]
    OUTPUT_COST_PER_1K_FIELD_NUMBER: _ClassVar[int]
    input_cost_per_1k: float
    output_cost_per_1k: float
    def __init__(self, input_cost_per_1k: _Optional[float] = ..., output_cost_per_1k: _Optional[float] = ...) -> None: ...

class TerminateRequest(_message.Message):
    __slots__ = ("session_id",)
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    def __init__(self, session_id: _Optional[str] = ...) -> None: ...

class TerminateResponse(_message.Message):
    __slots__ = ("success", "final_telemetry")
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    FINAL_TELEMETRY_FIELD_NUMBER: _ClassVar[int]
    success: bool
    final_telemetry: _common_pb2.TelemetryBatch
    def __init__(self, success: bool = ..., final_telemetry: _Optional[_Union[_common_pb2.TelemetryBatch, _Mapping]] = ...) -> None: ...

class HealthStatus(_message.Message):
    __slots__ = ("healthy", "runtime_id", "checks")
    class ChecksEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    HEALTHY_FIELD_NUMBER: _ClassVar[int]
    RUNTIME_ID_FIELD_NUMBER: _ClassVar[int]
    CHECKS_FIELD_NUMBER: _ClassVar[int]
    healthy: bool
    runtime_id: str
    checks: _containers.ScalarMap[str, str]
    def __init__(self, healthy: bool = ..., runtime_id: _Optional[str] = ..., checks: _Optional[_Mapping[str, str]] = ...) -> None: ...

class DisconnectMcpRequest(_message.Message):
    __slots__ = ("session_id", "server_name")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    SERVER_NAME_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    server_name: str
    def __init__(self, session_id: _Optional[str] = ..., server_name: _Optional[str] = ...) -> None: ...

class DisconnectMcpResponse(_message.Message):
    __slots__ = ("success",)
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    success: bool
    def __init__(self, success: bool = ...) -> None: ...

class PauseRequest(_message.Message):
    __slots__ = ("session_id",)
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    def __init__(self, session_id: _Optional[str] = ...) -> None: ...

class PauseResponse(_message.Message):
    __slots__ = ("success",)
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    success: bool
    def __init__(self, success: bool = ...) -> None: ...

class ResumeRequest(_message.Message):
    __slots__ = ("session_id",)
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    def __init__(self, session_id: _Optional[str] = ...) -> None: ...

class ResumeResponse(_message.Message):
    __slots__ = ("success", "session_id")
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    success: bool
    session_id: str
    def __init__(self, success: bool = ..., session_id: _Optional[str] = ...) -> None: ...
