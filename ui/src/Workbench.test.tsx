/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useState, type ComponentProps } from "react";
import { Workbench, type WorkbenchTab } from "./Workbench";
import { returnToPropertiesTab } from "./workbenchPanel";

afterEach(cleanup);

it("allows forward Tab to leave Properties after the last control", () => {
  render(
    <>
      <section id="workbench-properties" role="tabpanel" tabIndex={0} onKeyDown={returnToPropertiesTab}>
        <button>Last property</button>
      </section>
      <Workbench {...props} tab="properties" />
      <button>After workbench</button>
    </>,
  );
  const last = screen.getByRole("button", { name: "Last property" });
  vi.spyOn(last, "getClientRects").mockReturnValue([{}] as unknown as DOMRectList);
  last.focus();
  fireEvent.keyDown(last, { key: "Tab" });
  const tab = screen.getByRole("tab", { name: "Properties" });
  expect(document.activeElement).toBe(tab);
  expect(fireEvent.keyDown(tab, { key: "Tab" })).toBe(true);
  expect(document.activeElement).toBe(tab);
  // jsdom does not perform default Tab navigation; lack of preventDefault lets
  // the browser advance beyond the tablist rather than reenter the overlay.
});

it("tabs into the preceding Properties overlay and returns with Shift+Tab", () => {
  render(
    <>
      <section id="workbench-properties" role="tabpanel" tabIndex={0} onKeyDown={returnToPropertiesTab}>
        Properties
      </section>
      <Workbench {...props} tab="properties" />
    </>,
  );
  const tab = screen.getByRole("tab", { name: "Properties" });
  tab.focus();
  fireEvent.keyDown(tab, { key: "Tab" });
  const panel = screen.getByRole("tabpanel");
  expect(document.activeElement).toBe(panel);
  fireEvent.keyDown(panel, { key: "Tab", shiftKey: true });
  expect(document.activeElement).toBe(tab);
});

const props: ComponentProps<typeof Workbench> = {
  tab: "tools",
  onTab: vi.fn(),
  tools: [],
  connected: false,
  onAdd: vi.fn(),
  onApplicationPicker: vi.fn(),
  librarySearch: "",
  onLibrarySearch: vi.fn(),
  onNewSession: vi.fn(),
  onDuplicate: vi.fn(),
  onDelete: vi.fn(),
  onUndo: vi.fn(),
  onRedo: vi.fn(),
  onDiscard: vi.fn(),
  sessions: [],
  selectedSessionId: "",
  onSelectSession: vi.fn(),
  sessionName: "",
  onNameChange: vi.fn(),
  diagnostics: [],
  backendActivity: [],
  mcpActivity: [],
  mcpSetupInfo: null,
  mcpTunnelStatus: null,
  onOpenMcpSetupPage: vi.fn(),
  onMcpTunnelConfigure: vi.fn(),
  onMcpTunnelConnect: vi.fn(),
  onMcpTunnelDisconnect: vi.fn(),
  onMcpTunnelForget: vi.fn(),
  clientsPanel: null,
  statusContent: <button>Retry check</button>,
  virtualCablesContent: null,
  appsContent: null,
  uiOptionsContent: null,
  recordingContent: null,
  advancedContent: null,
};

function Harness() {
  const [tab, onTab] = useState<WorkbenchTab>("tools");
  return <Workbench {...props} tab={tab} onTab={onTab} />;
}

it("uses one tab stop, wrapping arrows and Home/End with automatic activation", async () => {
  render(<Harness />);
  const tabs = screen.getAllByRole("tab");
  const selected = (index: number) => {
    expect(tabs.filter((tab) => tab.tabIndex === 0)).toEqual([tabs[index]]);
    expect(tabs[index].getAttribute("aria-selected")).toBe("true");
    expect(document.activeElement).toBe(tabs[index]);
  };
  tabs[0].focus();
  fireEvent.keyDown(tabs[0], { key: "ArrowLeft" });
  await waitFor(() => selected(12));
  fireEvent.keyDown(tabs[12], { key: "ArrowRight" });
  await waitFor(() => selected(0));
  fireEvent.keyDown(tabs[0], { key: "End" });
  await waitFor(() => selected(12));
  fireEvent.keyDown(tabs[12], { key: "Home" });
  await waitFor(() => selected(0));
  fireEvent.keyDown(tabs[0], { key: "ArrowRight" });
  await waitFor(() => selected(1));
  fireEvent.keyDown(tabs[1], { key: "ArrowRight", ctrlKey: true });
  selected(1);
});

it("associates active panels and keeps focus on tabs and surviving controls across errors", () => {
  const view = render(<Workbench {...props} tab="status" />);
  const status = screen.getByRole("tab", { name: "Status" });
  const panel = screen.getByRole("tabpanel", { name: "Status" });
  expect(status.getAttribute("aria-controls")).toBe(panel.id);
  expect(panel.getAttribute("aria-labelledby")).toBe(status.id);
  const retry = screen.getByRole("button", { name: "Retry check" });
  retry.focus();
  view.rerender(<Workbench {...props} tab="status" statusHasProblem />);
  expect(document.activeElement).toBe(retry);
  view.rerender(<Workbench {...props} tab="tools" />);
  expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Tools" }));
});

it("does not steal external focus when selection changes", () => {
  const view = render(
    <>
      <button>Canvas action</button>
      <Workbench {...props} />
    </>,
  );
  const outside = screen.getByRole("button", { name: "Canvas action" });
  outside.focus();
  view.rerender(
    <>
      <button>Canvas action</button>
      <Workbench {...props} tab="status" />
    </>,
  );
  expect(document.activeElement).toBe(outside);
});
