/**
 * Pure logic for the API tab's request builder (external app integrations
 * plan): choose a session, a tool and one setting, and get the exact local
 * REST request that changes it. Values come from the backend catalog, so
 * ranges and choices match what the backend accepts. Tokens never appear in
 * generated text; a placeholder is used instead.
 */
import type { DiscoveryDocument, Session } from "@audiorouter/contracts";

type Node = Session["nodes"][number];
export type ParameterSpec = DiscoveryDocument["nodeTypes"][number]["parameters"][number];

/** One setting the builder can change on a node. */
export type BuilderTarget =
  | { kind: "flag"; key: "enabled" | "bypass"; label: string }
  | { kind: "parameter"; key: string; label: string; spec: ParameterSpec; choices?: { value: string; label: string }[] };

export const TOKEN_PLACEHOLDER = "<your API token>";

/** Tools first, then devices; names disambiguated with the kind when repeated. */
export function builderNodes(session: Session): { id: string; label: string }[] {
  const counts = new Map<string, number>();
  for (const node of session.nodes) counts.set(node.name, (counts.get(node.name) ?? 0) + 1);
  return [...session.nodes]
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((node) => ({ id: node.id, label: (counts.get(node.name) ?? 0) > 1 ? `${node.name} (${node.kind}, ${node.id})` : node.name }));
}

/**
 * Settings of one node: Enabled and Bypass, then its catalog parameters.
 * A Mixer's per-input volume family expands to one entry per connected input,
 * and a node reference (Duck trigger) offers the other nodes by name.
 */
export function builderTargets(session: Session, node: Node, nodeTypes: DiscoveryDocument["nodeTypes"] | null): BuilderTarget[] {
  const targets: BuilderTarget[] = [
    { kind: "flag", key: "enabled", label: "Enabled (on/off)" },
    { kind: "flag", key: "bypass", label: "Bypass" },
  ];
  const specs = nodeTypes?.find((type) => type.type.split("@")[0] === node.kind)?.parameters ?? [];
  const nameOf = (id: string) => session.nodes.find((candidate) => candidate.id === id)?.name ?? id;
  for (const spec of specs) {
    if (spec.namePattern?.includes("<upstreamNodeId>")) {
      const prefix = spec.namePattern.slice(0, spec.namePattern.indexOf("<"));
      const upstream = [...new Set(session.edges.filter((edge) => edge.destinationNode === node.id).map((edge) => edge.sourceNode))];
      for (const id of upstream) targets.push({ kind: "parameter", key: `${prefix}${id}`, label: `Input volume: ${nameOf(id)}`, spec });
      continue;
    }
    if (spec.reference === "node") {
      const choices = session.nodes.filter((other) => other.id !== node.id).map((other) => ({ value: other.id, label: other.name }));
      targets.push({ kind: "parameter", key: spec.name, label: `${spec.name} (choose a node)`, spec, choices });
      continue;
    }
    targets.push({ kind: "parameter", key: spec.name, label: spec.unit ? `${spec.name} (${spec.unit})` : spec.name, spec });
  }
  return targets;
}

/** The value as the backend expects it, or an error that explains the limits. */
export function parseValue(target: BuilderTarget, text: string): { value: boolean | number | string } | { error: string } {
  if (target.kind === "flag" || target.spec.type === "boolean") {
    if (text === "true" || text === "false") return { value: text === "true" };
    return { error: "Choose true or false." };
  }
  if (target.spec.type === "number") {
    const value = Number(text);
    if (text.trim() === "" || !Number.isFinite(value)) return { error: "Enter a number." };
    const { minimum, maximum, unit } = target.spec;
    if ((minimum !== undefined && value < minimum) || (maximum !== undefined && value > maximum)) {
      return { error: `Enter a value from ${minimum ?? "−∞"} to ${maximum ?? "∞"}${unit ? ` ${unit}` : ""}.` };
    }
    return { value };
  }
  const allowed = target.choices?.map((choice) => choice.value) ?? target.spec.enum;
  if (allowed && !allowed.includes(text)) return { error: `Choose one of: ${(target.choices?.map((choice) => choice.label) ?? allowed).join(", ")}.` };
  if (text === "") return { error: "Enter a value." };
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
