import { useEffect, useMemo, useRef, useState, type MouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import type { Node } from "@audiorouter/contracts";
import type { UiBackendSnapshot } from "./backend";
import { askConfirm } from "./AppDialog";
import { SOUNDBOARD_SLOTS, soundboardSlots } from "./soundboard";

type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;

const shellInvoke = (): Invoke | undefined =>
  typeof window === "undefined" ? undefined : (window.__TAURI_INTERNALS__?.invoke as Invoke | undefined);

export type KeybindMode = "press" | "hold";

export type KeybindAction =
  | { type: "sessionToggle" }
  | { type: "privacyMuteToggle" }
  | { type: "privacyMutePushToTalk" }
  | {
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

export const KEYBIND_NAME_MAX = 32;
export const KEYBIND_MAX = 100;

export function keybindLimitMessage(count: number): string | null {
  return count >= KEYBIND_MAX ? `You can add up to ${KEYBIND_MAX} global keybinds.` : null;
}

/** Apply one drag stroke. Returns the same set when nothing changes. */
export function paintKeybindSelection(selected: Set<string>, ids: readonly string[], select: boolean): Set<string> {
  let changed = false;
  const next = new Set(selected);
  for (const id of ids) {
    if (select && !next.has(id)) {
      next.add(id);
      changed = true;
    } else if (!select && next.has(id)) {
      next.delete(id);
      changed = true;
    }
  }
  return changed ? next : selected;
}

export type KeybindHitBox = { id: string; left: number; right: number; top: number; bottom: number };

/** Boxes crossed by a pointer stroke, so a fast slide still selects the rows between samples. */
export function keybindIdsAlongStroke(
  boxes: readonly KeybindHitBox[],
  fromX: number,
  fromY: number,
  toX: number,
  toY: number,
): string[] {
  const top = Math.min(fromY, toY);
  const bottom = Math.max(fromY, toY);
  const left = Math.min(fromX, toX);
  const right = Math.max(fromX, toX);
  return boxes
    .filter((box) => box.bottom >= top && box.top <= bottom && box.right >= left && box.left <= right)
    .map((box) => box.id);
}

export function capKeybindName(name: string): string {
  return [...name].slice(0, KEYBIND_NAME_MAX).join("");
}

const ACTION_LABELS: Record<KeybindAction["type"], string> = {
  sessionToggle: "Start or stop the session",
  privacyMuteToggle: "Turn privacy mute on or off",
  privacyMutePushToTalk: "Push to talk (unmute while held)",
  soundboardTrigger: "Play soundboard slot",
};

/** Push-to-talk and soundboard triggers need the key-up; the others fire once. */
export function triggerModeFor(type: KeybindAction["type"]): KeybindMode {
  return type === "privacyMutePushToTalk" || type === "soundboardTrigger" ? "hold" : "press";
}

/** Enabled rules that share a trigger; every one of them runs on that chord. */
export function duplicateTriggerIds(rules: readonly KeybindRule[]): Set<string> {
  const byTrigger = new Map<string, string[]>();
  for (const rule of rules) {
    if (!rule.enabled) continue;
    const key = rule.trigger.keys.trim().toLowerCase();
    const ids = byTrigger.get(key);
    if (ids) ids.push(rule.id);
    else byTrigger.set(key, [rule.id]);
  }
  const duplicates = new Set<string>();
  for (const ids of byTrigger.values()) {
    if (ids.length > 1) for (const id of ids) duplicates.add(id);
  }
  return duplicates;
}

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

/** Physical key names the Rust parser accepts. Shifted characters (`+`, `?`) are not used. */
const CODE_TO_TOKEN: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Escape: "Escape",
  Backspace: "Backspace",
  Insert: "Insert",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowLeft: "ArrowLeft",
  ArrowUp: "ArrowUp",
  ArrowRight: "ArrowRight",
  ArrowDown: "ArrowDown",
  CapsLock: "CapsLock",
  ScrollLock: "ScrollLock",
  Pause: "Pause",
  PrintScreen: "PrintScreen",
  MediaPlayPause: "MediaPlayPause",
  MediaStop: "MediaStop",
  MediaTrackNext: "MediaNextTrack",
  MediaTrackPrevious: "MediaPreviousTrack",
  AudioVolumeMute: "AudioVolumeMute",
  AudioVolumeDown: "AudioVolumeDown",
  AudioVolumeUp: "AudioVolumeUp",
  Semicolon: "Semicolon",
  Equal: "Equal",
  Comma: "Comma",
  Minus: "Minus",
  Period: "Period",
  Slash: "Slash",
  Backquote: "Backquote",
  BracketLeft: "BracketLeft",
  Backslash: "Backslash",
  BracketRight: "BracketRight",
  Quote: "Quote",
};

const TRIGGER_MODIFIERS = new Set([
  "control",
  "ctrl",
  "lcontrol",
  "lctrl",
  "rcontrol",
  "rctrl",
  "alt",
  "lalt",
  "ralt",
  "shift",
  "lshift",
  "rshift",
  "meta",
  "win",
  "super",
  "lmeta",
  "lwin",
  "rmeta",
  "rwin",
]);

const NAMED_KEYS = new Set([
  "space",
  "enter",
  "return",
  "tab",
  "escape",
  "esc",
  "backspace",
  "insert",
  "delete",
  "home",
  "end",
  "pageup",
  "pagedown",
  "arrowleft",
  "left",
  "arrowup",
  "up",
  "arrowright",
  "right",
  "arrowdown",
  "down",
  "capslock",
  "lcontrol",
  "lctrl",
  "rcontrol",
  "rctrl",
  "lshift",
  "rshift",
  "lalt",
  "ralt",
  "lmeta",
  "lwin",
  "rmeta",
  "rwin",
  "scrolllock",
  "pause",
  "printscreen",
  "mediaplaypause",
  "mediastop",
  "medianexttrack",
  "mediaprevioustrack",
  "audiovolumeup",
  "audiovolumedown",
  "audiovolumemute",
  "semicolon",
  "equal",
  "comma",
  "minus",
  "period",
  "slash",
  "backquote",
  "bracketleft",
  "backslash",
  "bracketright",
  "quote",
  "mousemiddle",
  "mouse3",
  "mouse4",
  "mousex1",
  "mouse5",
  "mousex2",
]);

function isNamedOrPlainKey(token: string): boolean {
  const key = token.toLowerCase();
  if (NAMED_KEYS.has(key)) return true;
  if (key.length === 1 && /[a-z0-9]/.test(key)) return true;
  const fn = /^f(\d{1,2})$/.exec(key);
  if (fn) {
    const number = Number(fn[1]);
    return number >= 1 && number <= 24;
  }
  return /^numpad\d$/.test(key);
}

/** True when the Rust trigger parser would compile `spec` as that chord. */
export function isValidTrigger(spec: string): boolean {
  const parts = spec.split("+");
  if (parts.length === 0 || parts.some((part) => part.trim() === "")) return false;
  const key = parts[parts.length - 1];
  return parts.slice(0, -1).every((part) => TRIGGER_MODIFIERS.has(part.trim().toLowerCase())) && isNamedOrPlainKey(key);
}

type CaptureKeyboardEvent = {
  key: string;
  code?: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
};

export function keyFromKeyboardEvent(event: CaptureKeyboardEvent, heldModifiers: Set<string>): string | null {
  const code = event.code ?? "";
  if (code in MODIFIER_CODE_MAP) return null;
  let token: string | null = null;
  if (/^Numpad[0-9]$/.test(code) || /^F(?:[1-9]|1\d|2[0-4])$/.test(code)) {
    token = code;
  } else {
    const physical = /^(?:Key([A-Z])|Digit([0-9]))$/.exec(code);
    if (physical) {
      const plain = /^[A-Za-z0-9]$/.test(event.key);
      const typed = event.key.length === 1 ? event.key.toUpperCase() : event.key;
      token = !plain ? (physical[1] ?? physical[2]) : typed;
    } else if (code in CODE_TO_TOKEN) {
      token = CODE_TO_TOKEN[code];
    }
  }
  if (!token && event.key.length !== 1 && isNamedOrPlainKey(event.key)) token = event.key;
  if (!token || ["Control", "Meta", "Alt", "Shift"].includes(token)) return null;
  const chord = `${modifierPrefixFromCodes(heldModifiers, event)}${token}`;
  return isValidTrigger(chord) ? chord : null;
}

/** A key that is already down repeats. Capture accepts only a new press. */
export function chordFromCaptureKeyDown(
  event: CaptureKeyboardEvent & { repeat?: boolean },
  pressedDuringCapture: Set<string>,
): string | null {
  if (event.repeat) return null;
  const code = event.code ?? "";
  if (code in MODIFIER_CODE_MAP) {
    pressedDuringCapture.add(code);
    return null;
  }
  return keyFromKeyboardEvent(event, pressedDuringCapture);
}

/** Releasing a modifier sets the shortcut only when that modifier was pressed during this capture. */
export function chordFromCaptureModifierKeyUp(
  code: string,
  pressedDuringCapture: Set<string>,
  usedCombo: boolean,
): string | null {
  const pressedNow = pressedDuringCapture.delete(code);
  if (!pressedNow || usedCombo) return null;
  return MODIFIER_CODE_MAP[code] ?? null;
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
  const label = ACTION_LABELS[action.type];
  if (action.type !== "soundboardTrigger") {
    return `${label} — ${rule.trigger.keys}${suffix}`;
  }
  const board = nodes.find((node) => node.id === action.nodeId);
  const slotLabel = board ? soundboardSlots(board).find((entry) => entry.slot === action.slot)?.label : undefined;
  const detail = slotLabel ? `Slot ${action.slot} — ${slotLabel}` : `Slot ${action.slot}`;
  return `${label}: ${detail} — ${rule.trigger.keys}${suffix}`;
}

function nextKeybindName(rules: KeybindRule[]): string {
  const existing = new Set(rules.map((rule) => (rule.name ?? "").trim()));
  if (!existing.has("keybind")) return "keybind";
  let index = 1;
  while (existing.has(`keybind${index}`)) index += 1;
  return capKeybindName(`keybind${index}`);
}

function KeybindTrashIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      aria-hidden="true"
    >
      <path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7" />
    </svg>
  );
}

export function GlobalKeybinds({ snapshot }: { snapshot?: UiBackendSnapshot | null }) {
  const invoke = shellInvoke();
  const [rules, setRules] = useState<KeybindRule[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [recordingId, setRecordingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<KeybindRule | null>(null);
  const [editingNames, setEditingNames] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const rulesRef = useRef<KeybindRule[]>(rules);
  rulesRef.current = rules;
  const heldModifiersRef = useRef<Set<string>>(new Set());
  const usedModifierComboRef = useRef(false);
  const hitRefs = useRef(new Map<string, HTMLLabelElement>());
  const dragRef = useRef<{ select: boolean; x: number; y: number } | null>(null);
  const addButtonRef = useRef<HTMLButtonElement>(null);

  const nodes = useMemo(() => snapshot?.session?.nodes ?? [], [snapshot?.session?.nodes]);
  const soundboards = useMemo(() => nodes.filter((node) => node.kind === "soundboard"), [nodes]);
  const selectedCount = rules.filter((rule) => selectedIds.has(rule.id)).length;
  const allSelected = rules.length > 0 && selectedCount === rules.length;
  const duplicateIds = duplicateTriggerIds(rules);

  useEffect(() => {
    setSelectedIds((current) => {
      const live = new Set(rules.map((rule) => rule.id));
      let changed = false;
      const next = new Set<string>();
      for (const id of current) {
        if (live.has(id)) next.add(id);
        else changed = true;
      }
      return changed ? next : current;
    });
  }, [rules]);

  useEffect(() => {
    const onMove = (event: PointerEvent) => {
      const drag = dragRef.current;
      if (!drag) return;
      const boxes: KeybindHitBox[] = [];
      for (const [id, element] of hitRefs.current) {
        const rect = element.getBoundingClientRect();
        boxes.push({ id, left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom });
      }
      const ids = keybindIdsAlongStroke(boxes, drag.x, drag.y, event.clientX, event.clientY);
      drag.x = event.clientX;
      drag.y = event.clientY;
      setSelectedIds((current) => paintKeybindSelection(current, ids, drag.select));
    };
    const endDrag = () => {
      dragRef.current = null;
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", endDrag);
    window.addEventListener("pointercancel", endDrag);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", endDrag);
      window.removeEventListener("pointercancel", endDrag);
    };
  }, []);

  useEffect(() => {
    if (!invoke) return;
    let active = true;
    void invoke("keybinds_list")
      .then((result) => {
        if (active && Array.isArray(result)) {
          setRules((result as KeybindRule[]).map((rule) => ({ ...rule, name: capKeybindName(rule.name ?? "") })));
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
    if (!invoke) return false;
    const previous = rulesRef.current;
    setRules(nextRules);
    setBusy(true);
    setMessage(null);
    try {
      const saved = (await invoke("keybinds_save", { rules: nextRules })) as KeybindRule[];
      setRules(saved);
      setMessage("Global keybindings saved.");
      return true;
    } catch (error) {
      setRules(previous);
      setMessage(typeof error === "string" ? error : "Could not save global keybindings.");
      return false;
    } finally {
      setBusy(false);
    }
  };

  const setSuspended = (suspended: boolean) => {
    if (!invoke) return;
    void invoke("keybinds_set_suspended", { suspended }).catch(() => undefined);
  };

  const addRule = () => {
    const limit = keybindLimitMessage(rules.length);
    if (limit) {
      setMessage(limit);
      return;
    }
    const action: KeybindAction = { type: "sessionToggle" };
    const next: KeybindRule = {
      id: `kb-${Date.now().toString(36)}`,
      name: nextKeybindName(rules),
      enabled: true,
      trigger: {
        keys: "",
        mode: "press",
        passthrough: true,
      },
      action,
    };
    setEditingNames(false);
    setMessage(null);
    setDraft(next);
  };

  const updateRule = (id: string, updater: (rule: KeybindRule) => KeybindRule) => {
    if (draft?.id === id) {
      setDraft(updater(draft));
      return;
    }
    void persist(rules.map((rule) => (rule.id === id ? updater(rule) : rule)));
  };

  const exitNameEdit = () => {
    setEditingNames(false);
    void persist(rulesRef.current);
  };

  const updateRuleName = (id: string, name: string) => {
    const capped = capKeybindName(name);
    setRules((current) => current.map((rule) => (rule.id === id ? { ...rule, name: capped } : rule)));
  };
  const removeRules = async (removeIds: Set<string>) => {
    const previousSelection = new Set(selectedIds);
    const targets = rulesRef.current.filter((rule) => removeIds.has(rule.id));
    const count = targets.length;
    if (count === 0) return;
    const name = targets[0].name?.trim() || targets[0].trigger.keys;
    const confirmed = await askConfirm({
      title: count === 1 ? `Delete “${name}”?` : `Delete ${count} keybindings?`,
      message:
        count === 1
          ? `This removes only “${name}” (${targets[0].trigger.keys}). Its shortcut will no longer run. This cannot be undone.`
          : `This removes the ${count} selected global keybindings. Their shortcuts will no longer run. This cannot be undone.`,
      confirmLabel: count === 1 ? "Delete keybinding" : `Delete ${count} keybindings`,
      destructive: true,
    });
    if (!confirmed) return;
    if (await persist(rulesRef.current.filter((rule) => !removeIds.has(rule.id)))) {
      setMessage(count === 1 ? "Keybinding deleted." : `${count} keybindings deleted.`);
      requestAnimationFrame(() => addButtonRef.current?.focus());
    } else {
      setSelectedIds(previousSelection);
    }
  };

  const beginSelectionDrag = (id: string, event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    const select = !selectedIds.has(id);
    dragRef.current = { select, x: event.clientX, y: event.clientY };
    setSelectedIds((current) => paintKeybindSelection(current, [id], select));
  };

  const finishRecording = (ruleId: string, keys: string, target: HTMLInputElement) => {
    if (!isValidTrigger(keys)) {
      setMessage("That shortcut isn't valid. Try a letter, number, function key, or punctuation key.");
      return;
    }
    heldModifiersRef.current.clear();
    setRecordingId(null);
    setSuspended(false);
    target.blur();
    updateRule(ruleId, (current) => ({
      ...current,
      trigger: { ...current.trigger, keys },
    }));
  };

  const selectionCheckbox = (rule: KeybindRule) => (
    <label
      className="keybind-check"
      ref={(node) => {
        if (node) hitRefs.current.set(rule.id, node);
        else hitRefs.current.delete(rule.id);
      }}
    >
      <input
        type="checkbox"
        checked={selectedIds.has(rule.id)}
        aria-label={`Select ${rule.name?.trim() || rule.trigger.keys}`}
        onChange={() => undefined}
        onPointerDown={(event) => beginSelectionDrag(rule.id, event)}
        onKeyDown={(event) => {
          if (event.key !== " " && event.key !== "Enter") return;
          event.preventDefault();
          const select = !selectedIds.has(rule.id);
          setSelectedIds((current) => paintKeybindSelection(current, [rule.id], select));
        }}
      />
    </label>
  );

  return (
    <section className="panel shortcut-panel">
      <div className="section-heading">
        <h3>Global keybindings</h3>
        {invoke && (
          <div className="actions">
            <button
              ref={addButtonRef}
              type="button"
              onClick={addRule}
              disabled={!loaded || busy || !!draft || rules.length >= KEYBIND_MAX}
            >
              Add keybinding
            </button>
            {editingNames ? (
              <>
                <button
                  type="button"
                  className="secondary"
                  aria-label={allSelected ? "Clear selection" : "Select all keybinds"}
                  title={allSelected ? "Clear selection" : "Select all keybinds"}
                  onClick={() => setSelectedIds(allSelected ? new Set() : new Set(rules.map((rule) => rule.id)))}
                  disabled={!loaded || busy || rules.length === 0}
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    aria-hidden="true"
                  >
                    <rect x="4" y="4" width="16" height="16" rx="2" />
                    <path d="M8 12.5l2.5 2.5L16 9" />
                  </svg>
                </button>
                <button
                  type="button"
                  className="danger keybind-delete-selected"
                  title="Delete the selected keybindings"
                  onClick={() => void removeRules(new Set(selectedIds))}
                  disabled={!loaded || busy || selectedCount === 0}
                >
                  <KeybindTrashIcon />
                  Delete selected <span className="keybind-selection-count">({selectedCount})</span>
                </button>
                <button
                  type="button"
                  className="secondary"
                  aria-label="Done editing keybind names"
                  title="Exit name edit mode"
                  onClick={() => {
                    setSelectedIds(new Set());
                    exitNameEdit();
                  }}
                >
                  Done
                </button>
              </>
            ) : (
              <button
                type="button"
                className="secondary keybind-manage"
                aria-label="Manage keybindings"
                title="Rename keybindings or select several to delete"
                disabled={!!draft || busy || !loaded}
                onClick={() => setEditingNames(true)}
              >
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
                Manage
              </button>
            )}
          </div>
        )}
      </div>
      <p className="muted">
        Windows-wide hotkeys that work even when AudioRouter is minimized to the tray. Supports keyboard combos,
        function keys (F1–F24), and mouse side/middle buttons. These are the app's only audio shortcuts.
      </p>
      {editingNames && (
        <p className="muted">Select keybindings to delete, or edit their names. Choose Done when finished.</p>
      )}
      {!invoke ? (
        <small className="muted">Global Windows keybindings are available in the desktop app.</small>
      ) : rules.length === 0 && !draft ? (
        <p className="muted">
          No global keybindings yet. Add one to start or stop the session, mute the microphone, or play a soundboard
          slot.
        </p>
      ) : (
        <div className="keybind-list">
          {(draft ? [draft, ...rules.filter((rule) => rule.id !== draft.id)] : rules).map((rule) => {
            const isRecording = recordingId === rule.id;
            const isDraft = draft?.id === rule.id;
            const soundboardAction = rule.action.type === "soundboardTrigger" ? rule.action : null;
            return (
              <div key={rule.id} className="keybind-row">
                {editingNames ? (
                  <div className="keybind-edit-row">
                    {selectionCheckbox(rule)}
                    <input
                      className="keybind-name"
                      value={rule.name ?? ""}
                      maxLength={KEYBIND_NAME_MAX}
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
                ) : (
                  <details className="keybind-body" inert={busy} open={draft?.id === rule.id ? true : undefined}>
                    <summary>
                      <span className="keybind-title">
                        {isDraft ? "New keybinding" : describeRuleSummary(rule, nodes)}
                        {!isDraft && rule.name?.trim() && (
                          <small className="muted">
                            {rule.trigger.keys} · {ACTION_LABELS[rule.action.type]}
                          </small>
                        )}
                      </span>
                    </summary>
                    {isDraft && (
                      <>
                        <p className="muted">
                          Name it, choose what it does, and record a shortcut. It becomes active only when you save.
                        </p>
                        <label>
                          Name
                          <input
                            autoFocus
                            value={rule.name ?? ""}
                            maxLength={KEYBIND_NAME_MAX}
                            placeholder="e.g. Mute my microphone"
                            onFocus={(event) => event.currentTarget.select()}
                            onChange={(event) => setDraft({ ...rule, name: capKeybindName(event.target.value) })}
                          />
                        </label>
                      </>
                    )}
                    <label>
                      Action
                      <select
                        value={rule.action.type}
                        onChange={(event) => {
                          const nextType = event.target.value as KeybindAction["type"];
                          updateRule(rule.id, (current) => {
                            const action: KeybindAction =
                              nextType === "soundboardTrigger" ? defaultAction(soundboards) : { type: nextType };
                            return {
                              ...current,
                              trigger: { ...current.trigger, mode: triggerModeFor(nextType) },
                              action,
                            };
                          });
                        }}
                      >
                        {Object.entries(ACTION_LABELS).map(([value, label]) => (
                          <option key={value} value={value}>
                            {label}
                          </option>
                        ))}
                      </select>
                    </label>

                    {soundboardAction && (
                      <>
                        <label>
                          Soundboard
                          {soundboards.length > 0 ? (
                            <select
                              value={soundboardAction.nodeId}
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
                          ) : isDraft ? (
                            <small className="muted">
                              Add a Soundboard tool to this session first, or choose another action.
                            </small>
                          ) : (
                            <input
                              value={soundboardAction.nodeId}
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
                            value={soundboardAction.slot}
                            onChange={(event) => {
                              const slot = Math.min(SOUNDBOARD_SLOTS, Math.max(1, Number(event.target.value) || 1));
                              updateRule(rule.id, (current) =>
                                current.action.type === "soundboardTrigger"
                                  ? {
                                      ...current,
                                      trigger: { ...current.trigger, mode: "hold" },
                                      action: { ...current.action, slot, mode: null },
                                    }
                                  : current,
                              );
                            }}
                          >
                            {((targetNodeId) => {
                              const selectedNode = soundboards.find((node) => node.id === targetNodeId);
                              const assignedBySlot = new Map(
                                (selectedNode ? soundboardSlots(selectedNode) : []).map((entry) => [
                                  entry.slot,
                                  entry.label,
                                ]),
                              );
                              return Array.from({ length: SOUNDBOARD_SLOTS }, (_, index) => index + 1).map((slot) => {
                                const label = assignedBySlot.get(slot);
                                return (
                                  <option key={slot} value={slot}>
                                    {label ? `Slot ${slot} — ${label}` : `Slot ${slot}`}
                                  </option>
                                );
                              });
                            })(soundboardAction.nodeId)}
                          </select>
                        </label>
                      </>
                    )}
                    {rule.action.type === "privacyMutePushToTalk" && (
                      <small className="muted">Unmutes while the key is held and mutes again when released.</small>
                    )}

                    <label>
                      Shortcut
                      <input
                        readOnly
                        placeholder="Click here, then press your shortcut"
                        value={isRecording ? "Press key or mouse4/5…" : rule.trigger.keys}
                        aria-label={`Global trigger for ${rule.name?.trim() || ACTION_LABELS[rule.action.type]}`}
                        onFocus={() => {
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
                          if (!isRecording || event.repeat) return;
                          if (event.key === "Tab") {
                            setRecordingId(null);
                            heldModifiersRef.current.clear();
                            setSuspended(false);
                            return;
                          }
                          event.preventDefault();
                          const keys = chordFromCaptureKeyDown(event, heldModifiersRef.current);
                          if (!keys) {
                            if (event.code in MODIFIER_CODE_MAP) return;
                            setMessage(
                              "That key isn't supported. Use a letter, number, function key, or punctuation key.",
                            );
                            return;
                          }
                          usedModifierComboRef.current = true;
                          finishRecording(rule.id, keys, event.currentTarget);
                        }}
                        onKeyUp={(event) => {
                          if (!isRecording) return;
                          const keys = chordFromCaptureModifierKeyUp(
                            event.code,
                            heldModifiersRef.current,
                            usedModifierComboRef.current,
                          );
                          if (!keys) return;
                          event.preventDefault();
                          finishRecording(rule.id, keys, event.currentTarget);
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
                    <small className="keybind-capture-help muted">
                      {isRecording
                        ? "Listening… Press Tab or click another field to cancel recording."
                        : "Click or Tab into the shortcut field, then press a key combination or mouse side button."}
                    </small>

                    <div className="keybind-toggles">
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
                    </div>
                    {!isDraft && (
                      <div className="actions keybind-delete-action">
                        <button
                          type="button"
                          className="danger"
                          disabled={busy}
                          aria-label={`Delete keybinding ${rule.name?.trim() || rule.trigger.keys}`}
                          onClick={() => void removeRules(new Set([rule.id]))}
                        >
                          <KeybindTrashIcon />
                          Delete keybinding
                        </button>
                      </div>
                    )}
                    {isDraft && (
                      <>
                        <p className="keybind-draft-status muted" role="status">
                          {!rule.name?.trim()
                            ? "Enter a name for this keybinding."
                            : !rule.trigger.keys
                              ? "Record a shortcut before saving."
                              : soundboardAction && !soundboardAction.nodeId.trim()
                                ? "Choose a soundboard before saving."
                                : rules.some(
                                      (saved) =>
                                        saved.id !== rule.id &&
                                        saved.enabled &&
                                        saved.trigger.keys.toLowerCase() === rule.trigger.keys.toLowerCase(),
                                    )
                                  ? "This shortcut is already assigned. Both actions will run."
                                  : `Ready: ${rule.trigger.keys} → ${ACTION_LABELS[rule.action.type]}.`}
                        </p>
                        <div className="actions">
                          <button
                            type="button"
                            disabled={
                              busy ||
                              !rule.name?.trim() ||
                              !isValidTrigger(rule.trigger.keys) ||
                              !!(soundboardAction && !soundboardAction.nodeId.trim())
                            }
                            onClick={async () => {
                              if (await persist([...rulesRef.current, rule])) {
                                setDraft(null);
                                requestAnimationFrame(() => addButtonRef.current?.focus());
                              }
                            }}
                          >
                            Save keybinding
                          </button>
                          <button
                            type="button"
                            className="secondary"
                            disabled={busy}
                            onClick={() => {
                              setDraft(null);
                              setRecordingId(null);
                              heldModifiersRef.current.clear();
                              setSuspended(false);
                              setMessage(null);
                              requestAnimationFrame(() => addButtonRef.current?.focus());
                            }}
                          >
                            Cancel
                          </button>
                        </div>
                      </>
                    )}
                  </details>
                )}
              </div>
            );
          })}
        </div>
      )}
      {duplicateIds.size > 0 && (
        <p className="muted" role="alert">
          {duplicateIds.size === 2 ? "Two keybinds share" : `${duplicateIds.size} keybinds share`} a shortcut; each one
          runs. Change one to keep them separate.
        </p>
      )}
      <p className="keybind-draft-status muted" role="status">
        {message ?? ""}
      </p>
    </section>
  );
}
