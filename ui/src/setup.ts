export type SetupStep = { id: string; label: string; state: "ready" | "needs-attention" | "unavailable"; detail: string };

export function setupChecklist(input: { connected: boolean; audio: string | null; storage: string | null; deviceCount: number; applicationCount: number; vbCablePairAvailable: boolean }): SetupStep[] {
  return [
    { id: "backend", label: "Background service", state: input.connected ? "ready" : "unavailable", detail: input.connected ? "Connected" : "Not connected; changes cannot be applied" },
    { id: "audio", label: "Audio engine", state: input.audio === "available" ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.audio ?? "Not available until the service reports its status" },
    { id: "storage", label: "Saved settings", state: input.storage !== null ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.storage === "memory" ? "In-memory storage; persistence is not durable" : input.storage === "sqlite" ? "Sessions are saved on this PC" : "Not available until the service reports its status" },
    { id: "devices", label: "Audio devices", state: input.deviceCount > 0 ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.deviceCount > 0 ? `${input.deviceCount} device${input.deviceCount === 1 ? "" : "s"} found (listed below)` : "No devices found yet; press Refresh below" },
    { id: "vb-cable", label: "Virtual cable (optional)", state: input.vbCablePairAvailable ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.vbCablePairAvailable ? "VB-Cable found: send a route to CABLE Input, then pick CABLE Output as the microphone in Discord, OBS or a game" : input.connected ? "No VB-Cable found. Install one only if other apps should hear AudioRouter's sound" : "Unavailable until the device list is received" },
    { id: "applications", label: "Apps playing sound", state: input.applicationCount > 0 ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.applicationCount > 0 ? `${input.applicationCount} app${input.applicationCount === 1 ? "" : "s"} can be captured with Application source` : "Select Discord/OBS in their own settings when needed" },
  ];
}
