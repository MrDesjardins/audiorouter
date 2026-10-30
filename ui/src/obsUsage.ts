import type {
  ApplicationInfo,
  DeviceListItem,
  NodeKind,
  ObsAudioInput,
  ObsChangeRequest,
  ObsStatus,
  Session,
} from "@audiorouter/contracts";
import {
  appDevices,
  appLabel,
  cablePartner,
  endpointOf,
  microphoneNodes,
  outputsReachedFrom,
  sixteenChannelTwin,
  voiceCableOutputs,
  type AppCheck,
  type AppCheckState,
} from "./appUsage";

/**
 * What OBS records, judged against the session (DEC-22). The backend reports
 * OBS's audio sources as facts (`obs.status`); this module decides what each
 * source hears and which of them would put the voice on the stream wrongly.
 */

export interface ObsSourceRow {
  name: string;
  /** The device as Windows names it, "Default (…)", or the application. */
  device: string;
  /** What reaches the source through the session: "Voice", app names, sources. */
  hears: string[];
  muted: boolean;
  problem: string | null;
  /** The change "Fix in OBS" makes, and its plain description. */
  fix: ObsChangeRequest | null;
  fixLabel: string | null;
}

export type ObsView =
  /** OBS is not installed on this PC. */
  | "notInstalled"
  /** OBS is installed but closed: its last saved setup is shown. */
  | "closed"
  /** OBS runs; AudioRouter is not connected to it. */
  | "connect"
  | "connected";

export interface ObsCheck extends AppCheck {
  view: ObsView;
  sources: ObsSourceRow[];
}

const SOURCE_KINDS: readonly NodeKind[] = [
  "physicalInput",
  "applicationCapture",
  "endpointLoopback",
  "audioFile",
  "soundboard",
  "testSignal",
  "networkReceive",
  "virtualRenderSource",
];

/** OBS uses the communications default for capture and console for playback. */
function resolveDevice(
  input: ObsAudioInput,
  devices: readonly DeviceListItem[],
): { device: DeviceListItem | null; isDefault: boolean } {
  const direction = input.kind === "inputCapture" ? "capture" : "render";
  const role = direction === "capture" ? "communications" : "console";
  if (input.deviceId === "default")
    return {
      device: devices.find((device) => device.direction === direction && device.defaultRoles.includes(role)) ?? null,
      isDefault: true,
    };
  return { device: devices.find((device) => device.id === input.deviceId) ?? null, isDefault: false };
}

function deviceText(input: ObsAudioInput, device: DeviceListItem | null, isDefault: boolean): string {
  if (input.kind === "applicationCapture") return input.application ? appLabel(input.application) : "An application";
  if (isDefault) return device ? `Default (${device.name})` : "Default device";
  return device?.name ?? "A device that is not connected";
}

/** Labels of the session's sources that reach a playback device. */
function sourcesReaching(session: Session, renderId: string, microphones: readonly string[]): string[] {
  const labels = session.nodes
    .filter((node) => SOURCE_KINDS.includes(node.kind) && outputsReachedFrom(session, [node.id]).includes(renderId))
    .map((node) => {
      if (microphones.includes(node.id)) return "Voice";
      if (node.kind === "applicationCapture" && typeof node.parameters.executable === "string")
        return appLabel(node.parameters.executable);
      return node.name;
    });
  return [...new Set(labels)];
}

/** Initial state before the first `obs.status` answer: the same layout, nothing judged. */
export function pendingObsCheck(): ObsCheck {
  return {
    appId: "obs",
    state: "unknown",
    summary: "Checking OBS…",
    rows: [],
    view: "closed",
    sources: [],
  };
}

export function obsCheck(
  session: Session | null,
  status: ObsStatus | null,
  applications: readonly ApplicationInfo[],
  devices: readonly DeviceListItem[],
  stale = false,
): ObsCheck {
  if (!status) return pendingObsCheck();
  const running = appDevices(applications, "obs").running;
  const view: ObsView =
    status.connection === "connected"
      ? "connected"
      : !status.installed
        ? "notInstalled"
        : !running
          ? "closed"
          : "connect";
  const voice = session ? voiceCableOutputs(session, devices) : [];
  const microphones = session ? microphoneNodes(session, devices) : [];
  const microphoneIds = session
    ? microphones.map((id) => endpointOf(session, id)).filter((id): id is string => id !== null)
    : [];
  const voiceRenders = session ? outputsReachedFrom(session, microphones) : [];
  const sessionOutputs = session
    ? session.nodes
        .filter((node) => node.kind === "physicalOutput")
        .map((node) => endpointOf(session, node.id))
        .filter((id): id is string => id !== null)
    : [];
  const canFix = !stale && view === "connected" && status.source === "live" && status.sceneCollection !== null;

  const carriesVoice = new Set<string>();
  const sources: ObsSourceRow[] = status.inputs.map((input) => {
    const { device, isDefault } = resolveDevice(input, devices);
    const row: ObsSourceRow = {
      name: input.name,
      device: deviceText(input, device, isDefault),
      hears: [],
      muted: input.muted === true,
      problem: null,
      fix: null,
      fixLabel: null,
    };
    if (!session || input.kind === "applicationCapture" || !device) return row;
    if (input.kind === "inputCapture") {
      const partner = cablePartner(device, devices);
      if (voice.some((cable) => cable.id === device.id)) {
        row.hears = partner ? sourcesReaching(session, partner.id, microphones) : ["Voice"];
        if (!row.hears.includes("Voice")) row.hears.unshift("Voice");
      } else if (microphoneIds.includes(device.id)) {
        row.hears = ["Voice without effects"];
        if (voice.length > 0) {
          row.problem = "Records your microphone without AudioRouter's effects.";
          row.fix = { inputName: input.name, deviceId: voice[0].id };
          row.fixLabel = `${input.name}: ${device.name} → ${voice[0].name}`;
        }
      } else if (partner) {
        row.hears = sourcesReaching(session, partner.id, microphones);
      } else {
        const cable = voice.find((item) => sixteenChannelTwin(item, devices)?.id === device.id);
        if (cable) {
          row.problem =
            "Records the 16-channel side of your voice cable. Windows lets only one side be used at a time.";
          row.fix = { inputName: input.name, deviceId: cable.id };
          row.fixLabel = `${input.name}: ${device.name} → ${cable.name}`;
        }
      }
    } else {
      row.hears = sourcesReaching(session, device.id, microphones);
      const busy = sessionOutputs
        .map((id) => devices.find((item) => item.id === id))
        .find((item) => item && sixteenChannelTwin(item, devices)?.id === device.id);
      if (busy) {
        row.problem = `Records the 16-channel side of ${busy.name}, which AudioRouter plays to. Windows lets only one side be used at a time.`;
        row.fix = { inputName: input.name, deviceId: busy.id };
        row.fixLabel = `${input.name}: ${device.name} → ${busy.name}`;
      }
    }
    // A saved source or an unmuted source in another scene is not evidence
    // that the voice reaches any OBS output now. Missing facts stay unknown.
    if (
      status.source === "live" &&
      input.active === true &&
      input.muted === false &&
      input.monitorOnly === false &&
      input.audioTracks &&
      input.audioTracks.length > 0 &&
      (row.hears.includes("Voice") || voiceRenders.includes(device.id))
    )
      carriesVoice.add(input.name);
    return row;
  });

  // Confirmed sources can duplicate voice only on a shared OBS audio track.
  // The streaming/recording profile's selected tracks are not inferred here.
  if (session && carriesVoice.size > 1) {
    // A playback device the session feeds without the voice (game, chat).
    const elsewhere = sessionOutputs.find(
      (id) => !voiceRenders.includes(id) && sourcesReaching(session, id, microphones).length > 0,
    );
    for (const row of sources) {
      const input = status.inputs.find((item) => item.name === row.name);
      if (!input || input.kind !== "outputCapture" || !carriesVoice.has(row.name) || row.problem) continue;
      const overlaps = status.inputs.some(
        (other) =>
          other.name !== input.name &&
          carriesVoice.has(other.name) &&
          other.audioTracks?.some((track) => input.audioTracks?.includes(track)),
      );
      if (!overlaps) continue;
      row.problem = "Also plays your voice, so your voice is recorded twice on the same OBS audio track.";
      const target = elsewhere ? devices.find((device) => device.id === elsewhere) : undefined;
      if (target) {
        row.fix = { inputName: row.name, deviceId: target.id };
        row.fixLabel = `${row.name}: ${row.device} → ${target.name}`;
      }
    }
  }
  for (const row of sources) {
    // Older OBS servers may omit a source UUID. Keep those sources read-only.
    if (!canFix || !status.inputs.find((input) => input.name === row.name)?.uuid) {
      row.fix = null;
      row.fixLabel = null;
    }
  }

  const problems = sources.filter((row) => row.problem !== null).length;
  const state: AppCheckState = stale
    ? "unknown"
    : view === "notInstalled" || view === "closed"
      ? "notRunning"
      : problems > 0
        ? "check"
        : carriesVoice.size > 0
          ? "ready"
          : "unknown";
  const summary = stale
    ? "Could not refresh OBS; showing the last check."
    : view === "notInstalled"
      ? "OBS Studio is not installed on this PC."
      : view === "closed"
        ? status.source === "savedFile"
          ? `OBS is closed. This is the setup it saved last${problems > 0 ? `, with ${problems} setting${problems === 1 ? "" : "s"} to check` : ""}.`
          : "OBS is closed. Start it to check its audio sources."
        : problems > 0
          ? `${problems} setting${problems === 1 ? "" : "s"} to check`
          : carriesVoice.size > 0
            ? "Ready"
            : view === "connect"
              ? "Connect to check OBS's audio sources."
              : sources.some((row) => row.hears.includes("Voice"))
                ? "Voice source found. Check its scene, mute, monitoring and audio tracks in OBS."
                : voice.length > 0
                  ? `No OBS source records your processed voice. Add an Audio Input Capture with ${voice[0].name}.`
                  : "No OBS source records your voice.";
  return { appId: "obs", state, summary, rows: [], view, sources };
}

/** The fixes the "Fix in OBS" dialog offers, one per source. */
export function obsFixes(check: ObsCheck): { change: ObsChangeRequest; label: string; reason: string }[] {
  return check.sources.flatMap((row) =>
    row.fix && row.fixLabel ? [{ change: row.fix, label: row.fixLabel, reason: row.problem ?? "" }] : [],
  );
}
