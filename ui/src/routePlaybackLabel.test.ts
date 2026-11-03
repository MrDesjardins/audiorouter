import { expect, it } from "vitest";
import { routePlaybackLabel } from "./routePlaybackLabel";

const preview = { busy: false, running: true, fresh: true, generation: 4, started: { generation: 4, preview: true } };
it("identifies a current draft preview until Stop", () => {
  expect(routePlaybackLabel(preview)).toBe("Draft preview");
  expect(routePlaybackLabel({ ...preview, running: false })).toBe("Route stopped");
  expect(routePlaybackLabel({ ...preview, busy: true })).toBe("Updating audio…");
});
it("does not certify saved or preview audio from stale or externally replaced runtime", () => {
  expect(routePlaybackLabel({ ...preview, fresh: false })).toBe("Route running");
  expect(routePlaybackLabel({ ...preview, generation: 5 })).toBe("Route running");
  expect(routePlaybackLabel({ ...preview, generation: null })).toBe("Route running");
  expect(routePlaybackLabel({ ...preview, started: undefined })).toBe("Route running");
  expect(routePlaybackLabel({ ...preview, started: { generation: 4, preview: false } })).toBe("Route running");
});
