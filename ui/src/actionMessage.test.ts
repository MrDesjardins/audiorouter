import { describe, expect, it } from "vitest";
import { actionMessageTone } from "./actionMessage";

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

  it("keeps progress and success messages neutral", () => {
    for (const message of [
      "Preparing Discord.exe audio capture...",
      "Audio session is running (generation 4).",
      "Draft updated. Review and plan the changes before committing.",
      "Application source changed to Discord.exe (PID 36808). Review and plan the changes before committing.",
      null,
    ]) {
      expect(actionMessageTone(message), String(message)).toBe("info");
    }
  });
});
