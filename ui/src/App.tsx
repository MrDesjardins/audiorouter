import { optionalLocalStorage, readPresentationJson, writePresentationJson } from "./presentationStorage";
import { Icon } from "./icons";
import { CopyErrorButton } from "./CopyErrorButton";
import { StartupRecovery } from "./StartupRecovery";
import { keepNativeContextMenu } from "./contextMenuPolicy";
import { SessionSwitchDialog } from "./SessionSwitchDialog";
import { SidechainReplaceDialog } from "./SidechainReplaceDialog";
import {
  customThemeStyle,
  readCustomThemeLibrary,
  selectedCustomTheme,
  writeCustomThemeLibrary,
  type CustomThemeLibrary,
} from "./customTheme";
import { UpdatesPanel } from "./UpdateNotice";
import { readUpdateChannel, setUpdateChannel, setUpdateCheckEnabled } from "./updateCheck";
import { useCanvasGroups } from "./CanvasGroups";
import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useEffectEvent,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import type { CanvasRect, LayoutAction } from "./canvasGeometry";
import { emptyEditOrder, nextEditKind, recordEdit, recordRestore } from "./editOrder";
import type { Node, PluginParametersResult } from "@audiorouter/contracts";
import { LIBRARY_DROP_SOURCE, SessionFlowCanvas, type CanvasLayoutHistoryControls } from "./SessionFlowCanvas";
import {
  SETTINGS_PAGE_ICONS,
  settingsPageKeywords,
  SETTINGS_PAGES,
  settingsPageTitle,
  tabPlace,
  Workbench,
  type WorkbenchTab,
} from "./Workbench";
import type { PaletteItem } from "./paletteItems";
import { libraryEntryIcon, NODE_KIND_ICONS } from "./icons";
import { humanizeNodeKind } from "./canvasNodeKinds";
import { useWorkbenchStatus } from "./useWorkbenchStatus";
import { useRecordingActions } from "./useRecordingActions";
import { VersionLine } from "./UpdateNotice";
import { isAutomatedHarness, noNetworkUpdateFetcher, updateCheckEnabled, useUpdateCheck } from "./updateCheck";
import {
  formatUiError,
  uiError,
  notReadyNode,
  isRevisionConflict,
  SnapshotCache,
  type ApplicationRow,
  type RecorderStatus,
  type UiBackend,
  type UiBackendSnapshot,
} from "./backend";
import type { DeviceListItem } from "@audiorouter/contracts";
import { LiveTelemetry, useTelemetryStore } from "./liveTelemetry";
import { SoundboardContext, useSoundboardController } from "./soundboardLive";
import { assignSoundboardSlotDraft, isSoundboardParameter } from "./soundboard";
import { AppDialogHost, askConfirm, askText } from "./AppDialog";
import { KeyboardHelp } from "./KeyboardHelp";
import { useDraftRecovery } from "./draftRecovery";
import { useNotificationsClearTabs } from "./notificationPlacement";
import { useReportUnsaved } from "./unsavedReport";
import { routePlaybackLabel } from "./routePlaybackLabel";
import { QuitButton } from "./QuitButton";
import {
  addSourceToOccupiedOutput,
  addSourceToOccupiedInput,
  appendApplicationCaptureNode,
  appendApplicationCapturePathNode,
  appendApplicationCaptureGroup,
  applicationOnlyRouteSource,
  generatedOnlyRoute,
  needsNativePaths,
  unboundDeviceNodes,
  isParameterOnlyChange,
  mixedApplicationRouteOtherSources,
  mixerRouteSources,
  rebindApplicationCaptureNode,
  appendDraftConnection,
  appendEndpointLoopbackNode,
  appendEqPresetNode,
  appendLibraryNode,
  appendPluginPlaceholderNode,
  appendVirtualBusNode,
  appendVoiceChainPreset,
  appendCablePassThrough,
  applyGraphDraft,
  copyDraftNode,
  duplicateDraftNode,
  pasteDraftNode,
  insertDraftMixer,
  insertDraftPluginProcessor,
  insertDraftProcessor,
  removeDraftConnection,
  removeDraftNode,
  removeSinglePathDraftMixer,
  resetNodeDraftParameters,
  routeFedMixerToOccupiedOutput,
  replaceDraftSidechainConnection,
  setDraftConnectionEnabled,
  setNodeDraftFlag,
  setNodeDraftName,
  setNodeDraftParameter,
  setSessionDraftName,
  type EqPresetId,
  type InsertableProcessorKind,
  type LibraryNodeKind,
  type VoiceChainPresetId,
} from "./draft";
import {
  actionMessageLifetimeMs,
  actionMessageTone,
  errorMessage,
  offersAction,
  recordingStoppedEarly,
  say,
  type UiMessage,
  WARNING_LIFETIME_MS,
  withTone,
} from "./actionMessage";
import { unfedRouteNodes } from "./draft";
import { demoSession, demoSessions } from "./fixtures";
import {
  recordDraft,
  redoDraft as redoDraftHistory,
  undoDraft as undoDraftHistory,
  type DraftHistory,
  type HistoryTransition,
} from "./history";
import { applyCableAvailability, filterLibraryEntries, libraryEntries, toolDescription } from "./library";
import {
  cableEndpointName,
  cableName,
  defaultCable,
  getVirtualCableStatus,
  setVirtualCableStatus,
  useVirtualCableStatus,
} from "./virtualCableStatus";
import {
  readFlowAnimation,
  readLastSession,
  readLanguagePreference,
  readLiveAutosave,
  readSidebarWidth,
  readTheme,
  writeFlowAnimation,
  writeLanguagePreference,
  writeLastSession,
  writeLiveAutosave,
  writeSidebarWidth,
  writeTheme,
  type FlowAnimationMode,
  type ThemeMode,
} from "./preferences";
import { applyLanguage } from "./i18n";
import { useMessages } from "./i18n/useMessages";
import { backendText, tm } from "./actionText";
import { SidebarResizer } from "./SidebarResizer";
import {
  announceDeviceAccessChanged,
  DeviceAccessSetting,
  DeviceAccessDialog,
  isDeviceAccessDenied,
} from "./DeviceAccess";
import { LibraryDragOverlay } from "./libraryDrag";
import { isEditableShortcutTarget } from "./keyboardTarget";
import { setupChecklist } from "./setup";
import { uiIdempotencyKey } from "./idempotency";
import { pluginNodeParameterError, processorParameterError, type ProcessorDescriptor } from "./processorCatalog";
import { mergeSessionInventory, reconcileSessionDraft, sameSessionDraft } from "./sessionInventory";
import { unsavedNodeIds } from "./nodeSaveState";
import { PRIVACY_MUTED_KINDS } from "./nodeAttention";
import type { Connection } from "@xyflow/react";
import { decodeTopologyAction } from "./DraftConnectionList";
import { BackendConnectionContext } from "./backendConnectionContext";
import {
  appendClientDiagnostic,
  browserDiagnosticStorage,
  describePromiseRejection,
  describeWindowError,
  isNewUiErrorRow,
  onRpcFailure,
  readClientDiagnostics,
} from "./clientDiagnostics";
import { ErrorBoundary, RootRecoveryPanel } from "./ErrorBoundary";
import { EqBackendContext, PluginParameterContext, defaultBackend } from "./appContext";
import {
  findReplacementCaptureEndpoint,
  findVbCableEndpointPair,
  captureEndpointName,
  readEndpointBindingHint,
  rememberDeviceNames,
  rememberDeviceNameBinding,
  rememberSessionDeviceNames,
  rememberedDeviceName,
  routeEndpointBinding,
} from "./endpointBinding";

import { PluginScanPanel, PluginToolsGroup } from "./PluginPanels";
export { formatRecordingDuration, recorderHasCaptureSource } from "./RecordingPanels";
export { findVbCableEndpointPair, findVbCableCaptureEndpointId, deviceChoiceLabel } from "./endpointBinding";
import { useAudioFileStatus } from "./useAudioFileStatus";
import { WORKSPACE_EVENT_CATEGORIES, useWorkspaceEvents } from "./useWorkspaceEvents";
import { DIAGNOSTICS_REFRESH_INTERVAL_MS, useDiagnosticsRefresh } from "./useDiagnosticsRefresh";
import { formatNativePumpSummary, useNativeCounters, type NativePumpStats } from "./useNativeCounters";
import { setupCheckRefreshKey } from "./setupCheckRefreshKey";
import { useSetupCheck } from "./useSetupCheck";
import type { DiscordSetupChoice } from "./DiscordSetup";
import { appendDiscordRoute, discordSetupOptions } from "./discordRoute";
import { AppStatusStrip } from "./AppStatusStrip";
import {
  discordCheck,
  nodeDeviceUsers,
  pendingDiscordCheck,
  sessionInvolvesDiscord,
  voiceCableOutputs,
} from "./appUsage";
import { APP_USAGE_FOLLOW_UP_MS, useApplicationUsage } from "./useApplicationUsage";
import type { ObsActions } from "./ObsCard";
import { obsCheck } from "./obsUsage";
import { useObsStatus } from "./useObsStatus";
import { PropertiesPanel } from "./PropertiesPanel";
import { SignalFlowPanel } from "./SignalFlowPanel";
export { WORKSPACE_EVENT_CATEGORIES, DIAGNOSTICS_REFRESH_INTERVAL_MS, formatNativePumpSummary };
export { NATIVE_COUNTERS_REFRESH_MS } from "./useNativeCounters";

// These panels mount only when their workbench tab is selected. Keep the
// editor and playback controls available while the selected panel loads.
const StartupSettings = lazy(() =>
  import("./SettingsSections").then((module) => ({ default: module.StartupSettings })),
);
const ShortcutSettings = lazy(() =>
  import("./SettingsSections").then((module) => ({ default: module.ShortcutSettings })),
);
const TroubleshootPanel = lazy(() =>
  import("./SettingsSections").then((module) => ({ default: module.TroubleshootPanel })),
);
const CanvasKeyboardPanel = lazy(() =>
  import("./SettingsSections").then((module) => ({ default: module.CanvasKeyboardPanel })),
);
const ProcessorCatalog = lazy(() =>
  import("./AdvancedPanels").then((module) => ({ default: module.ProcessorCatalog })),
);
const PresetCatalog = lazy(() => import("./AdvancedPanels").then((module) => ({ default: module.PresetCatalog })));
const SessionTransferPanel = lazy(() =>
  import("./AdvancedPanels").then((module) => ({ default: module.SessionTransferPanel })),
);
const ApplicationIdentityPanel = lazy(() =>
  import("./ApplicationIdentityPanel").then((module) => ({ default: module.ApplicationIdentityPanel })),
);
const RecordingWorkbench = lazy(() =>
  import("./RecordingWorkbench").then((module) => ({ default: module.RecordingWorkbench })),
);
const SetupWorkbench = lazy(() => import("./SetupWorkbench").then((module) => ({ default: module.SetupWorkbench })));
const UiOptionsWorkbench = lazy(() =>
  import("./UiOptionsWorkbench").then((module) => ({ default: module.UiOptionsWorkbench })),
);
const ClientsPanel = lazy(() => import("./AdvancedPanels").then((module) => ({ default: module.ClientsPanel })));
const RequestBuilder = lazy(() => import("./ApiRequestBuilder").then((module) => ({ default: module.RequestBuilder })));
const SessionFilePanel = lazy(() =>
  import("./SessionFilePanel").then((module) => ({ default: module.SessionFilePanel })),
);
const ConnectionForm = lazy(() => import("./ConnectionForm").then((module) => ({ default: module.ConnectionForm })));
const DeviceTroubleshooting = lazy(() =>
  import("./DeviceTroubleshooting").then((module) => ({ default: module.DeviceTroubleshooting })),
);
const SignalTimingPanel = lazy(() =>
  import("./SignalTiming").then((module) => ({ default: module.SignalTimingPanel })),
);
const AppsPanel = lazy(() => import("./AppsPanel").then((module) => ({ default: module.AppsPanel })));
const DiscordSetup = lazy(() => import("./DiscordSetup").then((module) => ({ default: module.DiscordSetup })));
const ObsCard = lazy(() => import("./ObsCard").then((module) => ({ default: module.ObsCard })));
const GoalDialog = lazy(() => import("./GoalDialog").then((module) => ({ default: module.GoalDialog })));
const FirstMinuteTour = lazy(() => import("./FirstMinuteTour").then((module) => ({ default: module.FirstMinuteTour })));
const CommandPalette = lazy(() => import("./CommandPalette").then((module) => ({ default: module.CommandPalette })));
const ChangeHistory = lazy(() => import("./ChangeHistory").then((module) => ({ default: module.ChangeHistory })));
const MicTest = lazy(() => import("./MicTest").then((module) => ({ default: module.MicTest })));
const FirstRunGuide = lazy(() => import("./FirstRunGuide").then((module) => ({ default: module.FirstRunGuide })));

/** Root failures outlive AppContent's state, so they go straight to storage for the next window. */
function recordStoredUiDiagnostic(message: string) {
  const storage = browserDiagnosticStorage();
  appendClientDiagnostic(storage, readClientDiagnostics(storage), message);
}

/**
 * The selection after a session list refresh: kept while listed, else the
 * first listed session. With a backend, the demo placeholder is kept too:
 * the startup snapshot replaces it with the remembered or active session,
 * and picking the first listed one here won that race and overwrote the
 * backend's active session.
 */
function keepSelectedSession(current: string, items: { id: string }[], connected: boolean): string {
  if (items.some((item) => item.id === current)) return current;
  if (connected && current === demoSession.id) return current;
  return items[0]?.id ?? current;
}

const FIRST_RUN_KEY = "audiorouter.ui.first-run-done";
const readFirstRunDone = () => readPresentationJson(optionalLocalStorage(), FIRST_RUN_KEY) === true;
const writeFirstRunDone = () => writePresentationJson(optionalLocalStorage(), FIRST_RUN_KEY, true);

/** Focus a canvas card once React Flow has rendered and shown it. */
function focusCanvasCardWhenShown(nodeId: string, attempts = 30) {
  window.requestAnimationFrame(() => {
    const card = Array.from(document.querySelectorAll<HTMLElement>(".react-flow__node")).find(
      (element) => element.dataset.id === nodeId,
    );
    // React Flow keeps a new card hidden until measured, and a hidden card
    // cannot take focus, so try again until focus actually lands.
    card?.focus();
    if (document.activeElement !== card && attempts > 1) focusCanvasCardWhenShown(nodeId, attempts - 1);
  });
}

export function App({ backend }: { backend?: UiBackend } = {}) {
  const resolvedBackend = backend ?? defaultBackend;
  return (
    <BackendConnectionContext.Provider value={resolvedBackend.connected}>
      <EqBackendContext.Provider value={resolvedBackend}>
        <ErrorBoundary
          area="AudioRouter"
          onError={recordStoredUiDiagnostic}
          fallback={() => (
            <RootRecoveryPanel
              onPrivacyMute={
                resolvedBackend.connected
                  ? () => resolvedBackend.setPrivacyMute(true, uiIdempotencyKey("privacy-mute"))
                  : undefined
              }
            />
          )}
        >
          <AppContent backend={resolvedBackend} />
        </ErrorBoundary>
      </EqBackendContext.Provider>
    </BackendConnectionContext.Provider>
  );
}

function AppContent({ backend = defaultBackend }: { backend?: UiBackend } = {}) {
  const m = useMessages();
  const [snapshotCache] = useState(() => new SnapshotCache());
  const [snapshotState, setSnapshotState] = useState(snapshotCache.current());
  const staleSnapshot = snapshotState.stale && snapshotState.snapshot !== null;
  const backendReady = backend.connected && !staleSnapshot;
  // Keep refresh and emergency safety actions on the original transport.
  // Editors must not mutate a route whose last snapshot could not be refreshed.
  const editorBackend = useMemo(
    () => (staleSnapshot ? { ...backend, connected: false } : backend),
    [backend, staleSnapshot],
  );
  const [rememberedSessionId] = useState(() => readLastSession(browserDiagnosticStorage()));
  const [selectedSessionId, setSelectedSessionId] = useState(rememberedSessionId ?? demoSession.id);
  // The backend's active session as last read or written, and this window's
  // writes still in flight (see the active-session effect below).
  const backendActiveSessionRef = useRef<string | null>(null);
  const activeSessionWritesRef = useRef(0);
  const currentSessionIdRef = useRef(selectedSessionId);
  currentSessionIdRef.current = selectedSessionId;
  const [selectedNodeId, setSelectedNodeId] = useState(demoSession.nodes[0].id);
  const [selectedNodeIds, setSelectedNodeIdsState] = useState<string[]>([demoSession.nodes[0].id]);
  const setSelectedNodeIds = (ids: string[]) => {
    setSelectedNodeIdsState((current) =>
      current.length === ids.length && current.every((id, index) => id === ids[index]) ? current : ids,
    );
  };
  const [draft, setDraft] = useState(demoSession);
  const draftRef = useRef(draft);
  draftRef.current = draft;
  const copiedNodeRef = useRef<Node | null>(null);
  const [hasCopiedNode, setHasCopiedNode] = useState(false);
  const authoritativeSession = useRef(demoSession);
  const hasAuthoritativeSession = useRef(!backend.connected);
  const [draftHistory, setDraftHistory] = useState<DraftHistory>({ past: [], future: [] });
  const [goalDialogOpen, setGoalDialogOpen] = useState(false);
  const [arrangeSessionId, setArrangeSessionId] = useState<string | null>(null);
  const [tourOpen, setTourOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [keyboardHelpOpen, setKeyboardHelpOpen] = useState(false);
  const [actionMessage, setActionMessage] = useState<UiMessage | null>(null);
  const [dismissedRouteWarning, setDismissedRouteWarning] = useState("");
  const [recordings, setRecordings] = useState<import("@audiorouter/contracts").RecordingRow[]>([]);
  const [recorderFormat, setRecorderFormat] = useState<import("@audiorouter/contracts").RecorderFileFormat>("wavPcm24");
  const [recorderStatuses, setRecorderStatuses] = useState<RecorderStatus[]>([]);
  const [audioSourceStates, setAudioSourceStates] = useState<Record<string, "playing" | "paused" | "stopped">>({});
  const audioSourceActivity = useRef(new Map<string, { version: number; pending: number }>());
  const [recorderStatusAvailable, setRecorderStatusAvailable] = useState(!backend.connected);
  const [applications, setApplications] = useState<ApplicationRow[]>([]);
  const [devices, setDevices] = useState<DeviceListItem[]>([]);
  const [captureEndpointId, setCaptureEndpointId] = useState(
    () => readEndpointBindingHint(demoSession.id).captureEndpointId ?? "",
  );
  const [renderEndpointId, setRenderEndpointId] = useState(
    () => readEndpointBindingHint(demoSession.id).renderEndpointId ?? "",
  );
  const [, setDevicesError] = useState<string | null>(null);
  const [applicationsError, setApplicationsError] = useState<string | null>(null);
  const [recordingsError, setRecordingsError] = useState<string | null>(null);
  const [metadataTitles, setMetadataTitles] = useState<Record<string, string>>({});
  const [metadataArtists, setMetadataArtists] = useState<Record<string, string>>({});
  const [metadataComments, setMetadataComments] = useState<Record<string, string>>({});
  const {
    previewMessage,
    recoveryMessage,
    recordingMutationBusyState,
    previewRecording,
    inspectRecovery,
    saveRecordingMetadata,
    removeRecordingEntry,
    renameRecording,
    revealRecording,
    recycleRecording,
  } = useRecordingActions({ backend, setRecordings, metadataTitles, metadataArtists, metadataComments });
  const [recordingSearch, setRecordingSearch] = useState("");
  const [pendingWarnings, setPendingWarnings] = useState<string[]>([]);
  const [acknowledgedWarnings, setAcknowledgedWarnings] = useState<Set<string>>(() => new Set());
  const [pendingOperation, setPendingOperation] = useState<string | null>(null);
  const [pendingGraphPlan, setPendingGraphPlan] = useState<{ planId: string; baseRevision: number } | null>(null);
  const [graphBusy, setGraphBusy] = useState(false);
  const sessionBusy = useRef(false);
  const nodeFlagBusy = useRef(false);
  const [sessionActionBusy, setSessionActionBusy] = useState(false);
  const sessionCrudBusy = useRef(false);
  const [sessionCrudBusyState, setSessionCrudBusyState] = useState(false);
  const safetyActionBusy = useRef(false);
  const [safetyActionBusyState, setSafetyActionBusyState] = useState(false);
  const [privacyMuted, setPrivacyMuted] = useState(true);
  const [librarySearch, setLibrarySearch] = useState("");
  const [processors, setProcessors] = useState<ProcessorDescriptor[] | null>(null);
  const [processorError, setProcessorError] = useState<string | null>(null);
  const [pluginParameters, setPluginParameters] = useState<PluginParametersResult | null>(null);
  const [pluginParameterError, setPluginParameterError] = useState<string | null>(null);
  const [presets, setPresets] = useState<import("@audiorouter/contracts").DiscoveryDocument["presets"] | null>(null);
  const [presetError, setPresetError] = useState<string | null>(null);
  const [theme, setTheme] = useState<ThemeMode>(() => readTheme(optionalLocalStorage()));
  const [language, setLanguage] = useState<string | null>(() => readLanguagePreference(optionalLocalStorage()));
  const changeLanguage = (preference: string | null) => {
    setLanguage(preference);
    writeLanguagePreference(optionalLocalStorage(), preference);
    void applyLanguage(preference);
  };
  const [customThemes, setCustomThemes] = useState(() => readCustomThemeLibrary(optionalLocalStorage()));
  const changeCustomThemes = (value: CustomThemeLibrary) => {
    setCustomThemes(value);
    writeCustomThemeLibrary(optionalLocalStorage(), value);
  };
  // "custom" with no saved custom theme (all deleted) shows the dark theme.
  const activeCustomTheme = theme === "custom" ? (selectedCustomTheme(customThemes)?.theme ?? null) : null;
  const themeClassName = activeCustomTheme
    ? `theme-${activeCustomTheme.base} theme-custom`
    : `theme-${theme === "custom" ? "dark" : theme}`;
  const themeStyle = activeCustomTheme ? customThemeStyle(activeCustomTheme) : undefined;
  const [flowAnimation, setFlowAnimation] = useState<FlowAnimationMode>(() =>
    readFlowAnimation(optionalLocalStorage()),
  );
  const changeFlowAnimation = (mode: FlowAnimationMode) => {
    setFlowAnimation(mode);
    writeFlowAnimation(optionalLocalStorage(), mode);
  };
  const [liveAutosave, setLiveAutosave] = useState(() => readLiveAutosave(optionalLocalStorage()));
  const changeLiveAutosave = (enabled: boolean) => {
    setLiveAutosave(enabled);
    writeLiveAutosave(optionalLocalStorage(), enabled);
  };
  const [sidebarWidth, setSidebarWidth] = useState(() => readSidebarWidth(optionalLocalStorage()));
  const changeSidebarWidth = (width: number) => {
    setSidebarWidth(width);
    writeSidebarWidth(optionalLocalStorage(), width);
  };
  const groupState = useCanvasGroups(selectedSessionId);
  const selectedGroup = groupState.groups.find((group) => group.id === groupState.selectedGroupId);
  const canvasLayoutActions = useRef<((action: LayoutAction) => void) | null>(null);
  const canvasLayoutHistory = useRef<CanvasLayoutHistoryControls | null>(null);
  const [layoutHistoryState, setLayoutHistoryState] = useState({ canUndo: false, canRedo: false });
  const editOrderRef = useRef(emptyEditOrder());
  const [canvasLayoutSelection, setCanvasLayoutSelection] = useState<{ sessionId: string; items: CanvasRect[] }>({
    sessionId: "",
    items: [],
  });
  const updateCanvasLayoutSelection = useCallback((sessionId: string, items: CanvasRect[]) => {
    setCanvasLayoutSelection((current) =>
      current.sessionId === sessionId && JSON.stringify(current.items) === JSON.stringify(items)
        ? current
        : { sessionId, items },
    );
  }, []);
  const [workbenchTab, setWorkbenchTab] = useState<WorkbenchTab>("tools");
  // "Set up for Discord" (D2), open for one session; each opening keys a
  // fresh form, and `added` (Discord's microphone) is step 2.
  const [discordSetup, setDiscordSetup] = useState<{
    key: number;
    sessionId: string;
    added: import("@audiorouter/contracts").DeviceListItem | null;
  } | null>(null);
  // The Apps tab and top-bar strip start after the first snapshot: the backend
  // serves one request at a time, and `apps.list` sent first held back the
  // start-up snapshot and so the first screen. The desktop backend now builds
  // the list on its own thread, but a backend without its service (CLI,
  // harness) still enumerates inline for 30-100 ms.
  const startupSnapshotLoaded = !backend.connected || snapshotState.snapshot !== null;
  const appChecksReady = backend.connected && startupSnapshotLoaded;
  // Which apps use which devices, for the Apps tab and the top-bar strip.
  const listApplicationUsage = useMemo(
    () => (appChecksReady ? () => backend.listApplications() : null),
    [backend, appChecksReady],
  );
  // Device nodes show which apps use their device (D1), so poll while the
  // canvas has one; the backend builds the list off its control thread.
  const draftHasDeviceNode = Boolean(
    draft?.nodes.some((node) => node.kind === "physicalInput" || node.kind === "physicalOutput"),
  );
  const applicationUsage = useApplicationUsage(listApplicationUsage, {
    polling: workbenchTab === "apps" || draftHasDeviceNode,
  });
  // OBS Studio facts (DEC-22): cheap reads of the backend's OBS thread.
  const obsStatus = useObsStatus(appChecksReady ? backend : null, { active: workbenchTab === "apps" });
  const obsActions = useMemo<ObsActions | null>(() => {
    const { obsConnect, obsDisconnect, obsPreview, obsApply, obsRestore } = backend;
    if (!backend.connected || !obsConnect || !obsDisconnect || !obsPreview || !obsApply || !obsRestore) return null;
    return {
      connect: (password) => obsConnect.call(backend, password),
      disconnect: () => obsDisconnect.call(backend),
      preview: (changes) => obsPreview.call(backend, changes),
      apply: (previewId) => obsApply.call(backend, previewId),
      restore: () => obsRestore.call(backend),
      poll: obsStatus.poll,
    };
  }, [backend, obsStatus.poll]);
  // The rows are worded when computed, so a language switch recomputes them.
  const discordAppCheck = useMemo(() => {
    void language;
    return applicationUsage.loaded
      ? discordCheck(draft, applicationUsage.applications, devices, applicationUsage.stale)
      : pendingDiscordCheck();
  }, [applicationUsage.loaded, applicationUsage.applications, applicationUsage.stale, draft, devices, language]);
  const deviceUsers = useMemo(() => {
    void language;
    return draft && applicationUsage.loaded ? nodeDeviceUsers(draft, devices, applicationUsage.applications) : null;
  }, [applicationUsage.loaded, applicationUsage.applications, draft, devices, language]);
  const obsAppCheck = useMemo(() => {
    void language;
    return obsCheck(draft, obsStatus.status, applicationUsage.applications, devices, obsStatus.stale);
  }, [draft, obsStatus.status, obsStatus.stale, applicationUsage.applications, devices, language]);
  // Shown from the session alone, never because an app started (UI-17):
  // Discord when the session captures it or has a voice cable, OBS when it
  // has a voice cable.
  const appStripChecks = useMemo(() => {
    if (!backend.connected) return [];
    const checks = sessionInvolvesDiscord(draft, devices) ? [discordAppCheck] : [];
    if (draft && voiceCableOutputs(draft, devices).length > 0) checks.push(obsAppCheck);
    return checks;
  }, [backend, draft, devices, discordAppCheck, obsAppCheck]);
  // UI-18: optional daily new-version check (GitHub public releases list).
  const [updateChecks, setUpdateChecks] = useState(updateCheckEnabled);
  const [updateChannel, setUpdateChannelState] = useState(readUpdateChannel);
  const updateState = useUpdateCheck(
    updateChecks,
    updateChannel,
    isAutomatedHarness() ? noNetworkUpdateFetcher : undefined,
  );
  const availableUpdate = updateState.update;
  const selectNodeProperties = (id: string) => {
    groupState.selectGroup("");
    setSelectedNodeId(id);
    setSelectedNodeIds([id]);
    setWorkbenchTab("properties");
  };
  const [setupTarget, setSetupTarget] = useState<{ sessionId: string; nodeId?: string } | null>(null);
  const [inputBindingFocusRequest, setInputBindingFocusRequest] = useState(0);
  const openLoadedSetupTarget = useEffectEvent(() => {
    if (!setupTarget || draft.id !== setupTarget.sessionId) return;
    setSetupTarget(null);
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectFinding"));
      return;
    }
    if (setupTarget.nodeId) {
      if (!draft.nodes.some((node) => node.id === setupTarget.nodeId)) {
        setActionMessage(say("g1.appFindingToolGone"));
        return;
      }
      selectNodeProperties(setupTarget.nodeId);
      setActionMessage(say("g1.appFindingOpened"));
      const needsInputSelector =
        draftRef.current.nodes.find((node) => node.id === setupTarget.nodeId)?.kind === "physicalInput";
      if (needsInputSelector) setInputBindingFocusRequest((current) => current + 1);
      if (needsInputSelector && snapshot?.status.activeSessionIds.includes(setupTarget.sessionId)) {
        setActionMessage(say("g1.appFindingOpenedStop"));
      }
    } else {
      setWorkbenchTab("session");
      setActionMessage(say("g1.appFindingRoute"));
    }
    const isPhysicalInput = setupTarget.nodeId
      ? draftRef.current.nodes.find((node) => node.id === setupTarget.nodeId)?.kind === "physicalInput"
      : false;
    if (!isPhysicalInput) {
      window.setTimeout(
        () => document.querySelector<HTMLElement>('.right-workbench [role="tab"][aria-selected="true"]')?.focus(),
        0,
      );
    }
  });
  useEffect(() => {
    openLoadedSetupTarget();
  }, [setupTarget, draft]);
  // Read once: the storage object never changes, which keeps recordUiDiagnostic stable.
  const [diagnosticStorage] = useState(browserDiagnosticStorage);
  const uiDiagnosticsRef = useRef<string[]>(readClientDiagnostics(diagnosticStorage));
  const [uiDiagnostics, setUiDiagnostics] = useState<string[]>(uiDiagnosticsRef.current);
  const [canvasKeyboardPanel, setCanvasKeyboardPanel] = useState<"routeList" | "connect" | null>(null);
  const [connectionSource, setConnectionSource] = useState("");
  const [connectionDestination, setConnectionDestination] = useState("");
  const [connectionDialogOpen, setConnectionDialogOpen] = useState(false);
  const [sessionSwitchPrompt, setSessionSwitchPrompt] = useState<{
    names: string[];
    decide: (confirmed: boolean) => void;
  } | null>(null);
  const [sidechainReplacePrompt, setSidechainReplacePrompt] = useState<{
    occupiedEdgeId: string;
    sourceNodeId: string;
    sourcePort: string;
    sourceName: string;
    destinationNodeId: string;
    destinationPort: string;
    pluginName: string;
    currentSourceName: string;
  } | null>(null);
  const connectionDialogReturnFocus = useRef<HTMLElement | null>(null);
  const connectionDialog = useRef<HTMLElement | null>(null);
  const connectionDialogSource = useRef<HTMLSelectElement | null>(null);
  const [pluginPickerOpen, setPluginPickerOpen] = useState(false);
  const [pluginInsertEdgeId, setPluginInsertEdgeId] = useState<string | null>(null);
  const pluginPickerReturnFocus = useRef<HTMLElement | null>(null);
  const pluginPickerDialog = useRef<HTMLElement | null>(null);
  const [applicationPickerOpen, setApplicationPickerOpen] = useState(false);
  const [applicationPickerSelection, setApplicationPickerSelection] = useState("");
  const applicationPickerReturnFocus = useRef<HTMLElement | null>(null);
  const applicationPickerDialog = useRef<HTMLElement | null>(null);
  const [createdSessions, setCreatedSessions] = useState<import("@audiorouter/contracts").Session[]>([]);
  const [listedSessions, setListedSessions] = useState<import("@audiorouter/contracts").Session[]>(
    backend.connected ? [] : demoSessions,
  );
  const [, setSessionInventoryError] = useState<string | null>(null);
  const [nativeGenerations, setNativeGenerations] = useState<
    Record<string, { generation: number; kind: string; preview?: boolean }>
  >({});
  const [nativePumpStats, setNativePumpStats] = useState<NativePumpStats | null>(null);
  const eventCursor = useRef({ backendEpoch: 0, sequence: 0 });
  const applicationRefreshGeneration = useRef(0);
  const applicationFollowUp = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(applicationFollowUp.current), []);
  const deviceRefreshGeneration = useRef(0);
  const sessionRefreshGeneration = useRef(0);
  const recordingRefreshGeneration = useRef(0);
  const recorderRefreshGeneration = useRef(0);
  // Startup decides which route opens; a route list arriving first must not pick one.
  const startupSettled = useRef(false);
  useEffect(() => {
    let mounted = true;
    let replacedRememberedSession = false;
    void (async () => {
      // With nothing remembered (first launch, cleared WebView data), open
      // the session the backend has active instead of the shell's default:
      // writing the default back would overwrite the saved choice, which
      // autoplay then plays.
      const backendActive =
        rememberedSessionId || !backend.getActiveSession
          ? null
          : await backend.getActiveSession().then(
              (result) => result.sessionId,
              () => null,
            );
      if (backendActive) backendActiveSessionRef.current = backendActive;
      const first = await snapshotCache.refresh(backend, rememberedSessionId ?? backendActive ?? undefined, {
        startup: true,
      });
      if (!first.error || !rememberedSessionId) return first;
      // The remembered route may be gone (removed by a script, or an older database
      // restored). Only the list can say so; any other failure keeps it and offers Reconnect.
      const listed = await backend.listSessions().catch(() => null);
      if (!listed || listed.some((item) => item.id === rememberedSessionId)) return first;
      const active = await backend.getActiveSession?.().then(
        (result) => result.sessionId,
        () => null,
      );
      const next = listed.some((item) => item.id === active) ? active! : listed[0]?.id;
      replacedRememberedSession = true;
      return snapshotCache.refresh(backend, next);
    })().then((nextState) => {
      if (mounted) {
        startupSettled.current = true;
        setSnapshotState(nextState);
        if (nextState.snapshot && replacedRememberedSession) {
          setSelectedSessionId(nextState.snapshot.session.id);
          setActionMessage(say("g1.appRouteGone", { name: nextState.snapshot.session.name }));
        } else if (nextState.snapshot && !rememberedSessionId)
          setSelectedSessionId((current) => (current === demoSession.id ? nextState.snapshot!.session.id : current));
        if (nextState.snapshot)
          eventCursor.current = {
            backendEpoch: nextState.snapshot.status.eventCursor.backendEpoch,
            sequence: nextState.snapshot.status.eventCursor.latestSequence,
          };
      }
    });
    return () => {
      mounted = false;
    };
  }, [backend, snapshotCache, rememberedSessionId]);
  const readApplications = () => {
    const generation = ++applicationRefreshGeneration.current;
    void backend
      .listApplications()
      .then((items) => {
        if (generation !== applicationRefreshGeneration.current) return;
        setApplications(items);
        setApplicationsError(null);
      })
      .catch((error) => {
        if (generation !== applicationRefreshGeneration.current) return;
        setApplications([]);
        setApplicationsError(formatUiError(error, "Application inventory unavailable"));
      });
  };
  // The backend answers with the list it has and rebuilds it off its control
  // thread; read again once the newer list is ready (useApplicationUsage).
  const refreshApplications = () => {
    readApplications();
    window.clearTimeout(applicationFollowUp.current);
    applicationFollowUp.current = window.setTimeout(readApplications, APP_USAGE_FOLLOW_UP_MS);
  };
  const refreshDevices = ({ announce = true }: { announce?: boolean } = {}) => {
    const generation = ++deviceRefreshGeneration.current;
    if (announce) setActionMessage(say("g1.appRefreshingEndpoints"));
    void backend
      .listDevices()
      .then((items) => {
        if (generation !== deviceRefreshGeneration.current) return;
        rememberDeviceNames(items);
        setDevices(items);
        setDevicesError(null);
        if (announce)
          setActionMessage(
            items.length === 0
              ? say("g1.appRefreshNone")
              : say(items.length === 1 ? "g1.appRefreshOne" : "g1.appRefreshMany", { count: items.length }),
          );
      })
      .catch((error) => {
        if (generation !== deviceRefreshGeneration.current) return;
        setDevices([]);
        const message = formatUiError(error, "Device inventory unavailable");
        setDevicesError(message);
        if (announce) setActionMessage(say("g1.appRefreshFailed", { message }));
      });
  };
  const refreshSessions = () => {
    const generation = ++sessionRefreshGeneration.current;
    return backend
      .listSessions()
      .then((items) => {
        if (generation !== sessionRefreshGeneration.current) return;
        setListedSessions(items);
        setSessionInventoryError(null);
        setSelectedSessionId((current) =>
          startupSettled.current ? keepSelectedSession(current, items, backend.connected) : current,
        );
      })
      .catch((error) => {
        if (generation !== sessionRefreshGeneration.current) return;
        setSessionInventoryError(formatUiError(error, "Session inventory unavailable"));
      });
  };
  const refreshRecordings = (sessionId: string) => {
    const generation = ++recordingRefreshGeneration.current;
    void backend
      .listRecordings(sessionId)
      .then((items) => {
        if (generation !== recordingRefreshGeneration.current) return;
        setRecordings(items);
        setRecordingsError(null);
      })
      .catch((error) => {
        if (generation !== recordingRefreshGeneration.current) return;
        setRecordings([]);
        setRecordingsError(formatUiError(error, "Recording library unavailable"));
      });
  };
  // One-click Record / Stop for a Recorder node (canvas button and Properties).
  const [recordingBusy, setRecordingBusy] = useState<string | null>(null);
  const [recordingMessage, setRecordingMessage] = useState<{ nodeId: string; text: string } | null>(null);
  const [lastRecordingPaths, setLastRecordingPaths] = useState<Record<string, string>>({});
  const toggleNodeRecording = async (nodeId: string, record: boolean) => {
    if (recordingBusy || !backend.connected) return;
    setRecordingBusy(nodeId);
    try {
      const result = record
        ? await backend.startNodeRecording(session.id, nodeId, uiIdempotencyKey("record-start"))
        : await backend.stopNodeRecording(session.id, nodeId, uiIdempotencyKey("record-stop"));
      const keptPath = typeof result.path === "string" ? result.path : result.paths?.at(-1);
      if (keptPath) setLastRecordingPaths((current) => ({ ...current, [nodeId]: keptPath }));
      if (!record && result.state === "failed") {
        // The take stopped saving early; the file is kept up to that point.
        const early = recordingStoppedEarly(result.reason);
        setRecordingMessage({ nodeId, text: early.text });
        setActionMessage(early);
      } else {
        setRecordingMessage(null);
        setActionMessage(record ? say("g1.appRecordingStarted") : say("g1.appRecordingSaved"));
      }
      refreshRecordings(session.id);
    } catch (error) {
      const text = formatUiError(error, record ? tm("g1.appRecordingStartFail") : tm("g1.appRecordingStopFail"));
      setRecordingMessage({ nodeId, text });
      setActionMessage(errorMessage(text));
    } finally {
      setRecordingBusy(null);
      refreshRecorders();
    }
  };
  const refreshRecorders = () => {
    const generation = ++recorderRefreshGeneration.current;
    void backend
      .listRecorders()
      .then((items) => {
        if (generation !== recorderRefreshGeneration.current) return;
        setRecorderStatuses(items);
        setRecorderStatusAvailable(true);
      })
      .catch(() => {
        if (generation !== recorderRefreshGeneration.current) return;
        setRecorderStatuses([]);
        setRecorderStatusAvailable(false);
      });
  };
  const refresh = async () => {
    const requestedSessionId = session.id;
    const nextState = await snapshotCache.refresh(backend, requestedSessionId);
    if (currentSessionIdRef.current !== requestedSessionId) return nextState;
    setSnapshotState(nextState);
    await refreshSessions();
    refreshRecordings(session.id);
    refreshRecorders();
    refreshApplications();
    refreshDevices({ announce: false });
    return nextState;
  };
  // Reconnect reloads everything from the backend; it reports progress and
  // the outcome in the message slot so the click is visibly acknowledged.
  const [reconnecting, setReconnecting] = useState(false);
  const [savedDataDamaged, setSavedDataDamaged] = useState(false);
  const reconnect = async () => {
    if (reconnecting) return;
    setReconnecting(true);
    setActionMessage(say("g1.appReconnecting"));
    try {
      const nextState = await refresh();
      if (!backend.connected) {
        setActionMessage(say("g1.appReconnectNotRunning"));
      } else if (nextState.error) {
        setActionMessage(say("g1.appReconnectFailedWith", { error: nextState.error }));
      } else {
        setSavedDataDamaged(false);
        setActionMessage(say("g1.appReconnected"));
      }
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appReconnectFailed")));
    } finally {
      setReconnecting(false);
    }
  };
  const snapshot = snapshotState.snapshot;
  const hasSnapshot = snapshot !== null;
  const telemetryStore = useTelemetryStore();
  const diagnosticsRef = useRef(snapshot?.diagnostics ?? null);
  diagnosticsRef.current = snapshot?.diagnostics ?? null;
  const recordUiDiagnostic = useCallback(
    (message: string) => {
      const next = appendClientDiagnostic(diagnosticStorage, uiDiagnosticsRef.current, message);
      uiDiagnosticsRef.current = next;
      setUiDiagnostics(next);
    },
    [diagnosticStorage],
  );
  useEffect(() => {
    writeTheme(optionalLocalStorage(), theme);
  }, [theme]);
  // Presentation only: the native shell's tray text follows the same choice.
  // A shell without the command (browser harness) simply has no effect.
  useEffect(() => {
    void backend.setUiLanguage?.(language).catch(() => undefined);
  }, [backend, language]);
  // AudioRouter cable status (spec 17 §7.1), published to the library,
  // canvas palette, cable properties and Setup. The backend caches it until
  // Windows reports an endpoint change (at most 30 s), so this poll is cheap.
  useEffect(() => {
    const getStatus = backend.getVirtualCableStatus?.bind(backend);
    if (!backend.connected || !getStatus) {
      setVirtualCableStatus(null);
      return;
    }
    let active = true;
    let statusFailed = false;
    const poll = () => {
      getStatus()
        .then((status) => {
          if (active) {
            statusFailed = false;
            setVirtualCableStatus(status);
          }
        })
        // Keep the last answer through a failed poll (UI-17); a backend
        // without the method never answers, so the status stays unknown.
        .catch(() => {
          if (active && !statusFailed) {
            statusFailed = true;
            recordUiDiagnostic("Virtual cable status read failed");
          }
        });
    };
    poll();
    const timer = window.setInterval(poll, 2000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [backend, backend.connected, recordUiDiagnostic]);
  useEffect(() => {
    const onError = (event: ErrorEvent) => {
      const row = describeWindowError(event);
      if (row && isNewUiErrorRow(row)) recordUiDiagnostic(row);
    };
    const onReject = (event: PromiseRejectionEvent) => {
      const row = describePromiseRejection(event.reason);
      if (isNewUiErrorRow(row)) recordUiDiagnostic(row);
    };
    window.addEventListener("error", onError);
    window.addEventListener("unhandledrejection", onReject);
    return () => {
      window.removeEventListener("error", onError);
      window.removeEventListener("unhandledrejection", onReject);
    };
  }, [recordUiDiagnostic]);
  // The Logs tab's verbose switch; stable per backend so the panel reads it once.
  const verboseLogging = useMemo(
    () =>
      backend.getVerboseDiagnostics && backend.setVerboseDiagnostics
        ? {
            get: () => backend.getVerboseDiagnostics!(),
            set: (enabled: boolean) => backend.setVerboseDiagnostics!(enabled),
          }
        : undefined,
    [backend],
  );
  // Failed backend requests, with the correlation ID that finds them in shell.jsonl and backend.jsonl (P2-3).
  useEffect(() => onRpcFailure(recordUiDiagnostic), [recordUiDiagnostic]);
  const { mcpActivity, mcpClients, mcpSetupInfo, mcpTunnelStatus, backendActivity } = useWorkbenchStatus(
    workbenchTab,
    recordUiDiagnostic,
    backend,
  );
  // Follow the backend's mute state when it changes. Diagnostics refresh
  // every 50 ms while playing and carry the current value; `status` is only
  // refreshed on a full snapshot, so reacting to every new snapshot object
  // reverted a live toggle to the stale status value.
  const backendPrivacyMuted = snapshot
    ? (snapshot.diagnostics?.privacyMute?.muted ?? snapshot.status.privacyMute.muted)
    : undefined;
  useEffect(() => {
    if (backendPrivacyMuted !== undefined) setPrivacyMuted(backendPrivacyMuted);
  }, [backendPrivacyMuted]);
  const savedSnapshotSession = snapshot?.session ?? null;
  const availableSessions = useMemo(
    () => mergeSessionInventory(listedSessions, savedSnapshotSession, createdSessions),
    [listedSessions, savedSnapshotSession, createdSessions],
  );
  const rememberedInputBindingKey = JSON.stringify(
    availableSessions.flatMap((saved) =>
      saved.nodes
        .filter((node) => node.kind === "physicalInput" && typeof node.parameters.endpointId === "string")
        .map((node) => [saved.id, node.id, node.parameters.endpointId]),
    ),
  );
  const rememberCurrentSessionDeviceNames = useEffectEvent(() => {
    if (devices.length > 0 && availableSessions.length > 0) {
      rememberSessionDeviceNames(availableSessions, devices);
    }
  });
  useEffect(() => {
    rememberCurrentSessionDeviceNames();
  }, [devices, rememberedInputBindingKey]);
  // A fresh install holds only the untouched starter session, or nothing yet.
  const looksLikeFirstRun =
    backend.connected &&
    snapshot !== null &&
    listedSessions.every((item) => item.id === "desktop-session" && item.revision === 0);
  useEffect(() => {
    if (looksLikeFirstRun && readFirstRunDone() === false) setGoalDialogOpen(true);
  }, [looksLikeFirstRun]);
  useEffect(() => {
    const onPaletteKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onPaletteKey);
    return () => window.removeEventListener("keydown", onPaletteKey);
  }, []);
  const checkSetupCall = useMemo(() => backend.checkSetup?.bind(backend), [backend]);
  const setupCheckKey = setupCheckRefreshKey(devices, availableSessions, snapshot?.status.activeSessionIds ?? []);
  const setupCheck = useSetupCheck({
    connected: backend.connected,
    ready: appChecksReady,
    checkSetup: checkSetupCall,
    refreshKey: setupCheckKey,
  });
  const session =
    availableSessions.find((item) => item.id === selectedSessionId) ?? availableSessions[0] ?? demoSession;
  const sessionIsAvailable = availableSessions.some((item) => item.id === session.id);
  useEffect(() => {
    if (!backend.connected || !backend.setActiveSession || !sessionIsAvailable) return;
    // Until the first snapshot picks the session, the selection is the demo
    // placeholder or a remembered route not yet confirmed, and `session` falls back
    // to the first listed one; writing that would overwrite the backend's saved
    // active session.
    if (selectedSessionId === demoSession.id || selectedSessionId !== session.id) return;
    // Each write emits `session.selectionChanged`, which this window also
    // follows; writing a value the backend already has started a loop that
    // flipped between two sessions at startup.
    if (backendActiveSessionRef.current === session.id) return;
    activeSessionWritesRef.current += 1;
    void backend
      .setActiveSession(session.id)
      .then(
        (result) => {
          backendActiveSessionRef.current = result.sessionId;
        },
        (error) => setSessionInventoryError(formatUiError(error, "Active session selection unavailable")),
      )
      .finally(() => {
        activeSessionWritesRef.current -= 1;
      });
  }, [backend, session.id, sessionIsAvailable, selectedSessionId]);
  // Another client (MCP, Stream Deck) changed the active session. While this
  // window's own write is in flight, a read can return the value it is
  // replacing; adopting that reverted the user's choice. The write's own
  // event brings the final value once it lands.
  const followBackendActiveSession = (sessionId: string) => {
    if (activeSessionWritesRef.current > 0) return false;
    backendActiveSessionRef.current = sessionId;
    if (sessionId !== draftRef.current.id && !sameSessionDraft(draftRef.current, authoritativeSession.current)) {
      setActionMessage(say("g1.appOtherAppSession"));
      return false;
    }
    setSelectedSessionId(sessionId);
    return true;
  };
  const selectedSessionListed = availableSessions.some((item) => item.id === selectedSessionId);
  useEffect(() => {
    if (selectedSessionListed) writeLastSession(diagnosticStorage, selectedSessionId);
  }, [selectedSessionId, selectedSessionListed, diagnosticStorage]);
  currentSessionIdRef.current = session.id;
  const selectedNode = draft.nodes.find((node) => node.id === selectedNodeId) ?? (draft.nodes[0] as Node | undefined);
  const attemptedDeviceRebinds = useRef(new Set<string>());
  useEffect(() => {
    recordUiDiagnostic(
      `Graph checkpoint: ${draft.nodes.length} nodes, ${draft.edges.length} connections; revision ${session.revision}`,
    );
  }, [draft.nodes.length, draft.edges.length, session.revision, recordUiDiagnostic]);
  const sessionRunning = snapshot?.status.activeSessionIds.includes(session.id) ?? false;
  const routeChanged = !sameSessionDraft(draft, session);
  const reconcileNamedInputs = useEffectEvent(async () => {
    if (!backend.connected || !setupCheck.result) return;
    if (!setupCheck.result.findings.some((finding) => finding.code === "deviceMissing" && finding.nodeId)) return;
    if (sessionBusy.current || sessionActionBusy || graphBusy) return;
    let currentDevices: import("@audiorouter/contracts").DeviceListItem[];
    try {
      currentDevices = await backend.listDevices(true);
      rememberDeviceNames(currentDevices);
    } catch {
      return;
    }
    for (const finding of setupCheck.result?.findings ?? []) {
      if (finding.code !== "deviceMissing" || !finding.nodeId) continue;
      const saved = availableSessions.find((item) => item.id === finding.sessionId);
      if (!saved || snapshot?.status.activeSessionIds.includes(saved.id)) continue;
      if (saved.id === draft.id && routeChanged) continue;
      let latest: UiBackendSnapshot;
      try {
        latest = await backend.snapshot(saved.id);
      } catch {
        continue;
      }
      if (
        latest.status.activeSessionIds.includes(saved.id) ||
        latest.session.revision !== saved.revision ||
        !sameSessionDraft(latest.session, saved)
      ) {
        continue;
      }
      const node = latest.session.nodes.find((item) => item.id === finding.nodeId);
      if (
        !node ||
        node.kind !== "physicalInput" ||
        typeof node.parameters.endpointId !== "string" ||
        (finding.deviceId && node.parameters.endpointId !== finding.deviceId)
      ) {
        continue;
      }
      const savedEndpointName =
        captureEndpointName(currentDevices, node.parameters.endpointId) ??
        finding.suggestedEndpointName ??
        rememberedDeviceName(String(node.parameters.endpointId));
      if (!savedEndpointName) continue;
      // The Setup Check result is only a hint and may be stale by the time the
      // graph commit is planned. Re-resolve against this fresh inventory and
      // require exactly one active capture match before proceeding.
      const candidate = findReplacementCaptureEndpoint(currentDevices, node.parameters.endpointId, savedEndpointName);
      if (!candidate || (finding.suggestedEndpointId && candidate.id !== finding.suggestedEndpointId)) continue;
      const attemptKey = `${saved.id}/${saved.revision}/${node.id}/${candidate.id}`;
      if (attemptedDeviceRebinds.current.has(attemptKey)) continue;
      attemptedDeviceRebinds.current.add(attemptKey);
      const updated: import("@audiorouter/contracts").Session = {
        ...latest.session,
        nodes: latest.session.nodes.map((item) =>
          item.id === node.id ? { ...item, parameters: { ...item.parameters, endpointId: candidate.id } } : item,
        ),
      };
      try {
        const plan = await backend.planGraph(updated);
        if (plan.baseRevision !== saved.revision || plan.warnings.length > 0) {
          attemptedDeviceRebinds.current.delete(attemptKey);
          continue;
        }
        const beforeCommit = await backend.snapshot(saved.id);
        const liveDiagnostics = await backend.refreshDiagnostics();
        if (
          sessionBusy.current ||
          graphBusy ||
          beforeCommit.status.activeSessionIds.includes(saved.id) ||
          (liveDiagnostics.nativeSessionId === saved.id && liveDiagnostics.nativeAdapter === "running") ||
          beforeCommit.session.revision !== plan.baseRevision ||
          (saved.id === draftRef.current.id && !sameSessionDraft(draftRef.current, authoritativeSession.current))
        ) {
          attemptedDeviceRebinds.current.delete(attemptKey);
          continue;
        }
        const result = await backend.commitGraph(
          plan.planId,
          plan.baseRevision,
          uiIdempotencyKey("device-name-rebind"),
          undefined,
          saved.id,
        );
        rememberDeviceNameBinding(String(node.parameters.endpointId), savedEndpointName, "capture");
        setActionMessage(say("g1.appInputUpdated", { node: node.name, candidate: candidate.name }));
        await refreshSessions();
        if (currentSessionIdRef.current === saved.id) await refresh();
        recordUiDiagnostic(`Exact-name input binding updated for ${node.id}; revision ${result.revision}`);
        return;
      } catch (error) {
        if (error instanceof Error && error.message.includes("session to be stopped")) {
          attemptedDeviceRebinds.current.delete(attemptKey);
        }
        if (currentSessionIdRef.current === saved.id) {
          setActionMessage(uiError(error, tm("g1.appInputSaveFail", { node: node.name })));
        }
      }
    }
  });
  useEffect(() => {
    void reconcileNamedInputs();
  }, [
    backend.connected,
    setupCheck.result,
    devices,
    availableSessions,
    draft.id,
    routeChanged,
    graphBusy,
    sessionActionBusy,
  ]);
  const routeMicMuted =
    sessionRunning &&
    privacyMuted &&
    draft.nodes.some((node) => node.enabled && !node.bypass && PRIVACY_MUTED_KINDS.includes(node.kind));
  const unfedNodes = unfedRouteNodes(draft);
  const routeWarningKey = JSON.stringify([draft.id, unfedNodes.map((node) => node.id).sort()]);
  const [routeWarningHeld, setRouteWarningHeld] = useState(false);
  const routeWarningShown = unfedNodes.length > 0 && dismissedRouteWarning !== routeWarningKey;
  useEffect(() => {
    // A warning closes itself after 30 s like other warnings (owner, 2026-11-07);
    // a different set of unfed cards brings it back.
    if (!routeWarningShown || routeWarningHeld) return;
    const timer = window.setTimeout(() => setDismissedRouteWarning(routeWarningKey), WARNING_LIFETIME_MS);
    return () => window.clearTimeout(timer);
  }, [routeWarningShown, routeWarningHeld, routeWarningKey]);
  // Lets the shell free the WebView on close only when nothing would be lost.
  useReportUnsaved(routeChanged, recordUiDiagnostic);
  useEffect(() => {
    setAudioSourceStates({});
    audioSourceActivity.current.clear();
  }, [session.id]);
  const refreshApplicationsOnChange = useEffectEvent(() => refreshApplications());
  useEffect(() => {
    if (selectedNode?.kind === "applicationCapture" && backend.connected) refreshApplicationsOnChange();
  }, [selectedNode?.id, selectedNode?.kind, backend.connected]);
  useAudioFileStatus({
    backend,
    draft,
    session,
    sessionRunning,
    audioSourceStates,
    transportActivity: audioSourceActivity,
    setAudioSourceStates,
    recordUiDiagnostic,
  });
  const testSignalPlaybackReady = backendReady && sameSessionDraft(draft, session);
  const testSignalEndpointPrepared =
    snapshot?.diagnostics.nativeSessionId === session.id &&
    snapshot.diagnostics.nativeAdapter === "configured-stopped" &&
    snapshot.diagnostics.nativeAdapterKind === "endpoint";
  useEffect(() => {
    const hint = readEndpointBindingHint(session.id);
    setCaptureEndpointId(hint.captureEndpointId ?? "");
    setRenderEndpointId(hint.renderEndpointId ?? "");
  }, [session.id]);
  // Read when the session changes: before the first snapshot or session list,
  // `session` is only the demo placeholder.
  const waitingForFirstSession = useEffectEvent(() => backend.connected && !snapshot && availableSessions.length === 0);
  useEffect(() => {
    if (waitingForFirstSession()) return;
    const previous = authoritativeSession.current;
    const transition = hasAuthoritativeSession.current
      ? reconcileSessionDraft(draftRef.current, previous, session)
      : "adopt";
    hasAuthoritativeSession.current = true;
    if (transition === "unchanged") return;
    authoritativeSession.current = session;
    setPendingWarnings([]);
    setAcknowledgedWarnings(new Set());
    setPendingOperation(null);
    setPendingGraphPlan(null);
    if (transition === "conflict") {
      setDraftHistory({ past: [], future: [] });
      recordUiDiagnostic(
        `Graph refresh conflict: kept draft revision ${draftRef.current.revision}; backend revision ${session.revision}`,
      );
      setActionMessage(say("g1.appChangedElsewhere"));
      return;
    }
    setDraft(session);
    setDraftHistory({ past: [], future: [] });
    editGroup.current = null;
    setSelectedNodeId((current) =>
      session.nodes.some((node) => node.id === current) ? current : (session.nodes[0]?.id ?? ""),
    );
    setSelectedNodeIdsState((current) => {
      const retained = current.filter((id) => session.nodes.some((node) => node.id === id));
      return retained.length ? retained : session.nodes[0] ? [session.nodes[0].id] : [];
    });
    setConnectionSource("");
    setConnectionDestination("");
  }, [session, recordUiDiagnostic]);
  // After the adoption above, so a restored copy is not replaced by the saved route.
  const [olderEdits, setOlderEdits] = useState<typeof session | null>(null);
  const draftRecovery = useDraftRecovery({
    ready: backend.connected && snapshot !== null && sessionIsAvailable && selectedSessionId === session.id,
    draft,
    session,
    unsaved: routeChanged,
    knownSessionIds: availableSessions.map((item) => item.id),
    onRestore: (restored) => {
      // Undo must return to the saved route, and the ref can still hold the startup placeholder
      // when this runs in the same commit that adopted the route.
      draftRef.current = session;
      recordDraftChange(restored);
      setActionMessage(say("g1.appEditsRestored"));
    },
    onOlder: (edits) => {
      setOlderEdits(edits);
      setActionMessage(say("g1.appOlderEdits"));
    },
    onOtherRoute: (sessionId) => {
      const name = availableSessions.find((item) => item.id === sessionId)?.name ?? "another route";
      setActionMessage(say("g1.appUnsavedOn", { name }));
    },
    onFailure: recordUiDiagnostic,
  });
  const notificationsRef = useNotificationsClearTabs(`${actionMessage?.text ?? null}|${routeWarningKey}`);
  const discardRouteEdits = () => {
    setDraft(session);
    setDraftHistory({ past: [], future: [] });
    setPendingWarnings([]);
    setAcknowledgedWarnings(new Set());
    setPendingOperation(null);
    setPendingGraphPlan(null);
    setActionMessage(say("g1.appEditsDiscarded"));
  };
  useEffect(() => {
    let active = true;
    void backend
      .listPresets()
      .then((items) => {
        if (active) {
          setPresets(items);
          setPresetError(null);
        }
      })
      .catch((error) => {
        if (active) {
          setPresets(null);
          setPresetError(formatUiError(error, "Preset catalog unavailable"));
        }
      });
    return () => {
      active = false;
    };
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend
      .listProcessors()
      .then((items) => {
        if (active) {
          setProcessors(items);
          setProcessorError(null);
        }
      })
      .catch((error) => {
        if (active) {
          setProcessors(null);
          setProcessorError(formatUiError(error, "Processor catalog unavailable"));
        }
      });
    return () => {
      active = false;
    };
  }, [backend]);
  useEffect(() => {
    if (selectedNode?.kind !== "plugin" || !backend.connected) {
      setPluginParameters(null);
      setPluginParameterError(null);
      return;
    }
    const path = selectedNode.parameters.path;
    const fingerprint = selectedNode.parameters.fingerprint;
    if (typeof path !== "string" || typeof fingerprint !== "string") {
      setPluginParameters(null);
      setPluginParameterError(tm("g1.appPluginNoPath"));
      return;
    }
    let active = true;
    setPluginParameters(null);
    setPluginParameterError(null);
    void backend
      .describePluginParameters(
        path,
        typeof selectedNode.parameters.classId === "string" ? selectedNode.parameters.classId : undefined,
      )
      .then((result) => {
        if (!active) return;
        if (result.sha256 !== fingerprint) {
          setPluginParameterError(tm("g1.appPluginChanged"));
          return;
        }
        setPluginParameters(result);
      })
      .catch((error) => {
        if (active) setPluginParameterError(formatUiError(error, tm("g1.appPluginParams")));
      });
    return () => {
      active = false;
    };
  }, [
    backend,
    selectedNode?.id,
    selectedNode?.kind,
    selectedNode?.parameters.path,
    selectedNode?.parameters.fingerprint,
    selectedNode?.parameters.classId,
  ]);
  useEffect(() => {
    let active = true;
    void backend
      .listSessions()
      .then((items) => {
        if (active) {
          setListedSessions(items);
          setSessionInventoryError(null);
          setSelectedSessionId((current) =>
            startupSettled.current ? keepSelectedSession(current, items, backend.connected) : current,
          );
        }
      })
      .catch((error) => {
        if (active) {
          setSessionInventoryError(formatUiError(error, "Session inventory unavailable"));
          setListedSessions(backend.connected ? [] : demoSessions);
        }
      });
    return () => {
      active = false;
    };
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend
      .listRecordings(session.id)
      .then((items) => {
        if (active) {
          setRecordings(items);
          setRecordingsError(null);
        }
      })
      .catch((error) => {
        if (active) {
          setRecordings([]);
          setRecordingsError(formatUiError(error, "Recording library unavailable"));
        }
      });
    return () => {
      active = false;
    };
  }, [backend, session.id]);
  useEffect(() => {
    let active = true;
    void backend
      .listRecorders()
      .then((items) => {
        if (active) {
          setRecorderStatuses(items);
          setRecorderStatusAvailable(true);
        }
      })
      .catch(() => {
        if (active) {
          setRecorderStatuses([]);
          setRecorderStatusAvailable(false);
        }
      });
    return () => {
      active = false;
    };
  }, [backend]);
  // After the first snapshot, like the Apps strip: the picker does not need it sooner.
  useEffect(() => {
    if (!startupSnapshotLoaded) return;
    let active = true;
    void backend
      .listApplications()
      .then((items) => {
        if (active) {
          setApplications(items);
          setApplicationsError(null);
        }
      })
      .catch((error) => {
        if (active) {
          setApplications([]);
          setApplicationsError(formatUiError(error, "Application inventory unavailable"));
        }
      });
    return () => {
      active = false;
    };
  }, [backend, startupSnapshotLoaded]);
  useEffect(() => {
    let active = true;
    void backend
      .listDevices()
      .then((items) => {
        if (active) {
          rememberDeviceNames(items);
          setDevices(items);
          setDevicesError(null);
        }
      })
      .catch((error) => {
        if (active) {
          setDevices([]);
          setDevicesError(formatUiError(error, "Device inventory unavailable"));
        }
      });
    return () => {
      active = false;
    };
  }, [backend]);
  useWorkspaceEvents({
    backend,
    hasSnapshot,
    session,
    snapshotCache,
    eventCursor,
    setSnapshotState,
    followBackendActiveSession,
    setActionMessage,
    refreshApplications,
    refreshDevices,
    refreshSessions,
    setRecordings,
    setRecordingsError,
    setRecorderStatuses,
    setRecorderStatusAvailable,
    reconnecting,
  });
  useDiagnosticsRefresh({
    backend,
    sessionRunning,
    telemetryStore,
    diagnosticsRef,
    setSnapshotState,
    recordUiDiagnostic,
  });
  useNativeCounters({ backend, nativeGenerations, snapshot, session, setNativePumpStats, recordUiDiagnostic });
  const outputPorts = draft.nodes.flatMap((node) =>
    node.ports
      .filter((port) => port.direction === "output")
      .map((port) => ({ nodeId: node.id, nodeName: node.name, portName: port.name, channels: port.channels })),
  );
  const inputPorts = draft.nodes.flatMap((node) =>
    node.ports
      .filter((port) => port.direction === "input")
      .map((port) => ({ nodeId: node.id, nodeName: node.name, portName: port.name, channels: port.channels })),
  );
  const encodePort = (nodeId: string, portName: string) => `${nodeId}::${portName}`;
  const decodePort = (value: string) => {
    const separator = value.indexOf("::");
    return separator < 0 ? null : { nodeId: value.slice(0, separator), portName: value.slice(separator + 2) };
  };
  const visibleRecordings = recordings.filter((recording) => {
    const query = recordingSearch.trim().toLocaleLowerCase();
    if (!query) return true;
    return [recording.id, recording.title, recording.artist, recording.comment, recording.path]
      .filter((value): value is string => value !== null)
      .some((value) => value.toLocaleLowerCase().includes(query));
  });
  const cableStatus = useVirtualCableStatus();
  const visibleLibraryEntries = filterLibraryEntries(
    applyCableAvailability(libraryEntries, cableStatus),
    librarySearch,
  );
  const setupSteps = setupChecklist({
    connected: backend.connected,
    audio: snapshot?.status.audio ?? null,
    activeSessionCount: snapshot?.status.activeSessionCount ?? null,
    audioReason: backendText(snapshot?.status.reason) ?? null,
    storage: snapshot?.status.storage ?? null,
    deviceCount: devices.length,
    applicationCount: applications.length,
    vbCablePairAvailable: findVbCableEndpointPair(devices) !== null,
  });
  const connectionLabel = backend.connected ? tm("g1.appBackendReady") : tm("g1.appBackendUnavailable");
  const nativePumpSummary = formatNativePumpSummary(nativePumpStats, sessionRunning);
  const schedulerTelemetry = snapshot?.diagnostics.schedulerTelemetry;
  const schedulerSummary = schedulerTelemetry
    ? ` - ${tm("g1.appSchedulerSummary", { quanta: schedulerTelemetry.processedQuanta, xruns: schedulerTelemetry.xruns })}`
    : "";
  const statusSummary = `${
    snapshot
      ? tm("g1.appStatusCore", {
          audio: snapshot.status.audio,
          reason: backendText(snapshot.status.reason),
          storage: snapshot.status.storage,
          sessions: tm(snapshot.status.sessionCount === 1 ? "g1.appSessionsOne" : "g1.appSessionsMany", {
            count: snapshot.status.sessionCount,
          }),
        })
      : tm("g1.appStatusWaiting")
  }${schedulerSummary}${nativePumpSummary ? ` - ${nativePumpSummary}` : ""}${sessionCrudBusyState ? ` - ${tm("g1.appUpdatingSession")}` : ""}`;
  const editGroup = useRef<{ key: string; time: number } | null>(null);
  const recordDraftChange = (next: import("@audiorouter/contracts").Session, group?: string) => {
    const previous = draftRef.current;
    if (sameSessionDraft(previous, next)) return;
    const now = Date.now();
    const coalesce = group && editGroup.current?.key === group && now - editGroup.current.time < 750;
    editGroup.current = group ? { key: group, time: now } : null;
    if (!coalesce) editOrderRef.current = recordEdit(editOrderRef.current, "route");
    draftRef.current = next;
    setDraftHistory((history) =>
      coalesce && history.past.length > 0 ? { ...history, future: [] } : recordDraft(history, previous, next),
    );
    setDraft(next);
    setPendingWarnings([]);
    setAcknowledgedWarnings(new Set());
    setPendingOperation(null);
    setPendingGraphPlan(null);
  };
  const restoreDraftHistory = (direction: "undo" | "redo", steps = 1) => {
    if (!backend.connected || graphBusy || sessionActionBusy) return;
    editGroup.current = null;
    const current = draftRef.current;
    let transition: HistoryTransition = { current, history: draftHistory };
    for (let step = 0; step < steps; step += 1) {
      transition =
        direction === "undo"
          ? undoDraftHistory(transition.history, transition.current)
          : redoDraftHistory(transition.history, transition.current);
    }
    if (transition.current === current) return;
    // History restores content; all subsequent commits use today's revision.
    const restored = { ...transition.current, revision: current.revision };
    editOrderRef.current = recordRestore(editOrderRef.current, "route", direction);
    setDraftHistory(transition.history);
    draftRef.current = restored;
    setDraft(restored);
    setPendingWarnings([]);
    setAcknowledgedWarnings(new Set());
    setPendingOperation(null);
    setPendingGraphPlan(null);
    setActionMessage(direction === "undo" ? say("g1.appUndid") : say("g1.appRedid"));
  };
  const undoDraft = () => restoreDraftHistory("undo");
  const undoDraftSteps = (steps: number) => restoreDraftHistory("undo", steps);
  const redoDraft = () => restoreDraftHistory("redo");
  const routeHistoryReady = backend.connected && !graphBusy && !sessionActionBusy;
  const canRestoreEdit = {
    undo: (routeHistoryReady && draftHistory.past.length > 0) || layoutHistoryState.canUndo,
    redo: (routeHistoryReady && draftHistory.future.length > 0) || layoutHistoryState.canRedo,
  };
  /** The window's Undo and Redo, and their shortcuts: the newest change of either kind goes first. */
  const restoreNewestEdit = (direction: "undo" | "redo") => {
    const kind = nextEditKind(editOrderRef.current, direction, {
      route: routeHistoryReady && (direction === "undo" ? draftHistory.past : draftHistory.future).length > 0,
      layout: direction === "undo" ? layoutHistoryState.canUndo : layoutHistoryState.canRedo,
    });
    if (kind === "route") restoreDraftHistory(direction);
    else if (kind === "layout") canvasLayoutHistory.current?.restore(direction);
  };
  const onHistoryShortcut = useEffectEvent(restoreNewestEdit);
  useEffect(() => {
    const onHistoryKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || !(event.ctrlKey || event.metaKey)) return;
      const key = event.key.toLowerCase();
      if (key !== "z" && key !== "y") return;
      const target = event.target instanceof Element ? event.target : null;
      if (target?.closest("[role='dialog']")) return;
      const numeric = target?.closest(
        "[role='spinbutton'], input[type='range'], input[type='number'], input[type='checkbox'], select",
      );
      if (!numeric && isEditableShortcutTarget(event.target)) return;
      event.preventDefault();
      if (numeric instanceof HTMLElement) numeric.blur();
      onHistoryShortcut(key === "y" || event.shiftKey ? "redo" : "undo");
    };
    window.addEventListener("keydown", onHistoryKey);
    return () => window.removeEventListener("keydown", onHistoryKey);
  }, []);
  const changeNodeFlag = async (flag: "enabled" | "bypass", value: boolean) => {
    if (!selectedNode || sessionBusy.current || nodeFlagBusy.current || graphBusy || !backendReady) return;
    const next = setNodeDraftFlag(draftRef.current, selectedNode.id, flag, value);
    const wasRunning = sessionRunning;
    if (!wasRunning) {
      recordDraftChange(next);
      setActionMessage(say("g1.appToolToggled"));
      return;
    }
    if (!session.nodes.some((node) => node.id === selectedNode.id)) {
      setActionMessage(say("g1.appNewToolNotPlaying"));
      return;
    }
    // Commit only this live flag, keeping unsaved names, parameters and wiring
    // in the local draft rather than requiring (or silently performing) Save.
    const submitted = setNodeDraftFlag(session, selectedNode.id, flag, value);
    nodeFlagBusy.current = true;
    setGraphBusy(true);
    try {
      const result = await applyGraphDraft(backend, submitted, uiIdempotencyKey("node-state"));
      recordDraftChange(setNodeDraftFlag(draftRef.current, selectedNode.id, flag, value));
      await finishGraphSave(submitted, result.revision, result.activation);
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appChangeNotApplied")));
    } finally {
      nodeFlagBusy.current = false;
      setGraphBusy(false);
    }
  };
  const changeNodeName = (name: string) => {
    if (!selectedNode) return;
    try {
      recordDraftChange(setNodeDraftName(draft, selectedNode.id, name));
      setActionMessage(say("g1.appNameChanged"));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appRenameToolFail")));
    }
  };
  const renameCanvasNode = async (nodeId: string) => {
    if (!backend.connected || document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    const current = draftRef.current;
    const node = current.nodes.find((item) => item.id === nodeId);
    if (!node) return;
    const name = await askText({
      title: tm("g1.appRenameTitle"),
      label: tm("g1.appToolName"),
      initialValue: node.name,
      confirmLabel: tm("g1.appRename"),
      maxLength: 120,
    });
    if (!name || draftRef.current.id !== current.id) return;
    try {
      recordDraftChange(setNodeDraftName(draftRef.current, nodeId, name));
      setActionMessage(say("g1.appNameChanged"));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appRenameToolFail")));
    }
  };
  const changeNodeParameterOn = (nodeId: string, name: string, value: boolean | number | string) => {
    if (staleSnapshot) return;
    const targetNode = draftRef.current.nodes.find((candidate) => candidate.id === nodeId);
    if (!targetNode) return;
    const error =
      targetNode.kind === "soundboard"
        ? !isSoundboardParameter(name) || (typeof value === "string" && value.length > 255)
          ? tm("g1.appInvalidSoundboard")
          : null
        : targetNode.kind === "audioFile"
          ? !(name === "mediaId" && typeof value === "string" && value.length <= 128) &&
            !(name === "fileName" && typeof value === "string" && value.length <= 255) &&
            !(name === "loop" && typeof value === "boolean")
            ? tm("g1.appInvalidAudioSource")
            : null
          : targetNode.kind === "plugin"
            ? pluginNodeParameterError(name, value, pluginParameters, () =>
                processorParameterError(processors, "plugin", name, value, snapshot?.discovery?.nodeTypes ?? null),
              )
            : processorParameterError(processors, targetNode.kind, name, value, snapshot?.discovery?.nodeTypes ?? null);
    if (error) {
      setActionMessage(say("g1.appChangeRejected", { error }));
      return;
    }
    recordDraftChange(setNodeDraftParameter(draftRef.current, nodeId, name, value), nodeId + ":" + name);
    // No notification per edit: a slider drag would announce and redraw one on every step, and
    // the enabled Save button and the node's "Save required" mark already say it needs saving.
    setActionMessage(null);
  };
  const changeNodeParameter = (name: string, value: boolean | number | string) => {
    if (selectedNode) changeNodeParameterOn(selectedNode.id, name, value);
  };
  const resetNodeParameters = () => {
    if (!selectedNode) return;
    recordDraftChange(resetNodeDraftParameters(draft, selectedNode.id));
    setActionMessage(say("g1.appSettingsReset"));
  };
  const changeSessionName = (name: string) => {
    try {
      recordDraftChange(setSessionDraftName(draft, name));
      setActionMessage(say("g1.appSessionRenamed"));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appSessionRenameFail")));
    }
  };
  const finishGraphSave = async (
    submitted: import("@audiorouter/contracts").Session,
    revision: number,
    activation?: import("@audiorouter/contracts").GraphCommitResult["activation"],
  ) => {
    setPendingWarnings([]);
    setAcknowledgedWarnings(new Set());
    setPendingOperation(null);
    setPendingGraphPlan(null);
    const saved = { ...submitted, revision };
    setCreatedSessions((current) => [...current.filter((item) => item.id !== saved.id), saved]);
    if (currentSessionIdRef.current === saved.id) {
      authoritativeSession.current = saved;
      setDraft((current) => (current.id === saved.id ? { ...current, revision } : current));
      editGroup.current = null;
      const native = activation && activation.state === "running" ? activation.native : null;
      // A change applied to the playing audio starts a new runtime generation;
      // keep servicing the route with it (the old one is rejected as stale).
      if ((native?.state === "applied" || native?.state === "restarted") && activation?.state === "running") {
        const nextGeneration = activation.generation;
        setNativeGenerations((current) =>
          current[saved.id]
            ? { ...current, [saved.id]: { ...current[saved.id], generation: nextGeneration, preview: false } }
            : current,
        );
      }
      setActionMessage(
        native?.state === "applied"
          ? say("g1.appSavedApplied", { revision })
          : native?.state === "restarted"
            ? say("g1.appSavedRestarted", { revision })
            : native?.state === "restartRequired"
              ? say("g1.appSavedNotApplied", { revision, reason: native.reason })
              : say("g1.appRouteSaved", { revision }),
      );
    }
    recordUiDiagnostic(
      `Graph commit succeeded: revision ${revision}; ${submitted.nodes.length} nodes, ${submitted.edges.length} connections`,
    );
    await refresh();
  };
  const lastAutoSaveDraft = useRef<string | null>(null);
  const autoSaveRunning = useRef(false);
  const planChanges = async (automatic = false) => {
    if (graphBusy || !backendReady) return;
    if (automatic && autoSaveRunning.current) return;
    if (automatic) {
      autoSaveRunning.current = true;
      lastAutoSaveDraft.current = JSON.stringify(draft);
    }
    setGraphBusy(true);
    if (!automatic) setActionMessage(say("g1.appCheckingRoute"));
    try {
      const operation = uiIdempotencyKey("graph-commit");
      const plan = await backend.planGraph(draft);
      if (currentSessionIdRef.current !== draft.id || !sameSessionDraft(draftRef.current, draft)) {
        // An automatic save can race the user's next edit. Stay quiet and let
        // the effect save the latest draft instead of asking for a review.
        if (!automatic) setActionMessage(say("g1.appEditsChangedInReview"));
        return;
      }
      if (plan.baseRevision !== draft.revision)
        throw new Error("Backend returned a plan for a different session revision");
      if (plan.warnings.length === 0) {
        const result = await backend.commitGraph(plan.planId, plan.baseRevision, operation);
        await finishGraphSave(draft, result.revision, result.activation);
      } else {
        setPendingOperation(operation);
        setPendingGraphPlan({ planId: plan.planId, baseRevision: plan.baseRevision });
        setPendingWarnings(plan.warnings);
        setAcknowledgedWarnings(new Set());
        setActionMessage(say("g1.appReviewWarnings"));
      }
    } catch (error) {
      if (isRevisionConflict(error)) {
        setPendingWarnings([]);
        setAcknowledgedWarnings(new Set());
        setPendingOperation(null);
        setPendingGraphPlan(null);
        void snapshotCache.refresh(backend, session.id).then((nextState) => {
          setSnapshotState(nextState);
          void refreshSessions();
        });
        setActionMessage(uiError(error, tm("g1.appGraphElsewhere")));
      } else setActionMessage(uiError(error, tm("g1.appGraphApplyFail")));
    } finally {
      setGraphBusy(false);
      if (automatic) autoSaveRunning.current = false;
    }
  };
  // While audio plays, slider-style edits (parameter values only) are saved
  // after a short pause so the change is heard without Stop/Play. Topology
  // edits still wait for an explicit Save.
  // Edits made while an automatic save runs are picked up by the effect below
  // once it finishes, so they coalesce into one newer save.
  const autoSavePending =
    liveAutosave &&
    sessionRunning &&
    backendReady &&
    !pendingGraphPlan &&
    isParameterOnlyChange(session, draft) &&
    (graphBusy || lastAutoSaveDraft.current !== JSON.stringify(draft));
  const planChangesRef = useRef(planChanges);
  planChangesRef.current = planChanges;
  useEffect(() => {
    if (
      !liveAutosave ||
      !sessionRunning ||
      graphBusy ||
      !backendReady ||
      pendingGraphPlan ||
      !isParameterOnlyChange(session, draft)
    )
      return;
    const key = JSON.stringify(draft);
    if (lastAutoSaveDraft.current === key) return;
    const timer = window.setTimeout(() => {
      void planChangesRef.current(true);
    }, 400);
    return () => window.clearTimeout(timer);
  }, [draft, session, sessionRunning, graphBusy, backendReady, pendingGraphPlan, liveAutosave]);
  const commitAcknowledgedPlan = async () => {
    if (
      graphBusy ||
      !backendReady ||
      !pendingOperation ||
      !pendingGraphPlan ||
      acknowledgedWarnings.size !== pendingWarnings.length
    )
      return;
    setGraphBusy(true);
    setActionMessage(say("g1.appSavingRoute"));
    const submitted = draft;
    try {
      const result =
        pendingWarnings.length === 0
          ? await backend.commitGraph(pendingGraphPlan.planId, pendingGraphPlan.baseRevision, pendingOperation)
          : await applyGraphDraft(backend, draft, pendingOperation, [...acknowledgedWarnings]);
      await finishGraphSave(submitted, result.revision);
    } catch (error) {
      if (isRevisionConflict(error)) {
        setPendingWarnings([]);
        setAcknowledgedWarnings(new Set());
        setPendingOperation(null);
        setPendingGraphPlan(null);
        void snapshotCache.refresh(backend, session.id).then((nextState) => {
          setSnapshotState(nextState);
          void refreshSessions();
        });
        setActionMessage(uiError(error, tm("g1.appGraphElsewhere")));
      } else setActionMessage(uiError(error, tm("g1.appGraphCommitFail")));
    } finally {
      setGraphBusy(false);
    }
  };
  const saveDisabled =
    !backendReady ||
    graphBusy ||
    (pendingGraphPlan ? acknowledgedWarnings.size !== pendingWarnings.length : !routeChanged);
  const saveChanges = () => {
    if (!saveDisabled) void (pendingGraphPlan ? commitAcknowledgedPlan() : planChanges());
  };
  const onSaveShortcut = useEffectEvent((event: KeyboardEvent) => {
    if (
      event.defaultPrevented ||
      event.repeat ||
      isEditableShortcutTarget(event.target) ||
      document.querySelector('[role="dialog"][aria-modal="true"]')
    )
      return;
    if (
      !event.ctrlKey &&
      !event.metaKey &&
      !event.altKey &&
      ((event.key === "F1" && !event.shiftKey) || event.key === "?")
    ) {
      event.preventDefault();
      setKeyboardHelpOpen(true);
      return;
    }
    if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "s") {
      event.preventDefault();
      saveChanges();
    }
  });
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => onSaveShortcut(event);
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
  const togglePrivacyMute = async () => {
    if (safetyActionBusy.current || !backend.connected) return;
    safetyActionBusy.current = true;
    setSafetyActionBusyState(true);
    const next = !privacyMuted;
    setActionMessage(next ? say("g1.appMuteEnabling") : say("g1.appMuteDisabling"));
    try {
      await backend.setPrivacyMute(next, uiIdempotencyKey("privacy-mute"));
      setPrivacyMuted(next);
      setActionMessage(next ? say("g1.appMuteEnabled") : say("g1.appMuteDisabled"));
    } catch (error) {
      setPrivacyMuted(true);
      setActionMessage(uiError(error, tm("g1.appMuteFail")));
    } finally {
      safetyActionBusy.current = false;
      setSafetyActionBusyState(false);
    }
  };
  const clearRecoverySafeMode = async () => {
    if (safetyActionBusy.current || !backend.connected) return;
    safetyActionBusy.current = true;
    setSafetyActionBusyState(true);
    setActionMessage(say("g1.appSafeClearing"));
    try {
      await backend.clearRecoverySafeMode(uiIdempotencyKey("recovery-clear"));
      await refresh();
      setActionMessage(say("g1.appSafeCleared"));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appSafeFail")));
    } finally {
      safetyActionBusy.current = false;
      setSafetyActionBusyState(false);
    }
  };
  const createSession = async () => {
    if (sessionCrudBusy.current || !backend.connected) return;
    const name = await askText({
      title: m("workflowActions.newSession"),
      label: m("workflowUi.sessionName"),
      initialValue: m("workflowActions.newSession"),
      confirmLabel: m("workflowActions.createSession"),
    });
    if (!name || sessionCrudBusy.current || !backend.connected) return;
    sessionCrudBusy.current = true;
    setSessionCrudBusyState(true);
    const id = `session-${Date.now()}`;
    try {
      const result = await backend.createSession(
        {
          ...demoSession,
          id,
          name,
          revision: 0,
          nodes: demoSession.nodes.map((node) => ({ ...node, parameters: { ...node.parameters } })),
          edges: [...demoSession.edges],
        },
        uiIdempotencyKey("session-create"),
      );
      setCreatedSessions((current) => [...current, result.session]);
      const switched = await switchSession(result.session.id);
      if (switched.switched)
        setActionMessage(
          say(switched.stopped ? "workflowActions.sessionCreatedStopped" : "workflowActions.sessionCreated", {
            name: result.session.name,
          }),
        );
    } catch (error) {
      setActionMessage(uiError(error, m("workflowActions.sessionCreateError")));
    } finally {
      sessionCrudBusy.current = false;
      setSessionCrudBusyState(false);
    }
  };
  const duplicateSession = async () => {
    if (sessionCrudBusy.current || !backend.connected) return;
    sessionCrudBusy.current = true;
    setSessionCrudBusyState(true);
    const id = `session-copy-${Date.now()}`;
    const name = tm("g1.appCopyName", { name: session.name });
    try {
      const result = await backend.duplicateSession(session.id, id, name, uiIdempotencyKey("session-duplicate"));
      setCreatedSessions((current) => [...current, result.session]);
      const switched = await switchSession(result.session.id);
      if (switched.switched)
        setActionMessage(
          say(switched.stopped ? "workflowActions.sessionDuplicatedStopped" : "workflowActions.sessionDuplicated", {
            name: result.session.name,
          }),
        );
    } catch (error) {
      setActionMessage(uiError(error, m("workflowActions.sessionDuplicateError")));
    } finally {
      sessionCrudBusy.current = false;
      setSessionCrudBusyState(false);
    }
  };
  const deleteSession = async () => {
    if (sessionCrudBusy.current || !backend.connected) return;
    const confirmed = await askConfirm({
      title: m("workflowActions.sessionDeleteTitle", { name: session.name }),
      message: m("workflowActions.sessionDeleteHelp"),
      confirmLabel: m("workflowUi.deleteSession"),
      destructive: true,
    });
    if (!confirmed || sessionCrudBusy.current || !backend.connected) return;
    sessionCrudBusy.current = true;
    setSessionCrudBusyState(true);
    try {
      await backend.deleteSession(session.id, uiIdempotencyKey("session-delete"));
      setCreatedSessions((current) => current.filter((item) => item.id !== session.id));
      // Ask the backend which sessions remain: right after start-up the cached
      // list can still be empty, which left the window on the deleted session.
      let remaining = availableSessions;
      try {
        const items = await backend.listSessions();
        ++sessionRefreshGeneration.current;
        setListedSessions(items);
        remaining = items;
      } catch {
        // Keep the cached list; the next refresh reports the inventory error.
        recordUiDiagnostic("Session inventory refresh after deletion failed");
      }
      const fallback = remaining.find((item) => item.id !== session.id);
      if (fallback) await switchSession(fallback.id, true);
      setActionMessage(say("workflowActions.sessionDeleted", { name: session.name }));
    } catch (error) {
      setActionMessage(uiError(error, m("workflowActions.sessionDeleteError")));
    } finally {
      sessionCrudBusy.current = false;
      setSessionCrudBusyState(false);
    }
  };
  // First Play on a fresh install: ask once before opening audio devices.
  const [deviceConsent, setDeviceConsent] = useState<{ busy: boolean; error: string | null } | null>(null);
  const allowDeviceAccessAndPlay = async () => {
    if (!backend.setDeviceAccess) return;
    setDeviceConsent({ busy: true, error: null });
    try {
      await backend.setDeviceAccess(true, uiIdempotencyKey("device-access"));
      announceDeviceAccessChanged();
      setDeviceConsent(null);
      void startSessionRef.current();
    } catch (error) {
      setDeviceConsent({ busy: false, error: formatUiError(error, tm("g1.appChoiceNotSaved")) });
    }
  };
  // Native audio stays attached to the session that prepared it, even after
  // that session stops, and the backend refuses another session's Play until
  // it is released. Release a stopped adapter of any other session.
  const releaseOtherSessionAudio = async (
    diagnostics: Awaited<ReturnType<typeof backend.refreshDiagnostics>>,
    keepSessionId: string,
  ) => {
    const other = diagnostics.nativeSessionId;
    if (!other || other === keepSessionId || diagnostics.nativeAdapter === "running") return diagnostics;
    if (diagnostics.nativeAdapterKind === "duplex") {
      if (!backend.detachNativeDuplex) return diagnostics;
      await backend.detachNativeDuplex(other);
    } else if (diagnostics.nativeAdapterKind !== "render-source") {
      if (!backend.detachNativeEndpoint) return diagnostics;
      await backend.detachNativeEndpoint(other);
    } else {
      return diagnostics;
    }
    recordUiDiagnostic("Released the stopped audio adapter of another session");
    return backend.refreshDiagnostics();
  };
  // Nodes the last Play refused (`nodeNotReady`), with the node's parameters
  // at that moment: the mark stays until the node is changed or Play works.
  const [nodeIssues, setNodeIssues] = useState<{
    sessionId: string;
    issues: Record<string, { message: string; parameters: string }>;
  } | null>(null);
  const visibleNodeIssues: Record<string, string> = {};
  if (nodeIssues?.sessionId === draft.id)
    for (const [nodeId, issue] of Object.entries(nodeIssues.issues)) {
      const node = draft.nodes.find((candidate) => candidate.id === nodeId);
      if (node && JSON.stringify(node.parameters) === issue.parameters) visibleNodeIssues[nodeId] = issue.message;
    }
  const firstIssueNode = draft.nodes.find((node) => Object.hasOwn(visibleNodeIssues, node.id));
  // Messages offering a button stay until they change; the same conditions
  // render those buttons below.
  const actionMessageHasButton =
    !!actionMessage &&
    ((offersAction(actionMessage, "goToIssueNode") && !!firstIssueNode) ||
      (offersAction(actionMessage, "openStatus") && workbenchTab !== "status") ||
      (offersAction(actionMessage, "openSession") && workbenchTab !== "session") ||
      (offersAction(actionMessage, "discardEdits") && routeChanged) ||
      (offersAction(actionMessage, "olderEdits") && !!olderEdits));
  const [actionMessageHeld, setActionMessageHeld] = useState(false);
  const actionMessageLifetime = actionMessageLifetimeMs(actionMessage, actionMessageHasButton);
  // The text, not the object, restarts the timer: showing the same message again keeps its clock.
  const actionMessageText = actionMessage?.text ?? null;
  useEffect(() => {
    // Paused while the pointer or focus is on the message, so it can be read.
    if (actionMessageLifetime === null || actionMessageHeld) return;
    const timer = window.setTimeout(() => setActionMessage(null), actionMessageLifetime);
    return () => window.clearTimeout(timer);
  }, [actionMessageText, actionMessageLifetime, actionMessageHeld]);
  // Mark the nodes that stopped Play before it reached the backend (a device
  // not chosen yet); the mark clears when the node changes.
  const flagNodes = (nodes: Node[], message: string) =>
    setNodeIssues({
      sessionId: draftRef.current.id,
      issues: Object.fromEntries(
        nodes.map((node) => [node.id, { message, parameters: JSON.stringify(node.parameters) }]),
      ),
    });
  const startSession = async (): Promise<boolean> => {
    if (sessionBusy.current || !backendReady) return false;
    if (graphBusy) {
      setActionMessage(say("g1.appWaitRouteCheck"));
      return false;
    }
    sessionBusy.current = true;
    setSessionActionBusy(true);
    setActionMessage(say("g1.appCheckingEndpoints"));
    setNativePumpStats(null);
    try {
      let diagnostics = await backend.refreshDiagnostics();
      if (diagnostics.nativeSessionId && diagnostics.nativeSessionId !== session.id) {
        if (diagnostics.nativeAdapter === "running") {
          const other = availableSessions.find((item) => item.id === diagnostics.nativeSessionId)?.name;
          setActionMessage(other ? say("g1.appOtherPlayingNamed", { name: other }) : say("g1.appOtherPlaying"));
          return false;
        }
        setActionMessage(say("g1.appReleasingDevices"));
        diagnostics = await releaseOtherSessionAudio(diagnostics, session.id);
      }
      // Several independent paths (or several devices on one path) run as one
      // session on the multi-path worker, which reads each device from its
      // node. Every path starts and stops together.
      // A Test Signal or Audio File route needs no input device. When no input
      // was ever chosen, or the saved route has its output chosen on the node,
      // it plays on the multi-path worker so no microphone is opened. An
      // unsaved edit with a chosen input keeps the temporary preview below.
      const generatedOnly = generatedOnlyRoute(draftRef.current);
      const multiPathRoute =
        needsNativePaths(draftRef.current) ||
        (generatedOnly &&
          (!routeEndpointBinding(draftRef.current, session.id).captureEndpointId ||
            (sameSessionDraft(draftRef.current, session) && unboundDeviceNodes(draftRef.current).length === 0)));
      if (multiPathRoute) {
        if (generatedOnly && unboundDeviceNodes(draftRef.current).length > 0) {
          flagNodes(unboundDeviceNodes(draftRef.current), tm("g1.appChooseDevice"));
          setActionMessage(say("g1.appNoAudioPickOutput"));
          return false;
        }
        if (!sameSessionDraft(draftRef.current, session)) {
          setActionMessage(generatedOnly ? say("g1.appNoAudioSaveGenerated") : say("g1.appNoAudioSaveMulti"));
          return false;
        }
        const unbound = unboundDeviceNodes(session);
        if (unbound.length > 0) {
          flagNodes(unbound, tm("g1.appChooseDevice"));
          setActionMessage(say("g1.appNoAudioPickDevices", { names: unbound.map((node) => node.name).join(", ") }));
          return false;
        }
        if (!backend.prepareNativePaths) {
          setActionMessage(say("g1.appNoAudioNoMulti"));
          return false;
        }
        if (diagnostics.nativeSessionId === session.id && backend.detachNativeEndpoint)
          await backend.detachNativeEndpoint(session.id);
        setActionMessage(say("g1.appPreparingPaths"));
        await backend.prepareNativePaths(session.id);
        diagnostics = await backend.refreshDiagnostics();
      }
      const applicationSource = multiPathRoute ? null : applicationOnlyRouteSource(draftRef.current);
      const mixerSources = applicationSource || multiPathRoute ? null : mixerRouteSources(draftRef.current);
      const mixerApplicationRoute =
        mixerSources !== null &&
        mixerSources.length >= 2 &&
        mixerSources.some((node) => node.kind === "applicationCapture");
      if (mixerApplicationRoute && mixerSources) {
        // Applications mixed with a microphone run on the native multi-input
        // Mixer, which keeps each application reconnecting after a restart.
        // Test Signal and Audio File are generated inside the Mixer input;
        // endpoint loopback and virtual sources are not supported here yet.
        const unsupported = mixerSources.filter(
          (node) =>
            !["physicalInput", "applicationCapture", "testSignal", "audioFile", "soundboard"].includes(node.kind),
        );
        const microphones = mixerSources.filter((node) => node.kind === "physicalInput");
        if (unsupported.length > 0) {
          setActionMessage(
            say("g1.appNoAudioMixerUnsupported", { names: unsupported.map((node) => node.name).join(", ") }),
          );
          return false;
        }
        if (microphones.length > 1) {
          setActionMessage(say("g1.appNoAudioOneInput"));
          return false;
        }
        if (!sameSessionDraft(draftRef.current, session)) {
          setActionMessage(say("g1.appNoAudioSaveApp"));
          return false;
        }
        const endpoints = routeEndpointBinding(draftRef.current, session.id);
        if (!endpoints.renderEndpointId || (microphones.length === 1 && !endpoints.captureEndpointId)) {
          flagNodes(unboundDeviceNodes(draftRef.current), tm("g1.appChooseDevice"));
          setActionMessage(
            !endpoints.renderEndpointId ? say("g1.appNoAudioPickOutput") : say("g1.appNoAudioPickInput"),
          );
          return false;
        }
        const unbound = mixerSources.find(
          (node) =>
            node.kind === "applicationCapture" &&
            (typeof node.parameters.processId !== "number" ||
              typeof node.parameters.creationTime100ns !== "string" ||
              typeof node.parameters.executable !== "string"),
        );
        if (unbound) {
          setActionMessage(say("g1.appNoAudioPickApp", { name: unbound.name }));
          return false;
        }
        if (!backend.prepareNativeMultiInputs || !backend.prepareNativeOutputs) {
          setActionMessage(say("g1.appNoAudioNoMixerApp"));
          return false;
        }
        if (!(
          diagnostics.nativeSessionId === session.id &&
          diagnostics.nativeAdapterKind === "multi-input" &&
          ["configured-stopped", "running"].includes(diagnostics.nativeAdapter)
        )) {
          if (diagnostics.nativeSessionId === session.id && backend.detachNativeEndpoint)
            await backend.detachNativeEndpoint(session.id);
          const bindings = mixerSources.map((node): import("@audiorouter/contracts").NativeMultiInputSourceBinding =>
            node.kind === "testSignal" || node.kind === "audioFile" || node.kind === "soundboard"
              ? { kind: "generated" }
              : node.kind === "physicalInput"
                ? { kind: "physical", endpointId: endpoints.captureEndpointId ?? "" }
                : {
                    kind: "application",
                    processId: Number(node.parameters.processId),
                    executable: String(node.parameters.executable),
                    executablePath:
                      typeof node.parameters.executablePath === "string" ? node.parameters.executablePath : null,
                    creationTime100ns: String(node.parameters.creationTime100ns),
                    mode: "include",
                  },
          );
          setActionMessage(say("g1.appPreparingMixer", { count: bindings.length }));
          await backend.prepareNativeMultiInputs(session.id, undefined, bindings);
          await backend.prepareNativeOutputs(session.id, undefined, [endpoints.renderEndpointId]);
          diagnostics = await backend.refreshDiagnostics();
        }
      }
      const mixedApplicationSource =
        applicationSource || mixerApplicationRoute || multiPathRoute
          ? null
          : (draftRef.current.nodes.find((node) => node.kind === "applicationCapture" && node.enabled) ?? null);
      if (mixedApplicationSource) {
        const executable = String(mixedApplicationSource.parameters.executable ?? "an application");
        const others = mixedApplicationRouteOtherSources(draftRef.current, mixedApplicationSource.id)
          .map((node) => node.name)
          .join(", ");
        setActionMessage(
          others ? say("g1.appNoAudioOnlySource", { executable, others }) : say("g1.appNoAudioPickExe", { executable }),
        );
        recordUiDiagnostic("Session start blocked: application capture mixed with other sources");
        return false;
      }
      if (applicationSource) {
        // An application-only route runs on the process-loopback worker, which
        // also reconnects the app after it restarts. It binds the saved node.
        if (!sameSessionDraft(draftRef.current, session)) {
          setActionMessage(say("g1.appNoAudioSaveApp"));
          return false;
        }
        const outputEndpointId = routeEndpointBinding(draftRef.current, session.id).renderEndpointId;
        if (!outputEndpointId) {
          flagNodes(
            draftRef.current.nodes.filter((node) => node.enabled && node.kind === "physicalOutput"),
            tm("g1.appChooseDevice"),
          );
          setActionMessage(say("g1.appNoAudioPickOutput"));
          return false;
        }
        if (!backend.prepareNativeApplication) {
          setActionMessage(say("g1.appNoAudioNoAppCapture"));
          return false;
        }
        if (!(
          diagnostics.nativeSessionId === session.id &&
          diagnostics.nativeAdapterKind === "process-loopback" &&
          ["configured-stopped", "running"].includes(diagnostics.nativeAdapter)
        )) {
          if (diagnostics.nativeSessionId === session.id && backend.detachNativeEndpoint)
            await backend.detachNativeEndpoint(session.id);
          const executable = String(applicationSource.parameters.executable);
          setActionMessage(say("g1.appPreparingCapture", { executable }));
          await backend.prepareNativeApplication({
            sessionId: session.id,
            processId: Number(applicationSource.parameters.processId),
            executable,
            executablePath:
              typeof applicationSource.parameters.executablePath === "string"
                ? applicationSource.parameters.executablePath
                : null,
            creationTime100ns: String(applicationSource.parameters.creationTime100ns),
            mode: "include",
            renderEndpointId: outputEndpointId,
          });
          diagnostics = await backend.refreshDiagnostics();
        }
      } else if (
        !mixerApplicationRoute &&
        !multiPathRoute &&
        (diagnostics.nativeSessionId !== session.id ||
          !["configured-stopped", "running"].includes(diagnostics.nativeAdapter))
      ) {
        const binding = routeEndpointBinding(draftRef.current, session.id);
        if (!binding.renderEndpointId || !binding.captureEndpointId) {
          flagNodes(
            draftRef.current.nodes.filter(
              (node) =>
                node.enabled &&
                typeof node.parameters.endpointId !== "string" &&
                node.kind === (binding.renderEndpointId ? "physicalInput" : "physicalOutput"),
            ),
            tm("g1.appChooseDevice"),
          );
          setActionMessage(!binding.renderEndpointId ? say("g1.appNoAudioPickOutput") : say("g1.appNoAudioNeedInput"));
          recordUiDiagnostic("Session start blocked: exact endpoint selection is incomplete");
          return false;
        }
        if (!backend.prepareNativeEndpoint) {
          setActionMessage(say("g1.appNoAudioNoDevices"));
          return false;
        }
        setActionMessage(say("g1.appPreparingDevices"));
        await backend.prepareNativeEndpoint(session.id, binding.captureEndpointId, binding.renderEndpointId);
        diagnostics = await backend.refreshDiagnostics();
      }
      if (
        !multiPathRoute &&
        !applicationSource &&
        !mixerApplicationRoute &&
        diagnostics.nativeSessionId === session.id &&
        diagnostics.nativeAdapter === "configured-stopped" &&
        diagnostics.nativeAdapterKind === "endpoint"
      ) {
        // A stopped WASAPI client may still reference the endpoints selected
        // before a device restart or a changed UI binding. Reopen only the
        // exact selected pair before activation; never substitute a default.
        const binding = routeEndpointBinding(draftRef.current, session.id);
        if (binding.captureEndpointId && binding.renderEndpointId && backend.rebindNativeEndpoint) {
          setActionMessage(say("g1.appRefreshingDevices"));
          await backend.rebindNativeEndpoint(session.id, binding.captureEndpointId, binding.renderEndpointId);
        }
      }
      // Prepare the exact visible draft for this run. The backend validates it
      // and publishes it only to the in-memory native graph; no graph revision
      // is committed by preview playback.
      if (currentSessionIdRef.current !== session.id || graphBusy) {
        setActionMessage(say("g1.appRouteChangedChecking"));
        return false;
      }
      const candidate = sameSessionDraft(draftRef.current, session) ? undefined : draftRef.current;
      if (candidate) {
        setActionMessage(say("g1.appCheckingPreview"));
        const plan = await backend.planGraph(candidate);
        if (currentSessionIdRef.current !== session.id || !sameSessionDraft(draftRef.current, candidate)) {
          setActionMessage(say("g1.appRouteChangedPreview"));
          return false;
        }
        if (plan.warnings.length > 0) {
          setPendingGraphPlan({ planId: plan.planId, baseRevision: plan.baseRevision });
          setPendingWarnings(plan.warnings);
          setAcknowledgedWarnings(new Set());
          setActionMessage(say("g1.appNoAudioWarnings"));
          return false;
        }
      }
      setActionMessage(candidate ? say("g1.appStartingPreview") : say("g1.appStartingSession"));
      const idempotencyKey = uiIdempotencyKey("session-start");
      const result = candidate
        ? await backend.startSession(session.id, idempotencyKey, candidate)
        : await backend.startSession(session.id, idempotencyKey);
      if (result.runtime !== "native") {
        await backend.stopSession(session.id, uiIdempotencyKey("simulation-stop"));
        await refresh();
        setActionMessage(say("g1.appNoAudioSimulated"));
        recordUiDiagnostic("Session start rejected simulated runtime; stopped it");
        return false;
      }
      setNativeGenerations((current) => ({
        ...current,
        [session.id]: {
          generation: result.generation,
          preview: result.preview ?? Boolean(candidate),
          kind: diagnostics.nativeSessionId === session.id ? (diagnostics.nativeAdapterKind ?? "endpoint") : "endpoint",
        },
      }));
      await refresh();
      setActionMessage(
        candidate
          ? say("g1.appPreviewRunning", { generation: result.generation })
          : say("g1.appSessionRunning", { generation: result.generation }),
      );
      recordUiDiagnostic(`Native session started: generation ${result.generation}`);
      setNodeIssues(null);
      return true;
    } catch (error) {
      setNativePumpStats(null);
      if (isDeviceAccessDenied(error) && backend.setDeviceAccess) {
        setDeviceConsent({ busy: false, error: null });
        setActionMessage(say("g1.appNoAudioAllow"));
        return false;
      }
      const message = formatUiError(error, tm("g1.appStartFail"));
      setActionMessage(errorMessage(message));
      const refused = notReadyNode(error);
      const refusedNode = refused ? draftRef.current.nodes.find((node) => node.id === refused.nodeId) : undefined;
      if (refused && refusedNode) {
        setNodeIssues({
          sessionId: draftRef.current.id,
          issues: {
            [refused.nodeId]: { message: refused.message, parameters: JSON.stringify(refusedNode.parameters) },
          },
        });
        // Show which tool to fix: select it and open its Properties.
        selectNodeProperties(refused.nodeId);
      }
      recordUiDiagnostic("Session start failed; review the current error and backend activity.");
      return false;
    } finally {
      sessionBusy.current = false;
      setSessionActionBusy(false);
    }
  };
  const [timeShiftStatuses, setTimeShiftStatuses] = useState<
    Record<string, import("@audiorouter/contracts").MethodResult["timeShift.transport"]>
  >({});
  const transportTimeShift = async (
    nodeId: string,
    action: import("@audiorouter/contracts").MethodParams["timeShift.transport"]["action"],
  ) => {
    if (!backend.connected || !backend.transportTimeShift) return;
    try {
      const status = await backend.transportTimeShift(session.id, nodeId, action);
      if (currentSessionIdRef.current !== session.id) return;
      setTimeShiftStatuses((current) => ({ ...current, [nodeId]: status }));
    } catch (error) {
      if (action !== "status") setActionMessage(uiError(error, tm("g1.appTimeShiftFail")));
    }
  };
  const transportTimeShiftRef = useRef(transportTimeShift);
  transportTimeShiftRef.current = transportTimeShift;
  const timeShiftNodeIds = draft.nodes
    .filter((node) => node.kind === "timeShift" && node.enabled)
    .map((node) => node.id)
    .join(",");
  useEffect(() => {
    if (!sessionRunning || !timeShiftNodeIds) {
      setTimeShiftStatuses({});
      return;
    }
    const poll = () => {
      for (const nodeId of timeShiftNodeIds.split(",")) void transportTimeShiftRef.current(nodeId, "status");
    };
    poll();
    const timer = window.setInterval(poll, 1000);
    return () => window.clearInterval(timer);
  }, [sessionRunning, timeShiftNodeIds]);
  const transportAudioSource = async (nodeId: string, action: "play" | "pause" | "stop" | "status") => {
    if (!backend.connected || (staleSnapshot && action === "play")) return;
    const key = JSON.stringify([session.id, nodeId]);
    const activity = audioSourceActivity.current.get(key) ?? { version: 0, pending: 0 };
    activity.version += 1;
    activity.pending += 1;
    audioSourceActivity.current.set(key, activity);
    const requestVersion = activity.version;
    try {
      if (action === "play" && !sessionRunning && !(await startSession())) return;
      if (currentSessionIdRef.current !== session.id) return;
      const result = await backend.transportAudioSource(session.id, nodeId, action);
      if (currentSessionIdRef.current !== session.id) return;
      if (activity.version !== requestVersion) return;
      setAudioSourceStates((current) => ({ ...current, [nodeId]: result.state }));
      const transport = say("g1.appAudioFileState", {
        name: draft.nodes.find((node) => node.id === nodeId)?.name ?? tm("g1.appAudioFile"),
        state: result.state,
      });
      // "stopped" read as a success under the old wording rules; keep its green check.
      setActionMessage(result.state === "stopped" ? withTone(transport, "success") : transport);
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appTransportFail")));
    } finally {
      activity.pending -= 1;
    }
  };
  const startSessionRef = useRef(startSession);
  startSessionRef.current = startSession;
  const soundboard = useSoundboardController({
    backend: editorBackend,
    session,
    sessionRunning,
    startSession,
    report: recordUiDiagnostic,
    // One draft change from the latest draft, so an import that finishes after other
    // edits never undoes them.
    assign: (nodeId, slot, sound, keepCaption) => {
      try {
        recordDraftChange(
          assignSoundboardSlotDraft(draftRef.current, nodeId, slot, sound, keepCaption),
          `${nodeId}:slot${slot}`,
        );
      } catch (error) {
        setActionMessage(uiError(error, tm("g1.appSoundButtonFail")));
      }
    },
    announce: setActionMessage,
  });
  const stopSession = async (propagateFailure = false) => {
    if (sessionBusy.current || !backend.connected) return;
    sessionBusy.current = true;
    setSessionActionBusy(true);
    setActionMessage(say("g1.appStopping"));
    try {
      await backend.stopSession(session.id, uiIdempotencyKey("session-stop"));
      setNativeGenerations((current) => {
        const next = { ...current };
        delete next[session.id];
        return next;
      });
      setNativePumpStats(null);
      setAudioSourceStates({});
      await refresh();
      setActionMessage(say("g1.appStopped"));
    } catch (error) {
      setNativePumpStats(null);
      setActionMessage(uiError(error, tm("g1.appStopFail")));
      if (propagateFailure) throw error;
    } finally {
      sessionBusy.current = false;
      setSessionActionBusy(false);
    }
  };
  // Switching sessions stops whatever is playing and releases its audio
  // devices first; the newly selected session then starts only on Play.
  const switchSession = async (
    id: string,
    deletedCurrentSession = false,
  ): Promise<{ switched: boolean; stopped: boolean }> => {
    if (id === session.id) return { switched: true, stopped: false };
    if (routeChanged && !deletedCurrentSession) {
      setActionMessage(say("g1.appSwitchSaveFirst"));
      return { switched: false, stopped: false };
    }
    if (sessionBusy.current) {
      setActionMessage(say("g1.appSwitchWait"));
      return { switched: false, stopped: false };
    }
    let running: string[] = [];
    const stoppedNames: string[] = [];
    const sessionName = (sessionId: string) =>
      availableSessions.find((item) => item.id === sessionId)?.name ?? sessionId;
    sessionBusy.current = true;
    setSessionActionBusy(true);
    try {
      if (backend.connected) {
        // Running sessions are backend-wide. Read them through the target:
        // after a deletion the current session no longer exists.
        running = (await backend.snapshot(id)).status.activeSessionIds;
        while (running.length > 0) {
          const confirmed = await new Promise<boolean>((resolve) =>
            setSessionSwitchPrompt({
              names: running.map(sessionName),
              decide: (value) => {
                setSessionSwitchPrompt(null);
                resolve(value);
              },
            }),
          );
          if (!confirmed) return { switched: false, stopped: false };
          const fresh = (await backend.snapshot(id)).status.activeSessionIds;
          if (fresh.length === running.length && fresh.every((value) => running.includes(value))) break;
          running = fresh;
          setActionMessage(say("g1.appSwitchChanged"));
        }
        if (running.length > 0) setActionMessage(say("g1.appSwitchStopping"));
        for (const runningId of running) {
          await backend.stopSession(runningId, uiIdempotencyKey("session-switch-stop"));
          stoppedNames.push(sessionName(runningId));
        }
      }
    } catch (error) {
      setActionMessage(
        errorMessage(
          `${stoppedNames.length ? tm("g1.appSwitchStoppedList", { names: stoppedNames.join(", ") }) : ""}${tm("g1.appSwitchNotSwitched", { error: formatUiError(error, tm("g1.appSwitchVerifyFail")) })}`,
        ),
      );
      return { switched: false, stopped: stoppedNames.length > 0 };
    } finally {
      sessionBusy.current = false;
      setSessionActionBusy(false);
    }
    if (backend.connected) {
      try {
        await releaseOtherSessionAudio(await backend.refreshDiagnostics(), id);
      } catch {
        // Play releases it again before preparing the new session.
        recordUiDiagnostic("Releasing the previous session's audio adapter failed; Play will retry");
      }
    }
    setNativeGenerations({});
    setNativePumpStats(null);
    setAudioSourceStates({});
    setSelectedSessionId(id);
    if (backend.connected) void snapshotCache.refresh(backend, id).then(setSnapshotState);
    const name = availableSessions.find((item) => item.id === id)?.name ?? "the selected session";
    if (running.length > 0) setActionMessage(say("g1.appSwitchStopped", { names: stoppedNames.join(", "), name }));
    return { switched: true, stopped: running.length > 0 };
  };
  const addLibraryNode = (kind: LibraryNodeKind, _position?: { x: number; y: number }) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return;
    }
    const next = appendLibraryNode(draft, kind);
    const inserted = next.nodes[next.nodes.length - 1];
    recordDraftChange(next);
    setSelectedNodeId(inserted.id);
    setSelectedNodeIds([inserted.id]);
    setActionMessage(say("g1.appAddedSave", { name: inserted.name }));
    return inserted.id;
  };
  const addEndpointLoopback = (endpointId: string) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return;
    }
    try {
      const next = appendEndpointLoopbackNode(draft, endpointId);
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) {
        setSelectedNodeId(inserted.id);
        setSelectedNodeIds([inserted.id]);
      }
      setActionMessage(say("g1.appAddedSave", { name: inserted?.name ?? tm("g1.appEndpointLoopback") }));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appEndpointLoopbackFail")));
    }
  };
  const addVirtualBusNode = (busId: string, direction: "renderSource" | "captureSink") => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return undefined;
    }
    try {
      const next = appendVirtualBusNode(draft, busId, direction, getVirtualCableStatus());
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) {
        setSelectedNodeId(inserted.id);
        setSelectedNodeIds([inserted.id]);
      }
      setActionMessage(say("g1.appAddedSave", { name: inserted?.name ?? tm("g1.appVirtualBus") }));
      return inserted?.id;
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appVirtualBusFail")));
      return undefined;
    }
  };
  const addCablePassThrough = () => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return;
    }
    const status = getVirtualCableStatus();
    const source = defaultCable(status, "renderSource");
    const sink = defaultCable(status, "captureSink");
    try {
      recordDraftChange(appendCablePassThrough(draft, source, sink, status));
      setActionMessage(
        say("g1.appPassThroughAdded", {
          from: cableName(status, source),
          to: cableName(status, sink),
          input: cableEndpointName(source, "renderSource", status),
          output: cableEndpointName(sink, "captureSink", status),
        }),
      );
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appPassThroughFail")));
    }
  };
  const openDiscordSetup = () => {
    setWorkbenchTab("apps");
    setDiscordSetup((current) =>
      current?.sessionId === draftRef.current.id
        ? current
        : { key: Date.now(), sessionId: draftRef.current.id, added: null },
    );
  };
  /** Adds the Discord route as one undoable draft change; returns an error to show, or null. */
  const addDiscordRoute = (choice: DiscordSetupChoice): string | null => {
    if (!backend.connected) return tm("g1.appReconnectEdit");
    try {
      const result = appendDiscordRoute(draft, devices, choice);
      recordDraftChange(result.session);
      setDiscordSetup((current) => current && { ...current, added: result.discordMicrophone });
      setActionMessage(say("g1.appVoiceRouteAdded", { name: result.discordMicrophone.name }));
      return null;
    } catch (error) {
      return formatUiError(error, tm("g1.appDiscordRouteFail"));
    }
  };
  // A plugin dragged from the Plugins list onto the canvas lands where it is
  // dropped, like a built-in tool; it never replaces a pending insertion.
  const addDroppedPlugin = (entry: import("@audiorouter/contracts").PluginScanEntry) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return undefined;
    }
    try {
      const next = appendPluginPlaceholderNode(draft, entry);
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) {
        setSelectedNodeId(inserted.id);
        setSelectedNodeIds([inserted.id]);
      }
      setActionMessage(say("g1.appPluginAdded", { name: inserted?.name ?? tm("g1.appPlugin") }));
      return inserted?.id;
    } catch (error) {
      setActionMessage(errorMessage(error instanceof Error ? error.message : tm("g1.appPluginAddFail")));
      return undefined;
    }
  };
  const addPluginToDraft = (entry: import("@audiorouter/contracts").PluginScanEntry) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return;
    }
    try {
      if (pluginInsertEdgeId) {
        const next = insertDraftPluginProcessor(draft, pluginInsertEdgeId, entry);
        const inserted = next.nodes.at(-1);
        recordDraftChange(next);
        if (inserted) {
          setSelectedNodeId(inserted.id);
          setSelectedNodeIds([inserted.id]);
        }
        setActionMessage(say("g1.appPluginPlaceholder", { name: inserted?.name ?? tm("g1.appPlugin") }));
      } else {
        const next = appendPluginPlaceholderNode(draft, entry);
        const inserted = next.nodes.at(-1);
        recordDraftChange(next);
        if (inserted) {
          setSelectedNodeId(inserted.id);
          setSelectedNodeIds([inserted.id]);
        }
        setActionMessage(say("g1.appPluginNotLoaded", { name: inserted?.name ?? tm("g1.appPlugin") }));
      }
      closePluginPicker();
    } catch (error) {
      setActionMessage(errorMessage(error instanceof Error ? error.message : tm("g1.appPluginPlaceholderFail")));
    }
  };
  // A new cable node starts on a present cable (sources Cable A, outputs
  // Cable B by default); its Properties choose another.
  const addDroppedVirtualBusNode = (direction: "renderSource" | "captureSink", _position: { x: number; y: number }) =>
    addVirtualBusNode(defaultCable(getVirtualCableStatus(), direction), direction);
  const tryDraftConnection = (
    sourceNode: string,
    sourcePort: string,
    destinationNode: string,
    destinationPort: string,
  ): string | null => {
    const current = draftRef.current;
    try {
      const next = appendDraftConnection(current, sourceNode, sourcePort, destinationNode, destinationPort);
      const added = next.edges.find((edge) => !current.edges.some((existing) => existing.id === edge.id));
      recordDraftChange(next);
      setActionMessage(say("g1.appConnectionAdded"));
      return added?.id ?? null;
    } catch (error) {
      const occupied =
        error instanceof Error && error.message === "That input already has a connection"
          ? current.edges.find(
              (edge) => edge.destinationNode === destinationNode && edge.destinationPort === destinationPort,
            )
          : undefined;
      if (occupied) {
        const destinationName =
          current.nodes.find((node) => node.id === destinationNode)?.name ?? tm("g1.appThisInput");
        const priorName =
          current.nodes.find((node) => node.id === occupied.sourceNode)?.name ?? tm("g1.appAnotherSource");
        const destination = current.nodes.find((node) => node.id === destinationNode);
        if (destination?.kind === "plugin" && destinationPort === "sidechain") {
          setSidechainReplacePrompt({
            occupiedEdgeId: occupied.id,
            sourceNodeId: sourceNode,
            sourcePort,
            sourceName: current.nodes.find((node) => node.id === sourceNode)?.name ?? "the new source",
            destinationNodeId: destinationNode,
            destinationPort,
            pluginName: destination.name,
            currentSourceName: priorName,
          });
          setConnectionDialogOpen(false);
          return null;
        }
        if (destination?.kind === "physicalOutput") {
          const requestedSource = current.nodes.find((node) => node.id === sourceNode);
          if (requestedSource?.kind === "mixer") {
            try {
              const rerouted = routeFedMixerToOccupiedOutput(current, occupied.id, sourceNode, sourcePort);
              if (rerouted) {
                const newEdge = rerouted.edges.find(
                  (edge) =>
                    edge.sourceNode === sourceNode &&
                    edge.destinationNode === destinationNode &&
                    edge.destinationPort === destinationPort,
                );
                recordDraftChange(rerouted);
                setActionMessage(
                  say("g1.appConnectedRemoved", {
                    source: requestedSource.name,
                    destination: destinationName,
                    prior: priorName,
                  }),
                );
                setSelectedNodeId(requestedSource.id);
                setSelectedNodeIds([requestedSource.id]);
                return newEdge?.id ?? null;
              }
            } catch (mixError) {
              setActionMessage(uiError(mixError, tm("g1.appMixerRouteFail")));
              return null;
            }
          }
          try {
            const mixed = addSourceToOccupiedOutput(current, occupied.id, sourceNode, sourcePort);
            const mixer = mixed.nodes.at(-1);
            recordDraftChange(mixed);
            setActionMessage(
              say("g1.appMixerShare", {
                prior: priorName,
                source: current.nodes.find((node) => node.id === sourceNode)?.name ?? tm("g1.appNewSource"),
                destination: destinationName,
              }),
            );
            if (mixer) {
              setSelectedNodeId(mixer.id);
              setSelectedNodeIds([mixer.id]);
            }
            return mixer?.id ?? null;
          } catch (mixError) {
            setActionMessage(uiError(mixError, tm("g1.appCombineFail")));
            return null;
          }
        }
        try {
          const mixed = addSourceToOccupiedInput(current, occupied.id, sourceNode, sourcePort);
          const mixer = mixed.nodes.at(-1);
          recordDraftChange(mixed);
          setActionMessage(
            say("g1.appMixerFeed", {
              prior: priorName,
              source: current.nodes.find((node) => node.id === sourceNode)?.name ?? tm("g1.appNewSource"),
              destination: destinationName,
            }),
          );
          if (mixer) {
            setSelectedNodeId(mixer.id);
            setSelectedNodeIds([mixer.id]);
          }
          return mixer?.id ?? null;
        } catch (mixError) {
          setActionMessage(uiError(mixError, tm("g1.appCombineFail")));
          return null;
        }
      } else {
        setActionMessage(uiError(error, tm("g1.appConnectionFail")));
      }
      return null;
    }
  };
  const decideSidechainReplacement = (replace: boolean) => {
    const pending = sidechainReplacePrompt;
    setSidechainReplacePrompt(null);
    if (!replace || !pending) {
      setActionMessage(say("g1.appSidechainKept"));
      return;
    }
    try {
      const current = draftRef.current;
      const next = replaceDraftSidechainConnection(
        current,
        pending.occupiedEdgeId,
        pending.sourceNodeId,
        pending.sourcePort,
        pending.destinationNodeId,
        pending.destinationPort,
      );
      recordDraftChange(next);
      setActionMessage(say("g1.appSidechainReplaced", { name: pending.sourceName }));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appSidechainFail")));
    }
  };
  const addConnection = () => {
    const source = decodePort(connectionSource);
    const destination = decodePort(connectionDestination);
    if (!source || !destination) {
      setActionMessage(say("g1.appChoosePorts"));
      return false;
    }
    return tryDraftConnection(source.nodeId, source.portName, destination.nodeId, destination.portName) !== null;
  };
  const insertProcessor = (edgeId: string, kind: InsertableProcessorKind) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return;
    }
    try {
      const next = insertDraftProcessor(draft, edgeId, kind);
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) setSelectedNodeId(inserted.id);
      setActionMessage(say("g1.appAddedSave", { name: inserted?.name ?? kind }));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appInsertFail")));
    }
  };
  const appendPreset = (presetId: EqPresetId) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appConnectBeforePreset"));
      return;
    }
    try {
      const next = appendEqPresetNode(draft, presetId);
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) setSelectedNodeId(inserted.id);
      setActionMessage(say("g1.appAddedSave", { name: inserted?.name ?? tm("g1.appEqPreset") }));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appPresetFail")));
    }
  };
  const appendVoicePreset = (presetId: VoiceChainPresetId) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appConnectBeforePreset"));
      return;
    }
    try {
      const next = appendVoiceChainPreset(draft, presetId);
      const added = next.nodes.slice(draft.nodes.length);
      recordDraftChange(next);
      if (added[0]) setSelectedNodeId(added[0].id);
      setActionMessage(say("g1.appAddedSave", { name: added.map((node) => node.name).join(", ") }));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appVoicePresetFail")));
    }
  };
  // Canvas and library requests, handled with the latest draft.
  const onInsertProcessor = useEffectEvent((edgeId: string, kind: InsertableProcessorKind) =>
    insertProcessor(edgeId, kind),
  );
  const onAppendPreset = useEffectEvent((presetId: EqPresetId) => appendPreset(presetId));
  const onAppendVoicePreset = useEffectEvent((presetId: VoiceChainPresetId) => appendVoicePreset(presetId));
  useEffect(() => {
    const handleInsertProcessor = (event: Event) => {
      const detail = (event as CustomEvent<{ edgeId?: string; kind?: InsertableProcessorKind }>).detail;
      if (detail.edgeId && detail.kind) onInsertProcessor(detail.edgeId, detail.kind);
    };
    globalThis.addEventListener("audiorouter:insert-processor", handleInsertProcessor);
    return () => globalThis.removeEventListener("audiorouter:insert-processor", handleInsertProcessor);
  }, []);
  useEffect(() => {
    const handleRemoveNode = (event: Event) => {
      const nodeId = (event as CustomEvent<{ nodeId?: string }>).detail?.nodeId;
      if (!nodeId || !backend.connected) return;
      if (draft.nodes.length <= 1) {
        setActionMessage(say("g1.appKeepOneTool"));
        return;
      }
      try {
        const next = removeDraftNode(draft, nodeId);
        recordDraftChange(next);
        setSelectedNodeId(next.nodes[0]?.id ?? "");
        setActionMessage(say("g1.appToolRemoved"));
      } catch (error) {
        setActionMessage(uiError(error, tm("g1.appRemoveToolFail")));
      }
    };
    globalThis.addEventListener("audiorouter:remove-node", handleRemoveNode);
    return () => globalThis.removeEventListener("audiorouter:remove-node", handleRemoveNode);
  }, [backend, draft]);
  useEffect(() => {
    const handleAppendEqPreset = (event: Event) => {
      const detail = (event as CustomEvent<{ presetId?: EqPresetId }>).detail;
      if (detail.presetId) onAppendPreset(detail.presetId);
    };
    globalThis.addEventListener("audiorouter:append-eq-preset", handleAppendEqPreset);
    return () => globalThis.removeEventListener("audiorouter:append-eq-preset", handleAppendEqPreset);
  }, []);
  useEffect(() => {
    const handleAppendVoicePreset = (event: Event) => {
      const detail = (event as CustomEvent<{ presetId?: VoiceChainPresetId }>).detail;
      if (detail.presetId) onAppendVoicePreset(detail.presetId);
    };
    globalThis.addEventListener("audiorouter:append-voice-preset", handleAppendVoicePreset);
    return () => globalThis.removeEventListener("audiorouter:append-voice-preset", handleAppendVoicePreset);
  }, []);
  const addDroppedCanvasNode = (kind: string, _dropPosition?: { x: number; y: number }) => {
    if (kind === "virtualRenderSource" || kind === "virtualCaptureSink") {
      const direction = kind === "virtualRenderSource" ? "renderSource" : "captureSink";
      return addVirtualBusNode(defaultCable(getVirtualCableStatus(), direction), direction);
    }
    return addLibraryNode(kind as LibraryNodeKind);
  };
  const connectCanvas = (connection: Connection, dropPosition?: { x: number; y: number }) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appConnectBeforeConnection"));
      return;
    }
    if (connection.source === LIBRARY_DROP_SOURCE && connection.sourceHandle) {
      return addDroppedCanvasNode(connection.sourceHandle, dropPosition);
    }
    if (!connection.source || !connection.sourceHandle || !connection.target || !connection.targetHandle) {
      setActionMessage(say("g1.appChooseNamedPorts"));
      return;
    }
    return (
      tryDraftConnection(connection.source, connection.sourceHandle, connection.target, connection.targetHandle) ??
      undefined
    );
  };
  const openConnectionDialog = (event: React.MouseEvent<HTMLButtonElement>) => {
    connectionDialogReturnFocus.current = event.currentTarget;
    setConnectionDialogOpen(true);
  };
  const closeConnectionDialog = () => {
    setConnectionDialogOpen(false);
    window.setTimeout(() => connectionDialogReturnFocus.current?.focus(), 0);
  };
  useEffect(() => {
    if (!connectionDialogOpen) return;
    connectionDialogSource.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        closeConnectionDialog();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = connectionDialog.current?.querySelectorAll<HTMLElement>(
        "button:not([disabled]), select:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
      );
      if (!focusable?.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [connectionDialogOpen]);
  const openPluginPicker = (edgeId?: string) => {
    pluginPickerReturnFocus.current = document.activeElement as HTMLElement | null;
    setPluginInsertEdgeId(edgeId ?? null);
    setPluginPickerOpen(true);
  };
  const closePluginPicker = () => {
    setPluginPickerOpen(false);
    setPluginInsertEdgeId(null);
    window.setTimeout(() => pluginPickerReturnFocus.current?.focus(), 0);
  };
  useEffect(() => {
    if (!pluginPickerOpen) return;
    (
      pluginPickerDialog.current?.querySelector<HTMLElement>("input") ??
      pluginPickerDialog.current?.querySelector<HTMLElement>("button")
    )?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        closePluginPicker();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = pluginPickerDialog.current?.querySelectorAll<HTMLElement>(
        "button:not([disabled]), select:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
      );
      if (!focusable?.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [pluginPickerOpen]);
  const openApplicationPicker = () => {
    applicationPickerReturnFocus.current = document.activeElement as HTMLElement | null;
    setApplicationPickerSelection("");
    refreshApplications();
    setApplicationPickerOpen(true);
  };
  const closeApplicationPicker = () => {
    setApplicationPickerOpen(false);
    window.setTimeout(() => applicationPickerReturnFocus.current?.focus(), 0);
  };
  useEffect(() => {
    if (!applicationPickerOpen) return;
    (
      applicationPickerDialog.current?.querySelector<HTMLElement>("select") ??
      applicationPickerDialog.current?.querySelector<HTMLElement>("button:not([disabled])")
    )?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        closeApplicationPicker();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = applicationPickerDialog.current?.querySelectorAll<HTMLElement>(
        "button:not([disabled]), select:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])",
      );
      if (!focusable?.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [applicationPickerOpen]);
  const addApplicationCaptureFromPicker = (application: ApplicationRow) => {
    try {
      const next = appendApplicationCaptureNode(draft, application);
      const added = next.nodes.at(-1);
      if (!added) throw new Error("Application capture node was not created");
      recordDraftChange(next);
      setSelectedNodeId(added.id);
      setActionMessage(say("g1.appAppCaptureAdded"));
      closeApplicationPicker();
    } catch (error) {
      setActionMessage(errorMessage(error instanceof Error ? error.message : tm("g1.appAppCaptureFail")));
    }
  };
  const addApplicationCapturePathFromPicker = (path: string) => {
    try {
      const next = appendApplicationCapturePathNode(draftRef.current, path);
      const added = next.nodes.at(-1);
      if (!added) throw new Error("Application capture node was not created");
      recordDraftChange(next);
      setSelectedNodeId(added.id);
      setActionMessage(say("g1.appAppSelectorAdded", { executable: String(added.parameters.executable) }));
      closeApplicationPicker();
    } catch (error) {
      setActionMessage(errorMessage(error instanceof Error ? error.message : tm("g1.appAppCaptureFail")));
    }
  };
  const addApplicationGroupFromPicker = (chosen: ApplicationRow[], executablePaths: string[]): boolean => {
    try {
      const current = draftRef.current;
      const { session: next, mixerId } = appendApplicationCaptureGroup(current, chosen, executablePaths);
      const addedCount =
        next.nodes.filter((node) => node.kind === "applicationCapture").length -
        current.nodes.filter((node) => node.kind === "applicationCapture").length;
      recordDraftChange(next);
      setSelectedNodeId(mixerId);
      setActionMessage(say("g1.appAppGroupAdded", { count: addedCount }));
      closeApplicationPicker();
      return true;
    } catch (error) {
      setActionMessage(errorMessage(error instanceof Error ? error.message : tm("g1.appAppGroupFail")));
      return false;
    }
  };
  const rebindApplicationCapture = (nodeId: string, application: ApplicationRow) => {
    try {
      recordDraftChange(rebindApplicationCaptureNode(draftRef.current, nodeId, application));
      setActionMessage(
        say("g1.appAppSourceChanged", { executable: application.executable, pid: application.processId }),
      );
    } catch (error) {
      setActionMessage(errorMessage(error instanceof Error ? error.message : tm("g1.appAppSourceFail")));
    }
  };
  const removeConnection = (edgeId: string) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return;
    }
    const topologyAction = decodeTopologyAction(edgeId);
    try {
      if (topologyAction?.kind === "insertMixer") recordDraftChange(insertDraftMixer(draft, topologyAction.id));
      else if (topologyAction?.kind === "removeMixer")
        recordDraftChange(removeSinglePathDraftMixer(draft, topologyAction.id));
      else recordDraftChange(removeDraftConnection(draft, edgeId));
      setActionMessage(
        topologyAction?.kind === "insertMixer"
          ? say("g1.appMixerAdded")
          : topologyAction?.kind === "removeMixer"
            ? say("g1.appMixerRemoved")
            : say("g1.appConnectionRemoved"),
      );
    } catch (error) {
      setActionMessage(
        uiError(
          error,
          topologyAction?.kind === "insertMixer"
            ? tm("g1.appMixerInsertFail")
            : topologyAction?.kind === "removeMixer"
              ? tm("g1.appMixerRemoveFail")
              : tm("g1.appConnectionRemoveFail"),
        ),
      );
    }
  };
  const toggleConnection = (edgeId: string, enabled: boolean) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return;
    }
    try {
      recordDraftChange(setDraftConnectionEnabled(draft, edgeId, enabled));
      setActionMessage(say(enabled ? "g1.appConnectionOn" : "g1.appConnectionOff"));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appConnectionStateFail")));
    }
  };
  const removeSelectedNode = () => {
    if (!selectedNode) return;
    if (draft.nodes.length <= 1) {
      setActionMessage(say("g1.appKeepOneTool"));
      return;
    }
    try {
      const next = removeDraftNode(draft, selectedNode.id);
      recordDraftChange(next);
      setSelectedNodeId(next.nodes[0]?.id ?? "");
      setActionMessage(say("g1.appToolRemoved"));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appRemoveToolFail")));
    }
  };
  const unloadPluginNode = async (nodeId: string) => {
    const node = draft.nodes.find((candidate) => candidate.id === nodeId);
    if (!node) return;
    if (draft.nodes.length <= 1) {
      setActionMessage(say("g1.appKeepOneTool"));
      return;
    }
    const confirmed = await askConfirm({
      title: tm("g1.appRemoveTitle", { name: node.name }),
      message: tm("g1.appRemovePluginText"),
      confirmLabel: tm("g1.appRemovePlugin"),
      destructive: true,
    });
    if (!confirmed) return;
    try {
      // The route may have changed while the question was open.
      const next = removeDraftNode(draftRef.current, nodeId);
      recordDraftChange(next);
      if (selectedNodeId === nodeId) {
        setSelectedNodeId(next.nodes[0]?.id ?? "");
        setSelectedNodeIds(next.nodes[0] ? [next.nodes[0].id] : []);
      }
      setActionMessage(say("g1.appNodeRemoved", { name: node.name }));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appUnloadFail")));
    }
  };
  const duplicateNode = (nodeId: string) => {
    if (!backend.connected) return;
    try {
      const next = duplicateDraftNode(draft, nodeId);
      const copy = next.nodes[next.nodes.length - 1];
      recordDraftChange(next);
      setSelectedNodeId(copy.id);
      setSelectedNodeIds([copy.id]);
      setActionMessage(say("g1.appDuplicated", { name: copy.name }));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appDuplicateFail")));
    }
  };
  const duplicateSelectedNode = () => {
    if (selectedNode) duplicateNode(selectedNode.id);
  };
  const copyNodeToClipboard = (nodeId: string) => {
    try {
      const source = draftRef.current.nodes.find((node) => node.id === nodeId);
      if (!source) throw new Error(`Unknown draft node: ${nodeId}`);
      copiedNodeRef.current = copyDraftNode(draftRef.current, nodeId);
      setHasCopiedNode(true);
      setActionMessage(say("g1.appCopied", { name: source.name }));
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appCopyFail")));
    }
  };
  const pasteNodeFromClipboard = (_position: { x: number; y: number }) => {
    if (!backend.connected) {
      setActionMessage(say("g1.appReconnectEdit"));
      return undefined;
    }
    const copied = copiedNodeRef.current;
    if (!copied) return undefined;
    try {
      const next = pasteDraftNode(draftRef.current, copied);
      const inserted = next.nodes.at(-1);
      if (!inserted) return undefined;
      recordDraftChange(next);
      selectNodeProperties(inserted.id);
      setActionMessage(say("g1.appPasted", { name: inserted.name }));
      return inserted.id;
    } catch (error) {
      setActionMessage(uiError(error, tm("g1.appPasteFail")));
      return undefined;
    }
  };
  if (backend.connected && !hasSnapshot)
    return (
      <div className={`app-shell app-startup ${themeClassName}`} style={themeStyle}>
        <main
          className="panel startup-card"
          aria-label={savedDataDamaged ? tm("g1.appStartupRecoveryLabel") : tm("g1.appStartupLoadingLabel")}
          aria-busy={!snapshotState.error || reconnecting}
        >
          <p className="eyebrow">{tm("g1.appWorkspace")}</p>
          <h1>AudioRouter</h1>
          <p role="status">
            {reconnecting
              ? tm("g1.appReconnectingEllipsis")
              : savedDataDamaged
                ? tm("g1.appDataAttention")
                : snapshotState.error
                  ? tm("g1.appLoadFail", { error: snapshotState.error })
                  : tm("g1.appLoadingSession")}
          </p>
          <p className="muted">
            {savedDataDamaged
              ? tm("g1.appServiceStopped")
              : snapshotState.error
                ? tm("g1.appReconnectToRetry")
                : tm("g1.appRouteWillAppear")}
          </p>
          {snapshotState.error && !savedDataDamaged && (
            <CopyErrorButton message={tm("g1.appLoadFail", { error: snapshotState.error })} />
          )}
          {snapshotState.error && !savedDataDamaged && (
            <button type="button" onClick={() => void reconnect()} disabled={reconnecting} aria-busy={reconnecting}>
              {tm("g1.appReconnect")}
            </button>
          )}
          <StartupRecovery
            onRestored={async () => {
              window.location.reload();
            }}
            onDamaged={() => setSavedDataDamaged(true)}
          />
        </main>
      </div>
    );
  const connectionWorkbenchContent = (
    <ConnectionForm
      backend={editorBackend}
      connectionSource={connectionSource}
      setConnectionSource={setConnectionSource}
      outputPorts={outputPorts}
      encodePort={encodePort}
      connectionDestination={connectionDestination}
      setConnectionDestination={setConnectionDestination}
      inputPorts={inputPorts}
      addConnection={addConnection}
    />
  );
  const canvasToolbarActions = (
    <>
      <button
        type="button"
        className="secondary"
        aria-pressed={canvasKeyboardPanel === "routeList"}
        title={m("workspaceAudit.routeListHint")}
        onClick={() => setCanvasKeyboardPanel((current) => (current === "routeList" ? null : "routeList"))}
      >
        {m("settings.canvasKeyboard.routeList")}
      </button>
      <button
        type="button"
        className="secondary"
        aria-pressed={canvasKeyboardPanel === "connect"}
        title={m("workspaceAudit.connectHint")}
        onClick={() => setCanvasKeyboardPanel((current) => (current === "connect" ? null : "connect"))}
      >
        {m("workspaceAudit.connect")}
      </button>
    </>
  );
  const canvasOverlay = canvasKeyboardPanel && (
    <Suspense fallback={null}>
      <CanvasKeyboardPanel
        mode={canvasKeyboardPanel}
        onClose={() => setCanvasKeyboardPanel(null)}
        draft={draft}
        selectedNode={selectedNode}
        selectNodeProperties={selectNodeProperties}
        removeConnection={removeConnection}
        toggleConnection={toggleConnection}
        insertProcessor={insertProcessor}
        openPluginPicker={openPluginPicker}
        connectionContent={connectionWorkbenchContent}
      />
    </Suspense>
  );
  // Built once per App render, outside the per-tick telemetry render prop:
  // meter ticks reach the canvas only through its telemetry store.
  const flowCanvas = (
    <SessionFlowCanvas
      arrangeSessionId={arrangeSessionId}
      onArranged={() => setArrangeSessionId(null)}
      toolbarActions={canvasToolbarActions}
      canvasOverlay={canvasOverlay}
      nodeIssues={visibleNodeIssues}
      groups={groupState.groups}
      selectedGroupId={groupState.selectedGroupId}
      selectedGroupIds={groupState.selectedGroupIds}
      onSelectGroup={(id) => {
        groupState.selectGroup(id);
        if (id) {
          setSelectedNodeIds([]);
          setWorkbenchTab("properties");
        }
      }}
      onChangeGroup={groupState.changeGroup}
      onChangeGroups={groupState.changeGroups}
      onRemoveGroup={groupState.removeGroup}
      session={draft}
      pendingNodeIds={unsavedNodeIds(session, draft)}
      nodeSaveState={graphBusy ? "saving" : autoSavePending ? "autosave" : "needs-save"}
      selectedNodeId={selectedNode?.id ?? ""}
      selectedNodeIds={selectedNodeIds}
      onSelectItems={(ids) => {
        const groupIds = new Set(groupState.groups.map((group) => group.id));
        groupState.selectGroups(ids.filter((id) => groupIds.has(id)));
        const nodes = ids.filter((id) => !groupIds.has(id));
        setSelectedNodeIds(nodes);
        if (nodes[0]) setSelectedNodeId(nodes[0]);
        if (ids.length) setWorkbenchTab("properties");
      }}
      layoutActionsRef={canvasLayoutActions}
      layoutHistoryRef={canvasLayoutHistory}
      onLayoutHistory={setLayoutHistoryState}
      onLayoutHistoryStep={(step) => {
        editOrderRef.current =
          step === "edit"
            ? recordEdit(editOrderRef.current, "layout")
            : recordRestore(editOrderRef.current, "layout", step);
      }}
      panelWidth={sidebarWidth}
      onPanelWidth={changeSidebarWidth}
      onLayoutSelection={updateCanvasLayoutSelection}
      onLayoutMessage={setActionMessage}
      onCopyNode={copyNodeToClipboard}
      onDuplicateNode={duplicateNode}
      onRenameNode={(nodeId) => void renameCanvasNode(nodeId)}
      onPasteNode={pasteNodeFromClipboard}
      canPasteNode={hasCopiedNode}
      diagnostics={snapshot?.diagnostics ?? null}
      telemetryStore={telemetryStore}
      flowAnimation={flowAnimation}
      sessionRunning={sessionRunning}
      sessionActionBusy={sessionActionBusy}
      testSignalPlaybackReady={testSignalPlaybackReady}
      testSignalEndpointPrepared={testSignalEndpointPrepared}
      onStartTestSignal={() => void startSession()}
      onStopTestSignal={() => void stopSession()}
      onAudioSourceTransport={(nodeId, action) => void transportAudioSource(nodeId, action)}
      onTimeShiftTransport={(nodeId, action) => void transportTimeShift(nodeId, action)}
      timeShiftStatuses={timeShiftStatuses}
      audioSourceStates={audioSourceStates}
      recorderStatuses={recorderStatuses}
      onSetNodeParameter={changeNodeParameterOn}
      onToggleRecording={(nodeId, record) => void toggleNodeRecording(nodeId, record)}
      recordingBusyNodeId={recordingBusy}
      onSelect={selectNodeProperties}
      onSelectMany={(ids) => {
        setSelectedNodeIds(ids);
        if (ids[0]) setSelectedNodeId(ids[0]);
      }}
      onConnect={connectCanvas}
      onRemoveConnection={removeConnection}
      onToggleConnection={toggleConnection}
      onInsertProcessor={insertProcessor}
      onAddLibraryNode={addLibraryNode}
      onAddPluginNode={addDroppedPlugin}
      onAddVirtualBusNode={addDroppedVirtualBusNode}
      onOpenPluginPicker={openPluginPicker}
      onOpenApplicationPicker={openApplicationPicker}
      onConnectionRejected={setActionMessage}
      canEdit={backendReady}
      deviceUsers={backend.connected ? deviceUsers : undefined}
    />
  );
  const statusWorkbenchContent = (
    <SetupWorkbench
      section="status"
      setupCheck={setupCheck}
      onOpenFinding={(finding) => {
        if (!backend.connected) {
          setActionMessage(say("g1.appReconnectFinding"));
          return;
        }
        if (!availableSessions.some((item) => item.id === finding.sessionId)) {
          setActionMessage(say("g1.appFindingSessionGone"));
          return;
        }
        if (finding.sessionId !== draft.id && routeChanged) {
          setActionMessage(say("g1.appFindingSaveFirst"));
          return;
        }
        if (finding.sessionId === draft.id) {
          setSetupTarget({ sessionId: finding.sessionId, nodeId: finding.nodeId });
        } else {
          void switchSession(finding.sessionId).then(({ switched }) => {
            if (switched) {
              setSetupTarget({ sessionId: finding.sessionId, nodeId: finding.nodeId });
            }
          });
        }
      }}
      runningSessionIds={snapshot?.status.activeSessionIds ?? []}
      backend={backend}
      devices={devices}
      refreshDevices={refreshDevices}
      setupSteps={setupSteps}
      sessions={availableSessions}
    />
  );
  const refreshApps = applicationUsage.refresh;
  const refreshObs = obsStatus.refresh;
  const appsWorkbenchContent = (
    <AppsPanel
      discord={discordAppCheck}
      devices={devices}
      applications={applicationUsage.loaded ? applicationUsage.applications : []}
      stale={applicationUsage.stale}
      connected={backend.connected}
      onRefresh={() => {
        refreshApps();
        refreshObs();
      }}
      onOpenVirtualCables={() => setWorkbenchTab("virtualCables")}
      onSetUpDiscord={openDiscordSetup}
      discordSetup={
        discordSetup?.sessionId === draft.id && (
          <DiscordSetup
            key={discordSetup.key}
            options={discordSetupOptions(draft, devices)}
            connected={backend.connected}
            // After an Undo the route is gone, so the form comes back.
            added={
              discordSetup.added &&
              voiceCableOutputs(draft, devices).some((device) => device.id === discordSetup.added?.id)
                ? discordSetup.added
                : null
            }
            onAdd={addDiscordRoute}
            onClose={() => setDiscordSetup(null)}
            onOpenVirtualCables={() => setWorkbenchTab("virtualCables")}
          />
        )
      }
      obs={
        <ObsCard
          key={draft.id}
          check={obsAppCheck}
          status={obsStatus.status}
          actions={obsStatus.stale ? null : obsActions}
        />
      }
    />
  );
  const cableWorkbenchContent = (
    <SetupWorkbench
      section="virtualCables"
      backend={backend}
      devices={devices}
      refreshDevices={refreshDevices}
      setupSteps={setupSteps}
      cableRoute={snapshot?.diagnostics?.cableRoute}
      onAddCablePassThrough={addCablePassThrough}
      sessions={availableSessions}
      onActionMessage={setActionMessage}
    />
  );
  const uiOptionsWorkbenchContent = (
    <UiOptionsWorkbench
      theme={theme}
      onTheme={setTheme}
      language={language}
      onLanguage={changeLanguage}
      customThemes={customThemes}
      onCustomThemes={changeCustomThemes}
      flowAnimation={flowAnimation}
      onFlowAnimation={changeFlowAnimation}
      liveAutosave={liveAutosave}
      onLiveAutosave={changeLiveAutosave}
    />
  );
  const devicesWorkbenchContent = (
    <DeviceTroubleshooting
      backend={backend}
      session={session}
      devices={devices}
      sessionRunning={sessionRunning}
      startSession={startSession}
      stopSession={stopSession}
      addEndpointLoopback={addEndpointLoopback}
      captureEndpointId={captureEndpointId}
      setCaptureEndpointId={setCaptureEndpointId}
      renderEndpointId={renderEndpointId}
      setRenderEndpointId={setRenderEndpointId}
      addVirtualBusNode={addVirtualBusNode}
    />
  );
  const recordingWorkbenchContent = (
    <RecordingWorkbench
      backend={backend}
      session={session}
      recorderStatuses={recorderStatuses}
      recorderStatusAvailable={recorderStatusAvailable}
      draft={draft}
      selectedNode={selectedNode}
      setSelectedNodeId={setSelectedNodeId}
      setSelectedNodeIds={setSelectedNodeIds}
      recorderFormat={recorderFormat}
      setRecorderFormat={setRecorderFormat}
      recordings={recordings}
      recordingMutationBusyState={recordingMutationBusyState}
      renameRecording={renameRecording}
      revealRecording={revealRecording}
      recycleRecording={recycleRecording}
      recordingsError={recordingsError}
      visibleRecordings={visibleRecordings}
      recordingSearch={recordingSearch}
      setRecordingSearch={setRecordingSearch}
      metadataTitles={metadataTitles}
      setMetadataTitles={setMetadataTitles}
      metadataArtists={metadataArtists}
      setMetadataArtists={setMetadataArtists}
      metadataComments={metadataComments}
      setMetadataComments={setMetadataComments}
      saveRecordingMetadata={saveRecordingMetadata}
      previewRecording={previewRecording}
      inspectRecovery={inspectRecovery}
      removeRecordingEntry={removeRecordingEntry}
      previewMessage={previewMessage}
      recoveryMessage={recoveryMessage}
    />
  );
  const finishGoal = (result: import("@audiorouter/contracts").SessionFromGoalResult) => {
    setGoalDialogOpen(false);
    const firstRun = readFirstRunDone() === false;
    writeFirstRunDone();
    setCreatedSessions((current) => [...current, result.session]);
    setArrangeSessionId(result.session.id);
    const choose =
      result.stillToChoose.length > 0
        ? ` ${tm("g1.appStillToChoose", { names: result.stillToChoose.join(", ") })}`
        : "";
    void switchSession(result.session.id).then(({ switched }) => {
      if (switched) {
        const made = say("g1.appGoalMade", { name: result.session.name });
        setActionMessage({ ...made, text: `${made.text}${choose}` });
      }
    });
    if (firstRun) setTourOpen(true);
  };
  const closeGoalDialog = () => {
    setGoalDialogOpen(false);
    writeFirstRunDone();
  };
  const paletteItems = (): PaletteItem[] => {
    const ready = backend.connected;
    const items: PaletteItem[] = [
      {
        id: "play",
        group: "Actions",
        icon: sessionRunning ? "square" : "play",
        title: sessionRunning ? tm("g1.appPalStop") : tm("g1.appPalPlay"),
        detail: sessionRunning ? tm("g1.appPalStopRoute") : tm("g1.appPalStartRoute"),
        disabled: !ready,
        run: () => void (sessionRunning ? stopSession() : startSession()),
      },
      {
        id: "privacy",
        group: "Actions",
        icon: "mic-off",
        title: tm("g1.appPalPrivacy"),
        detail: privacyMuted ? tm("g1.appPalTurnOff") : tm("g1.appPalTurnOn"),
        keywords: "silence microphone",
        disabled: !ready,
        run: () => void togglePrivacyMute(),
      },
      {
        id: "goal",
        group: "Actions",
        icon: "square-dashed",
        title: tm("g1.appPalGoal"),
        detail: tm("g1.appPalGoalDetail"),
        keywords: "template new session",
        disabled: !ready,
        run: () => setGoalDialogOpen(true),
      },
      {
        id: "tour",
        group: "Actions",
        icon: "circle-help",
        title: tm("g1.appPalTour"),
        detail: tm("g1.appPalTourDetail"),
        keywords: "help",
        run: () => setTourOpen(true),
      },
      {
        id: "history",
        group: "Actions",
        icon: "undo-2",
        title: tm("g1.appPalHistory"),
        detail: tm("g1.appPalHistoryDetail"),
        keywords: "versions undo",
        run: () => setWorkbenchTab("session"),
      },
    ];
    for (const entry of libraryEntries) {
      const kind = entry.kind;
      if (!kind || entry.unavailableReason) continue;
      items.push({
        id: `tool-${entry.id}`,
        group: "Tools",
        icon: libraryEntryIcon(entry),
        title: tm("g1.appPalAdd", { name: entry.label }),
        detail: toolDescription(kind) ?? entry.category,
        keywords: `${entry.category} ${entry.note ?? ""}`,
        disabled: !ready,
        run: () => addLibraryNode(kind),
      });
    }
    for (const [page] of SETTINGS_PAGES) {
      items.push({
        id: `settings-${page}`,
        group: "Settings",
        icon: SETTINGS_PAGE_ICONS[page],
        title: settingsPageTitle(page),
        detail: tm("g1.appPalOpenSettings"),
        keywords: settingsPageKeywords(page),
        run: () => setWorkbenchTab(page),
      });
    }
    for (const node of draft.nodes) {
      items.push({
        id: `card-${node.id}`,
        group: "On this route",
        icon: NODE_KIND_ICONS[node.kind],
        title: node.name,
        detail: tm("g1.appPalGoTo", { kind: humanizeNodeKind(node) }),
        run: () => {
          selectNodeProperties(node.id);
          setWorkbenchTab("properties");
        },
      });
    }
    for (const item of availableSessions) {
      if (item.id === session.id) continue;
      items.push({
        id: `session-${item.id}`,
        group: "Sessions",
        icon: "folder",
        title: item.name,
        detail: tm("g1.appPalSwitch"),
        disabled: !ready,
        run: () => void switchSession(item.id),
      });
    }
    return items;
  };
  const refreshAfterPanelAction = async () => {
    await refresh();
  };
  const settingsWorkbenchContent = {
    plugins: <PluginScanPanel backend={backend} onAddPlaceholder={addPluginToDraft} />,
    help: (
      <section className="panel help-page" aria-labelledby="help-page-heading">
        <h2 id="help-page-heading">{tm("g1.appHelp")}</h2>
        <p className="muted">{m("settingsCopy.helpSearch")}</p>
        <div className="actions">
          <button type="button" className="secondary" onClick={() => setTourOpen(true)}>
            {tm("g1.appPalTour")}
          </button>
          <button
            type="button"
            className="secondary"
            disabled={!backend.connected}
            onClick={() => setGoalDialogOpen(true)}
          >
            {tm("g1.appPalGoal")}
          </button>
        </div>
      </section>
    ),
    deviceAccess: (
      <>
        <DeviceAccessSetting backend={backend} />
        <FirstRunGuide
          devices={devices}
          connected={backend.connected}
          onRefresh={() => refreshDevices()}
          onOpenTools={() => setWorkbenchTab("tools")}
          onSetUpDiscord={openDiscordSetup}
        />
      </>
    ),
    startup: <StartupSettings backend={backend} />,
    shortcuts: <ShortcutSettings snapshot={snapshot} />,
    versions: (
      <UpdatesPanel
        enabled={updateChecks}
        channel={updateChannel}
        onChange={(enabled) => {
          setUpdateCheckEnabled(enabled);
          setUpdateChecks(enabled);
        }}
        onChannelChange={(channel) => {
          setUpdateChannel(channel);
          setUpdateChannelState(channel);
        }}
        updateState={updateState}
      />
    ),
    clients: <ClientsPanel backend={backend} />,
    sessionJson: (
      <SessionTransferPanel
        backend={backend}
        session={session}
        quarantinedSessions={snapshot?.status.quarantinedSessions}
        onRecovered={() => void refresh()}
        onImported={(imported) => {
          setCreatedSessions((current) => [...current.filter((item) => item.id !== imported.id), imported]);
          void switchSession(imported.id);
          void refresh();
        }}
      />
    ),
    catalog: (
      <>
        <ProcessorCatalog processors={processors} error={processorError} node={selectedNode} backend={backend} />
        <PresetCatalog presets={presets} error={presetError} />
      </>
    ),
    identity: <ApplicationIdentityPanel applications={applications} />,
  };
  const statusWithTroubleshooting = (
    <>
      <Suspense fallback={null}>
        <LiveTelemetry store={telemetryStore} diagnostics={snapshot?.diagnostics ?? null}>
          {(liveDiagnostics) => (
            <MicTest
              session={session}
              diagnostics={liveDiagnostics}
              running={sessionRunning}
              privacyMuted={privacyMuted}
            />
          )}
        </LiveTelemetry>
      </Suspense>
      {statusWorkbenchContent}
      <TroubleshootPanel
        backend={backend}
        devicesContent={devicesWorkbenchContent}
        refresh={refreshAfterPanelAction}
        snapshot={snapshot}
        clearRecoverySafeMode={clearRecoverySafeMode}
        safetyActionBusyState={safetyActionBusyState}
      />
    </>
  );
  const renderedWorkspace = (
    <PluginParameterContext.Provider value={{ parameters: pluginParameters, error: pluginParameterError }}>
      <SoundboardContext.Provider value={soundboard}>
        <div
          className={`app-shell ${themeClassName}`}
          style={themeStyle}
          onContextMenu={(event) => {
            if (!keepNativeContextMenu(event.target, window.getSelection())) event.preventDefault();
          }}
        >
          <nav className="skip-navigation" aria-label={m("topbar.skipNavigation")}>
            {tabPlace(workbenchTab) !== "settings" && (
              <a
                href="#routing-canvas"
                onClick={(event) => {
                  event.preventDefault();
                  document.getElementById("routing-canvas")?.focus();
                }}
              >
                {m("topbar.skipToCanvas")}
              </a>
            )}
            <a
              href="#routing-workbench"
              onClick={(event) => {
                event.preventDefault();
                document.getElementById("routing-workbench")?.focus();
              }}
            >
              {m("topbar.skipToTools")}
            </a>
          </nav>
          <header className="topbar">
            <div>
              <VersionLine update={availableUpdate} />
              <h1>
                <button
                  type="button"
                  className="header-session-name"
                  title={m("topbar.openSessionTab")}
                  onClick={() => setWorkbenchTab("session")}
                >
                  {draft.name || m("topbar.routingWorkspace")}
                </button>
              </h1>
            </div>
            <div className={`status-cluster ${backend.connected ? "connected" : "disconnected"}`} aria-live="polite">
              {/* Only when the connection needs it. It sits first: the cluster is right-aligned,
                  so its arrival moves nothing else (UI-17). */}
              {(!backend.connected || staleSnapshot || reconnecting) && (
                <button type="button" onClick={() => void reconnect()} disabled={reconnecting} aria-busy={reconnecting}>
                  {m("topbar.reconnect")}
                </button>
              )}
              <span
                className={`audio-run-state${sessionRunning && !staleSnapshot ? " is-running" : ""}${
                  routeMicMuted && !staleSnapshot ? " is-mic-muted" : ""
                }`}
                role="status"
                title={
                  routeMicMuted && !staleSnapshot
                    ? tm("g1.appStatusMuted")
                    : staleSnapshot
                      ? tm("g1.appStatusLost")
                      : routeChanged
                        ? tm("g1.appStatusUnsaved")
                        : tm("g1.appStatusSaved")
                }
              >
                <span className="audio-state-dot" aria-hidden="true" />
                {staleSnapshot
                  ? m("topbar.statusUnavailable")
                  : routePlaybackLabel({
                      busy: sessionActionBusy,
                      running: sessionRunning,
                      fresh:
                        backend.connected &&
                        !snapshotState.stale &&
                        snapshot?.diagnostics.nativeSessionId === session.id,
                      generation: snapshot?.diagnostics.schedulerTelemetry?.activeGeneration,
                      started: nativeGenerations[session.id],
                      micMuted: routeMicMuted,
                    })}
              </span>
              <span className="status-detail" title={statusSummary}>
                {connectionLabel}
              </span>
              <AppStatusStrip checks={appStripChecks} onOpen={() => setWorkbenchTab("apps")} />
              <span className="history-toolbar">
                <button
                  type="button"
                  className="secondary"
                  aria-label={m("topbar.undo")}
                  title={m("topbar.undoTitle")}
                  onClick={() => restoreNewestEdit("undo")}
                  disabled={!canRestoreEdit.undo}
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M9 4 4 9l5 5M4 9h10a6 6 0 0 1 0 12" />
                  </svg>
                  {m("topbar.undo")}
                </button>
                <button
                  type="button"
                  className="secondary"
                  aria-label={m("topbar.redo")}
                  title={m("topbar.redoTitle")}
                  onClick={() => restoreNewestEdit("redo")}
                  disabled={!canRestoreEdit.redo}
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="m15 4 5 5-5 5m5-5H10a6 6 0 0 0 0 12" />
                  </svg>
                  {m("topbar.redo")}
                </button>
              </span>
              <button
                type="button"
                className={routeChanged ? "secondary save-pending" : "secondary"}
                title={
                  autoSavePending
                    ? m("topbar.saveTitleAuto")
                    : routeChanged
                      ? m("topbar.saveTitlePending")
                      : m("topbar.saveTitleSaved")
                }
                data-save-required={routeChanged && !autoSavePending && !graphBusy}
                aria-busy={graphBusy}
                onClick={saveChanges}
                disabled={saveDisabled}
              >
                {graphBusy ? m("topbar.saving") : pendingGraphPlan ? m("topbar.confirmSave") : m("topbar.save")}
              </button>
              <button
                type="button"
                className={sessionRunning ? "play-stop stop-audio" : "play-stop primary"}
                title={m("topbar.playTitle")}
                onClick={() => void (sessionRunning ? stopSession() : startSession())}
                disabled={!backend.connected || sessionActionBusy || (!sessionRunning && staleSnapshot)}
                aria-busy={sessionActionBusy}
              >
                <span data-tour="play">{sessionRunning ? m("topbar.stop") : m("topbar.play")}</span>
              </button>
              <QuitButton onMessage={setActionMessage} beforeQuit={draftRecovery.flush} />
            </div>
          </header>
          <aside
            ref={notificationsRef}
            className="workspace-notifications"
            aria-label={m("topbar.notifications")}
            style={{ "--sidebar-width": `${sidebarWidth}px` } as CSSProperties}
          >
            {unfedNodes.length > 0 &&
              dismissedRouteWarning !== routeWarningKey &&
              tabPlace(workbenchTab) !== "settings" && (
                <div
                  className="panel-message is-warning inactive-route-warning"
                  onMouseEnter={() => setRouteWarningHeld(true)}
                  onMouseLeave={() => setRouteWarningHeld(false)}
                  onFocus={() => setRouteWarningHeld(true)}
                  onBlur={() => setRouteWarningHeld(false)}
                >
                  <p role="status">
                    {m("workspaceAudit.unfed", { names: unfedNodes.map((node) => node.name).join(", ") })}
                  </p>
                  <button
                    type="button"
                    className="secondary"
                    aria-label={m("workspaceAudit.dismissWarning")}
                    onClick={() => setDismissedRouteWarning(routeWarningKey)}
                  >
                    <Icon name="x" />
                  </button>
                </div>
              )}
            {(() => {
              const text = actionMessage
                ? actionMessage.text
                : !backend.connected
                  ? tm("g1.appAudioUnavailable", {
                      reason: (backendText(snapshot?.status.reason) ?? tm("g1.appBackendDisconnected")).replace(
                        /\.+$/,
                        "",
                      ),
                    })
                  : snapshot?.status.quarantinedSessions?.length
                    ? tm("g1.appQuarantined")
                    : "";
              const tone = !backend.connected
                ? "error"
                : actionMessage
                  ? actionMessageTone(actionMessage)
                  : snapshot?.status.quarantinedSessions?.length
                    ? "warning"
                    : "info";
              return (
                <div
                  className={`global-action-message is-${tone}${text ? "" : " is-empty"}`}
                  aria-hidden={!text}
                  onMouseEnter={() => setActionMessageHeld(true)}
                  onMouseLeave={() => setActionMessageHeld(false)}
                  onFocus={() => setActionMessageHeld(true)}
                  onBlur={() => setActionMessageHeld(false)}
                  role={tone === "error" ? "alert" : "status"}
                  aria-live={tone === "error" ? "assertive" : "polite"}
                >
                  <span className="global-action-message-icon" aria-hidden="true">
                    <Icon
                      name={
                        tone === "error"
                          ? "circle-alert"
                          : tone === "warning"
                            ? "triangle-alert"
                            : tone === "success"
                              ? "circle-check"
                              : "info"
                      }
                      size={18}
                    />
                  </span>
                  <span className="global-action-message-text" title={text}>
                    {text}
                  </span>
                  {text && tone === "error" && <CopyErrorButton message={text} />}
                  {!actionMessage && backend.connected && Boolean(snapshot?.status.quarantinedSessions?.length) && (
                    <button type="button" className="secondary" onClick={() => setWorkbenchTab("sessionJson")}>
                      {tm("g1.appOpenRecovery")}
                    </button>
                  )}
                  {backend.connected && offersAction(actionMessage, "goToIssueNode") && firstIssueNode && (
                    <button type="button" className="secondary" onClick={() => selectNodeProperties(firstIssueNode.id)}>
                      Go to {firstIssueNode.name}
                    </button>
                  )}
                  {backend.connected && offersAction(actionMessage, "openStatus") && workbenchTab !== "status" && (
                    <button type="button" className="secondary" onClick={() => setWorkbenchTab("status")}>
                      {tm("g1.appOpenStatus")}
                    </button>
                  )}
                  {backend.connected && offersAction(actionMessage, "openSession") && workbenchTab !== "session" && (
                    <button type="button" className="secondary" onClick={() => setWorkbenchTab("session")}>
                      {tm("g1.appOpenSession")}
                    </button>
                  )}
                  {backend.connected && offersAction(actionMessage, "discardEdits") && routeChanged && (
                    <button type="button" className="secondary" onClick={discardRouteEdits}>
                      {tm("g1.appDiscardEdits")}
                    </button>
                  )}
                  {backend.connected && offersAction(actionMessage, "olderEdits") && olderEdits && (
                    <>
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => {
                          recordDraftChange({ ...olderEdits, revision: session.revision });
                          setOlderEdits(null);
                          setActionMessage(say("g1.appOlderBack"));
                        }}
                      >
                        {tm("g1.appUseMyEdits")}
                      </button>
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => {
                          setOlderEdits(null);
                          setActionMessage(null);
                          void draftRecovery
                            .forget()
                            .catch(() => recordUiDiagnostic("Removing older unsaved edits failed"));
                        }}
                      >
                        {tm("g1.appKeepSaved")}
                      </button>
                    </>
                  )}
                  {backend.connected && actionMessage && (
                    <button
                      type="button"
                      className="secondary global-action-message-dismiss"
                      aria-label={tm("g1.appDismissMessage")}
                      title={tm("g1.appDismissMessage")}
                      onClick={() => {
                        setActionMessage(null);
                      }}
                    >
                      <Icon name="x" />
                    </button>
                  )}
                </div>
              );
            })()}
          </aside>
          {pendingGraphPlan && pendingWarnings.length > 0 && (
            <section
              className="save-warning-review"
              aria-label={tm("g1.appSaveWarnings")}
              style={{ "--sidebar-width": `${sidebarWidth}px` } as CSSProperties}
            >
              <strong>{tm("g1.appReviewBefore")}</strong>
              {pendingWarnings.map((warning) => (
                <label key={warning}>
                  <input
                    type="checkbox"
                    checked={acknowledgedWarnings.has(warning)}
                    onChange={(event) =>
                      setAcknowledgedWarnings((current) => {
                        const next = new Set(current);
                        if (event.target.checked) next.add(warning);
                        else next.delete(warning);
                        return next;
                      })
                    }
                  />
                  {warning}
                </label>
              ))}
            </section>
          )}
          <div className="workspace-grid">
            <main
              className={`main-content${tabPlace(workbenchTab) === "settings" ? " settings-open" : ""}`}
              style={{ "--sidebar-width": `${sidebarWidth}px` } as CSSProperties}
            >
              <SidebarResizer
                width={sidebarWidth}
                onWidth={changeSidebarWidth}
                onResized={(before, after) => canvasLayoutHistory.current?.rememberPanelWidth(before, after)}
              />
              <section className="workspace-title">
                <div>
                  <label className="session-name">
                    {tm("g1.appSessionName")}
                    <input
                      value={draft.name}
                      maxLength={120}
                      disabled={!backend.connected}
                      onChange={(event) => changeSessionName(event.target.value)}
                    />
                  </label>
                  <p className="muted">
                    Revision {session.revision} -{" "}
                    {backend.connected
                      ? "edits stay unsaved until you review and save them"
                      : "changes are presentation-only in this preview"}
                  </p>
                </div>
                <div className="actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void duplicateSession()}
                    disabled={!backend.connected}
                  >
                    {tm("g1.appDuplicate")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void deleteSession()}
                    disabled={!backend.connected}
                  >
                    {tm("g1.appDelete")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={undoDraft}
                    disabled={!backend.connected || draftHistory.past.length === 0}
                  >
                    {tm("g1.appUndoEdit")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={redoDraft}
                    disabled={!backend.connected || draftHistory.future.length === 0}
                  >
                    {tm("g1.appRedoEdit")}
                  </button>
                  <button type="button" className="secondary" onClick={discardRouteEdits} disabled={!backend.connected}>
                    {tm("g1.appDiscardEdits")}
                  </button>
                  <button
                    type="button"
                    className="primary"
                    onClick={() => void planChanges()}
                    disabled={!backend.connected || graphBusy}
                  >
                    {tm("g1.appSaveChanges")}
                  </button>
                </div>
              </section>
              <ErrorBoundary area={tm("g1.appAreaFlow")} onError={recordUiDiagnostic}>
                <LiveTelemetry store={telemetryStore} diagnostics={snapshot?.diagnostics ?? null}>
                  {(liveDiagnostics) => (
                    <SignalFlowPanel
                      flowCanvas={flowCanvas}
                      backend={editorBackend}
                      connectionSource={connectionSource}
                      setConnectionSource={setConnectionSource}
                      outputPorts={outputPorts}
                      encodePort={encodePort}
                      connectionDestination={connectionDestination}
                      setConnectionDestination={setConnectionDestination}
                      inputPorts={inputPorts}
                      addConnection={addConnection}
                      openConnectionDialog={openConnectionDialog}
                      connectionDialogOpen={connectionDialogOpen}
                      connectionDialog={connectionDialog}
                      closeConnectionDialog={closeConnectionDialog}
                      connectionDialogSource={connectionDialogSource}
                      pluginPickerOpen={pluginPickerOpen}
                      pluginPickerDialog={pluginPickerDialog}
                      pluginInsertEdgeId={pluginInsertEdgeId}
                      closePluginPicker={closePluginPicker}
                      draft={draft}
                      selectedNode={selectedNode}
                      setSelectedNodeId={setSelectedNodeId}
                      setSelectedNodeIds={setSelectedNodeIds}
                      unloadPluginNode={unloadPluginNode}
                      liveDiagnostics={liveDiagnostics}
                      addPluginToDraft={addPluginToDraft}
                      applicationPickerOpen={applicationPickerOpen}
                      applications={applications}
                      applicationPickerSelection={applicationPickerSelection}
                      applicationPickerDialog={applicationPickerDialog}
                      closeApplicationPicker={closeApplicationPicker}
                      refreshApplications={refreshApplications}
                      applicationsError={applicationsError}
                      setApplicationPickerSelection={setApplicationPickerSelection}
                      addApplicationCaptureFromPicker={addApplicationCaptureFromPicker}
                      addApplicationCapturePathFromPicker={addApplicationCapturePathFromPicker}
                      addApplicationGroupFromPicker={addApplicationGroupFromPicker}
                    />
                  )}
                </LiveTelemetry>
              </ErrorBoundary>
              <ErrorBoundary area={tm("g1.appAreaProps")} onError={recordUiDiagnostic}>
                <LiveTelemetry store={telemetryStore} diagnostics={snapshot?.diagnostics ?? null}>
                  {(liveDiagnostics) => (
                    <PropertiesPanel
                      layoutItems={canvasLayoutSelection.sessionId === draft.id ? canvasLayoutSelection.items : []}
                      onLayoutAction={(action) => canvasLayoutActions.current?.(action)}
                      selectedGroup={selectedGroup}
                      groupState={groupState}
                      selectedNode={selectedNode}
                      backend={editorBackend}
                      sessionRunning={sessionRunning}
                      liveDiagnostics={liveDiagnostics}
                      session={session}
                      recordDraftChange={recordDraftChange}
                      draft={draft}
                      setActionMessage={setActionMessage}
                      changeNodeParameterOn={changeNodeParameterOn}
                      devices={devices}
                      refreshDevices={refreshDevices}
                      setCaptureEndpointId={setCaptureEndpointId}
                      inputBindingFocusRequest={inputBindingFocusRequest}
                      renderEndpointId={renderEndpointId}
                      applications={applications}
                      applicationsError={applicationsError}
                      refreshApplications={refreshApplications}
                      deviceUsers={backend.connected ? deviceUsers : undefined}
                      rebindApplicationCapture={rebindApplicationCapture}
                      setRenderEndpointId={setRenderEndpointId}
                      captureEndpointId={captureEndpointId}
                      recorderStatuses={recorderStatuses}
                      recordingBusy={recordingBusy}
                      lastRecordingPaths={lastRecordingPaths}
                      recordingMessage={recordingMessage}
                      toggleNodeRecording={toggleNodeRecording}
                      audioSourceStates={audioSourceStates}
                      currentSessionIdRef={currentSessionIdRef}
                      transportAudioSource={transportAudioSource}
                      unloadPluginNode={unloadPluginNode}
                      changeNodeName={changeNodeName}
                      sessionActionBusy={sessionActionBusy}
                      graphBusy={graphBusy}
                      changeNodeFlag={changeNodeFlag}
                      processors={processors}
                      snapshot={snapshot}
                      changeNodeParameter={changeNodeParameter}
                      resetNodeParameters={resetNodeParameters}
                      duplicateSelectedNode={duplicateSelectedNode}
                      removeSelectedNode={removeSelectedNode}
                      togglePrivacyMute={togglePrivacyMute}
                      privacyMuted={privacyMuted}
                    />
                  )}
                </LiveTelemetry>
              </ErrorBoundary>
              <ErrorBoundary area={tm("g1.appAreaSide")} onError={recordUiDiagnostic}>
                <Workbench
                  onAddGroup={() => {
                    groupState.addGroup();
                    setSelectedNodeIds([]);
                    setWorkbenchTab("properties");
                  }}
                  tab={workbenchTab}
                  onTab={(tab) => setWorkbenchTab(tab)}
                  sessionFileContent={
                    // Its own boundary: the session picker and actions above
                    // must not wait for this lazily loaded section.
                    <Suspense fallback={<p role="status">{m("workflowUi.loadingSessionFile")}</p>}>
                      <SessionFilePanel
                        backend={editorBackend}
                        session={session}
                        unsaved={routeChanged}
                        onImported={(imported) => {
                          setCreatedSessions((current) => [
                            ...current.filter((item) => item.id !== imported.id),
                            imported,
                          ]);
                          void switchSession(imported.id);
                        }}
                      />
                    </Suspense>
                  }
                  pluginsContent={
                    <PluginToolsGroup
                      backend={editorBackend}
                      connected={backendReady}
                      search={librarySearch}
                      refreshKey={pluginPickerOpen}
                      onAdd={addPluginToDraft}
                      onOpenPicker={() => openPluginPicker()}
                    />
                  }
                  tools={visibleLibraryEntries}
                  connected={backendReady}
                  onAdd={(kind, options) => {
                    const id = addLibraryNode(kind);
                    // Added with the keyboard: focus the new card so the arrow
                    // keys place it, the keyboard path for drag and drop.
                    if (id && options?.focus) focusCanvasCardWhenShown(id);
                  }}
                  onAddCable={(direction) =>
                    addVirtualBusNode(defaultCable(getVirtualCableStatus(), direction), direction)
                  }
                  onApplicationPicker={openApplicationPicker}
                  librarySearch={librarySearch}
                  onLibrarySearch={(value) => setLibrarySearch(value.slice(0, 80))}
                  onNewSession={() => void createSession()}
                  onDuplicate={() => void duplicateSession()}
                  onDelete={() => void deleteSession()}
                  onDiscard={discardRouteEdits}
                  onPlan={() => void planChanges()}
                  onCommit={() => void commitAcknowledgedPlan()}
                  canCommit={acknowledgedWarnings.size === pendingWarnings.length && !graphBusy}
                  pendingPlan={Boolean(pendingGraphPlan)}
                  actionMessage={actionMessage}
                  apiBuilder={(baseUrl) => (
                    <RequestBuilder
                      sessions={availableSessions}
                      activeSessionId={session.id}
                      nodeTypes={snapshot?.discovery?.nodeTypes ?? null}
                      baseUrl={baseUrl}
                      connected={backend.connected}
                      onSend={backend.setNode ? (params) => backend.setNode!(params) : undefined}
                    />
                  )}
                  sessions={availableSessions}
                  selectedSessionId={session.id}
                  onSelectSession={(id) => void switchSession(id)}
                  revision={session.revision}
                  sessionName={draft.name}
                  onNameChange={changeSessionName}
                  warnings={pendingWarnings}
                  acknowledgedWarnings={[...acknowledgedWarnings]}
                  onAcknowledgeWarning={(warning, checked) =>
                    setAcknowledgedWarnings((current) => {
                      const next = new Set(current);
                      if (checked) next.add(warning);
                      else next.delete(warning);
                      return next;
                    })
                  }
                  diagnostics={uiDiagnostics}
                  verboseLogging={verboseLogging}
                  backendActivity={backendActivity}
                  mcpActivity={mcpActivity}
                  mcpClients={mcpClients}
                  mcpSetupInfo={mcpSetupInfo}
                  mcpTunnelStatus={mcpTunnelStatus}
                  onOpenMcpSetupPage={(page) =>
                    window.__TAURI_INTERNALS__?.invoke
                      ? window.__TAURI_INTERNALS__.invoke("mcp_open_setup_page", { page }).then(() => undefined)
                      : Promise.reject(new Error(tm("g1.appOpenAiOnlyApp")))
                  }
                  onMcpTunnelConfigure={(tunnelId, runtimeKey) => {
                    const invoke = window.__TAURI_INTERNALS__?.invoke;
                    return invoke
                      ? invoke("mcp_tunnel_configure", { tunnelId, runtimeKey }).then(() => undefined)
                      : Promise.reject(new Error(tm("g1.appTunnelOnlyApp")));
                  }}
                  onMcpTunnelConnect={() => {
                    const invoke = window.__TAURI_INTERNALS__?.invoke;
                    if (!invoke) return Promise.reject(new Error(tm("g1.appTunnelOnlyApp")));
                    const clientId = "audiorouter-chatgpt";
                    return backend
                      .listClients()
                      .then((clients) => {
                        const current = clients.find((client) => client.clientId === clientId);
                        return !current || current.revoked
                          ? backend.authorizeClient(clientId, "observer", uiIdempotencyKey(`authorize-${clientId}`))
                          : undefined;
                      })
                      .then(() => invoke("mcp_tunnel_connect"))
                      .then(() => undefined);
                  }}
                  onMcpTunnelDisconnect={() => {
                    const invoke = window.__TAURI_INTERNALS__?.invoke;
                    return invoke
                      ? invoke("mcp_tunnel_disconnect").then(() => undefined)
                      : Promise.reject(new Error(tm("g1.appTunnelOnlyApp")));
                  }}
                  onExportCoworkPlugin={async () => {
                    const invoke = window.__TAURI_INTERNALS__?.invoke;
                    if (!invoke) throw new Error(tm("g1.appExportPluginDesktop"));
                    const clientId = "audiorouter-cowork-local";
                    const clients = await backend.listClients();
                    const current = clients.find((client) => client.clientId === clientId);
                    if (!current || current.revoked)
                      await backend.authorizeClient(clientId, "observer", uiIdempotencyKey(`authorize-${clientId}`));
                    const path = await invoke("mcp_export_cowork_plugin");
                    if (typeof path !== "string") throw new Error(tm("g1.appExportPluginNoPath"));
                    return path;
                  }}
                  onMcpTunnelForget={() => {
                    const invoke = window.__TAURI_INTERNALS__?.invoke;
                    return invoke
                      ? invoke("mcp_tunnel_forget").then(() => undefined)
                      : Promise.reject(new Error(tm("g1.appTunnelOnlyApp")));
                  }}
                  clientsPanel={(initialClientId) => (
                    <ClientsPanel backend={backend} initialClientId={initialClientId} />
                  )}
                  statusContent={statusWithTroubleshooting}
                  statusHasProblem={
                    setupCheck.result?.findings.some((finding) => finding.severity === "problem") ?? false
                  }
                  virtualCablesContent={cableWorkbenchContent}
                  appsContent={appsWorkbenchContent}
                  uiOptionsContent={uiOptionsWorkbenchContent}
                  timingContent={
                    <LiveTelemetry store={telemetryStore} diagnostics={snapshot?.diagnostics ?? null}>
                      {(liveDiagnostics) => (
                        <SignalTimingPanel
                          session={session}
                          telemetry={liveDiagnostics?.nodeTelemetry ?? []}
                          running={sessionRunning}
                        />
                      )}
                    </LiveTelemetry>
                  }
                  recordingContent={recordingWorkbenchContent}
                  settingsContent={settingsWorkbenchContent}
                  savedVersionsContent={
                    <Suspense fallback={<p role="status">{tm("g1.appLoadingHistory")}</p>}>
                      <ChangeHistory
                        backend={backend}
                        session={session}
                        past={draftHistory.past}
                        current={draft}
                        routeChanged={routeChanged}
                        running={sessionRunning}
                        onUndoSteps={undoDraftSteps}
                        onReverted={refreshAfterPanelAction}
                      />
                    </Suspense>
                  }
                />
              </ErrorBoundary>
            </main>
          </div>
          <LibraryDragOverlay />
          {goalDialogOpen && (
            <Suspense fallback={null}>
              <GoalDialog backend={backend} onCreated={finishGoal} onClose={closeGoalDialog} />
            </Suspense>
          )}
          {tourOpen && (
            <Suspense fallback={null}>
              <FirstMinuteTour onOpenTools={() => setWorkbenchTab("tools")} onClose={() => setTourOpen(false)} />
            </Suspense>
          )}
          {paletteOpen && (
            <Suspense fallback={null}>
              <CommandPalette items={paletteItems()} onClose={() => setPaletteOpen(false)} />
            </Suspense>
          )}
          {sessionSwitchPrompt && (
            <SessionSwitchDialog names={sessionSwitchPrompt.names} onDecision={sessionSwitchPrompt.decide} />
          )}
          {sidechainReplacePrompt && (
            <SidechainReplaceDialog
              pluginName={sidechainReplacePrompt.pluginName}
              currentSourceName={sidechainReplacePrompt.currentSourceName}
              nextSourceName={sidechainReplacePrompt.sourceName}
              onDecision={decideSidechainReplacement}
            />
          )}
          {deviceConsent && (
            <DeviceAccessDialog
              busy={deviceConsent.busy}
              error={deviceConsent.error}
              onAllow={() => void allowDeviceAccessAndPlay()}
              onCancel={() => setDeviceConsent(null)}
            />
          )}
          {keyboardHelpOpen && <KeyboardHelp onClose={() => setKeyboardHelpOpen(false)} />}
          <AppDialogHost />
        </div>
      </SoundboardContext.Provider>
    </PluginParameterContext.Provider>
  );
  return (
    <BackendConnectionContext.Provider value={backendReady}>{renderedWorkspace}</BackendConnectionContext.Provider>
  );
}
