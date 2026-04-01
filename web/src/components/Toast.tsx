import { useEffect } from "react";
import { useAtom } from "jotai";
import {
  CheckCircle,
  XCircle,
  AlertTriangle,
  Info,
  X,
} from "lucide-react";
import { toastsAtom, removeToastAtom, type Toast } from "../atoms/ui";

const ICON_MAP = {
  success: CheckCircle,
  error: XCircle,
  warning: AlertTriangle,
  info: Info,
} as const;

const COLOR_MAP = {
  success: "border-green-600 bg-green-950/80 text-green-200",
  error: "border-red-600 bg-red-950/80 text-red-200",
  warning: "border-amber-600 bg-amber-950/80 text-amber-200",
  info: "border-blue-600 bg-blue-950/80 text-blue-200",
} as const;

const ICON_COLOR_MAP = {
  success: "text-green-400",
  error: "text-red-400",
  warning: "text-amber-400",
  info: "text-blue-400",
} as const;

function ToastItem({ toast }: { toast: Toast }) {
  const [, remove] = useAtom(removeToastAtom);
  const Icon = ICON_MAP[toast.type];

  useEffect(() => {
    const timer = setTimeout(() => {
      remove(toast.id);
    }, toast.duration ?? 5000);
    return () => clearTimeout(timer);
  }, [toast.id, toast.duration, remove]);

  return (
    <div
      className={`pointer-events-auto flex w-80 items-start gap-3 rounded-lg border p-3 shadow-lg backdrop-blur-sm ${COLOR_MAP[toast.type]}`}
      role="alert"
    >
      <Icon className={`mt-0.5 h-5 w-5 shrink-0 ${ICON_COLOR_MAP[toast.type]}`} />
      <div className="min-w-0 flex-1">
        {toast.title && (
          <p className="text-sm font-medium">{toast.title}</p>
        )}
        <p className="text-sm opacity-90">{toast.message}</p>
      </div>
      <button
        onClick={() => remove(toast.id)}
        className="shrink-0 rounded p-0.5 opacity-60 transition-opacity hover:opacity-100"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}

export function ToastContainer() {
  const [toasts] = useAtom(toastsAtom);

  if (toasts.length === 0) return null;

  return (
    <div className="pointer-events-none fixed right-4 top-4 z-50 flex flex-col gap-2">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>
  );
}
