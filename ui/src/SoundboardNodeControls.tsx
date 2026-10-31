// The Soundboard card on the canvas: one button per assigned sound, plus a row to add one.
import { useContext } from "react";
import type { Node } from "@audiorouter/contracts";
import { SOUNDBOARD_SLOTS, firstFreeSlot, soundboardSlots, type SoundboardSlot } from "./soundboard";
import { useSoundDrop } from "./soundboardDrop";
import { SoundPlayButton } from "./SoundboardNodeEditor";
import { SoundboardContext, useSoundboardLive, type SoundboardLive } from "./soundboardLive";

function SoundboardCardButton({ node, slot, live }: { node: Node; slot: SoundboardSlot; live: SoundboardLive | null }) {
  // Dropping a file on a button replaces its sound (SND-19).
  const drop = useSoundDrop({ nodeId: node.id, slot: slot.slot, freeSlot: null, disabled: false });
  const state = live?.slots.find((item) => item.slot === slot.slot);
  const playing = state?.playing === true;
  return (
    <div className={`soundboard-drop${drop.active ? " is-drop-target" : ""}`} {...drop.handlers}>
      <SoundPlayButton
        nodeId={node.id}
        slot={slot.slot}
        mode={slot.mode}
        label={`Play ${slot.label}`}
        disabled={!node.enabled}
        className={`soundboard-button${playing ? " is-playing" : ""}`}
      >
        <span
          className="soundboard-button-progress"
          style={{ transform: `scaleX(${playing ? (state?.progress ?? 0) : 0})` }}
        />
        <span className="soundboard-button-caption" title={`${slot.label} · ${slot.fileName}`}>
          {slot.label}
        </span>
      </SoundPlayButton>
    </div>
  );
}

/**
 * Fixed row height and a truncated caption keep the card from moving while sounds play
 * (UI-17); the playing state and progress change colour and a transform only. A file
 * dropped on the card fills the first free slot; dropped on a button it replaces that sound.
 */
export function SoundboardNodeControls({ node }: { node: Node }) {
  const live = useSoundboardLive(node.id);
  const controller = useContext(SoundboardContext);
  const slots = soundboardSlots(node);
  const free = firstFreeSlot(node);
  const drop = useSoundDrop({ nodeId: node.id, slot: null, freeSlot: free, disabled: false });
  return (
    <div
      className={`soundboard-node nodrag nopan${drop.active ? " is-drop-target" : ""}`}
      aria-label={`${node.name} sounds`}
      {...drop.handlers}
    >
      {slots.map((slot) => (
        <SoundboardCardButton key={slot.slot} node={node} slot={slot} live={live} />
      ))}
      {free !== null && (
        <button
          type="button"
          className="soundboard-button soundboard-add"
          title="Select the tool to add sounds in its properties, or drop a WAV or MP3 file here"
        >
          <span className="soundboard-button-caption">
            {slots.length === 0 ? "Add sound in properties" : `+ Add sound (${slots.length}/${SOUNDBOARD_SLOTS})`}
          </span>
        </button>
      )}
      <button
        type="button"
        className="soundboard-stop"
        aria-label={`Stop all sounds on ${node.name}`}
        disabled={!controller.ready || slots.length === 0}
        onClick={() => controller.stop(node.id)}
      >
        Stop all
      </button>
    </div>
  );
}
