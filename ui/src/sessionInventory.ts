import type { Session } from "@audiorouter/contracts";

/** Never let an older snapshot or creation response replace a newer revision. */
export function mergeSessionInventory(
  listed: Session[],
  snapshot: Session | null,
  created: Session[],
): Session[] {
  const sessions = new Map(listed.map((session) => [session.id, session]));
  if (snapshot && snapshot.revision >= (sessions.get(snapshot.id)?.revision ?? -1)) sessions.set(snapshot.id, snapshot);
  for (const session of created) {
    const current = sessions.get(session.id);
    if (!current || session.revision > current.revision) sessions.set(session.id, session);
  }
  return [...sessions.values()];
}

/** Runtime telemetry is deliberately absent: it must never reset an editor. */
export function sameSessionDraft(left: Session, right: Session): boolean {
  return sameJsonValue(left, right);
}

function sameJsonValue(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (!left || !right || typeof left !== "object" || typeof right !== "object") return false;
  if (Array.isArray(left) || Array.isArray(right)) {
    return Array.isArray(left) && Array.isArray(right) && left.length === right.length && left.every((value, index) => sameJsonValue(value, right[index]));
  }
  const a = left as Record<string, unknown>, b = right as Record<string, unknown>;
  const keys = Object.keys(a).filter((key) => a[key] !== undefined);
  return keys.length === Object.keys(b).filter((key) => b[key] !== undefined).length
    && keys.every((key) => Object.hasOwn(b, key) && sameJsonValue(a[key], b[key]));
}

export function reconcileSessionDraft(draft: Session, previous: Session, incoming: Session): "unchanged" | "adopt" | "conflict" {
  if (incoming.id !== previous.id) return "adopt";
  if (incoming.revision < previous.revision || sameSessionDraft(previous, incoming)) return "unchanged";
  return sameSessionDraft(draft, previous) || sameSessionDraft(draft, incoming) ? "adopt" : "conflict";
}
