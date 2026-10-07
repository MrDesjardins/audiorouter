import { useEffect, useState, type InputHTMLAttributes } from "react";

type TextFieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange" | "type"> & {
  value: string;
  /** Receives the trimmed text whenever it is non-empty and changed. */
  onValue: (value: string) => void;
};

/**
 * A name field that keeps exactly what the user types. Names are stored
 * trimmed, so feeding each keystroke's trimmed value back into the field
 * would drop a space the moment it is typed ("My " → "My"). This field keeps
 * its own text, commits the trimmed value when it is not empty, follows a
 * stored name changed elsewhere (for example Revert edits), and shows the
 * stored name again on blur (for example after clearing the field).
 */
export function TextField({ value, onValue, onBlur, ...rest }: TextFieldProps) {
  const [text, setText] = useState(value);
  useEffect(() => {
    setText((current) => (current.trim() === value ? current : value));
  }, [value]);
  return (
    <input
      {...rest}
      type="text"
      value={text}
      onBlur={(event) => {
        setText(value);
        onBlur?.(event);
      }}
      onChange={(event) => {
        setText(event.target.value);
        const trimmed = event.target.value.trim();
        if (trimmed && trimmed !== value) onValue(trimmed);
      }}
    />
  );
}
