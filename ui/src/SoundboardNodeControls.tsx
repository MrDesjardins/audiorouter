// The Soundboard card on the canvas: one button per assigned sound, plus a row to add one.
import { useContext } from "react";
import type { Node } from "@audiorouter/contracts";
import { SOUNDBOARD_SLOTS, firstFreeSlot, soundboardSlots } from "./soundboard";
import { SoundPlayButton } from "./SoundboardNodeEditor";
import { SoundboardContext, useSoundboardLive } from "./soundboardLive";

/**
 * Fixed row height and a truncated caption keep the card from moving while sounds play
 * (UI-17); the playing state and progress change colour and a transform only.
 */
export function SoundboardNodeControls({ node }: { node: Node }) {
  const live = useSoundboardLive(node.id);
  const controller = useContext(SoundboardContext);
  const slots = soundboardSlots(node);
  const free = firstFreeSlot(node);
  return (
    <div className="soundboard-node nodrag nopan" aria-label={`${node.name} sounds`}>
      {slots.map((slot) => {
        const state = live?.slots.find((item) => item.slot === slot.slot);
        const playing = state?.playing === true;
        return (
          <SoundPlayButton
            key={slot.slot}
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
        );
      })}
      {free !== null && (
        <button
          type="button"
          className="soundboard-button soundboard-add"
          title="Select the tool to add sounds in its properties"
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
