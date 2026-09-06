import { describe, expect, it } from "vitest";
import { setupChecklist } from "./setup";

describe("guided setup checklist", () => {
  it("reports readiness from authoritative observations", () => {
    const steps = setupChecklist({ connected: true, audio: "available", storage: "available", deviceCount: 2, applicationCount: 1 });
    expect(steps.every((step) => step.state === "ready")).toBe(true);
  });

  it("does not claim readiness while disconnected", () => {
    const steps = setupChecklist({ connected: false, audio: null, storage: null, deviceCount: 0, applicationCount: 0 });
    expect(steps.map((step) => step.state)).toEqual(["unavailable", "unavailable", "unavailable", "unavailable", "unavailable"]);
    expect(steps.at(-1)?.detail).toContain("own settings");
  });
});
