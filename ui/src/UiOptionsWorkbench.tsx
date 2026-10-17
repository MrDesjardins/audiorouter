import { useState } from "react";
import { FlowAnimationSetting } from "./FlowAnimationSetting";
import { type FlowAnimationMode, type ThemeMode } from "./preferences";
import {
  DEFAULT_CUSTOM_THEME,
  MAX_THEME_BYTES,
  THEME_COLORS,
  addCustomTheme,
  deleteCustomTheme,
  parseCustomTheme,
  selectedCustomTheme,
  uniqueThemeName,
  updateCustomTheme,
  type CustomTheme,
  type CustomThemeLibrary,
  type ThemeColor,
} from "./customTheme";
import { TextField } from "./TextField";

const CUSTOM_PREFIX = "custom:";

export function UiOptionsWorkbench({
  theme,
  onTheme,
  customThemes,
  onCustomThemes,
  flowAnimation,
  onFlowAnimation,
}: {
  theme: ThemeMode;
  onTheme: (theme: ThemeMode) => void;
  customThemes: CustomThemeLibrary;
  onCustomThemes: (library: CustomThemeLibrary) => void;
  flowAnimation: FlowAnimationMode;
  onFlowAnimation: (mode: FlowAnimationMode) => void;
}) {
  const [message, setMessage] = useState("");
  const [error, setError] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const selected = selectedCustomTheme(customThemes);
  const active = theme === "custom" ? selected : null;
  const report = (text: string, isError = false) => {
    setError(isError);
    setMessage(text);
  };
  const exportTheme = (customTheme: CustomTheme) => {
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(customTheme, null, 2) + "\n"], { type: "application/json" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download =
      (customTheme.name
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "") || "audiorouter-theme") + ".json";
    link.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
    report("Theme exported. The file contains colors and appearance only.");
  };
  /** Add a theme and switch to it; false when the library is full. */
  const add = (next: CustomTheme): CustomThemeLibrary | null => {
    try {
      const library = addCustomTheme(customThemes, next);
      onCustomThemes(library);
      onTheme("custom");
      setConfirmDelete(false);
      return library;
    } catch (cause) {
      report(cause instanceof Error ? cause.message : "Could not add the theme.", true);
      return null;
    }
  };
  const updateActive = (next: CustomTheme) => {
    if (active) onCustomThemes(updateCustomTheme(customThemes, active.id, next));
  };
  const selectValue = active ? CUSTOM_PREFIX + active.id : theme === "custom" ? "dark" : theme;
  return (
    <div className="ui-options">
      <section className="panel theme-options" aria-labelledby="theme-options-title">
        <h3 id="theme-options-title">Appearance</h3>
        <label>
          Color theme
          <select
            aria-label="Color theme"
            value={selectValue}
            onChange={(event) => {
              const value = event.target.value;
              setConfirmDelete(false);
              if (value.startsWith(CUSTOM_PREFIX)) {
                onCustomThemes({ ...customThemes, selectedId: value.slice(CUSTOM_PREFIX.length) });
                onTheme("custom");
              } else onTheme(value as ThemeMode);
            }}
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
            <option value="high-contrast">High contrast</option>
            {customThemes.themes.map((entry) => (
              <option key={entry.id} value={CUSTOM_PREFIX + entry.id}>
                {entry.theme.name} (custom)
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="secondary"
          onClick={() => {
            // Start from the theme on screen when it is custom, else the default palette.
            const start = active?.theme ?? DEFAULT_CUSTOM_THEME;
            if (add({ ...start, name: active ? `${start.name} copy` : DEFAULT_CUSTOM_THEME.name }))
              report("New theme created. Colors save automatically on this PC.");
          }}
        >
          Create a new theme
        </button>
        <label>
          Import theme file
          <input
            type="file"
            accept=".json,application/json"
            aria-label="Import theme file"
            onChange={(event) => {
              const file = event.target.files?.[0];
              event.target.value = "";
              if (!file) return;
              if (file.size > MAX_THEME_BYTES) {
                report("Theme files must be 32 KB or smaller.", true);
                return;
              }
              void file
                .text()
                .then((text) => {
                  const library = add(parseCustomTheme(text));
                  if (library) report(`Imported ${selectedCustomTheme(library)?.theme.name ?? "the theme"}.`);
                })
                .catch((cause: unknown) => {
                  report(cause instanceof Error ? cause.message : "Could not read the theme file.", true);
                });
            }}
          />
        </label>
        <button
          type="button"
          className="secondary"
          disabled={!active}
          title={active ? undefined : "Choose a custom theme to export it."}
          onClick={() => active && exportTheme(active.theme)}
        >
          Export this theme
        </button>
        <p className={`theme-file-message${error ? " is-error" : ""}`} role={error ? "alert" : "status"}>
          {message || "Themes are local appearance settings. Importing a file never changes your audio or session."}
        </p>
        {active && (
          <div className="theme-editor">
            <label>
              Theme name
              <TextField
                aria-label="Theme name"
                value={active.theme.name}
                maxLength={80}
                onValue={(name) => updateActive({ ...active.theme, name })}
                onBlur={() => {
                  const unique = uniqueThemeName(customThemes.themes, active.theme.name, active.id);
                  if (unique !== active.theme.name) updateActive({ ...active.theme, name: unique });
                }}
              />
            </label>
            <label>
              Base appearance
              <select
                aria-label="Base appearance"
                value={active.theme.base}
                onChange={(event) => updateActive({ ...active.theme, base: event.target.value as "dark" | "light" })}
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
              </select>
            </label>
            <div className="theme-preview" aria-label="Custom theme preview">
              <strong>Your sound. Your workspace.</strong>
              <span>Panel text and fields use this palette.</span>
              <span className="theme-preview-accent">Selected tool</span>
            </div>
            <details>
              <summary>Edit palette colors</summary>
              <div className="theme-palette">
                {(Object.entries(THEME_COLORS) as [ThemeColor, string][]).map(([key, label]) => (
                  <label key={key}>
                    {label}
                    <input
                      type="color"
                      aria-label={label + " color"}
                      value={active.theme.colors[key]}
                      onChange={(event) =>
                        updateActive({ ...active.theme, colors: { ...active.theme.colors, [key]: event.target.value } })
                      }
                    />
                    <output>{active.theme.colors[key]}</output>
                  </label>
                ))}
              </div>
            </details>
            {confirmDelete ? (
              <div className="theme-delete-confirmation" role="group" aria-label="Confirm theme deletion">
                <p>Delete {active.theme.name}? This cannot be undone. Export it first to keep a copy.</p>
                <div className="theme-delete-actions">
                  <button
                    type="button"
                    className="danger"
                    onClick={() => {
                      const name = active.theme.name;
                      const library = deleteCustomTheme(customThemes, active.id);
                      onCustomThemes(library);
                      if (!selectedCustomTheme(library)) onTheme("dark");
                      setConfirmDelete(false);
                      report(`Deleted ${name}.`);
                    }}
                  >
                    Delete theme
                  </button>
                  <button type="button" className="secondary" onClick={() => setConfirmDelete(false)}>
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              <button type="button" className="secondary" onClick={() => setConfirmDelete(true)}>
                Delete this theme…
              </button>
            )}
          </div>
        )}
      </section>
      <FlowAnimationSetting mode={flowAnimation} onChange={onFlowAnimation} />
    </div>
  );
}
