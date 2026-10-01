// @vitest-environment jsdom
import { fireEvent, render, screen, act, cleanup } from "@testing-library/react";
import { beforeEach, afterEach, expect, test } from "vitest";
import { CanvasGroupInspector, readGroups, useCanvasGroups, type CanvasGroup } from "./CanvasGroups";

beforeEach(() => localStorage.clear());
afterEach(cleanup);
function Fixture({ session = "one" }: { session?: string }) {
  const groups = useCanvasGroups(session);
  const selected = groups.groups.find((group) => group.id === groups.selectedGroupId);
  return <><button onClick={groups.addGroup}>Add group</button><output>{groups.groups.length}</output>{selected && <CanvasGroupInspector group={selected} onChange={(patch) => groups.changeGroup(selected.id, patch)} onRemove={() => groups.removeGroup(selected.id)} />}</>;
}
test("groups persist presentation edits independently by session with 5% default opacity", async () => {
  const view = render(<Fixture />);
  fireEvent.click(screen.getByText("Add group"));
  expect((screen.getByLabelText("Group opacity") as HTMLInputElement).value).toBe("5");
  fireEvent.change(screen.getByLabelText("Group name"), { target: { value: "Game" } });
  fireEvent.change(screen.getByLabelText("Group background color"), { target: { value: "#ee8822" } });
  fireEvent.change(screen.getByLabelText("Group opacity"), { target: { value: "40" } });
  expect(readGroups("audiorouter.ui.groups.one")[0]).toMatchObject({ name: "Game", color: "#ee8822", opacity: 40 });
  await act(async () => view.rerender(<Fixture session="two" />));
  expect(screen.getByRole("status").textContent).toBe("0");
  await act(async () => view.rerender(<Fixture />));
  expect(screen.getByRole("status").textContent).toBe("1");
});
test("imported annotations reject invalid geometry and duplicate IDs, and clamp opacity", () => {
  const group: CanvasGroup = { id: "group-test", name: "Game", color: "#abcdef", opacity: 25, fontSize: 18, x: 0, y: 0, width: 540, height: 340 };
  localStorage.setItem("groups", JSON.stringify([group, group, { ...group, id: "audio-node" }, { ...group, id: "group-bad", width: 1e100 }, { ...group, id: "group-opacity", opacity: 101 }]));
  expect(readGroups("groups")).toEqual([group, { ...group, id: "group-opacity", opacity: 100 }]);
});
