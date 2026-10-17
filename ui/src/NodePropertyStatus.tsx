import type { Node, DiagnosticsSnapshot } from "@audiorouter/contracts";

export function nodePropertyStatus(
  node: Node,
  connected: boolean,
  running: boolean,
  snapshot: DiagnosticsSnapshot | null,
): string {
  if (!connected) return "Disconnected";
  if (!node.enabled) return "Off";
  if (node.bypass) return "Bypass";
  const plugin = snapshot?.nodeTelemetry.find((item) => item.nodeId === node.id)?.plugin;
  if (plugin?.state === "failed" || plugin?.state === "quarantined") return "Failed";
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
}: {
  node: Node;
  connected: boolean;
  running: boolean;
  snapshot: DiagnosticsSnapshot | null;
}) {
  const status = nodePropertyStatus(node, connected, running, snapshot);
  return (
    <span
      className={`node-state node-property-status is-${nodePropertyStatusTone(status)}`}
      role="status"
      aria-label={`Node status: ${status}`}
    >
      {status}
    </span>
  );
}
