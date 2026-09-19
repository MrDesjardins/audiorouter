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
