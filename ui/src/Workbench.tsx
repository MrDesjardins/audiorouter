import { ToolGuideLink } from "./ToolGuideLink";
import { ApiPanel } from "./ApiPanel";
import { LogFilesPanel, type VerboseLoggingControl } from "./LogFilesPanel";
import { SessionIdentity } from "./NodeIdentity";
import { endLibraryDrag, startLibraryDrag } from "./libraryDrag";
import { TOOL_HELP, type LibraryEntry } from "./library";
import { TextField } from "./TextField";
import { HelpDisclosure } from "./HelpDisclosure";
import { DuckGlyph } from "./DuckGlyph";
import type { Session } from "@audiorouter/contracts";
import { useLayoutEffect, useRef, useState, type ReactNode } from "react";

const TOOL_ICONS: Record<string, ReactNode> = {
  volume: "◖",
  "bass-treble": "♮",
  dehum: "≁",
  declick: "⌇",
  denoise: "░",
  "speech-denoise": "☊",
  "spectral-gate": "▥",
  "fun-voice-changer": "☺",
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
  | "tools"
  | "properties"
  | "timing"
  | "session"
  | "status"
  | "virtualCables"
  | "uiOptions"
  | "recording"
  | "advanced"
  | "mcp"
  | "api"
  | "diagnostics";

const TAB_HELP: Record<Exclude<WorkbenchTab, "properties">, { title: string; content: ReactNode }> = {
  tools: {
    title: "How to build a route",
    content: (
      <ol>
        <li>Add a source: Input Device, Test Signal, Audio File, or an application.</li>
        <li>Add processing tools such as Gain or Advanced EQ if needed.</li>
        <li>Add an Output Device and choose your speakers or headphones in Properties.</li>
        <li>Drag from the sending node's blue dot to the receiving node's orange dot.</li>
        <li>Press Play to hear it. Use Save at the top when the canvas marks changes that need saving.</li>
      </ol>
    ),
  },
  timing: {
    title: "How to read timing",
    content: (
      <p>
        Play the session to see measured delay along each route. Timing shows where audio waits and how much processing
        adds; it does not prove that an input has signal.
      </p>
    ),
  },
  session: {
    title: "How to manage sessions",
    content: (
      <p>
        Switch between saved setups, create or duplicate a session, or export a stopped copy. Save at the top keeps
        marked edits. Revert edits returns to the saved route.
      </p>
    ),
  },
  status: {
    title: "How to read status",
    content: (
      <p>
        Check the six readiness indicators and the audio devices available on this PC. Choose a node’s device in
        Properties.
      </p>
    ),
  },
  virtualCables: {
    title: "How to use virtual cables",
    content: (
      <p>
        A cable joins two apps: send audio to its render endpoint, then select its matching capture endpoint in the
        receiving app.
      </p>
    ),
  },
  uiOptions: {
    title: "How to personalize the UI",
    content: (
      <p>
        Choose a theme, customize its colors, or exchange a theme file. These preferences stay on this PC and do not
        change your audio.
      </p>
    ),
  },
  recording: {
    title: "How to record audio",
    content: (
      <p>
        Connect a Recorder to the audio you want to save, choose a recording folder and format, then start the session
        and press Record. Recording starts only on your explicit action.
      </p>
    ),
  },
  advanced: {
    title: "How to use advanced controls",
    content: (
      <p>
        Use keyboard graph controls to connect nodes without dragging. Troubleshooting provides manual device binding
        and recovery. Review each action before applying it.
      </p>
    ),
  },
  mcp: {
    title: "How to connect an assistant",
    content: (
      <p>
        Choose the setup for your assistant and grant only the access it needs. Client permissions control reads and
        edits. The activity section shows tool names and outcomes without audio or secret values.
      </p>
    ),
  },
  api: {
    title: "How to use the API",
    content: (
      <p>
        Start the optional HTTP listener, choose who can connect, and use its documentation or request builder. Keep the
        bearer token private; API edits use the same backend as the app.
      </p>
    ),
  },
  diagnostics: {
    title: "How to use logs",
    content: (
      <p>
        Open the logs folder or copy a support bundle when investigating a problem. Verbose logging temporarily records
        more detail. Support bundles are saved locally; they are not uploaded.
      </p>
    ),
  },
};
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
  coworkPluginAvailable?: boolean;
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
  onExportCoworkPlugin,
  clientsPanel,
  statusContent,
  virtualCablesContent,
  uiOptionsContent,
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
  onExportCoworkPlugin?: () => Promise<string>;
  clientsPanel: ReactNode;
  statusContent: ReactNode;
  virtualCablesContent: ReactNode;
  uiOptionsContent: ReactNode;
  timingContent?: ReactNode;
  recordingContent: ReactNode;
  advancedContent: ReactNode;
  pluginsContent?: ReactNode;
  sessionFileContent?: ReactNode;
  apiBuilder?: (baseUrl: string) => ReactNode;
  verboseLogging?: VerboseLoggingControl;
}) {
  const tabsRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const tabs = tabsRef.current;
    const workspace = tabs?.closest<HTMLElement>(".main-content");
    if (!tabs || !workspace) return;
    const measure = () => {
      const height = Math.ceil(tabs.getBoundingClientRect().height);
      if (height > 0) workspace.style.setProperty("--workbench-tabs-height", `${height}px`);
    };
    measure();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    observer?.observe(tabs);
    return () => {
      observer?.disconnect();
      workspace.style.removeProperty("--workbench-tabs-height");
    };
  }, []);
  const [localAssistant, setLocalAssistant] = useState("codex");
  const [mcpClientId, setMcpClientId] = useState("audiorouter-local");
  const [copyMessage, setCopyMessage] = useState("");
  const [coworkBusy, setCoworkBusy] = useState(false);
  const [coworkMessage, setCoworkMessage] = useState("");
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
    ["status", "Status"],
    ["virtualCables", "Virtual Cables"],
    ["uiOptions", "UI Options"],
    ["recording", "Recording"],
    ["advanced", "Advanced"],
    ["mcp", "MCP"],
    ["api", "API"],
    ["diagnostics", "Logs"],
  ];
  return (
    <aside className="right-workbench" aria-label="Tools and settings">
      <div ref={tabsRef} className="workbench-tabs" role="tablist" aria-label="Right sidebar">
        {tabs.map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={tab === id ? "active" : ""}
            onClick={(event) => {
              const workbench = event.currentTarget.closest(".right-workbench");
              if (workbench instanceof HTMLElement) workbench.scrollTop = 0;
              onTab(id);
            }}
          >
            {label}
          </button>
        ))}
      </div>
      {tab !== "properties" && (
        <div className="workbench-help">
          <HelpDisclosure key={tab} title={TAB_HELP[tab].title}>
            {TAB_HELP[tab].content}
          </HelpDisclosure>
        </div>
      )}
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
          <div className="tool-library-entry">
            <button type="button" className="tool-card" onClick={onAddGroup}>
              <span className="tool-card-icon" aria-hidden="true">
                ▢
              </span>
              <span>
                <strong>Group</strong>
                <small>A named visual background; no audio routing</small>
              </span>
            </button>
            <ToolGuideLink kind="group" compact toolName="Group" />
          </div>
          <p className="muted">
            Add and connect tools, press Play to hear the route, then use Save at the top to keep it.
          </p>
          {(["input", "tool", "output"] as const).map((flow) => (
            <section className="tool-group" key={flow}>
              <h3>{flow === "input" ? "Inputs" : flow === "tool" ? "Processing" : "Outputs"}</h3>
              {flow === "input" && (
                <div className="tool-library-entry">
                  <ApplicationSourceAction onClick={onApplicationPicker} disabled={!connected} />
                  <ToolGuideLink kind="applicationCapture" compact toolName="Application Capture" />
                </div>
              )}
              {tools
                .filter((entry) => entry.flow === flow)
                .sort((left, right) => left.label.localeCompare(right.label, undefined, { sensitivity: "base" }))
                .map((entry) => {
                  const help = entry.unavailableReason ?? entry.note ?? TOOL_HELP[entry.id] ?? entry.category;
                  // Click adds the tool; drag places it where it is dropped on the canvas.
                  return (
                    <div className="tool-library-entry" key={entry.id}>
                      <button
                        className="tool-card"
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
                      <ToolGuideLink
                        kind={
                          entry.kind ??
                          entry.virtualKind ??
                          (entry.id === "endpoint-loopback" ? "endpointLoopback" : entry.id)
                        }
                        compact
                        toolName={entry.label}
                      />
                    </div>
                  );
                })}
              {flow === "tool" && pluginsContent}
            </section>
          ))}
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
      {tab === "status" && (
        <section className="workbench-page" role="tabpanel">
          <h2>Status</h2>
          {statusContent}
        </section>
      )}
      {tab === "virtualCables" && (
        <section className="workbench-page" role="tabpanel">
          <h2>Virtual Cables</h2>
          {virtualCablesContent}
        </section>
      )}
      {tab === "uiOptions" && (
        <section className="workbench-page" role="tabpanel">
          <h2>UI Options</h2>
          {uiOptionsContent}
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
            <p className="eyebrow">Model Context Protocol</p>
            <h2>MCP connections</h2>
            <p className="muted">
              AudioRouter speaks standard MCP. Choose a local app below, or connect ChatGPT through its optional OpenAI
              tunnel.
            </p>
          </header>

          <section className="mcp-chatgpt-card" aria-labelledby="mcp-chatgpt-title">
            <div className="mcp-card-heading">
              <div>
                <h3 id="mcp-chatgpt-title">ChatGPT · OpenAI tunnel</h3>
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
              ChatGPT uses the separate <code>audiorouter-chatgpt</code> client with read-only access. To let it create
              or edit sessions, grant Editor under Client permissions. Audio stays on this PC; only MCP requests and
              results cross the encrypted tunnel.
            </p>
            {mcpTunnelStatus && !mcpTunnelStatus.runtimeAvailable && (
              <p className="mcp-tunnel-error" role="status">
                The OpenAI tunnel runtime is missing from this AudioRouter installation. Update or reinstall
                AudioRouter, then try again.
              </p>
            )}
            {!mcpTunnelStatus && (
              <p className="mcp-tunnel-message" role="status" aria-live="polite">
                Checking this PC for the optional OpenAI tunnel…
              </p>
            )}
            {mcpTunnelStatus?.runtimeAvailable &&
              (!mcpTunnelStatus.configured || mcpTunnelStatus.status === "needsSetup" || showTunnelSetup) && (
                <div className="mcp-tunnel-setup">
                  <p className="muted">
                    <strong>1.</strong> Create or choose an OpenAI tunnel. <strong>2.</strong> Create a restricted
                    runtime key with <strong>Tunnels Read + Use</strong>. <strong>3.</strong> Enter both below. The key
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
                      tunnelBusy ||
                      !(tunnelIdInput.trim() || mcpTunnelStatus?.tunnelId?.trim()) ||
                      !runtimeKeyInput ||
                      !mcpTunnelStatus?.runtimeAvailable
                    }
                    onClick={() => {
                      const tunnelId = tunnelIdInput.trim() || mcpTunnelStatus?.tunnelId?.trim() || "";
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
            {mcpTunnelStatus?.configured &&
              mcpTunnelStatus.runtimeAvailable &&
              mcpTunnelStatus.status !== "needsSetup" &&
              !showTunnelSetup && (
                <div className="mcp-tunnel-actions">
                  <div className="mcp-tunnel-primary-action">
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
                              setTunnelMessage(
                                error instanceof Error ? error.message : "Could not connect the tunnel.",
                              ),
                            )
                            .finally(() => setTunnelBusy(false));
                        }}
                      >
                        {tunnelBusy
                          ? "Connecting…"
                          : mcpTunnelStatus.status === "actionNeeded"
                            ? "Retry connection"
                            : "Connect"}
                      </button>
                    )}
                  </div>
                  <div className="mcp-tunnel-handoff-slot">
                    {mcpTunnelStatus.status === "connected" && (
                      <>
                        <button
                          type="button"
                          className="mcp-chatgpt-open"
                          onClick={() => openMcpSetupPage("chatgptConnectors")}
                        >
                          Open ChatGPT connector settings
                        </button>
                        <p className="muted mcp-tunnel-chat-guidance">
                          Next: add this tunnel as a connector, then enable AudioRouter from the add-tools menu in a
                          conversation.
                        </p>
                      </>
                    )}
                    {mcpTunnelStatus.status !== "connected" && (
                      <p className="muted mcp-tunnel-chat-guidance">
                        {mcpTunnelStatus.status === "actionNeeded"
                          ? "Retry the tunnel connection. ChatGPT setup appears here after it connects."
                          : mcpTunnelStatus.status === "connecting"
                            ? "When the tunnel connects, add it in ChatGPT and enable it from the conversation's add-tools menu."
                            : "After connecting, add this tunnel in ChatGPT, then enable it from the conversation's add-tools menu."}
                      </p>
                    )}
                  </div>
                  <div className="mcp-tunnel-management-actions">
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
                            setTunnelMessage(
                              "Saved details removed from this PC. The OpenAI tunnel and key still exist.",
                            );
                          })
                          .catch(() => setTunnelMessage("Could not remove the saved tunnel details."))
                          .finally(() => setTunnelBusy(false));
                      }}
                    >
                      Forget saved details
                    </button>
                  </div>
                </div>
              )}
            <p className="mcp-tunnel-message" role="status" aria-live="polite">
              {tunnelMessage || " "}
            </p>
          </section>

          <section className="mcp-remote-info" aria-labelledby="mcp-cowork-title">
            <div className="mcp-card-heading">
              <h3 id="mcp-cowork-title">Cowork</h3>
              <span className="mcp-status">Local plugin</span>
            </div>
            <p className="muted">
              For local Cowork on this PC, export the plugin, then upload it in Cowork → Customize → Plugins. Keep
              AudioRouter running. Export enables read-only access; choose Editor in Client permissions to allow edits.
              Cloud Cowork tasks need a remote connection, which is not available yet.
            </p>
            <div className="mcp-cowork-action">
              {mcpSetupInfo?.cliAvailable && mcpSetupInfo.coworkPluginAvailable && onExportCoworkPlugin ? (
                <button
                  type="button"
                  disabled={!connected || coworkBusy}
                  onClick={() => {
                    setCoworkBusy(true);
                    setCoworkMessage("Checking the local MCP connection…");
                    void onExportCoworkPlugin()
                      .then((path) =>
                        setCoworkMessage(`Saved to ${path}. Upload this ZIP in Cowork → Customize → Plugins.`),
                      )
                      .catch((error: unknown) =>
                        setCoworkMessage(error instanceof Error ? error.message : String(error)),
                      )
                      .finally(() => setCoworkBusy(false));
                  }}
                >
                  Export Cowork plugin
                </button>
              ) : (
                <p className="muted">
                  Open the current AudioRouter desktop app with its installed MCP command to export.
                </p>
              )}
            </div>
            <p className="mcp-cowork-message" role="status" aria-live="polite">
              {coworkMessage || "The plugin is for this PC and does not include audio or credentials."}
            </p>
          </section>

          <details className="mcp-secondary-section local-assistant-setup">
            <summary>Connect a local assistant</summary>
            <p>
              For Codex, Claude Code, or another assistant running on this PC. Your assistant launches AudioRouter’s
              local MCP command; keep AudioRouter open while using it.
            </p>
            <ol className="setup-instructions">
              <li>
                <h3>Choose your assistant</h3>
                <label>
                  Assistant
                  <select
                    aria-label="Local assistant"
                    value={localAssistant}
                    onChange={(event) => setLocalAssistant(event.target.value)}
                  >
                    <option value="codex">Codex</option>
                    <option value="claude">Claude Code</option>
                    <option value="other">Another MCP app</option>
                  </select>
                </label>
              </li>
              <li>
                <h3>Name this connection and grant access</h3>
                <p>This name identifies permissions in AudioRouter. It is not your assistant account.</p>
                <label>
                  Connection name
                  <input
                    aria-label="Assistant client ID"
                    value={mcpClientId}
                    maxLength={128}
                    onChange={(event) => setMcpClientId(event.target.value.trim())}
                  />
                </label>
                <button
                  type="button"
                  className="secondary"
                  onClick={(event) => {
                    const panel = event.currentTarget
                      .closest(".workbench-page")
                      ?.querySelector<HTMLDetailsElement>("[data-client-permissions]");
                    if (panel) {
                      panel.open = true;
                      panel.scrollIntoView({ block: "start", behavior: "smooth" });
                      panel.querySelector("summary")?.focus();
                    }
                  }}
                >
                  Set client permissions
                </button>
                <p className="muted">
                  Use this exact connection name below. Start with observer access; allow edits only when you need them.
                </p>
              </li>
              <li>
                <h3>
                  {localAssistant === "codex"
                    ? "Add this to your Codex configuration"
                    : localAssistant === "claude"
                      ? "Run this in PowerShell"
                      : "Add a command connection in your MCP app"}
                </h3>
                {mcpSetupInfo?.cliAvailable ? (
                  <>
                    {localAssistant === "codex" ? (
                      <>
                        <p>Paste this block into your Codex config.toml, then reload its MCP connections.</p>
                        <pre className="mcp-command">{codexConfig}</pre>
                        <button
                          type="button"
                          className="secondary"
                          disabled={!mcpClientId}
                          onClick={() => copy(codexConfig)}
                        >
                          Copy Codex config
                        </button>
                      </>
                    ) : localAssistant === "claude" ? (
                      <>
                        <p>Run this once, then restart Claude Code or reconnect its MCP server.</p>
                        <pre className="mcp-command">{claudeCommand}</pre>
                        <button
                          type="button"
                          className="secondary"
                          disabled={!mcpClientId}
                          onClick={() => copy(claudeCommand)}
                        >
                          Copy Claude command
                        </button>
                      </>
                    ) : (
                      <>
                        <p>
                          Choose the standard command / stdio transport. Enter the executable and arguments separately.
                        </p>
                        <label>
                          Executable
                          <input readOnly value={cli} />
                        </label>
                        <label>
                          Arguments
                          <textarea
                            readOnly
                            value={JSON.stringify([
                              "mcp",
                              "serve",
                              "--client-id",
                              mcpClientId,
                              "--database",
                              db,
                              "--pipe",
                              pipe,
                            ])}
                          />
                        </label>
                        <p className="muted">
                          The app starts this command for you. Do not paste it into a web address field.
                        </p>
                      </>
                    )}
                  </>
                ) : (
                  <p className="mcp-tunnel-message" role="status">
                    {mcpSetupInfo
                      ? "The local MCP command is missing. Repair or update your AudioRouter installation before continuing."
                      : "Checking the installed local MCP command…"}
                  </p>
                )}
              </li>
              <li>
                <h3>Confirm the connection</h3>
                <p>
                  Ask your assistant to list AudioRouter sessions. Check Recent MCP activity below for the request and
                  its outcome. If access is denied, match the connection name in Client permissions.
                </p>
              </li>
            </ol>
            <p className="mcp-copy-message" role="status">
              {copyMessage || "Copy a setup above when you are ready."}
            </p>
          </details>

          <details className="mcp-secondary-section" data-client-permissions>
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
