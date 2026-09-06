import type { Session } from "@audiorouter/contracts";

export type DraftHistory = { past: Session[]; future: Session[] };
export type HistoryTransition = { current: Session; history: DraftHistory };

const LIMIT = 20;

export function recordDraft(history: DraftHistory, current: Session, next: Session): DraftHistory {
  if (next === current) return history;
  return { past: [...history.past.slice(-(LIMIT - 1)), current], future: [] };
}

export function undoDraft(history: DraftHistory, current: Session): HistoryTransition {
  const previous = history.past.at(-1);
  if (!previous) return { current, history };
  return {
    current: previous,
    history: { past: history.past.slice(0, -1), future: [...history.future.slice(-(LIMIT - 1)), current] },
  };
}

export function redoDraft(history: DraftHistory, current: Session): HistoryTransition {
  const next = history.future.at(-1);
  if (!next) return { current, history };
  return {
    current: next,
    history: { past: [...history.past.slice(-(LIMIT - 1)), current], future: history.future.slice(0, -1) },
  };
}
