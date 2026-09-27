import { useEffect, useRef, useState, type InputHTMLAttributes } from "react";

type NumberFieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange" | "type" | "min" | "max"> & {
  value: number;
  min?: number;
  max?: number;
  /** Called with each valid, in-range value while typing. */
  onValue: (value: number) => void;
};

/** Accepts what a person types on the way to a number: "", "-", "1.", "-0." */
function isPartialNumber(text: string): boolean {
  return /^-?\d*\.?\d*$/.test(text.trim());
}

/**
 * A numeric text field that keeps the typed text while it is being edited.
 * A controlled `<input type="number">` bound to a saved value snaps back on
 * every intermediate state that is not yet valid (deleting digits through an
 * out-of-range value, or starting with "-"), so values could only be appended
 * to. This field applies each valid, in-range value immediately and restores
 * the saved value only when focus leaves with invalid text.
 */
export function NumberField({ value, min, max, onValue, onBlur, onFocus, ...rest }: NumberFieldProps) {
  const [text, setText] = useState(() => String(value));
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) setText(String(value));
  }, [value]);
  const inRange = (number: number) => Number.isFinite(number) && (min === undefined || number >= min) && (max === undefined || number <= max);
  const parsed = text.trim() === "" ? Number.NaN : Number(text);
  return <input
    {...rest}
    type="text"
    inputMode={min !== undefined && min >= 0 ? "decimal" : "text"}
    autoComplete="off"
    spellCheck={false}
    role="spinbutton"
    aria-valuenow={Number.isFinite(value) ? value : undefined}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-invalid={!inRange(parsed) && text.trim() !== "" ? true : undefined}
    value={text}
    onFocus={(event) => { editing.current = true; onFocus?.(event); }}
    onChange={(event) => {
      const next = event.target.value;
      if (!isPartialNumber(next)) return;
      setText(next);
      const number = Number(next);
      if (next.trim() !== "" && next.trim() !== "-" && inRange(number)) onValue(number);
    }}
    onKeyDown={(event) => {
      if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
      event.preventDefault();
      const step = Number(rest.step) > 0 ? Number(rest.step) : 1;
      const base = inRange(parsed) ? parsed : value;
      const decimals = (String(step).split(".")[1] ?? "").length;
      let next = Number((base + (event.key === "ArrowUp" ? step : -step)).toFixed(decimals));
      if (min !== undefined) next = Math.max(min, next);
      if (max !== undefined) next = Math.min(max, next);
      setText(String(next));
      onValue(next);
    }}
    onBlur={(event) => {
      editing.current = false;
      if (!inRange(parsed)) setText(String(value));
      onBlur?.(event);
    }}
  />;
}
