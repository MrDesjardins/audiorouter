import { describe, expect, it } from "vitest";
import { actionMessageTone } from "./actionMessage";
import { formatUiError } from "./backend";

describe("action message tone", () => {
  it("marks playback, permission, and route failures as errors", () => {
    for (const message of [
      "This route includes an audio combination the current engine cannot play. Try a separate Test Signal → Physical Output route. Your saved route was not changed.",
      "permission denied: DeviceAdministration [permissionDenied] request the required permission scope for the target operation",
      "No audio started. Save the route, then press Play: application capture runs the saved route.",
      "Draft rejected: Invalid audio source setting.",
      "Could not change the application source.",
      "Unable to start session.",
      "Application inventory unavailable: timeout",
    ]) {
      expect(actionMessageTone(message), message).toBe("error");
    }
  });

  it("marks any caught backend error as an error whatever its wording", () => {
    const shown = formatUiError(new Error("enabled plugin nodes require an attached native endpoint session"), "Unable to start session.");
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
      "Draft updated. Review and plan the changes before committing.",
      "Application source changed to Discord.exe (PID 36808). Review and plan the changes before committing.",
      null,
    ]) {
      expect(actionMessageTone(message), String(message)).toBe("info");
    }
  });
});
