// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";
import { CanvasLayoutInspector } from "./CanvasLayoutInspector";
import { readGroups, useCanvasGroups, type CanvasGroup } from "./CanvasGroups";
const base: CanvasGroup = {
  id: "group-a",
  name: "A",
  x: 0,
  y: 0,
  width: 540,
  height: 340,
  fontSize: 18,
  opacity: 5,
  color: "#5599dd",
};
afterEach(() => {
  cleanup();
  localStorage.clear();
});
function Fixture() {
  const state = useCanvasGroups("options");
  const groups = state.groups.slice(0, 2);
  return (
    <CanvasLayoutInspector
      items={[
        ...groups.map((group) => ({ ...group, group: true })),
        { id: "tool", x: 0, y: 0, width: 218, height: 180 },
      ]}
      groups={groups}
      onAction={() => {}}
      onGroupOptionsChange={(patch) =>
        state.changeGroupOptions(
          groups.map((group) => group.id),
          patch,
        )
      }
    />
  );
}
test("mixed group settings update selected locked groups without changing rectangles or other groups", () => {
  const groups = [
    base,
    { ...base, id: "group-b", locked: true, moveContents: true },
    { ...base, id: "group-c", locked: true },
  ];
  localStorage.setItem("audiorouter.ui.groups.options", JSON.stringify(groups));
  render(<Fixture />);
  const lock = screen.getByRole("checkbox", { name: "Lock position and size" }) as HTMLInputElement;
  const carry = screen.getByRole("checkbox", { name: "Move contents with group" }) as HTMLInputElement;
  expect(lock.indeterminate).toBe(true);
  expect(carry.indeterminate).toBe(true);
  fireEvent.click(lock);
  expect(lock.checked).toBe(true);
  expect(lock.indeterminate).toBe(false);
  fireEvent.click(lock);
  fireEvent.click(carry);
  expect(carry.checked).toBe(true);
  const saved = readGroups("audiorouter.ui.groups.options");
  expect(saved).toEqual([
    { ...base, locked: false, moveContents: true },
    { ...groups[1], locked: false, moveContents: true },
    groups[2],
  ]);
  for (const name of [
    "Align left",
    "Align center",
    "Align right",
    "Align top",
    "Align middle",
    "Align bottom",
    "Space horizontally",
    "Space vertically",
  ])
    expect(screen.getByRole("button", { name }).querySelector('svg[aria-hidden="true"]')).not.toBeNull();
});
test("tool-only selections have layout icons and no group controls", () => {
  render(
    <CanvasLayoutInspector
      items={[
        { id: "a", x: 0, y: 0, width: 218, height: 180 },
        { id: "b", x: 400, y: 0, width: 218, height: 180 },
      ]}
      onAction={() => {}}
    />,
  );
  expect(screen.queryByRole("checkbox")).toBeNull();
  expect(screen.getAllByRole("button")).toHaveLength(12);
});
