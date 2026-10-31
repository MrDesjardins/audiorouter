import test from "node:test";
import assert from "node:assert/strict";
import {
  familyIssues,
  loadFamilies,
  representativeKey,
} from "./tool-guide-families.mjs";

const families = loadFamilies();
const eq = families.parametricEq;
const clip = (name) => ({ video: `assets/${name}.mp4`, poster: `assets/${name}.jpg` });
const shared = { ...clip("other-bands"), property: "otherBands" };

test("Soundboard buttons map each repeated control to button 1", () => {
  for (let slot = 2; slot <= 9; slot++)
    for (const field of ["Caption", "GainDb", "Mode"])
      assert.equal(representativeKey(families.soundboard, `slot${slot}${field}`), `slot1${field}`);
  assert.equal(representativeKey(families.soundboard, "slot1Caption"), null);
  assert.equal(representativeKey(families.soundboard, "slot2MediaId"), null);
  assert.equal(families.soundboard.clip.audible, false);
});

// A two-band manifest: band 1 (representative) and band 2 (member).
function fixture(overrides = {}) {
  const representative = {
    key: "band0Q",
    status: "recorded",
    recordings: [clip("band0Q")],
  };
  const member = {
    key: "band1Q",
    status: "recorded",
    representativeKey: "band0Q",
    recordings: [shared, clip("band0Q")],
  };
  return {
    manifest: {
      tools: [
        {
          kind: "parametricEq",
          properties: [
            { ...representative, ...overrides.representative },
            { ...member, ...overrides.member },
          ],
        },
      ],
    },
    exists: overrides.exists ?? (() => true),
  };
}
const issues = ({ manifest, exists }) => familyIssues(manifest, families, exists);

test("band keys map to the band 1 field, and only band 2-16 are members", () => {
  assert.equal(representativeKey(eq, "band1Enabled"), "band0Enabled");
  assert.equal(representativeKey(eq, "band15FrequencyHz"), "band0FrequencyHz");
  assert.equal(representativeKey(eq, "band7GainDb"), "band0GainDb");
  assert.equal(representativeKey(eq, "band0Q"), null, "band 1 is the representative");
  assert.equal(representativeKey(eq, "frequencyHz"), null, "legacy keys are not members");
  assert.equal(representativeKey(undefined, "band1Q"), null);
});

test("a complete family row passes", () => {
  assert.deepEqual(issues(fixture()), []);
});

test("a member row cannot be recorded without the shared clip", () => {
  const result = issues(fixture({ member: { recordings: [clip("band0Q")] } }));
  assert.match(result.join("\n"), /band1Q is recorded without the "Other bands" clip/);
});

test("a member row must list every clip of its representative", () => {
  const result = issues(fixture({ member: { recordings: [shared] } }));
  assert.match(result.join("\n"), /does not list its representative clip assets\/band0Q\.mp4/);
});

test("a member row cannot be recorded while its representative is not", () => {
  const result = issues(fixture({ representative: { status: "pending", recordings: [] } }));
  assert.match(result.join("\n"), /band1Q is recorded but its representative band0Q is pending/);
});

test("the shared clip's files must exist", () => {
  const result = issues(fixture({ exists: (path) => !path.endsWith("other-bands.mp4") }));
  assert.match(result.join("\n"), /"Other bands" video is missing/);
});

test("a stale or missing representative key is reported", () => {
  const { manifest, exists } = fixture({ member: { representativeKey: undefined } });
  assert.match(issues({ manifest, exists }).join("\n"), /should name representative band0Q/);
});

test("pending member rows need no clips yet", () => {
  assert.deepEqual(issues(fixture({ member: { status: "pending", recordings: [] } })), []);
});

test("tools without a family are ignored", () => {
  const manifest = {
    tools: [{ kind: "gain", properties: [{ key: "band1Q", status: "recorded", recordings: [] }] }],
  };
  assert.deepEqual(familyIssues(manifest, families, () => true), []);
});
