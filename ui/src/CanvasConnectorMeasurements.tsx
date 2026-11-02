import { useEffect, useState } from "react";
import { useReactFlow, useUpdateNodeInternals } from "@xyflow/react";
import { connectorGeometryReady, type ConnectorRequirement } from "./canvasReadiness";

// Card dimensions can arrive before connector bounds on a cold WebView load.
// A later resize is not guaranteed, leaving saved edges and hit testing inert.
// Repair only missing bounds, with bounded retries rather than a meter timer.
export function CanvasConnectorMeasurements({
  geometryKey,
  sessionId,
  revision,
}: {
  geometryKey: string;
  sessionId: string;
  revision: number;
}) {
  const updateNodeInternals = useUpdateNodeInternals();
  const { getInternalNode } = useReactFlow();
  const [measurement, setMeasurement] = useState<{ key: string; status: "ready" | "exhausted" } | null>(null);
  useEffect(() => {
    const expected = JSON.parse(geometryKey) as ConnectorRequirement[];
    let frame = 0;
    let attempts = 0;
    let cancelled = false;
    let fontsReady = !document.fonts || document.fonts.status === "loaded";
    const finish = (status: "ready" | "exhausted") =>
      setMeasurement((current) =>
        current?.key === geometryKey && current.status === status ? current : { key: geometryKey, status },
      );
    const repair = () => {
      if (cancelled) return;
      const missing = expected.filter((requirement) => {
        const node = getInternalNode(requirement.id);
        return !connectorGeometryReady(node?.measured, node?.internals.handleBounds, requirement);
      });
      if (missing.length && attempts++ < 8) {
        updateNodeInternals(missing.map(({ id }) => id));
        frame = requestAnimationFrame(repair);
      } else if (!missing.length && fontsReady) finish("ready");
      else if (missing.length) finish("exhausted");
    };
    frame = requestAnimationFrame(repair);
    void document.fonts?.ready.then(() => {
      if (!cancelled) {
        fontsReady = true;
        cancelAnimationFrame(frame);
        attempts = 0;
        frame = requestAnimationFrame(repair);
      }
    });
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
    };
  }, [geometryKey, getInternalNode, updateNodeInternals]);
  const status = measurement?.key === geometryKey ? measurement.status : "waiting";
  return (
    <span
      hidden
      data-testid="canvas-readiness"
      data-ready={status === "ready"}
      data-state={status}
      data-session-id={sessionId}
      data-revision={revision}
    />
  );
}
