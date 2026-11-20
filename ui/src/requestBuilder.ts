/**
 * Pure logic for the API tab's request builder (external app integrations
 * plan): choose a session, a tool and one setting, and get the exact local
 * REST request that changes it. Values come from the backend catalog, so
 * ranges and choices match what the backend accepts. Tokens never appear in
 * generated text; a placeholder is used instead.
 */
import type { DiscoveryDocument, Session } from "@audiorouter/contracts";
import type { MessageKey } from "./i18n/generated/messageKeys";

type Node = Session["nodes"][number];
export type ParameterSpec = DiscoveryDocument["nodeTypes"][number]["parameters"][number];

/** A catalog message that replaces the English `label` or `error` in the UI. */
export type MessageRef = { key: MessageKey; vars?: Record<string, string> };

/** One setting the builder can change on a node. */
export type BuilderTarget =
  | { kind: "flag"; key: "enabled" | "bypass"; label: string; message?: MessageRef }
  | {
      kind: "parameter";
      key: string;
      label: string;
      message?: MessageRef;
      spec: ParameterSpec;
      choices?: { value: string; label: string }[];
    };

export const TOKEN_PLACEHOLDER = "<your API token>";

/** Tools first, then devices; names disambiguated with the kind when repeated. */
export function builderNodes(session: Session): { id: string; label: string }[] {
  const counts = new Map<string, number>();
  for (const node of session.nodes) counts.set(node.name, (counts.get(node.name) ?? 0) + 1);
  return [...session.nodes]
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((node) => ({
      id: node.id,
      label: (counts.get(node.name) ?? 0) > 1 ? `${node.name} (${node.kind}, ${node.id})` : node.name,
    }));
}

/**
 * Settings of one node: Enabled and Bypass, then its catalog parameters.
 * A Mixer's per-input volume family expands to one entry per connected input,
 * and a node reference (Duck trigger) offers the other nodes by name.
 */
export function builderTargets(
  session: Session,
  node: Node,
  nodeTypes: DiscoveryDocument["nodeTypes"] | null,
): BuilderTarget[] {
  const targets: BuilderTarget[] = [
    { kind: "flag", key: "enabled", label: "Enabled (on/off)", message: { key: "g6.apiBuilderFlagEnabled" } },
    { kind: "flag", key: "bypass", label: "Bypass", message: { key: "g6.apiBuilderFlagBypass" } },
  ];
  const specs = nodeTypes?.find((type) => type.type.split("@")[0] === node.kind)?.parameters ?? [];
  const nameOf = (id: string) => session.nodes.find((candidate) => candidate.id === id)?.name ?? id;
  for (const spec of specs) {
    if (spec.namePattern?.includes("<upstreamNodeId>")) {
      const prefix = spec.namePattern.slice(0, spec.namePattern.indexOf("<"));
      const upstream = [
        ...new Set(session.edges.filter((edge) => edge.destinationNode === node.id).map((edge) => edge.sourceNode)),
      ];
      for (const id of upstream)
        targets.push({
          kind: "parameter",
          key: `${prefix}${id}`,
          label: `Input volume: ${nameOf(id)}`,
          message: { key: "g6.apiBuilderInputVolume", vars: { name: nameOf(id) } },
          spec,
        });
      continue;
    }
    // A numbered family such as a Soundboard's `slot<N>Mode` gives one key per slot.
    if (spec.namePattern?.includes("<N>")) {
      for (let slot = spec.slots?.minimum ?? 1; slot <= (spec.slots?.maximum ?? 0); slot += 1) {
        const key = spec.namePattern.replace("<N>", String(slot));
        targets.push({ kind: "parameter", key, label: spec.unit ? `${key} (${spec.unit})` : key, spec });
      }
      continue;
    }
    // Any other family is never a literal key.
    if (spec.namePattern) continue;
    if (spec.reference === "node") {
      const choices = session.nodes
        .filter((other) => other.id !== node.id)
        .map((other) => ({ value: other.id, label: other.name }));
      targets.push({
        kind: "parameter",
        key: spec.name,
        label: `${spec.name} (choose a node)`,
        message: { key: "g6.apiBuilderChooseNode", vars: { name: spec.name } },
        spec,
        choices,
      });
      continue;
    }
    targets.push({
      kind: "parameter",
      key: spec.name,
      label: spec.unit ? `${spec.name} (${spec.unit})` : spec.name,
      spec,
    });
  }
  return targets;
}

/** The value as the backend expects it, or an error that explains the limits. */
export function parseValue(
  target: BuilderTarget,
  text: string,
): { value: boolean | number | string } | { error: string; message: MessageRef } {
  if (target.kind === "flag" || target.spec.type === "boolean") {
    if (text === "true" || text === "false") return { value: text === "true" };
    return { error: "Choose true or false.", message: { key: "g6.apiBuilderErrBoolean" } };
  }
  if (target.spec.type === "number") {
    const value = Number(text);
    if (text.trim() === "" || !Number.isFinite(value))
      return { error: "Enter a number.", message: { key: "g6.apiBuilderErrNumber" } };
    const { minimum, maximum, unit } = target.spec;
    if ((minimum !== undefined && value < minimum) || (maximum !== undefined && value > maximum)) {
      return {
        error: `Enter a value from ${minimum ?? "−∞"} to ${maximum ?? "∞"}${unit ? ` ${unit}` : ""}.`,
        message: {
          key: "g6.apiBuilderErrRange",
          vars: { min: String(minimum ?? "−∞"), max: String(maximum ?? "∞"), unit: unit ? ` ${unit}` : "" },
        },
      };
    }
    return { value };
  }
  const allowed = target.choices?.map((choice) => choice.value) ?? target.spec.enum;
  if (allowed && !allowed.includes(text)) {
    const list = (target.choices?.map((choice) => choice.label) ?? allowed).join(", ");
    return { error: `Choose one of: ${list}.`, message: { key: "g6.apiBuilderErrChoose", vars: { list } } };
  }
  if (text === "") return { error: "Enter a value.", message: { key: "g6.apiBuilderErrValue" } };
  return { value: text };
}

export type BuiltRequest = {
  method: "POST";
  url: string;
  body: Record<string, unknown>;
  json: string;
  curl: string;
  powershell: string;
};

/**
 * The `nodes.set` request for one setting. Following the active session
 * omits `sessionId` so the request acts on whatever session is selected in
 * the app; pinning includes it. The node is addressed by its stable ID.
 */
export function buildRequest(options: {
  baseUrl: string;
  sessionId: string | null;
  nodeId: string;
  target: BuilderTarget;
  value: boolean | number | string;
  idempotencyKey: string;
}): BuiltRequest {
  const body: Record<string, unknown> = {};
  if (options.sessionId) body.sessionId = options.sessionId;
  body.node = options.nodeId;
  if (options.target.kind === "flag") body[options.target.key] = options.value;
  else body.parameters = { [options.target.key]: options.value };
  body.idempotencyKey = options.idempotencyKey;
  const url = `${options.baseUrl.replace(/\/+$/, "")}/api/v1/nodes/set`;
  const json = JSON.stringify(body, null, 2);
  const compact = JSON.stringify(body);
  return {
    method: "POST",
    url,
    body,
    json,
    curl: `curl -X POST "${url}" -H "Authorization: Bearer ${TOKEN_PLACEHOLDER}" -H "Content-Type: application/json" -d '${compact.replace(/'/g, "'\\''")}'`,
    powershell: `Invoke-RestMethod -Method Post -Uri "${url}" -Headers @{ Authorization = "Bearer ${TOKEN_PLACEHOLDER}" } -ContentType "application/json" -Body '${compact.replace(/'/g, "''")}'`,
  };
}

/** A readable, unique-enough key; integrations should send a new key per event. */
export function exampleIdempotencyKey(now = Date.now()): string {
  return `integration-${now.toString(36)}`;
}
