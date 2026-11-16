// Remembered endpoint bindings and device-choice helpers (moved from App.tsx).
import type { DeviceListItem } from "@audiorouter/contracts";

const DEVICE_NAME_CACHE_KEY = "audiorouter.ui.endpoint-names";
const MAX_REMEMBERED_DEVICES = 512;

type RememberedDevice = { name: string; direction: "capture" | "render" };

function readRememberedDevices(): Record<string, RememberedDevice> {
  try {
    const value: unknown = JSON.parse(window.localStorage.getItem(DEVICE_NAME_CACHE_KEY) ?? "null");
    if (!value || typeof value !== "object" || Array.isArray(value)) return {};
    const records = value as Record<string, unknown>;
    const result: Record<string, RememberedDevice> = {};
    for (const [id, entry] of Object.entries(records).slice(-MAX_REMEMBERED_DEVICES)) {
      if (!entry || typeof entry !== "object") continue;
      const candidate = entry as Record<string, unknown>;
      if (
        id.length > 0 &&
        id.length <= 32768 &&
        typeof candidate.name === "string" &&
        candidate.name.length > 0 &&
        candidate.name.length <= 2048 &&
        (candidate.direction === "capture" || candidate.direction === "render")
      ) {
        result[id] = { name: candidate.name, direction: candidate.direction };
      }
    }
    return result;
  } catch {
    return {};
  }
}

/** Friendly name last seen for one exact endpoint ID, if it was cached locally. */
export function rememberedDeviceName(id: string): string | null {
  return readRememberedDevices()[id]?.name ?? null;
}

/** Return a capture endpoint's current inventory name, active or inactive. */
export function captureEndpointName(devices: readonly DeviceListItem[], id: string): string | null {
  return devices.find((device) => device.direction === "capture" && device.id === id && "name" in device)?.name ?? null;
}

/** Keep the last observed endpoint labels locally for exact-ID recovery proposals. */
export function rememberDeviceNames(devices: readonly DeviceListItem[]) {
  try {
    const remembered = readRememberedDevices();
    for (const device of devices) {
      if ("name" in device && typeof device.name === "string") {
        delete remembered[device.id];
        remembered[device.id] = { name: device.name, direction: device.direction };
      }
    }
    const bounded = Object.fromEntries(Object.entries(remembered).slice(-MAX_REMEMBERED_DEVICES));
    window.localStorage.setItem(DEVICE_NAME_CACHE_KEY, JSON.stringify(bounded));
  } catch {
    // The cache is only a recovery hint; unavailable local storage cannot stop routing.
  }
}

/** Seed names for legacy physical-input IDs while Windows still reports them. */
export function rememberSessionDeviceNames(
  sessions: readonly import("@audiorouter/contracts").Session[],
  devices: readonly DeviceListItem[],
) {
  const names = new Map(
    devices
      .filter((device) => device.direction === "capture" && "name" in device)
      .map((device) => [device.id, device.name] as const),
  );
  for (const session of sessions) {
    for (const node of session.nodes) {
      if (node.kind !== "physicalInput" || typeof node.parameters.endpointId !== "string") continue;
      const name = names.get(node.parameters.endpointId);
      if (name) rememberDeviceNameBinding(node.parameters.endpointId, name, "capture");
    }
  }
}

/** Record a user-confirmed replacement for an old input ID so sibling sessions can recover too. */
export function rememberDeviceNameBinding(id: string, name: string, direction: "capture" | "render") {
  if (
    !id ||
    id.length > 32768 ||
    !name ||
    name.length > 2048 ||
    [...name].some((character) => {
      const code = character.charCodeAt(0);
      return code < 32 || code === 127;
    })
  )
    return;
  try {
    const remembered = readRememberedDevices();
    delete remembered[id];
    remembered[id] = { name, direction };
    window.localStorage.setItem(
      DEVICE_NAME_CACHE_KEY,
      JSON.stringify(Object.fromEntries(Object.entries(remembered).slice(-MAX_REMEMBERED_DEVICES))),
    );
  } catch {
    // The cache is a recovery aid, never a routing prerequisite.
  }
}

/** Return a proposal only for one active capture endpoint with an exact friendly-name match. */
export function findReplacementCaptureEndpoint(
  devices: readonly DeviceListItem[],
  missingEndpointId: string | undefined,
  savedName?: string,
): Extract<DeviceListItem, { state: "active" }> | null {
  if (!missingEndpointId) return null;
  const previous = readRememberedDevices()[missingEndpointId];
  const name = savedName || previous?.name;
  if (!name || (previous && previous.direction !== "capture")) return null;
  const matches = devices.filter(
    (device): device is Extract<DeviceListItem, { state: "active" }> =>
      device.state === "active" &&
      device.direction === "capture" &&
      device.id !== missingEndpointId &&
      device.name.toLowerCase() === name.toLowerCase(),
  );
  return matches.length === 1 ? matches[0] : null;
}

export function endpointBindingStorageKey(sessionId: string) {
  return `audiorouter.ui.endpoint-binding.${sessionId}`;
}

export function readEndpointBindingHint(sessionId: string): { captureEndpointId?: string; renderEndpointId?: string } {
  try {
    const value: unknown = JSON.parse(window.localStorage.getItem(endpointBindingStorageKey(sessionId)) ?? "null");
    if (!value || typeof value !== "object") return {};
    const record = value as Record<string, unknown>;
    return {
      captureEndpointId:
        typeof record.captureEndpointId === "string" ? record.captureEndpointId.slice(0, 32768) : undefined,
      renderEndpointId:
        typeof record.renderEndpointId === "string" ? record.renderEndpointId.slice(0, 32768) : undefined,
    };
  } catch {
    return {};
  }
}

/**
 * Devices for a single-device route: the endpoints chosen in the connected
 * Input and Output nodes' Properties, else the last choice made on this PC.
 * Never a default device.
 */
export function routeEndpointBinding(
  route: import("@audiorouter/contracts").Session,
  sessionId: string,
): { captureEndpointId?: string; renderEndpointId?: string } {
  const hint = readEndpointBindingHint(sessionId);
  const connected = (nodeId: string) =>
    route.edges.some((edge) => edge.enabled && (edge.sourceNode === nodeId || edge.destinationNode === nodeId));
  const bound = (kind: string) => {
    const value = route.nodes.find(
      (node) =>
        node.enabled && node.kind === kind && connected(node.id) && typeof node.parameters.endpointId === "string",
    )?.parameters.endpointId;
    return typeof value === "string" ? value : undefined;
  };
  return {
    captureEndpointId: bound("physicalInput") ?? hint.captureEndpointId,
    renderEndpointId: bound("physicalOutput") ?? hint.renderEndpointId,
  };
}

export function writeEndpointBindingHint(sessionId: string, captureEndpointId: string, renderEndpointId: string) {
  try {
    window.localStorage.setItem(
      endpointBindingStorageKey(sessionId),
      JSON.stringify({ captureEndpointId, renderEndpointId }),
    );
  } catch {
    // Local presentation persistence is best effort and never blocks routing.
  }
}

/**
 * Return a pair only when the read-only inventory contains one unambiguous
 * active VB-Cable capture/render endpoint.  Friendly names are used only as
 * a user-facing convenience; the returned values are still the exact stable
 * endpoint IDs sent to the backend.
 */
export function findVbCableEndpointPair(
  devices: DeviceListItem[],
): { captureEndpointId: string; renderEndpointId: string } | null {
  const isVbCable = (name: string) => {
    const normalized = name.toLocaleLowerCase();
    return (
      normalized.includes("vb-audio") ||
      normalized.includes("vb audio") ||
      normalized.includes("vb-cable") ||
      normalized.includes("vb cable")
    );
  };
  const capture = devices.filter((device) => device.id === findVbCableCaptureEndpointId(devices));
  const render = devices.filter(
    (device) =>
      device.state === "active" &&
      device.direction === "render" &&
      isVbCable(device.name) &&
      device.name.toLocaleLowerCase().includes("cable input"),
  );
  return capture.length === 1 && render.length === 1
    ? { captureEndpointId: capture[0].id, renderEndpointId: render[0].id }
    : null;
}

/** Return the unambiguous active VB-Cable capture endpoint, if present. */
export function findVbCableCaptureEndpointId(devices: DeviceListItem[]): string | null {
  const capture = devices.filter((device) => {
    const normalized = device.name.toLocaleLowerCase();
    return (
      device.state === "active" &&
      device.direction === "capture" &&
      (normalized.includes("vb-audio") ||
        normalized.includes("vb audio") ||
        normalized.includes("vb-cable") ||
        normalized.includes("vb cable")) &&
      normalized.includes("cable output")
    );
  });
  return capture.length === 1 ? capture[0].id : null;
}

export function sortDevicesAlphabetically<T extends DeviceListItem>(devices: T[]): T[] {
  return [...devices].sort(
    (left, right) =>
      left.name.localeCompare(right.name, undefined, { sensitivity: "base" }) || left.id.localeCompare(right.id),
  );
}

export function deviceChoiceLabel(device: Extract<DeviceListItem, { state: "active" }>): string {
  const channels =
    device.format.channels === 1
      ? "Mono · 1 channel"
      : device.format.channels === 2
        ? "Stereo · 2 channels"
        : `${device.format.channels}-channel multichannel`;
  return `${channels} · ${Math.round(device.format.sampleRateHz / 1000)} kHz — ${device.name}`;
}
