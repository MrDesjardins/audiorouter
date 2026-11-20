// Properties editor of a Soundboard node: nine sound slots with add, change, delete and play.
import { useContext, useEffect, useEffectEvent, useRef, useState, type ReactNode } from "react";
import type { Node } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { PanelMessage } from "./PanelMessage";
import { SliderField } from "./Slider";
import { SoundLibraryPicker } from "./SoundLibraryPicker";
import {
  MAX_CAPTION_LENGTH,
  SOUNDBOARD_MODES,
  SOUNDBOARD_SLOTS,
  firstFreeSlot,
  setLabel,
  slotParameter,
  soundboardSlots,
  type SoundboardMode,
  type SoundboardSlot,
} from "./soundboard";
import { useSoundDrop } from "./soundboardDrop";
import { SoundboardContext, useSoundboardLive } from "./soundboardLive";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

const MODE_MESSAGES = {
  once: "g2.sbModeOnce",
  loop: "g2.sbModeLoop",
  toggle: "g2.sbModeToggle",
  hold: "g2.sbModeHold",
} as const satisfies Record<SoundboardMode, MessageKey>;

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
  const heldKey = useRef<string | null>(null);
  const heldPointer = useRef(false);
  const release = () => {
    if (heldKey.current === null && !heldPointer.current) return;
    heldKey.current = null;
    heldPointer.current = false;
    controller.trigger(nodeId, slot, "release");
  };
  const releaseOnUnmount = useEffectEvent(release);
  useEffect(() => () => releaseOnUnmount(), []);
  return (
    <button
      type="button"
      className={className}
      aria-label={label}
      disabled={disabled}
      onClick={hold ? undefined : () => controller.trigger(nodeId, slot)}
      onKeyDown={
        hold
          ? (event) => {
              if (event.key !== " " && event.key !== "Enter") return;
              event.preventDefault();
              event.stopPropagation();
              if (event.repeat || heldKey.current !== null) return;
              heldKey.current = event.key;
              controller.trigger(nodeId, slot, "press");
            }
          : undefined
      }
      onKeyUp={(event) => {
        if (heldKey.current !== event.key) return;
        event.preventDefault();
        event.stopPropagation();
        release();
      }}
      onBlur={release}
      onPointerCancel={release}
      onPointerDown={
        hold
          ? (event) => {
              if (event.button === 0) {
                heldPointer.current = true;
                controller.trigger(nodeId, slot, "press");
              }
            }
          : undefined
      }
      onPointerUp={(event) => {
        if (event.button === 0) release();
      }}
      onPointerLeave={(event) => {
        if ((event.buttons & 1) !== 0) release();
      }}
    >
      {children}
    </button>
  );
}

function SoundboardSlotRow({
  node,
  slot,
  backend,
  disabled,
  playing,
  onChange,
  onPick,
  onDelete,
  onMessage,
}: {
  node: Node;
  slot: SoundboardSlot;
  backend: UiBackend;
  disabled: boolean;
  playing: boolean;
  onChange: (name: string, value: boolean | number | string) => void;
  onPick: () => void;
  onDelete: () => void;
  onMessage: (message: string) => void;
}) {
  const tr = useMessages();
  const drop = useSoundDrop({ nodeId: node.id, slot: slot.slot, freeSlot: null, disabled, onMessage });
  return (
    <li className={`soundboard-slot${drop.active ? " is-drop-target" : ""}`} {...drop.handlers}>
      <div className="soundboard-slot-head">
        <span className="soundboard-slot-number">{slot.slot}</span>
        <span className="soundboard-slot-file" title={slot.fileName}>
          {slot.fileName || (slot.set === null ? slot.mediaId : setLabel(slot.set))}
        </span>
        <span className="soundboard-slot-state" role="status">
          {playing ? tr("g2.sbPlaying") : ""}
        </span>
      </div>
      <label>
        {tr("g2.sbCaption")}
        <input
          type="text"
          value={slot.caption}
          maxLength={MAX_CAPTION_LENGTH}
          placeholder={slot.label}
          disabled={disabled}
          onChange={(event) => onChange(slotParameter(slot.slot, "Caption"), event.target.value)}
        />
      </label>
      <SliderField
        label={tr("g2.sbLevel")}
        name={tr("g2.sbLevelName", { n: slot.slot })}
        value={slot.gainDb}
        min={-24}
        max={6}
        step={1}
        unit="dB"
        defaultValue={0}
        disabled={disabled}
        onChange={(value) => onChange(slotParameter(slot.slot, "GainDb"), value)}
      />
      <label>
        {tr("g2.sbWhenPressed")}
        <select
          value={slot.mode}
          disabled={disabled}
          onChange={(event) => onChange(slotParameter(slot.slot, "Mode"), event.target.value)}
        >
          {SOUNDBOARD_MODES.map((mode) => (
            <option key={mode} value={mode}>
              {tr(MODE_MESSAGES[mode])}
            </option>
          ))}
        </select>
      </label>
      <div className="soundboard-slot-actions">
        <SoundPlayButton
          nodeId={node.id}
          slot={slot.slot}
          mode={slot.mode}
          label={tr("g2.sbPlay", { label: slot.label })}
          className="secondary"
          disabled={!backend.connected}
        >
          {tr("g2.sbPlayBtn")}
        </SoundPlayButton>
        <button
          type="button"
          className="secondary"
          aria-label={tr("g2.sbChangeAria", { n: slot.slot })}
          disabled={disabled}
          onClick={onPick}
        >
          {tr("g2.sbChange")}
        </button>
        <button
          type="button"
          className="secondary"
          aria-label={tr("g2.sbDeleteAria", { n: slot.slot })}
          disabled={disabled}
          onClick={onDelete}
        >
          {tr("g2.sbDelete")}
        </button>
      </div>
    </li>
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
  const tr = useMessages();
  const controller = useContext(SoundboardContext);
  const live = useSoundboardLive(node.id);
  const slots = soundboardSlots(node);
  const free = firstFreeSlot(node);
  // The slot the picker is choosing for; "new" adds to the first free slot.
  const [picking, setPicking] = useState<number | "new" | null>(null);
  const [message, setMessage] = useState(() => tr("g2.sbIntro"));
  const drop = useSoundDrop({ nodeId: node.id, slot: null, freeSlot: free, disabled, onMessage: setMessage });
  const targetSlot = () => (picking === "new" || picking === null ? free : picking);
  const chooseLibrarySound = (sound: { id: string; title: string }) => {
    const slot = targetSlot();
    if (slot === null) return;
    controller.assign(node.id, slot, { mediaId: sound.id, fileName: sound.title }, picking !== "new");
    setMessage(tr("g2.sbOnButton", { title: sound.title, slot }));
    setPicking(null);
  };
  const chooseFile = async (file: File) => {
    const slot = targetSlot();
    if (slot === null) return;
    const keepCaption = picking !== "new";
    const result = await controller.importFile(node.id, slot, file, keepCaption);
    setMessage(result.message);
    if (result.ok) setPicking(null);
  };
  const remove = (slot: SoundboardSlot) => {
    onRemove(SLOT_FIELDS.map((field) => slotParameter(slot.slot, field)));
    setMessage(tr("g2.sbRemoved", { slot: slot.slot, label: slot.label }));
  };
  const playingNow = (slot: number) => live?.slots.find((item) => item.slot === slot)?.playing === true;
  return (
    <div
      className={`node-binding-editor soundboard-editor${drop.active ? " is-drop-target" : ""}`}
      aria-label={tr("g2.sbSettingsAria")}
      {...drop.handlers}
    >
      <div>
        <p className="eyebrow">{tr("g2.sbEyebrow")}</p>
        <strong>{tr("g2.sbCount", { n: slots.length, max: SOUNDBOARD_SLOTS })}</strong>
      </div>
      <ol className="soundboard-slot-list">
        {slots.map((slot) => (
          <SoundboardSlotRow
            key={slot.slot}
            node={node}
            slot={slot}
            backend={backend}
            disabled={disabled}
            playing={playingNow(slot.slot)}
            onChange={onChange}
            onPick={() => setPicking(slot.slot)}
            onDelete={() => remove(slot)}
            onMessage={setMessage}
          />
        ))}
      </ol>
      <button type="button" className="primary" disabled={disabled || free === null} onClick={() => setPicking("new")}>
        {free === null ? tr("g2.sbAllUsed") : tr("g2.sbAdd")}
      </button>
      <button
        type="button"
        className="secondary"
        disabled={!controller.ready || slots.length === 0}
        onClick={() => controller.stop(node.id)}
      >
        {tr("g2.sbStopAll")}
      </button>
      <div className="soundboard-editor-status">
        <PanelMessage message={message} />
      </div>
      {picking !== null && (
        <SoundLibraryPicker
          backend={backend}
          title={picking === "new" ? tr("g2.sbPickerAdd") : tr("g2.sbPickerChange", { n: picking })}
          busy={controller.importing}
          onChoose={chooseLibrarySound}
          onChooseFile={(file) => void chooseFile(file)}
          onClose={() => setPicking(null)}
        />
      )}
    </div>
  );
}
