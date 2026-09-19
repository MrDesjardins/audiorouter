# AudioRouter UI redesign plan

Status: implementation in progress, 2026-09-18

## Objective

Make the primary AudioRouter workflow understandable at a glance: choose a
source, place processors, watch signal move through the graph, and inspect a
selected processor without decoding a control-plane dashboard.

## Evidence and references

- Ambient CSS: <https://ambientcss.com/> and its documented lighting model at
  <https://kikkupico.github.io/ambientcss/guide/concept/>. Adopt the shared
  light direction, tactile surfaces, restrained depth, and hardware-control
  feel; do not add the dependency or copy its implementation.
- Local Audio Hijack references: `C:\Users\miste\Downloads\audiohijack\`.
  The screenshots show a compact source-to-destination board, cyan category
  cards, readable titles, inline VU/Peak-RMS widgets, and orange dotted
  activity paths. The EQ reference uses a large graph with visible bands and
  direct manipulation.
- Product contracts: UI-01 through UI-08, UI-11, UI-12 and M05 visual-editor
  acceptance. The backend remains authoritative; this is renderer layout and
  presentation only.

## Design decisions

1. The canvas is the product surface. The default visual hierarchy is
   `Sources → Processing → Monitor / Output`, with the library acting as a
   compact add palette rather than a second competing dashboard.
2. Every graph node has a semantic category, readable name, state chip, and
   one useful visual. Source/output/monitor nodes show a live peak/RMS meter;
   EQ nodes show a curve and draggable band points; other processors show a
   compact activity or reduction view where telemetry exists.
3. Edges communicate both configuration and runtime. A connected edge is
   quiet when stopped/silent and becomes a clearly animated, highlighted
   path when bounded backend telemetry reports signal on both ends. Reduced
   motion keeps the static active state and disables animation.
4. Selection opens the detailed inspector; the node itself stays legible and
   useful. No audio processing, graph validation, or speculative fallback is
   moved into React.
5. Use CSS custom properties for a single directional light and material
   levels. Keep dark, light, high-contrast, keyboard focus, and 200% zoom
   behavior intact.

## Ordered implementation

- [x] Record the visual findings and acceptance scope in this plan.
- [x] Replace the flat node card with category-aware cards and inline telemetry
  widgets.
- [x] Add an inline eight-band EQ preview with pointer/keyboard-accessible
  band selection and drag updates routed through the existing draft parameter
  path.
- [x] Pass diagnostics telemetry into the canvas and mark active edges from
  real bounded observations; keep static connectivity visible when silent.
- [x] Rework the canvas/library/inspector styling around the shared Ambient-like
  light tokens and Audio Hijack's compact board vocabulary.
- [x] Add focused canvas tests for EQ geometry and active-edge classification;
  meter rendering and reduced-motion behavior remain covered by the visual
  shell check below.
- [ ] Run UI typecheck, Vitest, production build, M05 acceptance, and inspect
  the rebuilt native shell at 1280×720 and maximized size. Automated checks
  pass; native shell visual inspection remains pending because the attended
  computer-use surface is currently unavailable.

## Acceptance checklist

- A first-time user can identify where audio enters, what changes it, and
  where it exits without reading backend terminology.
- Meter-bearing nodes display peak/RMS values and a visible bar even when the
  signal is silent; unavailable/stopped state is explicit.
- EQ nodes show a meaningful frequency response and band markers in the node;
  selected EQ exposes a larger graph with direct band manipulation.
- Active connections visibly pulse only when telemetry says signal is moving;
  the static route remains visible when there is no signal.
- Click/add/list/keyboard alternatives remain available; drag is optional.
- No existing backend contracts, safety defaults, privacy mute, recording arm,
  or session lifecycle behavior changes.

## Validation and rollback

Commands: `npm.cmd --prefix ui run typecheck`, `npm.cmd --prefix ui test
-- --run`, `npm.cmd --prefix ui run build`, and
`tests/acceptance/m05-ui.ps1`. Roll back by reverting the UI-only commit; the
backend graph and persisted layout formats are unchanged.

## Known limits

Current diagnostics snapshots are bounded and may be unavailable while the
session is stopped or the native stage is not prepared. The UI must say that
honestly; it must not invent a meter signal. Full Windows Narrator and scaling
evidence remains an attended acceptance task.

## 2026-09-18 usability defect follow-up

Reported defects and fixes:

1. The initial React Flow could fit before the draft nodes arrived. The canvas
   now refits after node/edge/layout changes as well as on initialization.
2. A selected physical-input node did not expose microphone selection. The
   inspector now exposes the exact active capture endpoint and writes the same
   stopped session binding used by native preparation.
3. Recorder settings were hidden from the selected-node workflow. The
   inspector now exposes the recorder format and approved output-path state,
   while making the backend-owned safe-directory restriction explicit.
4. The fixed-width inspector made EQ controls cramped. The desktop layout now
   reserves 360–420 px for the inspector and stacks its controls; it collapses
   to one column below 900 px.

Automated verification: UI typecheck and 263 Vitest tests pass after this
follow-up. Native visual confirmation remains pending until the attended shell
surface is available again.

The follow-up also makes backend state and persistence explicit. A connected
backend uses a green status dot; disconnected status remains amber. A
physical-input inspector with no active capture endpoints offers a device-list
refresh and explains that a reboot is not normally required. `Plan changes`
only validates and stages a backend plan; `Commit changes` is the explicit save
operation, including when validation returns no warnings.

The first live refresh exposed a compatibility defect: the UI sent the
optional `includeInactive` field, but the active backend schema rejects that
field. The UI adapter now uses the accepted paged `devices.list` request with
no extra parameter; the focused backend adapter tests pass.

The live inventory confirmed active render endpoints, including the Focusrite
speakers. The inspector now exposes the same exact render binding when a
physical-output node is selected, rather than requiring the separate endpoint
panel.

The output-binding screenshot also exposed a dark-theme layout defect: long
endpoint labels could force the selector beyond the inspector card and native
select text inherited an unreadable color. Binding controls now constrain
widths to the card and set explicit dark/light option colors.

Canvas viewport behavior is also stabilized: initial fit occurs once after a
session graph is available, while node drops and later layout updates preserve
the user's current pan and zoom.

The next visual pass aligns the canvas and inspector tops, replaces native
checkboxes with compact toggle controls, applies explicit control contrast,
moves layout actions outside the graph surface, removes the minimap, and makes
the library palette scroll within short canvases.

The 2026-09-18 component review removes the remaining raised/3D treatment from
inspector text, numeric, and select controls, gives node-name and EQ values a
full-width readable surface, and keeps dark/light theme contrast explicit.
EQ band dragging now owns pointer state, captures the pointer on the SVG, and
uses React Flow's `nodrag`/`nopan` affordances so a band gesture is not
interpreted as moving the entire node. Keyboard band adjustment remains
available. Native visual confirmation is still pending because the attended
desktop surface inventory is empty.

The follow-up canvas interaction pass makes Graphic EQ render ten fixed-band
points from its authoritative `bandNDb` parameters, while Parametric EQ only
renders enabled bands; both now update through the existing draft parameter
path. React Flow's automatic fit and CSS transform transition were removed
from active dragging, eliminating the main sources of node drag lag. Each
relationship now stores independent source and target edge sides in local
presentation state; the edge menu can route either endpoint to the left, right,
top, or bottom of its node without changing the node's port layout. Edge action
icons are positioned on the Bezier midpoint, and the right-side add palette is
alphabetized. Canvas edges expose compact inline disable/remove/add-processor
actions, so Canvas view no longer duplicates the same topology controls in a
list below the graph.
