// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SidechainReplaceDialog } from "./SidechainReplaceDialog";

afterEach(cleanup);

describe("SidechainReplaceDialog", () => {
  it("offers an explicit replace or keep choice and explains the auxiliary route", () => {
    const decide = vi.fn();
    render(
      <SidechainReplaceDialog
        pluginName="Compressor"
        currentSourceName="Music"
        nextSourceName="Desktop audio"
        onDecision={decide}
      />,
    );

    expect(screen.getByRole("dialog", { name: "Replace this sidechain source?" }).textContent).toContain(
      "will not add a Mixer",
    );
    fireEvent.click(screen.getByRole("button", { name: "Keep current source" }));
    expect(decide).toHaveBeenCalledWith(false);
    fireEvent.click(screen.getByRole("button", { name: "Replace sidechain source" }));
    expect(decide).toHaveBeenCalledWith(true);
  });

  it("treats Escape as keeping the current source", () => {
    const decide = vi.fn();
    render(
      <SidechainReplaceDialog
        pluginName="Compressor"
        currentSourceName="Music"
        nextSourceName="Desktop audio"
        onDecision={decide}
      />,
    );

    fireEvent.keyDown(screen.getByRole("dialog", { name: "Replace this sidechain source?" }), { key: "Escape" });
    expect(decide).toHaveBeenCalledWith(false);
  });
});
