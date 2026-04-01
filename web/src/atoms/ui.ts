import { atom } from "jotai";

export type TabId = "chat" | "tasks" | "schedule" | "tools" | "debug" | "memory" | "mcp" | "collaboration";
export const activeTabAtom = atom<TabId>("chat");
export const sidebarOpenAtom = atom(false);

// ── Toast Notifications ──

export type ToastType = "success" | "error" | "warning" | "info";

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  title?: string;
  /** Auto-dismiss duration in ms (default: 5000) */
  duration?: number;
}

export type AddToastInput = Omit<Toast, "id">;

/** Read-only atom holding the current toast stack */
export const toastsAtom = atom<Toast[]>([]);

/** Write-only atom: add a toast */
export const addToastAtom = atom(null, (get, set, input: AddToastInput) => {
  const toast: Toast = { ...input, id: crypto.randomUUID() };
  set(toastsAtom, [...get(toastsAtom), toast]);
});

/** Write-only atom: remove a toast by id */
export const removeToastAtom = atom(null, (get, set, id: string) => {
  set(
    toastsAtom,
    get(toastsAtom).filter((t) => t.id !== id),
  );
});
