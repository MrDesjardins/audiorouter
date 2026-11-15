import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import type { Node } from "@audiorouter/contracts";
import type { UiBackendSnapshot } from "./backend";
import { SOUNDBOARD_SLOTS, soundboardSlots } from "./soundboard";

type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;

const shellInvoke = (): Invoke | undefined =>
  typeof window === "undefined" ? undefined : (window.__TAURI_INTERNALS__?.invoke as Invoke | undefined);

export type KeybindMode = "press" | "hold";

export type KeybindAction = {
  type: "soundboardTrigger";
  sessionId?: string | null;
  nodeId: string;
  slot: number;
  mode?: string | null;
};

export interface KeybindRule {
  id: string;
  name?: string;
  enabled: boolean;
  trigger: {
    keys: string;
    mode: KeybindMode;
    passthrough: boolean;
  };
  action: KeybindAction;
}

const ACTION_LABELS: Record<KeybindAction["type"], string> = {
  soundboardTrigger: "Play soundboard slot",
};

const MODIFIER_CODE_MAP: Record<string, string> = {
  ControlLeft: "LControl",
  ControlRight: "RControl",
  AltLeft: "LAlt",
  AltRight: "RAlt",
  ShiftLeft: "LShift",
  ShiftRight: "RShift",
  MetaLeft: "LMeta",
  MetaRight: "RMeta",
};

const MODIFIER_GROUPS = [
  ["ControlLeft", "ControlRight", "LControl", "RControl", "Control", "ctrlKey"],
  ["AltLeft", "AltRight", "LAlt", "RAlt", "Alt", "altKey"],
  ["ShiftLeft", "ShiftRight", "LShift", "RShift", "Shift", "shiftKey"],
  ["MetaLeft", "MetaRight", "LMeta", "RMeta", "Meta", "metaKey"],
] as const;

function modifierPrefixFromCodes(
  heldCodes: Set<string>,
  event: { ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean },
): string {
  const parts: string[] = [];
  for (const [leftCode, rightCode, leftName, rightName, anyName, flag] of MODIFIER_GROUPS) {
    if (heldCodes.has(leftCode)) parts.push(leftName);
    else if (heldCodes.has(rightCode)) parts.push(rightName);
    else if (event[flag]) parts.push(anyName);
  }
  return parts.length > 0 ? `${parts.join("+")}+` : "";
}

function keyFromKeyboardEvent(event: KeyboardEvent<HTMLInputElement>, heldModifiers: Set<string>): string | null {
  if (event.code in MODIFIER_CODE_MAP) return null;
  if (/^Numpad[0-9]$/.test(event.code ?? "")) {
    return `${modifierPrefixFromCodes(heldModifiers, event)}${event.code}`;
  }
  const physical = /^(?:Key([A-Z])|Digit([0-9]))$/.exec(event.code ?? "");
  const plain = /^[A-Za-z0-9]$/.test(event.key);
  const typed = event.key.length === 1 ? event.key.toUpperCase() : event.key;
  const key = !plain && physical ? (physical[1] ?? physical[2]) : typed;
  if (["Control", "Meta", "Alt", "Shift"].includes(key)) return null;
  const normalized = key === " " ? "Space" : key;
  return `${modifierPrefixFromCodes(heldModifiers, event)}${normalized}`;
}

function keyFromMouseEvent(event: MouseEvent<HTMLInputElement>, heldModifiers: Set<string>): string | null {
  const button =
    event.button === 1 ? "MouseMiddle" : event.button === 3 ? "Mouse4" : event.button === 4 ? "Mouse5" : null;
  if (!button) return null;
  return `${modifierPrefixFromCodes(heldModifiers, event)}${button}`;
}

function defaultAction(soundboards: Node[]): KeybindAction {
  return {
    type: "soundboardTrigger",
    nodeId: soundboards[0]?.id ?? "",
    slot: 1,
    mode: null,
  };
}

function describeRuleSummary(rule: KeybindRule, nodes: Node[]): string {
  const { action } = rule;
  const customName = rule.name?.trim();
  const suffix = rule.enabled ? "" : " (disabled)";
  if (customName) {
    return `${customName}${suffix}`;
  }
  const board = nodes.find((node) => node.id === action.nodeId);
  const slotLabel = board ? soundboardSlots(board).find((entry) => entry.slot === action.slot)?.label : undefined;
  const detail = slotLabel ? `Slot ${action.slot} — ${slotLabel}` : `Slot ${action.slot}`;
  return `Play soundboard slot: ${detail} — ${rule.trigger.keys}${suffix}`;
}

function nextKeybindName(rules: KeybindRule[]): string {
  const existing = new Set(rules.map((rule) => (rule.name ?? "").trim()));
  if (!existing.has("keybind")) return "keybind";
  let index = 1;
  while (existing.has(`keybind${index}`)) index += 1;
  return `keybind${index}`;
}

export function GlobalKeybinds({ snapshot }: { snapshot?: UiBackendSnapshot | null }) {
  const invoke = shellInvoke();
  const [rules, setRules] = useState<KeybindRule[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [recordingId, setRecordingId] = useState<string | null>(null);
  const [newlyAddedId, setNewlyAddedId] = useState<string | null>(null);
  const [editingNames, setEditingNames] = useState(false);
  const rulesRef = useRef<KeybindRule[]>(rules);
  rulesRef.current = rules;
  const heldModifiersRef = useRef<Set<string>>(new Set());
  const usedModifierComboRef = useRef(false);

  const nodes = useMemo(() => snapshot?.session?.nodes ?? [], [snapshot?.session?.nodes]);
  const soundboards = useMemo(() => nodes.filter((node) => node.kind === "soundboard"), [nodes]);

  useEffect(() => {
    if (!invoke) return;
    let active = true;
    void invoke("keybinds_list")
      .then((result) => {
        if (active && Array.isArray(result)) {
          setRules(result as KeybindRule[]);
          setLoaded(true);
        }
      })
      .catch(() => {
        if (active) setMessage("Could not load global keybindings.");
      });
    return () => {
      active = false;
      void invoke("keybinds_set_suspended", { suspended: false }).catch(() => undefined);
    };
  }, [invoke]);

  const persist = async (nextRules: KeybindRule[]) => {
    if (!invoke) return;
    setRules(nextRules);
    setBusy(true);
    setMessage(null);
    try {
      const saved = (await invoke("keybinds_save", { rules: nextRules })) as KeybindRule[];
      setRules(saved);
      setMessage("Global keybindings saved.");
    } catch (error) {
      setMessage(typeof error === "string" ? error : "Could not save global keybindings.");
    } finally {
      setBusy(false);
    }
  };

  const setSuspended = (suspended: boolean) => {
    if (!invoke) return;
    void invoke("keybinds_set_suspended", { suspended }).catch(() => undefined);
  };

  const addRule = () => {
    const action = defaultAction(soundboards);
    const next: KeybindRule = {
      id: `kb-${Date.now().toString(36)}`,
      name: nextKeybindName(rules),
      enabled: true,
      trigger: {
        keys: "F13",
        mode: "hold",
        passthrough: true,
      },
      action,
    };
    setNewlyAddedId(next.id);
    void persist([...rules, next]);
  };

  const updateRule = (id: string, updater: (rule: KeybindRule) => KeybindRule) => {
    void persist(rules.map((rule) => (rule.id === id ? updater(rule) : rule)));
  };

  const exitNameEdit = () => {
    setEditingNames(false);
    void persist(rulesRef.current);
  };

  const updateRuleName = (id: string, name: string) => {
    setRules((current) => current.map((rule) => (rule.id === id ? { ...rule, name } : rule)));
  };
  const removeRule = (id: string) => {
    void persist(rules.filter((rule) => rule.id !== id));
  };

  const finishRecording = (ruleId: string, keys: string, target: HTMLInputElement) => {
    heldModifiersRef.current.clear();
    setRecordingId(null);
    setSuspended(false);
    target.blur();
    updateRule(ruleId, (current) => ({
      ...current,
      trigger: { ...current.trigger, keys },
    }));
  };

  return (
    <section className="panel shortcut-panel">
      <div className="section-heading">
        <h3>Global keybindings</h3>
        {invoke && (
          <div className="actions">
            {rules.length > 0 && (
              <button
                type="button"
                className="secondary"
                aria-label={editingNames ? "Done editing keybind names" : "Edit keybind names"}
                title={editingNames ? "Exit name edit mode" : "Edit keybind names"}
                onClick={() => {
                  if (editingNames) exitNameEdit();
                  else setEditingNames(true);
                }}
              >
                {editingNames ? (
                  "Done"
                ) : (
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    aria-hidden="true"
                  >
                    <path d="M12 20h9" />
                    <path d="M16.5 3.5a2.121 2.121 0 1 1 3 3L7 19l-4 1 1-4 12.5-12.5z" />
                  </svg>
                )}
              </button>
            )}
            <button type="button" className="secondary" onClick={addRule} disabled={!loaded || busy}>
              Add global keybind
            </button>
          </div>
        )}
      </div>
      <p className="muted">
        Windows-wide hotkeys that work even when AudioRouter is minimized to the tray. Supports keyboard combos,
        function keys (F1–F24), and mouse side/middle buttons.
      </p>
      {!invoke ? (
        <small className="muted">Global Windows keybindings are available in the desktop app.</small>
      ) : rules.length === 0 ? (
        <p className="muted">No global keybindings yet. Add one for a soundboard slot, mic switch, or push-to-talk.</p>
      ) : (
        <div className="keybind-list">
          {rules.map((rule) => {
            const isRecording = recordingId === rule.id;
            if (editingNames) {
              return (
                <div key={rule.id} className="keybind-row">
                  <input
                    value={rule.name ?? ""}
                    placeholder={describeRuleSummary({ ...rule, name: "" }, nodes)}
                    aria-label={`Name for ${rule.trigger.keys}`}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === "Escape") {
                        event.preventDefault();
                        exitNameEdit();
                      }
                    }}
                    onChange={(event) => updateRuleName(rule.id, event.target.value)}
                  />
                </div>
              );
            }
            return (
              <details key={rule.id} className="keybind-row" open={newlyAddedId === rule.id ? true : undefined}>
                <summary>{describeRuleSummary(rule, nodes)}</summary>
                <label className="autoplay-toggle">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={(event) =>
                      updateRule(rule.id, (current) => ({ ...current, enabled: event.target.checked }))
                    }
                  />
                  Enabled
                </label>

                <label>
                  Action
                  <select
                    value={rule.action.type}
                    onChange={(event) => {
                      const nextType = event.target.value as KeybindAction["type"];
                      if (nextType === "soundboardTrigger") {
                        updateRule(rule.id, (current) => ({
                          ...current,
                          action: defaultAction(soundboards),
                        }));
                      }
                    }}
                  >
                    {Object.entries(ACTION_LABELS).map(([value, label]) => (
                      <option key={value} value={value}>
                        {label}
                      </option>
                    ))}
                  </select>
                </label>

                <label>
                  Soundboard
                  {soundboards.length > 0 ? (
                    <select
                      value={rule.action.nodeId}
                      onChange={(event) =>
                        updateRule(rule.id, (current) => ({
                          ...current,
                          action:
                            current.action.type === "soundboardTrigger"
                              ? { ...current.action, nodeId: event.target.value }
                              : current.action,
                        }))
                      }
                    >
                      {soundboards.map((node) => (
                        <option key={node.id} value={node.id}>
                          {node.name}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      value={rule.action.nodeId}
                      placeholder="Soundboard tool ID"
                      onChange={(event) =>
                        updateRule(rule.id, (current) => ({
                          ...current,
                          action:
                            current.action.type === "soundboardTrigger"
                              ? { ...current.action, nodeId: event.target.value }
                              : current.action,
                        }))
                      }
                    />
                  )}
                </label>
                <label>
                  Slot
                  <select
                    value={rule.action.slot}
                    onChange={(event) => {
                      const slot = Math.min(SOUNDBOARD_SLOTS, Math.max(1, Number(event.target.value) || 1));
                      updateRule(rule.id, (current) => ({
                        ...current,
                        trigger: { ...current.trigger, mode: "hold" },
                        action: { ...current.action, slot, mode: null },
                      }));
                    }}
                  >
                    {((targetNodeId) => {
                      const selectedNode = soundboards.find((node) => node.id === targetNodeId);
                      const assignedBySlot = new Map(
                        (selectedNode ? soundboardSlots(selectedNode) : []).map((entry) => [entry.slot, entry.label]),
                      );
                      return Array.from({ length: SOUNDBOARD_SLOTS }, (_, index) => index + 1).map((slot) => {
                        const label = assignedBySlot.get(slot);
                        return (
                          <option key={slot} value={slot}>
                            {label ? `Slot ${slot} — ${label}` : `Slot ${slot}`}
                          </option>
                        );
                      });
                    })(rule.action.nodeId)}
                  </select>
                </label>

                <label>
                  Trigger
                  <input
                    readOnly
                    value={isRecording ? "Press key or mouse4/5…" : rule.trigger.keys}
                    aria-label={`Global trigger for ${rule.name?.trim() || `Slot ${rule.action.slot}`}`}
                    onClick={() => {
                      heldModifiersRef.current.clear();
                      usedModifierComboRef.current = false;
                      setRecordingId(rule.id);
                      setSuspended(true);
                    }}
                    onBlur={() => {
                      heldModifiersRef.current.clear();
                      usedModifierComboRef.current = false;
                      setRecordingId(null);
                      setSuspended(false);
                    }}
                    onKeyDown={(event) => {
                      if (!isRecording) return;
                      event.preventDefault();
                      if (event.code in MODIFIER_CODE_MAP) {
                        heldModifiersRef.current.add(event.code);
                        return;
                      }
                      const keys = keyFromKeyboardEvent(event, heldModifiersRef.current);
                      if (!keys) return;
                      usedModifierComboRef.current = true;
                      finishRecording(rule.id, keys, event.currentTarget);
                    }}
                    onKeyUp={(event) => {
                      if (!isRecording || !(event.code in MODIFIER_CODE_MAP)) return;
                      event.preventDefault();
                      const standalone = MODIFIER_CODE_MAP[event.code];
                      heldModifiersRef.current.delete(event.code);
                      if (!usedModifierComboRef.current && standalone) {
                        finishRecording(rule.id, standalone, event.currentTarget);
                      }
                    }}
                    onMouseDown={(event) => {
                      if (!isRecording) return;
                      const keys = keyFromMouseEvent(event, heldModifiersRef.current);
                      if (!keys) return;
                      event.preventDefault();
                      usedModifierComboRef.current = true;
                      finishRecording(rule.id, keys, event.currentTarget);
                    }}
                  />
                </label>

                <label
                  className="autoplay-toggle"
                  title="When checked, other apps still receive this key/button. Uncheck to block other apps from seeing it."
                >
                  <input
                    type="checkbox"
                    checked={rule.trigger.passthrough}
                    onChange={(event) =>
                      updateRule(rule.id, (current) => ({
                        ...current,
                        trigger: { ...current.trigger, passthrough: event.target.checked },
                      }))
                    }
                  />
                  Pass key through
                </label>

                <button type="button" className="secondary" onClick={() => removeRule(rule.id)}>
                  Remove
                </button>
              </details>
            );
          })}
        </div>
      )}
      {message && (
        <p className="muted" role="status">
          {message}
        </p>
      )}
    </section>
  );
}
