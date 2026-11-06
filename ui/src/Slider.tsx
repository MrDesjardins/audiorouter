// The app's one slider. `SliderInput` is the only place that renders a range input
// (Slider.test.tsx enforces it), so every slider shares one look, one fill rule and one keyboard
// behavior. `SliderField` adds the standard layout: caption, track, exact value, unit and reset.
import {
  useId,
  useRef,
  type CSSProperties,
  type InputHTMLAttributes,
  type KeyboardEvent,
  type ReactNode,
  type Ref,
} from "react";
import { NumberField } from "./NumberField";

const ratio = (value: number, min: number, max: number) =>
  max > min ? Math.min(1, Math.max(0, (value - min) / (max - min))) : 0;

/** Where a fill starts: at zero on a range that crosses it, otherwise at the minimum. */
export function sliderFillOrigin(min: number, max: number): number {
  return min < 0 && max > 0 ? 0 : min;
}

/** The filled part of a track, as two fractions of its length (0 to 1, low end first). */
export function sliderFill(value: number, min: number, max: number, origin = sliderFillOrigin(min, max)) {
  const at = ratio(value, min, max);
  const from = ratio(origin, min, max);
  return { from: Math.min(at, from), to: Math.max(at, from) };
}

const decimalsOf = (step: number) => (String(step).split(".")[1] ?? "").length;

type SliderInputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "value" | "min" | "max" | "step"> & {
  value: number;
  min: number;
  max: number;
  step: number;
  /** Where the fill starts; zero on a range that crosses it, otherwise the minimum. */
  origin?: number;
  ref?: Ref<HTMLInputElement>;
  onValue: (value: number) => void;
};

/** A themed range input. Arrow keys move one step, Shift+Arrow ten; Home and End go to the limits. */
export function SliderInput({
  value,
  min,
  max,
  step,
  origin,
  onValue,
  className,
  style,
  onKeyDown,
  ...rest
}: SliderInputProps) {
  const fill = sliderFill(value, min, max, origin);
  return (
    <input
      {...rest}
      type="range"
      className={`app-slider${className ? ` ${className}` : ""}`}
      style={
        {
          ...style,
          "--slider-from": fill.from.toFixed(4),
          "--slider-to": fill.to.toFixed(4),
          // A fill that starts or ends at a limit runs to the end of the rail. The thumb's center stops
          // half a thumb short of each end, which would leave a stub of unfilled rail there.
          ...(fill.from === 0 ? { "--slider-low": "0%" } : {}),
          ...(fill.to === 1 ? { "--slider-high": "100%" } : {}),
        } as CSSProperties
      }
      value={value}
      min={min}
      max={max}
      step={step}
      onChange={(event) => onValue(Number(event.target.value))}
      onKeyDown={(event) => {
        onKeyDown?.(event);
        if (event.defaultPrevented || !event.shiftKey) return;
        const direction =
          event.key === "ArrowRight" || event.key === "ArrowUp"
            ? 1
            : event.key === "ArrowLeft" || event.key === "ArrowDown"
              ? -1
              : 0;
        if (direction === 0) return;
        event.preventDefault();
        const next = Math.min(max, Math.max(min, value + direction * step * 10));
        onValue(Number(next.toFixed(decimalsOf(step))));
      }}
    />
  );
}

/** The short unit written next to the exact field, or nothing for a unitless value. */
export function sliderUnit(unit: string | undefined): string {
  if (unit === "semitones") return "st";
  if (unit === "cents") return "ct";
  if (unit === "normalized") return "";
  return unit ?? "";
}

/** A range limit as a person writes it: "−24 dB", "+6 dB", "20 kHz", "100 %". */
export function sliderBoundLabel(value: number, unit: string | undefined, signed: boolean): string {
  const short = sliderUnit(unit);
  const size = Math.abs(value);
  const [divisor, written] =
    short === "Hz" && size >= 1000
      ? [1000, "kHz"]
      : short === "ms" && size >= 60_000
        ? [60_000, "min"]
        : short === "ms" && size >= 1000
          ? [1000, "s"]
          : [1, short];
  const shown = Number((value / divisor).toFixed(3));
  const sign = shown < 0 ? "−" : signed && shown > 0 ? "+" : "";
  const label = `${sign}${Math.abs(shown)}`;
  return written ? `${label} ${written}` : label;
}

/** Track positions of a logarithmic slider, which spends equal travel on each octave. */
const LOG_POSITIONS = 1000;

/**
 * A wide frequency or time range such as 20 Hz to 20 kHz, where a linear track would crowd every
 * useful value. Levels and percentages stay linear: people read those as even steps.
 */
export function usesLogTrack(min: number, max: number, unit: string | undefined): boolean {
  return (unit === "Hz" || unit === "ms") && min > 0 && max / min >= 100;
}

export function logTrackPosition(value: number, min: number, max: number): number {
  const clamped = Math.min(max, Math.max(min, value));
  return Math.round((Math.log(clamped / min) / Math.log(max / min)) * LOG_POSITIONS);
}

/** The value at a track position, rounded to three figures and to the step, as a person would pick. */
export function logTrackValue(position: number, min: number, max: number, step: number): number {
  if (position <= 0) return min;
  if (position >= LOG_POSITIONS) return max;
  const raw = min * (max / min) ** (position / LOG_POSITIONS);
  const figures = Number(raw.toPrecision(3));
  const snapped = Math.round(figures / step) * step;
  return Math.min(max, Math.max(min, Number(snapped.toFixed(decimalsOf(step)))));
}

function ResetIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M9 4 4 9l5 5M4 9h10a6 6 0 0 1 0 12" />
    </svg>
  );
}

export type SliderFieldProps = {
  /** The visible caption. */
  label: ReactNode;
  /** Names the control for assistive technology; the exact field adds " precise value". */
  name: string;
  /** Accessible name of the slider itself, when it differs from `name`. */
  sliderName?: string;
  value: number;
  min: number;
  max: number;
  step: number;
  /** Written next to the exact field and on the range labels. */
  unit?: string;
  /** Enables the reset button, double-click on the track and the tick on the track. */
  defaultValue?: number;
  disabled?: boolean;
  /** What a screen reader says for the value; defaults to the value and unit. */
  valueText?: string;
  /** Help shown between the caption and the track. */
  help?: ReactNode;
  /** Set false in dense lists where the panel already states the range once. */
  showScale?: boolean;
  /** Keys step by this much in the exact field when it should differ from the slider. */
  exactStep?: number;
  className?: string;
  /** Plain explanation for the tooltip and screen readers, when `help` is not shown. */
  description?: string;
  onChange: (value: number) => void;
};

/**
 * Caption, then one row with the track, the exact value and its unit, and a reset button that
 * appears once the value differs from the default. Its slot is always reserved, so changing a
 * value never moves anything (UI-17).
 */
export function SliderField({
  label,
  name,
  sliderName,
  value,
  min,
  max,
  step,
  unit,
  defaultValue,
  disabled = false,
  valueText,
  help,
  showScale = true,
  exactStep,
  className,
  description,
  onChange,
}: SliderFieldProps) {
  const id = useId();
  const track = useRef<HTMLInputElement>(null);
  const short = sliderUnit(unit);
  const spoken = valueText ?? (short ? `${value} ${short}` : String(value));
  const hasDefault = typeof defaultValue === "number" && Number.isFinite(defaultValue);
  const inRange = hasDefault && defaultValue >= min && defaultValue <= max;
  const signed = min < 0 && max > 0;
  const defaultLabel = hasDefault ? sliderBoundLabel(defaultValue, unit, signed) : "";
  const changed = inRange && value !== defaultValue;
  const log = usesLogTrack(min, max, unit);
  const position = (at: number) => (log ? logTrackPosition(at, min, max) / LOG_POSITIONS : ratio(at, min, max));
  const defaultAt = inRange ? position(defaultValue) : null;
  const showTick = defaultAt !== null && defaultAt > 0 && defaultAt < 1;
  // A middle label only fits when it stays clear of the two end labels.
  const showMiddle = showTick && defaultAt > 0.3 && defaultAt < 0.7;
  return (
    <div className={`slider-field${className ? ` ${className}` : ""}`} title={description}>
      <label className="slider-field-caption" htmlFor={id}>
        {label}
      </label>
      {help}
      <div className="slider-field-row" style={{ "--slider-tick": defaultAt ?? 0 } as CSSProperties}>
        <div className="slider-field-track">
          {showTick && <span className="slider-tick" aria-hidden="true" />}
          <SliderInput
            ref={track}
            id={id}
            aria-label={sliderName ?? name}
            aria-valuetext={spoken}
            aria-description={description}
            title={inRange ? `Double-click to restore ${defaultLabel}` : undefined}
            {...(log
              ? {
                  value: logTrackPosition(value, min, max),
                  min: 0,
                  max: LOG_POSITIONS,
                  step: 1,
                  // The input holds a track position; assistive technology gets the real value.
                  "aria-valuenow": value,
                  "aria-valuemin": min,
                  "aria-valuemax": max,
                  onValue: (next: number) => onChange(logTrackValue(next, min, max, step)),
                  onKeyDown: (event: KeyboardEvent<HTMLInputElement>) => {
                    const direction =
                      event.key === "ArrowRight" || event.key === "ArrowUp"
                        ? 1
                        : event.key === "ArrowLeft" || event.key === "ArrowDown"
                          ? -1
                          : 0;
                    if (direction === 0) return;
                    event.preventDefault();
                    const moved = logTrackPosition(value, min, max) + direction * (event.shiftKey ? 50 : 5);
                    const next = logTrackValue(moved, min, max, step);
                    // At the crowded low end a small move can round back to the same value.
                    onChange(
                      next === value
                        ? Math.min(max, Math.max(min, Number((value + direction * step).toFixed(decimalsOf(step)))))
                        : next,
                    );
                  },
                }
              : { value: Math.min(max, Math.max(min, value)), min, max, step, onValue: onChange })}
            disabled={disabled}
            onDoubleClick={() => {
              if (inRange && !disabled) onChange(defaultValue);
            }}
          />
        </div>
        <span className="slider-field-exact">
          <NumberField
            aria-label={`${name} precise value`}
            aria-valuetext={spoken}
            value={value}
            min={min}
            max={max}
            step={exactStep ?? step}
            disabled={disabled}
            onValue={onChange}
          />
          <span className="slider-field-unit" aria-hidden="true">
            {short}
          </span>
        </span>
        <span className="slider-field-reset">
          {changed && (
            <button
              type="button"
              className="secondary"
              aria-label={`Reset ${name} to ${defaultLabel}`}
              title={`Reset to ${defaultLabel}`}
              disabled={disabled}
              onClick={() => {
                onChange(defaultValue);
                // The button leaves with the changed value; keep keyboard focus nearby.
                track.current?.focus();
              }}
            >
              <ResetIcon />
            </button>
          )}
        </span>
        {showScale && (
          <small className="slider-field-scale" aria-hidden="true">
            <span>{sliderBoundLabel(min, unit, signed)}</span>
            {showMiddle && <span className="slider-field-scale-middle">{defaultLabel}</span>}
            <span>{sliderBoundLabel(max, unit, signed)}</span>
          </small>
        )}
      </div>
    </div>
  );
}
