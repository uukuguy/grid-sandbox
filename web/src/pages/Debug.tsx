import { useAtom } from "jotai";
import { TokenBudgetBar } from "@/components/debug/TokenBudgetBar";
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
    <div className="flex flex-1 flex-col overflow-auto">
      <div className="px-4 py-2 border-b border-border">
        <h2 className="text-sm font-medium">Debug Dashboard</h2>
        <p className="text-xs text-muted-foreground mt-0.5">
          Token budget updates after each LLM response. Tool stats update in real time.
        </p>
      </div>

      {/* Token Budget */}
      <TokenBudgetBar />

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

      {/* How to use */}
      <div className="px-4 py-3 border-t border-border mt-auto">
        <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-2">
          Usage Notes
        </h3>
        <ul className="text-xs text-muted-foreground space-y-1">
          <li>• Token bar colors: green &lt;60% → yellow &lt;80% → orange &lt;90% → red</li>
          <li>• Degradation levels: L0 None → L1 Soft Trim → L2 Hard Clear → L3 Compact</li>
          <li>• Tool details: see the <strong className="text-foreground">Tools</strong> tab</li>
        </ul>
      </div>
    </div>
  );
}
