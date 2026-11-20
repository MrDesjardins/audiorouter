import { useId } from "react";
import { useMessages } from "./i18n/useMessages";
import { customThemeStyle, type CustomTheme } from "./customTheme";

/**
 * A small stand-in for the workspace, drawn with the theme being edited. It
 * shows the canvas, a running route (two flowing connections), a silent one
 * and a paused one, plus tool-panel text, a field, a slider and a primary
 * action, so every palette colour has something real to colour.
 */
export function ThemePreview({ theme }: { theme: CustomTheme }) {
  const tr = useMessages();
  const gradientId = "theme-preview-flow-" + useId().replace(/[^a-zA-Z0-9_-]/g, "");
  const firstPath = gradientId + "-a";
  const secondPath = gradientId + "-b";
  return (
    <div
      className={`theme-preview theme-${theme.base} theme-custom`}
      style={customThemeStyle(theme)}
      role="img"
      aria-label={tr("g1.previewAria")}
    >
      <div className="theme-preview-topbar">
        <span className="theme-preview-brand">AudioRouter</span>
        <span className="theme-preview-status">
          <span className="theme-preview-status-dot" />
          {tr("g1.previewRunning")}
        </span>
      </div>
      <div className="theme-preview-body">
        <div className="theme-preview-rail">
          <span className="is-current">{tr("g1.previewRoute")}</span>
          <span>{tr("g1.previewTools")}</span>
          <span>{tr("g1.previewSettings")}</span>
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
              <span className="theme-preview-kind">{tr("g1.previewInput")}</span>
              <strong>{tr("g1.previewMicrophone")}</strong>
              <span className="theme-preview-port is-output" />
            </div>
            <div className="theme-preview-node theme-preview-gain">
              <span className="theme-preview-kind">{tr("g1.previewTool")}</span>
              <strong>{tr("g1.previewGain")}</strong>
              <span className="theme-preview-port is-input" />
              <span className="theme-preview-port is-output" />
            </div>
            <div className="theme-preview-node theme-preview-recorder">
              <span className="theme-preview-kind">{tr("g1.previewTool")}</span>
              <strong>{tr("g1.previewRecorder")}</strong>
              <span className="theme-preview-port is-input" />
            </div>
            <div className="theme-preview-node theme-preview-plugin is-plugin">
              <span className="theme-preview-kind">{tr("g1.previewPlugin")}</span>
              <strong>{tr("g1.previewVoiceFx")}</strong>
              <span className="theme-preview-port is-output" />
            </div>
            <div className="theme-preview-node theme-preview-speakers is-selected">
              <span className="theme-preview-kind">{tr("g1.previewOutput")}</span>
              <strong>{tr("g1.previewSpeakers")}</strong>
              <span className="theme-preview-port is-input" />
            </div>
          </div>
          <ul className="theme-preview-legend">
            <li>
              <i className="is-active" />
              {tr("g1.previewFlowing")}
            </li>
            <li>
              <i className="is-silent" />
              {tr("g1.previewSilent")}
            </li>
            <li>
              <i className="is-paused" />
              {tr("g1.previewPaused")}
            </li>
          </ul>
        </div>
        <div className="theme-preview-inspector">
          <span className="theme-preview-panel-title">{tr("g1.previewGain")}</span>
          <span className="theme-preview-kind">{tr("g1.previewTool")}</span>
          <span className="theme-preview-field">&minus;6.0 dB</span>
          <span className="theme-preview-slider" />
          <span className="theme-preview-chips">
            <span className="is-good">{tr("g1.previewStable")}</span>
            <span className="is-plugin">VST3</span>
            <span className="is-danger">{tr("g1.previewClip")}</span>
          </span>
          <span className="theme-preview-button">{tr("g1.previewApply")}</span>
        </div>
      </div>
      <p className="theme-preview-caption">{tr("g1.previewCaption")}</p>
    </div>
  );
}
