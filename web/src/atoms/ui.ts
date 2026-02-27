import { atom } from "jotai";

export type TabId = "chat" | "tools" | "debug" | "memory" | "mcp";
export const activeTabAtom = atom<TabId>("chat");
export const sidebarOpenAtom = atom(false);
