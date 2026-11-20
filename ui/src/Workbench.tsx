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
import { i18n } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";
import { useMessages } from "./i18n/useMessages";

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

const DAILY_TABS: [WorkbenchTab, MessageKey][] = [
  ["tools", "workbench.tools"],
  ["properties", "workbench.properties"],
  ["status", "workbench.status"],
  ["session", "workbench.session"],
  ["recording", "workbench.recording"],
];

const STATUS_VIEWS: [StatusView, MessageKey][] = [
  ["status", "workbench.readiness"],
  ["apps", "workbench.apps"],
  ["timing", "workbench.timing"],
];

const SETTINGS_GROUPS: { title: MessageKey; pages: [SettingsPage, MessageKey][] }[] = [
  {
    title: "settingsGroups.audioSetup",
    pages: [
      ["virtualCables", "settingsPages.virtualCables"],
      ["plugins", "settingsPages.plugins"],
      ["deviceAccess", "settingsPages.deviceAccess"],
    ],
  },
  {
    title: "settingsGroups.thisApp",
    pages: [
      ["uiOptions", "settingsPages.uiOptions"],
      ["startup", "settingsPages.startup"],
      ["shortcuts", "settingsPages.shortcuts"],
      ["versions", "settingsPages.versions"],
      ["help", "settingsPages.help"],
    ],
  },
  {
    title: "settingsGroups.forDevelopers",
    pages: [
      ["mcp", "settingsPages.mcp"],
      ["api", "settingsPages.api"],
      ["diagnostics", "settingsPages.diagnostics"],
      ["clients", "settingsPages.clients"],
      ["sessionJson", "settingsPages.sessionJson"],
      ["catalog", "settingsPages.catalog"],
      ["identity", "settingsPages.identity"],
    ],
  },
];

const SETTINGS_PAGE_TITLE_KEYS = Object.fromEntries(SETTINGS_GROUPS.flatMap((group) => group.pages)) as Record<
  SettingsPage,
  MessageKey
>;

export const SETTINGS_PAGES = SETTINGS_GROUPS.flatMap((group) => group.pages);

/** The Settings page name in the active language, for navigation and search. */
export function settingsPageTitle(page: SettingsPage): string {
  return i18n.t(SETTINGS_PAGE_TITLE_KEYS[page]) as string;
}

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
const SETTINGS_PAGE_KEYWORDS: Record<SettingsPage, string> = {
  virtualCables: "cable driver virtual device install",
  plugins: "vst vst2 vst3 effects scan folder",
  deviceAccess: "permission allow devices microphone privacy",
  uiOptions: "theme dark light high contrast colors look animation autosave save language",
  startup: "sign in login autostart autoplay tray",
  shortcuts: "keys keyboard hotkey shortcut chord",
  versions: "update release upgrade prerelease channel check now",
  help: "guide manual how to learn",
  mcp: "ai assistant claude chatgpt llm",
  api: "rest http token stream deck script",
  diagnostics: "logs support bug report export",
  clients: "connections access revoke",
  sessionJson: "export import json file",
  catalog: "tools list kinds",
  identity: "about version build",
};

/** Keep the English search alias available even after switching languages. */
export function settingsPageKeywords(page: SettingsPage): string {
  const keywords = SETTINGS_PAGE_KEYWORDS[page];
  return page === "uiOptions" ? `${keywords} ${i18n.t("appearance.language")}` : keywords;
}

/** Settings pages whose name or keywords match, in their usual order. */
export function matchingSettingsPages(query: string): Set<SettingsPage> {
  return new Set(
    SETTINGS_PAGES.filter(
      ([id]) =>
        paletteScore({ title: settingsPageTitle(id), detail: "", keywords: settingsPageKeywords(id) }, query) > 0,
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

const TAB_HELP: Partial<Record<WorkbenchTab, { title: MessageKey; body: MessageKey }>> = {
  timing: { title: "g6.wbHelpTimingTitle", body: "g6.wbHelpTimingBody" },
  status: { title: "g6.wbHelpStatusTitle", body: "g6.wbHelpStatusBody" },
  apps: { title: "g6.wbHelpAppsTitle", body: "g6.wbHelpAppsBody" },
  virtualCables: { title: "g6.wbHelpCablesTitle", body: "g6.wbHelpCablesBody" },
  uiOptions: { title: "g6.wbHelpUiTitle", body: "g6.wbHelpUiBody" },
  recording: { title: "g6.wbHelpRecordingTitle", body: "g6.wbHelpRecordingBody" },
  mcp: { title: "g6.wbHelpMcpTitle", body: "g6.wbHelpMcpBody" },
  api: { title: "g6.wbHelpApiTitle", body: "g6.wbHelpApiBody" },
  diagnostics: { title: "g6.wbHelpLogsTitle", body: "g6.wbHelpLogsBody" },
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

const TUNNEL_DESCRIPTION_KEYS: Record<McpTunnelStatus["status"], MessageKey> = {
  connected: "g6.wbTunnelDescConnected",
  connecting: "g6.wbTunnelDescConnecting",
  actionNeeded: "g6.wbTunnelDescAction",
  needsSetup: "g6.wbTunnelDescNeedsSetup",
  disconnected: "g6.wbTunnelDescDisconnected",
  notConfigured: "g6.wbTunnelDescNotConfigured",
};

const TUNNEL_STATUS_KEYS: Record<McpTunnelStatus["status"], MessageKey> = {
  connected: "g6.wbTunnelStatusConnected",
  connecting: "g6.wbTunnelStatusConnecting",
  actionNeeded: "g6.wbTunnelStatusAction",
  needsSetup: "g6.wbTunnelStatusNeedsSetup",
  disconnected: "g6.wbTunnelStatusDisconnected",
  notConfigured: "g6.wbTunnelStatusNotConfigured",
};

function clientRoleGuidance(role: ClientRow["role"] | null): MessageKey {
  switch (role) {
    case "observer":
      return "g6.wbRoleObserver";
    case "editor":
      return "g6.wbRoleEditor";
    case "operator":
      return "g6.wbRoleOperator";
    default:
      return "g6.wbRoleNone";
  }
}

function mcpFailureGuidance(errorKind: string | null | undefined): MessageKey {
  switch (errorKind) {
    case "permissionDenied":
      return "g6.wbFailPermission";
    case "unavailable":
      return "g6.wbFailUnavailable";
    case "revisionConflict":
      return "g6.wbFailRevision";
    case "invalidParams":
      return "g6.wbFailInvalid";
    case "notFound":
      return "g6.wbFailNotFound";
    case "rateLimited":
      return "g6.wbFailRate";
    case "unsupported":
      return "g6.wbFailUnsupported";
    default:
      return "g6.wbFailDefault";
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
  const tr = useMessages();
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
      .then(() => setTunnelMessage(tr("g6.wbTunnelOpened")))
      .catch((error: unknown) =>
        setTunnelMessage(error instanceof Error ? error.message : tr("g6.wbTunnelOpenFailed")),
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
    ? tr("g6.wbCheckPending")
    : !mcpSetupInfo.cliAvailable
      ? tr("g6.wbCheckMissing")
      : !connected
        ? tr("g6.wbCheckOffline")
        : !localClientRole
          ? localClient?.revoked
            ? tr("g6.wbCheckRevoked")
            : tr("g6.wbCheckUnauthorized")
          : localClientActivity?.outcome === "error"
            ? tr(mcpFailureGuidance(localClientActivity.errorKind))
            : localClientActivity?.outcome === "ok"
              ? tr("g6.wbCheckOk")
              : tr("g6.wbCheckReady");
  const copy = (value: string) => {
    if (!navigator.clipboard) {
      setCopyMessage(tr("g6.wbClipboardUnavailable"));
      return;
    }
    void navigator.clipboard
      .writeText(value)
      .then(() => setCopyMessage(tr("g6.wbCopied")))
      .catch(() => setCopyMessage(tr("g6.wbClipboardUnavailable")));
  };
  const tabs = DAILY_TABS;
  const selectedDailyTab: WorkbenchTab = place === "status" ? "status" : tab;
  const panelRef = usePanelMemory(tab);
  const panelProps = {
    id: `workbench-panel-${tab}`,
    "aria-labelledby": place === "status" ? `workbench-tab-status-view-${tab}` : `workbench-tab-${tab}`,
    tabIndex: 0,
  };
  const page = (id: SettingsPage, content: ReactNode, eyebrow?: MessageKey) =>
    tab === id && (
      <section className="workbench-page" role="tabpanel" {...panelProps}>
        {eyebrow && <p className="eyebrow">{tr(eyebrow)}</p>}
        <h2>{settingsPageTitle(id)}</h2>
        {content}
      </section>
    );
  return (
    <aside
      ref={panelRef}
      className={`right-workbench${inSettings ? " settings-view" : ""}`}
      id="routing-workbench"
      tabIndex={-1}
      aria-label={tr("workbench.toolsAndSettings")}
      onFocusCapture={(event) => {
        lastFocusedControl.current = event.target;
      }}
    >
      {inSettings && (
        <nav className="settings-nav" aria-label={tr("workbench.settings")}>
          <button
            type="button"
            className="secondary settings-back"
            onClick={() => startTransition(() => onTab(lastRouteTab.current))}
          >
            <span aria-hidden="true">←</span> {tr("workbench.backToRoute")}
          </button>
          <label className="settings-search">
            <span>{tr("workbench.findSetting")}</span>
            <input
              type="search"
              value={settingsQuery}
              placeholder={tr("workbench.searchPlaceholder")}
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
              {shownList.length === 0 ? tr("workbench.noSettingsMatch") : tr("workbench.searchHint")}
            </small>
          </label>
          <div
            role="tablist"
            aria-orientation="vertical"
            aria-label={tr("workbench.settingsPages")}
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
                  {tr(group.title)}
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
                      {tr(label)}
                    </button>
                  ))}
              </div>
            ))}
          </div>
        </nav>
      )}
      <div ref={inSettings ? undefined : tabsRef} className="workbench-tabs" hidden={inSettings}>
        <div className="workbench-tablist" role="tablist" aria-label={tr("workbench.sidebar")}>
          {tabs.map(([id, label]) => (
            <button
              key={id}
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
              title={id === "status" && statusHasProblem ? tr("workbench.statusProblem") : undefined}
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
                const index = tabs.findIndex(([key]) => key === id);
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
              {tr(label)}
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
          aria-label={tr("workbench.settings")}
          title={tr("workbench.settings")}
          onClick={() => startTransition(() => onTab(lastSettingsPage.current))}
        >
          <Icon name="settings" />
        </button>
      </div>
      <span id="status-problem-description" hidden>
        {tr("workbench.statusProblem")}
      </span>
      {place === "status" && (
        <div className="status-views" role="tablist" aria-label={tr("g6.wbStatusViews")}>
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
              {tr(label)}
            </button>
          ))}
        </div>
      )}
      {tab === "tools" && (
        <div className="workbench-help">
          <HelpDisclosure key={tab} title={tr("toolShelf.routeHelp")}>
            <ol>
              <li>{tr("toolShelf.step1")}</li>
              <li>{tr("toolShelf.step2")}</li>
              <li>{tr("toolShelf.step3")}</li>
              <li>{tr("toolShelf.step4")}</li>
              <li>{tr("toolShelf.step5")}</li>
            </ol>
          </HelpDisclosure>
        </div>
      )}
      {tab === "session" && (
        <div className="workbench-help">
          <HelpDisclosure key={tab} title={tr("workflowUi.sessionHelp")}>
            <p>{tr("workflowUi.sessionHelpBody")}</p>
          </HelpDisclosure>
        </div>
      )}
      {TAB_HELP[tab] && (
        <div className="workbench-help">
          <HelpDisclosure key={tab} title={tr(TAB_HELP[tab].title)}>
            <p>{tr(TAB_HELP[tab].body)}</p>
          </HelpDisclosure>
        </div>
      )}
      <Suspense
        fallback={
          <section className="workbench-page workbench-loading" role="status" aria-busy="true">
            {tr("g6.wbLoadingPanel")}
          </section>
        }
      >
        {tab === "tools" && (
          <div className="workbench-tool-search">
            <label>
              {tr("toolShelf.find")}
              <input
                ref={searchRef}
                type="search"
                aria-label={tr("toolShelf.find")}
                value={librarySearch}
                onChange={(event) => onLibrarySearch(event.target.value)}
                placeholder={tr("toolShelf.placeholder")}
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
            <p className="eyebrow">{tr("g6.wbTimingEyebrow")}</p>
            <h2>{tr("workbench.signalTiming")}</h2>
            {timingContent}
          </section>
        )}
        {tab === "properties" && (
          <section className="workbench-page">
            <h2>{tr("workbench.toolProperties")}</h2>
            <p className="muted">{tr("g6.wbSelectTool")}</p>
          </section>
        )}
        {tab === "status" && (
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <h2>{tr("workbench.status")}</h2>
            {statusContent}
          </section>
        )}
        {tab === "apps" && (
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <h2>{tr("workbench.apps")}</h2>
            {appsContent}
          </section>
        )}
        {page("virtualCables", virtualCablesContent)}
        {page("uiOptions", uiOptionsContent)}
        {page("plugins", settingsContent.plugins)}
        {page("deviceAccess", settingsContent.deviceAccess)}
        {page("startup", settingsContent.startup)}
        {page("shortcuts", settingsContent.shortcuts)}
        {page("versions", settingsContent.versions)}
        {page("help", settingsContent.help)}
        {page("clients", settingsContent.clients)}
        {page("sessionJson", settingsContent.sessionJson, "workbench.forScripts")}
        {page("catalog", settingsContent.catalog)}
        {page("identity", settingsContent.identity)}
        {tab === "recording" && (
          <section className="workbench-page" role="tabpanel" {...panelProps}>
            <h2>{tr("workbench.recording")}</h2>
            <p className="muted">{tr("workflowUi.recordingIntro")}</p>
            {recordingContent}
          </section>
        )}
        {tab === "session" && (
          <section className="workbench-page session-page" role="tabpanel" {...panelProps}>
            <h2>{tr("workbench.sessions")}</h2>
            <p className="muted">{tr("workflowUi.sessionIntro")}</p>
            <label>
              {tr("workflowUi.currentSession")}
              <select
                aria-label={tr("workflowUi.chooseSession")}
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
                {tr("workflowUi.newSessionName")}
                <TextField
                  aria-label={tr("workflowUi.sessionName")}
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
                {tr("workflowUi.rename")}
              </button>
            )}
            <div className="session-action-grid">
              <button type="button" onClick={onNewSession} disabled={!connected}>
                {tr("workflowUi.newSession")}
              </button>
              <button type="button" className="secondary" onClick={onDuplicate} disabled={!connected}>
                {tr("workflowUi.duplicate")}
              </button>
              <button type="button" className="secondary" onClick={onDiscard} disabled={!connected}>
                {tr("workflowUi.revert")}
              </button>
              <button type="button" className="danger" onClick={onDelete} disabled={!connected}>
                {tr("workflowUi.deleteSession")}
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
              <p className="eyebrow">{tr("g6.wbMcpEyebrow")}</p>
              <h2>{tr("workbench.mcpConnections")}</h2>
              <p className="muted">{tr("settingsCopy.mcpIntro")}</p>
            </header>

            <section className="mcp-path-picker" aria-labelledby="mcp-path-heading">
              <h3 id="mcp-path-heading">{tr("settingsCopy.mcpWhere")}</h3>
              <div className="mcp-path-options" role="group" aria-label={tr("settingsCopy.mcpType")}>
                <button
                  type="button"
                  aria-pressed={mcpConnectionPath === "local"}
                  className={mcpConnectionPath === "local" ? "active" : "secondary"}
                  onClick={() => setMcpConnectionPath("local")}
                >
                  {tr("g6.wbPathLocal")}
                </button>
                <button
                  type="button"
                  aria-pressed={mcpConnectionPath === "cowork"}
                  className={mcpConnectionPath === "cowork" ? "active" : "secondary"}
                  onClick={() => setMcpConnectionPath("cowork")}
                >
                  {tr("g6.wbPathCowork")}
                </button>
                <button
                  type="button"
                  aria-pressed={mcpConnectionPath === "chatgpt"}
                  className={mcpConnectionPath === "chatgpt" ? "active" : "secondary"}
                  onClick={() => setMcpConnectionPath("chatgpt")}
                >
                  {tr("g6.wbPathChatgpt")}
                </button>
              </div>
              <p className="muted mcp-path-note" aria-live="polite">
                {mcpConnectionPath === "local"
                  ? tr("g6.wbNoteLocal")
                  : mcpConnectionPath === "cowork"
                    ? tr("g6.wbNoteCowork")
                    : tr("g6.wbNoteChatgpt")}
              </p>
              <div className="mcp-client-access" role="status" aria-live="polite">
                <strong>
                  {selectedMcpClientRole
                    ? tr("g6.wbClientRole", { id: selectedMcpClientId, role: selectedMcpClientRole })
                    : selectedMcpClient?.revoked
                      ? tr("g6.wbClientRevoked", { id: selectedMcpClientId })
                      : tr("g6.wbClientNotAuthorized", {
                          id: selectedMcpClientId || tr("g6.wbClientIdRequired"),
                        })}
                </strong>
                <p>{tr(clientRoleGuidance(selectedMcpClientRole))}</p>
                <p className="muted">{tr("g6.wbNoAdmin")}</p>
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
                  {tr("g6.wbSetPermissions")}
                </button>
              </div>
            </section>

            {mcpConnectionPath === "chatgpt" && (
              <section className="mcp-chatgpt-card" aria-labelledby="mcp-chatgpt-title">
                <div className="mcp-card-heading">
                  <div>
                    <h3 id="mcp-chatgpt-title">{tr("g6.wbTunnelTitle")}</h3>
                    <p className="muted mcp-tunnel-description">
                      {tr(
                        mcpTunnelStatus ? TUNNEL_DESCRIPTION_KEYS[mcpTunnelStatus.status] : "g6.wbTunnelDescChecking",
                      )}
                    </p>
                  </div>
                  <span className={`mcp-status mcp-status-${mcpTunnelStatus?.status ?? "checking"}`}>
                    {tr(mcpTunnelStatus ? TUNNEL_STATUS_KEYS[mcpTunnelStatus.status] : "g6.wbTunnelStatusChecking")}
                  </span>
                </div>
                <p>
                  {tr("g6.wbChatgptInfoBefore")}
                  <code>audiorouter-chatgpt</code>
                  {tr("g6.wbChatgptInfoAfter")}
                </p>
                {mcpTunnelStatus && !mcpTunnelStatus.runtimeAvailable && (
                  <p className="mcp-tunnel-error" role="status">
                    {tr("g6.wbRuntimeMissing")}
                  </p>
                )}
                {!mcpTunnelStatus && (
                  <p className="mcp-tunnel-message" role="status" aria-live="polite">
                    {tr("g6.wbCheckingTunnel")}
                  </p>
                )}
                {mcpTunnelStatus?.runtimeAvailable &&
                  (!mcpTunnelStatus.configured || mcpTunnelStatus.status === "needsSetup" || showTunnelSetup) && (
                    <div className="mcp-tunnel-setup">
                      <p className="muted">
                        <strong>1.</strong> {tr("g6.wbSetupStep1")} <strong>2.</strong> {tr("g6.wbSetupStep2")}{" "}
                        <strong>Tunnels Read + Use</strong>. <strong>3.</strong> {tr("g6.wbSetupStep3")}
                      </p>
                      <button type="button" className="mcp-chatgpt-open" onClick={() => openMcpSetupPage("tunnels")}>
                        {tr("g6.wbOpenTunnelSettings")}
                      </button>
                      <button type="button" className="mcp-chatgpt-open" onClick={() => openMcpSetupPage("apiKeys")}>
                        {tr("g6.wbCreateKey")}
                      </button>
                      <label>
                        {tr("g6.wbTunnelId")}
                        <input
                          aria-label={tr("g6.wbTunnelIdAria")}
                          autoComplete="off"
                          value={tunnelIdInput || mcpTunnelStatus?.tunnelId || ""}
                          onChange={(event) => setTunnelIdInput(event.target.value.trim())}
                          placeholder="tunnel_…"
                          disabled={tunnelBusy}
                        />
                      </label>
                      <label>
                        {tr("g6.wbRuntimeKey")}
                        <input
                          aria-label={tr("g6.wbRuntimeKeyAria")}
                          type="password"
                          autoComplete="new-password"
                          value={runtimeKeyInput}
                          onChange={(event) => setRuntimeKeyInput(event.target.value)}
                          placeholder={tr("g6.wbPasteKey")}
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
                            .then(() => setTunnelMessage(tr("g6.wbTunnelStarting")))
                            .catch((error: unknown) =>
                              setTunnelMessage(error instanceof Error ? error.message : tr("g6.wbTunnelConnectFailed")),
                            )
                            .finally(() => setTunnelBusy(false));
                        }}
                      >
                        {tunnelBusy ? tr("g6.wbConnecting") : tr("g6.wbSaveConnect")}
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
                                .then(() => setTunnelMessage(tr("g6.wbTunnelStopped")))
                                .catch(() => setTunnelMessage(tr("g6.wbTunnelStopFailed")))
                                .finally(() => setTunnelBusy(false));
                            }}
                          >
                            {tr("g6.wbDisconnect")}
                          </button>
                        ) : (
                          <button
                            type="button"
                            disabled={tunnelBusy || !mcpTunnelStatus.runtimeAvailable}
                            onClick={() => {
                              setTunnelBusy(true);
                              setTunnelMessage("");
                              void onMcpTunnelConnect()
                                .then(() => setTunnelMessage(tr("g6.wbTunnelStarting")))
                                .catch((error: unknown) =>
                                  setTunnelMessage(
                                    error instanceof Error ? error.message : tr("g6.wbTunnelConnectFailed"),
                                  ),
                                )
                                .finally(() => setTunnelBusy(false));
                            }}
                          >
                            {tunnelBusy
                              ? tr("g6.wbConnecting")
                              : mcpTunnelStatus.status === "actionNeeded"
                                ? tr("g6.wbRetry")
                                : tr("g6.wbConnect")}
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
                              {tr("g6.wbOpenConnectors")}
                            </button>
                            <p className="muted mcp-tunnel-chat-guidance">{tr("g6.wbNextConnector")}</p>
                          </>
                        )}
                        {mcpTunnelStatus.status !== "connected" && (
                          <p className="muted mcp-tunnel-chat-guidance">
                            {mcpTunnelStatus.status === "actionNeeded"
                              ? tr("g6.wbGuideRetry")
                              : mcpTunnelStatus.status === "connecting"
                                ? tr("g6.wbGuideConnecting")
                                : tr("g6.wbGuideDefault")}
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
                          {tr("g6.wbUpdateTunnel")}
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
                                setTunnelMessage(tr("g6.wbForgetRemoved"));
                              })
                              .catch(() => setTunnelMessage(tr("g6.wbForgetFailed")))
                              .finally(() => setTunnelBusy(false));
                          }}
                        >
                          {tr("g6.wbForget")}
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
                  <h3 id="mcp-cowork-title">{tr("g6.wbCoworkTitle")}</h3>
                  <span className="mcp-status">{tr("g6.wbLocalPlugin")}</span>
                </div>
                <p className="muted">{tr("g6.wbCoworkHelp")}</p>
                <div className="mcp-cowork-action">
                  {mcpSetupInfo?.cliAvailable && mcpSetupInfo.coworkPluginAvailable && onExportCoworkPlugin ? (
                    <button
                      type="button"
                      disabled={!connected || coworkBusy}
                      onClick={() => {
                        setCoworkBusy(true);
                        setCoworkMessage(tr("g6.wbCoworkChecking"));
                        void onExportCoworkPlugin()
                          .then((path) => setCoworkMessage(tr("g6.wbCoworkSaved", { path })))
                          .catch((error: unknown) =>
                            setCoworkMessage(error instanceof Error ? error.message : String(error)),
                          )
                          .finally(() => setCoworkBusy(false));
                      }}
                    >
                      {tr("g6.wbCoworkExport")}
                    </button>
                  ) : (
                    <p className="muted">{tr("g6.wbCoworkDesktopOnly")}</p>
                  )}
                </div>
                <p className="mcp-cowork-message" role="status" aria-live="polite">
                  {coworkMessage || tr("g6.wbCoworkNote")}
                </p>
              </section>
            )}

            {mcpConnectionPath === "local" && (
              <details className="mcp-secondary-section local-assistant-setup" open>
                <summary>{tr("g6.wbLocalSummary")}</summary>
                <p>{tr("g6.wbLocalIntro")}</p>
                <ol className="setup-instructions">
                  <li>
                    <h3>{tr("g6.wbChooseAssistant")}</h3>
                    <label>
                      {tr("g6.wbAssistant")}
                      <select
                        aria-label={tr("g6.wbAssistantAria")}
                        value={localAssistant}
                        onChange={(event) => setLocalAssistant(event.target.value)}
                      >
                        <option value="codex">Codex</option>
                        <option value="claude">Claude Code</option>
                        <option value="other">{tr("g6.wbAssistantOther")}</option>
                      </select>
                    </label>
                  </li>
                  <li>
                    <h3>{tr("g6.wbNameConnection")}</h3>
                    <p>{tr("g6.wbNameHelp")}</p>
                    <label>
                      {tr("g6.wbConnectionName")}
                      <input
                        aria-label={tr("g6.wbClientIdAria")}
                        value={mcpClientId}
                        maxLength={128}
                        onChange={(event) => setMcpClientId(event.target.value.trim())}
                      />
                    </label>
                    <p className="muted">{tr("g6.wbUseExactName")}</p>
                  </li>
                  <li>
                    <h3>{tr("g6.wbCheckAccess")}</h3>
                    <p className="mcp-local-check" role="status" aria-live="polite">
                      {localCheckText}
                    </p>
                  </li>
                  <li>
                    <h3>
                      {localAssistant === "codex"
                        ? tr("g6.wbAddCodex")
                        : localAssistant === "claude"
                          ? tr("g6.wbRunPowershell")
                          : tr("g6.wbAddCommand")}
                    </h3>
                    {mcpSetupInfo?.cliAvailable ? (
                      <>
                        {localAssistant === "codex" ? (
                          <>
                            <p>{tr("g6.wbPasteCodex")}</p>
                            <pre className="mcp-command">{codexConfig}</pre>
                            <button
                              type="button"
                              className="secondary"
                              disabled={!mcpClientId}
                              onClick={() => copy(codexConfig)}
                            >
                              {tr("g6.wbCopyCodex")}
                            </button>
                          </>
                        ) : localAssistant === "claude" ? (
                          <>
                            <p>{tr("g6.wbRunOnce")}</p>
                            <pre className="mcp-command">{claudeCommand}</pre>
                            <button
                              type="button"
                              className="secondary"
                              disabled={!mcpClientId}
                              onClick={() => copy(claudeCommand)}
                            >
                              {tr("g6.wbCopyClaude")}
                            </button>
                          </>
                        ) : (
                          <>
                            <p>{tr("g6.wbStdioHelp")}</p>
                            <label>
                              {tr("g6.wbExecutable")}
                              <input readOnly value={cli} />
                            </label>
                            <label>
                              {tr("g6.wbArguments")}
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
                            <p className="muted">{tr("g6.wbNotWebAddress")}</p>
                          </>
                        )}
                      </>
                    ) : (
                      <p className="mcp-tunnel-message" role="status">
                        {mcpSetupInfo ? tr("g6.wbCliMissing") : tr("g6.wbCliChecking")}
                      </p>
                    )}
                  </li>
                  <li>
                    <h3>{tr("g6.wbConfirm")}</h3>
                    <p>{tr("g6.wbConfirmHelp")}</p>
                  </li>
                </ol>
                <p className="mcp-copy-message" role="status">
                  {copyMessage || tr("g6.wbCopyPrompt")}
                </p>
              </details>
            )}

            <details
              className="mcp-secondary-section"
              data-client-permissions
              open={mcpConnectionPath === "local" && !localClientRole}
            >
              <summary>{tr("g6.wbPermissions")}</summary>
              <p className="muted">{tr("g6.wbPermissionsHelp")}</p>
              {clientsPanel(selectedMcpClientId)}
            </details>

            <details className="mcp-secondary-section">
              <summary>{tr("g6.wbActivity")}</summary>
              <p className="muted">{tr("g6.wbActivityHelp")}</p>
              {mcpActivity.length === 0 ? (
                <p className="muted mcp-empty">{tr("g6.wbNoActivity")}</p>
              ) : (
                <ol className="diagnostic-list" aria-label={tr("g6.wbActivityAria")}>
                  {mcpActivity.map((entry, index) => (
                    <li key={`${entry.timeUnixMs}-${index}`}>
                      <strong>
                        {new Date(entry.timeUnixMs).toLocaleTimeString()} · {entry.tool}
                      </strong>
                      <small>
                        {" "}
                        {tr("g6.wbActivityRow", {
                          outcome: entry.outcome,
                          error: entry.errorKind ? ` (${entry.errorKind})` : "",
                          client: entry.clientId,
                          fields: entry.argumentFields.join(", ") || tr("g6.wbNone"),
                        })}
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
            <h2>{tr("workbench.clientDiagnostics")}</h2>
            <LogFilesPanel verbose={verboseLogging} clientDiagnostics={diagnostics} />
            <p className="muted">{tr("g6.wbDiagIntro")}</p>
            <ol className="diagnostic-list">
              {diagnostics.map((row, index) => (
                <li key={`${index}-${row}`}>{row}</li>
              ))}
            </ol>
            <h3>{tr("g6.wbBackendRpc")}</h3>
            {backendActivity.length === 0 ? (
              <p className="muted">{tr("g6.wbNoBackend")}</p>
            ) : (
              <ol className="diagnostic-list" aria-label={tr("g6.wbBackendAria")}>
                {backendActivity.map((entry, index) => (
                  <li key={`${String(entry.timeUnixMs)}-${index}`}>
                    <strong>{String(entry.method ?? "RPC")}</strong> · {String(entry.outcome ?? tr("g6.wbUnknown"))}
                    {entry.errorCode == null ? "" : ` · ${tr("g6.wbErrorCode", { code: String(entry.errorCode) })}`}
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
