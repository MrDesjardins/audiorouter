/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { DiagnosticsSnapshot, Node } from "@audiorouter/contracts";
import { PluginEditorControls, PluginNodeInspector, PluginRecoveryControls } from "./PluginPanels";

afterEach(cleanup);

it("prepares an update as a draft and makes a state reset explicit", async () => {
  const node = plugin();
  const parameters = { ...node.parameters, fingerprint: "b".repeat(64) };
  const preparePluginUpdate = vi.fn().mockResolvedValue({
    sessionId: "route",
    nodeId: node.id,
    parameters,
    previousFingerprint: node.parameters.fingerprint,
    statePreserved: true,
  });
  const onPrepared = vi.fn().mockReturnValue(true);
  render(
    <PluginRecoveryControls
      node={node}
      backend={{ preparePluginUpdate }}
      sessionId="route"
      running={false}
      onPrepared={onPrepared}
    />,
  );
  expect(preparePluginUpdate).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Use scanned version" }));
  await waitFor(() => expect(onPrepared).toHaveBeenCalledWith(node, parameters));
  expect(preparePluginUpdate).toHaveBeenCalledWith(node.id, "route", false);
  fireEvent.click(screen.getByRole("button", { name: "Start from defaults" }));
  await waitFor(() => expect(preparePluginUpdate).toHaveBeenLastCalledWith(node.id, "route", true));
});

it("does not prepare recovery while the route is running or the tool has unsaved settings", () => {
  const preparePluginUpdate = vi.fn();
  const view = render(
    <PluginRecoveryControls
      node={plugin()}
      backend={{ preparePluginUpdate }}
      sessionId="route"
      running
      onPrepared={() => true}
    />,
  );
  expect((screen.getByRole("button", { name: "Use scanned version" }) as HTMLButtonElement).disabled).toBe(true);
  view.rerender(
    <PluginRecoveryControls
      node={plugin()}
      backend={{ preparePluginUpdate }}
      sessionId="route"
      running={false}
      disabledReason="Save or discard changes first."
      onPrepared={() => true}
    />,
  );
  expect((screen.getByRole("button", { name: "Start from defaults" }) as HTMLButtonElement).disabled).toBe(true);
  expect(preparePluginUpdate).not.toHaveBeenCalled();
});

const plugin = (parameters: Node["parameters"] = {}): Node => ({
  id: "plugin-1",
  kind: "plugin",
  typeVersion: 1,
  name: "UA 1176",
  enabled: true,
  bypass: false,
  parameters: { path: "C:\\VST3\\uaudio_ua_1176.vst3", format: "vst3", fingerprint: "a".repeat(64), ...parameters },
  ports: [],
});

const snapshot = (plugin: Record<string, unknown>) =>
  ({
    nodeTelemetry: [{ nodeId: "plugin-1", kind: "plugin", meter: null, processor: null, plugin }],
  }) as unknown as DiagnosticsSnapshot;

/** The laid-out status block whose heading matches. */
const variant = (heading: RegExp): HTMLElement => screen.getByText(heading).parentElement!;

describe("PluginNodeInspector unchanged-audio guard", () => {
  it("removes the allow switch's key when it is turned off, so older builds still load the route", () => {
    const onChange = vi.fn();
    const onRemove = vi.fn();
    const { rerender } = render(
      <PluginNodeInspector
        node={plugin()}
        disabled={false}
        onUnload={() => {}}
        snapshot={null}
        onChange={onChange}
        onRemove={onRemove}
      />,
    );
    const toggle = screen.getByLabelText<HTMLInputElement>("Allow unchanged audio");
    expect(toggle.checked).toBe(false);
    expect(document.getElementById(toggle.getAttribute("aria-describedby") ?? "")?.textContent).toBe("Off");
    expect(variant(/^On$/).style.visibility).toBe("visible");
    fireEvent.click(toggle);
    expect(onChange).toHaveBeenCalledWith("passThroughAllowed", true);
    rerender(
      <PluginNodeInspector
        node={plugin({ passThroughAllowed: true })}
        disabled={false}
        onUnload={() => {}}
        snapshot={null}
        onChange={onChange}
        onRemove={onRemove}
      />,
    );
    expect(variant(/^Off for this plugin$/).style.visibility).toBe("visible");
    fireEvent.click(screen.getByLabelText("Allow unchanged audio"));
    expect(onRemove).toHaveBeenCalledWith(["passThroughAllowed"]);
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("sends plugins.resume for this node on Retry and says when the plugin is still muted", async () => {
    const resumePlugin = vi.fn().mockResolvedValue({ sessionId: "s", nodeId: "plugin-1", resumed: false });
    render(
      <PluginNodeInspector
        node={plugin()}
        disabled={false}
        onUnload={() => {}}
        snapshot={snapshot({ state: "running", failureCount: 0, unchangedAudio: true, silencedUnchanged: true })}
        backend={{ resumePlugin }}
        sessionId="s"
      />,
    );
    const guard = screen.getByRole("alert", { name: "Voice protection" });
    fireEvent.click(within(guard).getByRole("button", { name: "Retry" }));
    expect(resumePlugin).toHaveBeenCalledWith("s", "plugin-1");
    // Every status is always laid out (UI-17); only its visibility changes.
    await waitFor(() => expect(variant(/^Still muted$/).style.visibility).toBe("visible"));
    expect(screen.getByText(/still passes your voice through/).style.visibility).toBe("visible");
    expect(screen.getByText(/probably not licensed/).style.visibility).toBe("hidden");
  });

  it("offers both ways out inside the muted card: Retry and the allow switch", () => {
    render(
      <PluginNodeInspector
        node={plugin()}
        disabled={false}
        onUnload={() => {}}
        snapshot={snapshot({ state: "running", failureCount: 0, unchangedAudio: true, silencedUnchanged: true })}
        backend={{ resumePlugin: vi.fn() }}
        sessionId="s"
        onChange={vi.fn()}
      />,
    );
    const guard = screen.getByRole("alert", { name: "Voice protection" });
    expect(within(guard).getByLabelText<HTMLInputElement>("Allow unchanged audio").disabled).toBe(false);
    expect(within(guard).getByText("Meant to leave your voice unchanged?")).toBeTruthy();
    // The switch is numbered as step 2 only while the plugin is muted.
    expect(within(guard).getByText("2").style.visibility).toBe("visible");
    // The worker itself is fine, so its health box is not a second alert.
    expect(screen.queryByRole("alert", { name: "Plugin worker health" })).toBeNull();
  });

  it("says why the worker failed (owner report 2026-11-12)", () => {
    render(
      <PluginNodeInspector
        node={plugin()}
        disabled={false}
        onUnload={() => {}}
        snapshot={snapshot({
          state: "failed",
          failureCount: 1,
          failureReason: "processing audio failed: Timeout",
        })}
      />,
    );
    const health = screen.getByRole("alert", { name: "Plugin worker health" });
    expect(within(health).getByText("Why: processing audio failed: Timeout")).toBeTruthy();
  });

  it("keeps Retry out of reach while the plugin is not muted", () => {
    const { container } = render(
      <PluginNodeInspector
        node={plugin()}
        disabled={false}
        onUnload={() => {}}
        snapshot={snapshot({ state: "running", failureCount: 0, unchangedAudio: true })}
        backend={{ resumePlugin: vi.fn() }}
        sessionId="s"
      />,
    );
    // Laid out for UI-17 but hidden from assistive tech, the tab order and clicks.
    expect(screen.queryByRole("button", { name: "Retry" })).toBeNull();
    expect(screen.queryByRole("alert", { name: "Voice protection" })).toBeNull();
    const retry = container.querySelector<HTMLButtonElement>(".plugin-guard-step button")!;
    expect(retry.disabled).toBe(true);
    expect(retry.tabIndex).toBe(-1);
    expect(variant(/^Audio passes through unchanged$/).style.visibility).toBe("visible");
    // A hidden status must not show through: an explicit "visible" child overrides a hidden parent.
    expect(screen.getByText(/probably not licensed/).style.visibility).toBe("hidden");
  });
});

// Owner report (issue #136): "I want to open and configure the plugin before
// I press Play". The button used to be disabled until the route ran.
describe("PluginEditorControls before Play", () => {
  it("lets the plugin window open while the route is stopped, in the desktop app", async () => {
    const invoke = vi.fn().mockResolvedValue({ error: null });
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = { invoke };
    try {
      render(
        <PluginEditorControls
          node={plugin()}
          backend={{} as never}
          sessionId="route"
          running={false}
          onStateSaved={() => undefined}
        />,
      );
      const open = screen.getByRole("button", { name: "Open plugin window" }) as HTMLButtonElement;
      expect(open.disabled).toBe(false);
      expect(open.title).not.toMatch(/Press Play first/);
      fireEvent.click(open);
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith("open_plugin_editor", {
          sessionId: "route",
          nodeId: plugin().id,
          title: plugin().name,
        }),
      );
    } finally {
      delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    }
  });

  it("lists and selects VST2 programs and imports a preset from the plugin directory", async () => {
    const listPluginPrograms = vi.fn().mockResolvedValue({
      sessionId: "s",
      nodeId: "plugin-1",
      programs: [
        { id: 0, name: "Init" },
        { id: 1, name: "Bright" },
      ],
      units: [],
    });
    const selectPluginProgram = vi.fn().mockResolvedValue({ sessionId: "s", nodeId: "plugin-1", program: 1 });
    const applyPluginPreset = vi.fn().mockResolvedValue({
      sessionId: "s",
      nodeId: "plugin-1",
      presetPath: "C:\\VST2\\Lead.fxp",
    });
    render(
      <PluginNodeInspector
        node={plugin({ path: "C:\\VST2\\TDR Nova.dll", format: "vst2" })}
        disabled={false}
        onUnload={() => {}}
        snapshot={null}
        backend={{ listPluginPrograms, selectPluginProgram, applyPluginPreset }}
        sessionId="s"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Load programs" }));
    await waitFor(() => expect(listPluginPrograms).toHaveBeenCalledWith("s", "plugin-1"));
    fireEvent.change(screen.getByLabelText("VST2 program"), { target: { value: "1" } });
    await waitFor(() => expect(selectPluginProgram).toHaveBeenCalledWith("s", "plugin-1", 1));
    fireEvent.change(screen.getByLabelText("VST2 preset path"), {
      target: { value: "C:\\VST2\\Lead.fxp" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Import preset" }));
    await waitFor(() => expect(applyPluginPreset).toHaveBeenCalledWith("s", "plugin-1", "C:\\VST2\\Lead.fxp"));
  });

  it("lists VST3 programs read-only, with no choice and no preset import", async () => {
    const listPluginPrograms = vi.fn().mockResolvedValue({
      sessionId: "s",
      nodeId: "plugin-1",
      programs: [{ id: 0, name: "Init" }],
      units: [
        {
          id: 0,
          parent_id: -1,
          name: "Root",
          program_list_id: 7,
          program_count: 1,
          programs: [{ id: 0, name: "Init" }],
        },
        { id: 1, parent_id: 0, name: "Synth", program_list_id: null, program_count: 0, programs: [] },
      ],
    });
    const selectPluginProgram = vi.fn();
    render(
      <PluginNodeInspector
        node={plugin({ path: "C:\\VST3\\Synth.vst3", format: "vst3" })}
        disabled={false}
        onUnload={() => {}}
        snapshot={null}
        backend={{ listPluginPrograms, selectPluginProgram }}
        sessionId="s"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Load programs" }));
    await waitFor(() => expect(listPluginPrograms).toHaveBeenCalledWith("s", "plugin-1"));
    const select = await screen.findByLabelText("VST3 program");
    expect((select as HTMLSelectElement).disabled).toBe(true);
    expect(screen.getByRole("list", { name: "VST3 unit hierarchy" }).textContent).toContain("Within Root");
    expect(screen.getByRole("list", { name: "VST3 unit hierarchy" }).textContent).toContain("Program list · 1 program");
    expect(screen.queryByRole("button", { name: "Import preset" })).toBeNull();
    expect(selectPluginProgram).not.toHaveBeenCalled();
  });

  it("adds a VST3 sidechain input once, and explains a refusal without changing the panel", async () => {
    const addPluginSidechainPort = vi
      .fn()
      .mockRejectedValueOnce(new Error("the plugin declares no auxiliary input"))
      .mockResolvedValue({ sessionId: "s", nodeId: "plugin-1", port: "sidechain", channels: 2 });
    const { rerender } = render(
      <PluginNodeInspector
        node={plugin({ path: "C:\\VST3\\Comp.vst3", format: "vst3" })}
        disabled={false}
        onUnload={() => {}}
        snapshot={null}
        backend={{ addPluginSidechainPort }}
        sessionId="s"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Add sidechain input" }));
    expect((await screen.findByRole("alert")).textContent).toContain("no auxiliary input");
    fireEvent.click(screen.getByRole("button", { name: "Add sidechain input" }));
    await waitFor(() => expect(addPluginSidechainPort).toHaveBeenCalledTimes(2));
    expect(addPluginSidechainPort).toHaveBeenCalledWith("s", "plugin-1");
    expect(await screen.findByText(/has a sidechain input named/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Add sidechain input" })).toBeNull();
    // An effect that already has the port, or a VST2 plugin, offers no button.
    rerender(
      <PluginNodeInspector
        node={plugin({ path: "C:\\VST2\\x.dll", format: "vst2" })}
        disabled={false}
        onUnload={() => {}}
        snapshot={null}
        backend={{ addPluginSidechainPort }}
        sessionId="s"
      />,
    );
    expect(screen.queryByText("Sidechain input")).toBeNull();
  });

  it("removes a sidechain input and shows the backend's refusal when a connection uses it", async () => {
    const removePluginSidechainPort = vi
      .fn()
      .mockRejectedValueOnce(new Error("still has a connection into its sidechain port; disconnect it first"))
      .mockResolvedValue({ sessionId: "s", nodeId: "plugin-1", removed: true });
    const node = plugin({ path: "C:\\VST3\\Comp.vst3", format: "vst3" });
    node.ports = [...node.ports, { name: "sidechain", direction: "input", channels: 1 }];
    render(
      <PluginNodeInspector
        node={node}
        disabled={false}
        onUnload={() => {}}
        snapshot={null}
        backend={{ removePluginSidechainPort }}
        sessionId="s"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Remove sidechain input" }));
    expect((await screen.findByRole("alert")).textContent).toContain("disconnect it first");
    fireEvent.click(screen.getByRole("button", { name: "Remove sidechain input" }));
    await waitFor(() => expect(removePluginSidechainPort).toHaveBeenCalledTimes(2));
    expect(removePluginSidechainPort).toHaveBeenLastCalledWith("s", "plugin-1");
  });

  it("does not offer a sidechain input while the tool has unsaved changes", () => {
    const addPluginSidechainPort = vi.fn();
    render(
      <PluginNodeInspector
        node={plugin({ path: "C:\\VST3\\Comp.vst3", format: "vst3" })}
        disabled={false}
        onUnload={() => {}}
        snapshot={null}
        backend={{ addPluginSidechainPort }}
        sessionId="s"
        sidechainBlockedReason="Save or discard changes to this tool before adding a sidechain input."
      />,
    );
    const button = screen.getByRole("button", { name: "Add sidechain input" }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(screen.getByText(/Save or discard changes/)).toBeTruthy();
    expect(addPluginSidechainPort).not.toHaveBeenCalled();
  });

  it("still needs the desktop app, because the window belongs to the shell", () => {
    render(
      <PluginEditorControls
        node={plugin()}
        backend={{} as never}
        sessionId="route"
        running={false}
        onStateSaved={() => undefined}
      />,
    );
    expect((screen.getByRole("button", { name: "Open plugin window" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
