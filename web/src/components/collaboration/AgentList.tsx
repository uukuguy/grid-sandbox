import { useAtom } from "jotai";
import { collaborationAgentsAtom } from "@/atoms/collaboration";
import { cn } from "@/lib/utils";

export function AgentList() {
  const [agents] = useAtom(collaborationAgentsAtom);

  if (agents.length === 0) {
    return (
      <p className="text-xs text-muted-foreground py-4 text-center">
        No agents in collaboration session.
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {agents.map((agent) => (
        <div
          key={agent.id}
          className="rounded border border-border p-3 bg-card hover:bg-secondary/30 transition-colors"
        >
          <div className="flex items-center justify-between mb-1">
            <span className="text-sm font-medium">{agent.name}</span>
            <span className="text-xs font-mono text-muted-foreground">{agent.id}</span>
          </div>
          <div className="text-xs text-muted-foreground mb-1">
            Session: <span className="font-mono">{agent.session_id}</span>
          </div>
          {agent.capabilities.length > 0 && (
            <div className="flex flex-wrap gap-1 mt-1">
              {agent.capabilities.map((cap) => (
                <span
                  key={cap}
                  className={cn(
                    "inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium",
                    "bg-blue-500/10 text-blue-400 border border-blue-500/20",
                  )}
                >
                  {cap}
                </span>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
