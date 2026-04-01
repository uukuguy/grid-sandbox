import { useAtomValue } from "jotai";
import { isStreamingAtom, streamingTextAtom, streamingThinkingAtom, toolExecutionsAtom } from "@/atoms/session";
import type { ToolExecution } from "@/atoms/session";
import { Loader2, Terminal, FileText, Brain, CheckCircle2, XCircle } from "lucide-react";

function ToolExecItem({ tool }: { tool: ToolExecution }) {
  const isComplete = tool.status === "complete";
  const icon =
    tool.toolName === "bash" ? (
      <Terminal className="h-3 w-3 shrink-0" />
    ) : (
      <FileText className="h-3 w-3 shrink-0" />
    );

  const statusBadge = isComplete ? (
    tool.success ? (
      <span className="inline-flex items-center gap-0.5 rounded-full bg-green-500/15 px-1.5 py-0.5 text-[10px] font-medium text-green-600">
        <CheckCircle2 className="h-2.5 w-2.5" /> done
      </span>
    ) : (
      <span className="inline-flex items-center gap-0.5 rounded-full bg-red-500/15 px-1.5 py-0.5 text-[10px] font-medium text-red-600">
        <XCircle className="h-2.5 w-2.5" /> failed
      </span>
    )
  ) : (
    <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
  );

  const hasDetails =
    Object.keys(tool.input).length > 0 || (isComplete && tool.output);

  if (!hasDetails) {
    return (
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        {icon}
        <span className="font-mono">{tool.toolName}</span>
        {statusBadge}
      </div>
    );
  }

  return (
    <details className="rounded-md border border-border/50 text-xs text-muted-foreground">
      <summary className="flex cursor-pointer items-center gap-2 px-2 py-1.5 hover:bg-muted/30">
        {icon}
        <span className="font-mono">{tool.toolName}</span>
        {statusBadge}
      </summary>
      <div className="space-y-1.5 border-t border-border/30 px-2 py-2">
        {Object.keys(tool.input).length > 0 && (
          <div>
            <div className="mb-0.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground/60">
              Input
            </div>
            <pre className="max-h-40 overflow-auto rounded bg-muted/40 p-2 font-mono text-[11px] leading-relaxed">
              {JSON.stringify(tool.input, null, 2)}
            </pre>
          </div>
        )}
        {isComplete && tool.output && (
          <div>
            <div className="mb-0.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground/60">
              Output
            </div>
            <pre className="max-h-60 overflow-auto rounded bg-muted/40 p-2 font-mono text-[11px] leading-relaxed">
              {tool.output}
            </pre>
          </div>
        )}
      </div>
    </details>
  );
}

export function StreamingDisplay() {
  const isStreaming = useAtomValue(isStreamingAtom);
  const streamingText = useAtomValue(streamingTextAtom);
  const streamingThinking = useAtomValue(streamingThinkingAtom);
  const toolExecs = useAtomValue(toolExecutionsAtom);

  if (!isStreaming) return null;

  return (
    <div className="border-t border-border px-4 py-3">
      {streamingThinking && (
        <details className="mb-2">
          <summary className="flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground/70">
            <Brain className="h-3 w-3" />
            <span>Thinking... ({streamingThinking.length} chars)</span>
          </summary>
          <div className="mt-1 max-h-40 overflow-y-auto rounded-md bg-muted/50 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground/70 whitespace-pre-wrap font-mono">
            {streamingThinking}
            <span className="ml-0.5 inline-block h-3 w-1 animate-pulse bg-muted-foreground" />
          </div>
        </details>
      )}
      {toolExecs.length > 0 && (
        <div className="mb-2 space-y-1.5">
          {toolExecs.map((tool) => (
            <ToolExecItem key={tool.toolId} tool={tool} />
          ))}
        </div>
      )}
      {streamingText && (
        <div className="max-w-[80%] rounded-lg bg-secondary px-4 py-2 text-sm whitespace-pre-wrap text-foreground">
          {streamingText}
          <span className="ml-0.5 inline-block h-4 w-1.5 animate-pulse bg-foreground" />
        </div>
      )}
    </div>
  );
}
