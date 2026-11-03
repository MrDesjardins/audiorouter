import { expect, it } from "vitest";
import { routePlaybackLabel } from "./routePlaybackLabel";

const preview = { busy: false, running: true, fresh: true, generation: 4, started: { generation: 4, preview: true } };
it("identifies a current draft preview until Stop", () => {
  expect(routePlaybackLabel(preview)).toBe("Audio running · Draft preview");
  expect(routePlaybackLabel({ ...preview, running: false })).toBe("Audio stopped");
  expect(routePlaybackLabel({ ...preview, busy: true })).toBe("Updating audio…");
});
it("does not certify saved or preview audio from stale or externally replaced runtime", () => {
  expect(routePlaybackLabel({ ...preview, fresh: false })).toBe("Audio running");
  expect(routePlaybackLabel({ ...preview, generation: 5 })).toBe("Audio running");
  expect(routePlaybackLabel({ ...preview, generation: null })).toBe("Audio running");
  expect(routePlaybackLabel({ ...preview, started: undefined })).toBe("Audio running");
  expect(routePlaybackLabel({ ...preview, started: { generation: 4, preview: false } })).toBe("Audio running");
});
