import { atom } from "jotai";

// --- Live event stream for real-time debug dashboard ---

export interface LiveEvent {
  id: string;
  timestamp: number;
  type: string;
  summary: string;
  data?: unknown;
}

const LIVE_EVENTS_CAP = 500;

/** FIFO ring of live events from the WebSocket stream, capped at 500 entries. */
export const liveEventsAtom = atom<LiveEvent[]>([]);

/** Derived write atom that appends an event and enforces the cap. */
export const pushLiveEventAtom = atom(null, (_get, set, event: LiveEvent) => {
  set(liveEventsAtom, (prev) => {
    const next = [...prev, event];
    return next.length > LIVE_EVENTS_CAP ? next.slice(next.length - LIVE_EVENTS_CAP) : next;
  });
});

/** Context degradation status from the latest context_degraded event. */
export const contextStatusAtom = atom<{ level: string; usage_pct: number } | null>(null);

// --- Existing tool execution & budget atoms ---

export interface ToolExecutionRecord {
  id: string;
  session_id: string;
  tool_name: string;
  source: string;
  input: unknown;
  output: unknown | null;
  status: "running" | "success" | "failed" | "timeout";
  started_at: number;
  duration_ms: number | null;
  error: string | null;
}

export interface TokenBudget {
  total: number;
  system_prompt: number;
  dynamic_context: number;
  history: number;
  free: number;
  usage_percent: number;
  degradation_level: number;
}

export const executionRecordsAtom = atom<ToolExecutionRecord[]>([]);
export const tokenBudgetAtom = atom<TokenBudget | null>(null);
export const selectedExecutionIdAtom = atom<string | null>(null);
