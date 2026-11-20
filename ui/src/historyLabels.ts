import type { Session } from "@audiorouter/contracts";
import { i18n, tx } from "./i18n";

/** One plain sentence for what changed between two drafts. Layout is not part of a draft. */
export function describeChange(before: Session, after: Session): string {
  const added = after.nodes.filter((node) => !before.nodes.some((old) => old.id === node.id));
  const removed = before.nodes.filter((node) => !after.nodes.some((next) => next.id === node.id));
  if (added.length === 1 && removed.length === 0) return tx("historyUi.added", { name: added[0].name });
  if (removed.length === 1 && added.length === 0) return tx("historyUi.removed", { name: removed[0].name });
  if (added.length + removed.length > 0) {
    return tx("historyUi.changedTools", { added: added.length, removed: removed.length });
  }
  const connected = after.edges.length - before.edges.length;
  const name = (id: string) => after.nodes.find((node) => node.id === id)?.name ?? id;
  if (connected > 0) {
    const edge = after.edges.find((candidate) => !before.edges.some((old) => old.id === candidate.id));
    return edge
      ? tx("historyUi.connected", { source: name(edge.sourceNode), destination: name(edge.destinationNode) })
      : tx("historyUi.addConnection");
  }
  if (connected < 0) {
    const edge = before.edges.find((candidate) => !after.edges.some((next) => next.id === candidate.id));
    return edge
      ? tx("historyUi.disconnected", { source: name(edge.sourceNode), destination: name(edge.destinationNode) })
      : tx("historyUi.removeConnection");
  }
  if (before.name !== after.name) return tx("historyUi.sessionRenamed", { name: after.name });
  for (const node of after.nodes) {
    const old = before.nodes.find((candidate) => candidate.id === node.id);
    if (!old) continue;
    if (old.name !== node.name) return tx("historyUi.renamed", { old: old.name, name: node.name });
    if (old.enabled !== node.enabled)
      return tx(node.enabled ? "historyUi.enabled" : "historyUi.disabled", { name: node.name });
    if (old.bypass !== node.bypass)
      return tx(node.bypass ? "historyUi.bypassed" : "historyUi.unbypassed", { name: node.name });
    if (JSON.stringify(old.parameters) !== JSON.stringify(node.parameters))
      return tx("historyUi.changed", { name: node.name });
  }
  return tx("historyUi.changedRoute");
}

/** Who saved a version, as the backend reports it. A missing identity is never replaced by a guess (UI-07). */
export function savedByLabel(savedBy: string | null): string {
  if (!savedBy) return tx("historyUi.saverUnknown");
  if (savedBy === "this PC") return tx("historyUi.thisPc");
  return tx("historyUi.savedBy", { name: savedBy });
}

export function savedAtLabel(savedAtMs: number | null, now = Date.now()): string {
  if (savedAtMs === null) return tx("historyUi.timeUnknown");
  const date = new Date(savedAtMs);
  const time = date.toLocaleTimeString(i18n.resolvedLanguage, { hour: "2-digit", minute: "2-digit" });
  const today = new Date(now);
  if (date.toDateString() === today.toDateString()) return time;
  today.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return tx("historyUi.yesterday", { time });
  return `${date.toLocaleDateString(i18n.resolvedLanguage)}, ${time}`;
}
