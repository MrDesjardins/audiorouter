import { useEffect, useRef, useState } from "react";
import { AppLogo } from "./appLogos";
import type { ObsChangeRequest, ObsPreview, ObsStatus } from "@audiorouter/contracts";
import { AppStateBadge } from "./AppsPanel";
import { obsFixes, type ObsCheck } from "./obsUsage";
import { boldMarkup } from "./boldMarkup";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

export interface ObsActions {
  connect(password?: string): Promise<unknown>;
  disconnect(): Promise<unknown>;
  preview(changes: ObsChangeRequest[]): Promise<ObsPreview>;
  apply(previewId: string): Promise<unknown>;
  restore(): Promise<unknown>;
  /** Read the status again soon, after an action. */
  poll(): void;
}

const CHANGE_ERRORS = {
  changedInObs: "g2.obsErrChanged",
  refusedByObs: "g2.obsErrRefused",
  connectionLost: "g2.obsErrLost",
} as const satisfies Record<NonNullable<ObsStatus["lastChange"]["error"]>, MessageKey>;

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** The steps to turn on OBS's WebSocket server, or a password field when OBS asks for one. */
function ConnectSteps({ status, actions }: { status: ObsStatus; actions: ObsActions | null }) {
  const tr = useMessages();
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
        <li>{boldMarkup(tr("g2.obsStep1"))}</li>
        <li>{boldMarkup(tr("g2.obsStep2"))}</li>
        <li>{tr("g2.obsStep3")}</li>
      </ol>
      {askPassword && (
        <label className="obs-password">
          <span>{status.connection === "wrongPassword" ? tr("g2.obsPwWrong") : tr("g2.obsPwAsk")}</span>
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
            <span className={connecting ? "app-label-hidden" : undefined}>{tr("g2.obsConnect")}</span>
            <span className={connecting ? undefined : "app-label-hidden"}>{tr("g2.obsConnecting")}</span>
          </span>
        </button>
      </div>
      <p className="muted obs-note" role="status">
        {error ??
          (status.connection === "serverOff"
            ? tr("g2.obsServerOff")
            : status.connection === "failed"
              ? tr("g2.obsFailed")
              : tr("g2.obsPwNote"))}
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
  const tr = useMessages();
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
        if (!matches) throw new Error(tr("g2.obsChangedMeanwhile"));
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
        <h2 id="obs-fix-heading">{tr("g2.obsFixTitle")}</h2>
        <p id="obs-fix-description">{boldMarkup(tr("g2.obsFixDesc"))}</p>
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
            {busy
              ? tr("g2.obsChanging")
              : selected.length === 1
                ? tr("g2.obsChangeOne")
                : tr("g2.obsChangeMany", { n: selected.length })}
          </button>
          <button type="button" className="secondary" disabled={busy} onClick={onClose}>
            {tr("g2.dsCancel")}
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
  const tr = useMessages();
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
      ? tr("g2.obsChanging")
      : change?.state === "restoring"
        ? tr("g2.obsRestoring")
        : change?.state === "applied"
          ? change.changed === 1
            ? tr("g2.obsChangedOne")
            : tr("g2.obsChangedMany", { n: change.changed })
          : change?.state === "restored"
            ? tr("g2.obsRestored")
            : change?.state === "failed"
              ? change.error
                ? `${change.changed === 1 ? tr("g2.obsPartialOne") : change.changed > 0 ? tr("g2.obsPartialMany", { n: change.changed }) : ""}${tr(CHANGE_ERRORS[change.error])}`
                : tr("g2.obsNotComplete")
              : "");
  return (
    <section className="panel app-card" aria-labelledby="app-card-obs" data-app="obs">
      <header className="app-card-heading">
        <AppLogo app="obs" />
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
            {check.view === "connected" ? tr("g2.obsHears") : tr("g2.obsHeard")}
          </caption>
          <thead>
            <tr>
              <th scope="col">{tr("g2.obsColSource")}</th>
              <th scope="col">{tr("g2.obsColDevice")}</th>
              <th scope="col">{tr("g2.obsColHears")}</th>
            </tr>
          </thead>
          <tbody>
            {check.sources.map((row) => (
              <tr key={row.name} className={row.problem ? "is-check" : row.muted ? "is-muted" : undefined}>
                <th scope="row" title={row.name}>
                  {row.name}
                  {row.muted && <small>{tr("g2.obsMuted")}</small>}
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
            {tr("g2.obsFixTitle")}
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
            {tr("g2.obsRestore")}
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
            {tr("g2.obsDisconnect")}
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
