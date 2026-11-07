import { useId, type ReactNode } from "react";

/** A named switch with a stable text state, shared by tool properties. */
export function BooleanField({
  label,
  name = label,
  checked,
  disabled,
  help,
  stateText,
  onChange,
}: {
  label: string;
  name?: string;
  checked: boolean;
  disabled?: boolean;
  help?: ReactNode;
  /** Reserve a wider slot when a backend setting can be unavailable. */
  stateText?: "On" | "Off" | "Unavailable";
  onChange: (checked: boolean) => void;
}) {
  const stateId = useId();
  return (
    <label className="boolean-field">
      <span>{label}</span>
      {help}
      <span className="boolean-field-control">
        <input
          type="checkbox"
          aria-label={name}
          aria-describedby={stateId}
          checked={checked}
          disabled={disabled}
          onChange={(event) => onChange(event.target.checked)}
        />
        <span className={`boolean-field-state${stateText ? " has-explicit-state" : ""}`} id={stateId}>
          {stateText ?? (checked ? "On" : "Off")}
        </span>
      </span>
    </label>
  );
}
