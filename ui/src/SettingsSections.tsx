// The sections that used to live in one Advanced tab, one export per new home:
// Settings pages, Status › Troubleshoot and recover, and the canvas keyboard panel.
import type { ReactElement } from "react";
import type { Node, Session } from "@audiorouter/contracts";
import type { UiBackend, UiBackendSnapshot } from "./backend";
import type { InsertableProcessorKind } from "./draft";
import { ApiAutostartSetting, AutoplaySetting } from "./AutoplaySetting";
import { GlobalKeybinds } from "./GlobalKeybinds";
import { GraphList as NodeList } from "./GraphList";
import { OsTransitionPanel, RecoveryCheckpointPanel, StartupPanel } from "./AdvancedPanels";
import { useMessages } from "./i18n/useMessages";

export function StartupSettings({ backend }: { backend: UiBackend }) {
  const tr = useMessages();
  return (
    <div className="startup-group">
      <p className="muted">{tr("settings.startup.intro")}</p>
      <StartupPanel backend={backend} />
      <AutoplaySetting />
      <ApiAutostartSetting />
    </div>
  );
}

/** Settings › Keyboard shortcuts: Windows-wide keybinds only (UI-10). */
export function ShortcutSettings({ snapshot }: { snapshot?: UiBackendSnapshot | null }) {
  return <GlobalKeybinds snapshot={snapshot} />;
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
  const tr = useMessages();
  return (
    <details className="troubleshoot-group">
      <summary>{tr("settings.troubleshoot.heading")}</summary>
      <details className="device-troubleshooting">
        <summary>{tr("settings.troubleshoot.manualBinding")}</summary>
        <p className="muted">{tr("settings.troubleshoot.manualHelp")}</p>
        {devicesContent}
      </details>
      <OsTransitionPanel backend={backend} onRefresh={refresh} />
      <section className="panel recovery-panel">
        <div className="section-heading">
          <h3>{tr("settings.troubleshoot.crashRecovery")}</h3>
          <span className="badge">
            {tr("settings.troubleshoot.recent", { count: snapshot?.status.recovery.recentCrashes ?? 0 })}
          </span>
        </div>
        <p className="muted">
          {snapshot?.status.recovery.safeMode
            ? tr("settings.troubleshoot.safeMode")
            : tr("settings.troubleshoot.normalMode")}{" "}
          {tr("settings.troubleshoot.recoveryOwned")}
        </p>
        <button
          type="button"
          className="secondary"
          onClick={() => void clearRecoverySafeMode()}
          disabled={!backend.connected || safetyActionBusyState || !snapshot?.status.recovery.safeMode}
        >
          {tr("settings.troubleshoot.clearSafeMode")}
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
  selectedNode: Node | undefined;
  selectNodeProperties: (id: string) => void;
  removeConnection: (edgeId: string) => void;
  toggleConnection: (edgeId: string, enabled: boolean) => void;
  insertProcessor: (edgeId: string, kind: InsertableProcessorKind) => void;
  openPluginPicker: (edgeId?: string) => void;
  connectionContent: ReactElement;
}) {
  const tr = useMessages();
  const title = mode === "routeList" ? tr("settings.canvasKeyboard.routeList") : tr("settings.canvasKeyboard.connect");
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
          {tr("settings.canvasKeyboard.close")}
        </button>
      </header>
      {mode === "routeList" ? (
        <NodeList
          session={draft}
          selectedNodeId={selectedNode?.id ?? ""}
          onSelect={selectNodeProperties}
          onRemoveConnection={removeConnection}
          onToggleConnection={toggleConnection}
          onInsertProcessor={insertProcessor}
          onOpenPluginPicker={openPluginPicker}
        />
      ) : (
        <>
          <p className="muted">{tr("settings.canvasKeyboard.connectHelp")}</p>
          {connectionContent}
        </>
      )}
    </section>
  );
}
