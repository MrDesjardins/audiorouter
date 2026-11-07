import { APP_VERSION, type AvailableUpdate } from "./updateCheck";

/** Open a release page in the default browser (desktop) or a new tab. */
export function openReleasePage(update: AvailableUpdate) {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  if (invoke)
    void invoke("open_release_page", { tag: update.tag }).catch(() => window.open(update.url, "_blank", "noopener"));
  else window.open(update.url, "_blank", "noopener");
}

/**
 * "AudioRouter 0.0.8" in the header, followed by "0.0.9 available" when a
 * newer release exists. It stays on the header's single line, so the notice
 * never moves other content (UI-17).
 */
export function VersionLine({ update }: { update: AvailableUpdate | null }) {
  return (
    <p className="eyebrow app-version-line">
      AudioRouter{" "}
      <span className="app-version" aria-label={`Version ${APP_VERSION}`}>
        {APP_VERSION}
      </span>
      {update && (
        <>
          {" "}
          ·{" "}
          <button
            type="button"
            className="update-link"
            title={`Open the AudioRouter ${update.version} release page`}
            onClick={() => openReleasePage(update)}
          >
            {update.version} available
          </button>
        </>
      )}
    </p>
  );
}

/** Advanced settings: turn the daily new-version check on or off. */
export function UpdatesPanel({
  enabled,
  onChange,
  update,
}: {
  enabled: boolean;
  onChange: (enabled: boolean) => void;
  update: AvailableUpdate | null;
}) {
  return (
    <section className="panel updates-panel" aria-labelledby="updates-heading">
      <h3 id="updates-heading">New versions</h3>
      <p className="muted updates-summary">
        You have AudioRouter {APP_VERSION}.{" "}
        {enabled
          ? update
            ? `Version ${update.version} is available.`
            : "New versions appear here when a check finds one."
          : "Checking is off."}
      </p>
      <div className="setting-toggle-row">
        <label className="updates-toggle">
          <input
            type="checkbox"
            checked={enabled}
            aria-describedby="update-check-state"
            onChange={(event) => onChange(event.target.checked)}
          />
          Check GitHub once a day for a newer version
        </label>
        <span className="setting-toggle-state" id="update-check-state">
          {enabled ? "On" : "Off"}
        </span>
      </div>
      <small className="muted">
        Reads the public AudioRouter release list on github.com; nothing about you or your audio is sent. When a newer
        version exists, a link appears next to the version at the top of the window.
      </small>
      <div className="update-release-action">
        <button
          type="button"
          className="secondary"
          disabled={!update}
          onClick={() => {
            if (update) openReleasePage(update);
          }}
        >
          {update ? `Open the ${update.version} release page` : "No release to open"}
        </button>
      </div>
    </section>
  );
}
