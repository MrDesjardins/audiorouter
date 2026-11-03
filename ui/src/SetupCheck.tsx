// Setup check (UI-27): saved sessions judged against the active devices by
// the backend. The list box has a fixed height so a result never moves the
// devices panel below it (UI-17), and the last good result survives a failed
// read.
import type { SetupCheckResult, SetupFinding } from "@audiorouter/contracts";
import { useSetupCheck, type SetupCheckState } from "./useSetupCheck";

const SEVERITY_LABEL: Record<SetupFinding["severity"], string> = {
  problem: "Problem",
  warning: "Warning",
  tip: "Tip",
};

/** One line that sums up a result, for the fixed summary slot. */
export function setupCheckSummary(result: SetupCheckResult | null): string {
  if (!result) return "Checking your sessions and devices…";
  if (result.devicesChecked === 0 && result.findings.length === 0)
    return "No active Windows audio devices were checked.";
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
  onOpenFinding,
}: {
  connected: boolean;
  checkSetup?: () => Promise<SetupCheckResult>;
  /** Changes when devices or saved sessions change, so the check runs again. */
  refreshKey: string;
  onOpenFinding?: (finding: SetupFinding) => void;
}) {
  const state = useSetupCheck({ connected, checkSetup, refreshKey });
  return <SetupCheckPanel state={state} onOpenFinding={onOpenFinding} />;
}

export function SetupCheckPanel({
  state,
  onOpenFinding,
}: {
  state: SetupCheckState;
  onOpenFinding?: (finding: SetupFinding) => void;
}) {
  const { connected, available, result, checking, failure, run } = state;
  if (!available) return null;
  const status = !connected
    ? "Not connected; the check runs when the service is available."
    : failure
      ? `Last check failed: ${failure}`
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
        {!connected && !result
          ? "Connect to AudioRouter to check your setup."
          : failure && !result
            ? "Setup check unavailable."
            : setupCheckSummary(result)}
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
                  {onOpenFinding && (
                    <button
                      type="button"
                      className="secondary"
                      disabled={!connected}
                      onClick={() => onOpenFinding(finding)}
                    >
                      {finding.nodeId ? "Open affected tool" : "Inspect affected route"}
                    </button>
                  )}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="muted">
            {result?.devicesChecked === 0
              ? "Device checks are incomplete. See the check status below and try Check again."
              : result
                ? "No issues found in the device and feedback checks performed."
                : "Results appear here: devices Play would refuse, feedback loops, missing devices and cable rates."}
          </p>
        )}
      </div>
      <p className="muted setup-check-status" title={status}>
        {status}
      </p>
      <p className="muted">
        Checks saved routes after connection and relevant setup changes, even outside this tab. Unsaved edits are not
        included. Play validates the route separately.
      </p>
    </section>
  );
}
