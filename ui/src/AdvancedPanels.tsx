// Panels of the Advanced side-panel tab (moved from App.tsx).
import { useEffect, useEffectEvent, useRef, useState, type ChangeEvent } from "react";
import type { Node, RecordingRecoveryItem } from "@audiorouter/contracts";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";
import { infoMessage, say, type UiMessage } from "./actionMessage";
import { formatUiError, uiError, type ClientRow, type UiBackend } from "./backend";
import { backendText } from "./actionText";
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
  const tr = useMessages();
  const [items, setItems] = useState<RecordingRecoveryItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const unavailableText = useEffectEvent(() => tr("g3Adv.recUnavailable"));
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
          setError(formatUiError(reason, unavailableText()));
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
          <p className="eyebrow">{tr("g3Adv.recEyebrow")}</p>
          <h2 id="recovery-checkpoints-heading">{tr("g3Adv.recHeading")}</h2>
        </div>
        <span className="badge">{error ? tr("g3Adv.badgeUnavailable") : items.length}</span>
      </div>
      {error ? (
        <p className="muted">{error}</p>
      ) : items.length === 0 ? (
        <p className="muted">{tr("g3Adv.recNone")}</p>
      ) : (
        <ul aria-label={tr("g3Adv.recList")}>
          {items.map((item) => (
            <li key={item.recordingId}>
              <code>{item.recordingId}</code> — {item.status}
              {item.checkpoint ? ` (${item.checkpoint.state})` : ""}
            </li>
          ))}
        </ul>
      )}
      <p className="muted">{tr("g3Adv.recReadOnly")}</p>
    </section>
  );
}

export function StartupPanel({ backend }: { backend: UiBackend }) {
  const tr = useMessages();
  const [status, setStatus] = useState<import("@audiorouter/contracts").StartupStatus | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [message, setMessage] = useState<UiMessage | null>(null);
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
        if (clearMessage) setMessage(null);
      })
      .catch((error) => {
        if (generation === refreshGeneration.current) setMessage(uiError(error, tr("g3Adv.startupUnavailable")));
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
  // Refresh when the backend connects or changes, with the latest state.
  const refreshOnChange = useEffectEvent(() => refresh());
  useEffect(() => {
    refreshOnChange();
  }, [backend, backend.connected]);
  // The checkbox shows what Windows actually does at sign-in when the host can tell,
  // otherwise the saved preference.
  useEffect(() => {
    if (nativeRegistration !== "unavailable") setEnabled(nativeRegistration === "registered");
    else if (status) setEnabled(status.enabled);
  }, [nativeRegistration, status]);
  // One click: the backend plans and applies the change, then the desktop app registers it with Windows.
  const change = async (next: boolean) => {
    if (busy || !backend.connected) return;
    setBusy(true);
    setMessage(say(next ? "g3Adv.turningOn" : "g3Adv.turningOff"));
    try {
      const plan = await backend.planStartup(next);
      await backend.applyStartup(plan.planId, uiIdempotencyKey("startup-apply"));
      const planned = plan.enabled;
      if (backend.registerStartup) {
        try {
          await backend.registerStartup(planned);
          setNativeRegistration(planned ? "registered" : "unregistered");
          setEnabled(planned);
          setMessage(say(planned ? "g3Adv.willStart" : "g3Adv.willNotStart"));
        } catch (error) {
          setMessage(say("g3Adv.signInFailed", { reason: formatUiError(error, tr("g3Adv.signInSetupFailed")) }));
        }
      } else {
        setEnabled(planned);
        setMessage(infoMessage(plan.reason));
      }
      refresh(false);
    } catch (error) {
      setMessage(uiError(error, tr("g3Adv.startupChangeFailed")));
    } finally {
      setBusy(false);
    }
  };
  const stateLabel = !backend.connected
    ? tr("g3Adv.unavailable")
    : nativeRegistration === "registered"
      ? tr("g3Adv.on")
      : nativeRegistration === "unregistered"
        ? tr("g3Adv.off")
        : status === null
          ? tr("g3Adv.loading")
          : status.enabled
            ? tr("g3Adv.on")
            : tr("g3Adv.off");
  return (
    <section className="panel startup-panel" aria-labelledby="startup-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("settingsCopy.startupEyebrow")}</p>
          <h2 id="startup-heading">{tr("settingsCopy.startupHeading")}</h2>
        </div>
        <button type="button" className="secondary" onClick={() => refresh()} disabled={busy}>
          {tr("g3Adv.refresh")}
        </button>
      </div>
      <div className="setting-toggle-row">
        <label className="autoplay-toggle">
          <input
            type="checkbox"
            checked={enabled}
            disabled={!backend.connected || busy}
            aria-describedby="startup-state"
            aria-busy={busy}
            onChange={(event) => void change(event.target.checked)}
          />
          {tr("settingsCopy.startupToggle")}
        </label>
        <span className="setting-toggle-state" id="startup-state" data-testid="startup-state">
          {stateLabel}
        </span>
      </div>
      <p className="muted">
        {tr(backend.registerStartup ? "settingsCopy.startupNative" : "settingsCopy.startupPreview")}
      </p>
      <p className="muted" role="status">
        {message?.text ?? backendText(status?.reason) ?? " "}
      </p>
    </section>
  );
}

export function OsTransitionPanel({ backend, onRefresh }: { backend: UiBackend; onRefresh: () => void }) {
  const tr = useMessages();
  const [result, setResult] = useState<import("@audiorouter/contracts").OsTransitionResult | null>(null);
  const [message, setMessage] = useState<UiMessage | null>(null);
  const [busy, setBusy] = useState(false);
  const revalidate = async () => {
    if (!backend.connected || !backend.osTransition || busy) return;
    setBusy(true);
    setMessage(say("g3Adv.osRefreshing"));
    try {
      const next = await backend.osTransition("resume", uiIdempotencyKey("os-resume"));
      setResult(next);
      onRefresh();
      const waiting = next.sessionIds.length + next.nativeSessionIds.length;
      setMessage(
        next.action === "revalidateBeforeRestart"
          ? say(waiting === 1 ? "g3Adv.osRevalidate" : "g3Adv.osRevalidateOther", { n: waiting })
          : say("g3Adv.osNone"),
      );
    } catch (error) {
      setMessage(uiError(error, tr("g3Adv.osFailed")));
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
        failures.push(`${sessionId}: ${formatUiError(error, tr("g3Adv.startFailed"))}`);
      }
    }
    setMessage(
      failures.length === 0
        ? say(started === 1 ? "g3Adv.osRestarted" : "g3Adv.osRestartedOther", { n: started })
        : say("g3Adv.osRestartedFailed", { started, failed: failures.length, details: failures.join(" ") }),
    );
    onRefresh();
    setBusy(false);
  };
  return (
    <section className="panel os-transition-panel" aria-labelledby="os-transition-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g3Adv.osEyebrow")}</p>
          <h2 id="os-transition-heading">{tr("g3Adv.osHeading")}</h2>
        </div>
        <span className="badge">{result?.endpointInventory ?? tr("g3Adv.notRun")}</span>
      </div>
      <p className="muted">{tr("g3Adv.osIntro")}</p>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={() => void revalidate()}
          disabled={!backend.connected || !backend.osTransition || busy}
        >
          {tr("g3Adv.osRevalidateBtn")}
        </button>
        {result?.sessionIds.length ? (
          <button
            type="button"
            className="primary"
            onClick={() => void restartPortable()}
            disabled={!backend.connected || busy}
          >
            {tr("g3Adv.osRestartBtn")}
          </button>
        ) : null}
      </div>
      {result && (
        <p className="muted" role="status">
          {tr("g3Adv.osLast", {
            action: result.action,
            portable: result.sessionIds.length,
            native: result.nativeSessionIds.length,
          })}
        </p>
      )}
      <PanelMessage message={message} />
      <p className="muted">{tr("g3Adv.osNote")}</p>
    </section>
  );
}

export const CLIENT_ROLES = ["observer", "editor", "operator"] as const;

const ROLE_LABELS = {
  observer: "g3Adv.roleObserver",
  editor: "g3Adv.roleEditor",
  operator: "g3Adv.roleOperator",
} as const satisfies Record<(typeof CLIENT_ROLES)[number], MessageKey>;

export function ClientsPanel({ backend, initialClientId }: { backend: UiBackend; initialClientId?: string }) {
  const tr = useMessages();
  const roleLabel = (role: string) => (role in ROLE_LABELS ? tr(ROLE_LABELS[role as keyof typeof ROLE_LABELS]) : role);
  const [clients, setClients] = useState<ClientRow[]>([]);
  const [message, setMessage] = useState<UiMessage | null>(null);
  const [busy, setBusy] = useState(false);
  const [newClientId, setNewClientId] = useState("");
  const [newClientRole, setNewClientRole] = useState<(typeof CLIENT_ROLES)[number]>("observer");
  // Owner decision A1 (2026-11-27): recording control is a separate opt-in,
  // off unless ticked for this authorization.
  const [newClientRecords, setNewClientRecords] = useState(false);
  const refreshGeneration = useRef(0);
  useEffect(() => {
    if (initialClientId) setNewClientId(initialClientId);
  }, [initialClientId]);
  const refresh = () => {
    const generation = ++refreshGeneration.current;
    void backend
      .listClients()
      .then((result) => {
        if (generation === refreshGeneration.current) setClients(result);
      })
      .catch((error) => {
        if (generation === refreshGeneration.current) setMessage(uiError(error, tr("g3Adv.clientListUnavailable")));
      });
  };
  // Refresh when the backend connects or changes, with the latest state.
  const refreshOnChange = useEffectEvent(() => refresh());
  useEffect(() => {
    refreshOnChange();
  }, [backend, backend.connected]);
  const authorize = async () => {
    const clientId = newClientId.trim();
    if (!clientId || busy || !backend.connected) return;
    setBusy(true);
    try {
      await backend.authorizeClient(
        clientId,
        newClientRole,
        uiIdempotencyKey(`client-authorize-${clientId}`),
        newClientRecords,
      );
      setMessage(
        say(newClientRecords ? "g3Adv.authorizedRecording" : "g3Adv.authorized", {
          id: clientId,
          role: roleLabel(newClientRole).toLowerCase(),
        }),
      );
      setNewClientId("");
      setNewClientRecords(false);
      refresh();
    } catch (error) {
      setMessage(uiError(error, tr("g3Adv.authorizeFailed")));
    } finally {
      setBusy(false);
    }
  };
  const revoke = async (clientId: string) => {
    if (busy || !backend.connected) return;
    setBusy(true);
    try {
      await backend.revokeClient(clientId, uiIdempotencyKey(`client-revoke-${clientId}`));
      setMessage(say("g3Adv.revoked", { id: clientId }));
      refresh();
    } catch (error) {
      setMessage(uiError(error, tr("g3Adv.revokeFailed")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel clients-panel" aria-labelledby="clients-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g3Adv.secEyebrow")}</p>
          <h2 id="clients-heading">{tr("g3Adv.clientsHeading")}</h2>
        </div>
        <button type="button" className="secondary" onClick={refresh} disabled={busy}>
          {tr("g3Adv.refresh")}
        </button>
      </div>
      <p className="muted">{tr("g3Adv.clientsIntro")}</p>
      {clients.length === 0 ? (
        <p className="muted">{tr("g3Adv.noClients")}</p>
      ) : (
        <ul aria-label={tr("g3Adv.knownClients")}>
          {clients.map((client) => (
            <li key={client.clientId}>
              <strong>{client.clientId}</strong>{" "}
              <small>
                {roleLabel(client.role).toLowerCase()}
                {client.allowRecording && !client.revoked ? tr("g3Adv.recordingTag") : ""}
                {client.revoked ? tr("g3Adv.revokedTag") : ""}
              </small>{" "}
              {!client.revoked && (
                <button
                  type="button"
                  className="secondary"
                  disabled={!backend.connected || busy}
                  onClick={() => void revoke(client.clientId)}
                >
                  {tr("g3Adv.revoke")}
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      <div className="actions">
        <label>
          {tr("g3Adv.clientIdLabel")}
          <input
            type="text"
            aria-label={tr("g3Adv.newClientId")}
            value={newClientId}
            maxLength={120}
            disabled={!backend.connected || busy}
            onChange={(event) => setNewClientId(event.target.value)}
          />
        </label>
        <label>
          {tr("g3Adv.role")}
          <select
            aria-label={tr("g3Adv.newClientRole")}
            value={newClientRole}
            disabled={!backend.connected || busy}
            onChange={(event) => setNewClientRole(event.target.value as (typeof CLIENT_ROLES)[number])}
          >
            {CLIENT_ROLES.map((role) => (
              <option key={role} value={role}>
                {tr(ROLE_LABELS[role])}
              </option>
            ))}
          </select>
        </label>
        <label className="autoplay-toggle">
          <input
            type="checkbox"
            checked={newClientRecords}
            disabled={!backend.connected || busy}
            aria-describedby="client-recording-note"
            onChange={(event) => setNewClientRecords(event.target.checked)}
          />
          {tr("g3Adv.allowRecording")}
        </label>
      </div>
      <p className="muted" id="client-recording-note">
        {tr("g3Adv.allowRecordingNote")}
      </p>
      <div>
        <button
          type="button"
          className="secondary"
          disabled={!backend.connected || busy || !newClientId.trim()}
          onClick={() => void authorize()}
        >
          {tr("g3Adv.authorize")}
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
  node: Node | undefined;
  backend: UiBackend;
}) {
  const tr = useMessages();
  return (
    <section className="panel processor-catalog" aria-labelledby="processor-catalog-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g3Adv.dspEyebrow")}</p>
          <h2 id="processor-catalog-heading">{tr("g3Adv.procHeading")}</h2>
        </div>
        <span className="badge">{processors?.length ?? 0}</span>
      </div>
      {error ? (
        <p className="muted" role="status">
          {tr("g3Adv.procUnavailable", { error: backendText(error) })}
        </p>
      ) : processors === null ? (
        <p className="muted">{tr("g3Adv.procConnect")}</p>
      ) : processors.length === 0 ? (
        <p className="muted">{tr("g3Adv.procNone")}</p>
      ) : (
        <ul aria-label={tr("g3Adv.procList")}>
          {processors.map((processor) => (
            <li key={`${processor.id}@${processor.version}`}>
              <strong>{processor.id}</strong>{" "}
              <small>
                {processor.category} · {processorAvailabilityText(processor, tr)} ·{" "}
                {processorLatencyText(processor, tr)}
              </small>
              <br />
              <small>{tr("g3Adv.parameters", { text: processorParametersText(processor, tr) })}</small>
            </li>
          ))}
        </ul>
      )}
      <p className="muted">{tr("g3Adv.procReadOnly")}</p>
      {node && <EqResponsePreview node={node} backend={backend} />}
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
  const tr = useMessages();
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
          <p className="eyebrow">{tr("g3Adv.presetEyebrow")}</p>
          <h2 id="preset-catalog-heading">{tr("g3Adv.presetHeading")}</h2>
        </div>
        <span className="badge">{presets ? presets.voiceChains.length + presets.eq.length : 0}</span>
      </div>
      {error ? (
        <p className="muted" role="status">
          {tr("g3Adv.presetUnavailable", { error: backendText(error) })}
        </p>
      ) : presets === null ? (
        <p className="muted">{tr("g3Adv.presetConnect")}</p>
      ) : (
        <ul aria-label={tr("g3Adv.presetsList")}>
          {presets.voiceChains.map((preset) => (
            <li key={"voice-" + preset.id}>
              <strong>{preset.name}</strong>{" "}
              <small>{tr("g3Adv.voiceChain", { description: preset.description })}</small>
              <button
                type="button"
                className="secondary"
                disabled={!presets}
                onClick={() => request("voiceChain", preset.id)}
              >
                {tr("g3Adv.addVoiceChain")}
              </button>
            </li>
          ))}
          {presets.eq.map((preset) => (
            <li key={"eq-" + preset.id}>
              <strong>{preset.name}</strong> <small>{tr("g3Adv.eqLine", { description: preset.description })}</small>
              <button type="button" className="secondary" disabled={!presets} onClick={() => request("eq", preset.id)}>
                {tr("g3Adv.addEq")}
              </button>
            </li>
          ))}
        </ul>
      )}
      <p className="muted">{tr("g3Adv.presetNote")}</p>
    </section>
  );
}

export function SessionTransferPanel({
  backend,
  session,
  onImported,
  quarantinedSessions = [],
  onRecovered,
}: {
  backend: UiBackend;
  session: import("@audiorouter/contracts").Session;
  onImported: (session: import("@audiorouter/contracts").Session) => void;
  quarantinedSessions?: import("@audiorouter/contracts").QuarantinedSession[];
  onRecovered?: () => void;
}) {
  const tr = useMessages();
  const [message, setMessage] = useState<UiMessage | null>(null);
  const [plan, setPlan] = useState<import("@audiorouter/contracts").SessionImportPlanResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  const exportOriginal = async (id: string) => {
    setBusy(true);
    try {
      if (!backend.exportRawSession) throw new Error(tr("g3Adv.reconnectExport"));
      const { document: original } = await backend.exportRawSession(id);
      const url = URL.createObjectURL(new Blob([original], { type: "text/plain;charset=utf-8" }));
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "audiorouter-unavailable-session.txt";
      anchor.click();
      URL.revokeObjectURL(url);
      setMessage(say("g3Adv.exportedOriginal"));
    } catch (error) {
      setMessage(uiError(error, tr("g3Adv.exportOriginalFailed")));
    } finally {
      setBusy(false);
    }
  };
  const deleteUnavailable = async (id: string) => {
    setBusy(true);
    try {
      await backend.deleteSession(id, uiIdempotencyKey("delete-unavailable"));
      setPendingDelete(null);
      setMessage(say("g3Adv.deletedUnavailable"));
      onRecovered?.();
    } catch (error) {
      setMessage(uiError(error, tr("g3Adv.deleteUnavailableFailed")));
    } finally {
      setBusy(false);
    }
  };
  const importRequest = useRef(0);
  const readFileText = (file: File) =>
    typeof file.text === "function"
      ? file.text()
      : new Promise<string>((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(String(reader.result ?? ""));
          reader.onerror = () => reject(reader.error ?? new Error(tr("g3Adv.readFailed")));
          reader.readAsText(file);
        });
  const exportSession = async () => {
    setMessage(say("g3Adv.exporting"));
    try {
      const exported = await backend.exportSession(session.id);
      const blob = new Blob([JSON.stringify(exported, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `${exported.name.replace(/[^a-z0-9-_]+/gi, "-").replace(/^-+|-+$/g, "") || "audiorouter-session"}.audiorouter.json`;
      anchor.click();
      URL.revokeObjectURL(url);
      setMessage(say("g3Adv.exported", { name: exported.name }));
    } catch (error) {
      setMessage(uiError(error, tr("g3Adv.exportFailed")));
    }
  };
  const inspectImport = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    const request = ++importRequest.current;
    setPlan(null);
    setBusy(true);
    setMessage(say("g3Adv.validating"));
    try {
      const candidate = JSON.parse(await readFileText(file)) as import("@audiorouter/contracts").Session;
      const next = await backend.planSessionImport(candidate);
      if (request !== importRequest.current) return;
      setPlan(next);
      setMessage(say("g3Adv.validated", { name: next.session.name }));
    } catch (error) {
      if (request === importRequest.current) {
        setPlan(null);
        setMessage(uiError(error, tr("g3Adv.validateFailed")));
      }
    } finally {
      if (request === importRequest.current) setBusy(false);
    }
  };
  const commitImport = async () => {
    if (!plan || busy || !backend.connected) return;
    setBusy(true);
    setMessage(say("g3Adv.committing"));
    try {
      const result = await backend.commitSessionImport(plan.planId, uiIdempotencyKey("session-import"));
      setPlan(null);
      onImported(result.session);
      setMessage(say("g3Adv.imported", { name: result.session.name }));
    } catch (error) {
      setMessage(uiError(error, tr("g3Adv.commitFailed")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel session-transfer-panel" aria-labelledby="session-transfer-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g3Adv.transferEyebrow")}</p>
          <h2 id="session-transfer-heading">{tr("g3Adv.transferHeading")}</h2>
        </div>
        <span className="badge">{tr("g3Adv.stoppedOnly")}</span>
      </div>
      <p className="muted">{tr("g3Adv.transferIntro")}</p>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={() => void exportSession()}
          disabled={!backend.connected || busy}
        >
          {tr("g3Adv.exportSession")}
        </button>
        <label className="file-picker">
          {tr("g3Adv.importSession")}
          <input
            aria-label={tr("g3Adv.importAria")}
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
            {tr("g3Adv.saveImported")}
          </button>
        )}
      </div>
      {plan && (
        <p className="muted" role="status">
          {tr("g3Adv.validatedImport", { name: plan.session.name, seconds: Math.ceil(plan.expiresInMs / 1000) })}
        </p>
      )}
      <PanelMessage message={message} />
      {quarantinedSessions.length > 0 && (
        <section aria-label={tr("g3Adv.unavailableSessions")}>
          <h3>{tr("g3Adv.unavailableSessions")}</h3>
          <p>{tr("g3Adv.unavailableIntro")}</p>
          <ul>
            {quarantinedSessions.map((item) => (
              <li key={item.sessionId}>
                <strong>{item.sessionId}</strong>
                <p>{item.reason}</p>
                <div className="actions">
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy || !backend.connected || !backend.exportRawSession}
                    onClick={() => void exportOriginal(item.sessionId)}
                  >
                    {tr("g3Adv.exportOriginal")}
                  </button>
                  {pendingDelete === item.sessionId ? (
                    <>
                      <button
                        type="button"
                        className="secondary"
                        disabled={busy || !backend.connected}
                        onClick={() => void deleteUnavailable(item.sessionId)}
                      >
                        {tr("g3Adv.confirmDeletion")}
                      </button>
                      <button
                        type="button"
                        className="secondary"
                        disabled={busy}
                        onClick={() => setPendingDelete(null)}
                      >
                        {tr("g3Adv.keepSession")}
                      </button>
                    </>
                  ) : (
                    <button
                      type="button"
                      className="secondary"
                      disabled={busy || !backend.connected}
                      onClick={() => setPendingDelete(item.sessionId)}
                    >
                      {tr("g3Adv.deleteUnavailable")}
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        </section>
      )}
    </section>
  );
}
