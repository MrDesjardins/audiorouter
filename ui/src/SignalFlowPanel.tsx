// The Signal flow panel: canvas, connection form and the connection, plug-in and application dialogs (moved from App.tsx).
import type { Dispatch, MouseEvent, ReactElement, RefObject, SetStateAction } from "react";
import type { DiagnosticsSnapshot, Node, PluginScanEntry, Session } from "@audiorouter/contracts";
import type { ApplicationRow, UiBackend } from "./backend";
import { applicationCaptureChoices, applicationChoiceKey } from "./draft";
import { LoadedPluginsPanel, PluginScanPanel } from "./PluginPanels";
import { ApplicationChoiceOptions } from "./NodeEditors";

export function SignalFlowPanel({
  flowCanvas,
  backend,
  connectionSource,
  setConnectionSource,
  outputPorts,
  encodePort,
  connectionDestination,
  setConnectionDestination,
  inputPorts,
  addConnection,
  openConnectionDialog,
  connectionDialogOpen,
  connectionDialog,
  closeConnectionDialog,
  connectionDialogSource,
  pluginPickerOpen,
  pluginPickerDialog,
  pluginInsertEdgeId,
  closePluginPicker,
  draft,
  selectedNode,
  setSelectedNodeId,
  setSelectedNodeIds,
  unloadPluginNode,
  liveDiagnostics,
  addPluginToDraft,
  applicationPickerOpen,
  applications,
  applicationPickerSelection,
  applicationPickerDialog,
  closeApplicationPicker,
  refreshApplications,
  applicationsError,
  setApplicationPickerSelection,
  addApplicationCaptureFromPicker,
}: {
  flowCanvas: ReactElement;
  backend: UiBackend;
  connectionSource: string;
  setConnectionSource: Dispatch<SetStateAction<string>>;
  outputPorts: { nodeId: string; nodeName: string; portName: string; channels: number }[];
  encodePort: (nodeId: string, portName: string) => string;
  connectionDestination: string;
  setConnectionDestination: Dispatch<SetStateAction<string>>;
  inputPorts: { nodeId: string; nodeName: string; portName: string; channels: number }[];
  addConnection: () => boolean;
  openConnectionDialog: (event: MouseEvent<HTMLButtonElement>) => void;
  connectionDialogOpen: boolean;
  connectionDialog: RefObject<HTMLElement | null>;
  closeConnectionDialog: () => void;
  connectionDialogSource: RefObject<HTMLSelectElement | null>;
  pluginPickerOpen: boolean;
  pluginPickerDialog: RefObject<HTMLElement | null>;
  pluginInsertEdgeId: string | null;
  closePluginPicker: () => void;
  draft: Session;
  selectedNode: Node;
  setSelectedNodeId: Dispatch<SetStateAction<string>>;
  setSelectedNodeIds: (ids: string[]) => void;
  unloadPluginNode: (nodeId: string) => void;
  liveDiagnostics: DiagnosticsSnapshot | null;
  addPluginToDraft: (entry: PluginScanEntry) => void;
  applicationPickerOpen: boolean;
  applications: ApplicationRow[];
  applicationPickerSelection: string;
  applicationPickerDialog: RefObject<HTMLElement | null>;
  closeApplicationPicker: () => void;
  refreshApplications: () => void;
  applicationsError: string | null;
  setApplicationPickerSelection: Dispatch<SetStateAction<string>>;
  addApplicationCaptureFromPicker: (application: ApplicationRow) => void;
}) {
  return (
    <section id="signal-flow-panel" className="canvas-panel" aria-labelledby="canvas-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Signal flow</p>
          <h2 id="canvas-heading">Canvas</h2>
        </div>
      </div>
      <p className="muted canvas-handle-legend">
        <span className="canvas-handle-legend-dot canvas-handle-legend-dot-target" aria-hidden="true" /> Send (start)
        <span className="canvas-handle-legend-dot canvas-handle-legend-dot-source" aria-hidden="true" /> Receive (end) -
        drag from the sending tool's blue dot to the receiving tool's orange dot.
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
              <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
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
              <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
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
                  <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
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
                  <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
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
                {pluginInsertEdgeId ? "Insert a VST2/VST3 plugin into this connection" : "Add a VST2/VST3 plugin"}
              </h2>
              <button type="button" className="secondary" onClick={closePluginPicker} aria-label="Close plugin picker">
                Close
              </button>
            </div>
            <p id="plugin-picker-description" className="muted">
              Scan an absolute directory on this machine for supported x64 VST2/VST3 binaries, then{" "}
              {pluginInsertEdgeId ? "insert one directly into the connection" : "add one as a stopped node"}. Press
              Escape to close.
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
            capturable.find((application) => applicationKey(application) === applicationPickerSelection) ??
            capturable[0];
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
                  Pick a running application to capture its audio as an input node. After restart, AudioRouter
                  reconnects when it finds one verified matching instance; if several match, choose the intended
                  instance again. Playback does not start just because the app is open.
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
                  For advanced options (application-capture policy, preparing the native worker), use the Applications
                  panel in Full workspace mode.
                </p>
              </section>
            </div>
          );
        })()}
    </section>
  );
}
