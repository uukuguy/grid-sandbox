import { useState, useEffect, useMemo } from "react";
import { useAtomValue } from "jotai";
import { sessionIdAtom } from "@/atoms/session";

interface MemoryBlock {
  id: string;
  kind: string;
  label: string;
  value: string;
  priority: number;
  char_limit: number;
  is_readonly: boolean;
}

interface MemoryTimestamps {
  created_at: number;
  updated_at: number;
  accessed_at: number;
}

interface PersistentMemory {
  id: string;
  content: string;
  category: string;
  importance: number;
  created_at: string;
  memory_type?: string;
  session_id?: string;
  timestamps?: MemoryTimestamps;
}

interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: Array<{ type: string; text?: string }>;
}

interface ActiveSessionsResponse {
  sessions: string[];
  count: number;
  max: number;
}

type MemoryViewType = "working" | "session" | "persistent" | "timeline";

export default function Memory() {
  const [activeMemory, setActiveMemory] = useState<MemoryViewType>("working");
  const [workingMemory, setWorkingMemory] = useState<MemoryBlock[]>([]);
  const [persistentMemory, setPersistentMemory] = useState<PersistentMemory[]>([]);
  const [sessionMessages, setSessionMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [availableSessions, setAvailableSessions] = useState<string[]>([]);
  const [selectedSessionFilter, setSelectedSessionFilter] = useState<string>("");
  const sessionId = useAtomValue(sessionIdAtom);

  useEffect(() => {
    fetchWorkingMemory();
    fetchPersistentMemory();
    fetchAvailableSessions();
  }, []);

  useEffect(() => {
    if (activeMemory === "session" && sessionId) {
      fetchSessionMessages();
    }
  }, [activeMemory, sessionId]);

  // Re-fetch persistent memory when session filter changes
  useEffect(() => {
    fetchPersistentMemory();
  }, [selectedSessionFilter]);

  const fetchAvailableSessions = async () => {
    try {
      const res = await fetch("/api/v1/sessions/active");
      const data: ActiveSessionsResponse = await res.json();
      setAvailableSessions(data.sessions || []);
    } catch (error) {
      console.error("Failed to fetch active sessions:", error);
    }
  };

  const fetchSessionMessages = async () => {
    if (!sessionId) return;
    setLoading(true);
    try {
      const res = await fetch(`/api/v1/sessions/${sessionId}`);
      const data = await res.json();
      setSessionMessages(data.messages || []);
    } catch (error) {
      console.error("Failed to fetch session messages:", error);
    } finally {
      setLoading(false);
    }
  };

  const fetchWorkingMemory = async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/v1/memories/working");
      const data = await res.json();
      setWorkingMemory(data.blocks || []);
    } catch (error) {
      console.error("Failed to fetch working memory:", error);
    } finally {
      setLoading(false);
    }
  };

  const fetchPersistentMemory = async () => {
    setLoading(true);
    try {
      const sessionParam = selectedSessionFilter
        ? `&session_id=${encodeURIComponent(selectedSessionFilter)}`
        : "";
      const res = await fetch(`/api/v1/memories?limit=100${sessionParam}`);
      const data = await res.json();
      setPersistentMemory(data.results || []);
    } catch (error) {
      console.error("Failed to fetch persistent memory:", error);
    } finally {
      setLoading(false);
    }
  };

  const filteredWorkingMemory = workingMemory.filter((block) =>
    (block.value + " " + block.label).toLowerCase().includes(searchQuery.toLowerCase())
  );

  const filteredPersistentMemory = persistentMemory.filter((mem) =>
    mem.content.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Timeline entries: all persistent memories sorted by timestamp (newest first)
  const timelineEntries = useMemo(() => {
    const filtered = searchQuery
      ? persistentMemory.filter((mem) =>
          mem.content.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : persistentMemory;
    return [...filtered].sort((a, b) => {
      const tsA = a.timestamps?.created_at ?? 0;
      const tsB = b.timestamps?.created_at ?? 0;
      return tsB - tsA;
    });
  }, [persistentMemory, searchQuery]);

  const memoryTypes: { id: MemoryViewType; label: string }[] = [
    { id: "working", label: "Working Memory" },
    { id: "session", label: "Session Memory" },
    { id: "persistent", label: "Persistent Memory" },
    { id: "timeline", label: "Timeline" },
  ];

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {/* Header */}
      <div className="px-4 py-3 border-b border-border flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Memory Explorer</h2>
          <p className="text-sm text-muted-foreground">
            View and manage AI memory across different layers
          </p>
        </div>
        <div className="flex items-center gap-2">
          {/* Session filter dropdown */}
          <select
            value={selectedSessionFilter}
            onChange={(e) => setSelectedSessionFilter(e.target.value)}
            className="text-xs px-2 py-1 rounded border border-border bg-secondary focus:outline-none focus:ring-2 focus:ring-primary"
          >
            <option value="">All sessions</option>
            {availableSessions.map((sid) => (
              <option key={sid} value={sid}>
                {sid.length > 12 ? `${sid.slice(0, 12)}...` : sid}
              </option>
            ))}
          </select>
          <button
            onClick={() => {
              fetchWorkingMemory();
              fetchPersistentMemory();
              fetchAvailableSessions();
              if (sessionId) fetchSessionMessages();
            }}
            className="text-xs px-2 py-1 rounded border border-border hover:bg-secondary/50"
          >
            Refresh
          </button>
        </div>
      </div>

      {/* Memory Type Tabs */}
      <div className="flex gap-2 px-4 py-2 border-b border-border">
        {memoryTypes.map((type) => (
          <button
            key={type.id}
            onClick={() => setActiveMemory(type.id)}
            className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
              activeMemory === type.id
                ? "bg-secondary text-foreground"
                : "text-muted-foreground hover:text-foreground hover:bg-secondary/50"
            }`}
          >
            {type.label}
          </button>
        ))}
      </div>

      {/* Search */}
      <div className="px-4 py-2 border-b border-border">
        <input
          type="text"
          placeholder="Search memories..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full px-3 py-2 text-sm bg-secondary border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
        />
      </div>

      {/* Memory Content */}
      <div className="flex-1 overflow-auto p-4">
        {loading ? (
          <div className="flex items-center justify-center h-full">
            <span className="text-muted-foreground">Loading...</span>
          </div>
        ) : activeMemory === "working" ? (
          <WorkingMemoryView blocks={filteredWorkingMemory} />
        ) : activeMemory === "session" ? (
          <SessionMemoryView messages={sessionMessages} />
        ) : activeMemory === "timeline" ? (
          <TimelineView entries={timelineEntries} />
        ) : (
          <PersistentMemoryView memories={filteredPersistentMemory} />
        )}
      </div>

      {/* Stats Footer */}
      <div className="px-4 py-2 border-t border-border bg-card text-xs text-muted-foreground">
        <span className="mr-4">
          Working: {workingMemory.length} blocks
        </span>
        <span className="mr-4">
          Session: {sessionMessages.length} messages
        </span>
        <span className="mr-4">
          Persistent: {persistentMemory.length} memories
        </span>
        {selectedSessionFilter && (
          <span className="mr-4 text-primary">
            Filtered: {selectedSessionFilter.slice(0, 12)}...
          </span>
        )}
      </div>
    </div>
  );
}

function WorkingMemoryView({ blocks }: { blocks: MemoryBlock[] }) {
  if (blocks.length === 0) {
    return (
      <div className="text-center text-muted-foreground py-8">
        <p>No working memory blocks</p>
        <p className="text-sm mt-2">Working memory contains current context</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="font-medium">Context Blocks</h3>
        <span className="text-sm text-muted-foreground">{blocks.length} blocks</span>
      </div>
      <div className="space-y-2">
        {blocks.map((block) => (
          <div
            key={block.id}
            className="p-3 bg-secondary rounded-lg border border-border"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium px-2 py-0.5 bg-primary/10 rounded font-mono">
                  {block.kind}
                </span>
                <span className="text-xs text-muted-foreground">{block.label}</span>
              </div>
              <span className="text-xs text-muted-foreground">
                {block.char_limit} char limit
              </span>
            </div>
            {block.value ? (
              <p className="text-sm">{block.value}</p>
            ) : (
              <p className="text-sm text-muted-foreground italic">empty</p>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function SessionMemoryView({ messages }: { messages: ChatMessage[] }) {
  if (messages.length === 0) {
    return (
      <div className="text-center text-muted-foreground py-8">
        <p>No session messages</p>
        <p className="text-sm mt-2">
          Start a conversation to see messages here
        </p>
      </div>
    );
  }

  // Extract text content from message
  const getTextContent = (msg: ChatMessage): string => {
    return msg.content
      .filter((c) => c.type === "text" && c.text)
      .map((c) => c.text)
      .join("");
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="font-medium">Conversation History</h3>
        <span className="text-sm text-muted-foreground">{messages.length} messages</span>
      </div>
      {messages.map((msg, idx) => (
        <div
          key={idx}
          className={`p-3 rounded-lg border ${
            msg.role === "user"
              ? "bg-primary/5 border-border"
              : msg.role === "assistant"
              ? "bg-secondary border-border"
              : "bg-muted border-border"
          }`}
        >
          <div className="flex items-center gap-2 mb-2">
            <span className="text-xs font-medium px-2 py-0.5 bg-primary/10 rounded">
              {msg.role}
            </span>
          </div>
          <p className="text-sm whitespace-pre-wrap">{getTextContent(msg)}</p>
        </div>
      ))}
    </div>
  );
}

function PersistentMemoryView({ memories }: { memories: PersistentMemory[] }) {
  if (memories.length === 0) {
    return (
      <div className="text-center text-muted-foreground py-8">
        <p>No persistent memories</p>
        <p className="text-sm mt-2">
          Use memory_store tool to save important information
        </p>
      </div>
    );
  }

  const categories = [...new Set(memories.map((m) => m.category))];

  return (
    <div className="space-y-4">
      {/* Category Filter */}
      <div className="flex gap-2 flex-wrap">
        {categories.map((cat) => (
          <span
            key={cat}
            className="text-xs px-2 py-1 bg-secondary rounded-full text-muted-foreground"
          >
            {cat}
          </span>
        ))}
      </div>

      {/* Memory List */}
      <div className="space-y-2">
        {memories.map((mem) => (
          <div
            key={mem.id}
            className="p-3 bg-secondary rounded-lg border border-border hover:border-primary/50 transition-colors"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium px-2 py-0.5 bg-primary/10 rounded">
                  {mem.category}
                </span>
                <span className="text-xs text-muted-foreground">
                  {mem.importance.toFixed(1)}
                </span>
              </div>
              <span className="text-xs text-muted-foreground">
                {mem.created_at}
              </span>
            </div>
            <p className="text-sm line-clamp-3">{mem.content}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

/** Format a unix timestamp (seconds) to a human-readable date/time string. */
function formatTimestamp(ts: number): string {
  if (!ts || ts === 0) return "Unknown";
  const d = new Date(ts * 1000);
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** Badge color mapping for memory types. */
function memoryTypeBadgeClass(memType?: string): string {
  switch (memType) {
    case "semantic":
      return "bg-blue-500/15 text-blue-400";
    case "episodic":
      return "bg-amber-500/15 text-amber-400";
    case "procedural":
      return "bg-emerald-500/15 text-emerald-400";
    default:
      return "bg-primary/10 text-muted-foreground";
  }
}

function TimelineView({ entries }: { entries: PersistentMemory[] }) {
  if (entries.length === 0) {
    return (
      <div className="text-center text-muted-foreground py-8">
        <p>No timeline entries</p>
        <p className="text-sm mt-2">
          Memory entries will appear here sorted by time
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="font-medium">Memory Timeline</h3>
        <span className="text-sm text-muted-foreground">
          {entries.length} entries
        </span>
      </div>

      {/* Vertical timeline */}
      <div className="relative pl-6">
        {/* Connecting line */}
        <div className="absolute left-2 top-2 bottom-2 w-px bg-border" />

        <div className="space-y-4">
          {entries.map((entry) => {
            const ts = entry.timestamps?.created_at ?? 0;
            return (
              <div key={entry.id} className="relative">
                {/* Timeline dot */}
                <div className="absolute -left-6 top-3 w-4 h-4 rounded-full border-2 border-primary bg-card" />

                {/* Entry card */}
                <div className="p-3 bg-secondary rounded-lg border border-border hover:border-primary/50 transition-colors">
                  {/* Top row: timestamp + badges */}
                  <div className="flex items-center justify-between mb-1.5">
                    <span className="text-xs font-mono text-muted-foreground">
                      {formatTimestamp(ts)}
                    </span>
                    <div className="flex items-center gap-1.5">
                      {entry.memory_type && (
                        <span
                          className={`text-xs font-medium px-2 py-0.5 rounded ${memoryTypeBadgeClass(
                            entry.memory_type
                          )}`}
                        >
                          {entry.memory_type}
                        </span>
                      )}
                      <span className="text-xs font-medium px-2 py-0.5 bg-primary/10 rounded">
                        {entry.category}
                      </span>
                    </div>
                  </div>

                  {/* Content preview */}
                  <p className="text-sm line-clamp-2">{entry.content}</p>

                  {/* Session badge (if present) */}
                  {entry.session_id && (
                    <div className="mt-1.5">
                      <span className="text-xs text-muted-foreground font-mono">
                        session: {entry.session_id.slice(0, 12)}...
                      </span>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
