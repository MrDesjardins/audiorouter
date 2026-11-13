import { returnToPropertiesTab } from "./workbenchPanel";
import { ToolGuideLink } from "./ToolGuideLink";
import { CanvasLayoutInspector } from "./CanvasLayoutInspector";
import type { CanvasRect, LayoutAction } from "./canvasGeometry";
// The Properties panel: the selected node's (or canvas group's) inspector and editors (moved from App.tsx).
import { useId, useRef, type Dispatch, type RefObject, type SetStateAction } from "react";
import { useSvgTextFloor } from "./useSvgTextFloor";
import type { DeviceListItem, DiagnosticsSnapshot, Node, Session } from "@audiorouter/contracts";
import type { ApplicationRow, RecorderStatus, UiBackend, UiBackendSnapshot } from "./backend";
import type { ProcessorDescriptor } from "./processorCatalog";
import { MeterInspector } from "./MeterInspector";
import { NodePropertyStatus } from "./NodePropertyStatus";
import { ToolCompare, ToolPresetPicker } from "./ToolCompare";
import { takesPresets } from "./compareRules";
import { NodeIdentity } from "./NodeIdentity";
import { CanvasGroupInspector, type CanvasGroup, type useCanvasGroups } from "./CanvasGroups";
import { SpatialAudioField } from "./SpatialAudioField";
import { InputChannelsField } from "./InputChannelsField";
import { libraryEntries, toolDescription } from "./library";
import { EditableNodeName } from "./EditableNodeName";
import { HelpDisclosure } from "./HelpDisclosure";
import { PropertySection } from "./PropertySection";
import { SpectralGateEditor } from "./SpectralGateEditor";
import { FunVoiceChangerEditor } from "./FunVoiceChangerEditor";
import { DynamicsEditor } from "./DynamicsEditor";
import { DuckEditor } from "./DuckEditor";
import { RecorderControls, RecordingFolderField } from "./RecorderControls";
import {
  BassTrebleEditor,
  DehumEditor,
  DelayEditor,
  GraphicEqEditor,
  InputSwitchEditor,
  OutputSwitchEditor,
  LevelEditor,
  PitchEditor,
  StrengthEditor,
} from "./ToolVisuals";
import { dynamicsControlSplit, isDynamicsKind } from "./dynamics";
import { NetworkNodeEditor } from "./NetworkNodeEditor";
import { EqSpectrumContext } from "./AdvancedEqEditor";
import { writeEndpointBindingHint } from "./endpointBinding";
import { recorderHasCaptureSource } from "./RecordingPanels";
import { PluginEditorControls, PluginNodeInspector, PluginRecoveryControls } from "./PluginPanels";
import {
  ApplicationCaptureBinding,
  CableBinding,
  DenoiseLearnEditor,
  FirFilterEditor,
  MixerInputsEditor,
  NodeTelemetryPanel,
  PhysicalInputBinding,
  PhysicalOutputBinding,
  ProcessorParameterEditor,
} from "./NodeEditors";
import { AudioFileNodeEditor } from "./AudioFileNodeEditor";
import { SoundboardNodeEditor } from "./SoundboardNodeEditor";
import { removeNodeDraftParameters, setVirtualBusDraftCable, unfedRouteNodes } from "./draft";
import { useVirtualCableStatus } from "./virtualCableStatus";
import type { NodeDeviceUsers } from "./appUsage";

export function PropertiesPanel({
  layoutItems = [],
  onLayoutAction,
  selectedGroup,
  groupState,
  selectedNode,
  backend,
  sessionRunning,
  liveDiagnostics,
  session,
  recordDraftChange,
  draft,
  setActionMessage,
  changeNodeParameterOn,
  devices,
  refreshDevices,
  setCaptureEndpointId,
  renderEndpointId,
  applications,
  applicationsError,
  refreshApplications,
  deviceUsers,
  rebindApplicationCapture,
  setRenderEndpointId,
  captureEndpointId,
  recorderStatuses,
  recordingBusy,
  lastRecordingPaths,
  recordingMessage,
  toggleNodeRecording,
  audioSourceStates,
  currentSessionIdRef,
  transportAudioSource,
  unloadPluginNode,
  changeNodeName,
  sessionActionBusy,
  graphBusy,
  changeNodeFlag,
  processors,
  snapshot,
  changeNodeParameter,
  resetNodeParameters,
  duplicateSelectedNode,
  removeSelectedNode,
  togglePrivacyMute,
  privacyMuted,
}: {
  layoutItems?: CanvasRect[];
  onLayoutAction?: (action: LayoutAction) => void;
  selectedGroup: CanvasGroup | undefined;
  groupState: ReturnType<typeof useCanvasGroups>;
  selectedNode: Node | undefined;
  backend: UiBackend;
  sessionRunning: boolean;
  liveDiagnostics: DiagnosticsSnapshot | null;
  session: Session;
  recordDraftChange: (next: Session, group?: string) => void;
  draft: Session;
  setActionMessage: Dispatch<SetStateAction<string | null>>;
  changeNodeParameterOn: (nodeId: string, name: string, value: boolean | number | string) => void;
  devices: DeviceListItem[];
  refreshDevices: ({ announce }?: { announce?: boolean }) => void;
  setCaptureEndpointId: Dispatch<SetStateAction<string>>;
  renderEndpointId: string;
  applications: ApplicationRow[];
  applicationsError: string | null;
  refreshApplications: () => void;
  /** Apps on each device node's device (D1); null while checking, absent offline. */
  deviceUsers?: Map<string, NodeDeviceUsers> | null;
  rebindApplicationCapture: (nodeId: string, application: ApplicationRow) => void;
  setRenderEndpointId: Dispatch<SetStateAction<string>>;
  captureEndpointId: string;
  recorderStatuses: RecorderStatus[];
  recordingBusy: string | null;
  lastRecordingPaths: Record<string, string>;
  recordingMessage: { nodeId: string; text: string } | null;
  toggleNodeRecording: (nodeId: string, record: boolean) => Promise<void>;
  audioSourceStates: Record<string, "paused" | "playing" | "stopped">;
  currentSessionIdRef: RefObject<string>;
  transportAudioSource: (nodeId: string, action: "play" | "pause" | "stop" | "status") => Promise<void>;
  unloadPluginNode: (nodeId: string) => void;
  changeNodeName: (name: string) => void;
  sessionActionBusy: boolean;
  graphBusy: boolean;
  changeNodeFlag: (flag: "enabled" | "bypass", value: boolean) => Promise<void>;
  processors: ProcessorDescriptor[] | null;
  snapshot: UiBackendSnapshot | null;
  changeNodeParameter: (name: string, value: boolean | number | string) => void;
  resetNodeParameters: () => void;
  duplicateSelectedNode: () => void;
  removeSelectedNode: () => void;
  togglePrivacyMute: () => Promise<void>;
  privacyMuted: boolean;
}) {
  const cableStatus = useVirtualCableStatus();
  const flagId = useId();
  const inspector = useRef<HTMLElement>(null);
  useSvgTextFloor(inspector, selectedNode?.id ?? "", !selectedGroup && !(layoutItems.length > 1 && onLayoutAction));
  return layoutItems.length > 1 && onLayoutAction ? (
    <CanvasLayoutInspector
      items={layoutItems}
      onAction={onLayoutAction}
      groups={groupState.groups.filter((group) => layoutItems.some((item) => item.id === group.id))}
      onGroupOptionsChange={(patch) =>
        groupState.changeGroupOptions(
          layoutItems.filter((item) => item.group).map((item) => item.id),
          patch,
        )
      }
    />
  ) : selectedGroup ? (
    <CanvasGroupInspector
      group={selectedGroup}
      onChange={(patch) => groupState.changeGroup(selectedGroup.id, patch)}
      onRemove={() => groupState.removeGroup(selectedGroup.id)}
    />
  ) : !selectedNode ? (
    // An empty route (for example the "Start empty" goal) has no tool to describe.
    <section
      className="panel inspector"
      id="workbench-properties"
      role="tabpanel"
      aria-labelledby="workbench-tab-properties"
      tabIndex={0}
    >
      <h2 id="inspector-heading">No tool selected</h2>
      <p className="muted">This route is empty. Open the Tools tab and add a tool, such as your microphone.</p>
    </section>
  ) : (
    <>
      <section
        className="panel inspector"
        ref={inspector}
        id="workbench-properties"
        onKeyDown={returnToPropertiesTab}
        role="tabpanel"
        aria-labelledby="workbench-tab-properties"
        tabIndex={0}
      >
        <div className="section-heading">
          <div>
            <div className="inspector-title">
              <h2 id="inspector-heading" aria-label={selectedNode.name}>
                <EditableNodeName
                  key={selectedNode.id}
                  name={selectedNode.name}
                  disabled={!backend.connected}
                  onRename={changeNodeName}
                />
              </h2>
              <NodePropertyStatus
                node={selectedNode}
                connected={backend.connected}
                running={sessionRunning}
                snapshot={liveDiagnostics ?? null}
                unfed={unfedRouteNodes(draft).some((node) => node.id === selectedNode.id)}
              />
            </div>
          </div>
          <span className="badge">
            {libraryEntries.find((entry) => entry.kind === selectedNode.kind)?.label ?? selectedNode.kind}
          </span>
        </div>
        {toolDescription(selectedNode.kind) && (
          <p className="muted tool-description">{toolDescription(selectedNode.kind)}</p>
        )}
        <ToolGuideLink key={selectedNode.kind} kind={selectedNode.kind} />
        <HelpDisclosure title="How to edit this tool">
          <p>Click the name to rename this tool. Press Enter or click away to finish; Escape cancels.</p>
          <p>
            Parameter edits save automatically after a short pause while audio plays. Other changes need Save at the
            top; the canvas marks tools that need saving.
          </p>
          <p>
            For delay and processing measurements, open Timing while audio plays. Ready means enabled for the next
            playback; Active does not guarantee an incoming signal.
          </p>
          <p>
            Off silences inputs and outputs; an off effect passes sound without processing. Bypass passes sound around
            an effect; on a prepared input, output, or Mixer it silences that contribution. Live toggles do not require
            Save or Stop and keep your other edits. Off devices remain open until Stop and contribute silence.
          </p>
        </HelpDisclosure>
        {selectedNode.kind === "meter" ? (
          <MeterInspector
            key={selectedNode.id}
            node={selectedNode}
            sessionId={session.id}
            snapshot={liveDiagnostics ?? null}
            running={sessionRunning}
            backend={backend}
            onUpgrade={() => {
              recordDraftChange({
                ...draft,
                nodes: draft.nodes.map((n) =>
                  n.id === selectedNode.id
                    ? {
                        ...n,
                        ports: [
                          ...n.ports,
                          {
                            name: "out",
                            direction: "output",
                            channels: n.ports.find((p) => p.direction === "input")?.channels ?? 2,
                          },
                        ],
                      }
                    : n,
                ),
              });
              setActionMessage("Meter output added. Connect it to the next tool and Save to keep it.");
            }}
          />
        ) : selectedNode.kind === "duck" ? (
          <DuckEditor
            key={selectedNode.id}
            node={selectedNode}
            session={draft}
            telemetry={
              liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.processor ?? null
            }
            running={sessionRunning}
            disabled={!backend.connected}
            gameRound={liveDiagnostics?.gameRound ?? null}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        ) : isDynamicsKind(selectedNode.kind) ? (
          <DynamicsEditor
            key={selectedNode.id}
            kind={selectedNode.kind}
            node={selectedNode}
            telemetry={
              liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.processor ?? null
            }
            channels={selectedNode.ports.find((port) => port.direction === "input")?.channels ?? 2}
            running={sessionRunning}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
            mainControls={
              <ProcessorParameterEditor
                node={selectedNode}
                processors={processors}
                nodeTypes={snapshot?.discovery?.nodeTypes ?? null}
                connected={backend.connected}
                onChange={changeNodeParameter}
                only={dynamicsControlSplit(selectedNode.kind).main}
                compact
              />
            }
            timingControls={
              <ProcessorParameterEditor
                node={selectedNode}
                processors={processors}
                nodeTypes={snapshot?.discovery?.nodeTypes ?? null}
                connected={backend.connected}
                onChange={changeNodeParameter}
                only={dynamicsControlSplit(selectedNode.kind).timing}
                compact
              />
            }
          />
        ) : (
          <NodeTelemetryPanel node={selectedNode} snapshot={liveDiagnostics ?? null} running={sessionRunning} />
        )}
        {(selectedNode.kind === "virtualRenderSource" || selectedNode.kind === "virtualCaptureSink") && (
          <CableBinding
            status={cableStatus}
            value={String(selectedNode.parameters.busId ?? "")}
            direction={selectedNode.kind === "virtualRenderSource" ? "renderSource" : "captureSink"}
            disabled={sessionRunning}
            onChange={(busId) => {
              try {
                recordDraftChange(
                  setVirtualBusDraftCable(draft, selectedNode.id, busId, cableStatus),
                  selectedNode.id + ":busId",
                );
                setActionMessage("Cable changed. Save to keep it.");
              } catch (error) {
                setActionMessage(error instanceof Error ? error.message : "Unable to change the cable.");
              }
            }}
          />
        )}
        {selectedNode.kind === "physicalInput" && (
          <PhysicalInputBinding
            devices={devices}
            surround={
              selectedNode.parameters.spatialMode === "headphones" || selectedNode.parameters.spatialMode === "speakers"
            }
            value={typeof selectedNode.parameters.endpointId === "string" ? selectedNode.parameters.endpointId : ""}
            disabled={!backend.connected || sessionRunning}
            onRefresh={refreshDevices}
            onChange={(value) => {
              setCaptureEndpointId(value);
              writeEndpointBindingHint(session.id, value, renderEndpointId);
              if (value) changeNodeParameterOn(selectedNode.id, "endpointId", value);
            }}
          />
        )}
        {selectedNode.kind === "physicalInput" && (
          <InputChannelsField
            devices={devices}
            endpointId={
              typeof selectedNode.parameters.endpointId === "string" ? selectedNode.parameters.endpointId : ""
            }
            mode={selectedNode.parameters.channelMode}
            surround={
              selectedNode.parameters.spatialMode === "headphones" || selectedNode.parameters.spatialMode === "speakers"
            }
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "physicalInput" && (
          <PropertySection
            kind={selectedNode.kind}
            title="Spatial audio"
            summary={
              selectedNode.parameters.spatialMode === "headphones"
                ? "Headphones"
                : selectedNode.parameters.spatialMode === "speakers"
                  ? "Speakers"
                  : "Off"
            }
          >
            <SpatialAudioField
              devices={devices}
              endpointId={
                typeof selectedNode.parameters.endpointId === "string" ? selectedNode.parameters.endpointId : ""
              }
              mode={
                typeof selectedNode.parameters.spatialMode === "string" ? selectedNode.parameters.spatialMode : "off"
              }
              disabled={!backend.connected}
              roomPercent={
                typeof selectedNode.parameters.spatialRoomPercent === "number"
                  ? selectedNode.parameters.spatialRoomPercent
                  : 0
              }
              running={sessionRunning}
              onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
            />
          </PropertySection>
        )}
        {selectedNode.kind === "firFilter" && (
          <FirFilterEditor
            node={selectedNode}
            backend={backend}
            disabled={!backend.connected}
            onChange={(changes) => {
              for (const [name, value] of changes) changeNodeParameterOn(selectedNode.id, name, value);
            }}
          />
        )}
        {selectedNode.kind === "spectralGate" && (
          <SpectralGateEditor
            node={selectedNode}
            running={sessionRunning}
            levelsDb={
              liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.spectrum?.levelsDb ?? null
            }
            liveProfile={
              liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.noiseProfile ?? null
            }
            disabled={!backend.connected}
            onChange={(changes) => {
              for (const [name, value] of changes) changeNodeParameterOn(selectedNode.id, name, value);
            }}
          />
        )}
        {selectedNode.kind === "funVoiceChanger" && (
          <FunVoiceChangerEditor
            node={selectedNode}
            presets={snapshot?.discovery?.presets?.voiceChanger ?? null}
            telemetry={
              liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.voiceChanger ?? null
            }
            running={sessionRunning}
            processors={processors}
            nodeTypes={snapshot?.discovery?.nodeTypes ?? null}
            connected={backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
            onApply={(changes) => {
              for (const [name, value] of changes) changeNodeParameterOn(selectedNode.id, name, value);
            }}
          />
        )}
        {selectedNode.kind === "denoise" && (
          <DenoiseLearnEditor
            node={selectedNode}
            running={sessionRunning}
            liveProfile={
              liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.noiseProfile ?? null
            }
            disabled={!backend.connected}
            onChange={(changes) => {
              for (const [name, value] of changes) changeNodeParameterOn(selectedNode.id, name, value);
            }}
          />
        )}
        {selectedNode.kind === "dehum" && (
          <DehumEditor
            node={selectedNode}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "graphicEq" && (
          <GraphicEqEditor
            node={selectedNode}
            backend={backend}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "bassTreble" && (
          <BassTrebleEditor
            node={selectedNode}
            backend={backend}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "pitch" && (
          <PitchEditor
            node={selectedNode}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "delay" && (
          <DelayEditor
            node={selectedNode}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "inputSwitch" && (
          <InputSwitchEditor
            node={selectedNode}
            session={draft}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "outputSwitch" && (
          <OutputSwitchEditor
            node={selectedNode}
            session={draft}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {(selectedNode.kind === "declick" || selectedNode.kind === "speechDenoise") && (
          <StrengthEditor
            node={selectedNode}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {(selectedNode.kind === "volume" || selectedNode.kind === "gain") && (
          <LevelEditor
            node={selectedNode}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "mixer" && (
          <MixerInputsEditor
            session={draft}
            mixer={selectedNode}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "applicationCapture" && (
          <ApplicationCaptureBinding
            node={selectedNode}
            applications={applications}
            error={applicationsError}
            disabled={!backend.connected || sessionRunning}
            onRefresh={refreshApplications}
            onSelect={(application) => rebindApplicationCapture(selectedNode.id, application)}
          />
        )}
        {selectedNode.kind === "physicalOutput" && (
          <PhysicalOutputBinding
            devices={devices}
            value={typeof selectedNode.parameters.endpointId === "string" ? selectedNode.parameters.endpointId : ""}
            disabled={!backend.connected || sessionRunning}
            onRefresh={refreshDevices}
            onChange={(value) => {
              setRenderEndpointId(value);
              writeEndpointBindingHint(session.id, captureEndpointId, value);
              if (value) changeNodeParameterOn(selectedNode.id, "endpointId", value);
            }}
          />
        )}
        {(selectedNode.kind === "physicalInput" || selectedNode.kind === "physicalOutput") && (
          <DeviceAppsSection
            users={
              deviceUsers === undefined ? "unavailable" : deviceUsers && (deviceUsers.get(selectedNode.id) ?? null)
            }
          />
        )}
        {(selectedNode.kind === "networkSend" || selectedNode.kind === "networkReceive") && (
          <NetworkNodeEditor
            node={selectedNode}
            disabled={!backend.connected}
            telemetry={liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.network}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
        )}
        {selectedNode.kind === "recorder" && (
          <RecorderControls
            node={selectedNode}
            status={recorderStatuses.find((status) => status.nodeId === selectedNode.id)}
            running={sessionRunning}
            connected={backend.connected}
            busy={recordingBusy === selectedNode.id}
            lastPath={lastRecordingPaths[selectedNode.id] ?? null}
            message={recordingMessage?.nodeId === selectedNode.id ? recordingMessage.text : null}
            onToggle={(nodeId, record) => void toggleNodeRecording(nodeId, record)}
          />
        )}
        {selectedNode.kind === "recorder" && <RecordingFolderField backend={backend} connected={backend.connected} />}
        {selectedNode.kind === "audioFile" && (
          <AudioFileNodeEditor
            node={selectedNode}
            backend={backend}
            disabled={!backend.connected || sessionRunning}
            transportDisabled={!backend.connected}
            sessionRunning={sessionRunning}
            state={audioSourceStates[selectedNode.id] ?? "stopped"}
            sessionId={session.id}
            currentSessionIdRef={currentSessionIdRef}
            recorderNodes={session.nodes.filter(
              (item) => item.kind === "recorder" && item.enabled && recorderHasCaptureSource(session, item.id),
            )}
            recorderStatuses={recorderStatuses}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
            onTransport={(nodeId, action) => void transportAudioSource(nodeId, action)}
          />
        )}
        {selectedNode.kind === "soundboard" && (
          <SoundboardNodeEditor
            node={selectedNode}
            backend={backend}
            disabled={!backend.connected}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
            onRemove={(names) =>
              recordDraftChange(removeNodeDraftParameters(draft, selectedNode.id, names), `${selectedNode.id}:remove`)
            }
          />
        )}
        {selectedNode.kind === "plugin" && (
          <>
            <PluginEditorControls
              node={selectedNode}
              backend={backend}
              sessionId={session.id}
              running={sessionRunning}
              onStateSaved={(stateId) => changeNodeParameterOn(selectedNode.id, "stateId", stateId)}
            />
            <PluginNodeInspector
              node={selectedNode}
              disabled={!backend.connected}
              onUnload={() => unloadPluginNode(selectedNode.id)}
              snapshot={liveDiagnostics ?? null}
              backend={backend}
              sessionId={session.id}
              onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
              onRemove={(names) =>
                recordDraftChange(removeNodeDraftParameters(draft, selectedNode.id, names), `${selectedNode.id}:remove`)
              }
            />
            <PluginRecoveryControls
              node={selectedNode}
              backend={backend}
              sessionId={session.id}
              running={sessionRunning}
              disabledReason={
                JSON.stringify(session.nodes.find((node) => node.id === selectedNode.id)?.parameters) !==
                JSON.stringify(selectedNode.parameters)
                  ? "Save or discard changes to this tool before preparing recovery."
                  : undefined
              }
              onPrepared={(original, parameters) => {
                const current = draft.nodes.find((node) => node.id === original.id);
                if (!current || JSON.stringify(current.parameters) !== JSON.stringify(original.parameters))
                  return false;
                recordDraftChange(
                  {
                    ...draft,
                    nodes: draft.nodes.map((node) => (node.id === original.id ? { ...node, parameters } : node)),
                  },
                  "Prepared plugin recovery. Save to keep it.",
                );
                return true;
              }}
            />
          </>
        )}
        <ToolCompare backend={backend} session={session} node={selectedNode} running={sessionRunning} />
        {takesPresets(selectedNode) && (
          <PropertySection kind={selectedNode.kind} title="My presets" summary="Save and reuse settings">
            <ToolPresetPicker
              backend={backend}
              node={selectedNode}
              disabled={!backend.connected}
              onApply={(parameters, name) =>
                recordDraftChange(
                  {
                    ...draft,
                    nodes: draft.nodes.map((node) =>
                      node.id === selectedNode.id
                        ? { ...node, parameters: { ...node.parameters, ...parameters } }
                        : node,
                    ),
                  },
                  `Applied preset ${name}. Save to keep it.`,
                )
              }
            />
          </PropertySection>
        )}
        <div className="inspector-grid">
          <div className="inspector-toggles">
            <div className="inspector-flag">
              <label>
                Enabled
                <input
                  type="checkbox"
                  checked={selectedNode.enabled}
                  aria-describedby={`${flagId}-enabled`}
                  disabled={!backend.connected || sessionActionBusy || graphBusy}
                  onChange={(event) => changeNodeFlag("enabled", event.target.checked)}
                />
              </label>
              <span className="inspector-flag-state" id={`${flagId}-enabled`}>
                {selectedNode.enabled ? "On" : "Off"}
              </span>
            </div>
            <div className="inspector-flag">
              <label>
                Bypass
                <input
                  type="checkbox"
                  checked={selectedNode.bypass}
                  aria-describedby={`${flagId}-bypass`}
                  disabled={!backend.connected || sessionActionBusy || graphBusy}
                  onChange={(event) => changeNodeFlag("bypass", event.target.checked)}
                />
              </label>
              <span className="inspector-flag-state" id={`${flagId}-bypass`}>
                {selectedNode.bypass ? "On" : "Off"}
              </span>
            </div>
          </div>
          <EqSpectrumContext.Provider
            value={
              sessionRunning
                ? (liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.spectrum ?? null)
                : null
            }
          >
            {/* Compressor, Gate and Limiter show their settings under the curve and in a section. */}
            {!isDynamicsKind(selectedNode.kind) && (
              <ProcessorParameterEditor
                node={selectedNode}
                processors={processors}
                nodeTypes={snapshot?.discovery?.nodeTypes ?? null}
                connected={backend.connected}
                onChange={changeNodeParameter}
                compact
              />
            )}
          </EqSpectrumContext.Provider>
          {/* Privacy mute stays in plain sight in every tool, never behind a closed section (UI-10). */}
          <button
            type="button"
            className="secondary"
            onClick={() => void togglePrivacyMute()}
            disabled={!backend.connected}
          >
            {privacyMuted ? "Turn off privacy mute" : "Turn on privacy mute"}
          </button>
          <p className="muted">
            {!backend.connected
              ? "Controls are disabled while disconnected."
              : privacyMuted
                ? "Privacy mute is on: every microphone and captured app is silent, not only this tool."
                : "Privacy mute silences every microphone and captured app right away."}
          </p>
          <PropertySection kind={selectedNode.kind} title="Technical details" summary="Tool ID">
            <NodeIdentity nodeId={selectedNode.id} />
          </PropertySection>
          <div className="inspector-tool-actions">
            {processors?.some((processor) => processor.id === selectedNode.kind && processor.parameters.length > 0) && (
              <button
                type="button"
                className="secondary"
                aria-label="Reset parameters"
                onClick={resetNodeParameters}
                disabled={!backend.connected}
              >
                Reset
              </button>
            )}
            <button
              type="button"
              className="secondary"
              aria-label="Duplicate tool"
              onClick={duplicateSelectedNode}
              disabled={!backend.connected}
            >
              Duplicate
            </button>
            <button
              type="button"
              className="danger"
              aria-label="Remove tool"
              onClick={removeSelectedNode}
              disabled={!backend.connected}
            >
              Remove
            </button>
          </div>
        </div>
      </section>
    </>
  );
}

/** At most this many rows; the box keeps their height so polling never moves the panel (UI-17). */
const DEVICE_APP_ROWS = 3;

/** "Apps using this device" for a Physical Input or Output node (D1). */
function DeviceAppsSection({ users }: { users: NodeDeviceUsers | null | "unavailable" }) {
  const details = typeof users === "object" && users ? users : null;
  // A warning wraps over two rows, so its device names stay readable.
  const room = DEVICE_APP_ROWS - (details?.warning ? 2 : 0);
  const rows: { key: string; text: string; title?: string; warning?: boolean }[] = (details?.uses ?? []).map((use) => ({
    key: `${use.label} ${use.role}`,
    text: `${use.label} ${use.role}`,
    title: `${use.label} ${use.detail}`,
  }));
  if (rows.length > room) {
    const hidden = rows.length - room + 1;
    rows.splice(room - 1, rows.length, { key: "more", text: `and ${hidden} more` });
  }
  if (details?.warning) rows.unshift({ key: "warning", text: `Check: ${details.warning}`, warning: true });
  const empty =
    users === "unavailable"
      ? "Apps not checked: connect to AudioRouter."
      : details === null
        ? "Checking apps…"
        : rows.length === 0
          ? "No app is using it right now."
          : null;
  return (
    <section className="device-apps" aria-labelledby="device-apps-heading">
      <h4 id="device-apps-heading">Apps using this device</h4>
      <ul className="device-apps-list" aria-live="polite">
        {empty ? (
          <li className="muted">{empty}</li>
        ) : (
          rows.map((row) => (
            <li key={row.key} className={row.warning ? "is-warning" : undefined} title={row.title ?? row.text}>
              {row.text}
            </li>
          ))
        )}
      </ul>
    </section>
  );
}
