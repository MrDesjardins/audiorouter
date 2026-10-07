// Plug-in scanning, tool list and inspector panels (moved from App.tsx).
import { useEffect, useRef, useState } from "react";
import type { Node } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { pluginCatalog, STANDARD_PLUGIN_FOLDERS } from "./draft";
import { uiIdempotencyKey } from "./idempotency";
import { PanelMessage } from "./PanelMessage";

export function PluginScanPanel({
  backend,
  onAddPlaceholder,
  headingId = "plugin-scan-heading",
}: {
  backend: UiBackend;
  onAddPlaceholder?: (entry: import("@audiorouter/contracts").PluginScanEntry) => void;
  headingId?: string;
}) {
  const [directory, setDirectory] = useState("");
  const [result, setResult] = useState<import("@audiorouter/contracts").PluginScanResult | null>(null);
  const [inspectionPath, setInspectionPath] = useState("");
  const [inspection, setInspection] = useState<import("@audiorouter/contracts").PluginScanEntry | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const requestGeneration = useRef(0);
  const directoryRef = useRef("");
  const inspectionPathRef = useRef("");
  useEffect(() => {
    directoryRef.current = directory;
    inspectionPathRef.current = inspectionPath;
  }, [directory, inspectionPath]);
  const scan = async () => {
    if (!directory.trim()) {
      setMessage("Enter an absolute plugin directory.");
      return;
    }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage("Scanning selected directory...");
    try {
      const next = await backend.scanPlugins(requestedDirectory);
      if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) {
        setResult(next);
        setInspection(null);
        setInspectionPath("");
        setMessage("Plugin scan completed without loading plugin code.");
      }
    } catch (error) {
      if (request === requestGeneration.current) {
        setResult(null);
        setInspection(null);
        setInspectionPath("");
        setMessage(formatUiError(error, "Plugin scan unavailable."));
      }
    } finally {
      if (request === requestGeneration.current) setBusy(false);
    }
  };
  const list = async () => {
    if (!directory.trim()) {
      setMessage("Enter an absolute plugin directory.");
      return;
    }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage("Loading the last explicit plugin scan...");
    try {
      const next = await backend.listPlugins(requestedDirectory);
      if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) {
        setResult(next);
        setInspection(null);
        setInspectionPath("");
        setMessage("Loaded the last backend scan without rescanning.");
      }
    } catch (error) {
      if (request === requestGeneration.current) {
        setResult(null);
        setInspection(null);
        setInspectionPath("");
        setMessage(formatUiError(error, "Plugin inventory unavailable."));
      }
    } finally {
      if (request === requestGeneration.current) setBusy(false);
    }
  };
  const inspect = async () => {
    if (!inspectionPath.trim()) {
      setMessage("Enter an absolute plugin path.");
      return;
    }
    if (busy || !backend.connected) return;
    const requestedPath = inspectionPath.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage("Inspecting selected plugin path...");
    try {
      const next = await backend.inspectPlugin(requestedPath);
      if (request === requestGeneration.current && inspectionPathRef.current.trim() === requestedPath) {
        setInspection(next);
        setMessage("Plugin inspection completed without loading plugin code.");
      }
    } catch (error) {
      if (request === requestGeneration.current) {
        setInspection(null);
        setMessage(formatUiError(error, "Plugin inspection unavailable."));
      }
    } finally {
      if (request === requestGeneration.current) setBusy(false);
    }
  };
  const retry = async () => {
    if (!directory.trim()) {
      setMessage("Enter an absolute plugin directory.");
      return;
    }
    if (busy || !backend.connected) return;
    const requestedDirectory = directory.trim();
    const request = ++requestGeneration.current;
    setBusy(true);
    setMessage("Retrying selected directory scan...");
    try {
      const next = await backend.retryPlugins(requestedDirectory, uiIdempotencyKey("plugins-retry"));
      if (request === requestGeneration.current && directoryRef.current.trim() === requestedDirectory) {
        setResult(next);
        setInspection(null);
        setInspectionPath("");
        setMessage("Plugin scan retry completed without loading plugin code.");
      }
    } catch (error) {
      if (request === requestGeneration.current) {
        setResult(null);
        setInspection(null);
        setInspectionPath("");
        setMessage(formatUiError(error, "Plugin scan retry unavailable."));
      }
    } finally {
      if (request === requestGeneration.current) setBusy(false);
    }
  };
  const selectInspectionPath = (path: string) => {
    setInspectionPath(path);
    setInspection(null);
    setMessage("Selected the discovered path; inspect it explicitly when ready.");
  };
  const addToDraft = (entry: import("@audiorouter/contracts").PluginScanEntry) => {
    if (
      entry.identity &&
      ["supportedVst2X64Gated", "supportedVst3X64"].includes(entry.identity.compatibility) &&
      onAddPlaceholder
    ) {
      onAddPlaceholder(entry);
    }
  };
  return (
    <section className="panel plugin-scan-panel" aria-labelledby={headingId}>
      <div className="section-heading">
        <div>
          <p className="eyebrow">VST3 and VST2 discovery</p>
          <h2 id={headingId}>Plugin scan</h2>
        </div>
        <span className="badge">{result?.entries.length ?? 0}</span>
      </div>
      <p className="muted">
        Choose a directory explicitly. Discovery inspects bounded metadata only; it does not load or execute plugins.
      </p>
      <label>
        Absolute plugin directory
        <input
          aria-label="Absolute plugin directory"
          value={directory}
          onChange={(event) => setDirectory(event.target.value)}
          disabled={!backend.connected}
          placeholder="C:\Plugins"
        />
      </label>
      <button type="button" className="secondary" onClick={() => void scan()} disabled={!backend.connected}>
        Scan directory
      </button>
      <button type="button" className="secondary" onClick={() => void list()} disabled={!backend.connected}>
        Load last scan
      </button>
      <button type="button" className="secondary" onClick={() => void retry()} disabled={!backend.connected}>
        Retry scan
      </button>
      {result && (
        <ul aria-label="Plugin scan results">
          {result.entries.length === 0 ? (
            <li className="muted">No VST3, VST2, or other DLL candidates found.</li>
          ) : (
            result.entries.map((entry) => {
              const supported =
                entry.identity?.compatibility === "supportedVst2X64Gated" ||
                entry.identity?.compatibility === "supportedVst3X64";
              return (
                <li key={entry.path}>
                  <strong>{entry.path}</strong>{" "}
                  <small>
                    {entry.identity
                      ? `${entry.identity.format} · ${entry.identity.architecture} · ${entry.identity.compatibility}`
                      : `${entry.errorCode ?? "unknown"}: ${entry.error ?? "inspection failed"}`}
                  </small>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => selectInspectionPath(entry.path)}
                    disabled={!backend.connected}
                  >
                    Select for inspection
                  </button>
                  {supported && (
                    <button
                      type="button"
                      className="secondary"
                      aria-label={`Add to draft: ${entry.path}`}
                      onClick={() => addToDraft(entry)}
                      disabled={!backend.connected || !onAddPlaceholder}
                    >
                      Add to draft
                    </button>
                  )}
                </li>
              );
            })
          )}
        </ul>
      )}
      <label>
        Absolute plugin path
        <input
          aria-label="Absolute plugin path"
          value={inspectionPath}
          onChange={(event) => {
            setInspectionPath(event.target.value);
            setInspection(null);
          }}
          disabled={!backend.connected}
          placeholder="C:\Plugins\effect.vst3 or effect.dll"
        />
      </label>
      <button type="button" className="secondary" onClick={() => void inspect()} disabled={!backend.connected}>
        Inspect path
      </button>
      {inspection && (
        <p className="muted" role="status">
          {inspection.identity
            ? `${inspection.identity.format} ${inspection.identity.architecture} · ${inspection.identity.compatibility}`
            : `${inspection.errorCode ?? "unknown"}: ${inspection.error ?? "inspection failed"}`}
        </p>
      )}
      <PanelMessage message={message} />
      <p className="muted">
        The explicit <code>pluginScan</code> permission is required by the backend; selecting a result only copies its
        path, and inspection remains explicit. Adding a result to the graph is a separate explicit action. Loading the
        last scan never triggers a new filesystem scan.
      </p>
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
  const [inventories, setInventories] = useState<import("@audiorouter/contracts").PluginScanResult[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  useEffect(() => {
    if (!connected || !backend.pluginInventory) return;
    let active = true;
    void backend
      .pluginInventory()
      .then((result) => {
        if (active) setInventories(result.inventories);
      })
      .catch((error) => {
        if (active) setMessage(formatUiError(error, "Plugin list unavailable."));
      });
    return () => {
      active = false;
    };
  }, [backend, connected, refreshKey]);
  const scanStandardFolders = async () => {
    setBusy(true);
    setMessage("Scanning standard plugin folders…");
    const results: import("@audiorouter/contracts").PluginScanResult[] = [];
    let denied: string | null = null;
    for (const folder of STANDARD_PLUGIN_FOLDERS) {
      try {
        results.push(await backend.scanPlugins(folder));
      } catch (error) {
        if (/permission/i.test(error instanceof Error ? error.message : "")) {
          denied = formatUiError(error, "Plugin scanning is not permitted.");
          break;
        }
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
        `Found ${found} supported plugin${found === 1 ? "" : "s"} in ${results.length} standard folder${results.length === 1 ? "" : "s"}. Other folders: use Scan another folder.`,
    );
    setBusy(false);
  };
  const query = search.trim().toLocaleLowerCase();
  const catalog = pluginCatalog(inventories).filter(
    (item) =>
      !query ||
      `${item.name} ${item.format} ${item.entry.identity?.vendor ?? ""} plugin vst`.toLocaleLowerCase().includes(query),
  );
  return (
    <div className="tool-plugin-group" aria-label="Plugins">
      <h4>Plugins (VST2/VST3)</h4>
      {catalog.map((item) => {
        const help = `${item.format}${item.entry.identity?.vendor ? ` · ${item.entry.identity.vendor}` : ""} · ${item.folder}`;
        return (
          <button
            key={`${item.entry.identity?.sha256}-${item.entry.path}`}
            className="tool-card"
            type="button"
            disabled={!connected}
            title={help}
            aria-description={help}
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
        );
      })}
      {catalog.length === 0 && !query && (
        <p className="muted">No plugins found yet. Scan Windows' standard plugin folders, or pick another folder.</p>
      )}
      <div className="actions">
        <button
          type="button"
          className="secondary"
          disabled={!connected || busy}
          onClick={() => void scanStandardFolders()}
        >
          {busy ? "Scanning…" : "Scan standard folders"}
        </button>
        <button type="button" className="secondary" disabled={!connected} onClick={onOpenPicker}>
          Scan another folder…
        </button>
      </div>
      {message && <PanelMessage message={message} small />}
    </div>
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
  const plugins = nodes.filter((node) => node.kind === "plugin");
  return (
    <section className="panel loaded-plugins-panel" aria-labelledby="loaded-plugins-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Currently in this draft</p>
          <h2 id="loaded-plugins-heading">Loaded plugins</h2>
        </div>
        <span className="badge">{plugins.length}</span>
      </div>
      {plugins.length === 0 ? (
        <p className="muted">No VST2/VST3 plugin is loaded into the draft yet. Scan a directory below and add one.</p>
      ) : (
        <ul aria-label="Loaded plugins">
          {plugins.map((node) => {
            const format = node.parameters.format;
            const formatLabel = format === "vst3" ? "VST3" : format === "vst2" ? "VST2" : "Plugin";
            const fileName =
              String(node.parameters.path ?? "")
                .replace(/\\/g, "/")
                .split("/")
                .pop() || "unbound";
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
                    {formatLabel} · {fileName} · {node.bypass ? "bypass" : node.enabled ? "active" : "stopped"}
                    {health && ` · worker ${health.state}`}
                  </small>
                </span>
                <button type="button" className="secondary" onClick={() => onSelect(node.id)} disabled={disabled}>
                  Select
                </button>
                <button type="button" className="secondary" onClick={() => onUnload(node.id)} disabled={disabled}>
                  Unload
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

export const PLUGIN_HEALTH_LABELS: Record<string, string> = {
  unknown: "Unknown",
  stopped: "Stopped",
  running: "Running",
  failed: "Failed",
  quarantined: "Quarantined",
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
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const inShell = typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__?.invoke);
  const isVst2 = node.parameters.format === "vst2";
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
          ? `Editor unavailable: ${response.error.message ?? "the backend refused"}`
          : "Editor opened in its own window. Changes apply as you make them and are kept automatically when you close the window or press Stop.",
      );
    } catch (error) {
      setMessage(formatUiError(error, "The plugin editor could not be opened."));
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
      setMessage(
        `Plugin settings saved (${Math.max(1, Math.round(result.sizeBytes / 1024))} KB). They are restored every time this route plays.`,
      );
    } catch (error) {
      setMessage(formatUiError(error, "Plugin settings could not be saved."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="node-binding-editor" aria-label="Plugin editor and settings">
      <div>
        <p className="eyebrow">Plugin editor</p>
        <strong>Your changes are kept automatically</strong>
      </div>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          disabled={busy || !running || !inShell || !isVst2}
          title={
            !isVst2
              ? "VST3 editors are not supported yet; use the parameters below."
              : !inShell
                ? "Editors open only in the desktop app."
                : !running
                  ? "Start the route first."
                  : "Open the plugin's own window"
          }
          onClick={() => void openEditor()}
        >
          Open plugin editor
        </button>
        <button
          type="button"
          className="secondary"
          disabled={busy || !running || !backend.savePluginState}
          title={
            !running
              ? "Start the route first."
              : "Keep the current settings now, without stopping. Changes are also kept automatically when you close the editor or press Stop."
          }
          onClick={() => void saveState()}
        >
          Save plugin settings
        </button>
      </div>
      {message ? (
        <PanelMessage message={message} small />
      ) : (
        <small className="muted">
          {!running
            ? "Start the route to open the editor. Settings you change are kept automatically when you close the editor or press Stop."
            : !isVst2
              ? "This VST3 plugin's own editor cannot be opened yet; adjust it with the parameters below. Changes are kept when you press Stop."
              : "Open the editor and make changes. They are kept when you close its window or press Stop."}
        </small>
      )}
    </div>
  );
}

export function PluginNodeInspector({
  node,
  disabled,
  onUnload,
  snapshot,
}: {
  node: Node;
  disabled: boolean;
  onUnload: () => void;
  snapshot: import("@audiorouter/contracts").DiagnosticsSnapshot | null;
}) {
  const path = typeof node.parameters.path === "string" ? node.parameters.path : "";
  const format = node.parameters.format;
  const formatLabel = format === "vst3" ? "VST3" : format === "vst2" ? "VST2" : "Plugin";
  const fingerprint = typeof node.parameters.fingerprint === "string" ? node.parameters.fingerprint : "";
  const fileName = path.replace(/\\/g, "/").split("/").pop() || path;
  const health = snapshot?.nodeTelemetry.find((item) => item.nodeId === node.id)?.plugin ?? null;
  const healthAlert = health?.state === "failed" || health?.state === "quarantined";
  return (
    <div className="node-binding-editor plugin-node-editor" aria-label="VST plugin binding">
      <div>
        <p className="eyebrow">{formatLabel} plugin loaded from disk</p>
        <strong title={path}>{fileName || "No plugin binary bound"}</strong>
      </div>
      <label>
        Binary path
        <input aria-label="Plugin binary path" value={path} readOnly />
      </label>
      <label>
        SHA-256 fingerprint
        <input aria-label="Plugin fingerprint" value={fingerprint} readOnly />
      </label>
      <div
        className={`plugin-worker-health${healthAlert ? " is-alert" : ""}`}
        role={healthAlert ? "alert" : undefined}
        aria-label="Plugin worker health"
      >
        <span className="eyebrow">Isolated worker health</span>
        <strong>{health ? (PLUGIN_HEALTH_LABELS[health.state] ?? health.state) : "Not running"}</strong>
        {health && health.failureCount > 0 && (
          <small>
            {health.failureCount} failure{health.failureCount === 1 ? "" : "s"} recorded
            {health.state === "quarantined" ? "; requires deliberate retry after review" : ""}.
          </small>
        )}
        {!health && <small>Health is only observable while the session is running with this worker bound.</small>}
        {health?.outputMisses !== undefined && (
          <small>
            Audio continuity: {health.outputMisses} missing output blocks (includes startup), {health.inputDrops ?? 0}{" "}
            dropped input blocks. These counts are shared by plugins in the same worker.
          </small>
        )}
      </div>
      <button type="button" className="secondary" onClick={onUnload} disabled={disabled}>
        Unload plugin from draft
      </button>
      <small>
        Execution is bound to this exact binary and fingerprint; a changed file requires rescanning and re-adding it
        from Plugin scan. Unloading removes this node and its connections from the draft only — Plan and Commit changes
        to save the removal. AudioRouter does not open the plugin's own native editor window in this build — adjust its
        parameters with the controls below instead.
      </small>
    </div>
  );
}
