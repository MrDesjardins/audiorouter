import { tx } from "./i18n";

export type SetupStep = {
  id: string;
  label: string;
  state: "ready" | "idle" | "needs-attention" | "unavailable";
  detail: string;
};

export function setupChecklist(input: {
  connected: boolean;
  audio: string | null;
  activeSessionCount?: number | null;
  audioReason?: string | null;
  storage: string | null;
  deviceCount: number;
  applicationCount: number;
  vbCablePairAvailable: boolean;
}): SetupStep[] {
  return [
    {
      id: "backend",
      label: tx("h3.setBackend"),
      state: input.connected ? "ready" : "unavailable",
      detail: input.connected ? tx("h3.setConnected") : tx("h3.setNotConnected"),
    },
    {
      id: "audio",
      label: tx("h3.setAudio"),
      state: !input.connected
        ? "unavailable"
        : input.audio === "available"
          ? "ready"
          : input.activeSessionCount === 0
            ? "idle"
            : "needs-attention",
      detail: !input.connected
        ? tx("h3.setAudioNoService")
        : input.audio === "available"
          ? tx("h3.setRunning")
          : input.activeSessionCount === 0
            ? tx("h3.setStopped")
            : (input.audioReason ?? tx("h3.setAudioWaiting")),
    },
    {
      id: "storage",
      label: tx("h3.setStorage"),
      state: input.storage !== null ? "ready" : input.connected ? "needs-attention" : "unavailable",
      detail:
        input.storage === "memory"
          ? tx("h3.setMemory")
          : input.storage === "sqlite"
            ? tx("h3.setSqlite")
            : tx("h3.setStorageNone"),
    },
    {
      id: "devices",
      label: tx("h3.setDevices"),
      state: input.deviceCount > 0 ? "ready" : input.connected ? "needs-attention" : "unavailable",
      detail:
        input.deviceCount > 0
          ? tx(input.deviceCount === 1 ? "h3.setDevicesOne" : "h3.setDevicesMany", { count: input.deviceCount })
          : tx("h3.setDevicesNone"),
    },
    {
      id: "vb-cable",
      label: tx("h3.setCable"),
      state: input.vbCablePairAvailable ? "ready" : input.connected ? "needs-attention" : "unavailable",
      detail: input.vbCablePairAvailable
        ? tx("h3.setCableFound")
        : input.connected
          ? tx("h3.setCableNone")
          : tx("h3.setCableWait"),
    },
    {
      id: "applications",
      label: tx("h3.setApps"),
      state: input.applicationCount > 0 ? "ready" : input.connected ? "needs-attention" : "unavailable",
      detail:
        input.applicationCount > 0
          ? tx(input.applicationCount === 1 ? "h3.setAppsOne" : "h3.setAppsMany", { count: input.applicationCount })
          : tx("h3.setAppsNone"),
    },
  ];
}
