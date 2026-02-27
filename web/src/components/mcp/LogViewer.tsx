import { useState } from "react";

type LogLevel = "all" | "info" | "debug" | "warn" | "error" | "raw";

interface LogEntry {
  id: string;
  level: LogLevel;
  direction: "request" | "response" | "system";
  method?: string;
  message: string;
  timestamp: string;
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

export function LogViewer() {
  const [level, setLevel] = useState<LogLevel>("all");
  const [logs, setLogs] = useState<LogEntry[]>(mockLogs);

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
        return "→";
      case "response":
        return "←";
      default:
        return "•";
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex justify-between items-center mb-4">
        <div className="flex gap-2">
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
        </div>
        <div className="flex gap-2">
          <button
            className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 rounded"
            onClick={() => setLogs([])}
          >
            清空
          </button>
          <button className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 rounded">
            导出
          </button>
        </div>
      </div>

      {/* Log List */}
      <div className="flex-1 overflow-auto bg-gray-900 rounded border border-gray-700">
        {filteredLogs.length === 0 ? (
          <div className="p-4 text-gray-400">暂无日志</div>
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
