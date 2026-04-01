import { useState, useEffect, useRef, useCallback } from "react";

type LogLevel = "all" | "info" | "debug" | "warn" | "error" | "raw";

interface LogEntry {
  id: string;
  level: LogLevel;
  direction: "request" | "response" | "system";
  method?: string;
  message: string;
  timestamp: string;
}

/** Shape of a TelemetryEvent from the SSE endpoint */
interface TelemetryEvent {
  // Rust enum is tagged by the variant key structure
  LoopTurnStarted?: { session_id: string; turn: number };
  ToolCallStarted?: { session_id: string; tool_name: string };
  ToolCallCompleted?: { session_id: string; tool_name: string; duration_ms: number };
  ContextDegraded?: { session_id: string; level: string };
  LoopGuardTriggered?: { session_id: string; reason: string };
  TokenBudgetUpdated?: { session_id: string; used: number; total: number; ratio: number };
  // serde default tagging — externally tagged enum means the JSON key is the variant name
}

const mockLogs: LogEntry[] = [
  {
    id: "1",
    level: "info",
    direction: "request",
    method: "tools/call",
    message: '{"name": "read_file", "arguments": {"path": "/tmp/test.txt"}}',
    timestamp: "12:30:15",
  },
  {
    id: "2",
    level: "debug",
    direction: "response",
    method: "tools/call",
    message: '{"content": [{"type": "text", "text": "Hello world"}]}',
    timestamp: "12:30:15",
  },
  {
    id: "3",
    level: "error",
    direction: "response",
    message: '{"code": -32602, "message": "File not found"}',
    timestamp: "12:30:20",
  },
];

let nextLogId = 100;

function telemetryToLogEntry(event: TelemetryEvent): LogEntry | null {
  const now = new Date();
  const ts = now.toLocaleTimeString("en-US", { hour12: false });
  const id = String(nextLogId++);

  if (event.ToolCallStarted) {
    return {
      id,
      level: "info",
      direction: "request",
      method: "tools/call",
      message: `Tool started: ${event.ToolCallStarted.tool_name}`,
      timestamp: ts,
    };
  }
  if (event.ToolCallCompleted) {
    return {
      id,
      level: "info",
      direction: "response",
      method: "tools/call",
      message: `Tool completed: ${event.ToolCallCompleted.tool_name} (${event.ToolCallCompleted.duration_ms}ms)`,
      timestamp: ts,
    };
  }
  if (event.LoopTurnStarted) {
    return {
      id,
      level: "debug",
      direction: "system",
      message: `Loop turn ${event.LoopTurnStarted.turn} started`,
      timestamp: ts,
    };
  }
  if (event.ContextDegraded) {
    return {
      id,
      level: "warn",
      direction: "system",
      message: `Context degraded: ${event.ContextDegraded.level}`,
      timestamp: ts,
    };
  }
  if (event.LoopGuardTriggered) {
    return {
      id,
      level: "error",
      direction: "system",
      message: `Loop guard triggered: ${event.LoopGuardTriggered.reason}`,
      timestamp: ts,
    };
  }
  if (event.TokenBudgetUpdated) {
    const { used, total, ratio } = event.TokenBudgetUpdated;
    return {
      id,
      level: "debug",
      direction: "system",
      message: `Token budget: ${used}/${total} (${(ratio * 100).toFixed(1)}%)`,
      timestamp: ts,
    };
  }
  return null;
}

export function LogViewer() {
  const [level, setLevel] = useState<LogLevel>("all");
  const [logs, setLogs] = useState<LogEntry[]>(mockLogs);
  const [live, setLive] = useState(false);
  const [sseConnected, setSseConnected] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);
  const logContainerRef = useRef<HTMLDivElement>(null);
  const autoScrollRef = useRef(true);

  // Auto-scroll to bottom when new logs arrive and live mode is on
  useEffect(() => {
    if (live && autoScrollRef.current && logContainerRef.current) {
      const el = logContainerRef.current;
      el.scrollTop = el.scrollHeight;
    }
  }, [logs, live]);

  // Handle scroll — disable auto-scroll when user scrolls up
  const handleScroll = useCallback(() => {
    if (!logContainerRef.current) return;
    const el = logContainerRef.current;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    autoScrollRef.current = atBottom;
  }, []);

  // SSE connection lifecycle
  useEffect(() => {
    if (!live) {
      // Disconnect when live mode is turned off
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
        eventSourceRef.current = null;
        setSseConnected(false);
      }
      return;
    }

    // Connect to SSE endpoint
    const es = new EventSource("/api/v1/events/stream");
    eventSourceRef.current = es;

    es.addEventListener("telemetry", (e: MessageEvent) => {
      try {
        const event: TelemetryEvent = JSON.parse(e.data);
        const entry = telemetryToLogEntry(event);
        if (entry) {
          setLogs((prev) => [...prev, entry]);
        }
      } catch {
        // Ignore unparseable events
      }
    });

    es.addEventListener("error", (e: MessageEvent) => {
      try {
        const data = JSON.parse(e.data);
        if (data.warning) {
          setLogs((prev) => [
            ...prev,
            {
              id: String(nextLogId++),
              level: "warn",
              direction: "system",
              message: data.warning,
              timestamp: new Date().toLocaleTimeString("en-US", { hour12: false }),
            },
          ]);
        }
      } catch {
        // SSE connection error (not a data event)
      }
    });

    es.onopen = () => setSseConnected(true);
    es.onerror = () => {
      setSseConnected(false);
      // EventSource auto-reconnects, so we just update UI state
    };

    return () => {
      es.close();
      eventSourceRef.current = null;
      setSseConnected(false);
    };
  }, [live]);

  const filteredLogs = level === "all" ? logs : logs.filter((l) => l.level === level);

  const getLevelColor = (l: LogLevel) => {
    switch (l) {
      case "info":
        return "text-blue-400";
      case "debug":
        return "text-green-400";
      case "warn":
        return "text-yellow-400";
      case "error":
        return "text-red-400";
      case "raw":
        return "text-gray-400";
      default:
        return "text-gray-400";
    }
  };

  const getDirectionIcon = (d: string) => {
    switch (d) {
      case "request":
        return "\u2192";
      case "response":
        return "\u2190";
      default:
        return "\u2022";
    }
  };

  const handleExport = () => {
    const text = logs
      .map((l) => `${l.timestamp} [${l.level.toUpperCase()}] ${getDirectionIcon(l.direction)} ${l.method ? l.method + " " : ""}${l.message}`)
      .join("\n");
    const blob = new Blob([text], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `mcp-logs-${new Date().toISOString().slice(0, 19)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex justify-between items-center mb-4">
        <div className="flex gap-2 items-center">
          {(["all", "info", "debug", "warn", "error", "raw"] as LogLevel[]).map((l) => (
            <button
              key={l}
              className={`px-3 py-1 text-sm rounded ${
                level === l ? "bg-blue-600" : "bg-gray-700 hover:bg-gray-600"
              }`}
              onClick={() => setLevel(l)}
            >
              {l.toUpperCase()}
            </button>
          ))}
          <span className="mx-2 text-gray-600">|</span>
          {/* Live toggle */}
          <button
            className={`px-3 py-1 text-sm rounded flex items-center gap-1.5 ${
              live
                ? "bg-green-700 hover:bg-green-600 text-green-100"
                : "bg-gray-700 hover:bg-gray-600 text-gray-300"
            }`}
            onClick={() => setLive((v) => !v)}
            title={live ? "Stop live streaming" : "Start live streaming from SSE"}
          >
            <span
              className={`inline-block w-2 h-2 rounded-full ${
                live && sseConnected
                  ? "bg-green-400 animate-pulse"
                  : live
                    ? "bg-yellow-400"
                    : "bg-gray-500"
              }`}
            />
            LIVE
          </button>
        </div>
        <div className="flex gap-2">
          <button
            className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 rounded"
            onClick={() => setLogs([])}
          >
            Clear
          </button>
          <button
            className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 rounded"
            onClick={handleExport}
          >
            Export
          </button>
        </div>
      </div>

      {/* Log List */}
      <div
        ref={logContainerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-auto bg-gray-900 rounded border border-gray-700"
      >
        {filteredLogs.length === 0 ? (
          <div className="p-4 text-gray-400">
            {live ? "Waiting for events..." : "No logs"}
          </div>
        ) : (
          filteredLogs.map((log) => (
            <div key={log.id} className="border-b border-gray-800 p-2 font-mono text-sm">
              <span className="text-gray-500">{log.timestamp}</span>
              <span className={`mx-2 ${getLevelColor(log.level)}`}>
                {log.level.toUpperCase()}
              </span>
              <span className="text-gray-400 mx-1">{getDirectionIcon(log.direction)}</span>
              {log.method && <span className="text-blue-300 mr-2">{log.method}</span>}
              <span className="text-gray-300">{log.message}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
