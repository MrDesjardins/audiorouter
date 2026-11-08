import { describe, expect, it } from "vitest";
import { actionMessageLifetimeMs, actionMessageTone } from "./actionMessage";
import { formatUiError } from "./backend";

describe("action message tone", () => {
  it("marks playback, permission, and route failures as errors", () => {
    for (const message of [
      "This route includes an audio combination the current engine cannot play. Try a separate Test Signal → Output Device route. Your saved route was not changed.",
      "permission denied: DeviceAdministration [permissionDenied] request the required permission scope for the target operation",
      "No audio started. Save the route, then press Play: application capture runs the saved route.",
      "Change rejected: Invalid audio source setting.",
      "Could not change the application source.",
      "Unable to start session.",
      "Application inventory unavailable: timeout",
    ]) {
      expect(actionMessageTone(message), message).toBe("error");
    }
  });

  it("marks any caught backend error as an error whatever its wording", () => {
    const shown = formatUiError(
      new Error("enabled plugin nodes require an attached native endpoint session"),
      "Unable to start session.",
    );
    expect(actionMessageTone(shown)).toBe("error");
  });

  it("separates success, warning, and plain progress", () => {
    expect(actionMessageTone("Audio session is running (generation 4).")).toBe("success");
    expect(actionMessageTone("Route saved.")).toBe("success");
    expect(actionMessageTone("The selected route changed while checking audio. Press Play again.")).toBe("warning");
    expect(actionMessageTone("Review the route warnings in Session before saving.")).toBe("warning");
    for (const message of [
      "Preparing every path of this session...",
      "Starting session...",
      "Draft updated. Review and save the changes when you are ready.",
      "Application source changed to Discord.exe (PID 36808). Review and save the changes when you are ready.",
      null,
    ]) {
      expect(actionMessageTone(message), String(message)).toBe("info");
    }
  });
});

describe("message lifetime", () => {
  it("closes info after 8 s and warnings after 30 s, but keeps errors, progress and messages with a button", () => {
    expect(actionMessageLifetimeMs("Connection removed. Save to keep this.", false)).toBe(8_000);
    expect(actionMessageLifetimeMs("Review the warnings, then save.", false)).toBe(30_000);
    expect(actionMessageLifetimeMs("Unable to start session.", false)).toBeNull();
    expect(actionMessageLifetimeMs("Saving route...", false)).toBeNull();
    expect(actionMessageLifetimeMs("Connection removed. Save to keep this.", true)).toBeNull();
    expect(actionMessageLifetimeMs(null, false)).toBeNull();
  });
});
