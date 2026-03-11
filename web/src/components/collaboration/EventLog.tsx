import { useAtom } from "jotai";
import { collaborationEventsAtom } from "@/atoms/collaboration";

function eventSummary(event: Record<string, unknown>): string {
  // CollaborationEvent is a tagged enum; the key determines the variant
  if ("AgentJoined" in event) return `Agent joined: ${(event as { AgentJoined: { agent_id: string } }).AgentJoined?.agent_id}`;
  if ("AgentLeft" in event) return `Agent left: ${(event as { AgentLeft: { agent_id: string } }).AgentLeft?.agent_id}`;
  if ("TaskDelegated" in event) {
    const td = (event as { TaskDelegated: { from: string; to: string; task: string } }).TaskDelegated;
    return `Task delegated: ${td?.from} -> ${td?.to}: ${td?.task}`;
  }
  if ("StateUpdated" in event) {
    const su = (event as { StateUpdated: { agent_id: string; key: string } }).StateUpdated;
    return `State updated by ${su?.agent_id}: ${su?.key}`;
  }
  if ("MessageSent" in event) {
    const ms = (event as { MessageSent: { from: string; to: string } }).MessageSent;
    return `Message: ${ms?.from} -> ${ms?.to}`;
  }
  // Fallback: show raw JSON
  return JSON.stringify(event);
}

export function EventLog() {
  const [events] = useAtom(collaborationEventsAtom);

  if (events.length === 0) {
    return (
      <p className="text-xs text-muted-foreground py-4 text-center">
        No collaboration events yet.
      </p>
    );
  }

  return (
    <div className="space-y-0.5 max-h-60 overflow-auto">
      {events.map((evt, i) => (
        <div
          key={i}
          className="flex items-start gap-2 py-1 text-xs border-l-2 border-purple-500/30 pl-2"
        >
          <span className="text-muted-foreground shrink-0 w-6 text-right">{i + 1}.</span>
          <span className="break-all">{eventSummary(evt)}</span>
        </div>
      ))}
    </div>
  );
}
