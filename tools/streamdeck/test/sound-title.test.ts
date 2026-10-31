import { expect, it, vi } from "vitest";
import { SoundAction } from "../src/actions/sound.js";
import type { AudioRouterStore } from "../src/store.js";

it("updates the Sound key through title, settings and board changes in priority order", () => {
  const values: Record<string, unknown> = { slot1MediaId: "lib:horn", slot1Caption: "Horn", slot1FileName: "airhorn.wav" };
  let refresh = () => {};
  const store = {
    state: "online", summary: { sessionId: "s" },
    subscribe: (callback: () => void) => { refresh = callback; },
    node: () => ({ id: "board", name: "Board" }),
    setting: (_node: unknown, field: string) => values[field],
  } as unknown as AudioRouterStore;
  const action = new SoundAction(store);
  const key = { id: "key", isKey: () => true, setImage: vi.fn().mockResolvedValue(undefined) };
  const svg = () => decodeURIComponent(key.setImage.mock.lastCall![0].split(",")[1]);
  action.onWillAppear({ action: key, payload: { settings: { nodeId: "board", slot: 1 } } } as unknown as Parameters<typeof action.onWillAppear>[0]);
  expect(svg()).toContain(">Horn<");
  expect(svg()).not.toContain("SOUND 1");
  const title = (text: string, shown: boolean) => action.onTitleParametersDidChange({ action: key, payload: { title: text, titleParameters: { showTitle: shown } } } as unknown as Parameters<typeof action.onTitleParametersDidChange>[0]);
  const label = (text: string) => action.onDidReceiveSettings({ action: key, payload: { settings: { nodeId: "board", slot: 1, label: text } } } as unknown as Parameters<typeof action.onDidReceiveSettings>[0]);
  label("My horn");
  expect(svg()).toContain(">My horn<");
  title("Custom", true);
  expect(svg()).not.toContain("<text");
  title("Custom", false);
  expect(svg()).toContain(">Custom<");
  expect(svg()).not.toContain("SOUND 1");
  title("  ", true);
  expect(svg()).toContain(">My horn<");
  label("");
  expect(svg()).toContain(">Horn<");
  values.slot1Caption = "";
  refresh();
  expect(svg()).toContain(">airhorn<");
  values.slot1FileName = "";
  refresh();
  expect(svg()).toContain(">SOUND 1<");
  values.slot1Caption = "Clap";
  refresh();
  expect(svg()).toContain(">Clap<");
  expect(svg()).not.toContain("SOUND 1");
});
