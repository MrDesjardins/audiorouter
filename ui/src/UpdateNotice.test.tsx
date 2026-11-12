/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { VersionLine, UpdatesPanel, checkedAtLabel, nextCheckLabel } from "./UpdateNotice";
import { APP_VERSION, CHECK_INTERVAL_MS, type UpdateCheckState } from "./updateCheck";

afterEach(cleanup);

const update = {
  version: "0.0.28",
  tag: "v0.0.28",
  url: "https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.28",
};

function state(values: Partial<UpdateCheckState> = {}): UpdateCheckState {
  return {
    status: "current",
    update: null,
    lastCheckedAt: Date.now(),
    lastAttemptAt: Date.now(),
    nextCheckAt: Date.now() + CHECK_INTERVAL_MS,
    retrying: false,
    failure: null,
    failureCount: 0,
    checkNow: vi.fn(),
    ...values,
  };
}

function panel(values: Partial<UpdateCheckState> = {}, enabled = true) {
  const onChange = vi.fn();
  const onChannelChange = vi.fn();
  render(
    <UpdatesPanel
      enabled={enabled}
      channel="prerelease"
      onChange={onChange}
      onChannelChange={onChannelChange}
      updateState={state(values)}
    />,
  );
  return { onChange, onChannelChange };
}

describe("UpdatesPanel", () => {
  it("shows an available release and enables its action", () => {
    panel({ status: "available", update });
    expect(screen.getByText("Version 0.0.28 is available.")).toBeTruthy();
    expect(screen.getByText(new RegExp(`You have ${APP_VERSION}\\. Next check`))).toBeTruthy();
    expect((screen.getByRole("button", { name: "Open release page" }) as HTMLButtonElement).disabled).toBe(false);
    expect((screen.getByRole("button", { name: "Check now" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("reports the current version without a release action", () => {
    panel();
    expect(screen.getByText("You have the newest version.")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Open release page" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("does not call an installed prerelease the newest stable version", () => {
    render(
      <UpdatesPanel enabled channel="stable" onChange={vi.fn()} onChannelChange={vi.fn()} updateState={state()} />,
    );
    expect(screen.getByText("No newer stable version was found.")).toBeTruthy();
    expect(
      screen.getByText(
        new RegExp(`AudioRouter ${APP_VERSION} is installed\\. Only stable releases are checked\\. Next check`),
      ),
    ).toBeTruthy();
  });

  it("shows that checking is off and disables Check now", () => {
    panel({ status: "off" }, false);
    expect(screen.getByText("Checking is off.")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Check now" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("checkbox", { name: /Check GitHub once a day/ }) as HTMLInputElement).checked).toBe(false);
  });

  it("shows a checking state with a busy action", () => {
    panel({ status: "checking" });
    const button = screen.getByRole("button", { name: "Checking…" });
    expect((button as HTMLButtonElement).disabled).toBe(true);
    expect(button.getAttribute("aria-busy")).toBe("true");
  });

  it("explains a retry and a skipped day", () => {
    const now = Date.now();
    panel({
      status: "failed",
      failure: "offline",
      retrying: true,
      nextCheckAt: now + 5 * 60 * 1000,
    });
    expect(screen.getByText("Can't check for new versions right now.")).toBeTruthy();
    expect(screen.getByText(/No internet connection\. Trying again/)).toBeTruthy();
    cleanup();

    panel({
      status: "failed",
      failure: "rate-limited",
      retrying: false,
      nextCheckAt: now + CHECK_INTERVAL_MS,
    });
    expect(screen.getByText(/GitHub is limiting checks\. The next check is/)).toBeTruthy();
  });

  it("calls Check now, the enable switch and the channel choice", () => {
    const checkNow = vi.fn();
    const { onChange, onChannelChange } = panel({ checkNow });
    fireEvent.click(screen.getByRole("button", { name: "Check now" }));
    expect(checkNow).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("checkbox", { name: /Check GitHub once a day/ }));
    expect(onChange).toHaveBeenCalledWith(false);
    fireEvent.change(screen.getByRole("combobox", { name: "Versions to check" }), {
      target: { value: "stable" },
    });
    expect(onChannelChange).toHaveBeenCalledWith("stable");
  });
});

describe("VersionLine", () => {
  it("shows the version and opens a release page", () => {
    const open = vi.spyOn(window, "open").mockReturnValue(null);
    render(<VersionLine update={update} />);
    expect(screen.getByText(APP_VERSION)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "0.0.28 available" }));
    expect(open).toHaveBeenCalledWith(update.url, "_blank", "noopener");
    open.mockRestore();
  });
});

describe("status labels", () => {
  it("describes the last check and the next attempt", () => {
    const now = Date.UTC(2026, 10, 11, 12, 0, 0);
    expect(checkedAtLabel(null, now)).toBe("Not checked yet.");
    expect(checkedAtLabel(now - 30_000, now)).toBe("Checked just now.");
    expect(checkedAtLabel(now - 60_000, now)).toBe("Checked 1 minute ago.");
    expect(checkedAtLabel(now - 2 * 60 * 60 * 1000, now)).toBe("Checked 2 hours ago.");
    expect(checkedAtLabel(now - 3 * 24 * 60 * 60 * 1000, now)).toBe("Checked 3 days ago.");
    expect(nextCheckLabel(now - 1, now)).toBe("now");
    expect(nextCheckLabel(now + 5 * 60 * 1000, now)).toMatch(/^at /);
    expect(nextCheckLabel(now + CHECK_INTERVAL_MS, now)).toMatch(/^tomorrow at /);
  });
});
