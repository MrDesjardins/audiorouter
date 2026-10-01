import type { Session } from "@audiorouter/contracts";
import type { LayoutPositions } from "./layout";

/** Presentation only: topology layers, connected lanes and barycentric ordering. */
export function smartLayout(session: Session, measured: Record<string, { width: number; height: number }> = {}): LayoutPositions {
  const ids = session.nodes.map(n => n.id).sort();
  const parents = new Map(ids.map(id => [id, new Set<string>()]));
  const children = new Map(ids.map(id => [id, new Set<string>()]));
  // Include disabled edges: toggling audio must not rearrange the workspace.
  for (const edge of session.edges) {
    if (children.has(edge.sourceNode) && parents.has(edge.destinationNode) && edge.sourceNode !== edge.destinationNode) {
      children.get(edge.sourceNode)!.add(edge.destinationNode);
      parents.get(edge.destinationNode)!.add(edge.sourceNode);
    }
  }
  const positions: LayoutPositions = {};
  const visited = new Set<string>();
  let laneY = 0;
  for (const seed of ids) {
    if (visited.has(seed)) continue;
    const component: string[] = [], pending = [seed];
    while (pending.length) {
      const id = pending.pop()!;
      if (visited.has(id)) continue;
      visited.add(id); component.push(id);
      pending.push(...parents.get(id)!, ...children.get(id)!);
    }
    component.sort();
    const remaining = new Set(component);
    const ranks = new Map(component.map(id => [id, 0]));
    while (remaining.size) {
      const ready = [...remaining].filter(id => [...parents.get(id)!].every(p => !remaining.has(p))).sort();
      // Invalid draft cycles still get a bounded, deterministic arrangement.
      if (!ready.length) ready.push([...remaining].sort()[0]);
      for (const id of ready) {
        remaining.delete(id);
        for (const next of children.get(id)!) if (remaining.has(next)) ranks.set(next, Math.max(ranks.get(next)!, ranks.get(id)! + 1));
      }
    }
    const layers: string[][] = [];
    for (const id of component) (layers[ranks.get(id)!] ??= []).push(id);
    const order = () => new Map(layers.flatMap(layer => layer.map((id, index) => [id, index] as const)));
    for (let pass = 0; pass < 6; pass++) {
      const forward = pass % 2 === 0;
      for (const layer of forward ? layers : [...layers].reverse()) {
        const indices = order();
        const score = (id: string) => {
          const adjacent = [...(forward ? parents : children).get(id)!];
          return adjacent.length ? adjacent.reduce((sum, n) => sum + indices.get(n)!, 0) / adjacent.length : indices.get(id)!;
        };
        layer.sort((a, b) => score(a) - score(b) || a.localeCompare(b));
      }
    }
    const size = (id: string) => ({ width: Math.max(218, measured[id]?.width ?? 218), height: Math.max(180, measured[id]?.height ?? 180) });
    const heights = layers.map(layer => layer.reduce((sum, id) => sum + size(id).height, 0) + Math.max(0, layer.length - 1) * 70);
    const laneHeight = Math.max(...heights);
    let x = 0;
    layers.forEach((layer, index) => {
      let y = laneY + (laneHeight - heights[index]) / 2;
      for (const id of layer) { positions[id] = { x, y }; y += size(id).height + 70; }
      x += Math.max(...layer.map(id => size(id).width)) + 110;
    });
    laneY += laneHeight + 130;
  }
  return positions;
}
