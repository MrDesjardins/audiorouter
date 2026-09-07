import { describe, expect, it } from "vitest";
import { uiIdempotencyKey } from "./idempotency";

describe("UI idempotency keys", () => {
  it("includes the operation and produces distinct keys", () => {
    const first = uiIdempotencyKey("session-create");
    const second = uiIdempotencyKey("session-create");
    expect(first).toMatch(/^ui-session-create-.+/);
    expect(second).toMatch(/^ui-session-create-.+/);
    expect(second).not.toBe(first);
  });
});
