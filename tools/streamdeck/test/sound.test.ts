import { describe, expect, it } from "vitest";
import {
  releasesOnKeyUp,
  slotCaption,
  soundBehavior,
  soundSlot,
  soundSlotChoices,
  triggerParams,
} from "../src/sound-settings.js";

const board = (values: Record<string, unknown>) => (name: string) => values[name];

describe("sound key settings", () => {
  it("falls back to the slot's own behavior for a missing or unknown choice", () => {
    expect(soundBehavior({})).toBe("slot");
    expect(soundBehavior({ behavior: "sideways" as never })).toBe("slot");
    expect(soundBehavior({ behavior: "loop" })).toBe("loop");
  });

  it("accepts only slots one to nine", () => {
    expect(soundSlot({ slot: 9 })).toBe(9);
    for (const slot of [0, 10, 1.5, undefined]) expect(soundSlot({ slot })).toBeNull();
  });
});

describe("sound key request", () => {
  it("sends the key's own behavior as the mode and nothing for the slot's setting", () => {
    expect(triggerParams("s", "n", 3, { behavior: "loop" }, true)).toEqual({
      sessionId: "s",
      nodeId: "n",
      slot: 3,
      action: "press",
      mode: "loop",
    });
    expect(triggerParams("s", "n", 3, {}, false)).toEqual({
      sessionId: "s",
      nodeId: "n",
      slot: 3,
      action: "release",
    });
  });

  it("releases on key up only for play-while-held", () => {
    expect(releasesOnKeyUp({ behavior: "hold" }, "once")).toBe(true);
    expect(releasesOnKeyUp({ behavior: "once" }, "hold")).toBe(false);
    expect(releasesOnKeyUp({}, "hold")).toBe(true);
    expect(releasesOnKeyUp({}, "loop")).toBe(false);
  });
});

describe("sound names", () => {
  it("shows the caption, else the file name without extension, else the slot", () => {
    expect(slotCaption(board({ slot1Caption: " Horn " }), 1)).toBe("Horn");
    expect(slotCaption(board({ slot2FileName: "airhorn.wav" }), 2)).toBe("airhorn");
    expect(slotCaption(board({}), 3)).toBe("Sound 3");
  });

  it("lists nine slots for the panel with names for the filled ones", () => {
    const choices = soundSlotChoices(board({ slot2MediaId: "lib:burp-03", slot2FileName: "Burp 03" }));
    expect(choices).toHaveLength(9);
    expect(choices[1]).toEqual({ slot: 2, filled: true, label: "Burp 03" });
    expect(choices[0]).toEqual({ slot: 1, filled: false, label: "(empty)" });
  });
});
