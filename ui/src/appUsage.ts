import type { ApplicationInfo, DeviceListItem, Node, Session } from "@audiorouter/contracts";

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
 * The devices one app is using right now, split by direction, and the
 * devices it only keeps idle sessions on. Discord keeps idle sessions on
 * devices it used before (seen on the owner's PC, 2026-10-23): out of a
 * call, its microphone and speaker are idle, but so are older devices. Idle
 * devices may confirm a setting; judging them as wrong would raise false
 * warnings.
 */
export function appDevices(
  applications: readonly ApplicationInfo[],
  appId: KnownAppId,
): { running: boolean; capture: string[]; render: string[]; idleCapture: string[]; idleRender: string[] } {
  const mine = applications.filter((application) => knownAppOf(application.executable) === appId);
  const pick = (direction: "capture" | "render", active: boolean) => [
    ...new Set(
      mine.flatMap((application) =>
        (application.audioEndpoints ?? [])
          .filter((endpoint) => endpoint.direction === direction && endpoint.active === active)
          .map((endpoint) => endpoint.deviceId),
      ),
    ),
  ];
  const capture = pick("capture", true);
  const render = pick("render", true);
  return {
    running: mine.length > 0,
    capture,
    render,
    idleCapture: pick("capture", false).filter((id) => !capture.includes(id)),
    idleRender: pick("render", false).filter((id) => !render.includes(id)),
  };
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

const names = (uses: readonly DeviceUse[]) => uses.map((use) => use.label).join(" and ");

/** One app on a device node's device, or on the other end of its cable. */
export interface NodeAppUse {
  label: string;
  appId: KnownAppId | null;
  /** How the app relates to the audio this node carries, in plain words. */
  role: string;
}

/** What a Physical Input or Output node's footer and Properties show. */
export interface NodeDeviceUsers {
  /**
   * One line for the canvas footer, for example "Used by Discord · OBS", or
   * the warning's short form ("Discord is on CABLE-A Out 16ch").
   */
  summary: string;
  /** Every app with its role, for the tooltip and Properties. */
  uses: NodeAppUse[];
  /** An app on a sibling device it should not use, or null. */
  warning: string | null;
}

/**
 * The apps using each device node's device right now (D1). A node that plays
 * into a virtual cable also lists the apps recording from the cable's other
 * end: that is how Discord hears what the node sends. Only active sessions
 * count; idle ones linger on devices an app used before (see `appDevices`).
 */
export function nodeDeviceUsers(
  session: Session,
  devices: readonly DeviceListItem[],
  applications: readonly ApplicationInfo[],
): Map<string, NodeDeviceUsers> {
  const usage = deviceUsage(applications);
  const active = (deviceId: string, direction: "capture" | "render") =>
    (usage.get(deviceId) ?? []).filter((use) => use.active && use.direction === direction);
  const result = new Map<string, NodeDeviceUsers>();
  const microphones = microphoneNodes(session, devices);
  const voiceCables = voiceCableOutputs(session, devices);
  for (const node of session.nodes) {
    if (node.kind !== "physicalInput" && node.kind !== "physicalOutput") continue;
    const endpointId = endpointOf(session, node.id);
    const device = endpointId ? devices.find((item) => item.id === endpointId) : undefined;
    if (!endpointId) {
      result.set(node.id, { summary: "No device chosen", uses: [], warning: null });
      continue;
    }
    const output = node.kind === "physicalOutput";
    const uses: NodeAppUse[] = (output ? active(endpointId, "render") : active(endpointId, "capture")).map((use) => ({
      label: use.label,
      appId: use.appId,
      // Windows reports a loopback recorder like a player (spec CAP).
      role: output ? "plays to it or records it" : "records from it",
    }));
    const partner = device ? cablePartner(device, devices) : null;
    let warning: string | null = null;
    // The footer's short form of the warning: one line on a narrow card.
    let alert: string | null = null;
    if (partner) {
      for (const use of output ? active(partner.id, "capture") : active(partner.id, "render"))
        uses.push({
          label: use.label,
          appId: use.appId,
          role: output ? `records from ${partner.name}` : `plays into ${partner.name}`,
        });
      const twin = output ? sixteenChannelTwin(partner, devices) : null;
      const astray = twin ? active(twin.id, "capture").filter((use) => use.appId !== null) : [];
      if (twin && astray.length > 0) {
        warning = `${names(astray)} ${astray.length === 1 ? "records" : "record"} from ${twin.name}, not ${partner.name}`;
        alert = `${names(astray)} ${astray.length === 1 ? "is" : "are"} on ${twin.name.replace(VB_CABLE, "").trim()}`;
      }
    }
    // Discord or OBS on the raw microphone skips the processing this session
    // sends to its voice cable.
    if (!output && microphones.includes(node.id) && voiceCables.length > 0) {
      const raw = active(endpointId, "capture").filter((use) => use.appId !== null);
      if (raw.length > 0) {
        warning = `${names(raw)} ${raw.length === 1 ? "records" : "record"} this microphone without your processing; choose ${voiceCables.map((cable) => cable.name).join(" or ")} instead`;
        alert = `${names(raw)} ${raw.length === 1 ? "skips" : "skip"} your processing`;
      }
    }
    const labels = [...new Set(uses.map((use) => use.label))];
    result.set(node.id, {
      summary: alert ?? (labels.length > 0 ? `Used by ${labels.join(" · ")}` : "No app using it"),
      uses,
      warning,
    });
  }
  return result;
}

const SOURCES_AND_SILENT_BYPASSES: readonly Node["kind"][] = [
  "physicalInput",
  "applicationCapture",
  "endpointLoopback",
  "audioFile",
  "testSignal",
  "networkReceive",
  "virtualRenderSource",
  "physicalOutput",
  "virtualCaptureSink",
  "mixer",
  "inputSwitch",
  "outputSwitch",
];

/** Steady-state signal reachability, not simply the saved wire topology.
 * Ordinary disabled effects pass dry; sources, sinks and mixers silence.
 * The backend remains authoritative for runtime readiness and failures.
 */
function passesSignal(node: Node): boolean {
  if (SOURCES_AND_SILENT_BYPASSES.includes(node.kind) && (!node.enabled || node.bypass)) return false;
  return node.kind !== "mute" || !node.enabled || node.bypass || node.parameters.muted !== true;
}

function downstream(session: Session, start: readonly string[]): Set<string> {
  const nodes = new Map(session.nodes.map((node) => [node.id, node]));
  const reached = new Set(
    start.filter((id) => {
      const node = nodes.get(id);
      return node && passesSignal(node);
    }),
  );
  const queue = [...reached];
  while (queue.length > 0) {
    const id = queue.shift()!;
    const source = nodes.get(id)!;
    for (const edge of session.edges) {
      if (!edge.enabled || edge.sourceNode !== id || reached.has(edge.destinationNode)) continue;
      const destination = nodes.get(edge.destinationNode);
      if (!destination || !passesSignal(destination)) continue;
      if (source.kind === "outputSwitch" && edge.sourcePort !== (source.parameters.selected ?? "a")) continue;
      if (destination.kind === "inputSwitch" && edge.destinationPort !== (destination.parameters.selected ?? "a"))
        continue;
      if (destination.kind === "mixer" && destination.parameters[`inputVolume:${id}`] === 0) continue;
      reached.add(edge.destinationNode);
      queue.push(edge.destinationNode);
    }
  }
  return reached;
}

export function endpointOf(session: Session, nodeId: string): string | null {
  const value = session.nodes.find((node) => node.id === nodeId)?.parameters.endpointId;
  return typeof value === "string" && value ? value : null;
}

/** Playback devices a set of sources reaches through the session's enabled edges. */
export function outputsReachedFrom(session: Session, sourceIds: readonly string[]): string[] {
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
export function microphoneNodes(session: Session, devices: readonly DeviceListItem[]): string[] {
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
  stale = false,
): AppCheck {
  if (stale) {
    const previous = discordCheck(session, applications, devices);
    return { ...previous, state: "unknown", summary: "Could not refresh Discord; showing the last check." };
  }
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
  // Out of a call, an idle voice cable confirms the last call's microphone,
  // but only when every idle microphone is one: a stale raw-mic session could
  // be the newer choice, and Windows does not say which came last.
  const lastMicrophone =
    use.idleCapture.length > 0 && use.idleCapture.every((id) => voice.some((device) => device.id === id))
      ? use.idleCapture[0]
      : null;
  if (microphone === null && lastMicrophone !== null) {
    rows.push({
      id: "microphone",
      label: "Microphone",
      deviceName: nameOf(devices, lastMicrophone),
      state: "ready",
      message: `Carries your processed voice. ${LAST_CALL}`,
    });
  } else if (microphone === null) {
    rows.push({
      id: "microphone",
      label: "Microphone",
      deviceName: null,
      state: "info",
      message: "Checked when you join a voice channel in Discord.",
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

  const captureNodes = session ? capturesApp(session, "discord") : [];
  const discordOutputs = session ? outputsReachedFrom(session, captureNodes) : [];
  const sessionOutputs = session
    ? session.nodes
        .filter((node) => node.kind === "physicalOutput")
        .map((node) => endpointOf(session, node.id))
        .filter((id): id is string => id !== null)
    : [];
  const judgeSpeaker = (speaker: string): AppCheckRow => {
    const speakerName = nameOf(devices, speaker);
    const speakerDevice = devices.find((device) => device.id === speaker);
    const busyTwin = speakerDevice
      ? sessionOutputs
          .map((id) => devices.find((device) => device.id === id))
          .find((device) => device && sixteenChannelTwin(device, devices)?.id === speaker)
      : undefined;
    const speakerPartner = speakerDevice ? cablePartner(speakerDevice, devices) : null;
    const base = { id: "speaker", label: "Speaker", deviceName: speakerName } as const;
    if (
      speakerPartner !== null &&
      (speakerPartner.id === (microphone ?? lastMicrophone) || voice.some((device) => device.id === speakerPartner.id))
    )
      // The call's audio would come back as Discord's own microphone.
      return {
        ...base,
        state: "check",
        message:
          "Discord plays into the cable that carries your voice, so people in your call hear themselves. Choose another output for Discord.",
      };
    if (busyTwin)
      return {
        ...base,
        state: "check",
        message: `This is the 16-channel side of ${busyTwin.name}, which AudioRouter plays to. Windows lets only one of them be used, so choose ${busyTwin.name}.`,
        expectedName: busyTwin.name,
      };
    if (captureNodes.length > 0 && discordOutputs.includes(speaker))
      return {
        ...base,
        state: "check",
        message:
          "AudioRouter already sends Discord here, and Discord also plays here itself, so you hear it twice. Choose a virtual cable as Discord's output.",
      };
    return {
      ...base,
      state: "ready",
      message: captureNodes.length > 0 ? "AudioRouter captures Discord from the app itself." : "Discord plays here.",
    };
  };
  if (use.render.length > 0) rows.push(judgeSpeaker(use.render[0]));
  else {
    // Out of a call, trust a single idle output only when it would be ready;
    // Discord also leaves idle sessions on outputs it no longer uses.
    const last = use.idleRender.length === 1 ? judgeSpeaker(use.idleRender[0]) : null;
    rows.push(
      last?.state === "ready"
        ? { ...last, message: `${last.message} ${LAST_CALL}` }
        : {
            id: "speaker",
            label: "Speaker",
            deviceName: null,
            state: "info",
            message: "Checked when you join a voice channel in Discord.",
          },
    );
  }

  const problems = rows.filter((row) => row.state === "check").length;
  // A working speaker alone cannot confirm that the processed microphone
  // reaches Discord (for example when its cable output is disabled).
  const confirmed = rows.some((row) => row.id === "microphone" && row.state === "ready");
  const inCall = use.capture.length > 0;
  return {
    appId: "discord",
    state: problems > 0 ? "check" : confirmed ? "ready" : "unknown",
    summary:
      problems > 0
        ? `${problems} setting${problems === 1 ? "" : "s"} to check`
        : confirmed
          ? inCall
            ? "Ready"
            : "Ready, from Discord's last call. Join a voice channel to check again."
          : inCall
            ? "Waiting for your voice to reach a virtual cable"
            : "Waiting for you to join a voice channel in Discord",
    rows,
  };
}

/** Added to a row judged from an idle session, outside a call. */
const LAST_CALL = "Seen in Discord's last call.";

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
  { setting: "Noise suppression", value: "None if using Denoise", why: "Avoid cleaning the same microphone twice." },
  {
    setting: "Automatic gain control",
    value: "Off if using Compressor",
    why: "Two level controls can fight each other.",
  },
  {
    setting: "Echo cancellation",
    value: "Off with headphones",
    why: "Keep echo cancellation if your speakers can feed back into the microphone.",
  },
] as const;
