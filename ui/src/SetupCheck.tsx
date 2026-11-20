// Setup check (UI-27): saved sessions judged against the active devices by
// the backend. The list box has a fixed height so a result never moves the
// devices panel below it (UI-17), and the last good result survives a failed
// read.
import type { SetupCheckResult, SetupFinding } from "@audiorouter/contracts";
import { tx } from "./i18n";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";
import { useSetupCheck, type SetupCheckState } from "./useSetupCheck";

const SEVERITY_LABEL: Record<SetupFinding["severity"], MessageKey> = {
  problem: "g1.setupProblem",
  warning: "g1.setupWarning",
  tip: "g1.setupTip",
};

/** One line that sums up a result, for the fixed summary slot. */
export function setupCheckSummary(result: SetupCheckResult | null): string {
  if (!result) return tx("g1.setupChecking");
  if (result.devicesChecked === 0 && result.findings.length === 0) return tx("g1.setupNoDevices");
  const count = (severity: SetupFinding["severity"]) =>
    result.findings.filter((finding) => finding.severity === severity).length;
  const parts = (
    [
      [count("problem"), "g1.setupProblems", "g1.setupProblemsMany"],
      [count("warning"), "g1.setupWarnings1", "g1.setupWarningsMany"],
      [count("tip"), "g1.setupTips1", "g1.setupTipsMany"],
    ] as const
  )
    .filter(([n]) => n > 0)
    .map(([n, one, many]) => tx(n === 1 ? one : many, { count: n }));
  const sessions = tx(result.sessionsChecked === 1 ? "g1.setupSessions1" : "g1.setupSessionsMany", {
    count: result.sessionsChecked,
  });
  if (parts.length === 0) return tx("g1.setupNoneFound", { sessions });
  const more = result.truncated > 0 ? tx("g1.setupMore", { count: result.truncated }) : "";
  return tx("g1.setupFound", { parts: parts.join(", "), sessions, more });
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
  const tr = useMessages();
  const { connected, available, result, checking, failure, run } = state;
  if (!available) return null;
  const status = !connected
    ? tr("g1.setupNotConnected")
    : failure
      ? tr("g1.setupLastFailed", { failure })
      : checking && result
        ? tr("g1.setupCheckingAgain")
        : result?.notChecked[0]
          ? tr("g1.setupNotChecked", { reasons: result.notChecked.join(" ") })
          : "";
  return (
    <section className="panel setup-panel setup-check" aria-labelledby="setup-check-heading">
      <div className="section-heading">
        <h3 id="setup-check-heading">{tr("g1.setupHeading")}</h3>
        <button type="button" className="secondary" onClick={() => void run()} disabled={!connected || checking}>
          {tr("g1.setupCheckAgain")}
        </button>
      </div>
      <p className="setup-check-summary" aria-live="polite">
        {!connected && !result
          ? tr("g1.setupConnectToCheck")
          : failure && !result
            ? tr("g1.setupUnavailable")
            : setupCheckSummary(result)}
      </p>
      <div className="setup-check-findings" tabIndex={0} aria-label={tr("g1.setupFindingsLabel")}>
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
                    <strong>{tr(SEVERITY_LABEL[finding.severity])}:</strong> {finding.message}
                  </span>
                  <small>{finding.fix}</small>
                  <small className="muted">
                    {tr("g1.setupSession", { name: finding.sessionName })}
                    {finding.nodeName ? ` · ${finding.nodeName}` : ""}
                  </small>
                  {onOpenFinding && (
                    <button
                      type="button"
                      className="secondary"
                      disabled={!connected}
                      onClick={() => onOpenFinding(finding)}
                    >
                      {finding.nodeId ? tr("g1.setupOpenTool") : tr("g1.setupInspectRoute")}
                    </button>
                  )}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="muted">
            {result?.devicesChecked === 0
              ? tr("g1.setupIncomplete")
              : result
                ? tr("g1.setupNoIssues")
                : tr("g1.setupResultsHere")}
          </p>
        )}
      </div>
      <p className="muted setup-check-status" title={status}>
        {status}
      </p>
      <p className="muted">{tr("g1.setupFootnote")}</p>
    </section>
  );
}
