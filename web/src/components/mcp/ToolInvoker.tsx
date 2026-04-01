import { useState, useEffect, useMemo } from "react";

interface Tool {
  name: string;
  description?: string;
  input_schema: Record<string, unknown>;
}

interface Server {
  id: string;
  name: string;
  tools: Tool[];
}

const mockServers: Server[] = [
  {
    id: "1",
    name: "filesystem",
    tools: [
      { name: "read_file", description: "Read a file", input_schema: { type: "object", properties: { path: { type: "string" } }, required: ["path"] } },
      { name: "write_file", description: "Write a file", input_schema: { type: "object", properties: { path: { type: "string" }, content: { type: "string" } }, required: ["path", "content"] } },
      { name: "list_directory", description: "List directory", input_schema: { type: "object", properties: { path: { type: "string" } }, required: ["path"] } },
    ],
  },
  {
    id: "2",
    name: "memory",
    tools: [
      { name: "memory_search", description: "Search memories", input_schema: { type: "object", properties: { query: { type: "string" } }, required: ["query"] } },
      { name: "memory_read", description: "Read memory", input_schema: { type: "object", properties: { id: { type: "string" } }, required: ["id"] } },
    ],
  },
];

/** Validate a JSON string, returning null if valid or an error message. */
function validateJson(text: string): string | null {
  try {
    JSON.parse(text);
    return null;
  } catch (e) {
    return e instanceof Error ? e.message : "Invalid JSON";
  }
}

/** Render a JSON Schema's properties as a readable hint. */
function formatSchemaHint(schema: Record<string, unknown>): string | null {
  const props = schema.properties as Record<string, Record<string, unknown>> | undefined;
  if (!props) return null;
  const required = (schema.required as string[]) || [];
  const lines = Object.entries(props).map(([key, val]) => {
    const type = val.type ?? "any";
    const req = required.includes(key) ? " (required)" : "";
    const desc = val.description ? ` — ${val.description}` : "";
    return `  "${key}": ${type}${req}${desc}`;
  });
  return `{\n${lines.join(",\n")}\n}`;
}

export function ToolInvoker() {
  const [servers, setServers] = useState<Server[]>([]);
  const [selectedServer, setSelectedServer] = useState<string>("");
  const [selectedTool, setSelectedTool] = useState<string>("");
  const [serverTools, setServerTools] = useState<Tool[]>([]);
  const [toolsLoading, setToolsLoading] = useState(false);
  const [params, setParams] = useState<string>("{\n  \n}");
  const [result, setResult] = useState<string>("");
  const [resultDurationMs, setResultDurationMs] = useState<number | null>(null);
  const [resultIsError, setResultIsError] = useState(false);
  const [loading, setLoading] = useState(false);
  const [showRawResponse, setShowRawResponse] = useState(false);

  useEffect(() => {
    fetch("/api/v1/mcp/servers")
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data) && data.length > 0) {
          setServers(data);
        } else {
          setServers(mockServers);
        }
      })
      .catch(() => {
        setServers(mockServers);
      });
  }, []);

  // Fetch tools when server changes
  useEffect(() => {
    if (!selectedServer) {
      setServerTools([]);
      return;
    }

    setToolsLoading(true);
    fetch(`/api/v1/mcp/servers/${selectedServer}/tools`)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setServerTools(data);
        } else {
          // Fallback to mock
          const mock = mockServers.find((s) => s.id === selectedServer);
          setServerTools(mock?.tools || []);
        }
      })
      .catch(() => {
        const mock = mockServers.find((s) => s.id === selectedServer);
        setServerTools(mock?.tools || []);
      })
      .finally(() => setToolsLoading(false));
  }, [selectedServer]);

  const tool = serverTools.find((t) => t.name === selectedTool);
  const jsonError = useMemo(() => validateJson(params), [params]);
  const schemaHint = useMemo(
    () => (tool ? formatSchemaHint(tool.input_schema) : null),
    [tool],
  );

  const handleServerChange = (serverId: string) => {
    setSelectedServer(serverId);
    setSelectedTool("");
    setResult("");
    setResultDurationMs(null);
    setResultIsError(false);
  };

  const handleToolChange = (toolName: string) => {
    setSelectedTool(toolName);
    setResult("");
    setResultDurationMs(null);
    setResultIsError(false);
    // Generate skeleton params from schema
    const selected = serverTools.find((t) => t.name === toolName);
    if (selected?.input_schema) {
      const props = selected.input_schema.properties as Record<string, Record<string, unknown>> | undefined;
      if (props) {
        const skeleton: Record<string, string> = {};
        for (const key of Object.keys(props)) {
          skeleton[key] = "";
        }
        setParams(JSON.stringify(skeleton, null, 2));
        return;
      }
    }
    setParams("{\n  \n}");
  };

  const handleExecute = async () => {
    if (!selectedServer || !selectedTool) return;
    if (jsonError) return;

    setLoading(true);
    setResultIsError(false);
    setResultDurationMs(null);
    const startTime = performance.now();

    try {
      const response = await fetch(`/api/v1/mcp/servers/${selectedServer}/call`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tool_name: selectedTool,
          arguments: JSON.parse(params),
        }),
      });
      const elapsed = Math.round(performance.now() - startTime);
      setResultDurationMs(elapsed);

      const data = await response.json();
      if (!response.ok) {
        setResultIsError(true);
      }
      setResult(JSON.stringify(data, null, 2));
    } catch (err) {
      const elapsed = Math.round(performance.now() - startTime);
      setResultDurationMs(elapsed);
      setResultIsError(true);
      setResult(JSON.stringify({ error: "Failed to call tool", details: String(err) }, null, 2));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* Server & Tool Selection */}
      <div className="flex gap-4 mb-4">
        <div className="flex-1">
          <label className="block text-sm text-gray-400 mb-1">Server</label>
          <select
            className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2"
            value={selectedServer}
            onChange={(e) => handleServerChange(e.target.value)}
          >
            <option value="">Select server...</option>
            {servers.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </div>
        <div className="flex-1">
          <label className="block text-sm text-gray-400 mb-1">Tool</label>
          <select
            className="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2"
            value={selectedTool}
            onChange={(e) => handleToolChange(e.target.value)}
            disabled={!selectedServer || toolsLoading}
          >
            <option value="">{toolsLoading ? "Loading..." : "Select tool..."}</option>
            {serverTools.map((t) => (
              <option key={t.name} value={t.name}>
                {t.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Tool description & schema hint */}
      {tool && (
        <div className="mb-3">
          {tool.description && (
            <p className="text-sm text-gray-400 mb-1">{tool.description}</p>
          )}
          {schemaHint && (
            <details className="text-xs text-gray-500 mb-2">
              <summary className="cursor-pointer hover:text-gray-400">
                Input schema
              </summary>
              <pre className="mt-1 bg-gray-800 rounded p-2 font-mono whitespace-pre overflow-x-auto">
                {schemaHint}
              </pre>
            </details>
          )}
        </div>
      )}

      {/* Parameters */}
      {tool && (
        <div className="mb-4">
          <label className="block text-sm text-gray-400 mb-1">
            Parameters (JSON)
          </label>
          <textarea
            className={`w-full h-36 bg-gray-800 rounded px-3 py-2 font-mono text-sm border ${
              jsonError
                ? "border-red-500 focus:ring-red-500"
                : "border-gray-700 focus:ring-blue-500"
            } focus:outline-none focus:ring-1`}
            value={params}
            onChange={(e) => setParams(e.target.value)}
            spellCheck={false}
          />
          {jsonError && (
            <p className="text-xs text-red-400 mt-1">{jsonError}</p>
          )}
        </div>
      )}

      {/* Execute Button */}
      <div className="mb-4">
        <button
          className="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded font-medium disabled:opacity-50"
          onClick={handleExecute}
          disabled={!selectedServer || !selectedTool || loading || !!jsonError}
        >
          {loading ? "Executing..." : "Execute"}
        </button>
      </div>

      {/* Result */}
      {result && (
        <div className="flex-1 flex flex-col min-h-0">
          <div className="flex items-center justify-between mb-1">
            <label className="text-sm text-gray-400">Result</label>
            <div className="flex items-center gap-3 text-xs text-gray-500">
              {resultDurationMs !== null && (
                <span>{resultDurationMs}ms</span>
              )}
              <button
                className="hover:text-gray-300 underline"
                onClick={() => setShowRawResponse((v) => !v)}
              >
                {showRawResponse ? "Hide raw" : "Show raw"}
              </button>
            </div>
          </div>

          {/* Formatted result */}
          <pre
            className={`bg-gray-900 rounded p-4 text-sm overflow-auto flex-1 border ${
              resultIsError ? "border-red-600" : "border-gray-700"
            }`}
          >
            {result}
          </pre>

          {/* Collapsible raw response */}
          {showRawResponse && (
            <div className="mt-2">
              <label className="block text-xs text-gray-500 mb-1">
                Raw JSON response
              </label>
              <pre className="bg-gray-950 border border-gray-800 rounded p-3 text-xs font-mono overflow-auto max-h-48 text-gray-400">
                {result}
              </pre>
            </div>
          )}

          {/* Error message banner */}
          {resultIsError && (
            <div className="mt-2 px-3 py-2 bg-red-900/30 border border-red-700 rounded text-sm text-red-300">
              Tool invocation failed. Check the response above for details.
            </div>
          )}
        </div>
      )}
    </div>
  );
}
