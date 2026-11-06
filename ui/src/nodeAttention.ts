import type { Node } from "@audiorouter/contracts";

/** Sources that privacy mute silences. Tones, files and the Soundboard keep playing. */
export const PRIVACY_MUTED_KINDS: readonly string[] = [
  "physicalInput",
  "applicationCapture",
  "endpointLoopback",
  "virtualRenderSource",
];

export type NodeAttention = "no device" | "no input" | "muted";

/**
 * What stops this tool from doing its job right now, or null when nothing does.
 * Both status chips read this, so a card can never say "ready" beside "No device chosen".
 */
export function nodeAttention(
  node: Node,
  { unfed, privacyMuted }: { unfed: boolean; privacyMuted: boolean },
): NodeAttention | null {
  if (
    (node.kind === "physicalInput" || node.kind === "physicalOutput") &&
    typeof node.parameters.endpointId !== "string"
  )
    return "no device";
  if (unfed) return "no input";
  if (privacyMuted && PRIVACY_MUTED_KINDS.includes(node.kind)) return "muted";
  return null;
}
