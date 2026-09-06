import { describe, expect, it } from "vitest";
import { setupChecklist } from "./setup";

describe("guided setup checklist", () => {
  it("reports readiness from authoritative observations", () => {
    const steps = setupChecklist({ connected: true, audio: "available", storage: "memory", deviceCount: 2, applicationCount: 1 });
    expect(steps.every((step) => step.state === "ready")).toBe(true);
  });

  it("distinguishes memory storage without treating it as unavailable", () => {
    const storage = setupChecklist({ connected: true, audio: "unavailable", storage: "memory", deviceCount: 0, applicationCount: 0 }).find((step) => step.id === "storage");
    expect(storage).toMatchObject({ state: "ready", detail: "In-memory storage; persistence is not durable" });
  });

  it("does not claim readiness while disconnected", () => {
    const steps = setupChecklist({ connected: false, audio: null, storage: null, deviceCount: 0, applicationCount: 0 });
    expect(steps.map((step) => step.state)).toEqual(["unavailable", "unavailable", "unavailable", "unavailable", "unavailable"]);
    expect(steps.at(-1)?.detail).toContain("own settings");
  });
});
