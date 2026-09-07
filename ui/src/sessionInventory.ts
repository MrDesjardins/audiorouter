import type { Session } from "@audiorouter/contracts";

/** Merge inventory sources while letting the point-in-time snapshot win for its ID. */
export function mergeSessionInventory(
  listed: Session[],
  snapshot: Session | null,
  created: Session[],
): Session[] {
  const sessions = new Map(listed.map((session) => [session.id, session]));
  if (snapshot) sessions.set(snapshot.id, snapshot);
  for (const session of created) sessions.set(session.id, session);
  return [...sessions.values()];
}
