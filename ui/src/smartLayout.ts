import type { NodeKind, Session } from "@audiorouter/contracts";
import type { LayoutPositions } from "./layout";

/** Sources always sit in the first column. */
const INPUT_KINDS = new Set<NodeKind>([
  "physicalInput",
  "testSignal",
  "audioFile",
  "applicationCapture",
  "endpointLoopback",
  "virtualRenderSource",
  "networkReceive",
]);
/** Destinations always sit in the last column (a Recorder too, when nothing follows it). */
const OUTPUT_KINDS = new Set<NodeKind>(["physicalOutput", "virtualCaptureSink", "networkSend", "recorder"]);

export const LAYOUT_COLUMN_GAP = 200; // about one node width of breathing room
export const LAYOUT_ROW_GAP = 90;
const COMPONENT_GAP = 160;
const DEFAULT_SIZE = { width: 218, height: 180 };
/** Vertical room reserved where a long connection crosses a column. */
const PASS_THROUGH_HEIGHT = 48;

type Slot = { id: string; real: boolean; height: number };

/**
 * Presentation-only arrangement: inputs on the left, outputs on the right,
 * tools in between by signal order. Long connections reserve space in the
 * columns they cross, crossings are reduced by barycentre ordering, and each
 * node is pulled level with its neighbours so chains read as straight lines.
 * Deterministic for the same graph, whatever the node or edge order.
 */
export function smartLayout(
  session: Session,
  measured: Record<string, { width: number; height: number }> = {},
): LayoutPositions {
  const kinds = new Map(session.nodes.map((node) => [node.id, node.kind]));
  const ids = [...kinds.keys()].sort();
  const parents = new Map(ids.map((id) => [id, new Set<string>()]));
  const children = new Map(ids.map((id) => [id, new Set<string>()]));
  // Include paused connections: pausing audio must not rearrange the canvas.
  for (const edge of session.edges) {
    if (
      edge.sourceNode !== edge.destinationNode &&
      children.has(edge.sourceNode) &&
      parents.has(edge.destinationNode)
    ) {
      children.get(edge.sourceNode)!.add(edge.destinationNode);
      parents.get(edge.destinationNode)!.add(edge.sourceNode);
    }
  }
  const size = (id: string) => ({
    width: Math.max(DEFAULT_SIZE.width, measured[id]?.width ?? DEFAULT_SIZE.width),
    height: Math.max(DEFAULT_SIZE.height, measured[id]?.height ?? DEFAULT_SIZE.height),
  });
  const isInput = (id: string) =>
    parents.get(id)!.size === 0 && (INPUT_KINDS.has(kinds.get(id)!) || children.get(id)!.size > 0);
  const isOutput = (id: string) =>
    !isInput(id) && children.get(id)!.size === 0 && (OUTPUT_KINDS.has(kinds.get(id)!) || parents.get(id)!.size > 0);

  // Longest-path rank of every node; draft cycles are broken deterministically.
  const rank = new Map(ids.map((id) => [id, 0]));
  const remaining = new Set(ids);
  while (remaining.size) {
    const ready = [...remaining].filter((id) => [...parents.get(id)!].every((parent) => !remaining.has(parent))).sort();
    if (!ready.length) ready.push([...remaining].sort()[0]);
    for (const id of ready) {
      remaining.delete(id);
      for (const next of children.get(id)!)
        if (remaining.has(next)) rank.set(next, Math.max(rank.get(next)!, rank.get(id)! + 1));
    }
  }
  // Columns shared by every path: inputs 0, tools 1…, outputs last.
  for (const id of ids) {
    if (isInput(id)) rank.set(id, 0);
    else rank.set(id, Math.max(1, rank.get(id)!));
  }
  const lastToolColumn = Math.max(0, ...ids.filter((id) => !isInput(id) && !isOutput(id)).map((id) => rank.get(id)!));
  const outputColumn = lastToolColumn + 1;
  for (const id of ids) if (isOutput(id)) rank.set(id, outputColumn);
  const columnCount = Math.max(outputColumn, ...ids.map((id) => rank.get(id)!)) + 1;

  // Column x positions from the widest node of each column, across all paths.
  const columnWidth = Array.from({ length: columnCount }, (_, column) =>
    Math.max(DEFAULT_SIZE.width, ...ids.filter((id) => rank.get(id) === column).map((id) => size(id).width)),
  );
  const columnX: number[] = [];
  columnWidth.reduce((x, width, column) => {
    columnX[column] = x;
    return x + width + LAYOUT_COLUMN_GAP;
  }, 0);

  // Connected components, each laid out in its own band.
  const positions: LayoutPositions = {};
  const visited = new Set<string>();
  let bandTop = 0;
  for (const seed of ids) {
    if (visited.has(seed)) continue;
    const component: string[] = [];
    const pending = [seed];
    while (pending.length) {
      const id = pending.pop()!;
      if (visited.has(id)) continue;
      visited.add(id);
      component.push(id);
      pending.push(...parents.get(id)!, ...children.get(id)!);
    }
    component.sort();

    // Layers with pass-through slots for connections that skip columns.
    const layers: Slot[][] = Array.from({ length: columnCount }, () => []);
    const up = new Map<string, string[]>(); // slot -> neighbours to the left
    const down = new Map<string, string[]>(); // slot -> neighbours to the right
    const link = (from: string, to: string) => {
      (down.get(from) ?? down.set(from, []).get(from)!).push(to);
      (up.get(to) ?? up.set(to, []).get(to)!).push(from);
    };
    for (const id of component) layers[rank.get(id)!].push({ id, real: true, height: size(id).height });
    for (const from of component) {
      for (const to of [...children.get(from)!].sort()) {
        const start = rank.get(from)!,
          end = rank.get(to)!;
        if (end <= start) {
          link(from, to);
          continue;
        }
        let previous = from;
        for (let column = start + 1; column < end; column += 1) {
          const slot = `${from}->${to}@${column}`;
          layers[column].push({ id: slot, real: false, height: PASS_THROUGH_HEIGHT });
          link(previous, slot);
          previous = slot;
        }
        link(previous, to);
      }
    }
    for (const layer of layers) layer.sort((a, b) => a.id.localeCompare(b.id));

    // Order each column by the mean position of its neighbours (crossing reduction).
    const index = new Map<string, number>();
    const reindex = () => layers.forEach((layer) => layer.forEach((slot, position) => index.set(slot.id, position)));
    reindex();
    for (let pass = 0; pass < 12; pass += 1) {
      const forward = pass % 2 === 0;
      for (const layer of forward ? layers : [...layers].reverse()) {
        const score = (slot: Slot) => {
          const neighbours = (forward ? up : down).get(slot.id) ?? [];
          return neighbours.length
            ? neighbours.reduce((sum, id) => sum + index.get(id)!, 0) / neighbours.length
            : index.get(slot.id)!;
        };
        layer.sort((a, b) => score(a) - score(b) || a.id.localeCompare(b.id));
        layer.forEach((slot, position) => index.set(slot.id, position));
      }
    }

    // Heights: stack, then pull each slot level with its neighbours.
    const top = new Map<string, number>();
    for (const layer of layers) {
      let y = 0;
      for (const slot of layer) {
        top.set(slot.id, y);
        y += slot.height + LAYOUT_ROW_GAP;
      }
    }
    const centre = (id: string, layer: Slot[]) => top.get(id)! + layer.find((slot) => slot.id === id)!.height / 2;
    const slotsById = new Map(layers.flatMap((layer) => layer.map((slot) => [slot.id, { slot, layer }] as const)));
    for (let pass = 0; pass < 10; pass += 1) {
      const forward = pass % 2 === 0;
      for (const layer of forward ? layers : [...layers].reverse()) {
        if (!layer.length) continue;
        const desired = layer.map((slot) => {
          const neighbours = [...(up.get(slot.id) ?? []), ...(down.get(slot.id) ?? [])];
          if (!neighbours.length) return top.get(slot.id)!;
          const mean =
            neighbours.reduce((sum, id) => sum + centre(id, slotsById.get(id)!.layer), 0) / neighbours.length;
          return mean - slot.height / 2;
        });
        // Keep order and spacing: place top-down at or below the desired
        // height, then shift the column so it sits around its desired mean.
        let floor = -Infinity;
        const placed = layer.map((slot, position) => {
          const y = Math.max(desired[position], floor);
          floor = y + slot.height + LAYOUT_ROW_GAP;
          return y;
        });
        const shift = (desired.reduce((a, b) => a + b, 0) - placed.reduce((a, b) => a + b, 0)) / layer.length;
        layer.forEach((slot, position) => top.set(slot.id, placed[position] + shift));
      }
    }
    // Normalise the band to start at bandTop and place real nodes.
    const all = layers.flat();
    const minTop = Math.min(...all.map((slot) => top.get(slot.id)!));
    const maxBottom = Math.max(...all.map((slot) => top.get(slot.id)! + slot.height));
    for (const slot of all) {
      if (slot.real)
        positions[slot.id] = { x: columnX[rank.get(slot.id)!], y: Math.round(bandTop + top.get(slot.id)! - minTop) };
    }
    bandTop += maxBottom - minTop + COMPONENT_GAP;
  }
  return positions;
}
