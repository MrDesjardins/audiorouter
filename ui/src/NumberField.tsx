import { useEffect, useId, useRef, useState, type InputHTMLAttributes } from "react";

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
  const inRange = (number: number) =>
    Number.isFinite(number) && (min === undefined || number >= min) && (max === undefined || number <= max);
  const parsed = text.trim() === "" ? Number.NaN : Number(text);
  const invalid = !inRange(parsed) && text.trim() !== "";
  const hintId = useId();
  const hint = rangeHint(min, max);
  return (
    <>
      <input
        {...rest}
        title={invalid ? hint : rest.title}
        aria-describedby={
          invalid ? [rest["aria-describedby"], hintId].filter(Boolean).join(" ") : rest["aria-describedby"]
        }
        type="text"
        inputMode={min !== undefined && min >= 0 ? "decimal" : "text"}
        autoComplete="off"
        spellCheck={false}
        role="spinbutton"
        aria-valuenow={Number.isFinite(value) ? value : undefined}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-invalid={invalid ? true : undefined}
        value={text}
        onFocus={(event) => {
          editing.current = true;
          onFocus?.(event);
        }}
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
      />
      {/* Out of flow (sr-only is absolute), so it never shifts a grid or flex row. */}
      <span id={hintId} className="sr-only">
        {invalid ? hint : ""}
      </span>
    </>
  );
}

/** Plain words for the allowed range, shown while typed text is outside it. */
export function rangeHint(min?: number, max?: number): string {
  if (min !== undefined && max !== undefined) return `Enter a number from ${min} to ${max}.`;
  if (min !== undefined) return `Enter a number of at least ${min}.`;
  if (max !== undefined) return `Enter a number no greater than ${max}.`;
  return "Enter a number.";
}
