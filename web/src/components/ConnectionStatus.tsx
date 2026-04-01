import { useAtomValue } from "jotai";
import { connectionStatusAtom, reconnectAttemptAtom } from "@/atoms/ui";
import { cn } from "@/lib/utils";

const STATUS_CONFIG = {
  connected: {
    dotClass: "bg-emerald-500",
    label: "Connected",
  },
  reconnecting: {
    dotClass: "bg-yellow-500 animate-pulse",
    label: "Reconnecting",
  },
  disconnected: {
    dotClass: "bg-red-500",
    label: "Disconnected",
  },
} as const;

export function ConnectionStatus() {
  const status = useAtomValue(connectionStatusAtom);
  const attempt = useAtomValue(reconnectAttemptAtom);
  const config = STATUS_CONFIG[status];

  const label =
    status === "reconnecting" && attempt > 0
      ? `Reconnecting... (${attempt})`
      : config.label;

  return (
    <div
      className="flex items-center gap-1.5 text-xs text-muted-foreground shrink-0"
      title={label}
    >
      <span className={cn("h-2 w-2 rounded-full", config.dotClass)} />
      <span className="hidden sm:inline">{label}</span>
    </div>
  );
}
