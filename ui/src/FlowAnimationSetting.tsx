import type { FlowAnimationMode } from "./preferences";

const CHOICES: ReadonlyArray<{ value: FlowAnimationMode; label: string; help: string }> = [
  { value: "on", label: "On", help: "Lights travel along every connection that carries sound." },
  {
    value: "focus",
    label: "When AudioRouter is in focus",
    help: "The lights pause while you use another program (for example a game on another screen) and resume when you click back in. Colours, glow and meters keep updating.",
  },
  {
    value: "off",
    label: "Off",
    help: "No travelling lights. Colours, glow and meters still show the level. Uses the least CPU while this window is open.",
  },
];

/** Setup → Animated connections: the travelling lights cost most of the
 * canvas's CPU while audio plays, so they can pause or stay off. */
export function FlowAnimationSetting({
  mode,
  onChange,
}: {
  mode: FlowAnimationMode;
  onChange: (mode: FlowAnimationMode) => void;
}) {
  const selected = CHOICES.find((choice) => choice.value === mode) ?? CHOICES[0];
  return (
    <section className="panel flow-animation-panel" aria-labelledby="flow-animation-heading">
      <h3 id="flow-animation-heading">Animated connections</h3>
      <label>
        Travelling lights on connections
        <select value={selected.value} onChange={(event) => onChange(event.target.value as FlowAnimationMode)}>
          {CHOICES.map((choice) => (
            <option key={choice.value} value={choice.value}>
              {choice.label}
            </option>
          ))}
        </select>
      </label>
      <small className="muted flow-animation-help">
        {selected.help} Closing the window to the tray stops all drawing.
      </small>
    </section>
  );
}
