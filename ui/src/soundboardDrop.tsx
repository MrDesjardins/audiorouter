// Dropping a sound file on a Soundboard slot, on the card or in Properties (SND-19).
import { useContext, useState, type DragEvent } from "react";
import { droppedSoundFile, isFileDrag } from "./soundImport";
import { SoundboardContext } from "./soundboardLive";

/**
 * Drop-target handlers for slot `slot` (replaces its sound and keeps the caption) or, when
 * `slot` is null, for `freeSlot` (the first free one). Only file drags are taken, so a tool
 * dragged from the library still reaches the canvas. Everything is checked before an upload
 * starts. `active` drives an outline only, so the target never changes size (UI-17).
 */
export function useSoundDrop({
  nodeId,
  slot,
  freeSlot,
  disabled,
  onMessage,
}: {
  nodeId: string;
  slot: number | null;
  freeSlot: number | null;
  disabled: boolean;
  onMessage?: (message: string) => void;
}) {
  const controller = useContext(SoundboardContext);
  const [active, setActive] = useState(false);
  const target = slot ?? freeSlot;
  const accepts = (event: DragEvent) => !disabled && isFileDrag(event.dataTransfer);
  const take = (event: DragEvent) => {
    event.preventDefault();
    event.stopPropagation();
  };
  return {
    active,
    handlers: {
      onDragEnter: (event: DragEvent) => {
        if (!accepts(event)) return;
        take(event);
        setActive(true);
      },
      onDragOver: (event: DragEvent) => {
        if (!accepts(event)) return;
        take(event);
        event.dataTransfer.dropEffect = target === null || controller.importing ? "none" : "copy";
        if (!active) setActive(true);
      },
      onDragLeave: (event: DragEvent) => {
        if (event.relatedTarget instanceof globalThis.Node && event.currentTarget.contains(event.relatedTarget)) return;
        setActive(false);
      },
      onDrop: (event: DragEvent) => {
        if (!accepts(event)) return;
        take(event);
        setActive(false);
        const report = onMessage ?? controller.announce;
        if (target === null) {
          report("All nine buttons have a sound. Drop the file on a button to replace its sound.");
          return;
        }
        const dropped = droppedSoundFile(event.dataTransfer);
        if ("problem" in dropped) {
          report(dropped.problem);
          return;
        }
        void controller
          .importFile(nodeId, target, dropped.file, slot !== null)
          .then((result) => report(result.message));
      },
    },
  };
}
