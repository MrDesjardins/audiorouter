import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ApplicationIdentityPanel } from "./ApplicationIdentityPanel";
import type { ApplicationRow } from "./backend";

const application: ApplicationRow = {
  processId: 42,
  executable: "game.exe",
  executablePath: "C:\\Games\\game.exe",
  creationTime100ns: "123",
  audioActivity: "active",
  captureCapability: "observed",
  audioSessionCount: 1,
  activeAudioSessionCount: 1,
  captureSessionCount: 0,
  renderSessionCount: 1,
  audioDisplayNames: ["Game"],
};

describe("ApplicationIdentityPanel", () => {
  it("renders verified paths as read-only identity details", () => {
    const markup = renderToStaticMarkup(<ApplicationIdentityPanel applications={[application]} />);

    expect(markup).toContain("Verified executable paths");
    expect(markup).toContain("C:\\Games\\game.exe");
    expect(markup).toContain("PID 42");
    expect(markup).toContain("read-only discovery data");
  });

  it("reports when the backend has no verified paths", () => {
    const markup = renderToStaticMarkup(
      <ApplicationIdentityPanel applications={[{ ...application, executablePath: null }]} />,
    );

    expect(markup).toContain("No verified full executable paths");
    expect(markup).not.toContain("C:\\Games\\game.exe");
  });
});
