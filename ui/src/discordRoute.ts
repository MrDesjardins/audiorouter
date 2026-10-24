import type { DeviceListItem, Node, Session } from "@audiorouter/contracts";
import { cablePartner, endpointOf, isVirtualCableName, voiceCableOutputs } from "./appUsage";
import { appendDraftConnection, appendLibraryNode, insertDraftProcessor, type InsertableProcessorKind } from "./draft";

/**
 * "Set up for Discord" (D2): one undoable draft change that sends a
 * microphone to a virtual cable, whose other end Discord then records.
 * Discord itself is only read, never changed (DEC-22).
 */

export interface VoiceCableChoice {
  /** The playback end AudioRouter plays the voice into ("CABLE-A Input"). */
  input: DeviceListItem;
  /** The recording end Discord uses as its microphone ("CABLE-A Output"). */
  output: DeviceListItem;
  /** What already uses the cable in this session, which makes it a poor choice; null when free. */
  busy: string | null;
}

export interface DiscordSetupOptions {
  microphones: DeviceListItem[];
  cables: VoiceCableChoice[];
  /** Preselected microphone and cable input, or null when there is none. */
  microphone: string | null;
  cable: string | null;
}

/** The voice cleanup chain offered as an option, in signal order. */
export const VOICE_CLEANUP: readonly InsertableProcessorKind[] = ["gate", "compressor", "limiter"];

const active = (devices: readonly DeviceListItem[], direction: DeviceListItem["direction"]) =>
  devices.filter((device) => device.state === "active" && device.direction === direction);

const boundNodes = (session: Session, kind: Node["kind"], endpointId: string) =>
  session.nodes.filter((node) => node.kind === kind && endpointOf(session, node.id) === endpointId);

/** What feeds a playback node's input, as a readable name; null when nothing does. */
function feederOf(session: Session, nodeId: string): string | null {
  const edge = session.edges.find((candidate) => candidate.destinationNode === nodeId);
  return edge ? (session.nodes.find((node) => node.id === edge.sourceNode)?.name ?? edge.sourceNode) : null;
}

/**
 * The microphones and voice cables to offer, with sensible choices made:
 * the session's own microphone, else Windows' default for calls; the first
 * cable this session does not use yet.
 */
export function discordSetupOptions(session: Session, devices: readonly DeviceListItem[]): DiscordSetupOptions {
  const microphones = active(devices, "capture").filter((device) => !isVirtualCableName(device.name));
  const cables = active(devices, "render")
    .map((input) => ({ input, output: cablePartner(input, devices) }))
    .filter((pair): pair is { input: DeviceListItem; output: DeviceListItem } => pair.output !== null)
    .map(({ input, output }): VoiceCableChoice => {
      const players = boundNodes(session, "physicalOutput", input.id);
      const recorders = boundNodes(session, "physicalInput", output.id);
      const fed = players.map((node) => feederOf(session, node.id)).find((name) => name !== null);
      return {
        input,
        output,
        busy: fed
          ? `${fed} plays into it`
          : recorders.length > 0
            ? `${recorders[0].name} records from it`
            : players.length > 0
              ? `${players[0].name} uses it`
              : null,
      };
    });
  const sessionMicrophone = session.nodes
    .filter((node) => node.kind === "physicalInput")
    .map((node) => endpointOf(session, node.id))
    .find((id) => id !== null && microphones.some((device) => device.id === id));
  const microphone =
    sessionMicrophone ??
    microphones.find((device) => device.defaultRoles.includes("communications"))?.id ??
    microphones.find((device) => device.defaultRoles.includes("console"))?.id ??
    microphones[0]?.id ??
    null;
  const cable = (cables.find((choice) => choice.busy === null) ?? cables[0])?.input.id ?? null;
  return { microphones, cables, microphone, cable };
}

export interface DiscordSetupResult {
  session: Session;
  /** The cable's recording end: what Discord should use as its microphone. */
  discordMicrophone: DeviceListItem;
}

/**
 * Adds microphone → (cleanup) → voice cable to the draft, reusing the
 * session's nodes for those devices when they are free. Throws with a
 * readable message when the route cannot be added as asked.
 */
export function appendDiscordRoute(
  session: Session,
  devices: readonly DeviceListItem[],
  choice: { microphone: string; cable: string; cleanup: boolean },
): DiscordSetupResult {
  const microphone = devices.find((device) => device.id === choice.microphone);
  const input = devices.find((device) => device.id === choice.cable);
  const output = input ? cablePartner(input, devices) : null;
  if (!microphone) throw new Error("Choose a microphone.");
  if (!input || !output) throw new Error("Choose a virtual cable.");
  if (voiceCableOutputs(session, devices).some((device) => device.id === output.id))
    throw new Error(`This session already sends your voice to ${input.name}.`);

  let next = session;
  const add = (kind: "physicalInput" | "physicalOutput", name: string, endpointId: string) => {
    next = appendLibraryNode(next, kind);
    const node = next.nodes.at(-1)!;
    next = {
      ...next,
      nodes: next.nodes.map((candidate) =>
        candidate.id === node.id
          ? { ...candidate, name, parameters: { ...candidate.parameters, endpointId } }
          : candidate,
      ),
    };
    return node.id;
  };
  const microphoneNode =
    boundNodes(session, "physicalInput", microphone.id).find((node) => node.enabled)?.id ??
    add("physicalInput", "Microphone", microphone.id);
  const players = boundNodes(session, "physicalOutput", input.id);
  const taken = players.map((node) => feederOf(session, node.id)).find((name) => name !== null);
  if (taken) throw new Error(`${taken} already plays into ${input.name}. Choose another cable.`);
  const cableNode = players.find((node) => node.enabled)?.id ?? add("physicalOutput", "Voice to Discord", input.id);

  next = appendDraftConnection(next, microphoneNode, "out", cableNode, "in");
  if (choice.cleanup) {
    let edge = next.edges.at(-1)!.id;
    for (const kind of VOICE_CLEANUP) {
      next = insertDraftProcessor(next, edge, kind);
      const processor = next.nodes.at(-1)!.id;
      edge = next.edges.find(
        (candidate) => candidate.sourceNode === processor && candidate.destinationNode === cableNode,
      )!.id;
    }
  }
  return { session: next, discordMicrophone: output };
}
