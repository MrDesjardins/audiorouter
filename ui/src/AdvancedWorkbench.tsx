// Content of the Advanced side-panel tab (moved from App.tsx).
import type { Dispatch, KeyboardEvent, ReactElement, SetStateAction } from "react";
import type { DiscoveryDocument, Node, PluginScanEntry, Session } from "@audiorouter/contracts";
import type { ApplicationRow, UiBackend, UiBackendSnapshot } from "./backend";
import type { InsertableProcessorKind } from "./draft";
import type { ShortcutAction, ShortcutBinding } from "./shortcuts";
import type { ProcessorDescriptor } from "./processorCatalog";
import { ApiAutostartSetting, AutoplaySetting } from "./AutoplaySetting";
import { ApplicationIdentityPanel } from "./ApplicationIdentityPanel";
import { GraphList as NodeList } from "./GraphList";
import {
  ClientsPanel,
  GraphHistoryPanel,
  OsTransitionPanel,
  PresetCatalog,
  ProcessorCatalog,
  RecoveryCheckpointPanel,
  SessionTransferPanel,
  StartupPanel,
} from "./AdvancedPanels";
import { PluginScanPanel } from "./PluginPanels";

export function AdvancedWorkbench({
  backend,
  draft,
  selectedNode,
  selectNodeProperties,
  removeConnection,
  toggleConnection,
  insertProcessor,
  openPluginPicker,
  connectionWorkbenchContent,
  devicesWorkbenchContent,
  shortcuts,
  captureShortcut,
  shortcutMessage,
  addPluginToDraft,
  processors,
  processorError,
  presets,
  presetError,
  session,
  setCreatedSessions,
  setSelectedSessionId,
  refresh,
  applications,
  snapshot,
  clearRecoverySafeMode,
  safetyActionBusyState,
}: {
  backend: UiBackend;
  draft: Session;
  selectedNode: Node;
  selectNodeProperties: (id: string) => void;
  removeConnection: (edgeId: string) => void;
  toggleConnection: (edgeId: string, enabled: boolean) => void;
  insertProcessor: (edgeId: string, kind: InsertableProcessorKind) => void;
  openPluginPicker: (edgeId?: string) => void;
  connectionWorkbenchContent: ReactElement;
  devicesWorkbenchContent: ReactElement;
  shortcuts: ShortcutBinding;
  captureShortcut: (action: ShortcutAction, event: KeyboardEvent<HTMLInputElement>) => void;
  shortcutMessage: string | null;
  addPluginToDraft: (entry: PluginScanEntry) => void;
  processors: ProcessorDescriptor[] | null;
  processorError: string | null;
  presets: DiscoveryDocument["presets"] | null;
  presetError: string | null;
  session: Session;
  setCreatedSessions: Dispatch<SetStateAction<Session[]>>;
  setSelectedSessionId: Dispatch<SetStateAction<string>>;
  refresh: () => Promise<void>;
  applications: ApplicationRow[];
  snapshot: UiBackendSnapshot | null;
  clearRecoverySafeMode: () => Promise<void>;
  safetyActionBusyState: boolean;
}) {
  return (
    <div className="workbench-groups">
      <details className="startup-group">
        <summary>When AudioRouter starts</summary>
        <p className="muted">
          Start with Windows in the tray (no window) and, if you like, play the selected session and start the API right
          away. Closing the window keeps audio playing in the tray.
        </p>
        <StartupPanel backend={backend} />
        <AutoplaySetting />
        <ApiAutostartSetting />
      </details>
      <details>
        <summary>Keyboard graph controls</summary>
        <NodeList
          session={draft}
          selectedNodeId={selectedNode.id}
          onSelect={selectNodeProperties}
          onRemoveConnection={removeConnection}
          onToggleConnection={toggleConnection}
          onInsertProcessor={insertProcessor}
          onOpenPluginPicker={openPluginPicker}
        />
      </details>
      <details className="connection-form-group">
        <summary>Connect nodes without dragging</summary>
        <p className="muted">
          The same as dragging from one node to another on the canvas, for keyboard and screen-reader use.
        </p>
        {connectionWorkbenchContent}
      </details>
      <details className="device-troubleshooting">
        <summary>Troubleshooting: manual device binding</summary>
        <p className="muted">
          You normally do not need this. Choose each device in the Input Device or Output Device node's Properties and
          press Play. Use these controls to check a device's format, reopen a device after Windows reset it, run a
          deliberate VB-Cable loopback test, or add a loopback source.
        </p>
        {devicesWorkbenchContent}
      </details>
      <details>
        <summary>Keyboard shortcuts</summary>
        <section className="panel shortcut-panel">
          <h3>Local shortcuts</h3>
          <p className="muted">These work while AudioRouter is focused and never capture typing in a text field.</p>
          <label>
            Start or stop session
            <input
              aria-label="Start or stop session shortcut"
              value={shortcuts.sessionToggle}
              readOnly
              onKeyDown={(event) => captureShortcut("sessionToggle", event)}
            />
          </label>
          <label>
            Privacy mute
            <input
              aria-label="Privacy mute shortcut"
              value={shortcuts.privacyMute}
              readOnly
              onKeyDown={(event) => captureShortcut("privacyMute", event)}
            />
          </label>
          {shortcutMessage && (
            <p className="muted" role="alert">
              {shortcutMessage}
            </p>
          )}
          <small>Native tray and OS-wide registration remain platform validation work.</small>
        </section>
      </details>
      <details>
        <summary>Plug-ins</summary>
        <PluginScanPanel backend={backend} onAddPlaceholder={addPluginToDraft} />
      </details>
      <details>
        <summary>Built-in processors and presets</summary>
        <ProcessorCatalog processors={processors} error={processorError} node={selectedNode} backend={backend} />
        <PresetCatalog presets={presets} error={presetError} />
      </details>
      <details>
        <summary>JSON graph transfer (for scripts)</summary>
        <SessionTransferPanel
          backend={backend}
          session={session}
          onImported={(imported) => {
            setCreatedSessions((current) => [...current.filter((item) => item.id !== imported.id), imported]);
            setSelectedSessionId(imported.id);
            void refresh();
          }}
        />
      </details>
      <details>
        <summary>Resume after sleep and revision history</summary>
        <OsTransitionPanel backend={backend} onRefresh={refresh} />
        <GraphHistoryPanel backend={backend} session={session} onReverted={refresh} />
      </details>
      <details>
        <summary>Application identity and recovery</summary>
        <ApplicationIdentityPanel applications={applications} />
        <section className="panel recovery-panel">
          <div className="section-heading">
            <h3>Crash recovery</h3>
            <span className="badge">{snapshot?.status.recovery.recentCrashes ?? 0} recent</span>
          </div>
          <p className="muted">
            {snapshot?.status.recovery.safeMode ? "Safe mode is active." : "Normal startup mode."} Recovery state is
            owned by the backend.
          </p>
          <button
            type="button"
            className="secondary"
            onClick={() => void clearRecoverySafeMode()}
            disabled={!backend.connected || safetyActionBusyState || !snapshot?.status.recovery.safeMode}
          >
            Clear safe mode
          </button>
        </section>
        <RecoveryCheckpointPanel backend={backend} />
      </details>
      <details>
        <summary>Connected clients</summary>
        <ClientsPanel backend={backend} />
      </details>
    </div>
  );
}
