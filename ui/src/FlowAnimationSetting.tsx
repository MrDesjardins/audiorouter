import type { FlowAnimationMode } from "./preferences";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

const CHOICES: ReadonlyArray<{ value: FlowAnimationMode; label: MessageKey; help: MessageKey }> = [
  { value: "on", label: "settings.flowAnimation.on", help: "settings.flowAnimation.onHelp" },
  {
    value: "focus",
    label: "settings.flowAnimation.focus",
    help: "settings.flowAnimation.focusHelp",
  },
  {
    value: "off",
    label: "settings.flowAnimation.off",
    help: "settings.flowAnimation.offHelp",
  },
];

/** UI Options → Animated connections: the travelling lights cost most of the
 * canvas's CPU while audio plays, so they can pause or stay off. */
export function FlowAnimationSetting({
  mode,
  onChange,
}: {
  mode: FlowAnimationMode;
  onChange: (mode: FlowAnimationMode) => void;
}) {
  const tr = useMessages();
  const selected = CHOICES.find((choice) => choice.value === mode) ?? CHOICES[0];
  return (
    <section className="panel flow-animation-panel" aria-labelledby="flow-animation-heading">
      <h3 id="flow-animation-heading">{tr("settings.flowAnimation.heading")}</h3>
      <label>
        {tr("settings.flowAnimation.label")}
        <select value={selected.value} onChange={(event) => onChange(event.target.value as FlowAnimationMode)}>
          {CHOICES.map((choice) => (
            <option key={choice.value} value={choice.value}>
              {tr(choice.label)}
            </option>
          ))}
        </select>
      </label>
      <small className="muted flow-animation-help">
        {tr(selected.help)} {tr("settings.flowAnimation.closing")}
      </small>
    </section>
  );
}
