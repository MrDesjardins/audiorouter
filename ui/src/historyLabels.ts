import type { Session } from "@audiorouter/contracts";

/** One plain sentence for what changed between two drafts. Layout is not part of a draft. */
export function describeChange(before: Session, after: Session): string {
  const added = after.nodes.filter((node) => !before.nodes.some((old) => old.id === node.id));
  const removed = before.nodes.filter((node) => !after.nodes.some((next) => next.id === node.id));
  if (added.length === 1 && removed.length === 0) return `Added ${added[0].name}`;
  if (removed.length === 1 && added.length === 0) return `Removed ${removed[0].name}`;
  if (added.length + removed.length > 0) {
    return `Changed tools: ${added.length} added, ${removed.length} removed`;
  }
  const connected = after.edges.length - before.edges.length;
  const name = (id: string) => after.nodes.find((node) => node.id === id)?.name ?? id;
  if (connected > 0) {
    const edge = after.edges.find((candidate) => !before.edges.some((old) => old.id === candidate.id));
    return edge ? `Connected ${name(edge.sourceNode)} to ${name(edge.destinationNode)}` : "Added a connection";
  }
  if (connected < 0) {
    const edge = before.edges.find((candidate) => !after.edges.some((next) => next.id === candidate.id));
    return edge ? `Disconnected ${name(edge.sourceNode)} from ${name(edge.destinationNode)}` : "Removed a connection";
  }
  if (before.name !== after.name) return `Renamed the session to ${after.name}`;
  for (const node of after.nodes) {
    const old = before.nodes.find((candidate) => candidate.id === node.id);
    if (!old) continue;
    if (old.name !== node.name) return `Renamed ${old.name} to ${node.name}`;
    if (old.enabled !== node.enabled) return `${node.enabled ? "Turned on" : "Turned off"} ${node.name}`;
    if (old.bypass !== node.bypass) return `${node.bypass ? "Bypassed" : "Stopped bypassing"} ${node.name}`;
    if (JSON.stringify(old.parameters) !== JSON.stringify(node.parameters)) return `Changed ${node.name}`;
  }
  return "Changed the route";
}

/** Who saved a version, as the backend reports it. A missing identity is never replaced by a guess (UI-07). */
export function savedByLabel(savedBy: string | null): string {
  if (!savedBy) return "Saver not recorded";
  if (savedBy === "this PC") return "this PC";
  return `saved by “${savedBy}”`;
}

export function savedAtLabel(savedAtMs: number | null, now = Date.now()): string {
  if (savedAtMs === null) return "Time not recorded";
  const date = new Date(savedAtMs);
  const time = date.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  const today = new Date(now);
  if (date.toDateString() === today.toDateString()) return time;
  today.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return `Yesterday, ${time}`;
  return `${date.toLocaleDateString()}, ${time}`;
}
