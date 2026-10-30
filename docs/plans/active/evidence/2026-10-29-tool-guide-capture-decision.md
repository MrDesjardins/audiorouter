# Tool-guide recording scope and framing decision — 2026-10-29

## Owner decision

After reviewing the first Gate render, the owner rejected the canvas-inclusive
framing. The reviewed clip showed the canvas and clipped the tool card, while
the Gate's live response graph and meters at the top of Properties are the
important part during playback. The owner directed us to record all tools with
the following capture rules:

- Portrait 9:16 composition, limited to the selected tool's Properties panel.
- Keep the top live visual or status area visible during the main playback.
- Briefly show a lower control when it changes, then return to the top while
  playback continues.
- Cover every user-exposed property across all 37 domain node kinds, grouping
  controls only when each change is visible or documenting why the property is
  non-audible/configuration-only.
- Preserve the existing conceptual diagrams and interactive previews.

## Accuracy boundary

For processors with an audible effect, use deterministic synthetic input and
the production engine processor with its matching telemetry. Label the
recording as real AudioRouter UI with synthetic offline audio. Do not imply
live microphone/device capture, hardware qualification, remote network flow,
or arbitrary plugin behavior. For configuration-only properties, show the
real Properties UI without claiming the setting changed sound.

## Evidence and follow-up

The initial Gate audio path was verified, but its 960×540 canvas crop was
rejected. The replacement Gate captures are portrait 720×1280, show only the
Properties panel, and briefly scroll to the changed Threshold/Hold/Range field
before returning to the live visual. Edge reported all three capture tests as
passing; the runner still hangs after reporting and is interrupted, and
Firefox playback remains unverified. Artifacts are under the ignored
`target/tool-guide-spike/` directory. See the [active execution plan](../current.md#owner-revision-after-gate-review--2026-10-29)
for detailed measurements and current validation status.

The wider all-tool inventory, clips, UI-20 contract update, accessibility,
site weight, and Firefox checks remain in progress. Do not publish generated
guide media before those gates pass.
