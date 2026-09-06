export type SetupStep = { id: string; label: string; state: "ready" | "needs-attention" | "unavailable"; detail: string };

export function setupChecklist(input: { connected: boolean; audio: string | null; storage: string | null; deviceCount: number; applicationCount: number }): SetupStep[] {
  return [
    { id: "backend", label: "Control backend", state: input.connected ? "ready" : "unavailable", detail: input.connected ? "Connected" : "Connect the local backend to apply changes" },
    { id: "audio", label: "Audio capability", state: input.audio === "available" ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.audio ?? "Not available until a backend snapshot is received" },
    { id: "storage", label: "Configuration storage", state: input.storage === "available" ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.storage ?? "Not available until a backend snapshot is received" },
    { id: "devices", label: "Endpoint inventory", state: input.deviceCount > 0 ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.deviceCount > 0 ? `${input.deviceCount} endpoint descriptor${input.deviceCount === 1 ? "" : "s"} observed` : "No endpoint metadata observed; no endpoint is opened" },
    { id: "applications", label: "Application selection", state: input.applicationCount > 0 ? "ready" : input.connected ? "needs-attention" : "unavailable", detail: input.applicationCount > 0 ? `${input.applicationCount} application observation${input.applicationCount === 1 ? "" : "s"} available` : "Select Discord/OBS in their own settings when needed" },
  ];
}
