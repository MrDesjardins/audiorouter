// Panels of the Advanced side-panel tab (moved from App.tsx).
import { useEffect, useRef, useState, type ChangeEvent } from "react";
import type { Node, RecordingRecoveryItem } from "@audiorouter/contracts";
import { formatUiError, type ClientRow, type UiBackend } from "./backend";
import { uiIdempotencyKey } from "./idempotency";
import {
  processorAvailabilityText,
  processorLatencyText,
  processorParametersText,
  type ProcessorDescriptor,
} from "./processorCatalog";
import { PanelMessage } from "./PanelMessage";
import { EqResponsePreview } from "./NodeEditors";

export function RecoveryCheckpointPanel({ backend }: { backend: UiBackend }) {
  const [items, setItems] = useState<RecordingRecoveryItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    void backend
      .listRecordingRecovery()
      .then((result) => {
        if (active) {
          setItems(result.items);
          setError(null);
        }
      })
      .catch((reason) => {
        if (active) {
          setItems([]);
          setError(formatUiError(reason, "Recording recovery unavailable."));
        }
      });
    return () => {
      active = false;
    };
  }, [backend]);
  return (
    <section className="panel recovery-checkpoints" aria-labelledby="recovery-checkpoints-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Recorder files</p>
          <h2 id="recovery-checkpoints-heading">Recovery checkpoints</h2>
        </div>
        <span className="badge">{error ? "unavailable" : items.length}</span>
      </div>
      {error ? (
        <p className="muted">{error}</p>
      ) : items.length === 0 ? (
        <p className="muted">No persisted recorder checkpoints were found.</p>
      ) : (
        <ul aria-label="Persisted recorder checkpoints">
          {items.map((item) => (
            <li key={item.recordingId}>
              <code>{item.recordingId}</code> — {item.status}
              {item.checkpoint ? ` (${item.checkpoint.state})` : ""}
            </li>
          ))}
        </ul>
      )}
      <p className="muted">
        This list is read-only. Recovery inspection does not open, repair, play, or delete audio files.
      </p>
    </section>
  );
}

export function StartupPanel({ backend }: { backend: UiBackend }) {
  const [status, setStatus] = useState<import("@audiorouter/contracts").StartupStatus | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [plan, setPlan] = useState<import("@audiorouter/contracts").StartupPlanResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [nativeRegistration, setNativeRegistration] = useState<"registered" | "unregistered" | "unavailable">(
    "unavailable",
  );
  const refreshGeneration = useRef(0);
  const [busy, setBusy] = useState(false);
  const refresh = (clearMessage = true) => {
    const generation = ++refreshGeneration.current;
    void backend
      .getStartup()
      .then((result) => {
        if (generation !== refreshGeneration.current) return;
        setStatus(result);
        setEnabled(result.enabled);
        if (clearMessage) setMessage(null);
      })
      .catch((error) => {
        if (generation === refreshGeneration.current) setMessage(formatUiError(error, "Startup status unavailable."));
      });
    if (backend.startupRegistrationStatus)
      void backend
        .startupRegistrationStatus()
        .then((result) => {
          if (generation === refreshGeneration.current) setNativeRegistration(result);
        })
        .catch(() => {
          if (generation === refreshGeneration.current) setNativeRegistration("unavailable");
        });
  };
  useEffect(() => {
    setPlan(null);
    refresh();
  }, [backend, backend.connected]);
  const createPlan = async () => {
    if (busy || !backend.connected) return;
    setBusy(true);
    setMessage("Planning sign-in startup policy...");
    try {
      const result = await backend.planStartup(enabled);
      setPlan(result);
      setMessage(result.reason);
    } catch (error) {
      setMessage(formatUiError(error, "Startup planning unavailable."));
    } finally {
      setBusy(false);
    }
  };
  const applyPlan = async () => {
    if (!plan || busy || !backend.connected) return;
    setBusy(true);
    const plannedEnabled = plan.enabled;
    setMessage("Applying startup policy...");
    try {
      const result = await backend.applyStartup(plan.planId, uiIdempotencyKey("startup-apply"));
      if (backend.registerStartup) {
        try {
          const commandLine = await backend.registerStartup(plannedEnabled);
          setNativeRegistration(plannedEnabled ? "registered" : "unregistered");
          setMessage(
            `${plannedEnabled ? "Startup registration enabled" : "Startup registration disabled"}: ${commandLine}`,
          );
        } catch (error) {
          setMessage(
            `Backend policy saved, but native startup registration failed: ${formatUiError(error, "native registration failed")}`,
          );
        }
      } else {
        setMessage(result.reason);
      }
      setPlan(null);
      refresh(false);
    } catch (error) {
      setMessage(formatUiError(error, "Startup apply unavailable."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel startup-panel" aria-labelledby="startup-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Background lifecycle</p>
          <h2 id="startup-heading">Start at sign-in</h2>
        </div>
        <button type="button" className="secondary" onClick={() => refresh()} disabled={busy}>
          Refresh
        </button>
      </div>
      <p className="muted">{status?.reason ?? "Loading startup capability..."}</p>
      <p className="muted" role="status">
        Native registration: {nativeRegistration}
      </p>
      <label>
        Desired policy
        <select
          aria-label="Desired sign-in startup policy"
          value={enabled ? "enabled" : "disabled"}
          onChange={(event) => {
            setEnabled(event.target.value === "enabled");
            setPlan(null);
          }}
          disabled={!backend.connected || busy}
        >
          <option value="disabled">Disabled</option>
          <option value="enabled">Enabled</option>
        </select>
      </label>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={() => void createPlan()}
          disabled={!backend.connected || busy}
        >
          Plan startup policy
        </button>
        {plan && (
          <button
            type="button"
            className="secondary"
            onClick={() => void applyPlan()}
            disabled={!backend.connected || busy}
          >
            Apply planned policy
          </button>
        )}
      </div>
      {message && <PanelMessage message={message} />}
      <p className="muted">
        {backend.registerStartup
          ? "The native shell can register this user's startup preference after an authorized plan is applied."
          : "Native startup registration is unavailable in this host; planning remains a backend-only operation."}
      </p>
    </section>
  );
}

export function OsTransitionPanel({ backend, onRefresh }: { backend: UiBackend; onRefresh: () => void }) {
  const [result, setResult] = useState<import("@audiorouter/contracts").OsTransitionResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const revalidate = async () => {
    if (!backend.connected || !backend.osTransition || busy) return;
    setBusy(true);
    setMessage("Refreshing endpoints and checking stopped routes...");
    try {
      const next = await backend.osTransition("resume", uiIdempotencyKey("os-resume"));
      setResult(next);
      onRefresh();
      setMessage(
        next.action === "revalidateBeforeRestart"
          ? `Revalidation required for ${next.sessionIds.length + next.nativeSessionIds.length} stopped route${next.sessionIds.length + next.nativeSessionIds.length === 1 ? "" : "s"}. No route was restarted.`
          : "No stopped routes are waiting for revalidation.",
      );
    } catch (error) {
      setMessage(formatUiError(error, "OS-transition revalidation failed."));
    } finally {
      setBusy(false);
    }
  };
  const restartPortable = async () => {
    if (!result || result.sessionIds.length === 0 || busy) return;
    setBusy(true);
    let started = 0;
    const failures: string[] = [];
    for (const sessionId of result.sessionIds) {
      try {
        await backend.startSession(sessionId, uiIdempotencyKey(`os-restart-${sessionId}`));
        started += 1;
      } catch (error) {
        failures.push(`${sessionId}: ${formatUiError(error, "start failed")}`);
      }
    }
    setMessage(
      failures.length === 0
        ? `Restarted ${started} validated portable route${started === 1 ? "" : "s"}. Native routes remain stopped.`
        : `Restarted ${started} portable route${started === 1 ? "" : "s"}; ${failures.length} route${failures.length === 1 ? "" : "s"} failed. ${failures.join(" ")}`,
    );
    onRefresh();
    setBusy(false);
  };
  return (
    <section className="panel os-transition-panel" aria-labelledby="os-transition-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Background lifecycle</p>
          <h2 id="os-transition-heading">Resume validation</h2>
        </div>
        <span className="badge">{result?.endpointInventory ?? "not run"}</span>
      </div>
      <p className="muted">
        After sleep, sign-out, or an endpoint change, refresh the exact endpoint inventory before restarting any stopped
        route.
      </p>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={() => void revalidate()}
          disabled={!backend.connected || !backend.osTransition || busy}
        >
          Revalidate after resume
        </button>
        {result?.sessionIds.length ? (
          <button
            type="button"
            className="primary"
            onClick={() => void restartPortable()}
            disabled={!backend.connected || busy}
          >
            Restart validated portable routes
          </button>
        ) : null}
      </div>
      {result && (
        <p className="muted" role="status">
          Last result: {result.action}; portable routes {result.sessionIds.length}; native routes{" "}
          {result.nativeSessionIds.length}.
        </p>
      )}
      <PanelMessage message={message} />
      <p className="muted">
        Native routes remain stopped until deliberate endpoint/driver validation. This action never changes Windows
        defaults, volume, mute, or driver state.
      </p>
    </section>
  );
}

export function GraphHistoryPanel({
  backend,
  session,
  onReverted,
}: {
  backend: UiBackend;
  session: import("@audiorouter/contracts").Session;
  onReverted: () => void;
}) {
  const [history, setHistory] = useState<import("@audiorouter/contracts").Session[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const refreshGeneration = useRef(0);
  const refresh = () => {
    const generation = ++refreshGeneration.current;
    void backend
      .listGraphHistory(session.id, undefined, 10)
      .then((page) => {
        if (generation === refreshGeneration.current) setHistory(page.items);
      })
      .catch((error) => {
        if (generation === refreshGeneration.current) setMessage(formatUiError(error, "Revision history unavailable."));
      });
  };
  useEffect(() => {
    refresh();
  }, [backend, backend.connected, session.id, session.revision]);
  const undoLast = async () => {
    if (busy || !backend.connected) return;
    setBusy(true);
    setMessage("Undoing the last committed change...");
    try {
      const plan = await backend.undoGraphPlan(session.id, session.revision);
      await backend.commitGraph(plan.planId, plan.baseRevision, uiIdempotencyKey("graph-undo"));
      setMessage("Reverted to the previous committed revision.");
      onReverted();
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to undo the last committed change."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel graph-history-panel" aria-labelledby="graph-history-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Committed history</p>
          <h2 id="graph-history-heading">Revision history</h2>
        </div>
        <button type="button" className="secondary" onClick={refresh} disabled={busy}>
          Refresh
        </button>
      </div>
      <p className="muted">
        This lists revisions the backend has actually committed, separate from the local draft undo above the canvas.
        Undoing here reverts the authoritative graph to the state before the last commit; running it again steps to the
        revision before that.
      </p>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          disabled={!backend.connected || busy || history.length < 2}
          onClick={() => void undoLast()}
        >
          Undo last committed change
        </button>
      </div>
      {history.length === 0 ? (
        <p className="muted">No committed history is available yet.</p>
      ) : (
        <ol aria-label="Committed revisions">
          {history.map((entry) => (
            <li key={entry.revision}>
              <strong>Revision {entry.revision}</strong>{" "}
              <small>
                {entry.nodes.length} node{entry.nodes.length === 1 ? "" : "s"} · {entry.edges.length} connection
                {entry.edges.length === 1 ? "" : "s"}
              </small>
            </li>
          ))}
        </ol>
      )}
      <PanelMessage message={message} />
    </section>
  );
}

export const CLIENT_ROLES = ["observer", "editor", "operator"] as const;

export function ClientsPanel({ backend }: { backend: UiBackend }) {
  const [clients, setClients] = useState<ClientRow[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [newClientId, setNewClientId] = useState("");
  const [newClientRole, setNewClientRole] = useState<(typeof CLIENT_ROLES)[number]>("observer");
  const refreshGeneration = useRef(0);
  const refresh = () => {
    const generation = ++refreshGeneration.current;
    void backend
      .listClients()
      .then((result) => {
        if (generation === refreshGeneration.current) setClients(result);
      })
      .catch((error) => {
        if (generation === refreshGeneration.current) setMessage(formatUiError(error, "Client list unavailable."));
      });
  };
  useEffect(() => {
    refresh();
  }, [backend, backend.connected]);
  const authorize = async () => {
    const clientId = newClientId.trim();
    if (!clientId || busy || !backend.connected) return;
    setBusy(true);
    try {
      await backend.authorizeClient(clientId, newClientRole, uiIdempotencyKey(`client-authorize-${clientId}`));
      setMessage(`Authorized ${clientId} as ${newClientRole}.`);
      setNewClientId("");
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to authorize client."));
    } finally {
      setBusy(false);
    }
  };
  const revoke = async (clientId: string) => {
    if (busy || !backend.connected) return;
    setBusy(true);
    try {
      await backend.revokeClient(clientId, uiIdempotencyKey(`client-revoke-${clientId}`));
      setMessage(`Revoked ${clientId}.`);
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to revoke client."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel clients-panel" aria-labelledby="clients-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Security</p>
          <h2 id="clients-heading">Connected clients</h2>
        </div>
        <button type="button" className="secondary" onClick={refresh} disabled={busy}>
          Refresh
        </button>
      </div>
      <p className="muted">
        Every CLI, MCP, or shell connection is a separately authorized client. Revoke access immediately if a client
        should no longer reach this backend.
      </p>
      {clients.length === 0 ? (
        <p className="muted">No clients are known yet.</p>
      ) : (
        <ul aria-label="Known clients">
          {clients.map((client) => (
            <li key={client.clientId}>
              <strong>{client.clientId}</strong>{" "}
              <small>
                {client.role}
                {client.revoked ? " · revoked" : ""}
              </small>{" "}
              {!client.revoked && (
                <button
                  type="button"
                  className="secondary"
                  disabled={!backend.connected || busy}
                  onClick={() => void revoke(client.clientId)}
                >
                  Revoke
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      <div className="actions">
        <label>
          Client ID
          <input
            type="text"
            aria-label="New client ID"
            value={newClientId}
            maxLength={120}
            disabled={!backend.connected || busy}
            onChange={(event) => setNewClientId(event.target.value)}
          />
        </label>
        <label>
          Role
          <select
            aria-label="New client role"
            value={newClientRole}
            disabled={!backend.connected || busy}
            onChange={(event) => setNewClientRole(event.target.value as (typeof CLIENT_ROLES)[number])}
          >
            {CLIENT_ROLES.map((role) => (
              <option key={role} value={role}>
                {role.charAt(0).toUpperCase()}
                {role.slice(1)}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="secondary"
          disabled={!backend.connected || busy || !newClientId.trim()}
          onClick={() => void authorize()}
        >
          Authorize client
        </button>
      </div>
      <PanelMessage message={message} />
    </section>
  );
}

export function ProcessorCatalog({
  processors,
  error,
  node,
  backend,
}: {
  processors: ProcessorDescriptor[] | null;
  error: string | null;
  node: Node;
  backend: UiBackend;
}) {
  return (
    <section className="panel processor-catalog" aria-labelledby="processor-catalog-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">DSP catalog</p>
          <h2 id="processor-catalog-heading">Built-in processors</h2>
        </div>
        <span className="badge">{processors?.length ?? 0}</span>
      </div>
      {error ? (
        <p className="muted" role="status">
          Processor catalog unavailable: {error}
        </p>
      ) : processors === null ? (
        <p className="muted">Connect to the backend to load the authoritative processor catalog.</p>
      ) : processors.length === 0 ? (
        <p className="muted">No built-in processors are advertised.</p>
      ) : (
        <ul aria-label="Built-in processor catalog">
          {processors.map((processor) => (
            <li key={`${processor.id}@${processor.version}`}>
              <strong>{processor.id}</strong>{" "}
              <small>
                {processor.category} · {processorAvailabilityText(processor)} · {processorLatencyText(processor)}
              </small>
              <br />
              <small>Parameters: {processorParametersText(processor)}</small>
            </li>
          ))}
        </ul>
      )}
      <p className="muted">This catalog is read-only. Unavailable processors cannot be added or activated.</p>
      <EqResponsePreview node={node} backend={backend} />
    </section>
  );
}

export function PresetCatalog({
  presets,
  error,
}: {
  presets: import("@audiorouter/contracts").DiscoveryDocument["presets"] | null;
  error: string | null;
}) {
  const request = (kind: "eq" | "voiceChain", presetId: string) => {
    globalThis.dispatchEvent(
      new CustomEvent(kind === "eq" ? "audiorouter:append-eq-preset" : "audiorouter:append-voice-preset", {
        detail: { presetId },
      }),
    );
  };
  return (
    <section className="panel preset-catalog" aria-labelledby="preset-catalog-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Saved starting points</p>
          <h2 id="preset-catalog-heading">Presets</h2>
        </div>
        <span className="badge">{presets ? presets.voiceChains.length + presets.eq.length : 0}</span>
      </div>
      {error ? (
        <p className="muted" role="status">
          Preset catalog unavailable: {error}
        </p>
      ) : presets === null ? (
        <p className="muted">Connect to the backend to load the authoritative preset catalog.</p>
      ) : (
        <ul aria-label="Available presets">
          {presets.voiceChains.map((preset) => (
            <li key={"voice-" + preset.id}>
              <strong>{preset.name}</strong> <small>Voice chain · {preset.description}</small>
              <button
                type="button"
                className="secondary"
                disabled={!presets}
                onClick={() => request("voiceChain", preset.id)}
              >
                Add voice chain to draft
              </button>
            </li>
          ))}
          {presets.eq.map((preset) => (
            <li key={"eq-" + preset.id}>
              <strong>{preset.name}</strong> <small>EQ · {preset.description}</small>
              <button type="button" className="secondary" disabled={!presets} onClick={() => request("eq", preset.id)}>
                Add EQ to draft
              </button>
            </li>
          ))}
        </ul>
      )}
      <p className="muted">Presets expand into ordinary draft nodes; all actions remain subject to Plan changes.</p>
    </section>
  );
}

export function SessionTransferPanel({
  backend,
  session,
  onImported,
}: {
  backend: UiBackend;
  session: import("@audiorouter/contracts").Session;
  onImported: (session: import("@audiorouter/contracts").Session) => void;
}) {
  const [message, setMessage] = useState<string | null>(null);
  const [plan, setPlan] = useState<import("@audiorouter/contracts").SessionImportPlanResult | null>(null);
  const [busy, setBusy] = useState(false);
  const importRequest = useRef(0);
  const readFileText = (file: File) =>
    typeof file.text === "function"
      ? file.text()
      : new Promise<string>((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(String(reader.result ?? ""));
          reader.onerror = () => reject(reader.error ?? new Error("Unable to read import file."));
          reader.readAsText(file);
        });
  const exportSession = async () => {
    setMessage("Exporting the selected stopped-session configuration...");
    try {
      const exported = await backend.exportSession(session.id);
      const blob = new Blob([JSON.stringify(exported, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `${exported.name.replace(/[^a-z0-9-_]+/gi, "-").replace(/^-+|-+$/g, "") || "audiorouter-session"}.audiorouter.json`;
      anchor.click();
      URL.revokeObjectURL(url);
      setMessage(`Exported ${exported.name}. Credentials, grants, recordings, and plugin binaries are not included.`);
    } catch (error) {
      setMessage(formatUiError(error, "Unable to export session."));
    }
  };
  const inspectImport = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    const request = ++importRequest.current;
    setPlan(null);
    setBusy(true);
    setMessage("Validating the selected session import...");
    try {
      const candidate = JSON.parse(await readFileText(file)) as import("@audiorouter/contracts").Session;
      const next = await backend.planSessionImport(candidate);
      if (request !== importRequest.current) return;
      setPlan(next);
      setMessage(`Import validated for ${next.session.name}; it will remain stopped until you commit it.`);
    } catch (error) {
      if (request === importRequest.current) {
        setPlan(null);
        setMessage(formatUiError(error, "Unable to validate session import."));
      }
    } finally {
      if (request === importRequest.current) setBusy(false);
    }
  };
  const commitImport = async () => {
    if (!plan || busy || !backend.connected) return;
    setBusy(true);
    setMessage("Committing the validated stopped-session import...");
    try {
      const result = await backend.commitSessionImport(plan.planId, uiIdempotencyKey("session-import"));
      setPlan(null);
      onImported(result.session);
      setMessage(`Imported stopped session ${result.session.name}. Review bindings before starting it.`);
    } catch (error) {
      setMessage(formatUiError(error, "Unable to commit session import."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel session-transfer-panel" aria-labelledby="session-transfer-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Portable configuration</p>
          <h2 id="session-transfer-heading">Session transfer</h2>
        </div>
        <span className="badge">stopped only</span>
      </div>
      <p className="muted">
        Graph-only JSON transfer for scripts and review. To back up or move a whole setup (including imported audio and
        plugin settings), use Session → Session file instead. Imports never start audio, arm recorders, enable startup,
        or include credentials, recordings, plugin binaries, or machine-specific authorization.
      </p>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={() => void exportSession()}
          disabled={!backend.connected || busy}
        >
          Export session
        </button>
        <label className="file-picker">
          Import session
          <input
            aria-label="Import session configuration"
            type="file"
            accept=".json,.audiorouter.json,application/json"
            onChange={(event) => void inspectImport(event)}
            disabled={!backend.connected || busy}
          />
        </label>
        {plan && (
          <button
            type="button"
            className="primary"
            onClick={() => void commitImport()}
            disabled={!backend.connected || busy}
          >
            Commit stopped import
          </button>
        )}
      </div>
      {plan && (
        <p className="muted" role="status">
          Validated import: {plan.session.name} · expires in {Math.ceil(plan.expiresInMs / 1000)} seconds. Explicit
          commit is required.
        </p>
      )}
      <PanelMessage message={message} />
    </section>
  );
}
