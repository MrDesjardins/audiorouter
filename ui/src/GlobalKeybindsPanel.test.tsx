// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { GlobalKeybinds, type KeybindRule } from "./GlobalKeybinds";
import { AppDialogHost } from "./AppDialog";

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

test("Add shows name, action and shortcut together and saves only the completed draft", async () => {
  const invoke = fakeInvoke();
  window.__TAURI_INTERNALS__ = { invoke } as unknown as typeof window.__TAURI_INTERNALS__;
  render(<GlobalKeybinds snapshot={null} />);
  await screen.findByText("Start or stop the session — F13");
  fireEvent.click(screen.getByRole("button", { name: "Add keybinding" }));
  const draft = screen.getByText("New keybinding").closest("details")!;
  expect(draft.open).toBe(true);
  expect(within(draft).getByLabelText("Name")).toBeTruthy();
  expect(within(draft).getByLabelText("Action")).toBeTruthy();
  const save = within(draft).getByRole("button", { name: "Save keybinding" }) as HTMLButtonElement;
  expect(save.disabled).toBe(true);
  fireEvent.change(within(draft).getByLabelText("Name"), { target: { value: "My microphone" } });
  fireEvent.change(within(draft).getByLabelText("Action"), { target: { value: "privacyMutePushToTalk" } });
  const trigger = within(draft).getByRole("textbox", { name: /Global trigger/ });
  fireEvent.focus(trigger);
  fireEvent.keyDown(trigger, { key: "F9", code: "F9" });
  expect(invoke.mock.calls.filter(([command]) => command === "keybinds_save")).toHaveLength(0);
  fireEvent.click(save);
  await waitFor(() => expect(screen.queryByText("New keybinding")).toBeNull());
  expect(invoke.mock.calls.filter(([command]) => command === "keybinds_save")).toHaveLength(1);
  expect(invoke).toHaveBeenCalledWith("keybinds_save", {
    rules: [
      ...rules,
      expect.objectContaining({
        name: "My microphone",
        trigger: { keys: "F9", mode: "hold", passthrough: true },
        action: { type: "privacyMutePushToTalk" },
      }),
    ],
  });
  expect(screen.getByText("F9 · Push to talk (unmute while held)")).toBeTruthy();
});

test("cancel discards a draft without saving and restores shortcut handling", async () => {
  const invoke = fakeInvoke();
  window.__TAURI_INTERNALS__ = { invoke } as unknown as typeof window.__TAURI_INTERNALS__;
  render(<GlobalKeybinds snapshot={null} />);
  await screen.findByText("Start or stop the session — F13");
  fireEvent.click(screen.getByRole("button", { name: "Add keybinding" }));
  fireEvent.focus(screen.getByRole("textbox", { name: "Global trigger for keybind" }));
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.queryByText("New keybinding")).toBeNull();
  expect(invoke.mock.calls.filter(([command]) => command === "keybinds_save")).toHaveLength(0);
  expect(invoke).toHaveBeenLastCalledWith("keybinds_set_suspended", { suspended: false });
});

test("a rejected save preserves the completed draft for retry", async () => {
  const invoke = fakeInvoke();
  invoke.mockImplementation(async (command, args) => {
    if (command === "keybinds_list") return rules;
    if (command === "keybinds_save") throw "Could not register shortcut";
    return args?.rules;
  });
  window.__TAURI_INTERNALS__ = { invoke } as unknown as typeof window.__TAURI_INTERNALS__;
  render(<GlobalKeybinds snapshot={null} />);
  await screen.findByText("Start or stop the session — F13");
  fireEvent.click(screen.getByRole("button", { name: "Add keybinding" }));
  const trigger = screen.getByRole("textbox", { name: "Global trigger for keybind" });
  fireEvent.focus(trigger);
  fireEvent.keyDown(trigger, { key: "F9", code: "F9" });
  fireEvent.click(screen.getByRole("button", { name: "Save keybinding" }));
  await screen.findByText("Could not register shortcut");
  expect(screen.getByText("New keybinding")).toBeTruthy();
  expect((screen.getByRole("textbox", { name: "Global trigger for keybind" }) as HTMLInputElement).value).toBe("F9");
});

test("a soundboard draft explains the missing tool and cannot save an empty target", async () => {
  const invoke = fakeInvoke();
  window.__TAURI_INTERNALS__ = { invoke } as unknown as typeof window.__TAURI_INTERNALS__;
  render(<GlobalKeybinds snapshot={null} />);
  await screen.findByText("Start or stop the session — F13");
  fireEvent.click(screen.getByRole("button", { name: "Add keybinding" }));
  const draft = screen.getByText("New keybinding").closest("details")!;
  fireEvent.change(within(draft).getByLabelText("Action"), { target: { value: "soundboardTrigger" } });
  expect(
    within(draft).getByText("Add a Soundboard tool to this session first, or choose another action."),
  ).toBeTruthy();
  const trigger = within(draft).getByRole("textbox", { name: /Global trigger/ });
  fireEvent.focus(trigger);
  fireEvent.keyDown(trigger, { key: "F9", code: "F9" });
  expect((within(draft).getByRole("button", { name: "Save keybinding" }) as HTMLButtonElement).disabled).toBe(true);
  expect(invoke.mock.calls.filter(([command]) => command === "keybinds_save")).toHaveLength(0);
});

test("single delete names the binding, cancels safely, and removes only that binding", async () => {
  const invoke = fakeInvoke();
  window.__TAURI_INTERNALS__ = { invoke } as unknown as typeof window.__TAURI_INTERNALS__;
  render(
    <>
      <GlobalKeybinds snapshot={null} />
      <AppDialogHost />
    </>,
  );
  const title = await screen.findByText("Start or stop the session — F13");
  title.closest("details")!.open = true;
  fireEvent.click(screen.getByRole("button", { name: "Delete keybinding F13" }));
  let dialog = screen.getByRole("dialog", { name: "Delete “F13”?" });
  expect(within(dialog).getByText(/removes only “F13”/)).toBeTruthy();
  fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(invoke.mock.calls.filter(([command]) => command === "keybinds_save")).toHaveLength(0);
  fireEvent.click(screen.getByRole("button", { name: "Delete keybinding F13" }));
  dialog = screen.getByRole("dialog");
  fireEvent.click(within(dialog).getByRole("button", { name: "Delete keybinding" }));
  await waitFor(() => expect(screen.queryByText("Start or stop the session — F13")).toBeNull());
  expect(invoke).toHaveBeenCalledWith("keybinds_save", { rules: rules.slice(1) });
  expect(screen.getByText("Push to talk (unmute while held) — F14")).toBeTruthy();
});

test("bulk delete uses selected IDs and a failed save retains every binding and selection", async () => {
  const invoke = fakeInvoke();
  let rejectSave = true;
  let rejectPendingSave: ((reason: string) => void) | undefined;
  invoke.mockImplementation(async (command, args) => {
    if (command === "keybinds_list") return rules;
    if (command === "keybinds_save" && rejectSave)
      return await new Promise<unknown>((_resolve, reject) => {
        rejectPendingSave = reject;
      });
    return args?.rules;
  });
  window.__TAURI_INTERNALS__ = { invoke } as unknown as typeof window.__TAURI_INTERNALS__;
  render(
    <>
      <GlobalKeybinds snapshot={null} />
      <AppDialogHost />
    </>,
  );
  await screen.findByText("Start or stop the session — F13");
  fireEvent.click(screen.getByRole("button", { name: "Manage keybindings" }));
  fireEvent.keyDown(screen.getByRole("checkbox", { name: "Select F13" }), { key: " " });
  fireEvent.keyDown(screen.getByRole("checkbox", { name: "Select F15" }), { key: " " });
  fireEvent.click(screen.getByRole("button", { name: "Delete selected (2)" }));
  fireEvent.click(
    within(screen.getByRole("dialog", { name: "Delete 2 keybindings?" })).getByRole("button", {
      name: "Delete 2 keybindings",
    }),
  );
  await waitFor(() => expect(screen.getAllByRole("checkbox")).toHaveLength(1));
  rejectPendingSave!("Could not save global keybindings.");
  await screen.findByText("Could not save global keybindings.");
  expect((screen.getByRole("checkbox", { name: "Select F13" }) as HTMLInputElement).checked).toBe(true);
  expect(screen.getAllByRole("checkbox")).toHaveLength(3);
  rejectSave = false;
  fireEvent.click(screen.getByRole("button", { name: "Delete selected (2)" }));
  fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Delete 2 keybindings" }));
  await waitFor(() => expect(screen.getAllByRole("checkbox")).toHaveLength(1));
  expect(invoke).toHaveBeenLastCalledWith("keybinds_save", { rules: [rules[1]] });
});
