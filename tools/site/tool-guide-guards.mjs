// Checks shared by the tool guide render and recording scripts. Each one
// throws instead of letting a mistake become a broken recording.

// Kinds drawn only by the UI; they have no backend node type or parameters.
const uiOnlyKinds = new Set(["group"]);

// Resolve a tool kind to its catalog node type and parameter schema.
export function nodeTypeFor(kind, catalog) {
  if (uiOnlyKinds.has(kind)) return { wireType: null, version: 1, parameters: [] };
  const base = kind.replace(/[A-Z]/g, (letter) => "-" + letter.toLowerCase());
  const matches = catalog.nodeTypes.filter((node) => node.type.split("@")[0] === base);
  if (matches.length !== 1)
    throw new Error(
      matches.length
        ? `Tool ${kind} has several node types in tool-guide-catalog.json (${matches.map((node) => node.type).join(", ")})`
        : `Tool ${kind} (${base}) is not in tool-guide-catalog.json; run export-tool-guide-catalog.mjs`,
    );
  const [node] = matches;
  const version = Number(node.type.split("@")[1]);
  if (!Number.isInteger(version)) throw new Error(`Node type ${node.type} has no version number`);
  return { wireType: node.type, version, parameters: node.parameters ?? [] };
}

// Parse GUIDE_KIND into a list of kinds, rejecting names the manifest lacks.
export function selectedKinds(value, knownKinds) {
  if (value === undefined) return null;
  const kinds = value.split(",").map((kind) => kind.trim()).filter(Boolean);
  if (!kinds.length) throw new Error("GUIDE_KIND is set but names no tool");
  const unknown = kinds.filter((kind) => !knownKinds.includes(kind));
  if (unknown.length) throw new Error(`GUIDE_KIND names unknown tools: ${unknown.join(", ")}`);
  return kinds;
}

// Choose a changed number for a property demonstration.
export function exampleNumber(property, current, preferred, id) {
  const min = property.minimum,
    max = property.maximum;
  let next = preferred ?? (current === min ? max : min);
  if (Number.isFinite(min)) next = Math.max(min, next);
  if (Number.isFinite(max)) next = Math.min(max, next);
  if (next === current) next = current === min ? max : min;
  if (!Number.isFinite(next) || next === current)
    throw new Error(`No safe example choice for ${id}`);
  return next;
}

// Choose a different option for a property demonstration.
export function exampleOption(property, current, id) {
  const next = property.options.find((option) => option !== current);
  if (next === undefined) throw new Error(`No safe example choice for ${id}`);
  return next;
}
