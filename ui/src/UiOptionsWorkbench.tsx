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
  themeColorLabel,
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
import { supportedLocales } from "./i18n";
import { useMessages } from "./i18n/useMessages";

function ThemeContrastReport({
  theme,
  onSelectColor,
}: {
  theme: CustomTheme;
  onSelectColor: (color: ThemeColor) => void;
}) {
  const tr = useMessages();
  const warnings = themeContrastWarnings(theme);
  return (
    <div className={`theme-contrast${warnings.length ? " has-warnings" : ""}`}>
      {warnings.length === 0 ? (
        <p role="status">{tr("g1.themeContrastOk")}</p>
      ) : (
        <>
          <p role="status">{tr("g1.themeContrastBad")}</p>
          <ul>
            {warnings.map((warning) => (
              <li key={`${warning.fore}-${warning.back}`}>
                <button
                  type="button"
                  className="theme-contrast-jump"
                  title={tr("g1.themeGoTo", { color: themeColorLabel(warning.fore) })}
                  onClick={() => onSelectColor(warning.fore)}
                >
                  {themeColorLabel(warning.fore)}
                </button>
                {tr("g1.themeOn")}
                <button
                  type="button"
                  className="theme-contrast-jump"
                  title={tr("g1.themeGoTo", { color: themeColorLabel(warning.back) })}
                  onClick={() => onSelectColor(warning.back)}
                >
                  {themeColorLabel(warning.back).toLowerCase()}
                </button>
                {tr("g1.themeRatio", { ratio: warning.ratio.toFixed(1), minimum: warning.minimum })}
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
  language,
  onLanguage,
  customThemes,
  onCustomThemes,
  flowAnimation,
  onFlowAnimation,
  liveAutosave,
  onLiveAutosave,
}: {
  theme: ThemeMode;
  onTheme: (theme: ThemeMode) => void;
  /** Saved BCP 47 tag, or null for System default. */
  language: string | null;
  onLanguage: (language: string | null) => void;
  customThemes: CustomThemeLibrary;
  onCustomThemes: (library: CustomThemeLibrary) => void;
  flowAnimation: FlowAnimationMode;
  onFlowAnimation: (mode: FlowAnimationMode) => void;
  liveAutosave: boolean;
  onLiveAutosave: (enabled: boolean) => void;
}) {
  const tr = useMessages();
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
    report(tr("g1.themeExported"));
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
      report(cause instanceof Error ? cause.message : tr("g1.themeCouldNotAdd"), true);
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
        <h3 id="theme-options-title">{tr("appearance.title")}</h3>
        <label>
          {tr("appearance.language")}
          <select
            aria-label={tr("appearance.language")}
            value={language ?? "system"}
            onChange={(event) => onLanguage(event.target.value === "system" ? null : event.target.value)}
          >
            <option value="system">{tr("appearance.languageSystem")}</option>
            {supportedLocales().map((locale) => (
              <option key={locale.tag} value={locale.tag}>
                {locale.nativeName}
              </option>
            ))}
          </select>
        </label>
        <p className="muted">{tr("appearance.languageHint")}</p>
        <label>
          {tr("appearance.colorTheme")}
          <select
            aria-label={tr("appearance.colorTheme")}
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
            <option value="dark">{tr("appearance.themeDark")}</option>
            <option value="light">{tr("appearance.themeLight")}</option>
            <option value="high-contrast">{tr("appearance.themeHighContrast")}</option>
            {customThemes.themes.map((entry) => (
              <option key={entry.id} value={CUSTOM_PREFIX + entry.id}>
                {tr("g1.themeCustomSuffix", { name: entry.theme.name })}
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
            if (
              add({
                ...start,
                name: active ? tr("g1.themeCopySuffix", { name: start.name }) : DEFAULT_CUSTOM_THEME.name,
              })
            )
              report(tr("g1.themeCreated"));
          }}
        >
          {tr("g1.themeCreate")}
        </button>
        <label>
          {tr("g1.themeImportFile")}
          <input
            type="file"
            accept=".json,application/json"
            aria-label={tr("g1.themeImportFile")}
            onChange={(event) => {
              const file = event.target.files?.[0];
              event.target.value = "";
              if (!file) return;
              if (file.size > MAX_THEME_BYTES) {
                report(tr("g1.themeTooBig"), true);
                return;
              }
              void file
                .text()
                .then((text) => {
                  const library = add(parseCustomTheme(text));
                  if (library)
                    report(
                      tr("g1.themeImported", {
                        name: selectedCustomTheme(library)?.theme.name ?? tr("g1.themeTheTheme"),
                      }),
                    );
                })
                .catch((cause: unknown) => {
                  report(cause instanceof Error ? cause.message : tr("g1.themeCouldNotRead"), true);
                });
            }}
          />
        </label>
        <button
          type="button"
          className="secondary"
          disabled={!active}
          title={active ? undefined : tr("g1.themeChooseCustom")}
          onClick={() => active && exportTheme(active.theme)}
        >
          {tr("g1.themeExport")}
        </button>
        <p className={`theme-file-message${error ? " is-error" : ""}`} role={error ? "alert" : "status"}>
          {message || tr("g1.themeLocalNote")}
        </p>
        {active && (
          <div className="theme-editor">
            <label>
              {tr("g1.themeName")}
              <TextField
                aria-label={tr("g1.themeName")}
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
              {tr("g1.themeBase")}
              <select
                aria-label={tr("g1.themeBase")}
                value={active.theme.base}
                onChange={(event) => updateActive({ ...active.theme, base: event.target.value as "dark" | "light" })}
              >
                <option value="dark">{tr("g1.themeDark")}</option>
                <option value="light">{tr("g1.themeLight")}</option>
              </select>
            </label>
            <div className="theme-preview-block">
              <p className="theme-preview-heading">{tr("g1.themePreview")}</p>
              <ThemePreview theme={active.theme} />
            </div>
            <details
              onToggle={(event) => {
                if (!event.currentTarget.open) setPickerColor(null);
              }}
            >
              <summary>{tr("g1.themeEditPalette")}</summary>
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
              <div className="theme-delete-confirmation" role="group" aria-label={tr("g1.themeConfirmDelete")}>
                <p>{tr("g1.themeDeletePrompt", { name: active.theme.name })}</p>
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
                      report(tr("g1.themeDeleted", { name }));
                    }}
                  >
                    {tr("g1.themeDeleteButton")}
                  </button>
                  <button type="button" className="secondary" onClick={() => setConfirmDelete(false)}>
                    {tr("g1.themeCancel")}
                  </button>
                </div>
              </div>
            ) : (
              <button type="button" className="secondary" onClick={() => setConfirmDelete(true)}>
                {tr("g1.themeDeleteThis")}
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
