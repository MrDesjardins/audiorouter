import { useEffect, useRef, useState } from "react";
import { FlowAnimationSetting } from "./FlowAnimationSetting";
import { LiveAutosaveSetting } from "./LiveAutosaveSetting";
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
import { ThemeColorPicker } from "./ThemeColorPicker";
import { ThemePreview } from "./ThemePreview";
import { themeContrastWarnings } from "./themeContrast";

function ThemeContrastReport({
  theme,
  onSelectColor,
}: {
  theme: CustomTheme;
  onSelectColor: (color: ThemeColor) => void;
}) {
  const warnings = themeContrastWarnings(theme);
  return (
    <div className={`theme-contrast${warnings.length ? " has-warnings" : ""}`}>
      {warnings.length === 0 ? (
        <p role="status">Text in this theme is easy to read: every checked pair meets the WCAG contrast guideline.</p>
      ) : (
        <>
          <p role="status">Some text may be hard to read. Select a color name to change it:</p>
          <ul>
            {warnings.map((warning) => (
              <li key={`${warning.fore}-${warning.back}`}>
                <button
                  type="button"
                  className="theme-contrast-jump"
                  title={`Go to ${THEME_COLORS[warning.fore]} color`}
                  onClick={() => onSelectColor(warning.fore)}
                >
                  {THEME_COLORS[warning.fore]}
                </button>
                {" on "}
                <button
                  type="button"
                  className="theme-contrast-jump"
                  title={`Go to ${THEME_COLORS[warning.back]} color`}
                  onClick={() => onSelectColor(warning.back)}
                >
                  {THEME_COLORS[warning.back].toLowerCase()}
                </button>
                {`: ${warning.ratio.toFixed(1)}:1, needs ${warning.minimum}:1`}
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

const CUSTOM_PREFIX = "custom:";

export function UiOptionsWorkbench({
  theme,
  onTheme,
  customThemes,
  onCustomThemes,
  flowAnimation,
  onFlowAnimation,
  liveAutosave,
  onLiveAutosave,
}: {
  theme: ThemeMode;
  onTheme: (theme: ThemeMode) => void;
  customThemes: CustomThemeLibrary;
  onCustomThemes: (library: CustomThemeLibrary) => void;
  flowAnimation: FlowAnimationMode;
  onFlowAnimation: (mode: FlowAnimationMode) => void;
  liveAutosave: boolean;
  onLiveAutosave: (enabled: boolean) => void;
}) {
  const [message, setMessage] = useState("");
  const [error, setError] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [highlightColor, setHighlightColor] = useState<ThemeColor | null>(null);
  const [pickerColor, setPickerColor] = useState<ThemeColor | null>(null);
  const highlightTimer = useRef<number | null>(null);
  const selected = selectedCustomTheme(customThemes);
  const active = theme === "custom" ? selected : null;
  useEffect(
    () => () => {
      if (highlightTimer.current !== null) window.clearTimeout(highlightTimer.current);
    },
    [],
  );
  useEffect(() => {
    setPickerColor(null);
  }, [active?.id]);
  useEffect(() => {
    if (!pickerColor) return;
    const frame = window.requestAnimationFrame(() => {
      const item = document.getElementById(`theme-color-${pickerColor}`)?.closest(".theme-color-item");
      item?.querySelector<HTMLElement>(".theme-color-popover")?.scrollIntoView({ block: "nearest" });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [pickerColor]);
  /** Bring a contrast warning's colour picker into view, open it and focus its field. */
  const selectColor = (color: ThemeColor) => {
    setHighlightColor(color);
    if (highlightTimer.current !== null) window.clearTimeout(highlightTimer.current);
    highlightTimer.current = window.setTimeout(() => {
      highlightTimer.current = null;
      setHighlightColor(null);
    }, 1600);
    setPickerColor(color);
    const input = document.getElementById(`theme-color-${color}`);
    if (input instanceof HTMLInputElement) input.focus({ preventScroll: true });
  };
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
            <div className="theme-preview-block">
              <p className="theme-preview-heading">Preview</p>
              <ThemePreview theme={active.theme} />
            </div>
            <details
              onToggle={(event) => {
                if (!event.currentTarget.open) setPickerColor(null);
              }}
            >
              <summary>Edit palette colors</summary>
              <div className="theme-palette">
                {(Object.keys(THEME_COLORS) as ThemeColor[]).map((key) => (
                  <ThemeColorPicker
                    key={key}
                    colorKey={key}
                    theme={active.theme}
                    value={active.theme.colors[key]}
                    open={pickerColor === key}
                    onOpenChange={(open) => setPickerColor(open ? key : null)}
                    onChange={(hex) =>
                      updateActive({ ...active.theme, colors: { ...active.theme.colors, [key]: hex } })
                    }
                    highlighted={highlightColor === key}
                  />
                ))}
              </div>
              <ThemeContrastReport theme={active.theme} onSelectColor={selectColor} />
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
      <LiveAutosaveSetting enabled={liveAutosave} onChange={onLiveAutosave} />
    </div>
  );
}
