import { useRef, useState } from "react";

export function EditableNodeName({
  name,
  disabled,
  onRename,
}: {
  name: string;
  disabled: boolean;
  onRename: (name: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(name);
  const cancelled = useRef(false);
  const button = useRef<HTMLButtonElement>(null);
  const finish = () => {
    const next = text.trim();
    if (!cancelled.current && next && next !== name) onRename(next);
    setEditing(false);
  };
  return editing ? (
    <input
      aria-label="Node name"
      value={text}
      maxLength={120}
      disabled={disabled}
      autoFocus
      onFocus={(event) => event.currentTarget.select()}
      onChange={(event) => setText(event.target.value)}
      onBlur={finish}
      onKeyDown={(event) => {
        if (event.key !== "Enter" && event.key !== "Escape") return;
        event.preventDefault();
        cancelled.current = event.key === "Escape";
        event.currentTarget.blur();
        window.requestAnimationFrame(() => button.current?.focus());
      }}
    />
  ) : (
    <button
      ref={button}
      type="button"
      className="editable-node-name"
      title="Click to rename this node"
      aria-label={`Rename ${name}`}
      disabled={disabled}
      onClick={() => {
        cancelled.current = false;
        setText(name);
        setEditing(true);
      }}
    >
      <span>{name}</span>
      <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.8">
        <path d="m15 5 4 4M4 20l4-1L20 7a2.8 2.8 0 0 0-4-4L4 15z" />
      </svg>
    </button>
  );
}
