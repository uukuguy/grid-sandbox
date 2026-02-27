import { useState } from "react";
import { ServerList } from "../components/mcp/ServerList";
import { ToolInvoker } from "../components/mcp/ToolInvoker";
import { LogViewer } from "../components/mcp/LogViewer";

type Tab = "servers" | "invoker" | "logs";

export default function McpWorkbench() {
  const [activeTab, setActiveTab] = useState<Tab>("servers");

  return (
    <div className="h-full flex flex-col">
      {/* Tab Navigation */}
      <div className="flex border-b border-gray-700">
        <button
          className={`px-4 py-2 text-sm font-medium ${
            activeTab === "servers"
              ? "text-blue-400 border-b-2 border-blue-400"
              : "text-gray-400 hover:text-gray-200"
          }`}
          onClick={() => setActiveTab("servers")}
        >
          Servers
        </button>
        <button
          className={`px-4 py-2 text-sm font-medium ${
            activeTab === "invoker"
              ? "text-blue-400 border-b-2 border-blue-400"
              : "text-gray-400 hover:text-gray-200"
          }`}
          onClick={() => setActiveTab("invoker")}
        >
          Tool Invoker
        </button>
        <button
          className={`px-4 py-2 text-sm font-medium ${
            activeTab === "logs"
              ? "text-blue-400 border-b-2 border-blue-400"
              : "text-gray-400 hover:text-gray-200"
          }`}
          onClick={() => setActiveTab("logs")}
        >
          Logs
        </button>
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-auto p-4">
        {activeTab === "servers" && <ServerList />}
        {activeTab === "invoker" && <ToolInvoker />}
        {activeTab === "logs" && <LogViewer />}
      </div>
    </div>
  );
}
