import { ToolGuideLink } from "./ToolGuideLink";
// Plug-in scanning, tool list and inspector panels (moved from App.tsx).
import { useEffect, useEffectEvent, useRef, useState } from "react";
import type { MessageKey } from "./i18n/generated/messageKeys";
import { useMessages } from "./i18n/useMessages";
import type { MethodResult, Node } from "@audiorouter/contracts";
import { infoMessage, say, type UiMessage } from "./actionMessage";
import { formatUiError, uiError, type UiBackend } from "./backend";
import { pluginCatalog, STANDARD_PLUGIN_FOLDERS, SUPPORTED_PLUGIN_COMPATIBILITY } from "./draft";
import { uiIdempotencyKey } from "./idempotency";
import { PanelMessage } from "./PanelMessage";
import { BooleanField } from "./BooleanField";
import { endLibraryDrag, startPluginDrag } from "./libraryDrag";

export function PluginScanPanel({
  backend,
  onAddPlaceholder,
  headingId = "plugin-scan-heading",
}: {
  backend: UiBackend;
  onAddPlaceholder?: (entry: import("@audiorouter/contracts").PluginScanEntry) => void;
  headingId?: string;
}) {
  const tr = useMessages();
  const [directory, setDirectory] = useState("");
  const [result, setResult] = useState<import("@audiorouter/contracts").PluginScanResult | null>(null);
  const [inspectionPath, setInspectionPath] = useState("");
  const [inspection, setInspection] = useState<import("@audiorouter/contracts").PluginScanEntry | null>(null);
  const [message, setMessage] = useState<UiMessage | null>(null);
  const [busy, setBusy] = useState(false);
  const requestGeneration = useRef(0);
  const directoryRef = useRef("");
  const inspectionPathRef = useRef("");
  useEffect(() => {
    directoryRef.current = directory;
    inspectionPathRef.current = inspectionPath;
  }, [directory, inspectionPath]);
  const scan = async (shellPath?: string) => {
    if (!directory.trim()) {
      setMessage(say("g3Plug.enterDir"));
      return;
    }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage(say("g3Plug.scanning"));
    try {
      const next = shellPath
        ? await backend.scanPlugins(requestedDirectory, shellPath)
        : await backend.scanPlugins(requestedDirectory);
      if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) {
        setResult(next);
        setInspection(null);
        setInspectionPath("");
        setMessage(say(shellPath ? "g3Plug.shellChecked" : "g3Plug.scanDone"));
      }
    } catch (error) {
      if (request === requestGeneration.current) {
        setResult(null);
        setInspection(null);
        setInspectionPath("");
        setMessage(uiError(error, tr("g3Plug.scanUnavailable")));
      }
    } finally {
      if (request === requestGeneration.current) setBusy(false);
    }
  };
  const list = async () => {
    if (!directory.trim()) {
      setMessage(say("g3Plug.enterDir"));
      return;
    }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage(say("g3Plug.loadingLast"));
    try {
      const next = await backend.listPlugins(requestedDirectory);
      if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) {
        setResult(next);
        setInspection(null);
        setInspectionPath("");
        setMessage(say("g3Plug.loadedLast"));
      }
    } catch (error) {
      if (request === requestGeneration.current) {
        setResult(null);
        setInspection(null);
        setInspectionPath("");
        setMessage(uiError(error, tr("g3Plug.inventoryUnavailable")));
      }
    } finally {
      if (request === requestGeneration.current) setBusy(false);
    }
  };
  const inspect = async () => {
    if (!inspectionPath.trim()) {
      setMessage(say("g3Plug.enterPath"));
      return;
    }
    if (busy || !backend.connected) return;
    const requestedPath = inspectionPath.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage(say("g3Plug.inspecting"));
    try {
      const next = await backend.inspectPlugin(requestedPath);
      if (request === requestGeneration.current && inspectionPathRef.current.trim() === requestedPath) {
        setInspection(next);
        setMessage(say("g3Plug.inspectDone"));
      }
    } catch (error) {
      if (request === requestGeneration.current) {
        setInspection(null);
        setMessage(uiError(error, tr("g3Plug.inspectUnavailable")));
      }
    } finally {
      if (request === requestGeneration.current) setBusy(false);
    }
  };
  const retry = async () => {
    if (!directory.trim()) {
      setMessage(say("g3Plug.enterDir"));
      return;
    }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage(say("g3Plug.retrying"));
    try {
      const next = await backend.retryPlugins(requestedDirectory, uiIdempotencyKey("plugins-retry"));
      if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) {
        setResult(next);
        setInspection(null);
        setInspectionPath("");
        setMessage(say("g3Plug.retryDone"));
      }
    } catch (error) {
      if (request === requestGeneration.current) {
        setResult(null);
        setInspection(null);
        setInspectionPath("");
        setMessage(uiError(error, tr("g3Plug.retryUnavailable")));
      }
    } finally {
      if (request === requestGeneration.current) setBusy(false);
    }
  };
  const selectInspectionPath = (path: string) => {
    setInspectionPath(path);
    setInspection(null);
    setMessage(say("g3Plug.selectedPath"));
  };
  const addToDraft = (entry: import("@audiorouter/contracts").PluginScanEntry) => {
    if (entry.identity && SUPPORTED_PLUGIN_COMPATIBILITY.includes(entry.identity.compatibility) && onAddPlaceholder) {
      onAddPlaceholder(entry);
    }
  };
  return (
    <section className="panel plugin-scan-panel" aria-labelledby={headingId} aria-busy={busy}>
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g3Plug.scanEyebrow")}</p>
          <h2 id={headingId}>{tr("g3Plug.scanHeading")}</h2>
        </div>
        <span className="badge">{result?.entries.length ?? 0}</span>
      </div>
      <p className="muted">{tr("settingsCopy.pluginScan")}</p>
      <label>
        {tr("g3Plug.absDir")}
        <input
          aria-label={tr("g3Plug.absDir")}
          value={directory}
          onChange={(event) => setDirectory(event.target.value)}
          disabled={!backend.connected}
          placeholder="C:\Plugins"
        />
      </label>
      <div className="actions">
        <button type="button" className="secondary" onClick={() => void scan()} disabled={!backend.connected || busy}>
          {tr("g3Plug.scanDir")}
        </button>
        <button type="button" className="secondary" onClick={() => void list()} disabled={!backend.connected || busy}>
          {tr("g3Plug.loadLast")}
        </button>
        <button type="button" className="secondary" onClick={() => void retry()} disabled={!backend.connected || busy}>
          {tr("g3Plug.retryScan")}
        </button>
      </div>
      {result && (
        <ul aria-label={tr("g3Plug.scanResults")}>
          {result.entries.length === 0 ? (
            <li className="muted">{tr("g3Plug.noCandidates")}</li>
          ) : (
            result.entries.map((entry) => {
              const supported = SUPPORTED_PLUGIN_COMPATIBILITY.includes(entry.identity?.compatibility ?? "");
              return (
                <li key={entry.path}>
                  <strong>{entry.path}</strong>{" "}
                  <small>
                    {entry.identity
                      ? `${entry.identity.format} · ${entry.identity.architecture} · ${entry.identity.compatibility}`
                      : `${entry.errorCode ?? tr("g3Plug.unknown")}: ${entry.error ?? tr("g3Plug.inspectFailed")}`}
                  </small>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => selectInspectionPath(entry.path)}
                    disabled={!backend.connected}
                  >
                    {tr("g3Plug.selectInspect")}
                  </button>
                  {entry.identity?.format === "vst2" && (
                    <button
                      type="button"
                      className="secondary"
                      disabled={!backend.connected || busy}
                      onClick={() => void scan(entry.path)}
                    >
                      {tr("g3Plug.findShell")}
                    </button>
                  )}
                  {supported && entry.identity?.format === "vst2" && entry.identity.classIds.length > 0
                    ? entry.identity.classIds.map((id) => (
                        <button
                          key={id}
                          type="button"
                          className="secondary"
                          disabled={!backend.connected || !onAddPlaceholder}
                          onClick={() => {
                            if (entry.identity)
                              addToDraft({ ...entry, identity: { ...entry.identity, classIds: [id] } });
                          }}
                        >
                          {tr("g3Plug.addNamed", {
                            name: entry.identity?.shellEffects?.find((effect) => effect.id === id)?.name ?? id,
                          })}
                        </button>
                      ))
                    : supported && (
                        <button
                          type="button"
                          className="secondary"
                          aria-label={tr("g3Plug.addToRouteAria", { path: entry.path })}
                          onClick={() => addToDraft(entry)}
                          disabled={!backend.connected || !onAddPlaceholder}
                        >
                          {tr("g3Plug.addToRoute")}
                        </button>
                      )}
                </li>
              );
            })
          )}
        </ul>
      )}
      <label>
        {tr("g3Plug.absPath")}
        <input
          aria-label={tr("g3Plug.absPath")}
          value={inspectionPath}
          onChange={(event) => {
            setInspectionPath(event.target.value);
            setInspection(null);
          }}
          disabled={!backend.connected}
          placeholder="C:\Plugins\effect.vst3 or effect.dll"
        />
      </label>
      <button type="button" className="secondary" onClick={() => void inspect()} disabled={!backend.connected || busy}>
        {tr("g3Plug.inspectPath")}
      </button>
      {inspection && (
        <p className="muted" role="status">
          {inspection.identity
            ? `${inspection.identity.format} ${inspection.identity.architecture} · ${inspection.identity.compatibility}`
            : `${inspection.errorCode ?? tr("g3Plug.unknown")}: ${inspection.error ?? tr("g3Plug.inspectFailed")}`}
        </p>
      )}
      <PanelMessage message={message} />
      <p className="muted">{tr("g3Plug.scanNote")}</p>
    </section>
  );
}

/**
 * "Plugins" group of the Tools tab: every supported plugin from remembered
 * scans, a one-click scan of Windows' standard plugin folders, and click to
 * add. Scans read metadata only; adding creates a stopped placeholder whose
 * binary is re-verified before it ever runs in an isolated worker.
 */
export function PluginToolsGroup({
  backend,
  connected,
  search,
  refreshKey,
  onAdd,
  onOpenPicker,
}: {
  backend: UiBackend;
  connected: boolean;
  search: string;
  refreshKey: boolean;
  onAdd: (entry: import("@audiorouter/contracts").PluginScanEntry) => void;
  onOpenPicker: () => void;
}) {
  const tr = useMessages();
  const [inventories, setInventories] = useState<import("@audiorouter/contracts").PluginScanResult[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<UiMessage | null>(null);
  const listUnavailable = useEffectEvent(() => tr("g3Plug.listUnavailable"));
  useEffect(() => {
    if (!connected || !backend.pluginInventory) return;
    let active = true;
    void backend
      .pluginInventory()
      .then((result) => {
        if (active) setInventories(result.inventories);
      })
      .catch((error) => {
        if (active) setMessage(uiError(error, listUnavailable()));
      });
    return () => {
      active = false;
    };
  }, [backend, connected, refreshKey]);
  const scanStandardFolders = async () => {
    setBusy(true);
    setMessage(say("g3Plug.scanningStd"));
    const results: import("@audiorouter/contracts").PluginScanResult[] = [];
    let denied: UiMessage | null = null;
    const failures: string[] = [];
    for (const folder of STANDARD_PLUGIN_FOLDERS) {
      try {
        results.push(await backend.scanPlugins(folder));
      } catch (error) {
        if (/permission/i.test(error instanceof Error ? error.message : "")) {
          denied = uiError(error, tr("g3Plug.notPermitted"));
          break;
        }
        // Missing optional standard folders are normal. Other failures must
        // not be presented as a successful scan finding zero plugins.
        const text = error instanceof Error ? error.message : String(error);
        if (!/invalidRoot|not found|does not exist/i.test(text)) failures.push(`${folder}: ${text}`);
      }
    }
    const merged = [
      ...inventories.filter((inventory) => !results.some((result) => result.directory === inventory.directory)),
      ...results,
    ];
    setInventories(merged);
    const found = pluginCatalog(results).length;
    setMessage(
      denied ??
        (failures.length
          ? uiError(
              new Error(tr("g3Plug.scanFailedFolders", { n: failures.length, failures: failures.join("; "), found })),
              tr("g3Plug.scanFailed"),
            )
          : say(
              results.length === 1
                ? found === 1
                  ? "g3Plug.foundStdOne"
                  : "g3Plug.foundStdOneOther"
                : found === 1
                  ? "g3Plug.foundStd"
                  : "g3Plug.foundStdOther",
              { found, folders: results.length },
            )),
    );
    setBusy(false);
  };
  const query = search.trim().toLocaleLowerCase();
  const catalog = pluginCatalog(inventories).filter(
    (item) =>
      !query ||
      `${item.name} ${item.format} ${item.entry.identity?.vendor ?? ""} plugin vst`.toLocaleLowerCase().includes(query),
  );
  const renderRows = (items: typeof catalog) =>
    items.map((item) => {
      const help = `${item.format}${item.entry.identity?.vendor ? ` · ${item.entry.identity.vendor}` : ""} · ${item.folder}`;
      return (
        <div
          className="tool-library-entry"
          key={`${item.entry.identity?.sha256}-${item.entry.path}-${item.entry.identity?.classIds?.[0] ?? "default"}`}
        >
          <button
            className="tool-card"
            type="button"
            disabled={!connected}
            title={help}
            aria-description={help}
            // Click adds the plugin; drag places it where it is dropped on the canvas.
            draggable={connected}
            onDragStart={(event) => startPluginDrag(event, item.entry, item.name)}
            onDragEnd={endLibraryDrag}
            onClick={() => onAdd(item.entry)}
          >
            <span className="tool-card-icon tool-card-plugin-badge" aria-hidden="true">
              {item.format === "VST3" ? "V3" : "V2"}
            </span>
            <span>
              <strong>{item.name}</strong>
              <small>{help}</small>
            </span>
          </button>
          {/* No per-row guide: every row opened the same generic VST guide under
                the plugin's own name, beside the section's "?" (owner, 2026-11-07). */}
        </div>
      );
    });
  const effects = catalog.filter((item) => !item.instrument);
  const instruments = catalog.filter((item) => item.instrument);
  return (
    <>
      {instruments.length > 0 && (
        <div className="tool-plugin-group" aria-label={tr("g3Plug.instruments")}>
          <div className="tool-library-entry">
            <h4>{tr("g3Plug.instrumentsHeading")}</h4>
            <ToolGuideLink kind="instrument" compact toolName="Instrument" />
          </div>
          {renderRows(instruments)}
        </div>
      )}
      <div className="tool-plugin-group" aria-label={tr("g3Plug.plugins")}>
        <div className="tool-library-entry">
          <h4>{tr("g3Plug.pluginsHeading")}</h4>
          <ToolGuideLink kind="plugin" compact toolName="VST plugin" />
        </div>
        {renderRows(effects)}
        {effects.length === 0 && !query && <p className="muted">{tr("g3Plug.noneYet")}</p>}
        <div className="actions">
          <button
            type="button"
            className="secondary"
            disabled={!connected || busy}
            onClick={() => void scanStandardFolders()}
          >
            {busy ? tr("g3Plug.scanningShort") : tr("g3Plug.scanStd")}
          </button>
          <button type="button" className="secondary" disabled={!connected} onClick={onOpenPicker}>
            {tr("g3Plug.scanAnother")}
          </button>
        </div>
        {message && <PanelMessage message={message} small />}
      </div>
    </>
  );
}

export function LoadedPluginsPanel({
  nodes,
  selectedNodeId,
  disabled,
  onSelect,
  onUnload,
  diagnostics,
}: {
  nodes: Node[];
  selectedNodeId: string;
  disabled: boolean;
  onSelect: (nodeId: string) => void;
  onUnload: (nodeId: string) => void;
  diagnostics: import("@audiorouter/contracts").DiagnosticsSnapshot | null;
}) {
  const tr = useMessages();
  const plugins = nodes.filter((node) => node.kind === "plugin");
  return (
    <section className="panel loaded-plugins-panel" aria-labelledby="loaded-plugins-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g3Plug.inRoute")}</p>
          <h2 id="loaded-plugins-heading">{tr("g3Plug.loadedHeading")}</h2>
        </div>
        <span className="badge">{plugins.length}</span>
      </div>
      {plugins.length === 0 ? (
        <p className="muted">{tr("g3Plug.noneInRoute")}</p>
      ) : (
        <ul aria-label={tr("g3Plug.loadedList")}>
          {plugins.map((node) => {
            const format = node.parameters.format;
            const formatLabel = format === "vst3" ? "VST3" : format === "vst2" ? "VST2" : tr("g3Plug.pluginWord");
            const fileName =
              String(node.parameters.path ?? "")
                .replace(/\\/g, "/")
                .split("/")
                .pop() || tr("g3Plug.unbound");
            const health = diagnostics?.nodeTelemetry.find((item) => item.nodeId === node.id)?.plugin ?? null;
            const healthAlert = health?.state === "failed" || health?.state === "quarantined";
            return (
              <li
                key={node.id}
                className={
                  [node.id === selectedNodeId ? "selected" : null, healthAlert ? "is-alert" : null]
                    .filter(Boolean)
                    .join(" ") || undefined
                }
              >
                <span>
                  <strong>{node.name}</strong>{" "}
                  <small>
                    {formatLabel} · {fileName} ·{" "}
                    {node.bypass ? tr("g3Plug.bypass") : node.enabled ? tr("g3Plug.active") : tr("g3Plug.stopped")}
                    {health &&
                      ` · ${tr("g3Plug.workerState", { state: tr(PLUGIN_HEALTH_LABELS[health.state] ?? "g3Plug.hUnknown").toLowerCase() })}`}
                  </small>
                </span>
                <button type="button" className="secondary" onClick={() => onSelect(node.id)} disabled={disabled}>
                  {tr("g3Plug.select")}
                </button>
                <button type="button" className="secondary" onClick={() => onUnload(node.id)} disabled={disabled}>
                  {tr("g3Plug.unload")}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

export const PLUGIN_HEALTH_LABELS: Record<string, MessageKey> = {
  unknown: "g3Plug.hUnknown",
  stopped: "g3Plug.hStopped",
  running: "g3Plug.hRunning",
  failed: "g3Plug.hFailed",
  quarantined: "g3Plug.hQuarantined",
};

/**
 * Plugin editor and settings persistence. The vendor editor opens in a
 * native window owned by the desktop shell (VST2); closing it applies its
 * edits to the audio. "Save plugin settings" stores the plugin's full state
 * with the route so it is restored the next time the route plays.
 */
export function PluginEditorControls({
  node,
  backend,
  sessionId,
  running,
  onStateSaved,
}: {
  node: Node;
  backend: UiBackend;
  sessionId: string;
  running: boolean;
  onStateSaved: (stateId: string) => void;
}) {
  const tr = useMessages();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<UiMessage | null>(null);
  const inShell = typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__?.invoke);
  const openEditor = async () => {
    const invoke = window.__TAURI_INTERNALS__?.invoke;
    if (!invoke) return;
    setBusy(true);
    try {
      const response = (await invoke("open_plugin_editor", { sessionId, nodeId: node.id, title: node.name })) as {
        error?: { message?: string } | null;
      };
      setMessage(
        response?.error
          ? say("g3Plug.editorUnavailable", { reason: response.error.message ?? tr("g3Plug.backendRefused") })
          : say("g3Plug.windowOpen"),
      );
    } catch (error) {
      setMessage(uiError(error, tr("g3Plug.editorFailed")));
    } finally {
      setBusy(false);
    }
  };
  const saveState = async () => {
    if (!backend.savePluginState) return;
    setBusy(true);
    try {
      const result = await backend.savePluginState(sessionId, node.id);
      onStateSaved(result.stateId);
      setMessage(say("g3Plug.stateSaved", { kb: Math.max(1, Math.round(result.sizeBytes / 1024)) }));
    } catch (error) {
      setMessage(uiError(error, tr("g3Plug.stateSaveFailed")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="node-binding-editor" aria-label={tr("g3Plug.editorAria")}>
      <div>
        <p className="eyebrow">{tr("g3Plug.adjust")}</p>
        <strong>{node.name}</strong>
      </div>
      <div className="actions">
        <button
          type="button"
          disabled={busy || !inShell}
          title={!inShell ? tr("g3Plug.onlyDesktop") : tr("g3Plug.openOwn")}
          onClick={() => void openEditor()}
        >
          {tr("g3Plug.openWindow")}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={busy || !running || !backend.savePluginState}
          title={!running ? tr("g3Plug.pressPlayFirst") : tr("g3Plug.keepNow")}
          onClick={() => void saveState()}
        >
          {tr("g3Plug.saveNow")}
        </button>
      </div>
      {message ? (
        <PanelMessage message={message} small />
      ) : (
        <small className="muted">
          {node.kind === "instrument"
            ? !inShell
              ? tr("g3Plug.instOnlyDesktop")
              : tr("g3Plug.instOpen")
            : !inShell
              ? tr("g3Plug.plugOnlyDesktop")
              : tr("g3Plug.plugOpen")}
        </small>
      )}
    </div>
  );
}

export function PluginRecoveryControls({
  node,
  backend,
  sessionId,
  running,
  disabledReason,
  onPrepared,
}: {
  node: Node;
  backend: Pick<UiBackend, "preparePluginUpdate">;
  sessionId: string;
  running: boolean;
  disabledReason?: string;
  onPrepared: (original: Node, parameters: Node["parameters"]) => boolean;
}) {
  const tr = useMessages();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<UiMessage | null>(null);
  const current = useRef(onPrepared);
  const mounted = useRef(true);
  useEffect(() => {
    current.current = onPrepared;
  }, [onPrepared]);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  async function prepare(reset: boolean) {
    setBusy(true);
    setMessage(say("g3Plug.recPreparing"));
    try {
      const result = await backend.preparePluginUpdate(node.id, sessionId, reset);
      if (!mounted.current) return;
      if (!current.current(node, result.parameters)) throw new Error(tr("g3Plug.recChanged"));
      setMessage(say(reset ? "g3Plug.recDefaults" : "g3Plug.recScanned"));
    } catch (error) {
      if (mounted.current) setMessage(uiError(error, tr("g3Plug.recFailed")));
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  return (
    <section aria-label={tr("g3Plug.recAria")}>
      <p>{tr("g3Plug.recIntro")}</p>
      <div className="actions">
        <button type="button" disabled={running || busy || !!disabledReason} onClick={() => void prepare(false)}>
          {tr("g3Plug.useScanned")}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={running || busy || !!disabledReason}
          onClick={() => void prepare(true)}
        >
          {tr("g3Plug.fromDefaults")}
        </button>
      </div>
      <div style={{ height: "7rem", overflow: "auto" }}>
        <PanelMessage
          message={disabledReason ? infoMessage(disabledReason) : running ? say("g3Plug.stopFirst") : message}
        />
      </div>
    </section>
  );
}

export function PluginNodeInspector(props: Parameters<typeof PluginNodeInspectorContent>[0]) {
  // Each instance owns its async replies as well as programs, presets and sidechain feedback.
  return (
    <PluginNodeInspectorContent
      key={JSON.stringify([
        props.sessionId,
        props.node.id,
        props.node.parameters.path,
        props.node.parameters.fingerprint,
        props.node.parameters.classId,
      ])}
      {...props}
    />
  );
}

function PluginNodeInspectorContent({
  node,
  disabled,
  onUnload,
  snapshot,
  backend,
  sessionId,
  sidechainBlockedReason,
  onChange,
  onRemove,
}: {
  node: Node;
  disabled: boolean;
  onUnload: () => void;
  snapshot: import("@audiorouter/contracts").DiagnosticsSnapshot | null;
  backend?: Pick<
    UiBackend,
    | "resumePlugin"
    | "listPluginPrograms"
    | "selectPluginProgram"
    | "applyPluginPreset"
    | "addPluginSidechainPort"
    | "removePluginSidechainPort"
  >;
  sessionId?: string;
  /** Why adding a sidechain port is unavailable right now (for example unsaved changes). */
  sidechainBlockedReason?: string;
  onChange?: (name: string, value: boolean) => void;
  /** Remove settings from the node; "off" removes the key so older builds can still load the route. */
  onRemove?: (names: string[]) => void;
}) {
  const tr = useMessages();
  const path = typeof node.parameters.path === "string" ? node.parameters.path : "";
  const format = node.parameters.format;
  const formatLabel = format === "vst3" ? "VST3" : format === "vst2" ? "VST2" : tr("g3Plug.pluginWord");
  const fingerprint = typeof node.parameters.fingerprint === "string" ? node.parameters.fingerprint : "";
  const fileName = path.replace(/\\/g, "/").split("/").pop() || path;
  const health = snapshot?.nodeTelemetry.find((item) => item.nodeId === node.id)?.plugin ?? null;
  const silenced = health?.silencedUnchanged === true;
  const healthAlert = health?.state === "failed" || health?.state === "quarantined";
  const passThroughAllowed = node.parameters.passThroughAllowed === true;
  const isVst2 = format === "vst2";
  const [programs, setPrograms] = useState<{ id: number; name: string }[] | null>(null);
  const [units, setUnits] = useState<MethodResult["plugins.programs"]["units"]>([]);
  const [selectedProgram, setSelectedProgram] = useState("");
  const [programBusy, setProgramBusy] = useState(false);
  const [programError, setProgramError] = useState<string | null>(null);
  const [presetPath, setPresetPath] = useState("");
  const loadPrograms = async () => {
    if (!backend?.listPluginPrograms || !sessionId) return;
    setProgramBusy(true);
    setProgramError(null);
    try {
      const result = await backend.listPluginPrograms(sessionId, node.id);
      setPrograms(result.programs);
      setUnits(result.units);
      setSelectedProgram(result.programs.length > 0 ? String(result.programs[0].id) : "");
    } catch (error) {
      setProgramError(formatUiError(error, tr("g3Plug.programsFailed")));
    } finally {
      setProgramBusy(false);
    }
  };
  const chooseProgram = async (raw: string) => {
    if (!backend?.selectPluginProgram || !sessionId) return;
    const program = Number(raw);
    setSelectedProgram(raw);
    setProgramError(null);
    try {
      await backend.selectPluginProgram(sessionId, node.id, program);
    } catch (error) {
      setProgramError(formatUiError(error, tr("g3Plug.programFailed")));
    }
  };
  const applyPreset = async () => {
    if (!backend?.applyPluginPreset || !sessionId || presetPath.trim() === "") return;
    setProgramBusy(true);
    setProgramError(null);
    try {
      await backend.applyPluginPreset(sessionId, node.id, presetPath.trim());
    } catch (error) {
      setProgramError(formatUiError(error, tr("g3Plug.presetFailed")));
    } finally {
      setProgramBusy(false);
    }
  };
  const hasSidechainPort = node.ports.some((port) => port.name === "sidechain");
  const [sidechain, setSidechain] = useState<{ busy: boolean; error: string | null; added: boolean }>({
    busy: false,
    error: null,
    added: false,
  });
  const addSidechain = async () => {
    if (!backend?.addPluginSidechainPort || !sessionId) return;
    setSidechain({ busy: true, error: null, added: false });
    try {
      await backend.addPluginSidechainPort(sessionId, node.id);
      setSidechain({ busy: false, error: null, added: true });
    } catch (error) {
      setSidechain({
        busy: false,
        error: formatUiError(error, tr("g3Plug.noSidechain")),
        added: false,
      });
    }
  };
  const removeSidechain = async () => {
    if (!backend?.removePluginSidechainPort || !sessionId) return;
    setSidechain({ busy: true, error: null, added: false });
    try {
      await backend.removePluginSidechainPort(sessionId, node.id);
      setSidechain({ busy: false, error: null, added: false });
    } catch (error) {
      setSidechain({
        busy: false,
        error: formatUiError(error, tr("g3Plug.sidechainRemoveFailed")),
        added: false,
      });
    }
  };
  // The result of the last Retry while this plugin stays silenced.
  const [retry, setRetry] = useState<{ pending: boolean; refused: boolean; error: string | null }>({
    pending: false,
    refused: false,
    error: null,
  });
  const [retriedWhileSilenced, setRetriedWhileSilenced] = useState(silenced);
  if (retriedWhileSilenced !== silenced) {
    // A new silence starts with a clean Retry state.
    setRetriedWhileSilenced(silenced);
    setRetry({ pending: false, refused: false, error: null });
  }
  const resume = async () => {
    if (!backend?.resumePlugin || !sessionId) return;
    setRetry({ pending: true, refused: false, error: null });
    try {
      const result = await backend.resumePlugin(sessionId, node.id);
      setRetry({ pending: false, refused: !result.resumed, error: null });
    } catch (error) {
      setRetry({ pending: false, refused: false, error: formatUiError(error, tr("g3Plug.retryFailed")) });
    }
  };
  // What the Voice protection card says. Every status is laid out in the same
  // grid cell, so the card keeps one size whichever is shown (UI-17).
  const guard: "watching" | "allowed" | "hint" | "muted" = silenced
    ? "muted"
    : passThroughAllowed
      ? "allowed"
      : health?.unchangedAudio
        ? "hint"
        : "watching";
  const shownWhen = (visible: boolean) => ({
    "aria-hidden": visible ? undefined : true,
    style: { visibility: visible ? ("visible" as const) : ("hidden" as const) },
  });
  const retryNote = retry.error ? "error" : retry.refused ? "refused" : "ask";
  return (
    <div className="node-binding-editor plugin-node-editor" aria-label={tr("g3Plug.bindingAria")}>
      <div>
        <p className="eyebrow">{tr("g3Plug.loadedFromDisk", { format: formatLabel })}</p>
        <strong title={path}>{fileName || tr("g3Plug.noBinary")}</strong>
      </div>
      <label>
        {tr("g3Plug.binaryPath")}
        <input aria-label={tr("g3Plug.binaryPathAria")} value={path} readOnly />
      </label>
      <label>
        {tr("g3Plug.sha")}
        <input aria-label={tr("g3Plug.shaAria")} value={fingerprint} readOnly />
      </label>
      {isVst2 || format === "vst3" ? (
        <section className="plugin-programs" aria-label={tr("g3Plug.programsPresets", { format: formatLabel })}>
          <span className="eyebrow">{tr("g3Plug.programsTitle", { format: formatLabel })}</span>
          <button
            type="button"
            disabled={programBusy || !backend?.listPluginPrograms || !sessionId}
            onClick={() => void loadPrograms()}
          >
            {programBusy ? tr("g3Plug.reading") : tr("g3Plug.loadPrograms")}
          </button>
          {programs ? (
            programs.length === 0 ? (
              <small>{tr("g3Plug.noPrograms")}</small>
            ) : (
              <label>
                {tr("g3Plug.program")}
                <select
                  aria-label={tr("g3Plug.programAria", { format: formatLabel })}
                  value={selectedProgram}
                  disabled={!isVst2 || !backend?.selectPluginProgram || !sessionId}
                  onChange={(event) => void chooseProgram(event.target.value)}
                >
                  {programs.map((program) => (
                    <option key={program.id} value={String(program.id)}>
                      {program.name}
                    </option>
                  ))}
                </select>
              </label>
            )
          ) : null}
          {format === "vst3" && units.length > 0 ? (
            <ul aria-label={tr("g3Plug.unitTree")} className="plugin-unit-list">
              {units.map((unit) => {
                const parent = units.find((candidate) => candidate.id === unit.parent_id);
                const names = unit.programs
                  .slice(0, 6)
                  .map((program) => program.name)
                  .join(", ");
                return (
                  <li key={unit.id}>
                    <span>{unit.name || tr("g3Plug.rootUnit")}</span>
                    {parent ? <small>{tr("g3Plug.within", { name: parent.name })}</small> : null}
                    {unit.program_list_id !== null ? (
                      <>
                        <small>
                          {tr(unit.program_count === 1 ? "g3Plug.programList" : "g3Plug.programListOther", {
                            n: unit.program_count,
                          })}
                        </small>
                        {unit.programs.length > 0 ? (
                          <small>
                            {unit.programs.length > 6
                              ? tr("g3Plug.programsMore", { names, n: unit.programs.length - 6 })
                              : tr("g3Plug.programsList", { names })}
                          </small>
                        ) : null}
                      </>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          ) : null}
          {isVst2 ? (
            <>
              <label>
                {tr("g3Plug.presetFile")}
                <input
                  aria-label={tr("g3Plug.presetPathAria")}
                  value={presetPath}
                  onChange={(event) => setPresetPath(event.target.value)}
                  placeholder={tr("g3Plug.presetPlaceholder")}
                />
              </label>
              <button
                type="button"
                disabled={programBusy || !backend?.applyPluginPreset || !sessionId || presetPath.trim() === ""}
                onClick={() => void applyPreset()}
              >
                {tr("g3Plug.importPreset")}
              </button>
            </>
          ) : (
            <small>{tr("g3Plug.vst3ReadOnly")}</small>
          )}
          {programError ? <small role="alert">{programError}</small> : null}
        </section>
      ) : null}
      {format === "vst3" || (isVst2 && (!node.parameters.classId || node.parameters.classId === "default")) ? (
        <section className="plugin-programs" aria-label={tr("g3Plug.sidechainAria")}>
          <span className="eyebrow">{tr("g3Plug.sidechainAria")}</span>
          {hasSidechainPort || sidechain.added ? (
            <>
              <small>{tr("g3Plug.sidechainHas")}</small>
              <button
                type="button"
                disabled={
                  disabled ||
                  sidechain.busy ||
                  !backend?.removePluginSidechainPort ||
                  !sessionId ||
                  sidechainBlockedReason !== undefined
                }
                onClick={() => void removeSidechain()}
              >
                {sidechain.busy ? tr("g3Plug.removing") : tr("g3Plug.removeSidechain")}
              </button>
            </>
          ) : (
            <>
              <button
                type="button"
                disabled={
                  disabled ||
                  sidechain.busy ||
                  !backend?.addPluginSidechainPort ||
                  !sessionId ||
                  sidechainBlockedReason !== undefined
                }
                onClick={() => void addSidechain()}
              >
                {sidechain.busy ? tr("g3Plug.checkingPlugin") : tr("g3Plug.addSidechain")}
              </button>
              <div className="plugin-guard-stack">
                <small {...shownWhen(sidechainBlockedReason === undefined)}>
                  {isVst2 ? tr("g3Plug.worksVst2") : tr("g3Plug.worksVst3")}
                </small>
                <small {...shownWhen(sidechainBlockedReason !== undefined)}>
                  {sidechainBlockedReason ?? tr("g3Plug.saveFirst")}
                </small>
              </div>
            </>
          )}
          {sidechain.error ? <small role="alert">{sidechain.error}</small> : null}
        </section>
      ) : null}
      <section
        className={`plugin-voice-guard${silenced ? " is-alert" : ""}`}
        role={silenced ? "alert" : undefined}
        aria-label={tr("g3Plug.guardAria")}
      >
        <span className="eyebrow">{tr("g3Plug.guardEyebrow")}</span>
        <div className="plugin-guard-stack" aria-live="polite">
          <div {...shownWhen(guard === "watching")}>
            <strong>{tr("g3Plug.guardOn")}</strong>
            <small>{tr("g3Plug.guardOnText")}</small>
          </div>
          <div {...shownWhen(guard === "hint")}>
            <strong>{tr("g3Plug.guardUnchanged")}</strong>
            <small>{tr("g3Plug.guardUnchangedText")}</small>
          </div>
          <div {...shownWhen(guard === "allowed")}>
            <strong>{tr("g3Plug.guardOff")}</strong>
            <small>{tr("g3Plug.guardOffText")}</small>
          </div>
          <div {...shownWhen(guard === "muted")}>
            <strong className="plugin-guard-title">
              {retry.refused ? tr("g3Plug.stillMuted") : tr("g3Plug.muted")}
            </strong>
            <div className="plugin-guard-step">
              <span className="plugin-guard-number" aria-hidden="true">
                1
              </span>
              <span>
                <strong>{tr("g3Plug.expected")}</strong>
                <span className="plugin-guard-stack">
                  <small {...shownWhen(silenced && retryNote === "ask")}>{tr("g3Plug.notLicensed")}</small>
                  <small {...shownWhen(silenced && retryNote === "refused")}>{tr("g3Plug.stillPasses")}</small>
                  <small {...shownWhen(silenced && retryNote === "error")} role={retry.error ? "alert" : undefined}>
                    {retry.error}
                  </small>
                </span>
              </span>
              <button
                type="button"
                tabIndex={silenced ? undefined : -1}
                disabled={!silenced || retry.pending || !backend?.resumePlugin || !sessionId}
                onClick={() => void resume()}
              >
                {retry.pending ? tr("g3Plug.retrying2") : tr("g3Plug.retry")}
              </button>
            </div>
          </div>
        </div>
        <div className="plugin-guard-step">
          <span className="plugin-guard-number" {...shownWhen(silenced)}>
            2
          </span>
          <span>
            <strong>{tr("g3Plug.meantUnchanged")}</strong>
            <BooleanField
              label={tr("g3Plug.allowUnchanged")}
              checked={passThroughAllowed}
              disabled={disabled || !onChange}
              onChange={(checked) => {
                if (checked) onChange?.("passThroughAllowed", true);
                else if (onRemove) onRemove(["passThroughAllowed"]);
                else onChange?.("passThroughAllowed", false);
              }}
            />
            <small className="parameter-help">{tr("g3Plug.allowHelp")}</small>
          </span>
        </div>
      </section>
      <div
        className={`plugin-worker-health${healthAlert ? " is-alert" : ""}`}
        role={healthAlert ? "alert" : undefined}
        aria-label={tr("g3Plug.healthAria")}
      >
        <span className="eyebrow">{tr("g3Plug.healthEyebrow")}</span>
        <strong>
          {!health
            ? tr("g3Plug.notRunning")
            : silenced && health.state === "running"
              ? tr("g3Plug.silenced")
              : PLUGIN_HEALTH_LABELS[health.state]
                ? tr(PLUGIN_HEALTH_LABELS[health.state])
                : health.state}
        </strong>
        {health && health.failureCount > 0 && (
          <small>
            {tr(
              health.state === "quarantined"
                ? health.failureCount === 1
                  ? "g3Plug.failuresQuarantined"
                  : "g3Plug.failuresQuarantinedOther"
                : health.failureCount === 1
                  ? "g3Plug.failures"
                  : "g3Plug.failuresOther",
              { n: health.failureCount },
            )}
          </small>
        )}
        {health?.failureReason && (
          <small className="plugin-failure-reason">{tr("g3Plug.why", { reason: health.failureReason })}</small>
        )}
        {!health && <small>{tr("g3Plug.healthOnly")}</small>}
        {health?.outputMisses !== undefined && (
          <small>{tr("g3Plug.continuity", { misses: health.outputMisses, drops: health.inputDrops ?? 0 })}</small>
        )}
      </div>
      <button type="button" className="secondary" onClick={onUnload} disabled={disabled}>
        {tr("g3Plug.removePlugin")}
      </button>
      <small>{tr("g3Plug.footnote")}</small>
    </div>
  );
}
