// Properties editor of a Soundboard node: nine sound slots with add, change, delete and play.
import { useContext, useState, type ReactNode } from "react";
import type { Node } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { uploadAudioMedia } from "./audioUpload";
import { PanelMessage } from "./PanelMessage";
import { MAX_USER_SOUND_SECONDS, SoundLibraryPicker } from "./SoundLibraryPicker";
import {
  MAX_CAPTION_LENGTH,
  SOUNDBOARD_MODES,
  SOUNDBOARD_MODE_LABELS,
  SOUNDBOARD_SLOTS,
  firstFreeSlot,
  slotParameter,
  soundboardSlots,
  type SoundboardMode,
  type SoundboardSlot,
} from "./soundboard";
import { SoundboardContext, useSoundboardLive } from "./soundboardLive";

const SLOT_FIELDS = ["MediaId", "FileName", "Caption", "GainDb", "Mode", "Set"] as const;

/** Press-and-hold slots play while the button is down; every other slot plays on click. */
export function SoundPlayButton({
  nodeId,
  slot,
  label,
  mode,
  disabled,
  className,
  children,
}: {
  nodeId: string;
  slot: number;
  label: string;
  mode: SoundboardMode;
  disabled?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const controller = useContext(SoundboardContext);
  const hold = mode === "hold";
  return (
    <button
      type="button"
      className={className}
      aria-label={label}
      disabled={disabled}
      onClick={hold ? undefined : () => controller.trigger(nodeId, slot)}
      onPointerDown={hold ? () => controller.trigger(nodeId, slot, "press") : undefined}
      onPointerUp={hold ? () => controller.trigger(nodeId, slot, "release") : undefined}
      onPointerLeave={
        hold
          ? (event) => {
              if (event.buttons > 0) controller.trigger(nodeId, slot, "release");
            }
          : undefined
      }
    >
      {children}
    </button>
  );
}

export function SoundboardNodeEditor({
  node,
  backend,
  disabled,
  onChange,
  onRemove,
}: {
  node: Node;
  backend: UiBackend;
  disabled: boolean;
  onChange: (name: string, value: boolean | number | string) => void;
  onRemove: (names: string[]) => void;
}) {
  const controller = useContext(SoundboardContext);
  const live = useSoundboardLive(node.id);
  const slots = soundboardSlots(node);
  const free = firstFreeSlot(node);
  // The slot the picker is choosing for; "new" adds to the first free slot.
  const [picking, setPicking] = useState<number | "new" | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState(
    "Add up to nine sounds. Each shows as a button on the tool; press Play here or on the tool, or trigger it from a Stream Deck.",
  );
  const assign = (slot: number, mediaId: string, fileName: string, keepCaption: boolean) => {
    onChange(slotParameter(slot, "MediaId"), mediaId);
    onChange(slotParameter(slot, "FileName"), fileName);
    if (!keepCaption) onChange(slotParameter(slot, "Caption"), "");
  };
  const targetSlot = () => (picking === "new" || picking === null ? free : picking);
  const chooseLibrarySound = (sound: { id: string; title: string }) => {
    const slot = targetSlot();
    if (slot === null) return;
    assign(slot, sound.id, sound.title, picking !== "new");
    setMessage(`${sound.title} is on button ${slot}. Plan and commit the change, then press Play.`);
    setPicking(null);
  };
  const chooseFile = async (file: File) => {
    const slot = targetSlot();
    if (slot === null) return;
    setBusy(true);
    try {
      const media = await uploadAudioMedia(backend, file);
      if (media.durationMs > MAX_USER_SOUND_SECONDS * 1000) {
        setMessage(`${file.name} is longer than ${MAX_USER_SOUND_SECONDS} seconds, so it cannot be used on a Soundboard.`);
        return;
      }
      assign(slot, media.mediaId, media.fileName, picking !== "new");
      setMessage(`${media.fileName} is on button ${slot} (${Math.round(media.durationMs / 1000)} s).`);
      setPicking(null);
    } catch (error) {
      setMessage(formatUiError(error, "The sound could not be imported."));
    } finally {
      setBusy(false);
    }
  };
  const remove = (slot: SoundboardSlot) => {
    onRemove(SLOT_FIELDS.map((field) => slotParameter(slot.slot, field)));
    setMessage(`Button ${slot.slot} (${slot.label}) is removed in the draft.`);
  };
  const playingNow = (slot: number) => live?.slots.find((item) => item.slot === slot)?.playing === true;
  return (
    <div className="node-binding-editor soundboard-editor" aria-label="Soundboard settings">
      <div>
        <p className="eyebrow">Soundboard</p>
        <strong>
          {slots.length} of {SOUNDBOARD_SLOTS} sounds
        </strong>
      </div>
      <ol className="soundboard-slot-list">
        {slots.map((slot) => (
          <li key={slot.slot} className="soundboard-slot">
            <div className="soundboard-slot-head">
              <span className="soundboard-slot-number">{slot.slot}</span>
              <span className="soundboard-slot-file" title={slot.fileName}>
                {slot.fileName || slot.mediaId}
              </span>
              <span className="soundboard-slot-state" role="status">
                {playingNow(slot.slot) ? "Playing" : ""}
              </span>
            </div>
            <label>
              Caption
              <input
                type="text"
                value={slot.caption}
                maxLength={MAX_CAPTION_LENGTH}
                placeholder={slot.label}
                disabled={disabled}
                onChange={(event) => onChange(slotParameter(slot.slot, "Caption"), event.target.value)}
              />
            </label>
            <label>
              Level ({slot.gainDb > 0 ? "+" : ""}
              {slot.gainDb} dB)
              <input
                type="range"
                min={-24}
                max={6}
                step={1}
                value={slot.gainDb}
                disabled={disabled}
                onChange={(event) => onChange(slotParameter(slot.slot, "GainDb"), Number(event.target.value))}
              />
            </label>
            <label>
              When pressed
              <select
                value={slot.mode}
                disabled={disabled}
                onChange={(event) => onChange(slotParameter(slot.slot, "Mode"), event.target.value)}
              >
                {SOUNDBOARD_MODES.map((mode) => (
                  <option key={mode} value={mode}>
                    {SOUNDBOARD_MODE_LABELS[mode]}
                  </option>
                ))}
              </select>
            </label>
            <div className="soundboard-slot-actions">
              <SoundPlayButton
                nodeId={node.id}
                slot={slot.slot}
                mode={slot.mode}
                label={`Play ${slot.label}`}
                className="secondary"
                disabled={!backend.connected}
              >
                Play
              </SoundPlayButton>
              <button type="button" className="secondary" disabled={disabled} onClick={() => setPicking(slot.slot)}>
                Change
              </button>
              <button type="button" className="secondary" disabled={disabled} onClick={() => remove(slot)}>
                Delete
              </button>
            </div>
          </li>
        ))}
      </ol>
      <button type="button" className="primary" disabled={disabled || free === null} onClick={() => setPicking("new")}>
        {free === null ? "All nine slots are used" : "Add sound"}
      </button>
      <button
        type="button"
        className="secondary"
        disabled={!controller.ready || slots.length === 0}
        onClick={() => controller.stop(node.id)}
      >
        Stop all sounds
      </button>
      <div className="soundboard-editor-status">
        <PanelMessage message={message} />
      </div>
      {picking !== null && (
        <SoundLibraryPicker
          backend={backend}
          title={picking === "new" ? "Add a sound" : `Change sound ${picking}`}
          busy={busy}
          onChoose={chooseLibrarySound}
          onChooseFile={(file) => void chooseFile(file)}
          onClose={() => setPicking(null)}
        />
      )}
    </div>
  );
}
