import { useCanvasGroups } from "./CanvasGroups";
import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import type { PluginParametersResult } from "@audiorouter/contracts";
import { LIBRARY_DROP_SOURCE, SessionFlowCanvas } from "./SessionFlowCanvas";
import { SignalTimingPanel } from "./SignalTiming";
import { Workbench, type WorkbenchTab, type McpActivity, type McpSetupInfo } from "./Workbench";
import { RequestBuilder } from "./ApiRequestBuilder";
import { VersionLine } from "./UpdateNotice";
import { isAutomatedHarness, updateCheckEnabled, useUpdateCheck } from "./updateCheck";
import {
  formatUiError,
  isRevisionConflict,
  SnapshotCache,
  type ApplicationRow,
  type RecorderStatus,
  type UiBackend,
} from "./backend";
import type { DeviceListItem } from "@audiorouter/contracts";
import { LiveTelemetry, useTelemetryStore } from "./liveTelemetry";
import { useReportUnsaved } from "./unsavedReport";
import { QuitButton } from "./QuitButton";
import {
  addSourceToOccupiedOutput,
  appendApplicationCaptureNode,
  applicationCaptureChoices,
  applicationChoiceKey,
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
  applyGraphDraft,
  duplicateDraftNode,
  insertDraftMixer,
  insertDraftPluginProcessor,
  insertDraftProcessor,
  removeDraftConnection,
  removeDraftNode,
  removeSinglePathDraftMixer,
  resetNodeDraftParameters,
  routeFedMixerToOccupiedOutput,
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
import { actionMessageTone } from "./actionMessage";
import { unfedRouteNodes } from "./draft";
import { demoSession, demoSessions } from "./fixtures";
import {
  recordDraft,
  redoDraft as redoDraftHistory,
  undoDraft as undoDraftHistory,
  type DraftHistory,
} from "./history";
import { filterLibraryEntries, libraryEntries } from "./library";
import {
  readFlowAnimation,
  readLastSession,
  readShortcuts,
  readSidebarWidth,
  readTheme,
  writeFlowAnimation,
  writeLastSession,
  writeShortcuts,
  writeSidebarWidth,
  writeTheme,
  type FlowAnimationMode,
  type ThemeMode,
} from "./preferences";
import { SidebarResizer } from "./SidebarResizer";
import { SessionFilePanel } from "./SessionFilePanel";
import { announceDeviceAccessChanged, DeviceAccessDialog, isDeviceAccessDenied } from "./DeviceAccess";
import { LibraryDragOverlay } from "./libraryDrag";
import {
  defaultShortcutBinding,
  isEditableShortcutTarget,
  shortcutConflicts,
  shortcutFromKeyboardEvent,
  type ShortcutAction,
  type ShortcutBinding,
} from "./shortcuts";
import { setupChecklist } from "./setup";
import { uiIdempotencyKey } from "./idempotency";
import { processorParameterError, type ProcessorDescriptor } from "./processorCatalog";
import { mergeSessionInventory, reconcileSessionDraft, sameSessionDraft } from "./sessionInventory";
import type { Connection } from "@xyflow/react";
import { decodeTopologyAction } from "./DraftConnectionList";
import { BackendConnectionContext } from "./backendConnectionContext";
import {
  appendClientDiagnostic,
  browserDiagnosticStorage,
  onRpcFailure,
  readClientDiagnostics,
} from "./clientDiagnostics";
import { ErrorBoundary, RootRecoveryPanel } from "./ErrorBoundary";
import { EqBackendContext, PluginParameterContext, defaultBackend } from "./appContext";
import { findVbCableEndpointPair, readEndpointBindingHint, routeEndpointBinding } from "./endpointBinding";

import { ClientsPanel } from "./AdvancedPanels";
import { LoadedPluginsPanel, PluginScanPanel, PluginToolsGroup } from "./PluginPanels";
import { ApplicationChoiceOptions } from "./NodeEditors";
export { formatRecordingDuration, recorderHasCaptureSource } from "./RecordingPanels";
export { findVbCableEndpointPair, findVbCableCaptureEndpointId, deviceChoiceLabel } from "./endpointBinding";
import { useAudioFileStatus } from "./useAudioFileStatus";
import { WORKSPACE_EVENT_CATEGORIES, useWorkspaceEvents } from "./useWorkspaceEvents";
import { DIAGNOSTICS_REFRESH_INTERVAL_MS, useDiagnosticsRefresh } from "./useDiagnosticsRefresh";
import { formatNativePumpSummary, useNativeCounters, type NativePumpStats } from "./useNativeCounters";
import { ConnectionForm } from "./ConnectionForm";
import { SetupWorkbench } from "./SetupWorkbench";
import { DeviceTroubleshooting } from "./DeviceTroubleshooting";
import { RecordingWorkbench } from "./RecordingWorkbench";
import { AdvancedWorkbench } from "./AdvancedWorkbench";
import { PropertiesPanel } from "./PropertiesPanel";
export { WORKSPACE_EVENT_CATEGORIES, DIAGNOSTICS_REFRESH_INTERVAL_MS, formatNativePumpSummary };
export { NATIVE_COUNTERS_REFRESH_MS } from "./useNativeCounters";

/** Root failures outlive AppContent's state, so they go straight to storage for the next window. */
function recordStoredUiDiagnostic(message: string) {
  const storage = browserDiagnosticStorage();
  appendClientDiagnostic(storage, readClientDiagnostics(storage), message);
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
  const [snapshotCache] = useState(() => new SnapshotCache());
  const [snapshotState, setSnapshotState] = useState(snapshotCache.current());
  const [rememberedSessionId] = useState(() => readLastSession(browserDiagnosticStorage()));
  const [selectedSessionId, setSelectedSessionId] = useState(rememberedSessionId ?? demoSession.id);
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
  const authoritativeSession = useRef(demoSession);
  const hasAuthoritativeSession = useRef(!backend.connected);
  const [draftHistory, setDraftHistory] = useState<DraftHistory>({ past: [], future: [] });
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [recordings, setRecordings] = useState<import("@audiorouter/contracts").RecordingRow[]>([]);
  const [recorderFormat, setRecorderFormat] = useState<import("@audiorouter/contracts").RecorderFileFormat>("wavPcm24");
  const [recorderStatuses, setRecorderStatuses] = useState<RecorderStatus[]>([]);
  const [audioSourceStates, setAudioSourceStates] = useState<Record<string, "playing" | "paused" | "stopped">>({});
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
  const [previewMessage, setPreviewMessage] = useState<string | null>(null);
  const [recoveryMessage, setRecoveryMessage] = useState<string | null>(null);
  const previewRequest = useRef(0);
  const recoveryRequest = useRef(0);
  const [metadataTitles, setMetadataTitles] = useState<Record<string, string>>({});
  const [metadataArtists, setMetadataArtists] = useState<Record<string, string>>({});
  const [metadataComments, setMetadataComments] = useState<Record<string, string>>({});
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
  const recordingMutationBusy = useRef(false);
  const [recordingMutationBusyState, setRecordingMutationBusyState] = useState(false);
  const [privacyMuted, setPrivacyMuted] = useState(true);
  const [librarySearch, setLibrarySearch] = useState("");
  const [processors, setProcessors] = useState<ProcessorDescriptor[] | null>(null);
  const [processorError, setProcessorError] = useState<string | null>(null);
  const [pluginParameters, setPluginParameters] = useState<PluginParametersResult | null>(null);
  const [pluginParameterError, setPluginParameterError] = useState<string | null>(null);
  const [presets, setPresets] = useState<import("@audiorouter/contracts").DiscoveryDocument["presets"] | null>(null);
  const [presetError, setPresetError] = useState<string | null>(null);
  const [theme, setTheme] = useState<ThemeMode>(() =>
    readTheme(typeof window === "undefined" ? null : window.localStorage),
  );
  const [flowAnimation, setFlowAnimation] = useState<FlowAnimationMode>(() =>
    readFlowAnimation(typeof window === "undefined" ? null : window.localStorage),
  );
  const changeFlowAnimation = (mode: FlowAnimationMode) => {
    setFlowAnimation(mode);
    writeFlowAnimation(typeof window === "undefined" ? null : window.localStorage, mode);
  };
  const [sidebarWidth, setSidebarWidth] = useState(() =>
    readSidebarWidth(typeof window === "undefined" ? null : window.localStorage),
  );
  const changeSidebarWidth = (width: number) => {
    setSidebarWidth(width);
    writeSidebarWidth(typeof window === "undefined" ? null : window.localStorage, width);
  };
  const groupState = useCanvasGroups(selectedSessionId);
  const selectedGroup = groupState.groups.find((group) => group.id === groupState.selectedGroupId);
  const [workbenchTab, setWorkbenchTab] = useState<WorkbenchTab>("tools");
  // UI-18: optional daily new-version check (GitHub public releases list).
  const [updateChecks, setUpdateChecks] = useState(updateCheckEnabled);
  const availableUpdate = useUpdateCheck(updateChecks && !isAutomatedHarness());
  const selectNodeProperties = (id: string) => {
    setSelectedNodeId(id);
    setSelectedNodeIds([id]);
    setWorkbenchTab("properties");
  };
  const diagnosticStorage = browserDiagnosticStorage();
  const uiDiagnosticsRef = useRef<string[]>(readClientDiagnostics(diagnosticStorage));
  const [uiDiagnostics, setUiDiagnostics] = useState<string[]>(uiDiagnosticsRef.current);
  const [mcpActivity, setMcpActivity] = useState<McpActivity[]>([]);
  const [mcpSetupInfo, setMcpSetupInfo] = useState<McpSetupInfo | null>(null);
  const [backendActivity, setBackendActivity] = useState<Record<string, unknown>[]>([]);
  const [shortcuts, setShortcuts] = useState<ShortcutBinding>(() =>
    readShortcuts(typeof window === "undefined" ? null : window.localStorage, defaultShortcutBinding),
  );
  const [shortcutMessage, setShortcutMessage] = useState<string | null>(null);
  const [connectionSource, setConnectionSource] = useState("");
  const [connectionDestination, setConnectionDestination] = useState("");
  const [connectionReplacement, setConnectionReplacement] = useState<{
    sessionId: string;
    edgeId: string;
    sourceNode: string;
    sourcePort: string;
    destinationNode: string;
    destinationPort: string;
  } | null>(null);
  const [connectionDialogOpen, setConnectionDialogOpen] = useState(false);
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
  const [nativeGenerations, setNativeGenerations] = useState<Record<string, { generation: number; kind: string }>>({});
  const [nativePumpStats, setNativePumpStats] = useState<NativePumpStats | null>(null);
  const eventCursor = useRef({ backendEpoch: 0, sequence: 0 });
  const applicationRefreshGeneration = useRef(0);
  const deviceRefreshGeneration = useRef(0);
  const sessionRefreshGeneration = useRef(0);
  const recordingRefreshGeneration = useRef(0);
  const recorderRefreshGeneration = useRef(0);
  useEffect(() => {
    let mounted = true;
    void snapshotCache.refresh(backend).then((nextState) => {
      if (mounted) {
        setSnapshotState(nextState);
        if (nextState.snapshot && !rememberedSessionId)
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
  const refreshApplications = () => {
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
  const refreshDevices = ({ announce = true }: { announce?: boolean } = {}) => {
    const generation = ++deviceRefreshGeneration.current;
    if (announce) setActionMessage("Refreshing audio endpoints...");
    void backend
      .listDevices()
      .then((items) => {
        if (generation !== deviceRefreshGeneration.current) return;
        setDevices(items);
        setDevicesError(null);
        if (announce)
          setActionMessage(
            items.length === 0
              ? "Audio endpoint refresh completed: the backend returned no endpoints."
              : `Audio endpoint refresh completed: ${items.length} endpoint${items.length === 1 ? "" : "s"} found.`,
          );
      })
      .catch((error) => {
        if (generation !== deviceRefreshGeneration.current) return;
        setDevices([]);
        const message = formatUiError(error, "Device inventory unavailable");
        setDevicesError(message);
        if (announce) setActionMessage(`Audio endpoint refresh failed: ${message}`);
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
          items.some((item) => item.id === current) ? current : (items[0]?.id ?? current),
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
        const text = result.reason ?? "Recording stopped saving early. The file keeps everything up to that point.";
        setRecordingMessage({ nodeId, text });
        setActionMessage(text);
      } else {
        setRecordingMessage(null);
        setActionMessage(record ? "Recording started." : "Recording saved.");
      }
      refreshRecordings(session.id);
    } catch (error) {
      const text = formatUiError(error, record ? "Recording could not start." : "Recording could not stop.");
      setRecordingMessage({ nodeId, text });
      setActionMessage(text);
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
    if (currentSessionIdRef.current !== requestedSessionId) return;
    setSnapshotState(nextState);
    await refreshSessions();
    refreshRecordings(session.id);
    refreshRecorders();
    refreshApplications();
    refreshDevices({ announce: false });
  };
  const snapshot = snapshotState.snapshot;
  const hasSnapshot = snapshot !== null;
  const telemetryStore = useTelemetryStore();
  const diagnosticsRef = useRef(snapshot?.diagnostics ?? null);
  diagnosticsRef.current = snapshot?.diagnostics ?? null;
  const recordUiDiagnostic = (message: string) => {
    const next = appendClientDiagnostic(diagnosticStorage, uiDiagnosticsRef.current, message);
    uiDiagnosticsRef.current = next;
    setUiDiagnostics(next);
  };
  useEffect(() => {
    writeTheme(typeof window === "undefined" ? null : window.localStorage, theme);
  }, [theme]);
  useEffect(() => {
    const onError = (event: ErrorEvent) => {
      const rawName = event.error instanceof Error ? event.error.name : "unknown";
      const name = /^[A-Za-z][A-Za-z0-9_.-]{0,47}$/.test(rawName) ? rawName : "Error";
      const rawScript = event.filename.split(/[?#]/, 1)[0].split(/[\\/]/).pop() ?? "";
      const script = rawScript.replace(/[^A-Za-z0-9_.-]/g, "_").slice(0, 64);
      const location = script ? ` at ${script}:${event.lineno}:${event.colno}` : "";
      recordUiDiagnostic(`UI error (${name})${location}`);
    };
    const onReject = (event: PromiseRejectionEvent) => {
      const rawType = event.reason instanceof Error ? event.reason.name : typeof event.reason;
      const reasonType = /^[A-Za-z][A-Za-z0-9_.-]{0,47}$/.test(rawType) ? rawType : "Error";
      recordUiDiagnostic(`Unhandled promise rejection (${reasonType})`);
    };
    window.addEventListener("error", onError);
    window.addEventListener("unhandledrejection", onReject);
    return () => {
      window.removeEventListener("error", onError);
      window.removeEventListener("unhandledrejection", onReject);
    };
  }, []);
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
  useEffect(() => onRpcFailure(recordUiDiagnostic), []);
  useEffect(() => {
    if ((workbenchTab !== "mcp" && workbenchTab !== "diagnostics") || !window.__TAURI_INTERNALS__?.invoke) return;
    let active = true;
    void window.__TAURI_INTERNALS__
      .invoke("mcp_setup_info")
      .then((info) => {
        if (active) setMcpSetupInfo(info as McpSetupInfo);
      })
      .catch(() => {
        if (active) setMcpSetupInfo(null);
      });
    void window.__TAURI_INTERNALS__
      .invoke("backend_diagnostics_list")
      .then((items) => {
        if (active && Array.isArray(items)) setBackendActivity(items as Record<string, unknown>[]);
      })
      .catch(() => {
        if (active) setBackendActivity([]);
      });
    const refreshActivity = () => {
      void window.__TAURI_INTERNALS__
        ?.invoke?.("mcp_activity_list")
        .then((items) => {
          if (active && Array.isArray(items)) setMcpActivity(items as McpActivity[]);
        })
        .catch((error: unknown) => {
          if (active) {
            const type = error instanceof Error ? error.name.slice(0, 48) : typeof error;
            recordUiDiagnostic(`MCP activity read failed (${type})`);
          }
        });
      void window.__TAURI_INTERNALS__
        ?.invoke?.("backend_diagnostics_list")
        .then((items) => {
          if (active && Array.isArray(items)) setBackendActivity(items as Record<string, unknown>[]);
        })
        .catch(() => undefined);
    };
    refreshActivity();
    const timer = window.setInterval(refreshActivity, 2000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [workbenchTab]);
  useEffect(() => {
    writeShortcuts(typeof window === "undefined" ? null : window.localStorage, shortcuts);
  }, [shortcuts]);
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
  const availableSessions = mergeSessionInventory(listedSessions, snapshot?.session ?? null, createdSessions);
  const session =
    availableSessions.find((item) => item.id === selectedSessionId) ?? availableSessions[0] ?? demoSession;
  const sessionIsAvailable = availableSessions.some((item) => item.id === session.id);
  useEffect(() => {
    if (!backend.connected || !backend.setActiveSession || !sessionIsAvailable) return;
    void backend
      .setActiveSession(session.id)
      .catch((error) => setSessionInventoryError(formatUiError(error, "Active session selection unavailable")));
  }, [backend, session.id, sessionIsAvailable]);
  useEffect(() => {
    if (availableSessions.some((item) => item.id === selectedSessionId)) {
      writeLastSession(browserDiagnosticStorage(), selectedSessionId);
    }
  }, [selectedSessionId, listedSessions, snapshot?.session, createdSessions]);
  currentSessionIdRef.current = session.id;
  const selectedNode = draft.nodes.find((node) => node.id === selectedNodeId) ?? draft.nodes[0];
  useEffect(() => {
    recordUiDiagnostic(
      `Graph checkpoint: ${draft.nodes.length} nodes, ${draft.edges.length} connections; revision ${session.revision}`,
    );
  }, [draft.nodes.length, draft.edges.length, session.revision]);
  const sessionRunning = snapshot?.status.activeSessionIds.includes(session.id) ?? false;
  const routeChanged = !sameSessionDraft(draft, session);
  // Lets the shell free the WebView on close only when nothing would be lost.
  useReportUnsaved(routeChanged);
  useEffect(() => {
    setAudioSourceStates({});
  }, [session.id]);
  useEffect(() => {
    if (selectedNode?.kind === "applicationCapture" && backend.connected) refreshApplications();
  }, [selectedNode?.id, selectedNode?.kind, backend.connected]);
  useAudioFileStatus({
    backend,
    draft,
    session,
    sessionRunning,
    audioSourceStates,
    setAudioSourceStates,
    recordUiDiagnostic,
  });
  const testSignalPlaybackReady = backend.connected && sameSessionDraft(draft, session);
  const testSignalEndpointPrepared =
    snapshot?.diagnostics.nativeSessionId === session.id &&
    snapshot.diagnostics.nativeAdapter === "configured-stopped" &&
    snapshot.diagnostics.nativeAdapterKind === "endpoint";
  useEffect(() => {
    const hint = readEndpointBindingHint(session.id);
    setCaptureEndpointId(hint.captureEndpointId ?? "");
    setRenderEndpointId(hint.renderEndpointId ?? "");
  }, [session.id]);
  useEffect(() => {
    if (backend.connected && !snapshot && availableSessions.length === 0) return;
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
      setActionMessage(
        "This session changed elsewhere. Your draft is preserved. In Session, discard the draft to load the saved graph, or copy your edits before resolving the revision conflict.",
      );
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
  }, [session]);
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
      setPluginParameterError("The plugin placeholder is missing its verified path or fingerprint.");
      return;
    }
    let active = true;
    setPluginParameters(null);
    setPluginParameterError(null);
    void backend
      .describePluginParameters(path)
      .then((result) => {
        if (!active) return;
        if (result.sha256 !== fingerprint) {
          setPluginParameterError("Plugin identity changed; scan it again before editing parameters.");
          return;
        }
        setPluginParameters(result);
      })
      .catch((error) => {
        if (active) setPluginParameterError(formatUiError(error, "Plugin parameters unavailable."));
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
            items.some((item) => item.id === current) ? current : (items[0]?.id ?? current),
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
  useEffect(() => {
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
  }, [backend]);
  useEffect(() => {
    let active = true;
    void backend
      .listDevices()
      .then((items) => {
        if (active) {
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
    setSelectedSessionId,
    setActionMessage,
    refreshApplications,
    refreshDevices,
    refreshSessions,
    setRecordings,
    setRecordingsError,
    setRecorderStatuses,
    setRecorderStatusAvailable,
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
  const visibleLibraryEntries = filterLibraryEntries(libraryEntries, librarySearch);
  const setupSteps = setupChecklist({
    connected: backend.connected,
    audio: snapshot?.status.audio ?? null,
    storage: snapshot?.status.storage ?? null,
    deviceCount: devices.length,
    applicationCount: applications.length,
    vbCablePairAvailable: findVbCableEndpointPair(devices) !== null,
  });
  const connectionLabel = backend.connected ? "Backend ready" : "Backend unavailable";
  const nativePumpSummary = formatNativePumpSummary(nativePumpStats, sessionRunning);
  const schedulerTelemetry = snapshot?.diagnostics.schedulerTelemetry;
  const schedulerSummary = schedulerTelemetry
    ? ` - ${schedulerTelemetry.processedQuanta} quanta / ${schedulerTelemetry.xruns} xruns`
    : "";
  const statusSummary = `${snapshot ? `${snapshot.status.audio} audio (${snapshot.status.reason}) - ${snapshot.status.storage} storage - ${snapshot.status.sessionCount} session${snapshot.status.sessionCount === 1 ? "" : "s"}` : "Waiting for backend snapshot"}${schedulerSummary}${nativePumpSummary ? ` - ${nativePumpSummary}` : ""}${sessionCrudBusyState ? " - Updating session..." : ""}`;
  const editGroup = useRef<{ key: string; time: number } | null>(null);
  const recordDraftChange = (next: import("@audiorouter/contracts").Session, group?: string) => {
    const previous = draftRef.current;
    if (sameSessionDraft(previous, next)) return;
    const now = Date.now();
    const coalesce = group && editGroup.current?.key === group && now - editGroup.current.time < 750;
    editGroup.current = group ? { key: group, time: now } : null;
    draftRef.current = next;
    setDraftHistory((history) =>
      coalesce && history.past.length > 0 ? { ...history, future: [] } : recordDraft(history, previous, next),
    );
    setDraft(next);
    setConnectionReplacement(null);
    setPendingWarnings([]);
    setAcknowledgedWarnings(new Set());
    setPendingOperation(null);
    setPendingGraphPlan(null);
  };
  const restoreDraftHistory = (direction: "undo" | "redo") => {
    if (!backend.connected || graphBusy || sessionActionBusy) return;
    editGroup.current = null;
    const current = draftRef.current;
    const transition =
      direction === "undo" ? undoDraftHistory(draftHistory, current) : redoDraftHistory(draftHistory, current);
    if (transition.current === current) return;
    // History restores content; all subsequent commits use today's revision.
    const restored = { ...transition.current, revision: current.revision };
    setDraftHistory(transition.history);
    draftRef.current = restored;
    setDraft(restored);
    setConnectionReplacement(null);
    setPendingWarnings([]);
    setAcknowledgedWarnings(new Set());
    setPendingOperation(null);
    setPendingGraphPlan(null);
    setActionMessage(direction === "undo" ? "Undid the last change." : "Redid the change.");
  };
  const undoDraft = () => restoreDraftHistory("undo");
  const redoDraft = () => restoreDraftHistory("redo");
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
      restoreDraftHistory(key === "y" || event.shiftKey ? "redo" : "undo");
    };
    window.addEventListener("keydown", onHistoryKey);
    return () => window.removeEventListener("keydown", onHistoryKey);
  }, [backend.connected, graphBusy, sessionActionBusy, draftHistory]);
  const changeNodeFlag = async (flag: "enabled" | "bypass", value: boolean) => {
    if (sessionBusy.current || nodeFlagBusy.current || graphBusy || !backend.connected) return;
    const next = setNodeDraftFlag(draftRef.current, selectedNode.id, flag, value);
    const wasRunning = sessionRunning;
    if (!wasRunning) {
      recordDraftChange(next);
      setActionMessage("Node state changed in the draft. Save the route before pressing Play.");
      return;
    }
    if (!session.nodes.some((node) => node.id === selectedNode.id)) {
      setActionMessage("This new tool is not playing yet. Save the route to add it before changing its live state.");
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
      setActionMessage(
        formatUiError(
          error,
          "The node change could not be applied. Audio is still playing with its previous settings.",
        ),
      );
    } finally {
      nodeFlagBusy.current = false;
      setGraphBusy(false);
    }
  };
  const changeNodeName = (name: string) => {
    try {
      recordDraftChange(setNodeDraftName(draft, selectedNode.id, name));
      setActionMessage("Name changed. Save to keep it.");
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to rename node."));
    }
  };
  const changeNodeParameterOn = (nodeId: string, name: string, value: boolean | number | string) => {
    const targetNode = draftRef.current.nodes.find((candidate) => candidate.id === nodeId);
    if (!targetNode) return;
    const error =
      targetNode.kind === "audioFile"
        ? !(name === "mediaId" && typeof value === "string" && value.length <= 128) &&
          !(name === "fileName" && typeof value === "string" && value.length <= 255) &&
          !(name === "loop" && typeof value === "boolean")
          ? "Invalid audio source setting"
          : null
        : targetNode.kind === "plugin"
          ? (() => {
              const id = Number(name.slice("pluginParameter:".length));
              const descriptor = pluginParameters?.parameters.find((parameter) => parameter.parameterId === id);
              return !descriptor ||
                typeof value !== "number" ||
                !Number.isFinite(value) ||
                value < descriptor.minimum ||
                value > descriptor.maximum
                ? "Plugin parameter value is outside the worker-provided range"
                : null;
            })()
          : processorParameterError(processors, targetNode.kind, name, value, snapshot?.discovery?.nodeTypes ?? null);
    if (error) {
      setActionMessage(`Draft rejected: ${error}.`);
      return;
    }
    recordDraftChange(setNodeDraftParameter(draftRef.current, nodeId, name, value), nodeId + ":" + name);
    setActionMessage("Draft updated. Review and plan the changes before committing.");
  };
  const changeNodeParameter = (name: string, value: boolean | number | string) =>
    changeNodeParameterOn(selectedNode.id, name, value);
  const resetNodeParameters = () => {
    recordDraftChange(resetNodeDraftParameters(draft, selectedNode.id));
    setActionMessage("Processor parameters reset in the draft. Review and plan the changes before committing.");
  };
  const changeSessionName = (name: string) => {
    try {
      recordDraftChange(setSessionDraftName(draft, name));
      setActionMessage("Session name changed. Save to keep it.");
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to rename session."));
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
      if (native?.state === "applied" && activation?.state === "running") {
        const nextGeneration = activation.generation;
        setNativeGenerations((current) =>
          current[saved.id]
            ? { ...current, [saved.id]: { ...current[saved.id], generation: nextGeneration } }
            : current,
        );
      }
      setActionMessage(
        native?.state === "applied"
          ? `Saved (revision ${revision}) and applied to the playing audio.`
          : native?.state === "restarted"
            ? `Saved (revision ${revision}). The route changed, so audio restarted automatically with the new connections.`
            : native?.state === "restartRequired"
              ? `Saved (revision ${revision}), but the playing audio could not take this change: ${native.reason}`
              : `Route saved (revision ${revision}). Prepare devices before playing.`,
      );
    }
    recordUiDiagnostic(
      `Graph commit succeeded: revision ${revision}; ${submitted.nodes.length} nodes, ${submitted.edges.length} connections`,
    );
    await refresh();
  };
  const planChanges = async () => {
    if (graphBusy || !backend.connected) return;
    setGraphBusy(true);
    setActionMessage("Checking route before saving...");
    try {
      const operation = uiIdempotencyKey("graph-commit");
      const plan = await backend.planGraph(draft);
      if (currentSessionIdRef.current !== draft.id || !sameSessionDraft(draftRef.current, draft)) {
        setActionMessage("The draft changed during review. Plan changes again before committing.");
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
        setActionMessage(
          "Review the warnings in Session, then choose Confirm and save route. Nothing has been saved yet.",
        );
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
        setActionMessage(formatUiError(error, "Graph changed elsewhere."));
      } else setActionMessage(formatUiError(error, "Unable to apply graph changes."));
    } finally {
      setGraphBusy(false);
    }
  };
  // While audio plays, slider-style edits (parameter values only) are saved
  // after a short pause so the change is heard without Stop/Play. Topology
  // edits still wait for an explicit Save.
  const lastAutoSaveDraft = useRef<string | null>(null);
  const planChangesRef = useRef(planChanges);
  planChangesRef.current = planChanges;
  useEffect(() => {
    if (
      !sessionRunning ||
      graphBusy ||
      !backend.connected ||
      pendingGraphPlan ||
      !isParameterOnlyChange(session, draft)
    )
      return;
    const key = JSON.stringify(draft);
    if (lastAutoSaveDraft.current === key) return;
    const timer = window.setTimeout(() => {
      lastAutoSaveDraft.current = key;
      void planChangesRef.current();
    }, 400);
    return () => window.clearTimeout(timer);
  }, [draft, session, sessionRunning, graphBusy, backend.connected, pendingGraphPlan]);
  const commitAcknowledgedPlan = async () => {
    if (
      graphBusy ||
      !backend.connected ||
      !pendingOperation ||
      !pendingGraphPlan ||
      acknowledgedWarnings.size !== pendingWarnings.length
    )
      return;
    setGraphBusy(true);
    setActionMessage("Saving route...");
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
        setActionMessage(formatUiError(error, "Graph changed elsewhere."));
      } else setActionMessage(formatUiError(error, "Unable to commit acknowledged changes."));
    } finally {
      setGraphBusy(false);
    }
  };
  const previewRecording = async (recordingId: string) => {
    const request = ++previewRequest.current;
    setPreviewMessage("Inspecting recording...");
    try {
      const result = await backend.previewRecording(recordingId);
      if (request === previewRequest.current)
        setPreviewMessage(`${String(result.preview.status)} recording preview loaded.`);
    } catch (error) {
      if (request === previewRequest.current) setPreviewMessage(formatUiError(error, "Recording preview unavailable."));
    }
  };
  const inspectRecovery = async (recordingId: string) => {
    const request = ++recoveryRequest.current;
    setRecoveryMessage("Inspecting recorder recovery...");
    try {
      const result = await backend.getRecordingRecovery(recordingId);
      if (request === recoveryRequest.current)
        setRecoveryMessage(
          result.status === "missing"
            ? "No persisted recovery checkpoint is available."
            : `Recovery checkpoint: ${result.checkpoint.state}.`,
        );
    } catch (error) {
      if (request === recoveryRequest.current)
        setRecoveryMessage(formatUiError(error, "Recording recovery unavailable."));
    }
  };
  const saveRecordingMetadata = async (recording: import("@audiorouter/contracts").RecordingRow) => {
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      const title = metadataTitles[recording.id]?.trim() ?? recording.title ?? "";
      const artist = metadataArtists[recording.id]?.trim() ?? recording.artist ?? "";
      const comment = metadataComments[recording.id]?.trim() ?? recording.comment ?? "";
      await backend.setRecordingMetadata(recording.id, {
        title: title || null,
        artist: artist || null,
        comment: comment || null,
        idempotencyKey: uiIdempotencyKey("recording-metadata"),
      });
      setRecordings((current) =>
        current.map((item) =>
          item.id === recording.id
            ? { ...item, title: title || null, artist: artist || null, comment: comment || null }
            : item,
        ),
      );
      setPreviewMessage("Recording metadata saved; the audio file was unchanged.");
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to save recording metadata."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const removeRecordingEntry = async (recordingId: string) => {
    if (!window.confirm("Remove this library entry? The audio file will be preserved.")) return;
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      await backend.removeRecordingEntry(recordingId, uiIdempotencyKey("recording-entry-remove"));
      setRecordings((current) => current.filter((item) => item.id !== recordingId));
      setPreviewMessage("Library entry removed; the audio file was preserved.");
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to remove recording entry."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const renameRecording = async (recordingId: string, newPath: string) => {
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      const result = await backend.renameRecording(recordingId, newPath.trim(), uiIdempotencyKey("recording-rename"));
      setRecordings((current) =>
        current.map((item) => (item.id === recordingId ? { ...item, path: result.path, missing: false } : item)),
      );
      setPreviewMessage("Recording renamed within the approved directory.");
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to rename recording."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const revealRecording = async (recordingId: string) => {
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      const result = await backend.revealRecording(recordingId);
      setPreviewMessage(
        result.revealed
          ? "Recording revealed by the operating system."
          : "Recording is missing; no operating-system action was performed.",
      );
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to reveal recording."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const recycleRecording = async (recordingId: string, confirm: boolean) => {
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      const result = await backend.recycleRecording(
        recordingId,
        confirm,
        confirm ? uiIdempotencyKey("recording-recycle") : undefined,
      );
      setPreviewMessage(
        result.fileAction === "recycled"
          ? "Recording recycled."
          : result.fileAction === "recycle"
            ? "Recycle preview loaded; confirmation is still required."
            : `Recording was not recycled: ${result.reason}.`,
      );
      if (result.fileAction === "recycled")
        setRecordings((current) =>
          current.map((item) => (item.id === recordingId ? { ...item, missing: true } : item)),
        );
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to recycle recording."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const togglePrivacyMute = async () => {
    if (safetyActionBusy.current || !backend.connected) return;
    safetyActionBusy.current = true;
    setSafetyActionBusyState(true);
    const next = !privacyMuted;
    setActionMessage(next ? "Enabling privacy mute..." : "Disabling privacy mute...");
    try {
      await backend.setPrivacyMute(next, uiIdempotencyKey("privacy-mute"));
      setPrivacyMuted(next);
      setActionMessage(next ? "Privacy mute enabled." : "Privacy mute disabled.");
    } catch (error) {
      setPrivacyMuted(true);
      setActionMessage(formatUiError(error, "Unable to change privacy mute."));
    } finally {
      safetyActionBusy.current = false;
      setSafetyActionBusyState(false);
    }
  };
  const clearRecoverySafeMode = async () => {
    if (safetyActionBusy.current || !backend.connected) return;
    safetyActionBusy.current = true;
    setSafetyActionBusyState(true);
    setActionMessage("Clearing recovery safe mode...");
    try {
      await backend.clearRecoverySafeMode(uiIdempotencyKey("recovery-clear"));
      await refresh();
      setActionMessage("Recovery safe mode cleared.");
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to clear recovery safe mode."));
    } finally {
      safetyActionBusy.current = false;
      setSafetyActionBusyState(false);
    }
  };
  const createSession = async () => {
    if (sessionCrudBusy.current || !backend.connected) return;
    const name = window.prompt("New session name", "New session")?.trim();
    if (!name) return;
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
      setSelectedSessionId(result.session.id);
      setActionMessage(`Created stopped session ${result.session.name}.`);
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to create session."));
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
    const name = `${session.name} (copy)`;
    try {
      const result = await backend.duplicateSession(session.id, id, name, uiIdempotencyKey("session-duplicate"));
      setCreatedSessions((current) => [...current, result.session]);
      setSelectedSessionId(result.session.id);
      setActionMessage(`Duplicated stopped session ${result.session.name}.`);
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to duplicate session."));
    } finally {
      sessionCrudBusy.current = false;
      setSessionCrudBusyState(false);
    }
  };
  const deleteSession = async () => {
    if (sessionCrudBusy.current || !backend.connected || !window.confirm(`Delete stopped session “${session.name}”?`))
      return;
    sessionCrudBusy.current = true;
    setSessionCrudBusyState(true);
    try {
      await backend.deleteSession(session.id, uiIdempotencyKey("session-delete"));
      setCreatedSessions((current) => current.filter((item) => item.id !== session.id));
      const fallback = availableSessions.find((item) => item.id !== session.id);
      if (fallback) setSelectedSessionId(fallback.id);
      setActionMessage(`Deleted session ${session.name}.`);
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to delete session."));
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
      setDeviceConsent({ busy: false, error: formatUiError(error, "AudioRouter could not save your choice.") });
    }
  };
  const startSession = async (): Promise<boolean> => {
    if (sessionBusy.current || !backend.connected) return false;
    if (graphBusy) {
      setActionMessage("Wait for the current route check to finish before starting audio.");
      return false;
    }
    sessionBusy.current = true;
    setSessionActionBusy(true);
    setActionMessage("Checking audio endpoints...");
    setNativePumpStats(null);
    try {
      let diagnostics = await backend.refreshDiagnostics();
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
          setActionMessage(
            "No audio started. Select the speaker or headphone device in the Output Device node's Properties, then press Play.",
          );
          return false;
        }
        if (!sameSessionDraft(draftRef.current, session)) {
          setActionMessage(
            generatedOnly
              ? "No audio started. Save the route, then press Play: a Test Signal or Audio File route plays its saved version, without any input device."
              : "No audio started. Save the route, then press Play: a session with several paths plays its saved route.",
          );
          return false;
        }
        const unbound = unboundDeviceNodes(session);
        if (unbound.length > 0) {
          setActionMessage(
            `No audio started. Choose the device for ${unbound.map((node) => node.name).join(", ")} in Properties, save, then press Play.`,
          );
          return false;
        }
        if (!backend.prepareNativePaths) {
          setActionMessage("No audio started. This backend cannot prepare a session with several paths.");
          return false;
        }
        if (diagnostics.nativeSessionId === session.id && backend.detachNativeEndpoint)
          await backend.detachNativeEndpoint(session.id);
        setActionMessage("Preparing every path of this session...");
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
          (node) => !["physicalInput", "applicationCapture", "testSignal", "audioFile"].includes(node.kind),
        );
        const microphones = mixerSources.filter((node) => node.kind === "physicalInput");
        if (unsupported.length > 0) {
          setActionMessage(
            `No audio started. A Mixer that includes an application can combine applications, Test Signals, audio files, and one input device. Turn off ${unsupported.map((node) => node.name).join(", ")}, save, then press Play.`,
          );
          return false;
        }
        if (microphones.length > 1) {
          setActionMessage(
            "No audio started. A Mixer that includes an application can use one input device at a time. Turn off the extra input devices, save, then press Play.",
          );
          return false;
        }
        if (!sameSessionDraft(draftRef.current, session)) {
          setActionMessage(
            "No audio started. Save the route, then press Play: application capture runs the saved route.",
          );
          return false;
        }
        const endpoints = routeEndpointBinding(draftRef.current, session.id);
        if (!endpoints.renderEndpointId || (microphones.length === 1 && !endpoints.captureEndpointId)) {
          setActionMessage(
            !endpoints.renderEndpointId
              ? "No audio started. Select the speaker or headphone device in Physical Output Properties, then press Play."
              : "No audio started. Select the input device in Physical Input Properties, then press Play.",
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
          setActionMessage(
            `No audio started. Select a specific running application for ${unbound.name} in its Properties, save, then press Play.`,
          );
          return false;
        }
        if (!backend.prepareNativeMultiInputs || !backend.prepareNativeOutputs) {
          setActionMessage("No audio started. This backend cannot prepare a Mixer with application sources.");
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
            node.kind === "testSignal" || node.kind === "audioFile"
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
          setActionMessage(`Preparing ${bindings.length} Mixer sources...`);
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
          others
            ? `No audio started. Play can capture ${executable} only when it is the route's only enabled source. Turn off or remove the other sources (${others}), save, then press Play. Mixing an application with other sources is available only from Many-input routing in Full workspace, without automatic reconnect.`
            : `No audio started. Select a specific running ${executable} in the node's Properties, save, then press Play.`,
        );
        recordUiDiagnostic("Session start blocked: application capture mixed with other sources");
        return false;
      }
      if (applicationSource) {
        // An application-only route runs on the process-loopback worker, which
        // also reconnects the app after it restarts. It binds the saved node.
        if (!sameSessionDraft(draftRef.current, session)) {
          setActionMessage(
            "No audio started. Save the route, then press Play: application capture runs the saved route.",
          );
          return false;
        }
        const outputEndpointId = routeEndpointBinding(draftRef.current, session.id).renderEndpointId;
        if (!outputEndpointId) {
          setActionMessage(
            "No audio started. Select the speaker or headphone device in Physical Output Properties, then press Play.",
          );
          return false;
        }
        if (!backend.prepareNativeApplication) {
          setActionMessage("No audio started. This backend cannot prepare application capture.");
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
          setActionMessage(`Preparing ${executable} audio capture...`);
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
          setActionMessage(
            !binding.renderEndpointId
              ? "No audio started. Select the speaker or headphone device in Physical Output Properties, then press Play."
              : "No audio started. This route also needs an input device: add an Input Device node, choose its device in Properties and connect it, or choose one in Advanced → Troubleshooting. Then press Play.",
          );
          recordUiDiagnostic("Session start blocked: exact endpoint selection is incomplete");
          return false;
        }
        if (!backend.prepareNativeEndpoint) {
          setActionMessage(
            "No audio started. This backend cannot prepare the selected audio devices. See Advanced → Troubleshooting for details.",
          );
          return false;
        }
        setActionMessage("Preparing the selected audio devices...");
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
          setActionMessage("Refreshing the selected audio devices...");
          await backend.rebindNativeEndpoint(session.id, binding.captureEndpointId, binding.renderEndpointId);
        }
      }
      // Prepare the exact visible draft for this run. The backend validates it
      // and publishes it only to the in-memory native graph; no graph revision
      // is committed by preview playback.
      if (currentSessionIdRef.current !== session.id || graphBusy) {
        setActionMessage("The selected route changed while checking audio. Press Play again.");
        return false;
      }
      const candidate = sameSessionDraft(draftRef.current, session) ? undefined : draftRef.current;
      if (candidate) {
        setActionMessage("Checking the unsaved route for audio preview...");
        const plan = await backend.planGraph(candidate);
        if (currentSessionIdRef.current !== session.id || !sameSessionDraft(draftRef.current, candidate)) {
          setActionMessage(
            "The route changed while it was being checked. Press Play again to preview the latest draft.",
          );
          return false;
        }
        if (plan.warnings.length > 0) {
          setPendingGraphPlan({ planId: plan.planId, baseRevision: plan.baseRevision });
          setPendingWarnings(plan.warnings);
          setAcknowledgedWarnings(new Set());
          setActionMessage("No audio started. Review the route warnings in Session before previewing or saving.");
          return false;
        }
      }
      setActionMessage(candidate ? "Starting a temporary preview of the unsaved route..." : "Starting session...");
      const idempotencyKey = uiIdempotencyKey("session-start");
      const result = candidate
        ? await backend.startSession(session.id, idempotencyKey, candidate)
        : await backend.startSession(session.id, idempotencyKey);
      if (result.runtime !== "native") {
        await backend.stopSession(session.id, uiIdempotencyKey("simulation-stop"));
        await refresh();
        setActionMessage(
          "No audio played. The backend started a preview without a physical audio route, so it was stopped. Check the device chosen in each Input Device and Output Device node's Properties, then try again.",
        );
        recordUiDiagnostic("Session start rejected simulated runtime; stopped it");
        return false;
      }
      setNativeGenerations((current) => ({
        ...current,
        [session.id]: {
          generation: result.generation,
          kind: diagnostics.nativeSessionId === session.id ? (diagnostics.nativeAdapterKind ?? "endpoint") : "endpoint",
        },
      }));
      await refresh();
      setActionMessage(
        candidate
          ? `Temporary preview is running (generation ${result.generation}); the saved session is unchanged.`
          : `Audio session is running (generation ${result.generation}).`,
      );
      recordUiDiagnostic(`Native session started: generation ${result.generation}`);
      return true;
    } catch (error) {
      setNativePumpStats(null);
      if (isDeviceAccessDenied(error) && backend.setDeviceAccess) {
        setDeviceConsent({ busy: false, error: null });
        setActionMessage("No audio started. Allow AudioRouter to use your audio devices to play.");
        return false;
      }
      const message = formatUiError(error, "Unable to start session.");
      setActionMessage(message);
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
      if (action !== "status") setActionMessage(formatUiError(error, "Time Shift is unavailable."));
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
    if (!backend.connected) return;
    try {
      if (action === "play" && !sessionRunning && !(await startSession())) return;
      if (currentSessionIdRef.current !== session.id) return;
      const result = await backend.transportAudioSource(session.id, nodeId, action);
      if (currentSessionIdRef.current !== session.id) return;
      setAudioSourceStates((current) => ({ ...current, [nodeId]: result.state }));
      setActionMessage(`${draft.nodes.find((node) => node.id === nodeId)?.name ?? "Audio file"} ${result.state}.`);
    } catch (error) {
      setActionMessage(formatUiError(error, "Audio source transport failed."));
    }
  };
  const startSessionRef = useRef(startSession);
  startSessionRef.current = startSession;
  const stopSession = async (propagateFailure = false) => {
    if (sessionBusy.current || !backend.connected) return;
    sessionBusy.current = true;
    setSessionActionBusy(true);
    setActionMessage("Stopping session...");
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
      setActionMessage("Session stopped.");
    } catch (error) {
      setNativePumpStats(null);
      setActionMessage(formatUiError(error, "Unable to stop session."));
      if (propagateFailure) throw error;
    } finally {
      sessionBusy.current = false;
      setSessionActionBusy(false);
    }
  };
  // The listener below is re-registered only when its dependencies change, so
  // it calls this render's actions through a ref. Captured directly, a
  // shortcut pressed after Save started the pre-save `session` and so played
  // a "temporary preview of the unsaved route" instead of the saved one.
  const shortcutActions = useRef({ startSession, stopSession, togglePrivacyMute });
  shortcutActions.current = { startSession, stopSession, togglePrivacyMute };
  useEffect(() => {
    const onShortcut = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      if (isEditableShortcutTarget(event.target)) return;
      const shortcut = shortcutFromKeyboardEvent(event);
      if (!shortcut || shortcutConflicts(shortcuts).length > 0) return;
      const actions = shortcutActions.current;
      if (shortcut === shortcuts.sessionToggle && backend.connected) {
        event.preventDefault();
        void (sessionRunning ? actions.stopSession() : actions.startSession());
      } else if (shortcut === shortcuts.privacyMute && backend.connected) {
        event.preventDefault();
        void actions.togglePrivacyMute();
      }
    };
    window.addEventListener("keydown", onShortcut);
    return () => window.removeEventListener("keydown", onShortcut);
  }, [backend, sessionRunning, shortcuts, privacyMuted, session.id]);
  const captureShortcut = (action: ShortcutAction, event: React.KeyboardEvent<HTMLInputElement>) => {
    event.preventDefault();
    const shortcut = shortcutFromKeyboardEvent(event.nativeEvent);
    if (!shortcut) {
      setShortcutMessage("Use at least one modifier key and a non-modifier key.");
      return;
    }
    const next = { ...shortcuts, [action]: shortcut };
    setShortcuts(next);
    setShortcutMessage(
      shortcutConflicts(next).length > 0 ? "Shortcut conflict: duplicate bindings are disabled until resolved." : null,
    );
  };
  const addLibraryNode = (kind: LibraryNodeKind, _position?: { x: number; y: number }) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before changing the draft.");
      return;
    }
    const next = appendLibraryNode(draft, kind);
    const inserted = next.nodes[next.nodes.length - 1];
    recordDraftChange(next);
    setSelectedNodeId(inserted.id);
    setActionMessage(`${inserted.name} added to the draft. Review and plan the changes before committing.`);
    return inserted.id;
  };
  const addEndpointLoopback = (endpointId: string) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before changing the draft.");
      return;
    }
    try {
      const next = appendEndpointLoopbackNode(draft, endpointId);
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) setSelectedNodeId(inserted.id);
      setActionMessage(
        `${inserted?.name ?? "Endpoint loopback"} added to the stopped draft. Review and plan the changes before committing.`,
      );
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to add endpoint loopback."));
    }
  };
  const addVirtualBusNode = (busId: string, direction: "renderSource" | "captureSink") => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before changing the draft.");
      return undefined;
    }
    try {
      const next = appendVirtualBusNode(draft, busId, direction);
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) setSelectedNodeId(inserted.id);
      setActionMessage(
        `${inserted?.name ?? "Virtual bus node"} added to the stopped draft. Review and plan the changes before committing.`,
      );
      return inserted?.id;
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to add virtual bus node."));
      return undefined;
    }
  };
  const addPluginToDraft = (entry: import("@audiorouter/contracts").PluginScanEntry) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before changing the draft.");
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
        setActionMessage(
          `${inserted?.name ?? "Plugin"}: inserted a stopped plugin placeholder into the connection. Bind a worker before activation, then Plan changes.`,
        );
      } else {
        const next = appendPluginPlaceholderNode(draft, entry);
        const inserted = next.nodes.at(-1);
        recordDraftChange(next);
        if (inserted) {
          setSelectedNodeId(inserted.id);
          setSelectedNodeIds([inserted.id]);
        }
        setActionMessage(
          `${inserted?.name ?? "Plugin"}: added a stopped plugin placeholder to the draft. Bind a worker before activation, then Plan changes.`,
        );
      }
      closePluginPicker();
    } catch (error) {
      setActionMessage(error instanceof Error ? error.message : "Could not add plugin placeholder.");
    }
  };
  const addDroppedVirtualBusNode = (direction: "renderSource" | "captureSink", _position: { x: number; y: number }) => {
    const busId = window.prompt("Existing virtual bus ID", "virtual-bus")?.trim();
    return busId ? addVirtualBusNode(busId, direction) : undefined;
  };
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
      setActionMessage("Connection added to the draft. Save route when you are ready.");
      return added?.id ?? null;
    } catch (error) {
      const occupied =
        error instanceof Error && error.message === "That input already has a connection"
          ? current.edges.find(
              (edge) => edge.destinationNode === destinationNode && edge.destinationPort === destinationPort,
            )
          : undefined;
      if (occupied) {
        const destinationName = current.nodes.find((node) => node.id === destinationNode)?.name ?? "This input";
        const priorName = current.nodes.find((node) => node.id === occupied.sourceNode)?.name ?? "another source";
        const destination = current.nodes.find((node) => node.id === destinationNode);
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
                setConnectionReplacement(null);
                setActionMessage(
                  `Connected ${requestedSource.name} to ${destinationName} and removed the direct ${priorName} branch so the same signal is not heard twice.`,
                );
                setSelectedNodeId(requestedSource.id);
                setSelectedNodeIds([requestedSource.id]);
                return newEdge?.id ?? null;
              }
            } catch (mixError) {
              setActionMessage(formatUiError(mixError, "Unable to route this Mixer to the output."));
              return null;
            }
          }
          try {
            const mixed = addSourceToOccupiedOutput(current, occupied.id, sourceNode, sourcePort);
            const mixer = mixed.nodes.at(-1);
            recordDraftChange(mixed);
            setConnectionReplacement(null);
            setActionMessage(
              `Added a Mixer so ${priorName} and ${current.nodes.find((node) => node.id === sourceNode)?.name ?? "the new source"} can share ${destinationName}. Adjust the Mixer, then Save route when ready.`,
            );
            if (mixer) {
              setSelectedNodeId(mixer.id);
              setSelectedNodeIds([mixer.id]);
            }
            return mixer?.id ?? null;
          } catch (mixError) {
            setActionMessage(formatUiError(mixError, "Unable to combine these sources."));
            return null;
          }
        }
        setConnectionReplacement({
          sessionId: session.id,
          edgeId: occupied.id,
          sourceNode,
          sourcePort,
          destinationNode,
          destinationPort,
        });
        const directionHelp = destination?.ports.some((port) => port.direction === "output")
          ? ` To send audio out of ${destinationName}, start at its blue sending connector and drag to the receiving tool's orange connector.`
          : " An input accepts one source.";
        setActionMessage(
          `${destinationName} already receives ${priorName}.${directionHelp} Choose Replace input connection only to change what feeds ${destinationName}; Undo restores its previous input.`,
        );
        recordUiDiagnostic("Connection rejected: occupied destination input; replacement offered");
      } else {
        setConnectionReplacement(null);
        setActionMessage(formatUiError(error, "Unable to add connection."));
      }
      return null;
    }
  };
  const replaceInputConnection = () => {
    const pending = connectionReplacement;
    const current = draftRef.current;
    if (
      !pending ||
      pending.sessionId !== session.id ||
      !current.edges.some(
        (edge) =>
          edge.id === pending.edgeId &&
          edge.destinationNode === pending.destinationNode &&
          edge.destinationPort === pending.destinationPort,
      )
    ) {
      setConnectionReplacement(null);
      setActionMessage("That connection changed. Try connecting the nodes again.");
      return;
    }
    try {
      const next = appendDraftConnection(
        removeDraftConnection(current, pending.edgeId),
        pending.sourceNode,
        pending.sourcePort,
        pending.destinationNode,
        pending.destinationPort,
      );
      recordDraftChange(next);
      setActionMessage(
        "Input connection replaced in the draft. Undo restores the previous route; Save route when ready.",
      );
    } catch (error) {
      setConnectionReplacement(null);
      setActionMessage(formatUiError(error, "Unable to replace the input connection."));
    }
  };
  const addConnection = () => {
    const source = decodePort(connectionSource);
    const destination = decodePort(connectionDestination);
    if (!source || !destination) {
      setActionMessage("Choose an output and input port first.");
      return false;
    }
    return tryDraftConnection(source.nodeId, source.portName, destination.nodeId, destination.portName) !== null;
  };
  const insertProcessor = (edgeId: string, kind: InsertableProcessorKind) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before changing draft topology.");
      return;
    }
    try {
      const next = insertDraftProcessor(draft, edgeId, kind);
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) setSelectedNodeId(inserted.id);
      setActionMessage(
        `${inserted?.name ?? kind} inserted into the draft. Review and plan the changes before committing.`,
      );
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to insert processor."));
    }
  };
  const appendPreset = (presetId: EqPresetId) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before adding a preset.");
      return;
    }
    try {
      const next = appendEqPresetNode(draft, presetId);
      const inserted = next.nodes.at(-1);
      recordDraftChange(next);
      if (inserted) setSelectedNodeId(inserted.id);
      setActionMessage(
        `${inserted?.name ?? "EQ preset"} added to the draft. Review and plan the changes before committing.`,
      );
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to add preset."));
    }
  };
  const appendVoicePreset = (presetId: VoiceChainPresetId) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before adding a preset.");
      return;
    }
    try {
      const next = appendVoiceChainPreset(draft, presetId);
      const added = next.nodes.slice(draft.nodes.length);
      recordDraftChange(next);
      if (added[0]) setSelectedNodeId(added[0].id);
      setActionMessage(
        `${added.map((node) => node.name).join(", ")} added to the draft. Review and plan the changes before committing.`,
      );
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to add voice preset."));
    }
  };
  useEffect(() => {
    const handleInsertProcessor = (event: Event) => {
      const detail = (event as CustomEvent<{ edgeId?: string; kind?: InsertableProcessorKind }>).detail;
      if (detail.edgeId && detail.kind) insertProcessor(detail.edgeId, detail.kind);
    };
    globalThis.addEventListener("audiorouter:insert-processor", handleInsertProcessor);
    return () => globalThis.removeEventListener("audiorouter:insert-processor", handleInsertProcessor);
  }, [backend, draft]);
  useEffect(() => {
    const handleRemoveNode = (event: Event) => {
      const nodeId = (event as CustomEvent<{ nodeId?: string }>).detail?.nodeId;
      if (!nodeId || !backend.connected) return;
      if (draft.nodes.length <= 1) {
        setActionMessage("A draft must keep at least one node. Add another node before removing this one.");
        return;
      }
      try {
        const next = removeDraftNode(draft, nodeId);
        recordDraftChange(next);
        setSelectedNodeId(next.nodes[0]?.id ?? "");
        setActionMessage("Node removed. Choose Undo in Session to bring it back, or Save route to keep the change.");
      } catch (error) {
        setActionMessage(formatUiError(error, "Unable to remove node."));
      }
    };
    globalThis.addEventListener("audiorouter:remove-node", handleRemoveNode);
    return () => globalThis.removeEventListener("audiorouter:remove-node", handleRemoveNode);
  }, [backend, draft]);
  useEffect(() => {
    const handleAppendEqPreset = (event: Event) => {
      const detail = (event as CustomEvent<{ presetId?: EqPresetId }>).detail;
      if (detail.presetId) appendPreset(detail.presetId);
    };
    globalThis.addEventListener("audiorouter:append-eq-preset", handleAppendEqPreset);
    return () => globalThis.removeEventListener("audiorouter:append-eq-preset", handleAppendEqPreset);
  }, [backend, draft]);
  useEffect(() => {
    const handleAppendVoicePreset = (event: Event) => {
      const detail = (event as CustomEvent<{ presetId?: VoiceChainPresetId }>).detail;
      if (detail.presetId) appendVoicePreset(detail.presetId);
    };
    globalThis.addEventListener("audiorouter:append-voice-preset", handleAppendVoicePreset);
    return () => globalThis.removeEventListener("audiorouter:append-voice-preset", handleAppendVoicePreset);
  }, [backend, draft]);
  const addDroppedCanvasNode = (kind: string, _dropPosition?: { x: number; y: number }) => {
    if (kind === "virtualRenderSource" || kind === "virtualCaptureSink") {
      const busId = window.prompt("Existing virtual bus ID", "virtual-bus")?.trim();
      return busId
        ? addVirtualBusNode(busId, kind === "virtualRenderSource" ? "renderSource" : "captureSink")
        : undefined;
    }
    return addLibraryNode(kind as LibraryNodeKind);
  };
  const connectCanvas = (connection: Connection, dropPosition?: { x: number; y: number }) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before adding a canvas connection.");
      return;
    }
    if (connection.source === LIBRARY_DROP_SOURCE && connection.sourceHandle) {
      return addDroppedCanvasNode(connection.sourceHandle, dropPosition);
    }
    if (!connection.source || !connection.sourceHandle || !connection.target || !connection.targetHandle) {
      setActionMessage("Choose a named output and input port.");
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
      setActionMessage("Added an application-capture source; review identity and plan the graph before committing.");
      closeApplicationPicker();
    } catch (error) {
      setActionMessage(error instanceof Error ? error.message : "Could not add application capture.");
    }
  };
  const rebindApplicationCapture = (nodeId: string, application: ApplicationRow) => {
    try {
      recordDraftChange(rebindApplicationCaptureNode(draftRef.current, nodeId, application));
      setActionMessage(
        `Application source changed to ${application.executable} (PID ${application.processId}). Review and plan the changes before committing.`,
      );
    } catch (error) {
      setActionMessage(error instanceof Error ? error.message : "Could not change the application source.");
    }
  };
  const removeConnection = (edgeId: string) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before changing draft topology.");
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
          ? "Mixer inserted into the draft. Review and plan the changes before committing."
          : topologyAction?.kind === "removeMixer"
            ? "Mixer removed and its single path reconnected in the draft. Review and plan the changes before committing."
            : "Connection removed from the draft. Review and plan the changes before committing.",
      );
    } catch (error) {
      setActionMessage(
        formatUiError(
          error,
          topologyAction?.kind === "insertMixer"
            ? "Unable to insert mixer."
            : topologyAction?.kind === "removeMixer"
              ? "Unable to remove and reconnect mixer."
              : "Unable to remove connection.",
        ),
      );
    }
  };
  const toggleConnection = (edgeId: string, enabled: boolean) => {
    if (!backend.connected) {
      setActionMessage("Connect the backend before changing draft topology.");
      return;
    }
    try {
      recordDraftChange(setDraftConnectionEnabled(draft, edgeId, enabled));
      setActionMessage(
        `Connection ${enabled ? "enabled" : "disabled"} in the draft. Review and plan the changes before committing.`,
      );
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to change connection state."));
    }
  };
  const removeSelectedNode = () => {
    if (draft.nodes.length <= 1) {
      setActionMessage("A route must keep at least one node. Add another node before removing this one.");
      return;
    }
    try {
      const next = removeDraftNode(draft, selectedNode.id);
      recordDraftChange(next);
      setSelectedNodeId(next.nodes[0]?.id ?? "");
      setActionMessage("Node removed. Choose Undo in Session to bring it back, or Save route to keep the change.");
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to remove node."));
    }
  };
  const unloadPluginNode = (nodeId: string) => {
    const node = draft.nodes.find((candidate) => candidate.id === nodeId);
    if (!node) return;
    if (draft.nodes.length <= 1) {
      setActionMessage("A draft must keep at least one node. Add another node before removing this one.");
      return;
    }
    if (!window.confirm(`Unload plugin “${node.name}” and remove it from the draft?`)) return;
    try {
      const next = removeDraftNode(draft, nodeId);
      recordDraftChange(next);
      if (selectedNodeId === nodeId) {
        setSelectedNodeId(next.nodes[0]?.id ?? "");
        setSelectedNodeIds(next.nodes[0] ? [next.nodes[0].id] : []);
      }
      setActionMessage(`${node.name} unloaded from the draft. Review and plan the changes before committing.`);
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to unload plugin."));
    }
  };
  const duplicateSelectedNode = () => {
    try {
      const next = duplicateDraftNode(draft, selectedNode.id);
      const copy = next.nodes[next.nodes.length - 1];
      recordDraftChange(next);
      setSelectedNodeId(copy.id);
      setActionMessage(
        `${copy.name} added to the draft without connections. Review and plan the changes before committing.`,
      );
    } catch (error) {
      setActionMessage(formatUiError(error, "Unable to duplicate node."));
    }
  };
  if (backend.connected && !hasSnapshot)
    return (
      <div className={`app-shell theme-${theme}`}>
        <main className="panel" aria-label="Loading saved session">
          <h1>AudioRouter</h1>
          <p role="status">
            {snapshotState.error
              ? `Unable to load your saved session: ${snapshotState.error}`
              : "Loading your saved session…"}
          </p>
          {snapshotState.error && (
            <button type="button" onClick={refresh}>
              Reconnect
            </button>
          )}
        </main>
      </div>
    );
  // Built once per App render, outside the per-tick telemetry render prop:
  // meter ticks reach the canvas only through its telemetry store.
  const flowCanvas = (
    <SessionFlowCanvas
      groups={groupState.groups}
      selectedGroupId={groupState.selectedGroupId}
      onSelectGroup={(id) => {
        groupState.selectGroup(id);
        if (id) setWorkbenchTab("properties");
      }}
      onChangeGroup={groupState.changeGroup}
      onRemoveGroup={groupState.removeGroup}
      session={draft}
      selectedNodeId={selectedNode.id}
      selectedNodeIds={selectedNodeIds}
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
      onAddVirtualBusNode={addDroppedVirtualBusNode}
      onOpenPluginPicker={openPluginPicker}
      onOpenApplicationPicker={openApplicationPicker}
      onConnectionRejected={setActionMessage}
      canEdit={backend.connected}
    />
  );
  const connectionWorkbenchContent = (
    <ConnectionForm
      backend={backend}
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
  const setupWorkbenchContent = (
    <SetupWorkbench
      backend={backend}
      devices={devices}
      refreshDevices={refreshDevices}
      setWorkbenchTab={setWorkbenchTab}
      setupSteps={setupSteps}
      flowAnimation={flowAnimation}
      changeFlowAnimation={changeFlowAnimation}
      updateChecks={updateChecks}
      setUpdateChecks={setUpdateChecks}
      availableUpdate={availableUpdate}
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
  const advancedWorkbenchContent = (
    <AdvancedWorkbench
      backend={backend}
      draft={draft}
      selectedNode={selectedNode}
      selectNodeProperties={selectNodeProperties}
      removeConnection={removeConnection}
      toggleConnection={toggleConnection}
      insertProcessor={insertProcessor}
      openPluginPicker={openPluginPicker}
      connectionWorkbenchContent={connectionWorkbenchContent}
      devicesWorkbenchContent={devicesWorkbenchContent}
      shortcuts={shortcuts}
      captureShortcut={captureShortcut}
      shortcutMessage={shortcutMessage}
      addPluginToDraft={addPluginToDraft}
      processors={processors}
      processorError={processorError}
      presets={presets}
      presetError={presetError}
      session={session}
      setCreatedSessions={setCreatedSessions}
      setSelectedSessionId={setSelectedSessionId}
      refresh={refresh}
      applications={applications}
      snapshot={snapshot}
      clearRecoverySafeMode={clearRecoverySafeMode}
      safetyActionBusyState={safetyActionBusyState}
    />
  );
  return (
    <PluginParameterContext.Provider value={{ parameters: pluginParameters, error: pluginParameterError }}>
      <div className={`app-shell theme-${theme}`}>
        <header className="topbar">
          <div>
            <VersionLine update={availableUpdate} />
            <h1>{draft.name || "Routing workspace"}</h1>
          </div>
          <div className={`status-cluster ${backend.connected ? "connected" : "disconnected"}`} aria-live="polite">
            <span className={`audio-run-state${sessionRunning ? " is-running" : ""}`} role="status">
              {sessionActionBusy
                ? "Starting or stopping audio…"
                : sessionRunning
                  ? "● Audio running"
                  : "○ Audio stopped"}
            </span>
            <span className="status-detail" title={statusSummary}>
              {connectionLabel}
            </span>
            <span className="history-toolbar">
              <button
                type="button"
                className="secondary"
                aria-label="Undo"
                title="Undo (Ctrl+Z)"
                onClick={undoDraft}
                disabled={!backend.connected || graphBusy || sessionActionBusy || draftHistory.past.length === 0}
              >
                <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M9 4 4 9l5 5M4 9h10a6 6 0 0 1 0 12" />
                </svg>
                Undo
              </button>
              <button
                type="button"
                className="secondary"
                aria-label="Redo"
                title="Redo (Ctrl+Y)"
                onClick={redoDraft}
                disabled={!backend.connected || graphBusy || sessionActionBusy || draftHistory.future.length === 0}
              >
                <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="m15 4 5 5-5 5m5-5H10a6 6 0 0 0 0 12" />
                </svg>
                Redo
              </button>
            </span>
            <button
              type="button"
              className={routeChanged ? "primary" : "secondary"}
              onClick={() => void (pendingGraphPlan ? commitAcknowledgedPlan() : planChanges())}
              disabled={
                !backend.connected ||
                graphBusy ||
                (pendingGraphPlan ? acknowledgedWarnings.size !== pendingWarnings.length : !routeChanged)
              }
            >
              {graphBusy ? "Saving…" : pendingGraphPlan ? "Confirm Save" : "Save"}
            </button>
            <button
              type="button"
              className={sessionRunning ? "secondary" : "primary"}
              onClick={() => void (sessionRunning ? stopSession() : startSession())}
              disabled={!backend.connected || sessionActionBusy}
            >
              {sessionRunning ? "Stop" : "Play"}
            </button>
            <button
              type="button"
              className="secondary privacy-mute-action"
              aria-pressed={privacyMuted}
              aria-label={privacyMuted ? "Microphone muted" : "Mute microphone"}
              onClick={() => void togglePrivacyMute()}
              disabled={!backend.connected || safetyActionBusyState}
            >
              {privacyMuted ? "Mic muted" : "Mute mic"}
            </button>
            <label className="theme-picker">
              Theme
              <select
                aria-label="Color theme"
                value={theme}
                onChange={(event) => setTheme(event.target.value as ThemeMode)}
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
                <option value="high-contrast">High contrast</option>
              </select>
            </label>
            <button type="button" onClick={refresh}>
              Reconnect
            </button>
            <QuitButton onMessage={setActionMessage} />
          </div>
        </header>
        {unfedRouteNodes(draft).length > 0 && (
          <p className="panel-message is-warning inactive-route-warning" role="status">
            Warning: no input reaches{" "}
            {unfedRouteNodes(draft)
              .map((node) => node.name)
              .join(", ")}
            . These nodes are ignored during playback; connected routes can still play.
          </p>
        )}
        {(() => {
          const text = actionMessage
            ? actionMessage
            : !backend.connected
              ? `Audio unavailable: ${(snapshot?.status.reason ?? "the backend is disconnected").replace(/\.+$/, "")}. Reconnect to edit or play.`
              : sessionRunning
                ? "Audio is running through this session."
                : "Audio is stopped. Press Play to start this session. To set up a new route, add an input (such as your microphone) and an output (such as your headphones), then connect their ports on the canvas.";
          const tone = !backend.connected ? "error" : actionMessage ? actionMessageTone(actionMessage) : "info";
          return (
            <div
              className={`global-action-message is-${tone}`}
              role={tone === "error" ? "alert" : "status"}
              aria-live={tone === "error" ? "assertive" : "polite"}
            >
              <span className="global-action-message-icon" aria-hidden="true">
                {tone === "error" ? "!" : tone === "warning" ? "!" : tone === "success" ? "✓" : "i"}
              </span>
              <span className="global-action-message-text" title={text}>
                {text}
              </span>
              {backend.connected && connectionReplacement && actionMessage?.includes("Replace input connection") && (
                <button type="button" className="secondary" onClick={replaceInputConnection}>
                  Replace input connection
                </button>
              )}
              {backend.connected && actionMessage?.includes("Troubleshooting") && workbenchTab !== "advanced" && (
                <button type="button" className="secondary" onClick={() => setWorkbenchTab("advanced")}>
                  Open Advanced
                </button>
              )}
              {backend.connected && actionMessage?.includes("Open Session") && workbenchTab !== "session" && (
                <button type="button" className="secondary" onClick={() => setWorkbenchTab("session")}>
                  Open Session
                </button>
              )}
              {backend.connected && actionMessage && (
                <button
                  type="button"
                  className="secondary global-action-message-dismiss"
                  aria-label="Dismiss message"
                  title="Dismiss message"
                  onClick={() => {
                    setActionMessage(null);
                    setConnectionReplacement(null);
                  }}
                >
                  ×
                </button>
              )}
            </div>
          );
        })()}
        {pendingGraphPlan && pendingWarnings.length > 0 && (
          <section className="save-warning-review" aria-label="Save warnings">
            <strong>Review before saving</strong>
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
          <main className="main-content" style={{ "--sidebar-width": `${sidebarWidth}px` } as CSSProperties}>
            <SidebarResizer width={sidebarWidth} onWidth={changeSidebarWidth} />
            <section className="workspace-title">
              <div>
                <p className="eyebrow">{sessionRunning ? "Running session" : "Stopped session"}</p>
                <label className="session-name">
                  Session name
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
                    ? "draft changes require review, then an explicit commit"
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
                  Duplicate
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void deleteSession()}
                  disabled={!backend.connected}
                >
                  Delete
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={undoDraft}
                  disabled={!backend.connected || draftHistory.past.length === 0}
                >
                  Undo draft
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={redoDraft}
                  disabled={!backend.connected || draftHistory.future.length === 0}
                >
                  Redo draft
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => {
                    setDraft(session);
                    setDraftHistory({ past: [], future: [] });
                    setPendingWarnings([]);
                    setAcknowledgedWarnings(new Set());
                    setPendingOperation(null);
                    setPendingGraphPlan(null);
                    setActionMessage("Draft discarded.");
                  }}
                  disabled={!backend.connected}
                >
                  Discard draft
                </button>
                <button
                  type="button"
                  className="primary"
                  onClick={() => void planChanges()}
                  disabled={!backend.connected || graphBusy}
                >
                  Plan changes
                </button>
              </div>
            </section>
            <ErrorBoundary area="Signal flow" onError={recordUiDiagnostic}>
              <LiveTelemetry store={telemetryStore} diagnostics={snapshot?.diagnostics ?? null}>
                {(liveDiagnostics) => (
                  <section id="signal-flow-panel" className="canvas-panel" aria-labelledby="canvas-heading">
                    <div className="section-heading">
                      <div>
                        <p className="eyebrow">Signal flow</p>
                        <h2 id="canvas-heading">Canvas</h2>
                      </div>
                    </div>
                    <p className="muted canvas-handle-legend">
                      <span className="canvas-handle-legend-dot canvas-handle-legend-dot-target" aria-hidden="true" />{" "}
                      Send (start)
                      <span
                        className="canvas-handle-legend-dot canvas-handle-legend-dot-source"
                        aria-hidden="true"
                      />{" "}
                      Receive (end) - drag from the sending tool's blue dot to the receiving tool's orange dot.
                    </p>
                    {flowCanvas}
                    <fieldset className="connection-editor" disabled={!backend.connected}>
                      <legend>Add connection to draft</legend>
                      <label>
                        Output
                        <select
                          aria-label="Source output port"
                          value={connectionSource}
                          onChange={(event) => setConnectionSource(event.target.value)}
                        >
                          <option value="">Choose source</option>
                          {outputPorts.map((port) => (
                            <option
                              key={encodePort(port.nodeId, port.portName)}
                              value={encodePort(port.nodeId, port.portName)}
                            >
                              {port.nodeName} · {port.portName} · {port.channels}ch
                            </option>
                          ))}
                        </select>
                      </label>
                      <span aria-hidden="true">→</span>
                      <label>
                        Input
                        <select
                          aria-label="Destination input port"
                          value={connectionDestination}
                          onChange={(event) => setConnectionDestination(event.target.value)}
                        >
                          <option value="">Choose destination</option>
                          {inputPorts.map((port) => (
                            <option
                              key={encodePort(port.nodeId, port.portName)}
                              value={encodePort(port.nodeId, port.portName)}
                            >
                              {port.nodeName} · {port.portName} · {port.channels}ch
                            </option>
                          ))}
                        </select>
                      </label>
                      <button type="button" className="secondary" onClick={addConnection}>
                        Add connection
                      </button>
                      <button type="button" className="secondary" onClick={openConnectionDialog}>
                        Keyboard connection dialog
                      </button>
                    </fieldset>
                    {connectionDialogOpen && (
                      <div className="dialog-backdrop" role="presentation">
                        <section
                          ref={connectionDialog}
                          className="connection-dialog"
                          role="dialog"
                          aria-modal="true"
                          aria-labelledby="connection-dialog-heading"
                          aria-describedby="connection-dialog-description"
                        >
                          <div className="section-heading">
                            <h2 id="connection-dialog-heading">Keyboard connection</h2>
                            <button
                              type="button"
                              className="secondary"
                              onClick={closeConnectionDialog}
                              aria-label="Close keyboard connection dialog"
                            >
                              Close
                            </button>
                          </div>
                          <p id="connection-dialog-description" className="muted">
                            Choose an output and input, then add the connection to the draft. Press Escape to close.
                          </p>
                          <label>
                            Output
                            <select
                              ref={connectionDialogSource}
                              aria-label="Keyboard source output port"
                              value={connectionSource}
                              onChange={(event) => setConnectionSource(event.target.value)}
                            >
                              <option value="">Choose source</option>
                              {outputPorts.map((port) => (
                                <option
                                  key={encodePort(port.nodeId, port.portName)}
                                  value={encodePort(port.nodeId, port.portName)}
                                >
                                  {port.nodeName} · {port.portName} · {port.channels}ch
                                </option>
                              ))}
                            </select>
                          </label>
                          <label>
                            Input
                            <select
                              aria-label="Keyboard destination input port"
                              value={connectionDestination}
                              onChange={(event) => setConnectionDestination(event.target.value)}
                            >
                              <option value="">Choose destination</option>
                              {inputPorts.map((port) => (
                                <option
                                  key={encodePort(port.nodeId, port.portName)}
                                  value={encodePort(port.nodeId, port.portName)}
                                >
                                  {port.nodeName} · {port.portName} · {port.channels}ch
                                </option>
                              ))}
                            </select>
                          </label>
                          <div className="actions">
                            <button
                              type="button"
                              className="primary"
                              onClick={() => {
                                if (addConnection()) closeConnectionDialog();
                              }}
                            >
                              Add connection to draft
                            </button>
                            <button type="button" className="secondary" onClick={closeConnectionDialog}>
                              Cancel
                            </button>
                          </div>
                        </section>
                      </div>
                    )}
                    {pluginPickerOpen && (
                      <div className="dialog-backdrop" role="presentation">
                        <section
                          ref={pluginPickerDialog}
                          className="connection-dialog plugin-picker-dialog"
                          role="dialog"
                          aria-modal="true"
                          aria-labelledby="plugin-picker-heading"
                          aria-describedby="plugin-picker-description"
                        >
                          <div className="section-heading">
                            <h2 id="plugin-picker-heading">
                              {pluginInsertEdgeId
                                ? "Insert a VST2/VST3 plugin into this connection"
                                : "Add a VST2/VST3 plugin"}
                            </h2>
                            <button
                              type="button"
                              className="secondary"
                              onClick={closePluginPicker}
                              aria-label="Close plugin picker"
                            >
                              Close
                            </button>
                          </div>
                          <p id="plugin-picker-description" className="muted">
                            Scan an absolute directory on this machine for supported x64 VST2/VST3 binaries, then{" "}
                            {pluginInsertEdgeId
                              ? "insert one directly into the connection"
                              : "add one as a stopped node"}
                            . Press Escape to close.
                          </p>
                          <LoadedPluginsPanel
                            nodes={draft.nodes}
                            selectedNodeId={selectedNode.id}
                            disabled={!backend.connected}
                            onSelect={(nodeId) => {
                              setSelectedNodeId(nodeId);
                              setSelectedNodeIds([nodeId]);
                            }}
                            onUnload={unloadPluginNode}
                            diagnostics={liveDiagnostics ?? null}
                          />
                          <PluginScanPanel
                            backend={backend}
                            onAddPlaceholder={addPluginToDraft}
                            headingId="plugin-picker-scan-heading"
                          />
                        </section>
                      </div>
                    )}
                    {applicationPickerOpen &&
                      (() => {
                        const choices = applicationCaptureChoices(applications);
                        const capturable = [...choices.withAudio, ...choices.other];
                        const applicationKey = applicationChoiceKey;
                        const selected =
                          capturable.find(
                            (application) => applicationKey(application) === applicationPickerSelection,
                          ) ?? capturable[0];
                        return (
                          <div className="dialog-backdrop" role="presentation">
                            <section
                              ref={applicationPickerDialog}
                              className="connection-dialog plugin-picker-dialog"
                              role="dialog"
                              aria-modal="true"
                              aria-labelledby="application-picker-heading"
                              aria-describedby="application-picker-description"
                            >
                              <div className="section-heading">
                                <h2 id="application-picker-heading">Add an application capture source</h2>
                                <button
                                  type="button"
                                  className="secondary"
                                  onClick={closeApplicationPicker}
                                  aria-label="Close application picker"
                                >
                                  Close
                                </button>
                              </div>
                              <p id="application-picker-description" className="muted">
                                Pick a running application to capture its audio as an input node. After restart,
                                AudioRouter reconnects when it finds one verified matching instance; if several match,
                                choose the intended instance again. Playback does not start just because the app is
                                open.
                              </p>
                              <div className="actions">
                                <button type="button" className="secondary" onClick={refreshApplications}>
                                  Refresh applications
                                </button>
                              </div>
                              {applicationsError ? (
                                <p className="muted" role="status">
                                  Application inventory unavailable: {applicationsError}
                                </p>
                              ) : capturable.length === 0 ? (
                                <p className="muted" role="status">
                                  No running applications were found. Refresh to try again.
                                </p>
                              ) : (
                                <>
                                  <label>
                                    Application
                                    <select
                                      aria-label="Application to capture"
                                      value={selected ? applicationKey(selected) : ""}
                                      onChange={(event) => setApplicationPickerSelection(event.target.value)}
                                    >
                                      <ApplicationChoiceOptions applications={applications} />
                                    </select>
                                  </label>
                                  {selected && (
                                    <p className="muted" role="status">
                                      {selected.executable} ·{" "}
                                      {selected.audioSessionCount > 0
                                        ? `audio `
                                        : "no audio session yet; capture begins when it plays sound"}
                                    </p>
                                  )}
                                  <div className="actions">
                                    <button
                                      type="button"
                                      className="primary"
                                      onClick={() => selected && addApplicationCaptureFromPicker(selected)}
                                      disabled={!backend.connected || !selected}
                                    >
                                      Add capture source
                                    </button>
                                  </div>
                                </>
                              )}
                              <p className="muted">
                                For advanced options (application-capture policy, preparing the native worker), use the
                                Applications panel in Full workspace mode.
                              </p>
                            </section>
                          </div>
                        );
                      })()}
                  </section>
                )}
              </LiveTelemetry>
            </ErrorBoundary>
            <ErrorBoundary area="Properties" onError={recordUiDiagnostic}>
              <LiveTelemetry store={telemetryStore} diagnostics={snapshot?.diagnostics ?? null}>
                {(liveDiagnostics) => (
                  <PropertiesPanel
                    selectedGroup={selectedGroup}
                    groupState={groupState}
                    selectedNode={selectedNode}
                    backend={backend}
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
                    renderEndpointId={renderEndpointId}
                    applications={applications}
                    applicationsError={applicationsError}
                    refreshApplications={refreshApplications}
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
            <ErrorBoundary area="Side panel" onError={recordUiDiagnostic}>
              <Workbench
                onAddGroup={() => {
                  groupState.addGroup();
                  setWorkbenchTab("properties");
                }}
                tab={workbenchTab}
                onTab={(tab) => setWorkbenchTab(tab)}
                sessionFileContent={
                  <SessionFilePanel
                    backend={backend}
                    session={session}
                    unsaved={routeChanged}
                    onImported={(imported) => {
                      setCreatedSessions((current) => [...current.filter((item) => item.id !== imported.id), imported]);
                      setSelectedSessionId(imported.id);
                      void refresh();
                    }}
                  />
                }
                pluginsContent={
                  <PluginToolsGroup
                    backend={backend}
                    connected={backend.connected}
                    search={librarySearch}
                    refreshKey={pluginPickerOpen}
                    onAdd={addPluginToDraft}
                    onOpenPicker={() => openPluginPicker()}
                  />
                }
                tools={visibleLibraryEntries}
                connected={backend.connected}
                onAdd={addLibraryNode}
                onApplicationPicker={openApplicationPicker}
                librarySearch={librarySearch}
                onLibrarySearch={(value) => setLibrarySearch(value.slice(0, 80))}
                onNewSession={() => void createSession()}
                onDuplicate={() => void duplicateSession()}
                onDelete={() => void deleteSession()}
                onUndo={undoDraft}
                onRedo={redoDraft}
                onDiscard={() => {
                  setDraft(session);
                  setDraftHistory({ past: [], future: [] });
                  setPendingWarnings([]);
                  setAcknowledgedWarnings(new Set());
                  setPendingOperation(null);
                  setPendingGraphPlan(null);
                  setConnectionReplacement(null);
                  setActionMessage("Draft discarded.");
                }}
                onPlan={() => void planChanges()}
                onCommit={() => void commitAcknowledgedPlan()}
                canCommit={acknowledgedWarnings.size === pendingWarnings.length && !graphBusy}
                pendingPlan={Boolean(pendingGraphPlan)}
                actionMessage={actionMessage}
                onReplaceInputConnection={
                  connectionReplacement && actionMessage?.includes("Replace input connection")
                    ? replaceInputConnection
                    : undefined
                }
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
                onSelectSession={(id) => {
                  setConnectionReplacement(null);
                  setSelectedSessionId(id);
                }}
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
                mcpSetupInfo={mcpSetupInfo}
                clientsPanel={<ClientsPanel backend={backend} />}
                setupContent={setupWorkbenchContent}
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
                advancedContent={advancedWorkbenchContent}
              />
            </ErrorBoundary>
          </main>
        </div>
        <LibraryDragOverlay />
        {deviceConsent && (
          <DeviceAccessDialog
            busy={deviceConsent.busy}
            error={deviceConsent.error}
            onAllow={() => void allowDeviceAccessAndPlay()}
            onCancel={() => setDeviceConsent(null)}
          />
        )}
      </div>
    </PluginParameterContext.Provider>
  );
}
