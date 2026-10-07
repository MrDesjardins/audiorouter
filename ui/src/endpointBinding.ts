// Remembered endpoint bindings and device-choice helpers (moved from App.tsx).
import type { DeviceListItem } from "@audiorouter/contracts";

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
