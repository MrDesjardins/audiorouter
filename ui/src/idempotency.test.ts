import { describe, expect, it } from "vitest";
import { uiIdempotencyKey } from "./idempotency";

describe("UI idempotency keys", () => {
  it("produces compact distinct keys without repeating backend operation scoping", () => {
    const first = uiIdempotencyKey("session-create");
    const second = uiIdempotencyKey("session-create");
    expect(first).toMatch(/^ui-[a-z0-9-]+$/);
    expect(second).toMatch(/^ui-[a-z0-9-]+$/);
    expect(first.length).toBeLessThanOrEqual(64);
    expect(second).not.toBe(first);
  });
  it("keeps long client and operation names out of the durable key", () => {
    const key = uiIdempotencyKey("authorize-audiorouter-cowork-local".repeat(20));
    expect(key.length).toBeLessThanOrEqual(64);
    expect(key).not.toContain("authorize");
  });
});
