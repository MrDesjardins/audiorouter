// Setup check (UI-27): saved sessions judged against the active devices by
// the backend. The list box has a fixed height so a result never moves the
// devices panel below it (UI-17), and the last good result survives a failed
// read.
import { useCallback, useEffect, useRef, useState } from "react";
import type { SetupCheckResult, SetupFinding } from "@audiorouter/contracts";

const SEVERITY_LABEL: Record<SetupFinding["severity"], string> = {
  problem: "Problem",
  warning: "Warning",
  tip: "Tip",
};

/** One line that sums up a result, for the fixed summary slot. */
export function setupCheckSummary(result: SetupCheckResult | null): string {
  if (!result) return "Checking your sessions and devices…";
  const count = (severity: SetupFinding["severity"]) =>
    result.findings.filter((finding) => finding.severity === severity).length;
  const parts = [
    [count("problem"), "problem", "problems"],
    [count("warning"), "warning", "warnings"],
    [count("tip"), "tip", "tips"],
  ]
    .filter(([n]) => (n as number) > 0)
    .map(([n, one, many]) => `${n} ${n === 1 ? one : many}`);
  const sessions = `${result.sessionsChecked} session${result.sessionsChecked === 1 ? "" : "s"}`;
  if (parts.length === 0) return `No problems found in ${sessions}.`;
  const more = result.truncated > 0 ? ` (${result.truncated} more not shown)` : "";
  return `${parts.join(", ")} in ${sessions}${more}.`;
}

export function SetupCheck({
  connected,
  checkSetup,
  refreshKey,
}: {
  connected: boolean;
  checkSetup?: () => Promise<SetupCheckResult>;
  /** Changes when devices or saved sessions change, so the check runs again. */
  refreshKey: string;
}) {
  const [result, setResult] = useState<SetupCheckResult | null>(null);
  const [checking, setChecking] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  // Only the latest request may update the panel.
  const latest = useRef(0);
  const run = useCallback(async () => {
    if (!checkSetup) return;
    const request = ++latest.current;
    setChecking(true);
    try {
      const next = await checkSetup();
      if (request !== latest.current) return;
      setResult(next);
      setFailure(null);
    } catch (error) {
      if (request !== latest.current) return;
      setFailure(error instanceof Error ? error.message : "The check could not run.");
    } finally {
      if (request === latest.current) setChecking(false);
    }
  }, [checkSetup]);
  useEffect(() => {
    if (connected) void run();
  }, [connected, run, refreshKey]);
  useEffect(
    () => () => {
      latest.current += 1;
    },
    [],
  );
  if (!checkSetup) return null;
  const status = failure
    ? `Last check failed: ${failure}`
    : !connected
      ? "Not connected; the check runs when the service is available."
      : checking && result
        ? "Checking again…"
        : result?.notChecked[0]
          ? `Not checked: ${result.notChecked.join(" ")}`
          : "";
  return (
    <section className="panel setup-panel setup-check" aria-labelledby="setup-check-heading">
      <div className="section-heading">
        <h3 id="setup-check-heading">Setup check</h3>
        <button type="button" className="secondary" onClick={() => void run()} disabled={!connected || checking}>
          Check again
        </button>
      </div>
      <p className="setup-check-summary" aria-live="polite">
        {setupCheckSummary(result)}
      </p>
      <div className="setup-check-findings" tabIndex={0} aria-label="Setup check findings">
        {result && result.findings.length > 0 ? (
          <ul className="setup-status-list">
            {result.findings.map((finding, index) => (
              <li
                key={`${finding.code}-${finding.sessionId}-${finding.nodeId ?? finding.otherSessionId ?? index}`}
                className={`is-${finding.severity}`}
              >
                <span className="setup-status-dot" aria-hidden="true" />
                <span>
                  <span>
                    <strong>{SEVERITY_LABEL[finding.severity]}:</strong> {finding.message}
                  </span>
                  <small>{finding.fix}</small>
                  <small className="muted">
                    Session {finding.sessionName}
                    {finding.nodeName ? ` · ${finding.nodeName}` : ""}
                  </small>
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="muted">
            {result
              ? "Every device your sessions use fits how they use it."
              : "Results appear here: devices Play would refuse, feedback loops, missing devices and cable rates."}
          </p>
        )}
      </div>
      <p className="muted setup-check-status">{status}</p>
    </section>
  );
}
