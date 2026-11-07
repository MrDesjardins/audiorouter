import type { Node, DiagnosticsSnapshot } from "@audiorouter/contracts";
import { nodeAttention, type NodeAttention } from "./nodeAttention";

const ATTENTION_LABELS: Record<NodeAttention, string> = {
  "no device": "No device",
  "no input": "No input",
  muted: "Muted",
};

export function nodePropertyStatus(
  node: Node,
  connected: boolean,
  running: boolean,
  snapshot: DiagnosticsSnapshot | null,
  unfed = false,
): string {
  if (!connected) return "Disconnected";
  if (!node.enabled) return "Off";
  if (node.bypass) return "Bypass";
  const plugin = snapshot?.nodeTelemetry.find((item) => item.nodeId === node.id)?.plugin;
  if (plugin?.state === "failed" || plugin?.state === "quarantined") return "Failed";
  const attention = nodeAttention(node, { unfed, privacyMuted: snapshot?.privacyMute.muted ?? false });
  if (attention) return ATTENTION_LABELS[attention];
  return running ? "Active" : "Ready";
}

/** The canvas node-state tone for a status, so both read the same colours. */
export function nodePropertyStatusTone(status: string): "ready" | "bypassed" | "disabled" | "attention" {
  switch (status) {
    case "Active":
    case "Ready":
      return "ready";
    case "Bypass":
      return "bypassed";
    case "Failed":
    case "Disconnected":
    case "No device":
    case "No input":
    case "Muted":
      return "attention";
    default:
      return "disabled";
  }
}

export function NodePropertyStatus({
  node,
  connected,
  running,
  snapshot,
  unfed,
}: {
  node: Node;
  connected: boolean;
  running: boolean;
  snapshot: DiagnosticsSnapshot | null;
  unfed: boolean;
}) {
  const status = nodePropertyStatus(node, connected, running, snapshot, unfed);
  return (
    <span
      className={`node-state node-property-status is-${nodePropertyStatusTone(status)}`}
      role="status"
      aria-label={`Tool status: ${status}`}
    >
      {status}
    </span>
  );
}
