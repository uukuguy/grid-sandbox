import { useMemo } from "react";
import { useAtom } from "jotai";
import { executionRecordsAtom, selectedExecutionIdAtom } from "@/atoms/debug";
import { ExecutionDetail } from "./ExecutionDetail";

// Extract a one-line human-readable summary of what the tool did
function toolSummary(toolName: string, input: unknown): string {
  const inp = input as Record<string, unknown> | null;
  if (!inp) return "";

  if (toolName === "bash" || toolName === "run_command") {
    const cmd = inp.command ?? inp.cmd ?? inp.script;
    if (cmd) return String(cmd).slice(0, 80);
  }
  if (toolName === "file_read" || toolName === "read_file") {
    const p = inp.path ?? inp.file_path;
    if (p) return String(p);
  }
  if (toolName === "file_write" || toolName === "write_file") {
    const p = inp.path ?? inp.file_path;
    if (p) return String(p);
  }
  if (toolName === "memory_store") {
    const c = inp.content;
    if (c) return String(c).slice(0, 60);
  }
  if (toolName === "memory_recall") {
    const q = inp.query;
    if (q) return `query: ${String(q).slice(0, 60)}`;
  }
  if (toolName === "memory_forget") {
    const id = inp.id ?? inp.memory_id;
    if (id) return `id: ${String(id)}`;
  }
  // Generic: first string value of input
  const firstVal = Object.values(inp).find((v) => typeof v === "string");
  if (firstVal) return String(firstVal).slice(0, 60);
  return "";
}

export function ExecutionList() {
  const [executions] = useAtom(executionRecordsAtom);
  const [selectedId, setSelectedId] = useAtom(selectedExecutionIdAtom);

  // Memoize tool summaries to avoid recomputing on every render
  const summaries = useMemo(
    () => new Map(executions.map((exec) => [exec.id, toolSummary(exec.tool_name, exec.input)])),
    [executions]
  );

  if (executions.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center text-muted-foreground text-sm">
        No tool executions yet. Start a conversation to see tool calls here.
      </div>
    );
  }

  return (
    <div className="flex flex-col overflow-auto">
      <table className="w-full text-sm">
        <thead className="sticky top-0 bg-card border-b border-border">
          <tr className="text-left text-muted-foreground">
            <th className="px-3 py-2 font-medium w-28">Tool</th>
            <th className="px-3 py-2 font-medium">Command / Args</th>
            <th className="px-3 py-2 font-medium w-16">Status</th>
            <th className="px-3 py-2 font-medium w-16">Duration</th>
            <th className="px-3 py-2 font-medium w-20">Time</th>
          </tr>
        </thead>
        <tbody>
          {executions.map((exec) => {
            const summary = summaries.get(exec.id) ?? "";
            return (
              <tr
                key={exec.id}
                onClick={() => setSelectedId(selectedId === exec.id ? null : exec.id)}
                className="border-b border-border/50 cursor-pointer hover:bg-secondary/30"
              >
                <td className="px-3 py-2 font-mono text-xs">{exec.tool_name}</td>
                <td className="px-3 py-2 font-mono text-xs text-muted-foreground truncate max-w-xs">
                  {summary || <span className="italic opacity-50">—</span>}
                </td>
                <td className="px-3 py-2">
                  <StatusBadge status={exec.status} />
                </td>
                <td className="px-3 py-2 text-muted-foreground text-xs">
                  {exec.duration_ms != null ? `${(exec.duration_ms / 1000).toFixed(1)}s` : "—"}
                </td>
                <td className="px-3 py-2 text-muted-foreground text-xs">
                  {new Date(exec.started_at).toLocaleTimeString()}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      {selectedId && (
        <ExecutionDetail
          execution={executions.find((e) => e.id === selectedId) ?? null}
          onClose={() => setSelectedId(null)}
        />
      )}
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    running: "text-yellow-500",
    success: "text-green-500",
    failed: "text-red-500",
    timeout: "text-orange-500",
  };
  const icons: Record<string, string> = {
    running: "...",
    success: "ok",
    failed: "err",
    timeout: "t/o",
  };
  return (
    <span className={`font-mono text-xs ${styles[status] ?? ""}`}>
      {icons[status] ?? status}
    </span>
  );
}
