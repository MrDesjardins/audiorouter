import { useState } from "react";
import { FlowAnimationSetting } from "./FlowAnimationSetting";
import { type FlowAnimationMode, type ThemeMode } from "./preferences";
import { MAX_THEME_BYTES, THEME_COLORS, parseCustomTheme, type CustomTheme, type ThemeColor } from "./customTheme";
import { TextField } from "./TextField";

export function UiOptionsWorkbench({
  theme,
  onTheme,
  customTheme,
  onCustomTheme,
  flowAnimation,
  onFlowAnimation,
}: {
  theme: ThemeMode;
  onTheme: (theme: ThemeMode) => void;
  customTheme: CustomTheme;
  onCustomTheme: (theme: CustomTheme) => void;
  flowAnimation: FlowAnimationMode;
  onFlowAnimation: (mode: FlowAnimationMode) => void;
}) {
  const [message, setMessage] = useState("");
  const [error, setError] = useState(false);
  const exportTheme = () => {
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
    setError(false);
    setMessage("Theme exported. The file contains colors and appearance only.");
  };
  const update = (next: CustomTheme) => {
    onCustomTheme(next);
    onTheme("custom");
  };
  return (
    <div className="ui-options">
      <section className="panel theme-options" aria-labelledby="theme-options-title">
        <h3 id="theme-options-title">Appearance</h3>
        <label>
          Color theme
          <select aria-label="Color theme" value={theme} onChange={(event) => onTheme(event.target.value as ThemeMode)}>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
            <option value="high-contrast">High contrast</option>
            <option value="custom">{customTheme.name} (custom)</option>
          </select>
        </label>
        <button
          type="button"
          className="secondary"
          onClick={() => {
            update(customTheme);
            setError(false);
            setMessage("Your theme is ready to edit. Colors save automatically on this PC.");
          }}
        >
          Create your own theme
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
                setError(true);
                setMessage("Theme files must be 32 KB or smaller.");
                return;
              }
              void file
                .text()
                .then((text) => {
                  const next = parseCustomTheme(text);
                  update(next);
                  setError(false);
                  setMessage(`Imported ${next.name}.`);
                })
                .catch((cause: unknown) => {
                  setError(true);
                  setMessage(cause instanceof Error ? cause.message : "Could not read the theme file.");
                });
            }}
          />
        </label>
        <button type="button" className="secondary" onClick={exportTheme}>
          Export custom theme
        </button>
        <p className={`theme-file-message${error ? " is-error" : ""}`} role={error ? "alert" : "status"}>
          {message || "Themes are local appearance settings. Importing a file never changes your audio or session."}
        </p>
        {theme === "custom" && (
          <div className="theme-editor">
            <label>
              Theme name
              <TextField
                aria-label="Theme name"
                value={customTheme.name}
                maxLength={80}
                onValue={(name) => update({ ...customTheme, name })}
              />
            </label>
            <label>
              Base appearance
              <select
                aria-label="Base appearance"
                value={customTheme.base}
                onChange={(event) => update({ ...customTheme, base: event.target.value as "dark" | "light" })}
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
                      value={customTheme.colors[key]}
                      onChange={(event) =>
                        update({ ...customTheme, colors: { ...customTheme.colors, [key]: event.target.value } })
                      }
                    />
                    <output>{customTheme.colors[key]}</output>
                  </label>
                ))}
              </div>
            </details>
          </div>
        )}
      </section>
      <FlowAnimationSetting mode={flowAnimation} onChange={onFlowAnimation} />
    </div>
  );
}
