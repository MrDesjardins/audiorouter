import { useEffect, useRef, useState } from "react";
import type { ObsChangeRequest, ObsPreview, ObsStatus } from "@audiorouter/contracts";
import { AppStateBadge } from "./AppsPanel";
import { obsFixes, type ObsCheck } from "./obsUsage";

export interface ObsActions {
  connect(password?: string): Promise<unknown>;
  disconnect(): Promise<unknown>;
  preview(changes: ObsChangeRequest[]): Promise<ObsPreview>;
  apply(previewId: string): Promise<unknown>;
  restore(): Promise<unknown>;
  /** Read the status again soon, after an action. */
  poll(): void;
}

const CHANGE_ERRORS: Record<NonNullable<ObsStatus["lastChange"]["error"]>, string> = {
  changedInObs: "A setting changed in OBS after the preview, so it was left as it is. Check again, then fix it.",
  refusedByObs: "OBS refused a change.",
  connectionLost: "The connection to OBS was lost during the change.",
};

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** The steps to turn on OBS's WebSocket server, or a password field when OBS asks for one. */
function ConnectSteps({ status, actions }: { status: ObsStatus; actions: ObsActions | null }) {
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const connecting = busy || status.connection === "connecting";
  const askPassword = status.connection === "needsPassword" || status.connection === "wrongPassword";
  const connect = (pasted?: string) => {
    if (!actions) return;
    setBusy(true);
    setError(null);
    actions
      .connect(pasted)
      .then(() => actions.poll())
      .catch((reason: unknown) => setError(errorText(reason)))
      .finally(() => setBusy(false));
  };
  return (
    <div className="obs-connect">
      <ol className="obs-steps">
        <li>
          In OBS, open <strong>Tools › WebSocket Server Settings</strong>.
        </li>
        <li>
          Tick <strong>Enable WebSocket server</strong>, then choose <strong>OK</strong>.
        </li>
        <li>Connect. AudioRouter reads the server password from OBS's settings on this PC.</li>
      </ol>
      {askPassword && (
        <label className="obs-password">
          <span>
            {status.connection === "wrongPassword"
              ? "OBS refused the password. Paste the one from Show Connect Info:"
              : "OBS asks for a password. Paste the one from Show Connect Info:"}
          </span>
          <input
            type="password"
            autoComplete="off"
            value={password}
            maxLength={256}
            onChange={(event) => setPassword(event.target.value)}
          />
        </label>
      )}
      <div className="obs-actions">
        <button
          type="button"
          className="primary"
          disabled={!actions || connecting || (askPassword && password.length === 0)}
          onClick={() => connect(askPassword ? password : undefined)}
        >
          {/* Both labels share one cell, so the button keeps its width (UI-17). */}
          <span className="app-swap">
            <span className={connecting ? "app-label-hidden" : undefined}>Connect to OBS</span>
            <span className={connecting ? undefined : "app-label-hidden"}>Connecting…</span>
          </span>
        </button>
      </div>
      <p className="muted obs-note" role="status">
        {error ??
          (status.connection === "serverOff"
            ? "OBS's WebSocket server is off, or OBS is still starting."
            : status.connection === "failed"
              ? "Could not connect to OBS. Check the steps above and connect again."
              : "The password stays in memory while connected. It is never stored or shown.")}
      </p>
    </div>
  );
}

function FixDialog({
  fixes,
  inputs,
  actions,
  onClose,
}: {
  fixes: ReturnType<typeof obsFixes>;
  inputs: ObsStatus["inputs"];
  actions: ObsActions;
  onClose: () => void;
}) {
  const [chosen, setChosen] = useState(() => fixes.map(() => true));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const first = useRef<HTMLButtonElement>(null);
  const cancelled = useRef(false);
  useEffect(() => {
    cancelled.current = false;
    first.current?.focus();
    return () => {
      cancelled.current = true;
    };
  }, []);
  const selected = fixes.filter((_, index) => chosen[index]);
  const apply = () => {
    setBusy(true);
    setError(null);
    // Preview the exact selection, then apply that preview: the backend
    // refuses any source that changed in OBS since.
    actions
      .preview(selected.map((fix) => fix.change))
      .then((preview) => {
        if (cancelled.current) return;
        // The dialog showed these starting values. A newer backend snapshot
        // must not silently turn the user's click into a different preview.
        const matches =
          preview.changes.length === selected.length &&
          selected.every(({ change }) => {
            const source = inputs.find((input) => input.name === change.inputName);
            const device = "deviceId" in change;
            const field = device ? "device" : "muted";
            const before = device ? source?.deviceId : source?.muted;
            const after = device ? change.deviceId : change.muted;
            return (
              source &&
              preview.changes.some(
                (item) =>
                  item.inputName === change.inputName &&
                  item.field === field &&
                  item.before === before &&
                  item.after === after,
              )
            );
          });
        if (!matches) throw new Error("OBS settings changed while this dialog was open. Cancel and check again.");
        return actions.apply(preview.previewId);
      })
      .then(() => {
        if (cancelled.current) return;
        actions.poll();
        onClose();
      })
      .catch((reason: unknown) => {
        if (!cancelled.current) setError(errorText(reason));
      })
      .finally(() => {
        if (!cancelled.current) setBusy(false);
      });
  };
  return (
    <div
      className="dialog-backdrop"
      role="presentation"
      onKeyDown={(event) => {
        if (event.key === "Escape" && !busy) onClose();
      }}
    >
      <section
        className="connection-dialog obs-fix-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="obs-fix-heading"
        aria-describedby="obs-fix-description"
      >
        <h2 id="obs-fix-heading">Fix in OBS</h2>
        <p id="obs-fix-description">
          AudioRouter changes only the settings you tick. You can put them back with{" "}
          <strong>Restore previous OBS settings</strong>.
        </p>
        <ul className="obs-fix-list">
          {fixes.map((fix, index) => (
            <li key={fix.label}>
              <label>
                <input
                  type="checkbox"
                  checked={chosen[index]}
                  disabled={busy}
                  onChange={(event) =>
                    setChosen((previous) =>
                      previous.map((value, position) => (position === index ? event.target.checked : value)),
                    )
                  }
                />
                <span>
                  <strong>{fix.label}</strong>
                  <small>{fix.reason}</small>
                </span>
              </label>
            </li>
          ))}
        </ul>
        {error && (
          <p className="muted" role="alert">
            {error}
          </p>
        )}
        <div className="actions">
          <button
            ref={first}
            type="button"
            className="primary"
            disabled={busy || selected.length === 0}
            onClick={apply}
          >
            {busy ? "Changing OBS…" : `Change ${selected.length} setting${selected.length === 1 ? "" : "s"}`}
          </button>
          <button type="button" className="secondary" disabled={busy} onClick={onClose}>
            Cancel
          </button>
        </div>
      </section>
    </div>
  );
}

export function ObsCard({
  check,
  status,
  actions,
}: {
  check: ObsCheck;
  status: ObsStatus | null;
  actions: ObsActions | null;
}) {
  const [fixing, setFixing] = useState<{
    fixes: ReturnType<typeof obsFixes>;
    inputs: ObsStatus["inputs"];
    collection: string | null;
  } | null>(null);
  const [restoreError, setRestoreError] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);
  const fixes = obsFixes(check);
  const change = status?.lastChange;
  const changing = restoring || change?.state === "applying" || change?.state === "restoring";
  const dialogAvailable =
    check.view === "connected" &&
    !!actions &&
    status?.sceneCollection === fixing?.collection &&
    !!fixing?.fixes.every(({ change }) => {
      const original = fixing.inputs.find((input) => input.name === change.inputName);
      const current = status?.inputs.find((input) => input.name === change.inputName);
      return original?.uuid && current?.uuid === original.uuid;
    });
  useEffect(() => {
    if (fixing && !dialogAvailable) setFixing(null);
  }, [fixing, dialogAvailable]);
  const changeText =
    restoreError ??
    (change?.state === "applying"
      ? "Changing OBS…"
      : change?.state === "restoring"
        ? "Restoring OBS settings…"
        : change?.state === "applied"
          ? `Changed ${change.changed} OBS setting${change.changed === 1 ? "" : "s"}.`
          : change?.state === "restored"
            ? "OBS settings restored."
            : change?.state === "failed"
              ? change.error
                ? `${change.changed > 0 ? `${change.changed} setting${change.changed === 1 ? "" : "s"} completed before it stopped. ` : ""}${CHANGE_ERRORS[change.error]}`
                : "The change did not complete."
              : "");
  return (
    <section className="panel app-card" aria-labelledby="app-card-obs" data-app="obs">
      <header className="app-card-heading">
        <span className="app-monogram is-wide" aria-hidden="true">
          OBS
        </span>
        <h3 id="app-card-obs">OBS Studio</h3>
        <AppStateBadge state={check.state} />
      </header>
      <p className="muted app-card-summary" role="status">
        {check.summary}
      </p>
      {status && check.view === "connect" && <ConnectSteps status={status} actions={actions} />}
      {check.sources.length > 0 && (
        <table className="obs-sources">
          <caption className="app-label-hidden">
            {check.view === "connected" ? "What OBS hears" : "What OBS heard when it last saved"}
          </caption>
          <thead>
            <tr>
              <th scope="col">Source</th>
              <th scope="col">Device</th>
              <th scope="col">Hears</th>
            </tr>
          </thead>
          <tbody>
            {check.sources.map((row) => (
              <tr key={row.name} className={row.problem ? "is-check" : row.muted ? "is-muted" : undefined}>
                <th scope="row" title={row.name}>
                  {row.name}
                  {row.muted && <small> (muted)</small>}
                </th>
                <td title={row.device}>{row.device}</td>
                <td>
                  {row.hears.length > 0 ? row.hears.join(" · ") : "—"}
                  {row.problem && (
                    <small className="obs-problem">
                      <span aria-hidden="true">! </span>
                      {row.problem}
                    </small>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {check.view === "connected" && (
        <div className="obs-actions">
          <button
            type="button"
            className="primary"
            disabled={!actions || !status || changing || fixes.length === 0}
            onClick={() => {
              if (!status) return;
              setRestoreError(null);
              setFixing({
                fixes,
                inputs: status.inputs.map((input) => ({ ...input })),
                collection: status.sceneCollection,
              });
            }}
          >
            Fix in OBS
          </button>
          <button
            type="button"
            className="secondary"
            disabled={!actions || changing || !change?.restorable}
            onClick={() => {
              if (!actions) return;
              setRestoreError(null);
              setRestoring(true);
              actions
                .restore()
                .then(() => actions.poll())
                .catch((reason: unknown) => setRestoreError(errorText(reason)))
                .finally(() => setRestoring(false));
            }}
          >
            Restore previous OBS settings
          </button>
          <button
            type="button"
            className="app-link-button"
            disabled={!actions}
            onClick={() => {
              if (!actions) return;
              void actions
                .disconnect()
                .then(() => actions.poll())
                .catch(() => actions.poll());
            }}
          >
            Disconnect
          </button>
        </div>
      )}
      {check.view === "connected" && (
        <p className="muted obs-note" role="status">
          {changeText}
        </p>
      )}
      {fixing && actions && dialogAvailable && (
        <FixDialog fixes={fixing.fixes} inputs={fixing.inputs} actions={actions} onClose={() => setFixing(null)} />
      )}
    </section>
  );
}
