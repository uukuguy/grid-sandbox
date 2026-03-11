import { useAtom } from "jotai";
import { contextStatusAtom } from "@/atoms/debug";
import { cn } from "@/lib/utils";

const LEVEL_META: Record<string, { label: string; color: string }> = {
  L0: { label: "L0 None", color: "bg-green-500" },
  L1: { label: "L1 Soft Trim", color: "bg-yellow-500" },
  L2: { label: "L2 Hard Clear", color: "bg-orange-500" },
  L3: { label: "L3 Compact", color: "bg-red-500" },
};

function levelMeta(level: string) {
  return LEVEL_META[level] ?? { label: level, color: "bg-gray-500" };
}

export function ContextStatus() {
  const [status] = useAtom(contextStatusAtom);

  if (!status) return null;

  const meta = levelMeta(status.level);

  return (
    <div className="px-4 py-3 border-b border-border">
      <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-2">
        Context Degradation
      </h3>
      <div className="flex items-center gap-3">
        <span
          className={cn(
            "inline-flex items-center rounded px-2 py-0.5 text-xs font-semibold text-white",
            meta.color,
          )}
        >
          {meta.label}
        </span>
        <div className="flex-1 h-2 rounded bg-secondary/30 overflow-hidden">
          <div
            className={cn("h-full transition-all", meta.color)}
            style={{ width: `${Math.min(status.usage_pct, 100)}%` }}
          />
        </div>
        <span className="text-xs font-mono text-muted-foreground">
          {status.usage_pct.toFixed(0)}%
        </span>
      </div>
    </div>
  );
}
