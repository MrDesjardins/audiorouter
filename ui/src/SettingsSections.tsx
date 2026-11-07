// The sections that used to live in one Advanced tab, one export per new home:
// Settings pages, Status › Troubleshoot and recover, and the canvas keyboard panel.
import type { KeyboardEvent, ReactElement } from "react";
import type { Node, Session } from "@audiorouter/contracts";
import type { UiBackend, UiBackendSnapshot } from "./backend";
import type { InsertableProcessorKind } from "./draft";
import type { ShortcutAction, ShortcutBinding } from "./shortcuts";
import { ApiAutostartSetting, AutoplaySetting } from "./AutoplaySetting";
import { GraphList as NodeList } from "./GraphList";
import { OsTransitionPanel, RecoveryCheckpointPanel, StartupPanel } from "./AdvancedPanels";

export function StartupSettings({ backend }: { backend: UiBackend }) {
  return (
    <div className="startup-group">
      <p className="muted">
        Start with Windows in the tray (no window) and, if you like, play the selected session and start the API right
        away. Closing the window keeps audio playing in the tray.
      </p>
      <StartupPanel backend={backend} />
      <AutoplaySetting />
      <ApiAutostartSetting />
    </div>
  );
}

export function ShortcutSettings({
  shortcuts,
  captureShortcut,
  shortcutMessage,
}: {
  shortcuts: ShortcutBinding;
  captureShortcut: (action: ShortcutAction, event: KeyboardEvent<HTMLInputElement>) => void;
  shortcutMessage: string | null;
}) {
  return (
    <section className="panel shortcut-panel">
      <h3>Local shortcuts</h3>
      <p className="muted">These work while AudioRouter is focused and never capture typing in a text field.</p>
      <p className="muted">
        Ctrl+S saves your edits. Ctrl+K finds tools, settings and actions. Tab starts with links to skip to the canvas
        or the workbench.
      </p>
      <p className="muted">
        On the canvas: Ctrl+A selects all tools and groups; Ctrl+D duplicates one selected tool. Ctrl+plus and
        Ctrl+minus zoom; Ctrl+0 fits the route. Arrow keys move a focused card. F2 renames one selected tool. F1 or ?
        opens the shortcut reference.
      </p>
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
      <small>These are app shortcuts. Windows-wide shortcuts are not available here.</small>
    </section>
  );
}

/** Status › Troubleshoot and recover. */
export function TroubleshootPanel({
  backend,
  devicesContent,
  refresh,
  snapshot,
  clearRecoverySafeMode,
  safetyActionBusyState,
}: {
  backend: UiBackend;
  devicesContent: ReactElement;
  refresh: () => Promise<void>;
  snapshot: UiBackendSnapshot | null;
  clearRecoverySafeMode: () => Promise<void>;
  safetyActionBusyState: boolean;
}) {
  return (
    <details className="troubleshoot-group">
      <summary>Troubleshoot and recover</summary>
      <details className="device-troubleshooting">
        <summary>Manual device binding</summary>
        <p className="muted">
          You normally do not need this. Choose each device in the Input Device or Output Device tool's Properties and
          press Play. Use these controls to check a device's format, reopen a device after Windows reset it, run a
          deliberate VB-Cable loopback test, or add a loopback source.
        </p>
        {devicesContent}
      </details>
      <OsTransitionPanel backend={backend} onRefresh={refresh} />
      <section className="panel recovery-panel">
        <div className="section-heading">
          <h3>Crash recovery</h3>
          <span className="badge">{snapshot?.status.recovery.recentCrashes ?? 0} recent</span>
        </div>
        <p className="muted">
          {snapshot?.status.recovery.safeMode ? "Safe mode is active." : "Normal startup mode."} Recovery state is owned
          by the backend.
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
  );
}

export type CanvasKeyboardPanelMode = "routeList" | "connect";

/** Opened from the canvas toolbar: the route as a list, and connecting without dragging (UI-11). */
export function CanvasKeyboardPanel({
  mode,
  onClose,
  draft,
  selectedNode,
  selectNodeProperties,
  removeConnection,
  toggleConnection,
  insertProcessor,
  openPluginPicker,
  connectionContent,
}: {
  mode: CanvasKeyboardPanelMode;
  onClose: () => void;
  draft: Session;
  selectedNode: Node;
  selectNodeProperties: (id: string) => void;
  removeConnection: (edgeId: string) => void;
  toggleConnection: (edgeId: string, enabled: boolean) => void;
  insertProcessor: (edgeId: string, kind: InsertableProcessorKind) => void;
  openPluginPicker: (edgeId?: string) => void;
  connectionContent: ReactElement;
}) {
  const title = mode === "routeList" ? "Route list" : "Connect without dragging";
  return (
    <section
      className="canvas-keyboard-panel"
      aria-label={title}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        }
      }}
    >
      <header>
        <h3>{title}</h3>
        <button type="button" className="secondary" onClick={onClose}>
          Close
        </button>
      </header>
      {mode === "routeList" ? (
        <NodeList
          session={draft}
          selectedNodeId={selectedNode.id}
          onSelect={selectNodeProperties}
          onRemoveConnection={removeConnection}
          onToggleConnection={toggleConnection}
          onInsertProcessor={insertProcessor}
          onOpenPluginPicker={openPluginPicker}
        />
      ) : (
        <>
          <p className="muted">
            The same as dragging from one tool to another on the canvas, for keyboard and screen-reader use.
          </p>
          {connectionContent}
        </>
      )}
    </section>
  );
}
