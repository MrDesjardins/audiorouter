import type { DeviceListItem, Session } from "@audiorouter/contracts";

export function setupCheckRefreshKey(
  devices: DeviceListItem[],
  sessions: Session[],
  runningSessionIds: readonly string[],
) {
  return JSON.stringify([
    devices
      .map((device) => [device.id, device.state, "format" in device ? device.format : null])
      .sort(([a], [b]) => String(a).localeCompare(String(b))),
    sessions.map((session) => [session.id, session.revision]).sort(([a], [b]) => String(a).localeCompare(String(b))),
    [...runningSessionIds].sort(),
  ]);
}
