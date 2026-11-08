import { ToolShelf } from "./ToolShelf";
import type { VerboseLoggingControl } from "./LogFilesPanel";
import { SessionIdentity } from "./NodeIdentity";
import type { LibraryEntry } from "./library";
import { TextField } from "./TextField";
import { HelpDisclosure } from "./HelpDisclosure";
import { Icon } from "./icons";
import type { IconName } from "./iconShapes";
import { paletteScore } from "./paletteItems";
import { scrollBehavior } from "./reducedMotion";
import { usePanelMemory } from "./usePanelMemory";
import type { Session } from "@audiorouter/contracts";
import type { ClientRow } from "./backend";
import { lazy, startTransition, Suspense, useLayoutEffect, useRef, useState, type ReactNode } from "react";

const ApiPanel = lazy(() => import("./ApiPanel").then((module) => ({ default: module.ApiPanel })));
const LogFilesPanel = lazy(() => import("./LogFilesPanel").then((module) => ({ default: module.LogFilesPanel })));

/** A page of the full-window Settings view. */
export type SettingsPage =
  | "help"
  | "virtualCables"
  | "plugins"
  | "deviceAccess"
  | "uiOptions"
  | "startup"
  | "shortcuts"
  | "versions"
  | "mcp"
  | "api"
  | "diagnostics"
  | "clients"
  | "sessionJson"
  | "catalog"
  | "identity";

/** Status has three views under one tab. */
export type StatusView = "status" | "apps" | "timing";

export type WorkbenchTab = "tools" | "properties" | "session" | "recording" | StatusView | SettingsPage;

const DAILY_TABS: [WorkbenchTab, string][] = [
  ["tools", "Tools"],
  ["properties", "Properties"],
  ["status", "Status"],
  ["session", "Session"],
  ["recording", "Recording"],
];

const STATUS_VIEWS: [StatusView, string][] = [
  ["status", "Readiness"],
  ["apps", "Apps"],
  ["timing", "Timing"],
];

const SETTINGS_GROUPS: { title: string; pages: [SettingsPage, string][] }[] = [
  {
    title: "Audio setup",
    pages: [
      ["virtualCables", "Virtual Cables"],
      ["plugins", "Plug-ins"],
      ["deviceAccess", "Device access"],
    ],
  },
  {
    title: "This app",
    pages: [
      ["uiOptions", "Appearance"],
      ["startup", "When AudioRouter starts"],
      ["shortcuts", "Keyboard shortcuts"],
      ["versions", "New versions"],
      ["help", "Help"],
    ],
  },
  {
    title: "For developers",
    pages: [
      ["mcp", "AI assistants (MCP)"],
      ["api", "API"],
      ["diagnostics", "Logs and support"],
      ["clients", "Connected clients"],
      ["sessionJson", "Session as JSON"],
      ["catalog", "Tool catalog"],
      ["identity", "App identity"],
    ],
  },
];

export const SETTINGS_PAGES = SETTINGS_GROUPS.flatMap((group) => group.pages);

/** One icon per page; a Record, so a new page cannot ship without one. */
export const SETTINGS_PAGE_ICONS: Record<SettingsPage, IconName> = {
  virtualCables: "cable",
  plugins: "puzzle",
  deviceAccess: "shield-check",
  uiOptions: "palette",
  startup: "power",
  shortcuts: "keyboard",
  versions: "refresh-cw",
  help: "circle-help",
  mcp: "bot",
  api: "code-xml",
  diagnostics: "scroll-text",
  clients: "users",
  sessionJson: "braces",
  catalog: "book-open",
  identity: "app-window",
};

/** Words people search for, shared by the Settings search and Ctrl+K. */
export const SETTINGS_PAGE_KEYWORDS: Record<SettingsPage, string> = {
  virtualCables: "cable driver virtual device install",
  plugins: "vst vst2 vst3 effects scan folder",
  deviceAccess: "permission allow devices microphone privacy",
  uiOptions: "theme dark light high contrast colors look animation",
  startup: "sign in login autostart autoplay tray",
  shortcuts: "keys keyboard hotkey shortcut chord",
  versions: "update release upgrade",
  help: "guide manual how to learn",
  mcp: "ai assistant claude chatgpt llm",
  api: "rest http token stream deck script",
  diagnostics: "logs support bug report export",
  clients: "connections access revoke",
  sessionJson: "export import json file",
  catalog: "tools list kinds",
  identity: "about version build",
};

/** Settings pages whose name or keywords match, in their usual order. */
export function matchingSettingsPages(query: string): Set<SettingsPage> {
  return new Set(
    SETTINGS_PAGES.filter(
      ([id, title]) => paletteScore({ title, detail: "", keywords: SETTINGS_PAGE_KEYWORDS[id] }, query) > 0,
    ).map(([id]) => id),
  );
}

/** Exhaustive, so a new tab cannot silently fall into the wrong place. */
export function tabPlace(tab: WorkbenchTab): "daily" | "status" | "settings" {
  switch (tab) {
    case "tools":
    case "properties":
    case "session":
    case "recording":
      return "daily";
    case "status":
    case "apps":
    case "timing":
      return "status";
    case "virtualCables":
    case "plugins":
    case "deviceAccess":
    case "uiOptions":
    case "startup":
    case "shortcuts":
    case "versions":
    case "help":
    case "mcp":
    case "api":
    case "diagnostics":
    case "clients":
    case "sessionJson":
    case "catalog":
    case "identity":
      return "settings";
  }
}

const TAB_HELP: Partial<Record<WorkbenchTab, { title: string; content: ReactNode }>> = {
  tools: {
    title: "How to build a route",
    content: (
      <ol>
        <li>Add a source: Input Device, Test Signal, Audio File, or an application.</li>
        <li>Add processing tools such as Gain or Advanced EQ if needed.</li>
        <li>Add an Output Device and choose your speakers or headphones in Properties.</li>
        <li>Drag from the sending tool's blue dot to the receiving tool's orange dot.</li>
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
        Check the six readiness indicators and the audio devices available on this PC. Choose a tool’s device in
        Properties.
      </p>
    ),
  },
  apps: {
    title: "How Apps works",
    content: (
      <p>
        AudioRouter asks Windows which apps use each audio device and compares that with your route. It only reads: it
        never changes Discord, your games or Windows. Change a setting in the app, then switch back here to see the
        check update.
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

function clientRoleGuidance(role: ClientRow["role"] | null) {
  switch (role) {
    case "observer":
      return "Read-only: the assistant can inspect sessions, devices, and route status.";
    case "editor":
      return "Can also create and change saved sessions and routes.";
    case "operator":
      return "Can also control audio that is already playing, including mute and Stop.";
    default:
      return "Authorize this exact client as Observer to allow read-only access. Change its role only when you need additional actions.";
  }
}

function mcpFailureGuidance(errorKind: string | null | undefined) {
  switch (errorKind) {
    case "permissionDenied":
      return "The client lacks permission for that action. Check its role under Client permissions; a role change is always your choice.";
    case "unavailable":
      return "AudioRouter or a required local service was unavailable. Reconnect AudioRouter, then try the read-only session check again.";
    case "revisionConflict":
      return "The session changed during the request. Ask the assistant to reread the session and review the new change before trying again.";
    case "invalidParams":
      return "The request did not match the tool's required inputs. Ask the assistant to read the tool description and retry with the listed fields.";
    case "notFound":
      return "The requested session or item was not found. Refresh the session list and select an existing item.";
    case "rateLimited":
      return "Too many requests arrived at once. Wait briefly, then retry once.";
    case "unsupported":
      return "This action is not available in the current AudioRouter setup. Review the tool result and available capabilities.";
    default:
      return "See the activity entry for the tool and outcome. Argument values are intentionally not stored.";
  }
}

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
  onDiscard,
  sessions,
  selectedSessionId,
  onSelectSession,
  sessionName,
  onNameChange,
  diagnostics,
  backendActivity,
  mcpActivity,
  mcpClients,
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
  statusHasProblem = false,
  virtualCablesContent,
  appsContent,
  uiOptionsContent,
  timingContent,
  recordingContent,
  settingsContent,
  savedVersionsContent,
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
  onAdd: (kind: NonNullable<LibraryEntry["kind"]>, options?: { focus?: boolean }) => void;
  /** Adds a cable source or destination on its default cable. */
  onAddCable?: (direction: "renderSource" | "captureSink") => void;
  onNewSession: () => void;
  onDuplicate: () => void;
  onApplicationPicker: () => void;
  librarySearch: string;
  onLibrarySearch: (value: string) => void;
  onDelete: () => void;
  onDiscard: () => void;
  onPlan?: () => void;
  onCommit?: () => void;
  canCommit?: boolean;
  pendingPlan?: boolean;
  actionMessage?: string | null;
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
  mcpClients: ClientRow[];
  mcpSetupInfo: McpSetupInfo | null;
  mcpTunnelStatus: McpTunnelStatus | null;
  onOpenMcpSetupPage: (page: "tunnels" | "apiKeys" | "chatgptConnectors") => Promise<void>;
  onMcpTunnelConfigure: (tunnelId: string, runtimeKey: string) => Promise<void>;
  onMcpTunnelConnect: () => Promise<void>;
  onMcpTunnelDisconnect: () => Promise<void>;
  onMcpTunnelForget: () => Promise<void>;
  onExportCoworkPlugin?: () => Promise<string>;
  clientsPanel: (initialClientId?: string) => ReactNode;
  statusContent: ReactNode;
  statusHasProblem?: boolean;
  virtualCablesContent: ReactNode;
  appsContent: ReactNode;
  uiOptionsContent: ReactNode;
  timingContent?: ReactNode;
  recordingContent: ReactNode;
  /** Pages that App builds whole; the rest are drawn here. */
  settingsContent: Partial<Record<SettingsPage, ReactNode>>;
  /** Session › Saved versions. */
  savedVersionsContent?: ReactNode;
  pluginsContent?: ReactNode;
  sessionFileContent?: ReactNode;
  apiBuilder?: (baseUrl: string) => ReactNode;
  verboseLogging?: VerboseLoggingControl;
}) {
  const tabsRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const previousTab = useRef(tab);
  const lastFocusedControl = useRef<HTMLElement | null>(null);
  const place = tabPlace(tab);
  const inSettings = place === "settings";
  // Where "Back to the route" returns, and which page the gear reopens.
  const lastRouteTab = useRef<WorkbenchTab>(inSettings ? "tools" : tab);
  const lastSettingsPage = useRef<WorkbenchTab>(inSettings ? tab : "virtualCables");
  const [settingsQuery, setSettingsQuery] = useState("");
  const shownPages = matchingSettingsPages(settingsQuery);
  const shownList = SETTINGS_PAGES.filter(([id]) => shownPages.has(id));
  // Keep one shown tab reachable with Tab even when the open page is filtered out.
  const focusablePage = shownPages.has(tab as SettingsPage) ? tab : shownList[0]?.[0];
  useLayoutEffect(() => {
    if (inSettings) lastSettingsPage.current = tab;
    else lastRouteTab.current = tab;
  }, [inSettings, tab]);
  useLayoutEffect(() => {
    // Only move focus when a tab switch removed its owner. External selection
    // and telemetry/error updates must not steal focus from a surviving control.
    if (
      previousTab.current !== tab &&
      document.activeElement === document.body &&
      lastFocusedControl.current &&
      !lastFocusedControl.current.isConnected
    ) {
      tabsRef.current?.querySelector<HTMLButtonElement>('[aria-selected="true"]')?.focus();
    }
    previousTab.current = tab;
  }, [tab]);
  useLayoutEffect(() => {
    const tabs = tabsRef.current;
    const workspace = tabs?.closest<HTMLElement>(".main-content");
    if (!tabs || !workspace) return;
    // The Properties overlay sits under these tabs; Settings has no such overlay.
    if (inSettings) return;
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
  }, [inSettings]);
  const [localAssistant, setLocalAssistant] = useState("codex");
  const [mcpConnectionPath, setMcpConnectionPath] = useState<"local" | "cowork" | "chatgpt">("local");
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
  const localClient = mcpClients.find((client) => client.clientId === mcpClientId);
  const localClientActivity = mcpActivity.find((entry) => entry.clientId === mcpClientId);
  const localClientRole = localClient && !localClient.revoked ? localClient.role : null;
  const selectedMcpClientId =
    mcpConnectionPath === "chatgpt"
      ? "audiorouter-chatgpt"
      : mcpConnectionPath === "cowork"
        ? "audiorouter-cowork-local"
        : mcpClientId;
  const selectedMcpClient = mcpClients.find((client) => client.clientId === selectedMcpClientId);
  const selectedMcpClientRole = selectedMcpClient && !selectedMcpClient.revoked ? selectedMcpClient.role : null;
  const localCheckText = !mcpSetupInfo
    ? "Checking the installed MCP command and local client access…"
    : !mcpSetupInfo.cliAvailable
      ? "The installed MCP command is missing. Repair or update AudioRouter before connecting an assistant."
      : !connected
        ? "AudioRouter is not connected to its backend. Reconnect the app before testing MCP."
        : !localClientRole
          ? localClient?.revoked
            ? "This client is revoked. Review Client permissions before reconnecting it."
            : "This client is not authorized yet. Authorize it as Observer to allow read-only access."
          : localClientActivity?.outcome === "error"
            ? mcpFailureGuidance(localClientActivity.errorKind)
            : localClientActivity?.outcome === "ok"
              ? "A local MCP request succeeded. This confirms local access, not that an external connector is enabled."
              : "Ready for a read-only test. Ask the assistant to list AudioRouter sessions, then check Recent MCP activity.";
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
  const tabs = DAILY_TABS;
  const selectedDailyTab: WorkbenchTab = place === "status" ? "status" : tab;
  const panelRef = usePanelMemory(tab);
  const panelProps = {
    id: `workbench-panel-${tab}`,
    "aria-labelledby": place === "status" ? `workbench-tab-status-view-${tab}` : `workbench-tab-${tab}`,
    tabIndex: 0,
  };
  const page = (id: SettingsPage, heading: string, content: ReactNode, eyebrow?: string) =>
    tab === id && (
      <section className="workbench-page" role="tabpanel" {...panelProps}>
        {eyebrow && <p className="eyebrow">{eyebrow}</p>}
        <h2>{heading}</h2>
        {content}
      </section>
    );
  return (
    <aside
      ref={panelRef}
      className={`right-workbench${inSettings ? " settings-view" : ""}`}
      id="routing-workbench"
      tabIndex={-1}
      aria-label="Tools and settings"
      onFocusCapture={(event) => {
        lastFocusedControl.current = event.target;
      }}
    >
      {inSettings && (
        <nav className="settings-nav" aria-label="Settings">
          <button
            type="button"
            className="secondary settings-back"
            onClick={() => startTransition(() => onTab(lastRouteTab.current))}
          >
            <span aria-hidden="true">←</span> Back to the route
          </button>
          <label className="settings-search">
            <span>Find a setting</span>
            <input
              type="search"
              value={settingsQuery}
              placeholder="Shortcut, theme, logs…"
              aria-describedby="settings-search-hint"
              onChange={(event) => setSettingsQuery(event.target.value)}
              onKeyDown={(event) => {
                const first = shownList[0];
                if (event.key === "Enter" && first) {
                  event.preventDefault();
                  startTransition(() => onTab(first[0]));
                } else if (event.key === "Escape" && settingsQuery) {
                  event.preventDefault();
                  setSettingsQuery("");
                }
              }}
            />
            <small id="settings-search-hint">
              {shownList.length === 0
                ? "No settings match. Ctrl+K searches everything."
                : "Ctrl+K searches everything."}
            </small>
          </label>
          <div
            role="tablist"
            aria-orientation="vertical"
            aria-label="Settings pages"
            ref={tabsRef}
            onKeyDown={(event) => {
              if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
              const index = shownList.findIndex(([id]) => id === tab);
              const count = shownList.length;
              if (count === 0) return;
              const next = shownList[(index + (event.key === "ArrowDown" ? 1 : count - 1) + count) % count];
              if (!next) return;
              event.preventDefault();
              onTab(next[0]);
              window.requestAnimationFrame(() => document.getElementById(`workbench-tab-${next[0]}`)?.focus());
            }}
          >
            {SETTINGS_GROUPS.filter((group) => group.pages.some(([id]) => shownPages.has(id))).map((group) => (
              <div key={group.title} className="settings-nav-group" role="presentation">
                <p className="eyebrow" role="presentation">
                  {group.title}
                </p>
                {group.pages
                  .filter(([id]) => shownPages.has(id))
                  .map(([id, label]) => (
                    <button
                      key={id}
                      type="button"
                      role="tab"
                      id={`workbench-tab-${id}`}
                      aria-controls={`workbench-panel-${id}`}
                      aria-selected={tab === id}
                      tabIndex={focusablePage === id ? 0 : -1}
                      className={tab === id ? "active" : ""}
                      onClick={() => startTransition(() => onTab(id))}
                    >
                      <Icon name={SETTINGS_PAGE_ICONS[id]} size={16} />
                      {label}
                    </button>
                  ))}
              </div>
            ))}
          </div>
        </nav>
      )}
      <div ref={inSettings ? undefined : tabsRef} className="workbench-tabs" hidden={inSettings}>
        <div className="workbench-tablist" role="tablist" aria-label="Right sidebar">
          {tabs.map(([id, label]) => (
            <button
              key={label}
              type="button"
              role="tab"
              id={`workbench-tab-${id}`}
              aria-controls={
                id === "properties"
                  ? "workbench-properties"
                  : `workbench-panel-${id === "status" && place === "status" ? tab : id}`
              }
              tabIndex={selectedDailyTab === id ? 0 : -1}
              aria-selected={selectedDailyTab === id}
              aria-describedby={id === "status" && statusHasProblem ? "status-problem-description" : undefined}
              title={id === "status" && statusHasProblem ? "Setup problems found. Open Status for details." : undefined}
              className={selectedDailyTab === id ? "active" : ""}
              onKeyDown={(event) => {
                // The Properties overlay precedes the tablist in DOM order.
                if (id === "properties" && event.key === "Tab" && !event.shiftKey) {
                  if (event.currentTarget.dataset.propertiesExit) {
                    delete event.currentTarget.dataset.propertiesExit;
                    return;
                  }
                  const panel = document.getElementById("workbench-properties");
                  if (panel) {
                    event.preventDefault();
                    panel.focus();
                  }
                  return;
                }
                const index = tabs.findIndex(([, name]) => name === label);
                const next =
                  event.key === "Home"
                    ? 0
                    : event.key === "End"
                      ? tabs.length - 1
                      : event.key === "ArrowRight"
                        ? (index + 1) % tabs.length
                        : event.key === "ArrowLeft"
                          ? (index + tabs.length - 1) % tabs.length
                          : null;
                if (next === null || event.altKey || event.ctrlKey || event.metaKey) return;
                event.preventDefault();
                const target = tabsRef.current?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next];
                target?.focus();
                target?.click();
              }}
              onClick={() => startTransition(() => onTab(id))}
            >
              {label}
              {id === "status" && (
                <span
                  aria-hidden="true"
                  data-testid="status-problem-indicator"
                  className={`status-problem-indicator${statusHasProblem ? " is-problem" : ""}`}
                />
              )}
            </button>
          ))}
        </div>
        <button
          type="button"
          className="settings-gear"
          aria-label="Settings"
          title="Settings"
          onClick={() => startTransition(() => onTab(lastSettingsPage.current))}
        >
          <Icon name="settings" />
        </button>
      </div>
      <span id="status-problem-description" hidden>
        Setup problems found. Open Status for details.
      </span>
      {place === "status" && (
        <div className="status-views" role="tablist" aria-label="Status views">
          {STATUS_VIEWS.map(([id, label]) => (
            <button
              key={id}
              type="button"
              role="tab"
              id={`workbench-tab-status-view-${id}`}
              aria-controls={`workbench-panel-${id}`}
              aria-selected={tab === id}
              tabIndex={tab === id ? 0 : -1}
              className={tab === id ? "active" : ""}
              onKeyDown={(event) => {
                if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
                const index = STATUS_VIEWS.findIndex(([key]) => key === id);
                const count = STATUS_VIEWS.length;
                const next = STATUS_VIEWS[(index + (event.key === "ArrowRight" ? 1 : count - 1)) % count];
                if (!next) return;
                event.preventDefault();
                onTab(next[0]);
                document.getElementById(`workbench-tab-status-view-${next[0]}`)?.focus();
              }}
              onClick={() => startTransition(() => onTab(id))}
            >
              {label}
            </button>
          ))}
        </div>
      )}
      {TAB_HELP[tab] && (
        <div className="workbench-help">
          <HelpDisclosure key={tab} title={TAB_HELP[tab].title}>
            {TAB_HELP[tab].content}
          </HelpDisclosure>
        </div>
      )}
      <Suspense
        fallback={
          <section className="workbench-page workbench-loading" role="status" aria-busy="true">
            Loading panel...
          </section>
        }
      >
        {tab === "tools" && (
          <div className="workbench-tool-search">
            <label>
              Find a tool
              <input
                ref={searchRef}
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
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <ToolShelf
              tools={tools}
              connected={connected}
              search={librarySearch}
              searchRef={searchRef}
              pluginsContent={pluginsContent}
              onAdd={onAdd}
              onAddCable={onAddCable}
              onAddGroup={onAddGroup}
              onApplicationPicker={onApplicationPicker}
              onClearSearch={() => onLibrarySearch("")}
              onOpenVirtualCables={() => onTab("virtualCables")}
            />
          </section>
        )}
        {tab === "api" && (
          <div role="tabpanel" {...panelProps}>
            <ApiPanel builder={apiBuilder} />
          </div>
        )}
        {tab === "timing" && (
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <p className="eyebrow">Where the sound spends time</p>
            <h2>Signal timing</h2>
            {timingContent}
          </section>
        )}
        {tab === "properties" && (
          <section className="workbench-page">
            <h2>Tool properties</h2>
            <p className="muted">Select a tool on the canvas to edit it here.</p>
          </section>
        )}
        {tab === "status" && (
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <h2>Status</h2>
            {statusContent}
          </section>
        )}
        {tab === "apps" && (
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <h2>Apps</h2>
            {appsContent}
          </section>
        )}
        {page("virtualCables", "Virtual Cables", virtualCablesContent)}
        {page("uiOptions", "Appearance", uiOptionsContent)}
        {page("plugins", "Plug-ins", settingsContent.plugins)}
        {page("deviceAccess", "Device access", settingsContent.deviceAccess)}
        {page("startup", "When AudioRouter starts", settingsContent.startup)}
        {page("shortcuts", "Keyboard shortcuts", settingsContent.shortcuts)}
        {page("versions", "New versions", settingsContent.versions)}
        {page("help", "Help", settingsContent.help)}
        {page("clients", "Connected clients", settingsContent.clients)}
        {page("sessionJson", "Session as JSON", settingsContent.sessionJson, "For scripts")}
        {page("catalog", "Tool catalog", settingsContent.catalog)}
        {page("identity", "App identity", settingsContent.identity)}
        {tab === "recording" && (
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <h2>Recording</h2>
            <p className="muted">
              Recording starts only after you explicitly arm and start a recorder. Confirm the destination and format
              before recording.
            </p>
            {recordingContent}
          </section>
        )}
        {tab === "session" && (
          <section className="workbench-page session-page" role="tabpanel" {...panelProps}>
            <h2>Sessions</h2>
            <p className="muted">
              Switch between saved audio setups, or create a copy to try another arrangement. Use Save at the top when
              you want to keep your edits.
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
              <button type="button" className="secondary" onClick={onDiscard} disabled={!connected}>
                Revert edits
              </button>
              <button type="button" className="danger" onClick={onDelete} disabled={!connected}>
                Delete session
              </button>
            </div>
            {sessionFileContent}
            {savedVersionsContent}
            <SessionIdentity sessionId={selectedSessionId} />
          </section>
        )}
        {tab === "mcp" && (
          <section className="workbench-page mcp-page" role="tabpanel" {...panelProps}>
            <header className="mcp-heading">
              <p className="eyebrow">Model Context Protocol</p>
              <h2>MCP connections</h2>
              <p className="muted">
                AudioRouter speaks standard MCP. Choose a local app below, or connect ChatGPT through its optional
                OpenAI tunnel.
              </p>
            </header>

            <section className="mcp-path-picker" aria-labelledby="mcp-path-heading">
              <h3 id="mcp-path-heading">Where does your assistant run?</h3>
              <div className="mcp-path-options" role="group" aria-label="Assistant connection type">
                <button
                  type="button"
                  aria-pressed={mcpConnectionPath === "local"}
                  className={mcpConnectionPath === "local" ? "active" : "secondary"}
                  onClick={() => setMcpConnectionPath("local")}
                >
                  On this PC
                </button>
                <button
                  type="button"
                  aria-pressed={mcpConnectionPath === "cowork"}
                  className={mcpConnectionPath === "cowork" ? "active" : "secondary"}
                  onClick={() => setMcpConnectionPath("cowork")}
                >
                  Local Cowork
                </button>
                <button
                  type="button"
                  aria-pressed={mcpConnectionPath === "chatgpt"}
                  className={mcpConnectionPath === "chatgpt" ? "active" : "secondary"}
                  onClick={() => setMcpConnectionPath("chatgpt")}
                >
                  ChatGPT
                </button>
              </div>
              <p className="muted mcp-path-note" aria-live="polite">
                {mcpConnectionPath === "local"
                  ? "Use the local stdio command for Codex, Claude Code, or another assistant running on this PC."
                  : mcpConnectionPath === "cowork"
                    ? "Local Cowork uses an exported plugin on this PC. Cloud Cowork cannot use the local command; remote access is not available yet."
                    : "ChatGPT uses the optional OpenAI Secure MCP Tunnel below. It is separate from standard local MCP."}
              </p>
              <div className="mcp-client-access" role="status" aria-live="polite">
                <strong>
                  {selectedMcpClientRole
                    ? `${selectedMcpClientId}: ${selectedMcpClientRole}`
                    : selectedMcpClient?.revoked
                      ? `${selectedMcpClientId}: revoked`
                      : `${selectedMcpClientId || "Client ID required"}: not authorized`}
                </strong>
                <p>{clientRoleGuidance(selectedMcpClientRole)}</p>
                <p className="muted">
                  MCP clients cannot receive device administration or press Play to open audio devices.
                </p>
                <button
                  type="button"
                  className="secondary"
                  onClick={(event) => {
                    const panel = event.currentTarget
                      .closest(".workbench-page")
                      ?.querySelector<HTMLDetailsElement>("[data-client-permissions]");
                    if (panel) {
                      panel.open = true;
                      panel.scrollIntoView({ block: "start", behavior: scrollBehavior() });
                      panel.querySelector("summary")?.focus();
                    }
                  }}
                >
                  Set client permissions
                </button>
              </div>
            </section>

            {mcpConnectionPath === "chatgpt" && (
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
                  ChatGPT uses the separate <code>audiorouter-chatgpt</code> client with read-only access. To let it
                  create or edit sessions, grant Editor under Client permissions; grant Operator to also let it press
                  Soundboard sounds and control audio that is playing. You still press Play here. Audio stays on this
                  PC; only MCP requests and results cross the encrypted tunnel.
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
                        runtime key with <strong>Tunnels Read + Use</strong>. <strong>3.</strong> Enter both below. The
                        key is encrypted for your Windows account and is never saved in ChatGPT.
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
                              setTunnelMessage(
                                error instanceof Error ? error.message : "Could not connect the tunnel.",
                              ),
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
            )}

            {mcpConnectionPath === "cowork" && (
              <section className="mcp-remote-info" aria-labelledby="mcp-cowork-title">
                <div className="mcp-card-heading">
                  <h3 id="mcp-cowork-title">Cowork</h3>
                  <span className="mcp-status">Local plugin</span>
                </div>
                <p className="muted">
                  For local Cowork on this PC, export the plugin, then upload it in Cowork → Customize → Plugins. Keep
                  AudioRouter running. Export enables read-only access; choose Editor in Client permissions to allow
                  edits. Cloud Cowork tasks need a remote connection, which is not available yet.
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
            )}

            {mcpConnectionPath === "local" && (
              <details className="mcp-secondary-section local-assistant-setup" open>
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
                    <p className="muted">
                      Use this exact connection name below. Start with observer access; allow edits only when you need
                      them.
                    </p>
                  </li>
                  <li>
                    <h3>Check local access</h3>
                    <p className="mcp-local-check" role="status" aria-live="polite">
                      {localCheckText}
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
                              Choose the standard command / stdio transport. Enter the executable and arguments
                              separately.
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
                      Ask your assistant to list AudioRouter sessions. Check Recent MCP activity below for the request
                      and its outcome. This check confirms a request reached AudioRouter; it cannot confirm an external
                      app's connector approval or whether the assistant has enabled the connection in its conversation.
                    </p>
                  </li>
                </ol>
                <p className="mcp-copy-message" role="status">
                  {copyMessage || "Copy a setup above when you are ready."}
                </p>
              </details>
            )}

            <details
              className="mcp-secondary-section"
              data-client-permissions
              open={mcpConnectionPath === "local" && !localClientRole}
            >
              <summary>Client permissions</summary>
              <p className="muted">
                Observer reads; Editor creates or changes sessions; Operator also controls audio that is already
                playing. These roles do not grant device administration, microphone consent, recording access, or
                permission to press Play.
              </p>
              {clientsPanel(selectedMcpClientId)}
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
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <h2>Client diagnostics</h2>
            <LogFilesPanel verbose={verboseLogging} clientDiagnostics={diagnostics} />
            <p className="muted">
              Recent interface error categories and route checkpoints. Up to 80 short entries are saved in this local
              app profile so they survive a reload. Raw error text, file paths, and audio are excluded.
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
      </Suspense>
    </aside>
  );
}
