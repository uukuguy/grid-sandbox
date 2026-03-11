import { atom } from "jotai";

export interface CollaborationAgent {
  id: string;
  name: string;
  capabilities: string[];
  session_id: string;
}

export interface CollaborationProposal {
  id: string;
  from_agent: string;
  action: string;
  description: string;
  status: string;
  votes: CollaborationVote[];
}

export interface CollaborationVote {
  agent_id: string;
  approve: boolean;
  reason: string | null;
}

export interface CollaborationEvent {
  [key: string]: unknown;
}

export interface CollaborationStatus {
  id: string;
  agent_count: number;
  active_agent: string | null;
  pending_proposals: number;
  event_count: number;
  state_keys: string[];
}

export interface SharedStateEntry {
  key: string;
  value: unknown;
}

export const collaborationStatusAtom = atom<CollaborationStatus | null>(null);
export const collaborationAgentsAtom = atom<CollaborationAgent[]>([]);
export const collaborationProposalsAtom = atom<CollaborationProposal[]>([]);
export const collaborationEventsAtom = atom<CollaborationEvent[]>([]);
export const collaborationSharedStateAtom = atom<SharedStateEntry[]>([]);
export const collaborationLoadingAtom = atom(false);
