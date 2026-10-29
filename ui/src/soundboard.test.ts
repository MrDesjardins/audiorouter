import { describe, expect, it } from "vitest";
import type { Node } from "@audiorouter/contracts";
import { firstFreeSlot, isSoundboardParameter, slotParameter, soundboardSlots } from "./soundboard";

function board(parameters: Node["parameters"]): Node {
  return {
    id: "board",
    kind: "soundboard",
    typeVersion: 1,
    name: "Soundboard",
    enabled: true,
    bypass: false,
    parameters,
    ports: [{ name: "out", direction: "output", channels: 2 }],
  } as Node;
}

describe("soundboard slots", () => {
  it("lists only slots that have a sound, in order", () => {
    const slots = soundboardSlots(board({ slot3MediaId: "lib:a", slot1MediaId: "m1", slot2MediaId: "" }));
    expect(slots.map((slot) => slot.slot)).toEqual([1, 3]);
  });

  it("labels a slot with its caption, else the file name without extension", () => {
    const [plain, captioned] = soundboardSlots(
      board({
        slot1MediaId: "m1",
        slot1FileName: "airhorn.wav",
        slot2MediaId: "m2",
        slot2FileName: "airhorn.wav",
        slot2Caption: "Horn",
      }),
    );
    expect(plain?.label).toBe("airhorn");
    expect(captioned?.label).toBe("Horn");
  });

  it("falls back to safe defaults for missing or invalid settings", () => {
    const [slot] = soundboardSlots(board({ slot1MediaId: "m1", slot1Mode: "bogus", slot1GainDb: "loud" }));
    expect(slot).toMatchObject({ mode: "once", gainDb: 0, label: "Sound 1" });
  });

  it("finds the first free slot and reports a full board", () => {
    expect(firstFreeSlot(board({ slot1MediaId: "a", slot2MediaId: "b", slot4MediaId: "c" }))).toBe(3);
    const full = Object.fromEntries(Array.from({ length: 9 }, (_, index) => [`slot${index + 1}MediaId`, "x"]));
    expect(firstFreeSlot(board(full))).toBeNull();
  });

  it("recognises only slot parameters one to nine", () => {
    expect(isSoundboardParameter(slotParameter(9, "GainDb"))).toBe(true);
    for (const name of ["slot0MediaId", "slot10MediaId", "slot1Volume", "mediaId"]) {
      expect(isSoundboardParameter(name)).toBe(false);
    }
  });
});
