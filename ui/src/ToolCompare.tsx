import { useEffect, useEffectEvent, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import type { Node, Session, ToolPreset } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";

import { Icon } from "./icons";
import { uiIdempotencyKey } from "./idempotency";
import { canCompare, chainOf } from "./compareRules";
import { setComparing } from "./compareHold";

/**
 * Press and hold to hear the route without this tool, or its whole chain, then
 * let go. Nothing is saved (DEC-UI-21b). Release, losing focus and leaving the
 * panel all restore it, and the backend ends a forgotten hold by itself.
 */
export function ToolCompare({
  backend,
  session,
  node,
  running,
}: {
  backend: UiBackend;
  session: Session;
  node: Node;
  running: boolean;
}) {
  const [held, setHeld] = useState<"tool" | "chain" | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const holding = useRef(false);
  const chain = chainOf(session, node.id);
  const chainNames = chain.map((id) =>
    id === node.id ? `${node.name} (this tool)` : (session.nodes.find((item) => item.id === id)?.name ?? id),
  );

  const release = () => {
    if (!holding.current) return;
    holding.current = false;
    setHeld(null);
    setComparing([]);
    void backend
      .compareNodes?.({ sessionId: session.id, nodes: [], idempotencyKey: uiIdempotencyKey("compare-release") })
      .catch((error) => setMessage(formatUiError(error, "Could not end the comparison.")));
  };
  const releaseFromEffect = useEffectEvent(release);
  useEffect(() => {
    const onHidden = () => releaseFromEffect();
    window.addEventListener("blur", onHidden);
    document.addEventListener("visibilitychange", onHidden);
    return () => {
      window.removeEventListener("blur", onHidden);
      document.removeEventListener("visibilitychange", onHidden);
      releaseFromEffect();
    };
  }, []);
  // Choosing another tool leaves this panel's comparison behind.
  const nodeId = node.id;
  useEffect(() => () => releaseFromEffect(), [nodeId]);

  const hold = (which: "tool" | "chain") => {
    if (!running || holding.current || !backend.compareNodes) return;
    holding.current = true;
    setHeld(which);
    setMessage(null);
    const nodes = which === "tool" ? [node.id] : chain;
    setComparing(nodes);
    void backend
      .compareNodes({
        sessionId: session.id,
        nodes,
        idempotencyKey: uiIdempotencyKey("compare-hold"),
      })
      .catch((error) => {
        holding.current = false;
        setHeld(null);
        setComparing([]);
        setMessage(formatUiError(error, "This route could not be compared."));
      });
  };
  const handlers = (which: "tool" | "chain") => ({
    onPointerDown: (event: ReactPointerEvent<HTMLButtonElement>) => {
      event.currentTarget.setPointerCapture?.(event.pointerId);
      hold(which);
    },
    onPointerUp: () => release(),
    onPointerCancel: () => release(),
    onLostPointerCapture: () => release(),
    onKeyDown: (event: React.KeyboardEvent) => {
      if ((event.key === " " || event.key === "Enter") && !event.repeat) {
        event.preventDefault();
        hold(which);
      }
    },
    onKeyUp: (event: React.KeyboardEvent) => {
      if (event.key === " " || event.key === "Enter") release();
    },
    onBlur: () => release(),
  });

  if (!canCompare(node)) return null;
  return (
    <section className="tool-compare" aria-labelledby="tool-compare-heading">
      <h3 id="tool-compare-heading">Compare by ear</h3>
      <div className="tool-compare-buttons">
        <button
          type="button"
          className="secondary"
          disabled={!running}
          aria-pressed={held === "tool"}
          {...handlers("tool")}
        >
          <Icon name="ear" size={15} />
          Hold: without this tool
        </button>
        <button
          type="button"
          className="secondary"
          disabled={!running || chain.length < 2}
          aria-pressed={held === "chain"}
          {...handlers("chain")}
        >
          <Icon name="ear" size={15} />
          {chain.length < 2 ? "Hold: without the whole line" : `Hold: without all ${chain.length} tools in this line`}
        </button>
      </div>
      {/* Says which tools the second button leaves out: those before and after this one (owner, 2026-11-12).
          One reserved line, so the panel never moves (UI-17). */}
      <p className="tool-compare-line muted" title={chainNames.join(" → ")}>
        {chain.length < 2
          ? "No other tools are connected in a line with this one."
          : `Leaves out, in order: ${chainNames.join(" → ")}`}
      </p>
      <p className="tool-compare-note" role="status">
        <Icon name={running && !message ? "triangle-alert" : "info"} size={15} />
        <span>
          {message ??
            (running
              ? "This is live. While you hold, anyone hearing this route, on Discord or a stream, hears it too."
              : "Press Play to compare. Nothing here changes your saved settings.")}
        </span>
      </p>
    </section>
  );
}

/** Named settings for this kind of tool. Choosing one is an ordinary, undoable draft edit. */
export function ToolPresetPicker({
  backend,
  node,
  disabled,
  onApply,
}: {
  backend: UiBackend;
  node: Node;
  disabled: boolean;
  onApply: (parameters: ToolPreset["parameters"], name: string) => void;
}) {
  const [presets, setPresets] = useState<ToolPreset[] | null>(null);
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [selected, setSelected] = useState("");
  const kind = node.kind;
  const nodeId = node.id;
  const refresh = () => {
    void backend
      .listToolPresets?.(kind)
      .then((result) => setPresets(result.items))
      .catch(() => setPresets(null));
  };
  const refreshFromEffect = useEffectEvent(refresh);
  useEffect(() => {
    setSelected("");
    setNaming(false);
    setMessage(null);
    refreshFromEffect();
  }, [kind, nodeId, backend.connected]);

  const save = async () => {
    const trimmed = name.trim();
    if (!trimmed || !backend.saveToolPreset) return;
    try {
      const result = await backend.saveToolPreset({
        kind,
        name: trimmed,
        parameters: node.parameters,
        idempotencyKey: uiIdempotencyKey("tool-preset"),
      });
      setSelected(result.preset.id);
      setNaming(false);
      setName("");
      setMessage(`Saved “${result.preset.name}”.`);
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to save this preset."));
    }
  };
  const remove = async () => {
    if (!selected || !backend.deleteToolPreset) return;
    try {
      await backend.deleteToolPreset({ id: selected, idempotencyKey: uiIdempotencyKey("tool-preset-delete") });
      setSelected("");
      setMessage("Preset deleted.");
      refresh();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to delete this preset."));
    }
  };

  return (
    <section className="tool-presets" aria-labelledby="tool-presets-heading">
      <h3 id="tool-presets-heading">My presets</h3>
      {naming ? (
        <form
          className="tool-presets-row"
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <input
            aria-label="Preset name"
            value={name}
            maxLength={64}
            autoFocus
            placeholder="Name"
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") setNaming(false);
            }}
          />
          <button type="submit" disabled={!name.trim()}>
            Save
          </button>
          <button type="button" className="secondary" onClick={() => setNaming(false)}>
            Cancel
          </button>
        </form>
      ) : (
        <div className="tool-presets-row">
          <select
            aria-label="Preset"
            value={selected}
            disabled={disabled || !presets?.length}
            onChange={(event) => {
              const preset = presets?.find((item) => item.id === event.target.value);
              setSelected(event.target.value);
              if (preset) {
                onApply(preset.parameters, preset.name);
                setMessage(`Applied “${preset.name}”. Save to keep it on this route.`);
              }
            }}
          >
            <option value="">{presets?.length ? "Choose a preset" : "None saved yet"}</option>
            {presets?.map((preset) => (
              <option key={preset.id} value={preset.id}>
                {preset.name}
              </option>
            ))}
          </select>
          <button type="button" className="secondary" disabled={disabled} onClick={() => setNaming(true)}>
            Save as…
          </button>
          <button type="button" className="secondary" disabled={!selected} onClick={() => void remove()}>
            Delete
          </button>
        </div>
      )}
      <p className="tool-presets-message" role="status">
        {message ?? ""}
      </p>
    </section>
  );
}
