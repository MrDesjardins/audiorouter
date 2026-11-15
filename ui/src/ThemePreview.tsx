import { useId } from "react";
import { customThemeStyle, type CustomTheme } from "./customTheme";

/**
 * A small stand-in for the workspace, drawn with the theme being edited. It
 * shows the canvas, a running route (two flowing connections), a silent one
 * and a paused one, plus tool-panel text, a field, a slider and a primary
 * action, so every palette colour has something real to colour.
 */
export function ThemePreview({ theme }: { theme: CustomTheme }) {
  const gradientId = "theme-preview-flow-" + useId().replace(/[^a-zA-Z0-9_-]/g, "");
  const firstPath = gradientId + "-a";
  const secondPath = gradientId + "-b";
  return (
    <div
      className={`theme-preview theme-${theme.base} theme-custom`}
      style={customThemeStyle(theme)}
      role="img"
      aria-label="Theme preview: a workspace with a running route, two flowing connections, a silent connection and a paused connection, plus a tool panel."
    >
      <div className="theme-preview-topbar">
        <span className="theme-preview-brand">AudioRouter</span>
        <span className="theme-preview-status">
          <span className="theme-preview-status-dot" />
          Running
        </span>
      </div>
      <div className="theme-preview-body">
        <div className="theme-preview-rail">
          <span className="is-current">Route</span>
          <span>Tools</span>
          <span>Settings</span>
        </div>
        <div className="theme-preview-stage">
          <div className="theme-preview-canvas">
            <svg className="theme-preview-lines" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
              <defs>
                <linearGradient id={gradientId} x1="0" x2="1">
                  <stop offset="0%" style={{ stopColor: "var(--preview-flow-cold)" }} />
                  <stop offset="55%" style={{ stopColor: "var(--preview-flow-warm)" }} />
                  <stop offset="100%" style={{ stopColor: "var(--preview-flow-hot)" }} />
                </linearGradient>
              </defs>
              <path
                id={firstPath}
                className="theme-preview-line is-active"
                stroke={`url(#${gradientId})`}
                d="M27 30 C 31 30, 35 34, 39 34"
              />
              <path
                id={secondPath}
                className="theme-preview-line is-active"
                stroke={`url(#${gradientId})`}
                d="M61 34 C 67 34, 66 54, 73 70"
              />
              <path className="theme-preview-line is-silent" d="M61 34 C 65 34, 69 34, 73 34" />
              <path className="theme-preview-line is-paused" d="M27 70 C 45 70, 60 70, 73 70" />
              <circle className="theme-preview-comet" r="1.5">
                <animateMotion dur="2.6s" repeatCount="indefinite">
                  <mpath href={`#${firstPath}`} />
                </animateMotion>
              </circle>
              <circle className="theme-preview-comet" r="1.5">
                <animateMotion dur="3.1s" repeatCount="indefinite">
                  <mpath href={`#${secondPath}`} />
                </animateMotion>
              </circle>
            </svg>
            <div className="theme-preview-node theme-preview-mic">
              <span className="theme-preview-kind">Input</span>
              <strong>Microphone</strong>
              <span className="theme-preview-port is-output" />
            </div>
            <div className="theme-preview-node theme-preview-gain">
              <span className="theme-preview-kind">Tool</span>
              <strong>Gain</strong>
              <span className="theme-preview-port is-input" />
              <span className="theme-preview-port is-output" />
            </div>
            <div className="theme-preview-node theme-preview-recorder">
              <span className="theme-preview-kind">Tool</span>
              <strong>Recorder</strong>
              <span className="theme-preview-port is-input" />
            </div>
            <div className="theme-preview-node theme-preview-plugin is-plugin">
              <span className="theme-preview-kind">Plug-in</span>
              <strong>Voice FX</strong>
              <span className="theme-preview-port is-output" />
            </div>
            <div className="theme-preview-node theme-preview-speakers is-selected">
              <span className="theme-preview-kind">Output</span>
              <strong>Speakers</strong>
              <span className="theme-preview-port is-input" />
            </div>
          </div>
          <ul className="theme-preview-legend">
            <li>
              <i className="is-active" />
              Flowing
            </li>
            <li>
              <i className="is-silent" />
              Silent
            </li>
            <li>
              <i className="is-paused" />
              Paused
            </li>
          </ul>
        </div>
        <div className="theme-preview-inspector">
          <span className="theme-preview-panel-title">Gain</span>
          <span className="theme-preview-kind">Tool</span>
          <span className="theme-preview-field">&minus;6.0 dB</span>
          <span className="theme-preview-slider" />
          <span className="theme-preview-chips">
            <span className="is-good">Stable</span>
            <span className="is-plugin">VST3</span>
            <span className="is-danger">Clip</span>
          </span>
          <span className="theme-preview-button">Apply</span>
        </div>
      </div>
      <p className="theme-preview-caption">Colors update here as you choose them.</p>
    </div>
  );
}
