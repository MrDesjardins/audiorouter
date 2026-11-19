import { useMessages } from "./i18n/useMessages";

/** UI Options → Live autosave: save parameter edits while audio plays. */
export function LiveAutosaveSetting({ enabled, onChange }: { enabled: boolean; onChange: (enabled: boolean) => void }) {
  const tr = useMessages();
  return (
    <section className="panel live-autosave-panel" aria-labelledby="live-autosave-heading">
      <h3 id="live-autosave-heading">{tr("settings.liveAutosave.heading")}</h3>
      <div className="setting-toggle-row">
        <label className="autoplay-toggle">
          <input type="checkbox" checked={enabled} onChange={(event) => onChange(event.target.checked)} />
          {tr("settings.liveAutosave.label")}
        </label>
        <span className="setting-toggle-state">{enabled ? tr("common.on") : tr("common.off")}</span>
      </div>
      <p className="muted">{enabled ? tr("settings.liveAutosave.onHelp") : tr("settings.liveAutosave.offHelp")}</p>
    </section>
  );
}
