import { useAtom, useSetAtom } from "jotai";
import { useRef, useState, useEffect } from "react";
import { liveEventsAtom, type LiveEvent } from "@/atoms/debug";
import { cn } from "@/lib/utils";

type FilterKey = "all" | "tool" | "context" | "budget" | "security";

const FILTERS: { key: FilterKey; label: string }[] = [
  { key: "all", label: "All" },
  { key: "tool", label: "ToolCalls" },
  { key: "context", label: "Context" },
  { key: "budget", label: "Budget" },
  { key: "security", label: "Security" },
];

function matchesFilter(event: LiveEvent, filter: FilterKey): boolean {
  if (filter === "all") return true;
  if (filter === "tool")
    return ["tool_start", "tool_result", "tool_error", "tool_execution"].includes(event.type);
  if (filter === "context")
    return ["context_degraded", "memory_flushed"].includes(event.type);
  if (filter === "budget") return event.type === "token_budget_update";
  if (filter === "security")
    return ["security_blocked", "approval_required", "error"].includes(event.type);
  return true;
}

function eventColor(type: string): string {
  switch (type) {
    case "tool_start":
      return "text-green-400 border-green-500/30";
    case "tool_result":
      return "text-green-300 border-green-500/20";
    case "tool_error":
      return "text-red-400 border-red-500/30";
    case "text_complete":
      return "text-blue-400 border-blue-500/30";
    case "context_degraded":
      return "text-yellow-400 border-yellow-500/30";
    case "memory_flushed":
      return "text-purple-400 border-purple-500/30";
    case "token_budget_update":
      return "text-cyan-400 border-cyan-500/30";
    case "error":
    case "security_blocked":
      return "text-red-400 border-red-500/30";
    case "approval_required":
      return "text-orange-400 border-orange-500/30";
    default:
      return "text-muted-foreground border-border";
  }
}

function eventTypeIcon(type: string): string {
  switch (type) {
    case "tool_start":
      return "[T+]";
    case "tool_result":
      return "[T=]";
    case "tool_error":
      return "[T!]";
    case "context_degraded":
      return "[CD]";
    case "memory_flushed":
      return "[MF]";
    case "token_budget_update":
      return "[TB]";
    case "error":
      return "[ER]";
    case "security_blocked":
      return "[SB]";
    case "approval_required":
      return "[AP]";
    default:
      return "[--]";
  }
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString("en-GB", { hour12: false }) + "." + String(d.getMilliseconds()).padStart(3, "0");
}

export function EventStream() {
  const [events] = useAtom(liveEventsAtom);
  const setEvents = useSetAtom(liveEventsAtom);
  const [filter, setFilter] = useState<FilterKey>("all");
  const [paused, setPaused] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const filtered = events.filter((e) => matchesFilter(e, filter));

  // Auto-scroll to bottom when new events arrive (unless paused)
  useEffect(() => {
    if (!paused && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [filtered.length, paused]);

  return (
    <div className="flex flex-1 flex-col min-h-0 border-t border-border">
      {/* Header + controls */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border">
        <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
          Live Event Stream
          <span className="ml-2 text-foreground font-mono">{filtered.length}</span>
        </h3>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setPaused((p) => !p)}
            className={cn(
              "text-xs px-2 py-0.5 rounded",
              paused
                ? "bg-yellow-500/20 text-yellow-400"
                : "bg-secondary text-muted-foreground hover:text-foreground",
            )}
          >
            {paused ? "Resume" : "Pause"}
          </button>
          <button
            onClick={() => setEvents([])}
            className="text-xs px-2 py-0.5 rounded bg-secondary text-muted-foreground hover:text-foreground"
          >
            Clear
          </button>
        </div>
      </div>

      {/* Filter buttons */}
      <div className="flex items-center gap-1 px-4 py-1.5 border-b border-border">
        {FILTERS.map((f) => (
          <button
            key={f.key}
            onClick={() => setFilter(f.key)}
            className={cn(
              "text-xs px-2 py-0.5 rounded transition-colors",
              filter === f.key
                ? "bg-secondary text-foreground"
                : "text-muted-foreground hover:text-foreground hover:bg-secondary/50",
            )}
          >
            {f.label}
          </button>
        ))}
      </div>

      {/* Event list */}
      <div ref={scrollRef} className="flex-1 overflow-auto px-4 py-1">
        {filtered.length === 0 ? (
          <p className="text-xs text-muted-foreground py-4 text-center">
            No events yet. Start a conversation to see live events.
          </p>
        ) : (
          <div className="space-y-0.5">
            {filtered.map((evt) => (
              <div
                key={evt.id}
                className={cn(
                  "flex items-start gap-2 py-0.5 text-xs font-mono border-l-2 pl-2",
                  eventColor(evt.type),
                )}
              >
                <span className="text-muted-foreground shrink-0">{formatTime(evt.timestamp)}</span>
                <span className="shrink-0 w-8 text-center">{eventTypeIcon(evt.type)}</span>
                <span className="break-all">{evt.summary}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
