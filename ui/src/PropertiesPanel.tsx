// The Properties panel: the selected node's (or canvas group's) inspector and editors (moved from App.tsx).
import type { Dispatch, RefObject, SetStateAction } from "react";
import type { DeviceListItem, DiagnosticsSnapshot, Node, Session } from "@audiorouter/contracts";
import type { ApplicationRow, RecorderStatus, UiBackend, UiBackendSnapshot } from "./backend";
import type { ProcessorDescriptor } from "./processorCatalog";
import { MeterInspector } from "./MeterInspector";
import { NodePropertyStatus } from "./NodePropertyStatus";
import { NodeIdentity } from "./NodeIdentity";
import { CanvasGroupInspector, type CanvasGroup, type useCanvasGroups } from "./CanvasGroups";
import { SpatialAudioField } from "./SpatialAudioField";
import { InputChannelsField } from "./InputChannelsField";
import { libraryEntries, toolDescription } from "./library";
import { TextField } from "./TextField";
import { SpectralGateEditor } from "./SpectralGateEditor";
import { DynamicsEditor } from "./DynamicsEditor";
import { DuckEditor } from "./DuckEditor";
import { RecorderControls, RecordingFolderField } from "./RecorderControls";
import {
  BassTrebleEditor,
  DehumEditor,
  DelayEditor,
  GraphicEqEditor,
  InputSwitchEditor,
  LevelEditor,
  PitchEditor,
  StrengthEditor,
} from "./ToolVisuals";
import { isDynamicsKind } from "./dynamics";
import { NetworkNodeEditor } from "./NetworkNodeEditor";
import { EqSpectrumContext } from "./AdvancedEqEditor";
import { writeEndpointBindingHint } from "./endpointBinding";
import { recorderHasCaptureSource } from "./RecordingPanels";
import { PluginEditorControls, PluginNodeInspector } from "./PluginPanels";
import {
  ApplicationCaptureBinding,
  DenoiseLearnEditor,
  FirFilterEditor,
  InspectorChangeSummary,
  MixerInputsEditor,
  NodeTelemetryPanel,
  PhysicalInputBinding,
  PhysicalOutputBinding,
  ProcessorParameterEditor,
} from "./NodeEditors";
import { AudioFileNodeEditor } from "./AudioFileNodeEditor";

export function PropertiesPanel({
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
  selectedGroup: CanvasGroup | undefined;
  groupState: ReturnType<typeof useCanvasGroups>;
  selectedNode: Node;
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
  return selectedGroup ? (
    <CanvasGroupInspector
      group={selectedGroup}
      onChange={(patch) => groupState.changeGroup(selectedGroup.id, patch)}
      onRemove={() => groupState.removeGroup(selectedGroup.id)}
    />
  ) : (
    <>
      <section className="panel inspector" aria-labelledby="inspector-heading">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Selected node</p>
            <div className="inspector-title">
              <h2 id="inspector-heading">{selectedNode.name}</h2>
              <NodePropertyStatus
                node={selectedNode}
                connected={backend.connected}
                running={sessionRunning}
                snapshot={liveDiagnostics ?? null}
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
        <p className="muted">
          For delay and processing measurements, open Timing while audio plays. Ready means enabled for the next
          playback; Active does not guarantee an incoming signal.
        </p>
        <InspectorChangeSummary
          draftNode={selectedNode}
          authoritativeNode={session.nodes.find((node) => node.id === selectedNode.id)}
        />
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
              setActionMessage("Meter output added to the draft. Connect it downstream and Save to keep it.");
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
          />
        ) : (
          <NodeTelemetryPanel node={selectedNode} snapshot={liveDiagnostics ?? null} running={sessionRunning} />
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
          <SpatialAudioField
            devices={devices}
            endpointId={
              typeof selectedNode.parameters.endpointId === "string" ? selectedNode.parameters.endpointId : ""
            }
            mode={typeof selectedNode.parameters.spatialMode === "string" ? selectedNode.parameters.spatialMode : "off"}
            disabled={!backend.connected}
            roomPercent={
              typeof selectedNode.parameters.spatialRoomPercent === "number"
                ? selectedNode.parameters.spatialRoomPercent
                : 0
            }
            running={sessionRunning}
            onChange={(name, value) => changeNodeParameterOn(selectedNode.id, name, value)}
          />
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
            />
          </>
        )}
        <details className="inspector-help">
          <summary>How Enabled and Bypass work</summary>
          <p className="muted">
            Off silences inputs and outputs; an off effect passes sound without processing. Bypass passes sound around
            an effect; on a prepared input, output, or Mixer it silences that contribution. Live toggles do not require
            Save or Stop and keep other draft edits. Use Mute to silence a route. Enabled and Bypass apply to prepared
            nodes while audio keeps playing. Off devices remain open until Stop and contribute silence.
          </p>
        </details>
        <div className="inspector-grid">
          <label>
            Node name
            <TextField
              key={selectedNode.id}
              maxLength={120}
              value={selectedNode.name}
              disabled={!backend.connected}
              onValue={changeNodeName}
            />
          </label>
          <div className="inspector-toggles">
            <label>
              Enabled
              <input
                type="checkbox"
                checked={selectedNode.enabled}
                disabled={!backend.connected || sessionActionBusy || graphBusy}
                onChange={(event) => changeNodeFlag("enabled", event.target.checked)}
              />
            </label>
            <label>
              Bypass
              <input
                type="checkbox"
                checked={selectedNode.bypass}
                disabled={!backend.connected || sessionActionBusy || graphBusy}
                onChange={(event) => changeNodeFlag("bypass", event.target.checked)}
              />
            </label>
          </div>
          <EqSpectrumContext.Provider
            value={
              sessionRunning
                ? (liveDiagnostics?.nodeTelemetry.find((item) => item.nodeId === selectedNode.id)?.spectrum ?? null)
                : null
            }
          >
            <ProcessorParameterEditor
              node={selectedNode}
              processors={processors}
              nodeTypes={snapshot?.discovery?.nodeTypes ?? null}
              connected={backend.connected}
              onChange={changeNodeParameter}
            />
          </EqSpectrumContext.Provider>
          {processors?.some((processor) => processor.id === selectedNode.kind && processor.parameters.length > 0) && (
            <button type="button" className="secondary" onClick={resetNodeParameters} disabled={!backend.connected}>
              Reset parameters
            </button>
          )}
          <button type="button" className="secondary" onClick={duplicateSelectedNode} disabled={!backend.connected}>
            Duplicate node to draft
          </button>
          <button type="button" className="secondary" onClick={removeSelectedNode} disabled={!backend.connected}>
            Remove node from draft
          </button>
          <button
            type="button"
            onClick={() => void togglePrivacyMute()}
            disabled={!backend.connected}
            aria-pressed={privacyMuted}
          >
            {privacyMuted ? "Privacy mute enabled" : "Enable privacy mute"}
          </button>
          <p className="muted">
            {backend.connected
              ? "Plan changes reviews the draft; Commit changes saves it. Privacy mute is an immediate safety latch."
              : "Controls are disabled while disconnected. Selection is local presentation state only."}
          </p>
        </div>
        <NodeIdentity nodeId={selectedNode.id} />
      </section>
    </>
  );
}
