/** @vitest-environment jsdom */

import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { DiagnosticsSnapshot, Node } from "@audiorouter/contracts";
import { PluginNodeInspector } from "./PluginPanels";

afterEach(cleanup);

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
