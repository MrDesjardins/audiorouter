// Properties editor of an Instrument node: where its notes come from, an on-screen keyboard and a panic button.
import { useEffect, useEffectEvent, useRef, useState } from "react";
import type { Node } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { PanelMessage } from "./PanelMessage";
import { useMessages } from "./i18n/useMessages";

/** First key of the on-screen keyboard (C4 is 60; two octaves from here). */
const FIRST_NOTE = 60;
const OCTAVES = 2;
const WHITE_OFFSETS = [0, 2, 4, 5, 7, 9, 11];
const BLACK_AFTER_WHITE = new Map<number, number>([
  [0, 1],
  [1, 3],
  [3, 6],
  [4, 8],
  [5, 10],
]);
const NOTE_NAMES = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];

export const INSTRUMENT_KEYBOARD_ONLY = "keyboard";
export const INSTRUMENT_ALL_DEVICES = "all";

/** "C4" for MIDI note 60. */
export function noteLabel(note: number): string {
  return `${NOTE_NAMES[note % 12]}${Math.floor(note / 12) - 1}`;
}

type Key = { note: number; black: boolean; column: number };

/** The keys of the on-screen keyboard, white keys in a grid of columns and black keys between them. */
export function keyboardKeys(): Key[] {
  const keys: Key[] = [];
  const whites = WHITE_OFFSETS.length * OCTAVES + 1;
  for (let white = 0; white < whites; white += 1) {
    const octave = Math.floor(white / WHITE_OFFSETS.length);
    const degree = white % WHITE_OFFSETS.length;
    keys.push({ note: FIRST_NOTE + octave * 12 + WHITE_OFFSETS[degree], black: false, column: white });
    const black = BLACK_AFTER_WHITE.get(degree);
    if (black !== undefined && white < whites - 1) {
      keys.push({ note: FIRST_NOTE + octave * 12 + black, black: true, column: white });
    }
  }
  return keys;
}

export function InstrumentNodeEditor({
  node,
  backend,
  sessionId,
  running,
  disabled,
  onChange,
}: {
  node: Node;
  backend: UiBackend;
  sessionId: string;
  running: boolean;
  disabled: boolean;
  onChange: (name: string, value: string) => void;
}) {
  const tr = useMessages();
  const [devices, setDevices] = useState<string[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const held = useRef(new Set<number>());
  const requestTail = useRef<Promise<unknown> | null>(null);
  const saved = typeof node.parameters.midiInput === "string" ? node.parameters.midiInput : INSTRUMENT_KEYBOARD_ONLY;
  const midiInput = saved === "" ? INSTRUMENT_KEYBOARD_ONLY : saved;
  const canPlay = running && !disabled && Boolean(backend.midiSendEvents);

  const refreshDevices = async () => {
    if (!backend.midiDevices) return;
    try {
      setDevices((await backend.midiDevices()).devices);
    } catch {
      // Keep the last list: the select still shows the saved choice.
    }
  };
  const loadDevices = useEffectEvent(() => void refreshDevices());
  useEffect(() => {
    loadDevices();
  }, [backend.connected]);

  const sendTo = async (
    target: UiBackend,
    targetSession: string,
    targetNode: string,
    events: Parameters<NonNullable<UiBackend["midiSendEvents"]>>[2],
  ) => {
    const sendEvents = target.midiSendEvents;
    if (!sendEvents) return;
    const previous = requestTail.current;
    const invoke = () => sendEvents.call(target, targetSession, targetNode, events);
    const request = previous ? previous.catch(() => undefined).then(invoke) : invoke();
    requestTail.current = request;
    try {
      await request;
      setMessage(null);
    } catch (error) {
      setMessage(formatUiError(error, tr("g3Instrument.sendFail")));
    } finally {
      if (requestTail.current === request) requestTail.current = null;
    }
  };
  const send = (events: Parameters<NonNullable<UiBackend["midiSendEvents"]>>[2]) =>
    sendTo(backend, sessionId, node.id, events);
  const release = (note: number) => {
    if (!held.current.delete(note)) return;
    void send([{ type: "noteOff", note }]);
  };
  const releaseAll = useEffectEvent((target: UiBackend, targetSession: string, targetNode: string) => {
    if (held.current.size === 0) return;
    held.current.clear();
    void sendTo(target, targetSession, targetNode, [{ type: "allNotesOff" }]);
  });
  // Cleanup names the original target, even if the component is reused for another instrument.
  useEffect(() => {
    const silence = () => releaseAll(backend, sessionId, node.id);
    window.addEventListener("blur", silence);
    return () => {
      window.removeEventListener("blur", silence);
      silence();
    };
  }, [backend, sessionId, node.id, canPlay]);

  const press = (note: number) => {
    if (!canPlay || held.current.has(note)) return;
    held.current.add(note);
    void send([{ type: "noteOn", note, velocity: 100 }]);
  };

  const known =
    midiInput === INSTRUMENT_KEYBOARD_ONLY || midiInput === INSTRUMENT_ALL_DEVICES || devices.includes(midiInput);
  const keys = keyboardKeys();
  const whiteCount = WHITE_OFFSETS.length * OCTAVES + 1;

  return (
    <div className="node-binding-editor instrument-editor" aria-label={tr("g3Instrument.aria")}>
      <div>
        <p className="eyebrow">{tr("g3Instrument.play")}</p>
        <strong>{node.name}</strong>
      </div>
      <label className="field">
        <span>{tr("g3Instrument.notesFrom")}</span>
        <select
          value={midiInput}
          disabled={disabled}
          onFocus={() => void refreshDevices()}
          onChange={(event) => onChange("midiInput", event.target.value)}
        >
          <option value={INSTRUMENT_KEYBOARD_ONLY}>{tr("g3Instrument.kbOnly")}</option>
          <option value={INSTRUMENT_ALL_DEVICES}>{tr("g3Instrument.allMidi")}</option>
          {devices.map((device) => (
            <option key={device} value={device}>
              {device}
            </option>
          ))}
          {!known && <option value={midiInput}>{tr("g3Instrument.notConnected", { name: midiInput })}</option>}
        </select>
      </label>
      <div
        className="instrument-keyboard nodrag nopan"
        role="group"
        aria-label={tr("g3Instrument.kbGroup")}
        style={{ gridTemplateColumns: `repeat(${whiteCount}, 1fr)` }}
      >
        {keys.map((key) => (
          <button
            key={key.note}
            type="button"
            className={`instrument-key${key.black ? " is-black" : ""}`}
            style={{ gridColumn: key.black ? `${key.column + 1} / span 1` : key.column + 1 }}
            aria-label={noteLabel(key.note)}
            disabled={!canPlay}
            onPointerDown={(event) => {
              event.currentTarget.setPointerCapture?.(event.pointerId);
              press(key.note);
            }}
            onPointerUp={() => release(key.note)}
            onPointerCancel={() => release(key.note)}
            onLostPointerCapture={() => release(key.note)}
            onKeyDown={(event) => {
              if (event.key !== "Enter" && event.key !== " ") return;
              event.preventDefault();
              if (!event.repeat) press(key.note);
            }}
            onKeyUp={(event) => {
              if (event.key !== "Enter" && event.key !== " ") return;
              event.preventDefault();
              release(key.note);
            }}
            onBlur={() => release(key.note)}
            onClick={(event) => {
              // Assistive technology can activate a button without key/pointer events.
              if (event.detail === 0 && !held.current.has(key.note)) {
                press(key.note);
                release(key.note);
              }
            }}
          />
        ))}
      </div>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          disabled={!canPlay}
          title={tr("g3Instrument.stopTitle")}
          onClick={() => {
            held.current.clear();
            void send([{ type: "allNotesOff" }]);
          }}
        >
          {tr("g3Instrument.stopAll")}
        </button>
      </div>
      <div className="instrument-status">
        {message ? (
          <PanelMessage message={message} small />
        ) : (
          <small className="muted">{running ? tr("g3Instrument.holdKey") : tr("g3Instrument.pressPlay")}</small>
        )}
      </div>
    </div>
  );
}
