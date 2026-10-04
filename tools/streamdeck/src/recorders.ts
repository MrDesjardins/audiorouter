import type { AudioRouterStore, SummaryNode } from "./store.js";

export type RecordSettings = {
  /** A Recorder node of the selected session; empty: the session's only Recorder. */
  nodeId?: string;
  nodeName?: string;
  /** toggle: Record or Stop; start / stop: the key only does that. */
  press?: "toggle" | "start" | "stop";
};

/** The Recorder this key drives: the chosen one, or the session's only Recorder. */
export function recorderFor(store: AudioRouterStore, settings: RecordSettings): SummaryNode | undefined {
  if (settings.nodeId) {
    const node = store.node(settings.nodeId, settings.nodeName);
    return node?.kind === "recorder" ? node : undefined;
  }
  const recorders = (store.summary?.nodes ?? []).filter((node) => node.kind === "recorder");
  return recorders.length === 1 ? recorders[0] : undefined;
}
