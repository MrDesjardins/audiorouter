/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GoalSetupGuide } from "./GoalSetupGuide";

afterEach(cleanup);
describe("goal-led setup", () => {
  it("uses the existing voice setup only after deliberate action and never claims verification", () => {
    const onSetUpDiscord = vi.fn();
    render(<GoalSetupGuide connected onOpenTools={vi.fn()} onSetUpDiscord={onSetUpDiscord} />);
    fireEvent.click(screen.getByText("Choose a setup goal"));
    expect(onSetUpDiscord).not.toHaveBeenCalled();
    expect(screen.getByText(/Keep microphone monitoring muted/)).toBeTruthy();
    expect(screen.getAllByText(/not yet verified/)).toHaveLength(2);
    fireEvent.click(screen.getByRole("button", { name: "Choose microphone and voice cable" }));
    expect(onSetUpDiscord).toHaveBeenCalledOnce();
  });
  it("guides application recording without arming it", () => {
    const onOpenTools = vi.fn();
    render(<GoalSetupGuide connected onOpenTools={onOpenTools} />);
    fireEvent.click(screen.getByText("Choose a setup goal"));
    fireEvent.change(screen.getByLabelText("What do you want to do?"), { target: { value: "recording" } });
    expect(screen.getByText(/Leave it unarmed/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Open Tools for this goal" }));
    expect(onOpenTools).toHaveBeenCalledOnce();
  });
  it("withholds setup actions offline", () => {
    render(<GoalSetupGuide connected={false} onOpenTools={vi.fn()} />);
    fireEvent.click(screen.getByText("Choose a setup goal"));
    expect(screen.getByRole<HTMLButtonElement>("button").disabled).toBe(true);
  });
});
