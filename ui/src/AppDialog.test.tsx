/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AppDialogHost, askConfirm, askText } from "./AppDialog";

const confirmQuestion = {
  title: "Delete “Main”?",
  message: "It cannot be undone.",
  confirmLabel: "Delete session",
  destructive: true,
};
const textQuestion = {
  title: "New session",
  label: "Session name",
  initialValue: "New session",
  confirmLabel: "Create",
};

afterEach(cleanup);

describe("app question dialogs", () => {
  it("confirms, cancels and starts on the safe button", async () => {
    render(<AppDialogHost />);
    let answer = askConfirm(confirmQuestion);
    let dialog = await screen.findByRole("dialog", { name: "Delete “Main”?" });
    expect(document.activeElement, "Enter on the first focus must not delete").toBe(
      within(dialog).getByRole("button", { name: "Cancel" }),
    );
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    fireEvent.click(within(dialog).getByRole("button", { name: "Delete session" }));
    await expect(answer).resolves.toBe(true);

    answer = askConfirm(confirmQuestion);
    dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await expect(answer).resolves.toBe(false);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("treats Escape as cancel, and does not let it reach what is behind", async () => {
    let reachedBehind = false;
    render(
      <div
        onKeyDown={() => {
          reachedBehind = true;
        }}
      >
        <AppDialogHost />
      </div>,
    );
    const answer = askConfirm(confirmQuestion);
    fireEvent.keyDown(await screen.findByRole("dialog"), { key: "Escape" });
    await expect(answer).resolves.toBe(false);
    expect(reachedBehind).toBe(false);
  });

  it("keeps Tab inside the dialog, past a text field too", async () => {
    render(<AppDialogHost />);
    const answer = askText(textQuestion);
    const dialog = await screen.findByRole("dialog", { name: "New session" });
    const field = within(dialog).getByLabelText("Session name");
    const create = within(dialog).getByRole("button", { name: "Create" });
    expect(document.activeElement).toBe(field);

    create.focus();
    fireEvent.keyDown(create, { key: "Tab" });
    expect(document.activeElement).toBe(field);
    fireEvent.keyDown(field, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(create);

    fireEvent.keyDown(dialog, { key: "Escape" });
    await expect(answer).resolves.toBeNull();
  });

  it("returns trimmed text and refuses an empty name inside the dialog", async () => {
    render(<AppDialogHost />);
    const answer = askText(textQuestion);
    const dialog = await screen.findByRole("dialog");
    const field = within(dialog).getByLabelText("Session name");

    fireEvent.change(field, { target: { value: "   " } });
    fireEvent.submit(dialog.querySelector("form")!);
    expect(await within(dialog).findByText("Enter a name first.")).toBeTruthy();
    expect(screen.getByRole("dialog"), "still open").toBe(dialog);

    fireEvent.change(field, { target: { value: "  Practice  " } });
    expect(within(dialog).queryByText("Enter a name first.")).toBeNull();
    fireEvent.submit(dialog.querySelector("form")!);
    await expect(answer).resolves.toBe("Practice");
  });

  it("asks one question at a time and gives focus back", async () => {
    render(
      <>
        <button type="button">Behind</button>
        <AppDialogHost />
      </>,
    );
    const behind = screen.getByRole("button", { name: "Behind" });
    behind.focus();
    const first = askConfirm({ ...confirmQuestion, title: "First" });
    const second = askConfirm({ ...confirmQuestion, title: "Second" });
    const dialog = await screen.findByRole("dialog", { name: "First" });
    expect(screen.queryByRole("dialog", { name: "Second" })).toBeNull();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await expect(first).resolves.toBe(false);
    const next = await screen.findByRole("dialog", { name: "Second" });
    fireEvent.click(within(next).getByRole("button", { name: "Cancel" }));
    await expect(second).resolves.toBe(false);
    expect(document.activeElement).toBe(behind);
  });
});
