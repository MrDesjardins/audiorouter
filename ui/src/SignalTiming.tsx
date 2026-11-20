import type { DiagnosticsSnapshot, Session } from "@audiorouter/contracts";
import { useMessages } from "./i18n/useMessages";

type Node = Session["nodes"][number];
type NodeTiming = NonNullable<DiagnosticsSnapshot["nodeTelemetry"][number]["timing"]>;

const OUTPUT_KINDS = new Set<Node["kind"]>(["physicalOutput", "virtualCaptureSink", "recorder"]);

export interface TimingStep {
  node: Node;
  /** Measured delay at this step, or null before it has been measured. */
  delayMs: number | null;
  processingUsAvg: number | null;
}

export interface TimingRoute {
  output: Node;
  steps: TimingStep[];
  totalMs: number;
  /** The step that adds the most delay, when anything was measured. */
  slowest: TimingStep | null;
  measured: boolean;
}

/**
 * The route the sound takes to each enabled output, from its source in
 * travel order, with each step's measured delay. At a Mixer the slowest
 * input is followed, because it decides when the mixed sound arrives.
 */
export function signalTimingRoutes(session: Session, telemetry: DiagnosticsSnapshot["nodeTelemetry"]): TimingRoute[] {
  const byId = new Map(session.nodes.map((node) => [node.id, node]));
  const timing = new Map<string, NodeTiming>();
  for (const item of telemetry) if (item.timing) timing.set(item.nodeId, item.timing);
  const incoming = (nodeId: string) => session.edges.filter((edge) => edge.enabled && edge.destinationNode === nodeId);
  const delay = (node: Node) => (!node.enabled || node.bypass ? null : (timing.get(node.id)?.delayMs ?? null));
  const upstream = (node: Node, seen: Set<string>): { steps: Node[]; total: number } => {
    if (seen.has(node.id) || seen.size > 64) return { steps: [node], total: delay(node) ?? 0 };
    const next = new Set(seen).add(node.id);
    let best: { steps: Node[]; total: number } | null = null;
    for (const edge of incoming(node.id)) {
      const source = byId.get(edge.sourceNode);
      if (!source) continue;
      const candidate = upstream(source, next);
      if (!best || candidate.total > best.total) best = candidate;
    }
    const own = delay(node) ?? 0;
    return best ? { steps: [...best.steps, node], total: best.total + own } : { steps: [node], total: own };
  };
  return session.nodes
    .filter((node) => node.enabled && OUTPUT_KINDS.has(node.kind) && incoming(node.id).length > 0)
    .map((output) => {
      const route = upstream(output, new Set());
      const steps = route.steps.map((node): TimingStep => ({
        node,
        delayMs: delay(node),
        processingUsAvg: !node.enabled || node.bypass ? null : (timing.get(node.id)?.processingUsAvg ?? null),
      }));
      const measured = steps.some((step) => step.delayMs !== null);
      const slowest = measured
        ? steps.reduce<TimingStep | null>(
            (worst, step) =>
              step.delayMs !== null && (worst === null || step.delayMs > (worst.delayMs ?? 0)) ? step : worst,
            null,
          )
        : null;
      return { output, steps, totalMs: route.total, slowest, measured };
    });
}

const formatMs = (value: number) => (value < 10 ? value.toFixed(1) : value.toFixed(0));

/** Shows, for each output, how long the sound takes and which step is slowest. */
export function SignalTimingPanel({
  session,
  telemetry,
  running,
}: {
  session: Session;
  telemetry: DiagnosticsSnapshot["nodeTelemetry"];
  running: boolean;
}) {
  const tr = useMessages();
  const routes = signalTimingRoutes(session, running ? telemetry : []);
  return (
    <section className="signal-timing" aria-label={tr("g1.timingLabel")}>
      <p className="muted">{tr("g1.timingIntro")}</p>
      {!running && <p className="muted">{tr("g1.timingPressPlay")}</p>}
      {routes.length === 0 && <p className="muted">{tr("g1.timingNoOutput")}</p>}
      {routes.map((route) => (
        <article
          className="timing-route"
          key={route.output.id}
          aria-label={tr("g1.timingTo", { name: route.output.name })}
        >
          <header>
            <strong>
              {route.steps[0]?.node.name} → {route.output.name}
            </strong>
            <span className="timing-total">
              {route.measured ? `${formatMs(route.totalMs)} ms` : tr("g1.timingNotMeasured")}
            </span>
          </header>
          <ol>
            {route.steps.map((step) => {
              const share =
                route.totalMs > 0 && step.delayMs !== null
                  ? Math.max(2, Math.min(100, (step.delayMs / route.totalMs) * 100))
                  : 0;
              const slowest = route.slowest === step && (step.delayMs ?? 0) > 0;
              return (
                <li key={step.node.id} className={slowest ? "timing-step is-slowest" : "timing-step"}>
                  <div className="timing-step-label">
                    <span>
                      {step.node.name}
                      {!step.node.enabled
                        ? ` ${tr("g1.timingOff")}`
                        : step.node.bypass
                          ? ` ${tr("g1.timingBypassed")}`
                          : ""}
                      {slowest && <em> {tr("g1.timingSlowest")}</em>}
                    </span>
                    <span>{step.delayMs === null ? "–" : `${formatMs(step.delayMs)} ms`}</span>
                  </div>
                  <div className="timing-bar" aria-hidden="true">
                    <span style={{ width: `${share}%` }} />
                  </div>
                  {step.processingUsAvg !== null && (
                    <small className="muted">
                      {tr("g1.timingProcessing", {
                        value:
                          step.processingUsAvg < 100
                            ? step.processingUsAvg.toFixed(1)
                            : step.processingUsAvg.toFixed(0),
                      })}
                    </small>
                  )}
                </li>
              );
            })}
          </ol>
        </article>
      ))}
    </section>
  );
}
