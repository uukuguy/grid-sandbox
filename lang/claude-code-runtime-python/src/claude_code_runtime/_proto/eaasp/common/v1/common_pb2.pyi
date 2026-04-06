from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class HookDecision(_message.Message):
    __slots__ = ("decision", "reason", "modified_input")
    DECISION_FIELD_NUMBER: _ClassVar[int]
    REASON_FIELD_NUMBER: _ClassVar[int]
    MODIFIED_INPUT_FIELD_NUMBER: _ClassVar[int]
    decision: str
    reason: str
    modified_input: str
    def __init__(self, decision: _Optional[str] = ..., reason: _Optional[str] = ..., modified_input: _Optional[str] = ...) -> None: ...

class StopDecision(_message.Message):
    __slots__ = ("decision", "feedback")
    DECISION_FIELD_NUMBER: _ClassVar[int]
    FEEDBACK_FIELD_NUMBER: _ClassVar[int]
    decision: str
    feedback: str
    def __init__(self, decision: _Optional[str] = ..., feedback: _Optional[str] = ...) -> None: ...

class ToolCallEvent(_message.Message):
    __slots__ = ("session_id", "tool_name", "tool_id", "input_json")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    TOOL_NAME_FIELD_NUMBER: _ClassVar[int]
    TOOL_ID_FIELD_NUMBER: _ClassVar[int]
    INPUT_JSON_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    tool_name: str
    tool_id: str
    input_json: str
    def __init__(self, session_id: _Optional[str] = ..., tool_name: _Optional[str] = ..., tool_id: _Optional[str] = ..., input_json: _Optional[str] = ...) -> None: ...

class ToolResultEvent(_message.Message):
    __slots__ = ("session_id", "tool_name", "tool_id", "output", "is_error")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    TOOL_NAME_FIELD_NUMBER: _ClassVar[int]
    TOOL_ID_FIELD_NUMBER: _ClassVar[int]
    OUTPUT_FIELD_NUMBER: _ClassVar[int]
    IS_ERROR_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    tool_name: str
    tool_id: str
    output: str
    is_error: bool
    def __init__(self, session_id: _Optional[str] = ..., tool_name: _Optional[str] = ..., tool_id: _Optional[str] = ..., output: _Optional[str] = ..., is_error: bool = ...) -> None: ...

class StopRequest(_message.Message):
    __slots__ = ("session_id",)
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    def __init__(self, session_id: _Optional[str] = ...) -> None: ...

class TelemetryEvent(_message.Message):
    __slots__ = ("session_id", "runtime_id", "user_id", "event_type", "timestamp", "payload_json", "resource_usage")
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    RUNTIME_ID_FIELD_NUMBER: _ClassVar[int]
    USER_ID_FIELD_NUMBER: _ClassVar[int]
    EVENT_TYPE_FIELD_NUMBER: _ClassVar[int]
    TIMESTAMP_FIELD_NUMBER: _ClassVar[int]
    PAYLOAD_JSON_FIELD_NUMBER: _ClassVar[int]
    RESOURCE_USAGE_FIELD_NUMBER: _ClassVar[int]
    session_id: str
    runtime_id: str
    user_id: str
    event_type: str
    timestamp: str
    payload_json: str
    resource_usage: ResourceUsage
    def __init__(self, session_id: _Optional[str] = ..., runtime_id: _Optional[str] = ..., user_id: _Optional[str] = ..., event_type: _Optional[str] = ..., timestamp: _Optional[str] = ..., payload_json: _Optional[str] = ..., resource_usage: _Optional[_Union[ResourceUsage, _Mapping]] = ...) -> None: ...

class TelemetryBatch(_message.Message):
    __slots__ = ("events",)
    EVENTS_FIELD_NUMBER: _ClassVar[int]
    events: _containers.RepeatedCompositeFieldContainer[TelemetryEvent]
    def __init__(self, events: _Optional[_Iterable[_Union[TelemetryEvent, _Mapping]]] = ...) -> None: ...

class ResourceUsage(_message.Message):
    __slots__ = ("input_tokens", "output_tokens", "compute_ms")
    INPUT_TOKENS_FIELD_NUMBER: _ClassVar[int]
    OUTPUT_TOKENS_FIELD_NUMBER: _ClassVar[int]
    COMPUTE_MS_FIELD_NUMBER: _ClassVar[int]
    input_tokens: int
    output_tokens: int
    compute_ms: int
    def __init__(self, input_tokens: _Optional[int] = ..., output_tokens: _Optional[int] = ..., compute_ms: _Optional[int] = ...) -> None: ...

class Empty(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...
