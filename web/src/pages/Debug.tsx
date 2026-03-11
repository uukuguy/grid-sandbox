import { useAtom } from "jotai";
import { TokenBudgetBar } from "@/components/debug/TokenBudgetBar";
import { ContextStatus } from "@/components/debug/ContextStatus";
import { EventStream } from "@/components/debug/EventStream";
import { executionRecordsAtom } from "@/atoms/debug";

export default function Debug() {
  const [executions] = useAtom(executionRecordsAtom);

  const toolCounts = executions.reduce<Record<string, number>>((acc, e) => {
    acc[e.tool_name] = (acc[e.tool_name] ?? 0) + 1;
    return acc;
  }, {});

  const successCount = executions.filter((e) => e.status === "success").length;
  const failedCount = executions.filter((e) => e.status === "failed").length;

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="px-4 py-2 border-b border-border">
        <h2 className="text-sm font-medium">Debug Dashboard</h2>
        <p className="text-xs text-muted-foreground mt-0.5">
          Real-time agent event stream, token budget, and context degradation status.
        </p>
      </div>

      {/* Token Budget */}
      <TokenBudgetBar />

      {/* Context Degradation Status */}
      <ContextStatus />

      {/* Session Tool Stats */}
      <div className="px-4 py-3 border-t border-border">
        <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-2">
          Session Tool Stats
        </h3>
        {executions.length === 0 ? (
          <p className="text-xs text-muted-foreground">
            No tool calls yet — start a conversation.
          </p>
        ) : (
          <div className="space-y-1">
            <div className="flex gap-4 text-xs mb-2">
              <span className="text-green-500">{successCount} ok</span>
              {failedCount > 0 && (
                <span className="text-red-500">{failedCount} failed</span>
              )}
              <span className="text-muted-foreground">{executions.length} total</span>
            </div>
            <div className="grid grid-cols-2 gap-x-4 gap-y-0.5">
              {Object.entries(toolCounts)
                .sort((a, b) => b[1] - a[1])
                .map(([name, count]) => (
                  <div key={name} className="flex justify-between text-xs">
                    <span className="font-mono text-muted-foreground">{name}</span>
                    <span>{count}</span>
                  </div>
                ))}
            </div>
          </div>
        )}
      </div>

      {/* Live Event Stream — fills remaining space */}
      <EventStream />
    </div>
  );
}
