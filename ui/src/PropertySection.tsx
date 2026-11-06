import { useState, type ReactNode } from "react";
import { Icon } from "./icons";
import { optionalLocalStorage, readPresentationJson, writePresentationJson } from "./presentationStorage";

const STORAGE_KEY = "audiorouter.ui.property-sections";
const MAX_REMEMBERED = 512;

/** "compressor/Timing and knee" -> open or closed, for this PC only. Anything else reads as not chosen. */
function readRemembered(): Record<string, boolean> {
  const stored = readPresentationJson(optionalLocalStorage(), STORAGE_KEY);
  if (!stored || typeof stored !== "object" || Array.isArray(stored)) return {};
  const clean: Record<string, boolean> = Object.create(null);
  for (const [key, value] of Object.entries(stored).slice(0, MAX_REMEMBERED))
    if (typeof value === "boolean") clean[key] = value;
  return clean;
}

function remember(key: string, open: boolean) {
  writePresentationJson(optionalLocalStorage(), STORAGE_KEY, { ...readRemembered(), [key]: open });
}

/**
 * A closable part of Properties. Its values stay readable while it is closed, and each tool
 * type keeps its own open sections on this PC. Opening is a deliberate action, so it may move
 * what is below; the summary has a reserved line and live values never move anything (UI-17).
 */
export function PropertySection({
  kind,
  title,
  summary,
  defaultOpen = false,
  children,
}: {
  /** The tool type whose memory this belongs to. */
  kind: string;
  title: string;
  /** Current values, shown beside the title whether open or closed. */
  summary?: ReactNode;
  defaultOpen?: boolean;
  children: ReactNode;
}) {
  const key = `${kind}/${title}`;
  const [open, setOpen] = useState(() => readRemembered()[key] ?? defaultOpen);
  return (
    <details
      className="property-section"
      open={open}
      onToggle={(event) => {
        const next = event.currentTarget.open;
        if (next === open) return;
        setOpen(next);
        remember(key, next);
      }}
    >
      <summary>
        <Icon name={open ? "chevron-down" : "chevron-right"} size={14} />
        <span className="property-section-title">{title}</span>
        {summary !== undefined && <span className="property-section-summary">{summary}</span>}
      </summary>
      {/* A closed section renders nothing, so its hidden views do no telemetry work. */}
      {open && <div className="property-section-body">{children}</div>}
    </details>
  );
}
