// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { GlobalKeybinds, type KeybindRule } from "./GlobalKeybinds";

const rules: KeybindRule[] = [
  {
    id: "kb-1",
    name: "",
    enabled: true,
    trigger: { keys: "F13", mode: "press", passthrough: true },
    action: { type: "sessionToggle" },
  },
  {
    id: "kb-2",
    name: "",
    enabled: true,
    trigger: { keys: "F14", mode: "hold", passthrough: true },
    action: { type: "privacyMutePushToTalk" },
  },
  {
    id: "kb-3",
    name: "",
    enabled: true,
    trigger: { keys: "F15", mode: "hold", passthrough: true },
    action: { type: "soundboardTrigger", nodeId: "sb-1", slot: 2, mode: null },
  },
];

function fakeInvoke() {
  return vi.fn(async (command: string, args?: Record<string, unknown>) => {
    if (command === "keybinds_list") return rules;
    if (command === "keybinds_save") return args?.rules;
    return undefined;
  });
}

afterEach(() => {
  cleanup();
  delete window.__TAURI_INTERNALS__;
  vi.restoreAllMocks();
});

test("offers every action and shows soundboard fields only for a soundboard rule", async () => {
  window.__TAURI_INTERNALS__ = { invoke: fakeInvoke() } as unknown as typeof window.__TAURI_INTERNALS__;
  render(<GlobalKeybinds snapshot={null} />);

  await screen.findByText("Start or stop the session — F13");
  expect(screen.getByText("Push to talk (unmute while held) — F14")).toBeTruthy();
  expect(screen.getByText("Play soundboard slot: Slot 2 — F15")).toBeTruthy();
  expect(screen.getByText("Unmutes while the key is held and mutes again when released.")).toBeTruthy();

  const actions = screen.getAllByLabelText("Action");
  expect(
    within(actions[0])
      .getAllByRole("option")
      .map((option) => option.textContent),
  ).toEqual([
    "Start or stop the session",
    "Turn privacy mute on or off",
    "Push to talk (unmute while held)",
    "Play soundboard slot",
  ]);
  expect(screen.getAllByLabelText("Slot")).toHaveLength(1);
});

test("a new action sets the trigger mode its release behavior needs", async () => {
  const invoke = fakeInvoke();
  window.__TAURI_INTERNALS__ = { invoke } as unknown as typeof window.__TAURI_INTERNALS__;
  render(<GlobalKeybinds snapshot={null} />);

  await screen.findByText("Start or stop the session — F13");
  const actions = screen.getAllByLabelText("Action");
  fireEvent.change(actions[0], { target: { value: "privacyMutePushToTalk" } });
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "keybinds_save",
      expect.objectContaining({
        rules: expect.arrayContaining([
          expect.objectContaining({ id: "kb-1", trigger: expect.objectContaining({ mode: "hold" }) }),
        ]),
      }),
    ),
  );

  fireEvent.change(actions[1], { target: { value: "sessionToggle" } });
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "keybinds_save",
      expect.objectContaining({
        rules: expect.arrayContaining([
          expect.objectContaining({ id: "kb-2", trigger: expect.objectContaining({ mode: "press" }) }),
        ]),
      }),
    ),
  );
});
