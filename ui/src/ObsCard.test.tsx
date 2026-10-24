/** @vitest-environment jsdom */
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ObsStatus } from "@audiorouter/contracts";
import { ObsCard, type ObsActions } from "./ObsCard";
import type { ObsCheck } from "./obsUsage";

afterEach(cleanup);

function status(overrides: Partial<ObsStatus> = {}): ObsStatus {
  return {
    installed: true,
    connection: "connected",
    source: "live",
    sceneCollection: "Streaming",
    inputs: [],
    passwordSource: "obsSettings",
    lastChange: { state: "none", changed: 0, restorable: false, error: null },
    ...overrides,
  };
}

function actions() {
  return {
    connect: vi.fn<ObsActions["connect"]>(() => Promise.resolve()),
    disconnect: vi.fn<ObsActions["disconnect"]>(() => Promise.resolve()),
    preview: vi.fn<ObsActions["preview"]>(() =>
      Promise.resolve({ previewId: "obs-preview-1", changes: [], expiresInSeconds: 300 }),
    ),
    apply: vi.fn<ObsActions["apply"]>(() => Promise.resolve()),
    restore: vi.fn<ObsActions["restore"]>(() => Promise.resolve()),
    poll: vi.fn<ObsActions["poll"]>(),
  } satisfies ObsActions;
}

const twoProblems: ObsCheck = {
  appId: "obs",
  state: "check",
  summary: "2 settings to check",
  rows: [],
  view: "connected",
  sources: [
    {
      name: "Mic/Aux",
      device: "Microphone (PD200X Podcast Microphone)",
      hears: ["Voice without effects"],
      muted: false,
      problem: "Records your microphone without AudioRouter's effects.",
      fix: { inputName: "Mic/Aux", deviceId: "cable-a-out" },
      fixLabel: "Mic/Aux: Microphone (PD200X Podcast Microphone) → CABLE-A Output",
    },
    {
      name: "Desktop Audio",
      device: "Default (Speakers (Focusrite USB Audio))",
      hears: ["Voice", "Discord"],
      muted: false,
      problem: "Also plays your voice, so your voice is on the stream twice.",
      fix: { inputName: "Desktop Audio", deviceId: "cable-b-in" },
      fixLabel: "Desktop Audio: Default (Speakers (Focusrite USB Audio)) → CABLE-B Input",
    },
  ],
};

describe("ObsCard", () => {
  it("shows what each source hears, with its problem in words", () => {
    render(<ObsCard check={twoProblems} status={status()} actions={actions()} />);
    expect(screen.getByText("Voice · Discord")).toBeTruthy();
    expect(screen.getByText(/on the stream twice/)).toBeTruthy();
    expect(document.querySelector(".app-state-badge.is-check")?.textContent).toBe("! Check");
  });

  it("changes only the ticked settings, through a preview", async () => {
    const obs = actions();
    render(<ObsCard check={twoProblems} status={status()} actions={obs} />);
    fireEvent.click(screen.getByRole("button", { name: "Fix in OBS" }));
    expect(screen.getByRole("dialog", { name: "Fix in OBS" })).toBeTruthy();
    fireEvent.click(screen.getByRole("checkbox", { name: /Desktop Audio/ }));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Change 1 setting" }));
    });
    expect(obs.preview).toHaveBeenCalledWith([{ inputName: "Mic/Aux", deviceId: "cable-a-out" }]);
    expect(obs.apply).toHaveBeenCalledWith("obs-preview-1");
    expect(obs.poll).toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("keeps the dialog open with the reason when OBS changed since", async () => {
    const obs = actions();
    obs.preview.mockImplementationOnce(() => Promise.reject(new Error("OBS changed since the preview")));
    render(<ObsCard check={twoProblems} status={status()} actions={obs} />);
    fireEvent.click(screen.getByRole("button", { name: "Fix in OBS" }));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Change 2 settings" }));
    });
    expect(screen.getByRole("alert").textContent).toContain("OBS changed since the preview");
    expect(obs.apply).not.toHaveBeenCalled();
  });

  it("offers Restore only when there is something to restore", () => {
    const obs = actions();
    const { rerender } = render(<ObsCard check={twoProblems} status={status()} actions={obs} />);
    const restore = () => screen.getByRole("button", { name: "Restore previous OBS settings" }) as HTMLButtonElement;
    expect(restore().disabled).toBe(true);
    rerender(
      <ObsCard
        check={twoProblems}
        status={status({ lastChange: { state: "applied", changed: 2, restorable: true, error: null } })}
        actions={obs}
      />,
    );
    expect(restore().disabled).toBe(false);
    expect(screen.getByText("Changed 2 OBS settings.")).toBeTruthy();
    fireEvent.click(restore());
    expect(obs.restore).toHaveBeenCalled();
  });

  it("connects with OBS's own password, and asks for one only when OBS refuses", async () => {
    const obs = actions();
    const connect: ObsCheck = { ...twoProblems, state: "unknown", view: "connect", sources: [] };
    const { rerender } = render(<ObsCard check={connect} status={status({ connection: "serverOff" })} actions={obs} />);
    expect(screen.getByText(/Tools › WebSocket Server Settings/)).toBeTruthy();
    expect(screen.queryByLabelText(/Paste the one/)).toBeNull();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Connect to OBS/ }));
    });
    expect(obs.connect).toHaveBeenCalledWith(undefined);

    rerender(<ObsCard check={connect} status={status({ connection: "wrongPassword" })} actions={obs} />);
    const field = screen.getByLabelText(/OBS refused the password/) as HTMLInputElement;
    expect(field.type).toBe("password");
    const button = screen.getByRole("button", { name: /Connect to OBS/ }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    fireEvent.change(field, { target: { value: "pasted-secret" } });
    await act(async () => {
      fireEvent.click(button);
    });
    expect(obs.connect).toHaveBeenLastCalledWith("pasted-secret");
  });
});
