import type { ApplicationInfo, DeviceListItem, Session } from "@audiorouter/contracts";

/**
 * Which apps use which audio devices, from the backend's read-only session
 * inventory (`apps.list` `audioEndpoints`), and what that means for the
 * apps AudioRouter helps set up (DEC-22: Discord and OBS).
 */

export type KnownAppId = "discord" | "obs";

export const KNOWN_APPS: Record<KnownAppId, { label: string; monogram: string; executables: readonly string[] }> = {
  discord: {
    label: "Discord",
    monogram: "D",
    executables: ["discord.exe", "discordptb.exe", "discordcanary.exe"],
  },
  obs: { label: "OBS", monogram: "OBS", executables: ["obs64.exe"] },
};

export function knownAppOf(executable: string): KnownAppId | null {
  const name = executable.toLowerCase();
  for (const [id, app] of Object.entries(KNOWN_APPS) as [KnownAppId, (typeof KNOWN_APPS)[KnownAppId]][])
    if (app.executables.includes(name)) return id;
  return null;
}

/** A readable app name: the known label, else the executable without ".exe". */
export function appLabel(executable: string): string {
  const known = knownAppOf(executable);
  return known ? KNOWN_APPS[known].label : executable.replace(/\.exe$/i, "");
}

export interface DeviceUse {
  /** Lower-cased executable name; one entry per app, not per process. */
  executable: string;
  label: string;
  appId: KnownAppId | null;
  /**
   * "capture": the app records from this device. "render": the app plays to
   * it, or records what it plays (Windows reports a loopback recorder the
   * same way).
   */
  direction: "capture" | "render";
  active: boolean;
}

/**
 * Device ID to the apps using it. Several processes of one app (Discord runs
 * six) collapse into one entry; an active session anywhere wins. AudioRouter
 * itself is left out: its own sessions are the route, not an app to check.
 */
export function deviceUsage(applications: readonly ApplicationInfo[]): Map<string, DeviceUse[]> {
  const usage = new Map<string, DeviceUse[]>();
  for (const application of applications) {
    const executable = application.executable.toLowerCase();
    if (executable.startsWith("audiorouter")) continue;
    for (const endpoint of application.audioEndpoints ?? []) {
      const uses = usage.get(endpoint.deviceId) ?? [];
      const existing = uses.find((use) => use.executable === executable && use.direction === endpoint.direction);
      if (existing) existing.active ||= endpoint.active;
      else
        uses.push({
          executable,
          label: appLabel(application.executable),
          appId: knownAppOf(application.executable),
          direction: endpoint.direction,
          active: endpoint.active,
        });
      usage.set(endpoint.deviceId, uses);
    }
  }
  for (const uses of usage.values())
    uses.sort(
      (left, right) =>
        Number(right.appId !== null) - Number(left.appId !== null) || left.label.localeCompare(right.label),
    );
  return usage;
}

/**
 * The devices one app is using right now, split by direction. Only active
 * sessions count: Discord keeps idle sessions on devices it used before
 * (seen on the owner's PC, 2026-10-23), and judging those would raise false
 * warnings.
 */
export function appDevices(
  applications: readonly ApplicationInfo[],
  appId: KnownAppId,
): { running: boolean; capture: string[]; render: string[] } {
  const mine = applications.filter((application) => knownAppOf(application.executable) === appId);
  const pick = (direction: "capture" | "render") => [
    ...new Set(
      mine.flatMap((application) =>
        (application.audioEndpoints ?? [])
          .filter((endpoint) => endpoint.direction === direction && endpoint.active)
          .map((endpoint) => endpoint.deviceId),
      ),
    ),
  ];
  return { running: mine.length > 0, capture: pick("capture"), render: pick("render") };
}

const VB_CABLE = /\((VB-Audio[^)]*)\)\s*$/i;

/**
 * The other side of a VB-Cable: "CABLE-A Input (VB-Audio Virtual Cable A)"
 * (playback) pairs with "CABLE-A Output (VB-Audio Virtual Cable A)"
 * (recording). The "… 16ch" twins are separate endpoints of the same cable
 * and are never returned as a partner.
 */
export function cablePartner(device: DeviceListItem, devices: readonly DeviceListItem[]): DeviceListItem | null {
  const cable = VB_CABLE.exec(device.name)?.[1];
  if (!cable || /16ch/i.test(device.name)) return null;
  const want = device.direction === "render" ? "capture" : "render";
  const matches = devices.filter(
    (other) =>
      other.direction === want &&
      other.state === "active" &&
      !/16ch/i.test(other.name) &&
      VB_CABLE.exec(other.name)?.[1]?.toLowerCase() === cable.toLowerCase(),
  );
  return matches.length === 1 ? matches[0] : null;
}

/** The "… 16ch" endpoint of the same cable and direction, when Windows lists one. */
export function sixteenChannelTwin(device: DeviceListItem, devices: readonly DeviceListItem[]): DeviceListItem | null {
  const cable = VB_CABLE.exec(device.name)?.[1];
  if (!cable) return null;
  return (
    devices.find(
      (other) =>
        other.id !== device.id &&
        other.direction === device.direction &&
        VB_CABLE.exec(other.name)?.[1]?.toLowerCase() === cable.toLowerCase() &&
        /16ch/i.test(other.name) !== /16ch/i.test(device.name),
    ) ?? null
  );
}

function downstream(session: Session, start: readonly string[]): Set<string> {
  const reached = new Set(start);
  const queue = [...start];
  while (queue.length > 0) {
    const node = queue.shift()!;
    for (const edge of session.edges) {
      if (!edge.enabled || edge.sourceNode !== node || reached.has(edge.destinationNode)) continue;
      reached.add(edge.destinationNode);
      queue.push(edge.destinationNode);
    }
  }
  return reached;
}

function endpointOf(session: Session, nodeId: string): string | null {
  const value = session.nodes.find((node) => node.id === nodeId)?.parameters.endpointId;
  return typeof value === "string" && value ? value : null;
}

/** Playback devices a set of sources reaches through the session's enabled edges. */
function outputsReachedFrom(session: Session, sourceIds: readonly string[]): string[] {
  const reached = downstream(session, sourceIds);
  return [
    ...new Set(
      session.nodes
        .filter((node) => node.kind === "physicalOutput" && reached.has(node.id))
        .map((node) => endpointOf(session, node.id))
        .filter((id): id is string => id !== null),
    ),
  ];
}

/**
 * The recording devices that carry the session's processed microphone: the
 * cable partners of every playback device a microphone input reaches. This
 * is what Discord and OBS should use as their microphone.
 */
export function voiceCableOutputs(session: Session, devices: readonly DeviceListItem[]): DeviceListItem[] {
  return voiceCablesFrom(session, devices, microphoneNodes(session, devices));
}

/**
 * The session's microphones: inputs that are not a virtual cable. A cable's
 * Output brings another app's sound in (game audio, a call), so what it
 * reaches is not your voice.
 */
function microphoneNodes(session: Session, devices: readonly DeviceListItem[]): string[] {
  return session.nodes
    .filter((node) => {
      if (node.kind !== "physicalInput") return false;
      const endpoint = endpointOf(session, node.id);
      const name = endpoint ? nameOf(devices, endpoint) : null;
      return !(name && VB_CABLE.test(name));
    })
    .map((node) => node.id);
}

function voiceCablesFrom(
  session: Session,
  devices: readonly DeviceListItem[],
  microphones: readonly string[],
): DeviceListItem[] {
  return outputsReachedFrom(session, microphones)
    .map((id) => devices.find((device) => device.id === id))
    .filter((device): device is DeviceListItem => device !== undefined)
    .map((device) => cablePartner(device, devices))
    .filter((device): device is DeviceListItem => device !== null);
}

function capturesApp(session: Session, appId: KnownAppId): string[] {
  return session.nodes
    .filter(
      (node) =>
        node.kind === "applicationCapture" &&
        typeof node.parameters.executable === "string" &&
        knownAppOf(node.parameters.executable) === appId,
    )
    .map((node) => node.id);
}

/**
 * Whether the session is set up for Discord: it captures Discord, or it
 * sends the microphone to a virtual cable that Discord would use. This
 * decides whether the top-bar chip is shown, so it depends only on the
 * session the user edits, never on what is running (UI-17).
 */
export function sessionInvolvesDiscord(session: Session | null, devices: readonly DeviceListItem[]): boolean {
  if (!session) return false;
  return capturesApp(session, "discord").length > 0 || voiceCableOutputs(session, devices).length > 0;
}

export type AppCheckState = "ready" | "check" | "notRunning" | "unknown";

export interface AppCheckRow {
  id: "microphone" | "speaker";
  label: string;
  /** Device the app uses, or null when Windows reports none. */
  deviceName: string | null;
  state: "ready" | "check" | "info";
  message: string;
  /** Device name to suggest, for the "Copy device name" action. */
  expectedName?: string;
  /** A place in AudioRouter that fixes this row. */
  action?: "virtualCables";
}

export interface AppCheck {
  appId: KnownAppId;
  state: AppCheckState;
  summary: string;
  rows: AppCheckRow[];
}

const nameOf = (devices: readonly DeviceListItem[], id: string) =>
  devices.find((device) => device.id === id)?.name ?? null;

/**
 * Compare what Discord uses with what the session expects. Only facts
 * Windows reports are judged: an unknown setting is never shown as wrong.
 */
export function discordCheck(
  session: Session | null,
  applications: readonly ApplicationInfo[],
  devices: readonly DeviceListItem[],
): AppCheck {
  const use = appDevices(applications, "discord");
  // Always two rows, so the card keeps its size as Discord starts and stops (UI-17).
  if (!use.running)
    return {
      appId: "discord",
      state: "notRunning",
      summary: "Discord is not running. Start it to check its devices.",
      rows: [
        {
          id: "microphone",
          label: "Microphone",
          deviceName: null,
          state: "info",
          message: "Checked when Discord runs.",
        },
        { id: "speaker", label: "Speaker", deviceName: null, state: "info", message: "Checked when Discord runs." },
      ],
    };
  const rows: AppCheckRow[] = [];
  const voice = session ? voiceCableOutputs(session, devices) : [];
  const sessionInputs = session
    ? session.nodes
        .filter((node) => node.kind === "physicalInput")
        .map((node) => endpointOf(session, node.id))
        .filter((id): id is string => id !== null)
    : [];

  // Discord can record from more than one device for a moment (switching, a
  // mic test); judge the one that matches when any does.
  const microphone = use.capture.find((id) => voice.some((device) => device.id === id)) ?? use.capture[0] ?? null;
  const microphoneName = microphone ? nameOf(devices, microphone) : null;
  if (microphone === null) {
    rows.push({
      id: "microphone",
      label: "Microphone",
      deviceName: null,
      state: "info",
      message: "Discord is not recording yet. Join a voice channel to check its microphone.",
    });
  } else if (voice.some((device) => device.id === microphone)) {
    rows.push({
      id: "microphone",
      label: "Microphone",
      deviceName: microphoneName,
      state: "ready",
      message: "Carries your processed voice.",
    });
  } else if (voice.length > 0) {
    const expected = voice[0];
    const twin = sixteenChannelTwin(expected, devices);
    const usesRawMic = sessionInputs.includes(microphone);
    rows.push({
      id: "microphone",
      label: "Microphone",
      deviceName: microphoneName,
      state: "check",
      message:
        twin?.id === microphone
          ? `This is the 16-channel side of the same cable. Choose ${expected.name} instead.`
          : usesRawMic
            ? `This is your microphone without AudioRouter's effects. Choose ${expected.name} to send your processed voice.`
            : voice.length > 1
              ? `AudioRouter sends your processed voice to ${voice.map((device) => device.name).join(" and ")}. Choose one of them.`
              : `AudioRouter sends your processed voice to ${expected.name}.`,
      expectedName: expected.name,
    });
  } else {
    rows.push({
      id: "microphone",
      label: "Microphone",
      deviceName: microphoneName,
      state: "info",
      message:
        "This session does not send your voice to a virtual cable yet. Send it to a cable, then choose that cable as Discord's microphone.",
      action: "virtualCables",
    });
  }

  const speaker = use.render[0] ?? null;
  const speakerName = speaker ? nameOf(devices, speaker) : null;
  const captureNodes = session ? capturesApp(session, "discord") : [];
  const discordOutputs = session ? outputsReachedFrom(session, captureNodes) : [];
  const sessionOutputs = session
    ? session.nodes
        .filter((node) => node.kind === "physicalOutput")
        .map((node) => endpointOf(session, node.id))
        .filter((id): id is string => id !== null)
    : [];
  const speakerDevice = speaker ? devices.find((device) => device.id === speaker) : undefined;
  const busyTwin = speakerDevice
    ? sessionOutputs
        .map((id) => devices.find((device) => device.id === id))
        .find((device) => device && sixteenChannelTwin(device, devices)?.id === speaker)
    : undefined;
  const speakerPartner = speakerDevice ? cablePartner(speakerDevice, devices) : null;
  if (speaker === null) {
    rows.push({
      id: "speaker",
      label: "Speaker",
      deviceName: null,
      state: "info",
      message: "Discord is not playing sound yet.",
    });
  } else if (
    speakerPartner !== null &&
    (speakerPartner.id === microphone || voice.some((device) => device.id === speakerPartner.id))
  ) {
    // The call's audio would come back as Discord's own microphone.
    rows.push({
      id: "speaker",
      label: "Speaker",
      deviceName: speakerName,
      state: "check",
      message:
        "Discord plays into the cable that carries your voice, so people in your call hear themselves. Choose another output for Discord.",
    });
  } else if (busyTwin) {
    rows.push({
      id: "speaker",
      label: "Speaker",
      deviceName: speakerName,
      state: "check",
      message: `This is the 16-channel side of ${busyTwin.name}, which AudioRouter plays to. Windows lets only one of them be used, so choose ${busyTwin.name}.`,
      expectedName: busyTwin.name,
    });
  } else if (captureNodes.length > 0 && discordOutputs.includes(speaker)) {
    rows.push({
      id: "speaker",
      label: "Speaker",
      deviceName: speakerName,
      state: "check",
      message:
        "AudioRouter already sends Discord here, and Discord also plays here itself, so you hear it twice. Choose a virtual cable as Discord's output.",
    });
  } else {
    rows.push({
      id: "speaker",
      label: "Speaker",
      deviceName: speakerName,
      state: "ready",
      message: captureNodes.length > 0 ? "AudioRouter captures Discord from the app itself." : "Discord plays here.",
    });
  }

  const problems = rows.filter((row) => row.state === "check").length;
  const confirmed = rows.some((row) => row.state === "ready");
  return {
    appId: "discord",
    state: problems > 0 ? "check" : confirmed ? "ready" : "unknown",
    summary:
      problems > 0
        ? `${problems} setting${problems === 1 ? "" : "s"} to check`
        : confirmed
          ? "Ready"
          : "Waiting for Discord to use audio",
    rows,
  };
}

/** The check shown before the first answer arrives: the same two rows, so nothing moves. */
export function pendingDiscordCheck(): AppCheck {
  return {
    appId: "discord",
    state: "unknown",
    summary: "Checking which apps use your devices…",
    rows: [
      { id: "microphone", label: "Microphone", deviceName: null, state: "info", message: "Checking…" },
      { id: "speaker", label: "Speaker", deviceName: null, state: "info", message: "Checking…" },
    ],
  };
}

/** Settings AudioRouter's own tools replace; shown as advice, never read. */
export const DISCORD_RECOMMENDED_SETTINGS = [
  { setting: "Noise suppression", value: "None", why: "AudioRouter's Denoise already cleans your voice." },
  { setting: "Automatic gain control", value: "Off", why: "It fights the Compressor and changes your level." },
  { setting: "Echo cancellation", value: "Off", why: "Use headphones; it can cut your voice." },
] as const;
