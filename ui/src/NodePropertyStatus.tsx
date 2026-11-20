import type { Node, DiagnosticsSnapshot } from "@audiorouter/contracts";
import { nodeAttention, type NodeAttention } from "./nodeAttention";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

const STATUS_MESSAGES = {
  "No device": "workspaceAudit.noDevice",
  "No input": "workspaceAudit.noInput",
  Muted: "workspaceAudit.muted",
  Disconnected: "workspaceAudit.disconnected",
  Off: "common.off",
  Bypass: "toolProperties.bypass",
  Failed: "workspaceAudit.failed",
  Active: "workspaceAudit.active",
  Ready: "workspaceAudit.ready",
} as const satisfies Record<string, MessageKey>;

const ATTENTION_LABELS = {
  "no device": "No device",
  "no input": "No input",
  muted: "Muted",
} as const satisfies Record<NodeAttention, keyof typeof STATUS_MESSAGES>;

export function nodePropertyStatus(
  node: Node,
  connected: boolean,
  running: boolean,
  snapshot: DiagnosticsSnapshot | null,
  unfed = false,
): keyof typeof STATUS_MESSAGES {
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
  const tr = useMessages();
  const status = nodePropertyStatus(node, connected, running, snapshot, unfed);
  const label = tr(STATUS_MESSAGES[status]);
  return (
    <span
      className={`node-state node-property-status is-${nodePropertyStatusTone(status)}`}
      role="status"
      aria-label={tr("workspaceAudit.toolStatus", { status: label })}
    >
      {label}
    </span>
  );
}
