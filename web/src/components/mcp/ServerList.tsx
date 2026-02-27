import { useState, useEffect } from "react";

interface McpServer {
  id: string;
  name: string;
  source: string;
  command: string;
  args: string[];
  enabled: boolean;
  runtime_status: string;
  tool_count: number;
}

const mockServers: McpServer[] = [
  {
    id: "1",
    name: "filesystem",
    source: "template",
    command: "npx",
    args: ["-y", "@anthropic/mcp-server-filesystem", "/tmp"],
    enabled: true,
    runtime_status: "running",
    tool_count: 5,
  },
  {
    id: "2",
    name: "memory",
    source: "template",
    command: "npx",
    args: ["-y", "@anthropic/mcp-server-memory"],
    enabled: true,
    runtime_status: "stopped",
    tool_count: 0,
  },
];

export function ServerList() {
  const [servers, setServers] = useState<McpServer[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch("/api/mcp/servers")
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data) && data.length > 0) {
          setServers(data);
        } else {
          // Fallback to mock data if no servers
          setServers(mockServers);
        }
        setLoading(false);
      })
      .catch(() => {
        // Fallback to mock data on error
        setServers(mockServers);
        setLoading(false);
      });
  }, []);

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "running":
        return "🟢";
      case "stopped":
        return "⚪";
      case "error":
        return "🔴";
      case "starting":
        return "⏳";
      default:
        return "⚪";
    }
  };

  const getStatusText = (status: string) => {
    switch (status) {
      case "running":
        return "运行中";
      case "stopped":
        return "已停止";
      case "error":
        return "错误";
      case "starting":
        return "启动中";
      default:
        return status;
    }
  };

  if (loading) {
    return <div className="text-gray-400">加载中...</div>;
  }

  return (
    <div>
      <div className="flex justify-between items-center mb-4">
        <h2 className="text-lg font-medium">MCP Servers</h2>
        <div className="flex gap-2">
          <button className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 rounded">
            扫描
          </button>
          <button className="px-3 py-1 text-sm bg-blue-600 hover:bg-blue-500 rounded">
            添加
          </button>
        </div>
      </div>

      <div className="space-y-2">
        {servers.map((server) => (
          <div
            key={server.id}
            className="bg-gray-800 rounded-lg p-4 flex items-center justify-between"
          >
            <div className="flex items-center gap-3">
              <span className="text-xl">{getStatusIcon(server.runtime_status)}</span>
              <div>
                <div className="font-medium">{server.name}</div>
                <div className="text-sm text-gray-400">
                  {server.command} {server.args.join(" ")}
                </div>
                <div className="text-xs text-gray-500">
                  {getStatusText(server.runtime_status)}
                </div>
              </div>
            </div>
            <div className="flex items-center gap-4">
              <span className="text-sm text-gray-400">
                {server.tool_count} tools
              </span>
              <button className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 rounded">
                {server.runtime_status === "running" ? "停止" : "启动"}
              </button>
              <button className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 rounded">
                调用
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
