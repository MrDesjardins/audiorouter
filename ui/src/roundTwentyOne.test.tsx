/** @vitest-environment jsdom */

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Node, NodeKind, Session } from "@audiorouter/contracts";
import { ChangeHistory } from "./ChangeHistory";
import { CommandPalette } from "./CommandPalette";
import { canCompare, chainOf } from "./compareRules";
import { demoSession } from "./fixtures";
import { GoalDialog } from "./GoalDialog";
import { describeChange, savedAtLabel, savedByLabel } from "./historyLabels";
import { MicTest } from "./MicTest";
import { currentPeak, micVerdict } from "./micVerdict";
import { paletteResults, paletteScore, type PaletteItem } from "./paletteItems";
import { ToolCompare } from "./ToolCompare";
import type { UiBackend } from "./backend";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const node = (id: string, kind: NodeKind, name = id): Node => ({
  id,
  kind,
  typeVersion: 1,
  name,
  enabled: true,
  bypass: false,
  parameters: {},
  ports: [],
});
const edge = (source: string, destination: string) => ({
  id: `${source}-${destination}`,
  sourceNode: source,
  sourcePort: "out",
  destinationNode: destination,
  destinationPort: "in",
  matrix: [1],
  enabled: true,
});
const route: Session = {
  id: "route",
  name: "Route",
  schemaVersion: 1,
  revision: 3,
  nodes: [
    node("mic", "physicalInput", "Mic"),
    node("gate", "gate", "Gate"),
    node("comp", "compressor", "Compressor"),
    node("out", "physicalOutput", "Out"),
  ],
  edges: [edge("mic", "gate"), edge("gate", "comp"), edge("comp", "out")],
};

describe("mic verdict uses UI-21's voice zone", () => {
  it.each([
    [-Infinity, "silent"],
    [Number.NaN, "silent"],
    [-51, "silent"],
    [-50, "quiet"],
    [-18.1, "quiet"],
    [-18, "good"],
    [-6, "good"],
    [-5.9, "loud"],
  ])("%s dB is %s", (peak, verdict) => expect(micVerdict(peak)).toBe(verdict));

  it("reads the loudest channel and treats a missing meter as nothing", () => {
    expect(currentPeak({ channelCurrentPeakDb: [-30, -12] })).toBe(-12);
    expect(currentPeak(null)).toBeNull();
    expect(currentPeak({})).toBeNull();
  });
});

describe("Mic test card", () => {
  const diagnostics = (peak: number) =>
    ({ nodeTelemetry: [{ nodeId: "mic", meter: { currentPeakDb: peak } }] }) as never;

  it("refuses to start on a stopped route and starts nothing itself", () => {
    render(<MicTest session={route} diagnostics={null} running={false} privacyMuted={false} />);
    expect(screen.getByText("Press Play to test this mic")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Test my mic" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("will not call privacy mute silence a dead mic", () => {
    render(<MicTest session={route} diagnostics={diagnostics(-60)} running privacyMuted />);
    expect(screen.getByText("Privacy mute is on")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Test my mic" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("judges the loudest moment after five seconds and says what it cannot know", () => {
    vi.useFakeTimers();
    const { rerender } = render(
      <MicTest session={route} diagnostics={diagnostics(-40)} running privacyMuted={false} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Test my mic" }));
    expect(screen.getByText("Listening for 5 seconds…")).toBeTruthy();
    rerender(<MicTest session={route} diagnostics={diagnostics(-12)} running privacyMuted={false} />);
    rerender(<MicTest session={route} diagnostics={diagnostics(-40)} running privacyMuted={false} />);
    act(() => void vi.advanceTimersByTime(5000));
    expect(screen.getByText("Good: we hear you, in the voice zone")).toBeTruthy();
    expect(screen.getByText(/It cannot tell what other apps hear/)).toBeTruthy();
    expect(screen.getByText("Peak −12 dB")).toBeTruthy();
  });

  it("offers Windows sound settings only when nothing was heard", () => {
    vi.useFakeTimers();
    render(<MicTest session={route} diagnostics={diagnostics(-90)} running privacyMuted={false} />);
    const settings = screen.getByText("Open Windows sound settings") as HTMLButtonElement;
    expect(settings.hidden).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Test my mic" }));
    act(() => void vi.advanceTimersByTime(5000));
    expect(screen.getByText("No sound from this mic")).toBeTruthy();
    expect(settings.hidden).toBe(false);
  });
});

describe("compare rules", () => {
  it("finds the tools in line and stops at sources, outputs and splits", () => {
    expect(chainOf(route, "gate").sort()).toEqual(["comp", "gate"]);
    expect(canCompare(node("x", "physicalInput"))).toBe(false);
    const split: Session = {
      ...route,
      nodes: [...route.nodes, node("extra", "gain")],
      edges: [...route.edges, edge("gate", "extra")],
    };
    expect(chainOf(split, "comp")).toEqual(["comp"]);
  });
});

describe("hold to compare", () => {
  const setup = (running = true) => {
    const compareNodes = vi.fn(async () => ({ comparing: [], expiresInMs: 0 }));
    const backend = { compareNodes, connected: true } as unknown as UiBackend;
    const view = render(<ToolCompare backend={backend} session={route} node={route.nodes[1]} running={running} />);
    return { compareNodes, view, button: screen.getByRole("button", { name: /without this tool/ }) };
  };
  const lastNodes = (mock: ReturnType<typeof vi.fn>) => (mock.mock.calls.at(-1)![0] as { nodes: string[] }).nodes;

  it("warns that a hold is live and sends the tool only while held", () => {
    const { compareNodes, button } = setup();
    expect(screen.getByText(/anyone hearing this route, on Discord or a stream, hears it too/)).toBeTruthy();
    fireEvent.pointerDown(button, { pointerId: 1 });
    expect(lastNodes(compareNodes)).toEqual(["gate"]);
    fireEvent.pointerUp(button, { pointerId: 1 });
    expect(lastNodes(compareNodes)).toEqual([]);
  });

  it("leaves the whole chain out with the second button", () => {
    const { compareNodes } = setup();
    fireEvent.pointerDown(screen.getByRole("button", { name: /without the chain/ }), { pointerId: 1 });
    expect(lastNodes(compareNodes).sort()).toEqual(["comp", "gate"]);
  });

  it.each([
    ["the window losing focus", () => fireEvent.blur(window)],
    ["the page being hidden", () => fireEvent(document, new Event("visibilitychange"))],
  ])("restores on %s", (_name, leave) => {
    const { compareNodes, button } = setup();
    fireEvent.pointerDown(button, { pointerId: 1 });
    leave();
    expect(lastNodes(compareNodes)).toEqual([]);
    expect(compareNodes).toHaveBeenCalledTimes(2);
  });

  it("restores when the panel goes away", () => {
    const { compareNodes, button, view } = setup();
    fireEvent.pointerDown(button, { pointerId: 1 });
    view.unmount();
    expect(lastNodes(compareNodes)).toEqual([]);
  });

  it("does nothing on a stopped route", () => {
    const { compareNodes, button } = setup(false);
    fireEvent.pointerDown(button, { pointerId: 1 });
    expect(compareNodes).not.toHaveBeenCalled();
    expect(screen.getByText(/Press Play to compare/)).toBeTruthy();
  });
});

describe("palette", () => {
  const item = (
    id: string,
    group: PaletteItem["group"],
    title: string,
    extra: Partial<PaletteItem> = {},
  ): PaletteItem => ({
    id,
    group,
    icon: "search",
    title,
    detail: "",
    run: vi.fn(),
    ...extra,
  });

  it("prefers a title start, then a word start, then a synonym", () => {
    const entry = { title: "Add Gain", detail: "", keywords: "volume" };
    expect(paletteScore(entry, "add")).toBeGreaterThan(paletteScore(entry, "gain"));
    expect(paletteScore(entry, "gain")).toBeGreaterThan(paletteScore(entry, "volume"));
    expect(paletteScore(entry, "nothing")).toBe(0);
  });

  it("keeps the group order whatever matches best", () => {
    const results = paletteResults([item("s", "Sessions", "Mute night"), item("a", "Actions", "Privacy mute")], "mute");
    expect(results.map((result) => result.id)).toEqual(["a", "s"]);
  });

  it("runs the chosen item, moves with arrows and closes on Escape", () => {
    const first = item("a", "Actions", "Play");
    const second = item("b", "Actions", "Privacy mute");
    const onClose = vi.fn();
    render(<CommandPalette items={[first, second]} onClose={onClose} />);
    const input = screen.getByRole("combobox");
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(second.run).toHaveBeenCalledOnce();
    expect(first.run).not.toHaveBeenCalled();
    fireEvent.keyDown(input, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
    expect(screen.getByText("Nothing is deleted from here")).toBeTruthy();
  });

  it("will not run a disabled item", () => {
    const blocked = item("a", "Actions", "Play", { disabled: true });
    render(<CommandPalette items={[blocked]} onClose={vi.fn()} />);
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
    expect(blocked.run).not.toHaveBeenCalled();
  });
});

describe("change history", () => {
  it("describes a change in plain words", () => {
    const withTool = { ...route, nodes: [...route.nodes, node("g", "gain", "Gain")] };
    expect(describeChange(route, withTool)).toBe("Added Gain");
    expect(describeChange(withTool, route)).toBe("Removed Gain");
    expect(describeChange(route, { ...route, edges: route.edges.filter((item) => item.id !== "gate-comp") })).toBe(
      "Disconnected Gate from Compressor",
    );
    expect(
      describeChange(route, {
        ...route,
        nodes: route.nodes.map((item) => (item.id === "gate" ? { ...item, bypass: true } : item)),
      }),
    ).toBe("Bypassed Gate");
  });

  it("reports who saved exactly as told, and never guesses", () => {
    expect(savedByLabel(null)).toBe("Saver not recorded");
    expect(savedByLabel("this PC")).toBe("this PC");
    expect(savedByLabel("audiorouter-local")).toBe("saved by “audiorouter-local”");
    expect(savedAtLabel(null)).toBe("Time not recorded");
  });

  const versions = [
    { revision: 3, name: "Route", toolCount: 4, connectionCount: 3, savedAtMs: 1000, savedBy: "this PC" },
    { revision: 2, name: "Route", toolCount: 3, connectionCount: 2, savedAtMs: 500, savedBy: "assistant" },
  ];
  const panel = (backend: Partial<UiBackend>, routeChanged = false, past: Session[] = []) => (
    <ChangeHistory
      backend={{ connected: true, ...backend } as UiBackend}
      session={route}
      past={past}
      current={route}
      routeChanged={routeChanged}
      running={false}
      onUndoSteps={vi.fn()}
      onReverted={vi.fn()}
    />
  );

  it("goes back through plan and commit, and marks where you are", async () => {
    const restoreVersionPlan = vi.fn(async () => ({ planId: "p", baseRevision: 3, expiresInMs: 1 }));
    const commitGraph = vi.fn(async () => ({}));
    render(
      panel({
        listSavedVersions: vi.fn(async () => ({ items: versions })),
        restoreVersionPlan,
        commitGraph,
      } as never),
    );
    expect(await screen.findByText("You are here")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Go back to the version/ }));
    await waitFor(() => expect(commitGraph).toHaveBeenCalled());
    expect(restoreVersionPlan).toHaveBeenCalledWith("route", 3, 2);
    expect(await screen.findByText(/Later versions are still in this list/)).toBeTruthy();
  });

  it("does not list changes that are already saved, though Undo still has them", async () => {
    const before = { ...route, nodes: route.nodes.slice(0, 3) };
    render(panel({ listSavedVersions: vi.fn(async () => ({ items: versions })) } as never, false, [before]));
    expect(await screen.findByText("No unsaved changes.")).toBeTruthy();
    expect(screen.queryByText("Added Out")).toBeNull();
  });

  it("lists only what came after the save", async () => {
    const older = { ...route, nodes: route.nodes.slice(0, 2) };
    const saved = route;
    render(
      <ChangeHistory
        backend={{ connected: true } as UiBackend}
        session={saved}
        past={[older, saved]}
        current={{ ...route, nodes: route.nodes.slice(0, 3) }}
        routeChanged
        running={false}
        onUndoSteps={vi.fn()}
        onReverted={vi.fn()}
      />,
    );
    const list = screen.getByRole("list", { name: "Changes since last save" });
    expect(list.querySelectorAll("li")).toHaveLength(1);
    expect(list.textContent).toContain("Removed Out");
  });

  it("says so when Undo has gone past the save", async () => {
    render(
      <ChangeHistory
        backend={{ connected: true } as UiBackend}
        session={route}
        past={[{ ...route, nodes: route.nodes.slice(0, 1) }]}
        current={{ ...route, nodes: route.nodes.slice(0, 2) }}
        routeChanged
        running={false}
        onUndoSteps={vi.fn()}
        onReverted={vi.fn()}
      />,
    );
    expect(await screen.findByText(/You undid past your last save/)).toBeTruthy();
  });

  it("refuses to go back over unsaved changes", async () => {
    const restoreVersionPlan = vi.fn();
    render(panel({ listSavedVersions: vi.fn(async () => ({ items: versions })), restoreVersionPlan } as never, true));
    fireEvent.click(await screen.findByRole("button", { name: /Go back to the version/ }));
    expect(await screen.findByText(/Save or discard your unsaved changes/)).toBeTruthy();
    expect(restoreVersionPlan).not.toHaveBeenCalled();
  });

  it("lists unsaved changes newest first and undoes back to one", async () => {
    const onUndoSteps = vi.fn();
    const before = { ...route, nodes: route.nodes.slice(0, 3) };
    render(
      <ChangeHistory
        backend={{ connected: true } as UiBackend}
        session={before}
        past={[before, { ...route, name: "Other" }]}
        current={{ ...route, name: "Now" }}
        routeChanged
        running={false}
        onUndoSteps={onUndoSteps}
        onReverted={vi.fn()}
      />,
    );
    const list = screen.getByRole("list", { name: "Changes since last save" });
    expect(list.textContent).toMatch(/Renamed the session to Route.*Renamed the session to Other|Renamed/);
    fireEvent.click(screen.getByRole("button", { name: /Undo back to before: Added Out/ }));
    expect(onUndoSteps).toHaveBeenCalledWith(2);
  });
});

describe("goal dialog", () => {
  const goal = (id: string, main: boolean, applicationPrompt: string | null = null) => ({
    id,
    title: id,
    summary: "s",
    adds: [],
    main,
    applicationPrompt,
    stillToChoose: [],
  });
  const created = { session: demoSession, state: "stopped" as const, stillToChoose: [] };

  it("makes a route straight away when it needs no app, and promises nothing plays", async () => {
    const createSessionFromGoal = vi.fn(async () => created);
    const onCreated = vi.fn();
    render(
      <GoalDialog
        backend={
          {
            listSessionGoals: vi.fn(async () => ({ goals: [goal("clean-mic", true)] })),
            createSessionFromGoal,
          } as never
        }
        onCreated={onCreated}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByText(/Nothing plays until you press Play/)).toBeTruthy();
    fireEvent.click(await screen.findByRole("button", { name: "Use this: clean-mic" }));
    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(created));
    expect(createSessionFromGoal.mock.calls[0]).toEqual([
      expect.objectContaining({ goal: "clean-mic", idempotencyKey: expect.any(String) }),
    ]);
  });

  it("asks which app, and can make the route without one", async () => {
    const createSessionFromGoal = vi.fn(async () => created);
    const listApplications = vi.fn(async () => [
      { executable: "game.exe", executablePath: "C:\\game.exe", audioDisplayNames: ["Game"] },
    ]);
    render(
      <GoalDialog
        backend={
          {
            listSessionGoals: vi.fn(async () => ({ goals: [goal("podcast", true, "Which app?")] })),
            createSessionFromGoal,
            listApplications,
          } as never
        }
        onCreated={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(await screen.findByRole("button", { name: "Use this: podcast" }));
    expect(await screen.findByText("Which app?")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Make this route" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Choose later" }));
    await waitFor(() => expect(createSessionFromGoal).toHaveBeenCalled());
    expect(createSessionFromGoal.mock.calls[0]).toEqual([
      expect.not.objectContaining({ application: expect.anything() }),
    ]);
  });

  it("shows why a route could not be made and stays open", async () => {
    const onCreated = vi.fn();
    render(
      <GoalDialog
        backend={
          {
            listSessionGoals: vi.fn(async () => ({ goals: [goal("clean-mic", true)] })),
            createSessionFromGoal: vi.fn(async () => {
              throw new Error("nope");
            }),
          } as never
        }
        onCreated={onCreated}
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(await screen.findByRole("button", { name: "Use this: clean-mic" }));
    expect(await screen.findByRole("heading", { name: "What do you want to do?" })).toBeTruthy();
    await waitFor(() => expect(screen.getByRole("status").textContent).not.toBe(""));
    expect(onCreated).not.toHaveBeenCalled();
  });
});
