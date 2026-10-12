import { ApiPanel } from "./ApiPanel";
import { LogFilesPanel, type VerboseLoggingControl } from "./LogFilesPanel";
import { SessionIdentity } from "./NodeIdentity";
import { endLibraryDrag, startLibraryDrag } from "./libraryDrag";
import { TOOL_HELP, type LibraryEntry } from "./library";
import { TextField } from "./TextField";
import { DuckGlyph } from "./DuckGlyph";
import type { Session } from "@audiorouter/contracts";
import { useState, type ReactNode } from "react";

const TOOL_ICONS: Record<string, ReactNode> = {
  volume: "◖",
  "bass-treble": "♮",
  dehum: "≁",
  declick: "⌇",
  denoise: "░",
  "speech-denoise": "☊",
  "spectral-gate": "▥",
  "fir-filter": "⧉",
  "input-switch": "⇄",
  duck: <DuckGlyph className="tool-card-glyph" />,
  "time-shift": "↺",
  "physical-input": "◉",
  "test-signal": "∿",
  "audio-file": "♫",
  "endpoint-loopback": "↶",
  "virtual-render-source": "⊞",
  "physical-output": "◎",
  "virtual-capture-sink": "⊟",
  gain: "◢",
  mixer: "⋈",
  recorder: "●",
  mute: "⊘",
  meter: "▥",
  "parametric-eq": "⌁",
  compressor: "⤓",
  gate: "⊐",
  limiter: "⊤",
  delay: "◷",
  "graphic-eq": "▤",
  pitch: "↟",
  "network-send": "⇡",
  "network-receive": "⇣",
};

function ApplicationSourceAction({ onClick, disabled }: { onClick: () => void; disabled: boolean }) {
  return (
    <button type="button" className="secondary application-source-action" onClick={onClick} disabled={disabled}>
      <svg
        className="application-source-action-icon"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        focusable="false"
      >
        <rect x="3" y="4" width="14" height="15" rx="2" />
        <path d="M3 8h14M7 6h.01M10 6h.01M19 10v4m-2-2h4" />
      </svg>
      <span>
        <strong>Choose an application source</strong>
        <small>Capture audio from a running app into this route</small>
      </span>
    </button>
  );
}

export type WorkbenchTab =
  "tools" | "properties" | "timing" | "session" | "setup" | "recording" | "advanced" | "mcp" | "api" | "diagnostics";
export type McpActivity = {
  timeUnixMs: number;
  clientId: string;
  tool: string;
  argumentFields: string[];
  outcome: string;
  errorKind?: string | null;
};
export type McpSetupInfo = {
  cliPath: string;
  cliAvailable: boolean;
  databasePath: string;
  pipeName: string;
  transport: string;
};
export type McpTunnelStatus = {
  status: "notConfigured" | "needsSetup" | "disconnected" | "connecting" | "connected" | "actionNeeded";
  configured: boolean;
  tunnelId: string | null;
  runtimeAvailable: boolean;
};

export function Workbench({
  onAddGroup,
  tab,
  onTab,
  tools,
  connected,
  onAdd,
  onAddCable,
  onApplicationPicker,
  librarySearch,
  onLibrarySearch,
  onNewSession,
  onDuplicate,
  onDelete,
  onUndo,
  onRedo,
  onDiscard,
  sessions,
  selectedSessionId,
  onSelectSession,
  sessionName,
  onNameChange,
  diagnostics,
  backendActivity,
  mcpActivity,
  mcpSetupInfo,
  mcpTunnelStatus,
  onOpenMcpSetupPage,
  onMcpTunnelConfigure,
  onMcpTunnelConnect,
  onMcpTunnelDisconnect,
  onMcpTunnelForget,
  clientsPanel,
  setupContent,
  timingContent,
  recordingContent,
  advancedContent,
  pluginsContent,
  sessionFileContent,
  apiBuilder,
  verboseLogging,
}: {
  onAddGroup?: () => void;
  tab: WorkbenchTab;
  onTab: (tab: WorkbenchTab) => void;
  tools: LibraryEntry[];
  connected: boolean;
  onAdd: (kind: NonNullable<LibraryEntry["kind"]>) => void;
  /** Adds a cable source or destination on its default cable. */
  onAddCable?: (direction: "renderSource" | "captureSink") => void;
  onNewSession: () => void;
  onDuplicate: () => void;
  onApplicationPicker: () => void;
  librarySearch: string;
  onLibrarySearch: (value: string) => void;
  onDelete: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onDiscard: () => void;
  onPlan?: () => void;
  onCommit?: () => void;
  canCommit?: boolean;
  pendingPlan?: boolean;
  actionMessage?: string | null;
  onReplaceInputConnection?: () => void;
  sessions: Session[];
  selectedSessionId: string;
  onSelectSession: (id: string) => void;
  sessionName: string;
  onNameChange: (name: string) => void;
  revision?: number;
  warnings?: string[];
  acknowledgedWarnings?: string[];
  onAcknowledgeWarning?: (warning: string, checked: boolean) => void;
  diagnostics: string[];
  backendActivity: Record<string, unknown>[];
  mcpActivity: McpActivity[];
  mcpSetupInfo: McpSetupInfo | null;
  mcpTunnelStatus: McpTunnelStatus | null;
  onOpenMcpSetupPage: (page: "tunnels" | "apiKeys" | "chatgptConnectors") => Promise<void>;
  onMcpTunnelConfigure: (tunnelId: string, runtimeKey: string) => Promise<void>;
  onMcpTunnelConnect: () => Promise<void>;
  onMcpTunnelDisconnect: () => Promise<void>;
  onMcpTunnelForget: () => Promise<void>;
  clientsPanel: ReactNode;
  setupContent: ReactNode;
  timingContent?: ReactNode;
  recordingContent: ReactNode;
  advancedContent: ReactNode;
  pluginsContent?: ReactNode;
  sessionFileContent?: ReactNode;
  apiBuilder?: (baseUrl: string) => ReactNode;
  verboseLogging?: VerboseLoggingControl;
}) {
  const [mcpClientId, setMcpClientId] = useState("audiorouter-local");
  const [copyMessage, setCopyMessage] = useState("");
  const [tunnelIdInput, setTunnelIdInput] = useState("");
  const [runtimeKeyInput, setRuntimeKeyInput] = useState("");
  const openMcpSetupPage = (page: "tunnels" | "apiKeys" | "chatgptConnectors") => {
    setTunnelMessage("");
    void onOpenMcpSetupPage(page)
      .then(() => setTunnelMessage("Opened the setup page in your browser."))
      .catch((error: unknown) =>
        setTunnelMessage(error instanceof Error ? error.message : "Could not open the setup page."),
      );
  };
  const [tunnelBusy, setTunnelBusy] = useState(false);
  const [tunnelMessage, setTunnelMessage] = useState("");
  const [showTunnelSetup, setShowTunnelSetup] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const cli = mcpSetupInfo?.cliPath ?? "PATH_TO_AUDIOROUTER_CLI";
  const db = mcpSetupInfo?.databasePath ?? "PATH_TO_AUDIOROUTER_DATABASE";
  const pipe = mcpSetupInfo?.pipeName ?? "\\\\.\\pipe\\audiorouter-control";
  const quoted = (value: string) => JSON.stringify(value);
  const psQuoted = (value: string) => `'${value.replaceAll("'", "''")}'`;
  const codexConfig = `[mcp_servers.audiorouter]\ncommand = ${quoted(cli)}\nargs = ["mcp", "serve", "--client-id", ${quoted(mcpClientId)}, "--database", ${quoted(db)}, "--pipe", ${quoted(pipe)}]`;
  const claudeCommand = `claude mcp add --scope user audiorouter -- ${psQuoted(cli)} mcp serve --client-id ${psQuoted(mcpClientId)} --database ${psQuoted(db)} --pipe ${psQuoted(pipe)}`;
  const copy = (value: string) => {
    if (!navigator.clipboard) {
      setCopyMessage("Clipboard unavailable. Select the text and copy it.");
      return;
    }
    void navigator.clipboard
      .writeText(value)
      .then(() => setCopyMessage("Copied to clipboard."))
      .catch(() => setCopyMessage("Clipboard unavailable. Select the text and copy it."));
  };
  const tabs: [WorkbenchTab, string][] = [
    ["tools", "Tools"],
    ["properties", "Properties"],
    ["timing", "Timing"],
    ["session", "Session"],
    ["setup", "Setup"],
    ["recording", "Recording"],
    ["advanced", "Advanced"],
    ["mcp", "MCP"],
    ["api", "API"],
    ["diagnostics", "Logs"],
  ];
  return (
    <aside className="right-workbench" aria-label="Tools and settings">
      <div className="workbench-tabs" role="tablist" aria-label="Right sidebar">
        {tabs.map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={tab === id ? "active" : ""}
            onClick={() => onTab(id)}
          >
            {label}
          </button>
        ))}
      </div>
      {tab === "tools" && (
        <div className="workbench-tool-search">
          <label>
            Find a tool
            <input
              type="search"
              aria-label="Find a tool"
              value={librarySearch}
              onChange={(event) => onLibrarySearch(event.target.value)}
              placeholder="Input, gain, recorder…"
            />
          </label>
        </div>
      )}
      {tab === "tools" && (
        <section className="workbench-page" role="tabpanel" aria-label="Tools">
          <p className="eyebrow">Build a route</p>
          <h2>Add tools</h2>
          <button type="button" className="tool-card" onClick={onAddGroup}>
            <span className="tool-card-icon" aria-hidden="true">
              ▢
            </span>
            <span>
              <strong>Group</strong>
              <small>A named visual background; no audio routing</small>
            </span>
          </button>
          <p className="muted">
            Add and connect tools, press Play to hear the route, then use Save at the top to keep it.
          </p>
          {(["input", "tool", "output"] as const).map((flow) => (
            <section className="tool-group" key={flow}>
              <h3>{flow === "input" ? "Inputs" : flow === "tool" ? "Processing" : "Outputs"}</h3>
              {flow === "input" && <ApplicationSourceAction onClick={onApplicationPicker} disabled={!connected} />}
              {tools
                .filter((entry) => entry.flow === flow)
                .sort((left, right) => left.label.localeCompare(right.label, undefined, { sensitivity: "base" }))
                .map((entry) => {
                  const help = entry.unavailableReason ?? entry.note ?? TOOL_HELP[entry.id] ?? entry.category;
                  // Click adds the tool; drag places it where it is dropped on the canvas.
                  return (
                    <button
                      className="tool-card"
                      key={entry.id}
                      type="button"
                      draggable={connected && Boolean(entry.kind)}
                      onDragStart={(event) => {
                        if (entry.kind) startLibraryDrag(event, entry.kind, entry.label);
                      }}
                      onDragEnd={endLibraryDrag}
                      onClick={() => {
                        if (entry.kind) onAdd(entry.kind);
                        else if (entry.virtualKind)
                          onAddCable?.(entry.virtualKind === "virtualRenderSource" ? "renderSource" : "captureSink");
                      }}
                      disabled={
                        !connected ||
                        (!entry.kind && !(entry.virtualKind && onAddCable && entry.unavailableReason === undefined))
                      }
                      title={help}
                      aria-description={help}
                    >
                      <span className="tool-card-icon" aria-hidden="true">
                        {TOOL_ICONS[entry.id]}
                      </span>
                      <span>
                        <strong>{entry.label}</strong>
                        <small>{help}</small>
                      </span>
                    </button>
                  );
                })}
              {flow === "tool" && pluginsContent}
            </section>
          ))}
          <details className="tools-howto">
            <summary>How to build a route</summary>
            <ol>
              <li>Add a source: Input Device (microphone), Test Signal, Audio File, or an application.</li>
              <li>Add processing tools such as Gain or Advanced EQ if needed.</li>
              <li>Add an Output Device and choose your speakers or headphones in its Properties.</li>
              <li>Connect the nodes: drag from a node's right edge to the next node's left edge.</li>
              <li>Press Play to hear it, and Save at the top to keep it.</li>
            </ol>
          </details>
        </section>
      )}
      {tab === "api" && <ApiPanel builder={apiBuilder} />}
      {tab === "timing" && (
        <section className="workbench-page" role="tabpanel" aria-label="Timing">
          <p className="eyebrow">Where the sound spends time</p>
          <h2>Signal timing</h2>
          {timingContent}
        </section>
      )}
      {tab === "properties" && (
        <section className="workbench-page" role="tabpanel">
          <h2>Node properties</h2>
          <p className="muted">Select a node on the canvas to edit it here.</p>
        </section>
      )}
      {tab === "setup" && (
        <section className="workbench-page" role="tabpanel">
          <p className="eyebrow">For the whole app</p>
          <h2>Set up this PC</h2>
          <p className="muted">
            These settings apply to AudioRouter on this computer, not to one session. Build routes in Tools and choose
            each node's device in its Properties.
          </p>
          {setupContent}
        </section>
      )}
      {tab === "recording" && (
        <section className="workbench-page" role="tabpanel">
          <h2>Recording</h2>
          <p className="muted">
            Recording starts only after you explicitly arm and start a recorder. Confirm the destination and format
            before recording.
          </p>
          {recordingContent}
        </section>
      )}
      {tab === "advanced" && (
        <section className="workbench-page" role="tabpanel">
          <h2>Advanced controls</h2>
          <p className="muted">
            These controls affect plugins, startup, connected assistant clients, and recovery. Review each action before
            applying it.
          </p>
          {advancedContent}
        </section>
      )}
      {tab === "session" && (
        <section className="workbench-page session-page" role="tabpanel">
          <h2>Sessions</h2>
          <p className="muted">
            Switch between saved audio setups, or create a copy to try another arrangement. Use Save at the top when you
            want to keep your edits.
          </p>
          <label>
            Current session
            <select
              aria-label="Choose session"
              value={selectedSessionId}
              onChange={(event) => {
                setRenaming(false);
                onSelectSession(event.target.value);
              }}
            >
              {sessions.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </label>
          {renaming ? (
            <label>
              New session name
              <TextField
                aria-label="Session name"
                value={sessionName}
                maxLength={120}
                disabled={!connected}
                autoFocus
                onValue={onNameChange}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === "Escape") setRenaming(false);
                }}
              />
            </label>
          ) : (
            <button type="button" className="secondary" onClick={() => setRenaming(true)} disabled={!connected}>
              Rename
            </button>
          )}
          <div className="session-action-grid">
            <button type="button" onClick={onNewSession} disabled={!connected}>
              New
            </button>
            <button type="button" className="secondary" onClick={onDuplicate} disabled={!connected}>
              Duplicate
            </button>
            <button type="button" className="secondary" onClick={onUndo} disabled={!connected}>
              Undo
            </button>
            <button type="button" className="secondary" onClick={onRedo} disabled={!connected}>
              Redo
            </button>
            <button type="button" className="secondary" onClick={onDiscard} disabled={!connected}>
              Revert edits
            </button>
            <button type="button" className="danger" onClick={onDelete} disabled={!connected}>
              Delete session
            </button>
          </div>
          {sessionFileContent}
          <SessionIdentity sessionId={selectedSessionId} />
        </section>
      )}
      {tab === "mcp" && (
        <section className="workbench-page mcp-page" role="tabpanel">
          <header className="mcp-heading">
            <p className="eyebrow">MCP connection</p>
            <h2>Connect ChatGPT</h2>
            <p className="muted">Use AudioRouter tools from the ChatGPT Windows app.</p>
          </header>

          <section className="mcp-chatgpt-card" aria-labelledby="mcp-chatgpt-title">
            <div className="mcp-card-heading">
              <div>
                <h3 id="mcp-chatgpt-title">ChatGPT</h3>
                <p className="muted mcp-tunnel-description">
                  {mcpTunnelStatus?.status === "connected"
                    ? "The AudioRouter tunnel is ready. Finish adding it in ChatGPT connector settings."
                    : mcpTunnelStatus?.status === "connecting"
                      ? "Starting the secure connection…"
                      : mcpTunnelStatus?.status === "actionNeeded"
                        ? "The tunnel stopped. Check the tunnel ID, key access, and internet connection."
                        : mcpTunnelStatus?.status === "needsSetup"
                          ? "Your tunnel ID is saved. Add its restricted runtime key to connect."
                          : mcpTunnelStatus?.status === "disconnected"
                            ? "Your OpenAI tunnel is set up and ready to connect."
                            : mcpTunnelStatus?.status === "notConfigured"
                              ? "Connect an existing OpenAI tunnel to AudioRouter."
                              : "Checking the local tunnel…"}
                </p>
              </div>
              <span className={`mcp-status mcp-status-${mcpTunnelStatus?.status ?? "checking"}`}>
                {mcpTunnelStatus?.status === "connected"
                  ? "Tunnel ready"
                  : mcpTunnelStatus?.status === "connecting"
                    ? "Connecting"
                    : mcpTunnelStatus?.status === "actionNeeded"
                      ? "Action needed"
                      : mcpTunnelStatus?.status === "needsSetup"
                        ? "Needs setup"
                        : mcpTunnelStatus?.status === "disconnected"
                          ? "Disconnected"
                          : mcpTunnelStatus?.status === "notConfigured"
                            ? "Not connected"
                            : "Checking"}
              </span>
            </div>
            <p>
              The separate <code>audiorouter-chatgpt</code> client starts with Observer (read-only) access. Observer can
              inspect AudioRouter but cannot create or change sessions. To let ChatGPT create a session, expand Client
              permissions below and change this client to Editor; that grant stays in effect until you change or revoke
              it. Audio stays on this PC; only MCP requests and results cross the encrypted tunnel.
            </p>
            {!mcpTunnelStatus?.runtimeAvailable && (
              <p className="mcp-tunnel-error" role="status">
                The OpenAI tunnel runtime is missing from this AudioRouter installation. Update or reinstall
                AudioRouter, then try again.
              </p>
            )}
            {(!mcpTunnelStatus?.configured || mcpTunnelStatus.status === "needsSetup" || showTunnelSetup) && (
              <div className="mcp-tunnel-setup">
                <p className="muted">
                  Use an existing tunnel and a restricted runtime key with <strong>Tunnels Read + Use</strong>. The key
                  is encrypted for your Windows account and is never saved in ChatGPT.
                </p>
                <button type="button" className="mcp-chatgpt-open" onClick={() => openMcpSetupPage("tunnels")}>
                  Open OpenAI tunnel settings
                </button>
                <button type="button" className="mcp-chatgpt-open" onClick={() => openMcpSetupPage("apiKeys")}>
                  Create restricted runtime key
                </button>
                <label>
                  Tunnel ID
                  <input
                    aria-label="OpenAI tunnel ID"
                    autoComplete="off"
                    value={tunnelIdInput || mcpTunnelStatus?.tunnelId || ""}
                    onChange={(event) => setTunnelIdInput(event.target.value.trim())}
                    placeholder="tunnel_…"
                    disabled={tunnelBusy}
                  />
                </label>
                <label>
                  Restricted runtime key
                  <input
                    aria-label="OpenAI restricted runtime key"
                    type="password"
                    autoComplete="new-password"
                    value={runtimeKeyInput}
                    onChange={(event) => setRuntimeKeyInput(event.target.value)}
                    placeholder="Paste the key once"
                    disabled={tunnelBusy}
                  />
                </label>
                <button
                  type="button"
                  disabled={
                    tunnelBusy || !tunnelIdInput.trim() || !runtimeKeyInput || !mcpTunnelStatus?.runtimeAvailable
                  }
                  onClick={() => {
                    const tunnelId = tunnelIdInput.trim();
                    const runtimeKey = runtimeKeyInput;
                    setRuntimeKeyInput("");
                    setTunnelMessage("");
                    setTunnelBusy(true);
                    void onMcpTunnelConfigure(tunnelId, runtimeKey)
                      .then(() => {
                        setTunnelIdInput("");
                        setShowTunnelSetup(false);
                        return onMcpTunnelConnect();
                      })
                      .then(() => setTunnelMessage("Secure tunnel starting."))
                      .catch((error: unknown) =>
                        setTunnelMessage(error instanceof Error ? error.message : "Could not connect the tunnel."),
                      )
                      .finally(() => setTunnelBusy(false));
                  }}
                >
                  {tunnelBusy ? "Connecting…" : "Save securely and connect"}
                </button>
              </div>
            )}
            {mcpTunnelStatus?.configured && mcpTunnelStatus.status !== "needsSetup" && !showTunnelSetup && (
              <div className="mcp-tunnel-actions">
                {mcpTunnelStatus.status === "connected" || mcpTunnelStatus.status === "connecting" ? (
                  <button
                    type="button"
                    className="secondary"
                    disabled={tunnelBusy}
                    onClick={() => {
                      setTunnelBusy(true);
                      void onMcpTunnelDisconnect()
                        .then(() => setTunnelMessage("The local tunnel has stopped."))
                        .catch(() => setTunnelMessage("Could not stop the local tunnel."))
                        .finally(() => setTunnelBusy(false));
                    }}
                  >
                    Disconnect
                  </button>
                ) : (
                  <button
                    type="button"
                    disabled={tunnelBusy || !mcpTunnelStatus.runtimeAvailable}
                    onClick={() => {
                      setTunnelBusy(true);
                      setTunnelMessage("");
                      void onMcpTunnelConnect()
                        .then(() => setTunnelMessage("Secure tunnel starting."))
                        .catch((error: unknown) =>
                          setTunnelMessage(error instanceof Error ? error.message : "Could not connect the tunnel."),
                        )
                        .finally(() => setTunnelBusy(false));
                    }}
                  >
                    {tunnelBusy ? "Connecting…" : "Connect"}
                  </button>
                )}
                <button
                  type="button"
                  className="mcp-chatgpt-open"
                  onClick={() => openMcpSetupPage("chatgptConnectors")}
                >
                  Open ChatGPT connector settings
                </button>
                <p className="muted mcp-tunnel-chat-guidance">
                  After adding the connector, start a ChatGPT conversation and choose AudioRouter from the add-tools
                  menu. The connector must be enabled in that conversation before ChatGPT can call it.
                </p>
                <button
                  type="button"
                  className="secondary"
                  disabled={tunnelBusy}
                  onClick={() => setShowTunnelSetup(true)}
                >
                  Update tunnel details
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={tunnelBusy}
                  onClick={() => {
                    setTunnelBusy(true);
                    void onMcpTunnelForget()
                      .then(() => {
                        setShowTunnelSetup(false);
                        setTunnelIdInput("");
                        setTunnelMessage("Saved details removed from this PC. The OpenAI tunnel and key still exist.");
                      })
                      .catch(() => setTunnelMessage("Could not remove the saved tunnel details."))
                      .finally(() => setTunnelBusy(false));
                  }}
                >
                  Forget saved details
                </button>
              </div>
            )}
            <p className="mcp-tunnel-message" role="status" aria-live="polite">
              {tunnelMessage || " "}
            </p>
          </section>

          <details className="mcp-secondary-section">
            <summary>Other MCP apps (Codex and Claude Code)</summary>
            <p className="muted">
              This client ID is AudioRouter's local authorization name. It does not identify an assistant app.
            </p>
            <label>
              Client ID
              <input
                aria-label="Assistant client ID"
                value={mcpClientId}
                maxLength={128}
                onChange={(event) => setMcpClientId(event.target.value.trim())}
              />
            </label>
            {mcpSetupInfo && (
              <details>
                <summary>Connection paths</summary>
                <p>
                  <code>{mcpSetupInfo.cliPath}</code>
                </p>
                <p>
                  <code>{mcpSetupInfo.databasePath}</code>
                </p>
                <p>
                  <code>{mcpSetupInfo.pipeName}</code>
                </p>
                {!mcpSetupInfo.cliAvailable && <p role="status">CLI executable is missing from this installation.</p>}
              </details>
            )}
            <details>
              <summary>Codex config.toml</summary>
              <pre className="mcp-command">{codexConfig}</pre>
              <button className="secondary" onClick={() => copy(codexConfig)}>
                Copy Codex config
              </button>
            </details>
            <details>
              <summary>Claude Code in PowerShell</summary>
              <pre className="mcp-command">{claudeCommand}</pre>
              <button className="secondary" onClick={() => copy(claudeCommand)}>
                Copy Claude command
              </button>
            </details>
            {copyMessage && <p role="status">{copyMessage}</p>}
          </details>

          <details className="mcp-secondary-section">
            <summary>Client permissions</summary>
            <p className="muted">
              Start with observer access. Grant write access only for actions you want assistants to perform.
            </p>
            {clientsPanel}
          </details>

          <details className="mcp-secondary-section">
            <summary>Recent MCP activity</summary>
            <p className="muted">Tool names and outcomes only. Audio and argument values are excluded.</p>
            {mcpActivity.length === 0 ? (
              <p className="muted mcp-empty">No MCP calls yet.</p>
            ) : (
              <ol className="diagnostic-list" aria-label="MCP tool activity">
                {mcpActivity.map((entry, index) => (
                  <li key={`${entry.timeUnixMs}-${index}`}>
                    <strong>
                      {new Date(entry.timeUnixMs).toLocaleTimeString()} · {entry.tool}
                    </strong>
                    <small>
                      {" "}
                      {entry.outcome}
                      {entry.errorKind ? ` (${entry.errorKind})` : ""} · client {entry.clientId} · fields:{" "}
                      {entry.argumentFields.join(", ") || "none"}
                    </small>
                  </li>
                ))}
              </ol>
            )}
          </details>
        </section>
      )}
      {tab === "diagnostics" && (
        <section className="workbench-page" role="tabpanel">
          <h2>Client diagnostics</h2>
          <LogFilesPanel verbose={verboseLogging} clientDiagnostics={diagnostics} />
          <p className="muted">
            Recent UI error categories and graph checkpoints. Up to 80 short entries are saved in this local app profile
            so they survive a reload. Raw error text, file paths, and audio are excluded.
          </p>
          <ol className="diagnostic-list">
            {diagnostics.map((row, index) => (
              <li key={`${index}-${row}`}>{row}</li>
            ))}
          </ol>
          <h3>Backend RPC</h3>
          {backendActivity.length === 0 ? (
            <p className="muted">
              No backend diagnostics have been recorded yet. Log files are stored under LocalAppData/AudioRouter/logs.
            </p>
          ) : (
            <ol className="diagnostic-list" aria-label="Backend RPC activity">
              {backendActivity.map((entry, index) => (
                <li key={`${String(entry.timeUnixMs)}-${index}`}>
                  <strong>{String(entry.method ?? "RPC")}</strong> · {String(entry.outcome ?? "unknown")}
                  {entry.errorCode == null ? "" : ` · code ${String(entry.errorCode)}`}
                  {entry.errorKind == null ? "" : ` · ${String(entry.errorKind)}`}
                  <small> {JSON.stringify(entry.summary ?? entry.detail ?? {})}</small>
                </li>
              ))}
            </ol>
          )}
        </section>
      )}
    </aside>
  );
}
