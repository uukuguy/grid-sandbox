import type { ToolExecutionRecord } from "@/atoms/debug";
import { TimelineView } from "./TimelineView";
import { JsonViewer } from "./JsonViewer";

interface Props {
  execution: ToolExecutionRecord | null;
  onClose: () => void;
}

// Tool-specific summary row shown at the top of the detail panel
function ToolSummary({ execution }: { execution: ToolExecutionRecord }) {
  const input = execution.input as Record<string, unknown> | null;

  if (execution.tool_name === "bash" || execution.tool_name === "run_command") {
    const cmd = input?.command ?? input?.cmd ?? input?.script;
    if (cmd) {
      return (
        <div className="mb-3 rounded bg-zinc-900 border border-border px-3 py-2">
          <span className="text-xs text-muted-foreground mr-2">$</span>
          <span className="font-mono text-sm text-green-400 break-all">
            {String(cmd)}
          </span>
        </div>
      );
    }
  }

  if (execution.tool_name === "file_read" || execution.tool_name === "read_file") {
    const path = input?.path ?? input?.file_path;
    if (path) {
      return (
        <div className="mb-3 rounded bg-zinc-900 border border-border px-3 py-2 flex items-center gap-2">
          <span className="text-xs text-muted-foreground">file</span>
          <span className="font-mono text-sm text-blue-400 break-all">{String(path)}</span>
        </div>
      );
    }
  }

  if (execution.tool_name === "file_write" || execution.tool_name === "write_file") {
    const path = input?.path ?? input?.file_path;
    if (path) {
      return (
        <div className="mb-3 rounded bg-zinc-900 border border-border px-3 py-2 flex items-center gap-2">
          <span className="text-xs text-muted-foreground">write</span>
          <span className="font-mono text-sm text-yellow-400 break-all">{String(path)}</span>
        </div>
      );
    }
  }

  if (
    execution.tool_name === "memory_store" ||
    execution.tool_name === "memory_recall" ||
    execution.tool_name === "memory_forget"
  ) {
    const content = input?.content ?? input?.query ?? input?.id;
    if (content) {
      return (
        <div className="mb-3 rounded bg-zinc-900 border border-border px-3 py-2 flex items-center gap-2">
          <span className="text-xs text-muted-foreground">{execution.tool_name}</span>
          <span className="font-mono text-sm text-purple-400 break-all line-clamp-1">
            {String(content)}
          </span>
        </div>
      );
    }
  }

  return null;
}

export function ExecutionDetail({ execution, onClose }: Props) {
  if (!execution) return null;

  const timelineEvents = [
    {
      id: `${execution.id}-start`,
      timestamp: execution.started_at,
      type: "start" as const,
    },
    ...(execution.status === "success" || execution.status === "failed"
      ? [
          {
            id: `${execution.id}-end`,
            timestamp: execution.started_at + (execution.duration_ms || 0),
            type: "end" as const,
            duration: execution.duration_ms || undefined,
          },
        ]
      : []),
    ...(execution.error
      ? [
          {
            id: `${execution.id}-error`,
            timestamp: execution.started_at,
            type: "error" as const,
          },
        ]
      : []),
  ];

  return (
    <div className="border-t border-border bg-card/50 p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-mono text-sm font-medium">{execution.tool_name}</h3>
        <button
          onClick={onClose}
          className="text-muted-foreground hover:text-foreground text-xs"
        >
          close
        </button>
      </div>

      {/* Tool-specific command/path summary */}
      <ToolSummary execution={execution} />

      <div className="space-y-3">
        {/* Timeline */}
        <TimelineView events={timelineEvents} />

        {/* Input */}
        <details open>
          <summary className="text-xs text-muted-foreground cursor-pointer">Input</summary>
          <div className="mt-1">
            <JsonViewer data={execution.input} />
          </div>
        </details>

        {/* Output */}
        {execution.output != null && (
          <details open>
            <summary className="text-xs text-muted-foreground cursor-pointer">Output</summary>
            <div className="mt-1">
              {typeof execution.output === "string" ? (
                <pre className="text-xs font-mono whitespace-pre-wrap break-all bg-secondary/30 rounded p-2 max-h-48 overflow-auto">
                  {execution.output}
                </pre>
              ) : (
                <JsonViewer data={execution.output} />
              )}
            </div>
          </details>
        )}

        {/* Error */}
        {execution.error && (
          <div className="rounded bg-red-500/10 p-2 text-xs text-red-400">
            {execution.error}
          </div>
        )}
      </div>
    </div>
  );
}
