// The Signal flow panel: canvas, connection form and the connection, plug-in and application dialogs (moved from App.tsx).
import { useState } from "react";
import type { Dispatch, MouseEvent, ReactElement, RefObject, SetStateAction } from "react";
import type { DiagnosticsSnapshot, Node, PluginScanEntry, Session } from "@audiorouter/contracts";
import type { ApplicationRow, UiBackend } from "./backend";
import { applicationCaptureChoices, applicationChoiceKey } from "./draft";
import { LoadedPluginsPanel, PluginScanPanel } from "./PluginPanels";
import { ApplicationChoiceOptions, applicationChoiceLabel } from "./NodeEditors";

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
  addApplicationCapturePathFromPicker,
  addApplicationGroupFromPicker,
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
  selectedNode: Node | undefined;
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
  addApplicationCapturePathFromPicker: (path: string) => void;
  /** Returns whether the group was added (the picker then starts empty next time). */
  addApplicationGroupFromPicker: (applications: ApplicationRow[], executablePaths: string[]) => boolean;
}) {
  const tr = useMessages();
  const [applicationMode, setApplicationMode] = useState<"single" | "group">("single");
  const [groupApplicationKeys, setGroupApplicationKeys] = useState<string[]>([]);
  const [groupExecutablePaths, setGroupExecutablePaths] = useState<string[]>([]);
  const [executablePath, setExecutablePath] = useState("");
  const [pickerMessage, setPickerMessage] = useState("");
  return (
    <section id="signal-flow-panel" className="canvas-panel" aria-labelledby="canvas-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("canvasUi.signalFlow")}</p>
          <h2 id="canvas-heading">{tr("canvasUi.canvas")}</h2>
        </div>
      </div>
      {flowCanvas}
      <fieldset className="connection-editor" disabled={!backend.connected}>
        <legend>{tr("canvasUi.addConnection")}</legend>
        <label>
          {tr("canvasUi.output")}
          <select
            aria-label={tr("canvasUi.sourcePort")}
            value={connectionSource}
            onChange={(event) => setConnectionSource(event.target.value)}
          >
            <option value="">{tr("canvasUi.chooseSource")}</option>
            {outputPorts.map((port) => (
              <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
                {port.nodeName} · {port.portName} · {tr("g2.connChannels", { n: port.channels })}
              </option>
            ))}
          </select>
        </label>
        <span aria-hidden="true">→</span>
        <label>
          {tr("canvasUi.input")}
          <select
            aria-label={tr("canvasUi.destinationPort")}
            value={connectionDestination}
            onChange={(event) => setConnectionDestination(event.target.value)}
          >
            <option value="">{tr("canvasUi.chooseDestination")}</option>
            {inputPorts.map((port) => (
              <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
                {port.nodeName} · {port.portName} · {tr("g2.connChannels", { n: port.channels })}
              </option>
            ))}
          </select>
        </label>
        <button type="button" className="secondary" onClick={addConnection}>
          {tr("canvasUi.addConnection")}
        </button>
        <button type="button" className="secondary" onClick={openConnectionDialog}>
          {tr("signalFlowDialogs.keyboardConnectionOpen")}
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
              <h2 id="connection-dialog-heading">{tr("signalFlowDialogs.keyboardConnection")}</h2>
              <button
                type="button"
                className="secondary"
                onClick={closeConnectionDialog}
                aria-label={tr("signalFlowDialogs.keyboardConnectionClose")}
              >
                {tr("common.close")}
              </button>
            </div>
            <p id="connection-dialog-description" className="muted">
              {tr("signalFlowDialogs.keyboardConnectionHelp")}
            </p>
            <label>
              {tr("canvasUi.output")}
              <select
                ref={connectionDialogSource}
                aria-label={tr("signalFlowDialogs.keyboardSourceAria")}
                value={connectionSource}
                onChange={(event) => setConnectionSource(event.target.value)}
              >
                <option value="">{tr("canvasUi.chooseSource")}</option>
                {outputPorts.map((port) => (
                  <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
                    {port.nodeName} · {port.portName} · {tr("g2.connChannels", { n: port.channels })}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {tr("canvasUi.input")}
              <select
                aria-label={tr("signalFlowDialogs.keyboardDestinationAria")}
                value={connectionDestination}
                onChange={(event) => setConnectionDestination(event.target.value)}
              >
                <option value="">{tr("canvasUi.chooseDestination")}</option>
                {inputPorts.map((port) => (
                  <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
                    {port.nodeName} · {port.portName} · {tr("g2.connChannels", { n: port.channels })}
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
                {tr("canvasUi.addConnection")}
              </button>
              <button type="button" className="secondary" onClick={closeConnectionDialog}>
                {tr("common.cancel")}
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
                {tr(pluginInsertEdgeId ? "signalFlowDialogs.pluginInsertTitle" : "signalFlowDialogs.pluginAddTitle")}
              </h2>
              <button
                type="button"
                className="secondary"
                onClick={closePluginPicker}
                aria-label={tr("signalFlowDialogs.pluginPickerClose")}
              >
                {tr("common.close")}
              </button>
            </div>
            <p id="plugin-picker-description" className="muted">
              {tr(pluginInsertEdgeId ? "signalFlowDialogs.pluginHelpInsert" : "signalFlowDialogs.pluginHelpAdd")}
            </p>
            <LoadedPluginsPanel
              nodes={draft.nodes}
              selectedNodeId={selectedNode?.id ?? ""}
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
                  <h2 id="application-picker-heading">{tr("signalFlowDialogs.appPickerTitle")}</h2>
                  <button
                    type="button"
                    className="secondary"
                    onClick={closeApplicationPicker}
                    aria-label={tr("signalFlowDialogs.appPickerClose")}
                  >
                    {tr("common.close")}
                  </button>
                </div>
                <p id="application-picker-description" className="muted">
                  {tr("signalFlowDialogs.appPickerHelp")}
                </p>
                <div className="actions">
                  <button
                    type="button"
                    className={applicationMode === "single" ? "primary" : "secondary"}
                    aria-pressed={applicationMode === "single"}
                    onClick={() => setApplicationMode("single")}
                  >
                    {tr("signalFlowDialogs.oneApplication")}
                  </button>
                  <button
                    type="button"
                    className={applicationMode === "group" ? "primary" : "secondary"}
                    aria-pressed={applicationMode === "group"}
                    onClick={() => setApplicationMode("group")}
                  >
                    {tr("signalFlowDialogs.appGroup")}
                  </button>
                </div>
                <div className="actions">
                  <button type="button" className="secondary" onClick={refreshApplications}>
                    {tr("g5.appRefresh")}
                  </button>
                </div>
                <label>
                  {tr("signalFlowDialogs.exePathLabel")}
                  <input
                    aria-label={tr("signalFlowDialogs.exePathLabel")}
                    value={executablePath}
                    placeholder="C:\Games\game.exe"
                    onChange={(event) => setExecutablePath(event.target.value)}
                  />
                </label>
                <div className="actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={async () => {
                      const invoke = window.__TAURI_INTERNALS__?.invoke;
                      if (!invoke) {
                        setPickerMessage(tr("signalFlowDialogs.typePath"));
                        return;
                      }
                      try {
                        const path = await invoke("choose_application_executable");
                        if (typeof path === "string" && path) setExecutablePath(path);
                        setPickerMessage("");
                      } catch (error) {
                        setPickerMessage(error instanceof Error ? error.message : tr("signalFlowDialogs.pickerFailed"));
                      }
                    }}
                  >
                    {tr("signalFlowDialogs.browseExe")}
                  </button>
                  {applicationMode === "group" ? (
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => {
                        const path = executablePath.trim();
                        if (!path) {
                          setPickerMessage(tr("signalFlowDialogs.pathFirst"));
                          return;
                        }
                        setGroupExecutablePaths((paths) =>
                          paths.some((item) => item.toLocaleLowerCase() === path.toLocaleLowerCase())
                            ? paths
                            : [...paths, path],
                        );
                        setExecutablePath("");
                        setPickerMessage("");
                      }}
                    >
                      {tr("signalFlowDialogs.addPathToGroup")}
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="secondary"
                      disabled={!backend.connected}
                      onClick={() => {
                        addApplicationCapturePathFromPicker(executablePath);
                        setPickerMessage("");
                      }}
                    >
                      {tr("signalFlowDialogs.addThisAppSource")}
                    </button>
                  )}
                </div>
                <p className="muted application-picker-status" role="status" aria-live="polite">
                  <span title={pickerMessage || tr("signalFlowDialogs.pickerHint")}>
                    {pickerMessage || tr("signalFlowDialogs.pickerHint")}
                  </span>
                </p>
                {applicationsError ? (
                  <p className="muted" role="status">
                    {tr("g5.appInventoryError", { error: applicationsError })}
                  </p>
                ) : capturable.length === 0 ? (
                  <p className="muted" role="status">
                    {tr("signalFlowDialogs.noRunningApps")}
                  </p>
                ) : (
                  <>
                    {applicationMode === "single" ? (
                      <>
                        <label>
                          {tr("signalFlowDialogs.runningApplication")}
                          <select
                            aria-label={tr("g5.appSelectAria")}
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
                              ? tr("signalFlowDialogs.audioActive")
                              : tr("signalFlowDialogs.noAudioYet")}
                          </p>
                        )}
                        <div className="actions">
                          <button
                            type="button"
                            className="primary"
                            onClick={() => selected && addApplicationCaptureFromPicker(selected)}
                            disabled={!backend.connected || !selected}
                          >
                            {tr("signalFlowDialogs.addRunningApp")}
                          </button>
                        </div>
                      </>
                    ) : (
                      <fieldset className="application-group-options" disabled={!backend.connected}>
                        <legend>{tr("signalFlowDialogs.groupLegend")}</legend>
                        {capturable.map((application) => {
                          const key = applicationKey(application);
                          return (
                            <label key={key} className="checkbox-row">
                              <input
                                type="checkbox"
                                checked={groupApplicationKeys.includes(key)}
                                onChange={(event) =>
                                  setGroupApplicationKeys((keys) =>
                                    event.target.checked ? [...keys, key] : keys.filter((item) => item !== key),
                                  )
                                }
                              />
                              <span>{applicationChoiceLabel(application)}</span>
                            </label>
                          );
                        })}
                        {groupExecutablePaths.map((path) => (
                          <label key={path.toLocaleLowerCase()} className="checkbox-row">
                            <input
                              type="checkbox"
                              checked
                              onChange={() => setGroupExecutablePaths((paths) => paths.filter((item) => item !== path))}
                            />
                            <span>{path}</span>
                          </label>
                        ))}
                      </fieldset>
                    )}
                  </>
                )}
                {applicationMode === "group" && (
                  <button
                    type="button"
                    className="primary"
                    disabled={!backend.connected || groupApplicationKeys.length + groupExecutablePaths.length < 2}
                    onClick={() => {
                      const chosen = capturable.filter((application) =>
                        groupApplicationKeys.includes(applicationKey(application)),
                      );
                      if (addApplicationGroupFromPicker(chosen, groupExecutablePaths)) {
                        setGroupApplicationKeys([]);
                        setGroupExecutablePaths([]);
                      }
                    }}
                  >
                    {tr("signalFlowDialogs.createGroup")}
                  </button>
                )}
                <p className="muted">{tr("signalFlowDialogs.afterAdding")}</p>
              </section>
            </div>
          );
        })()}
    </section>
  );
}
import { useMessages } from "./i18n/useMessages";
