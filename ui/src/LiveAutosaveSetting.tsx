/** UI Options → Live autosave: save parameter edits while audio plays. */
export function LiveAutosaveSetting({ enabled, onChange }: { enabled: boolean; onChange: (enabled: boolean) => void }) {
  return (
    <section className="panel live-autosave-panel" aria-labelledby="live-autosave-heading">
      <h3 id="live-autosave-heading">Live autosave</h3>
      <div className="setting-toggle-row">
        <label className="autoplay-toggle">
          <input type="checkbox" checked={enabled} onChange={(event) => onChange(event.target.checked)} />
          Save parameter changes while audio plays
        </label>
        <span className="setting-toggle-state">{enabled ? "On" : "Off"}</span>
      </div>
      <p className="muted">
        {enabled
          ? "Changes to sliders and tool settings are saved automatically and applied to the playing audio. Changing the route still needs Save."
          : "Parameter changes stay as an unsaved draft until you choose Save. Turning a tool on or off still applies immediately while audio plays."}
      </p>
    </section>
  );
}
