# M05 visual editor evidence

## 2026-09-23 refresh, playback and canvas measurement defects

Scope: UI-01/02/04/05/07/08/09/12/15; selected-session graph API behavior;
existing-device Test Signal playback. The prior dirty working tree is preserved;
no user session was migrated or replaced.

Reproduction and fixes:

- Freshly deserialized sessions triggered an effect that discarded local edits,
  selection and messages. Refresh now reconciles graph content/revision;
  external edits conflict explicitly with a dirty draft.
- Stale creation responses/snapshots could override newer committed data.
  Inventory prefers newer revisions; commit adopts its revision immediately.
  Startup preview revision is not treated as saved backend data.
- Graph planning and inspection used the startup session after a UI session
  switch. Requests now carry the candidate/selected session ID.
- Start ran the older saved graph while the canvas held unsaved changes, and
  called a simulated runtime audio success. It now refuses unsaved changes,
  checks native readiness and guides endpoint preparation. Unexpected simulated
  starts are stopped and reported. Backend status no longer claims a production
  virtual driver is required for physical/VB-Cable endpoints.
- Consecutive Audio File updates discarded the uploaded media ID while saving
  its filename. Draft updates now compose using the latest draft reference;
  imports check the active session before updating the editor.
- A repeated browser lifecycle run failed 2/3 times: nodes remained in graph
  data but were hidden and edges disappeared. Controlled React Flow nodes were
  recreated at 20 Hz without measured sizes. Keeping dimensions, selection and
  drag positions fixed it; the same run passed 3/3. The final browser suite also
  checks that each node remains visible.
- Capture privacy mute incorrectly hid measured synthetic-source flow.
  Capture-only branches remain muted; Test Signal/Audio File branches can show
  reported output levels. No backend privacy behavior changed.

Executed on Windows 11:

- `npm.cmd run typecheck`: passed.
- `npm.cmd test`: 22 files / 309 tests passed, including fresh-snapshot,
  external-conflict, commit/start, selected-session and fake-runtime regressions.
- `npm.cmd run e2e`: all 15 tests passed. The stateful in-memory fixture persists
  commits, returns fresh objects, and simulates lifecycle and changing meters.
  Assertions cover visible nodes across polling, animated dash offset, changing
  stroke width and animation stopping. This is rendering evidence, not hardware.
- `cargo test -p audiorouter-control audio_status_names_the_attached_worker_kind
  -- --exact tests::audio_status_names_the_attached_worker_kind`: 1 passed.
- `cargo test -p audiorouter-control status_snapshot_tracks_sessions_and_event_cursor
  -- --exact tests::status_snapshot_tracks_sessions_and_event_cursor`: 1 passed.
- `m05-test-signal-native-live.ps1 -AllowLiveAudio` with exact observed active
  CABLE Output/Input IDs: passed, **180 quanta**, **-18.000 dB destination peak**,
  then stop and detach. The surrounding elevated command compared Media PnP
  identities/state and endpoint inventory/default roles before and afterward:
  unchanged. Initial non-elevated PnP access failed before any stream opened.
- `npm.cmd run build`: initially EPERM replacing an existing generated asset;
  the elevated retry passed after verifying dist was the project directory.
- `cargo check --manifest-path src-tauri/Cargo.toml --locked`: passed.
- Documentation validation after evidence updates: 58 Markdown files / 277
  local links passed; `git diff --check`: passed.

Screenshots inspected in `%TEMP%/audiorouter-designer-review/`:
`playback-setup-guidance.png` and `playback-lifecycle-fixture.png`. They show
setup instructions, clear Devices selection, stable nodes and the thick
directional connection. They contain simulated meter data.

Desktop build limitation: `cargo build --manifest-path src-tauri/Cargo.toml
--locked` linked a fresh `debug/deps/audiorouter_shell.exe`, then failed to replace
the existing `debug/audiorouter-shell.exe` with Windows access denied, including
an elevated retry. The fresh executable is available as
`src-tauri/target/debug/audiorouter-shell-review-20260923.exe`. The prior shell
was not terminated or relaunched. Native attended UI/playback on the user's saved
graph, microphone/application animation, accessibility and release gates remain
open. The current adapter still needs explicitly chosen capture and render
endpoints, including for Test Signal.

Review executable SHA-256:
`A3C4BF6B072331C0586C1FF4EFFFAFED85ED268F8378B74638DADE82447296F4`.

## 2026-09-17 - continuation acceptance refresh

`tests/acceptance/m05-ui.ps1` passed in the current worktree: TypeScript
typecheck, all 19 UI test files with 257 tests, and the temporary four-file
production build. The automated matrix still covers drag/drop into physical
output, existing-virtual output through the supported physical-output branch,
processors, and recorder destinations. No audio, driver, or machine
configuration changed; attended accessibility, first-run, scaling, and live
drag/drop review remain unverified without a targetable interactive surface.

## 2026-09-17 - current automated editor acceptance refresh

`tests/acceptance/m05-ui.ps1` passed TypeScript typecheck, all 19 UI test
files, 257 tests, and a temporary four-file production build. The automated
coverage includes draft graph editing, physical/processor/recorder
destinations, and existing-virtual-output shelf drops. This remains automated
UI evidence; attended keyboard/Narrator/scaling/first-run review and live
drag-and-drop acceptance remain unverified because no targetable interactive
surface is available.

## 2026-09-17 - drag/drop evidence count refresh

`tests/acceptance/m05-ui.ps1` passed again with TypeScript typecheck, 19 UI
test files, 257 passing tests, and a disposable four-file production build.
The current evidence was corrected so older requalification entries no longer
report the superseded 246-test count. New App-level regressions exercise
physical-input, physical-output, recorder, and existing-virtual-output drops
through the real draft adapter. The existing-virtual-output entry maps to the
supported physical-output branch; managed virtual-bus nodes remain explicitly
deferred. Automated coverage still proves stopped draft placement for physical,
existing-virtual, processor, and recorder nodes;
attended accessibility, first-run, scaling, and live drag/drop remain open.

## 2026-09-17 - attended review environment unavailable

The Windows computer-use inventory returned no targetable applications or
browsers, so an attended keyboard, Narrator, scaling, first-run, and live
drag/drop review could not be performed. This is an environment limitation,
not a pass. Automated UI evidence remains separate and the attended gate stays
open.

## 2026-09-17 - editor requalification

`m05-ui.ps1` passed TypeScript typecheck, all 19 UI test files (256 tests),
and the disposable four-file production build. The stopped-draft editor
continues to cover drag/drop into physical, virtual, processor, and recorder
destinations. This is UI-only evidence; attended accessibility, packaged
shell, loaded driver, and live endpoint behavior remain separate gates.

## 2026-09-16 - full editor requalification

`m05-ui.ps1` passed TypeScript typecheck, all 19 UI test files (256 tests),
and the disposable four-file production build after the native-routing
adapter and lifecycle changes. The stopped-draft editor continues to cover
drag/drop into physical, virtual, processor, and recorder destinations. No
audio, driver, or machine configuration changed; attended accessibility,
packaged shell, and loaded-endpoint behavior remain separate gates.

## 2026-09-16 - full editor acceptance after virtual drop fix

`tests/acceptance/m05-ui.ps1` passed TypeScript typecheck, all 19 UI test
files/256 tests, and the disposable four-file production build after the
virtual shelf-drop adapter change. This requalifies the integrated graph
authoring surface, including multi-input controls, physical/virtual node
placement, processor and recorder tooling, and route planning. It does not
replace attended accessibility, packaged-shell, or loaded-driver endpoint
evidence.

## 2026-09-16 - virtual shelf-drop adapter position regression

The App canvas adapter now returns the inserted virtual render-source or
capture-sink node identity from a library drop, allowing the canvas to persist
the exact bounded drop coordinates after the explicit existing-bus prompt.
The focused `SessionFlowCanvas` and App accessibility suites passed together
(99 tests), and TypeScript typecheck passed. This proves stopped draft
authoring and presentation layout only; attended accessibility, packaged
shell, and loaded-driver endpoint behavior remain separate gates.

## 2026-09-16 - multi-input native routing controls

The visual editor now exposes the shared `nativeMultiInputs.prepare` operation
through a stopped-only panel for 2–8 exact active capture endpoints, retaining
the user’s explicit source order. It recognizes the `multi-input` diagnostics
adapter kind, reports bounded multi-input pump telemetry, and leaves branch
binding to backend session start for physical outputs, virtual sinks,
recorders, and tool observers. UI typechecking and all 245 UI tests passed;
contracts typecheck and drift validation also passed. This is editor/API
evidence, not live endpoint or driver qualification.

The complete M05 acceptance was re-run after this integration: typecheck,
19 UI test files/245 tests, and the temporary production build passed. The
acceptance changed no audio, driver, or machine configuration.

The source-order control was requalified after correcting native multi-select
ordering behavior. Existing selections are retained in committed order while
new selections are appended; the explicit reorder controls then determine the
endpoint ID sequence sent to `nativeMultiInputs.prepare`. UI typecheck, 19 UI
test files/245 tests, and the temporary production build passed.

Direct interaction coverage now selects two exact active capture endpoints,
uses the accessible move-up control, and asserts that the reordered endpoint
ID sequence reaches `prepareNativeMultiInputs`. The current M05 acceptance
passed with 19 UI test files/256 tests, typecheck, and a temporary production
build; no audio, driver, or machine configuration changed.

## 2026-09-16 - current UI acceptance requalification

`tests/acceptance/m05-ui.ps1` passed at the current tree: TypeScript
typecheck, 19 UI test files with 245 tests, and a temporary production build
of four files. The build output was temporary and the acceptance scope changed
no audio, driver, or machine configuration. This is presentation/editor
evidence; backend commit validation and native endpoint qualification remain
authoritative separate gates.

## 2026-09-17 - destination binding boundary audit

The authoritative graph schema confirms that `physicalOutput` nodes are
destination topology nodes and do not store a machine-specific endpoint ID.
Exact physical or existing-virtual endpoint identity is deliberately owned by
the stopped native endpoint/fan-out binding API, which validates active IDs,
format, ownership, and generation before opening audio. M05 shelf drops for
physical input/output, existing virtual output, processors, and recorders
therefore create valid draft destinations; the endpoint binding panel supplies
the explicit existing-device selection before activation. This preserves
portable graph import and avoids silently persisting machine-specific device
identity. Automated drag/drop coverage passes; attended live drag/drop remains
unverified.

## 2026-09-17 - current UI and contract requalification

`tests/acceptance/m05-ui.ps1` passed on the current tree with TypeScript
typecheck, 19 UI test files and 257 tests, and a temporary four-file
production build. The shared contract drift check also passed with 86 methods,
19 node kinds, 7 processors, and 20 event categories. No audio endpoint,
driver, or machine configuration was changed. This remains automated UI and
contract evidence; attended accessibility, scaling, first-run, and live
drag/drop acceptance remain unverified.

## 2026-09-17 - current UI acceptance refresh

`tests/acceptance/m05-ui.ps1` passed again on the current tree: TypeScript
typecheck, 19 UI test files with 256 tests, and a temporary four-file
production build. The temporary output was removed by the harness and no
audio, driver, or machine configuration was changed. The CUA inventory still
returned no targetable Windows applications or browsers, so attended keyboard,
Narrator, scaling, first-run, and live drag/drop acceptance remains open.

## 2026-09-17 - current automated UI requalification refresh

`tests/acceptance/m05-ui.ps1` passed on the current tree: TypeScript
typecheck, 19 UI test files with 256 tests, and a temporary four-file
production build. This confirms the current editor, typed virtual-endpoint
shelf drops, physical/processor/recorder destination draft connections, and
accessibility regressions remain green. No audio, driver, or machine
configuration changed. Attended keyboard, Narrator, scaling, first-run, and
live drag/drop behavior remain unverified because no targetable interactive
shell is available.

## 2026-09-17 - attended shell availability recheck

The already-built `src-tauri/target/debug/audiorouter-shell.exe` was launched
as a disposable visible development process and remained running, but the
authoritative computer-use inventory still returned no targetable Windows apps
or browsers. The process was terminated after the check. This confirms the
blocker is the automation surface's lack of a targetable app, not a missing
AudioRouter shell binary; attended keyboard, Narrator, scaling, first-run,
and live drag/drop acceptance remain unverified.

## 2026-09-17 - current automated UI requalification

`tests/acceptance/m05-ui.ps1` passed on the current tree: TypeScript
typecheck, 19 UI test files with 256 tests, and a temporary four-file
production build. The automated gate covers draft/editor behavior and does not
replace attended keyboard, Narrator, scaling, first-run, or live drag-and-drop
acceptance; no targetable desktop surface was available for that review.
This earlier refresh covered typed virtual-bus shelf drops through the
explicit callback; the current VB-Cable-first tree now disables those
managed-driver entries and uses the supported Existing virtual output branch
instead. The focused draft/canvas regressions verify source connections into
physical output, processor, recorder, and existing virtual destinations.
The focused draft/canvas regressions also verify source connections into
physical output, processor, recorder, and existing virtual capture destinations.

## 2026-09-16 - virtual endpoint graph connectivity regression

Added draft-graph coverage for the complete virtual endpoint connection
direction: a virtual render-source output can connect to a physical output,
and a physical input output can connect to a virtual capture-sink input. The
focused draft/canvas suite passed 32 tests; the full UI suite passed 19 files
and 245 tests with TypeScript typecheck. This proves the UI draft adapter can
represent the routes; backend commit validation and loaded virtual endpoint
behavior remain authoritative.

## 2026-09-16 - cross-layer requalification after recorder binding

The full UI suite passed 19 files/242 tests and TypeScript typecheck passed
after Recorder node selection was wired through `recorders.create`. The
locked Rust workspace and doc-tests also passed. This confirms adapter
regression coverage only; attended accessibility and real managed-driver
routing remain open.

## 2026-09-16 - recorder node binding through typed create

The Recorder panel now enumerates draft recorder node identities and forwards
the selected node as `nodeId` through the shared `recorders.create` adapter.
Without a selected graph node it retains the session-level recorder path.
The focused App/accessibility suite passed 83 tests and UI typecheck passed.
No recorder was armed or started; backend commit, worker attachment, and file
I/O remain explicit lifecycle operations.

## 2026-09-16 - virtual bus drag/drop placement

Virtual render-source and capture-sink library drops now carry the exact
canvas coordinates through the existing app adapter and persist the returned
node identity in the presentation layout, matching processor drops. The
focused canvas regression and full UI suite passed (19 files, 237 tests), with
TypeScript typecheck green. The deliberate existing-bus prompt and stopped
draft/plan boundary remain intact. Manual visual/accessibility observation and
loaded-driver routing remain open; no audio or machine configuration changed.

## 2026-09-16 - explicit endpoint invalidation recovery notice

The live event loop now presents a bounded, actionable message when the
backend publishes `devices.bindingInvalidated`: audio is stopped, exact
endpoints must be reviewed, and the user must deliberately rebind before
restarting. The UI still refreshes authoritative state through the existing
snapshot path. UI tests passed at 234 and TypeScript typecheck passed. Manual
WebView2 visual/accessibility acceptance remains open because no targetable
desktop surface was available; no machine audio configuration changed.

## 2026-09-16 - explicit endpoint rebind UI

The native endpoint panel now exposes a stopped-only “Rebind exact endpoints”
action, forwarding the selected session and exact capture/render IDs through
the typed backend adapter. UI tests cover both request forwarding and visible
success feedback; the full UI suite passed 233 tests and TypeScript typecheck
passed. No endpoint or persistent machine configuration was changed by this
portable UI validation.

## 2026-09-16 - current visual editor requalification

`tests/acceptance/m05-ui.ps1` passed TypeScript typecheck, 227 tests across 19
files, and a temporary Vite production build transforming 214 modules. The
covered UI contracts include the drag/drop processor shelf, keyboard
connection dialog, graph mutations, selection, deletion, and accessibility
behavior. Temporary build output was removed; no audio, driver, or machine
configuration was accessed. Manual visual/accessibility observation remains
open because the desktop automation surface is not targetable.

## 2026-09-15 - scaled canvas layout correction

Anchored the React Flow canvas toolbar to the lower canvas edge instead of
leaving its absolute position dependent on browser static positioning. The
toolbar now wraps, and the processor shelf expands within narrow canvases;
light and high-contrast themes retain explicit panel boundaries. M05
typecheck, all 226 UI tests across 19 files, and a temporary 214-module Vite
production build passed. The existing non-failing bundle-size warning remains.
No audio, driver, or machine configuration was accessed.

## 2026-09-15 - retain one node in editable drafts

The shared draft mutation now rejects removal of the final node, and both the
inspector and canvas deletion paths explain that a second node must be added
first. This keeps the UI selection and graph draft valid before backend Plan
validation. The complete UI suite passed with 221 tests, TypeScript typecheck
passed, and the elevated production build passed for 214 modules (with the
existing chunk-size warning). No audio or machine configuration was changed.

## 2026-09-15 - canvas node deletion

Editable canvas nodes can now be selected and deleted with the Delete key.
The action asks for confirmation and removes the node plus its incident edges
from the local draft only; disconnected/read-only canvases do not enable the
action, and the authoritative backend still validates the subsequent
plan/commit. The complete UI suite passed (19 files, 220 tests), TypeScript
typecheck passed, and the production build passed. No audio or machine
configuration was accessed.

## 2026-09-15 - direct canvas connection removal

The visual editor now exposes draft edges as deletable React Flow edges. With
an editable connected backend, selecting an edge and pressing Delete routes
its identity through the existing draft-removal callback; disconnected or
read-only canvases do not enable deletion. The authoritative graph is still
unchanged until the existing plan/commit flow succeeds. The focused canvas
test passed (3 tests), the complete UI suite passed (19 files, 219 tests),
typecheck passed, and the production build passed. No audio or machine
configuration was accessed.

## 2026-09-13 - first-run route navigation

The connected UI now presents an accessible three-step Quick route panel:
select exact endpoints, build the visual graph, then start the session. Each
step links to the existing authoritative panel; no duplicate action or
automatic endpoint selection was added. UI typecheck and the complete suite
passed: 18 files, 153 tests. Manual visual/accessibility observation remains
open because no targetable desktop surface was available.

The pushed-head M05 acceptance also passed the TypeScript typecheck and a
disposable three-file Vite production build. No audio, driver, or machine
configuration was changed.

## 2026-09-12 - UI acceptance requalification

The M05 acceptance passed with TypeScript typecheck, all 17 UI test files and
128 tests, and a disposable production Vite build transforming 213 modules.
The runner removed its three temporary output files. This is UI-only evidence;
no audio endpoint, driver, or persistent machine configuration was accessed.

## 2026-09-08 - Application contract requalification

After the application inventory gained nullable `executablePath`, the UI
package passed TypeScript typecheck, all 91 Vitest tests across 14 files, and
a temporary production build. The temporary output was removed after
verification; no backend, audio, or machine configuration was changed.

## Initial React/Vite shell

The new `ui` package is a React/Vite/TypeScript application that consumes the
published local `@audiorouter/contracts` package. It provides a responsive
session sidebar, signal-flow node cards, source/effect/output library, and
recording panel. Node cards and controls are keyboard focusable, the layout
has a responsive list-friendly fallback, and the initial CSP allows only local
scripts/styles and same-origin connections.

The shell deliberately renders a disconnected/read-only state: it cannot apply
routes, access devices, or arm recording until a transport bridge is provided.
It also communicates that monitoring starts muted and recording starts
unarmed. `npm run build` passes, including TypeScript checking and Vite output;
the production-only dependency audit reports no vulnerabilities. Native shell,
transport wiring, live snapshots/events, graph editing, and accessibility
manual testing remain open.

## 2026-09-06 — Event cursor refresh safety

The connected React editor now seeds its event cursor from the initial
authoritative snapshot. When an event requires a refresh, it advances the cursor
only after that snapshot succeeds; a failed refresh therefore retries the same
event range instead of silently treating it as consumed. The UI suite passes 20
tests with TypeScript typecheck and production build green. Native transport and
manual visual acceptance remain open.

The shell now supports local presentation-only node selection through mouse or
keyboard Enter/Space. The selected card and inspector are exposed with
accessible labels and `aria-current`; mutation controls stay disabled while
disconnected and explain that a backend connection is required. This preserves
the rule that the UI cannot duplicate or bypass graph authority. The
production build and full dependency audit pass.

## Typed backend snapshot seam

`ui/src/backend.ts` now defines the UI-facing `UiBackend` boundary and a
`UiBackendSnapshot` containing status, discovery, and the selected session.
`createLiveBackend` uses the shared typed `AudioRouterClient` for the three
read-only protocol calls needed to hydrate that snapshot. The default
`createDisconnectedBackend` returns only local fixture data and exposes no
mutation surface, preserving safe disconnected startup. The React shell now
uses that backend state for its connection label. TypeScript checking and the
production Vite build pass.

The shell now hydrates its selected-node view from the backend snapshot on
mount, with local fixture data as the safe initial state. Snapshot completion
is guarded against an unmounted React tree; no mutation or device call is
introduced. TypeScript checking and the production Vite build pass.

The disconnected preview now exposes a typed session picker backed by the
same session-shaped fixtures used by the snapshot seam. Switching sessions
updates local presentation state only; it does not call a control method or
alter audio configuration. TypeScript checking and the production Vite build
pass.

Added presentation support for Windows accessibility preferences: reduced
motion disables transitions/animations, and forced-colors mode uses system
button colors plus non-color focus/selection outlines. The session picker is
also styled as a full-width labeled control. This remains renderer-only;
TypeScript checking, UI tests, production build, and high-severity audit pass.

`SnapshotCache` now retains the last successful `UiBackendSnapshot` and marks
it stale with an actionable error when refresh fails. A failed reconnect cannot
erase the last known session or create an unbounded edit queue; a later
successful refresh clears the stale state. This is transport/UI state only and
does not touch audio configuration. Contracts/UI typechecks, production build,
and high-severity audit pass.

The shared TypeScript contract now models `StateEvent` and the complete
`events.subscribe` result, including backend epoch, sequence, filtered events,
and explicit resync snapshots. `UiBackend.subscribe` exposes this read-only
replay path; the disconnected implementation returns an empty cursor and the
live adapter requests the bounded 500-event replay. Contracts and UI
typechecks, production build, and high-severity audit pass.

`ui/src/draft.ts` adds plan-only candidate editing for node enabled/bypass
flags and deterministic draft-change descriptions. It clones session data,
preserves the authoritative revision, rejects unknown node IDs, and leaves
validation/commit to the backend. This is UI preparation only and is not
wired to disconnected controls. TypeScript checking and the production Vite
build pass.

The TypeScript contract now models `RouteInspection` and `RoutePath` from the
authoritative Rust route explanation, and `UiBackend.inspectRoute` exposes the
read-only destination query. The disconnected adapter returns no invented
paths; the live adapter forwards the selected session and destination node.
Contracts/UI tests, typechecks, production build, and high-severity audit pass.

The React shell now consumes `SnapshotCache` state and exposes an accessible
status message when the retained backend snapshot is stale. The initial
disconnected preview still renders safely, while a future failed live refresh
will retain the last known session and explain the condition. UI typechecking
and production build pass.

The shared TypeScript contract now defines recording-library row/state shapes
matching the durable storage fields, including WAV/FLAC format, audio shape,
frame/byte counts, metadata, missing-file state, and terminal failure state.
This is schema groundwork only; no renderer file deletion or recording action
is exposed. Contracts/UI tests, typechecks, production build, and audit pass.

The status strip now renders typed snapshot facts for audio availability,
storage mode, and session count alongside the disconnected/connected state.
The values are backend-derived (or the explicit safe preview snapshot), so the
UI does not infer readiness from the presence of controls. UI tests, typecheck,
production build, and high-severity audit pass.

Added Vitest fake-backend coverage in `ui/src/backend.test.ts`: four tests
verify disconnected read-only snapshots, empty event cursors, stale snapshot
retention after a failed refresh, revision-preserving node drafts, and unknown
node rejection. `npm run test`, typecheck, production build, and high-severity
audit all pass.
The UI backend now exposes typed `planGraph` and `commitGraph` operations over
the shared client. Live adapters send the candidate's authoritative session
revision to `graph.plan` and pass the returned plan data to `graph.commit`;
the disconnected adapter rejects both operations with an actionable error.
Vitest (4 tests), TypeScript typecheck, and the production build pass. This is
an adapter seam; the rendered editor does not yet invoke graph mutations.

## 2026-09-06 — Current UI validation

The UI test suite passed all 20 tests at the current revision. TypeScript
typechecking and the Vite production build also passed, with local dependencies
only and no backend, audio endpoint, or machine configuration access. Manual
visual/accessibility acceptance and host-provided live transport injection
remain open.

The entry point now auto-selects this transport when a WebView2 host exposes
`chrome.webview` and injects a nonempty `window.__AUDIO_ROUTER_SESSION_ID__`.
An explicit `window.__AUDIO_ROUTER_HOST__` bridge still takes precedence, and
missing or malformed host inputs remain disconnected. Focused coverage verifies
WebView2 selection and invalid-session fallback; the native host's origin and
permission policy plus manual visual/accessibility acceptance remain open.

`applyGraphDraft` now provides the two-phase UI workflow: it plans the complete
candidate, verifies the returned base revision matches the draft, and commits
using the caller's idempotency key. Revision mismatch is rejected before
commit. UI tests now cover six cases; typecheck and production build pass.

The React shell now accepts an injected `UiBackend` and owns its
`SnapshotCache` instance, allowing the eventual live transport to be supplied
without changing the presentation components. The Reconnect action performs a
bounded snapshot refresh and preserves stale-state behavior on failure. UI
tests (6) and the production build pass.

The rendered editor now wires the existing draft helpers into the connected
backend: enabled and bypass flags are editable as local drafts, Undo restores
the authoritative session snapshot, and Plan changes performs the two-phase
plan/commit workflow with visible success or error status. The disconnected
adapter keeps the controls and commit actions disabled, preserving the safe
read-only preview. Vitest (6 tests), TypeScript typecheck, production build,
and diff checks pass.

Connected UI sessions now maintain a bounded event cursor and poll the shared
`events.subscribe` adapter once per second. The known backend epoch and replay
cursor are forwarded; any event or `resyncRequired` result triggers an
authoritative snapshot refresh, while failures retain the last view as stale.
The disconnected adapter does not poll or mutate. The adapter regression,
seven UI tests, typecheck, production build, and diff checks pass.

The rendered editor now exposes a read-only Route inspection panel for the
selected node. It calls `UiBackend.inspectRoute`, reports the backend's
reachable/path result, and shows an explicit not-loaded or unavailable state
without inferring edges or destinations. The disconnected backend returns no
route. Seven UI tests, typecheck, production build, and diff checks pass.

The connected editor now exposes a bounded Gain dB input for the selected Gain
node. It edits only the local candidate, remains disabled in disconnected
preview mode, and emits a deterministic /nodes/<index>/parameters/gainDb
change for the existing plan/commit flow. UI tests pass 8 cases with TypeScript
typecheck and the production build.

The selected Mute node now exposes its graph-level `muted` parameter as a
connected-only draft checkbox. This is distinct from the process privacy-mute
safety action, which remains unavailable in the editor; the parameter still
flows through the existing plan/commit path. UI coverage is 9 tests with
typecheck and production build passing.

Draft graph edits now use a bounded 20-entry undo/redo history. Undo and redo
restore complete candidate snapshots, a new edit clears the redo branch, and
switching sessions/discarding a draft resets history. Pure history regressions
cover round trips, redo invalidation, and bounds; UI coverage is 39 tests with
typecheck and production build passing. These actions never call the backend or
alter live audio.

The library now includes guided templates for Gaming + Discord, Processed
Microphone, and Mix-minus conversation. Loading one replaces only the local
candidate with an independent, stopped, inspectable graph; explicit edges and
channel maps are shown for review, while device bindings still require review
and backend plan/commit. Template regressions cover matrix shape, independent
snapshots, and distinct names; UI coverage is 41 tests with typecheck and
production build passing.

The editor now presents a guided readiness checklist derived from backend
connectivity, status snapshots, endpoint inventory, and application
observations. It distinguishes ready, attention-needed, and unavailable states
and explicitly tells users to select Discord/OBS through those applications'
own settings. Two pure checklist regressions were added; UI coverage is 54
tests with typecheck and production build passing. The checklist performs no
audio, endpoint, or external-application action.

The readiness checklist now aligns with the typed status contract by treating
both `memory` and `sqlite` storage as available, while labeling memory storage
as non-durable. This prevents a valid backend state from being reported as
unavailable; a focused regression was added and UI coverage is 55 tests with
typecheck and production build passing.

Route explanations now annotate each reported node with its authoritative
enabled, bypassed, muted, or disabled state. Unknown node IDs remain visible
instead of being silently dropped. A pure formatter regression was added; UI
coverage is 50 tests with typecheck and production build passing. Inspection
remains read-only and does not infer or alter topology.

Canvas node cards now expose every port's direction, role, and channel count,
so users can inspect connection endpoints without relying on proximity or
color. A pure port-label regression was added; UI coverage is 49 tests with
typecheck and production build passing. The canvas still cannot commit or
activate a connection directly.

The editor now exposes dark, light, and high-contrast themes through an
accessible select control. The preference is browser-local, invalid stored
values fall back to dark, and storage failures are non-fatal. Existing forced
colors and reduced-motion media behavior remains supported; two pure preference
regressions were added and UI coverage is 48 tests with typecheck and
production build passing.

The canvas toolbar now provides a Reset layout action. It removes only the
selected session's browser-local presentation coordinates and restores the
automatic layout; it does not alter graph topology, backend state, or audio
configuration. A pure regression covers the clear operation; UI coverage is 42
tests with typecheck and production build passing.

The structured graph-list alternative now exposes the same draft connection
enable/disable/remove operations as the canvas view. This keeps essential graph
editing keyboard/select accessible while retaining the connected-backend gate
and plan/commit boundary. UI coverage remains 46 tests with typecheck and
production build passing.

The node library now supports bounded local search across entry names,
categories, and unavailable-capability explanations. Search never changes the
draft; supported processors retain the connected-only add action and unavailable
entries remain explicit when matched. Two pure filtering regressions were
added; UI coverage is 46 tests with typecheck and production build passing.

Selecting a node now highlights its complete enabled upstream/downstream
component in the canvas and dims unrelated nodes and edges. Disabled edges do
not participate in the visual path, and the traversal is presentation-only;
topology and backend state are unchanged. Two pure graph-view regressions were
added; UI coverage is 44 tests with typecheck and production build passing.

The structured graph list now includes each node's direction, port role, and
channel count alongside its kind and enabled state. This keeps endpoint
inspection equivalent for keyboard/select users without requiring the canvas;
the same 49-test UI suite, typecheck, and production build pass.

The inspector now supports explicit removal of the selected node from the
draft after confirmation. Its incident edges are removed from the candidate,
while the session revision and committed state remain unchanged until backend
planning and commit; disconnected mode disables the action. UI coverage is 35
tests with typecheck and production build passing.

The inspector now supports duplicating the selected node into the draft. The
copy receives a deterministic unique identity and cloned parameters/ports but
no implicit edges, preventing accidental rerouting. The committed session and
revision remain unchanged until backend plan/commit; disconnected mode disables
the action. UI coverage is 36 tests with typecheck and production build
passing.

Draft connections in the canvas view can now be explicitly enabled or
disabled without deleting their topology. The text state and control action
remain local to the candidate until backend planning/commit, and unknown edge
IDs are rejected by the shared draft helper. UI coverage is 37 tests with
typecheck and production build passing.

The recording library Search control is now functional as a bounded local
filter over authorized metadata (identity, title, artist, comment, and path).
It reports filtered counts and an explicit no-match state, while never opening
recording files or changing backend state. UI typecheck, tests, and production
build pass.
## 2026-09-06 â€” Host transport adapter seam

The UI now exposes `createLiveBackendFromTransport`, which builds the shared
typed `AudioRouterClient` from a host-provided framed `RpcTransport` before
creating the live backend. React remains independent of named-pipe and native
WebView details, while the disconnected backend remains the safe default. Ten
Vitest tests, TypeScript typecheck, and the production Vite build pass.
## 2026-09-06 â€” Shared API contract parity

The TypeScript contract map now includes the implemented operations-cancel,
recording library/recovery/file-action, privacy-mute, startup, and event-epoch
parameter/result surfaces. This prevents UI host adapters from falling behind
Rust discovery. The contracts package typechecks; the dependent UI passes 10
tests, typecheck, and production build.

The rendered recording panel now requests the authorized session-scoped
`recordings.list` result and displays persisted title, state, missing status,
and path metadata. Disconnected mode returns an empty list and performs no
mutation. UI coverage is 11 tests with typecheck and production build passing.

Recording query failures are now rendered as an explicit unavailable state,
rather than being confused with an empty library. This preserves actionable
permission/backend feedback while retaining the safe disconnected empty state.

Each connected recording row now offers a read-only Preview action backed by
`recordings.preview`. The UI reports the returned status and never decodes,
opens, or modifies recording bytes; disconnected mode rejects the operation.
UI typecheck, 11 tests, and production build pass.

The UI backend regression also verifies that a live transport forwards the
recording ID to `recordings.preview` and preserves the read-only response. UI
coverage is now 12 tests.

The shared TypeScript `graph.commit` parameters now include the nullable
optional `acknowledgments` array, matching the Rust schema's bounded warning
ID validation. Contracts typecheck and the dependent UI test/typecheck/build
checks remain green.

The graph editor now preserves that safety boundary in its interaction flow:
it plans before committing, renders each returned warning as a required
acknowledgment, and sends the acknowledged warning IDs only after the user
explicitly checks every item. A warning plan cannot be committed by the normal
single-click path. UI coverage is 13 tests; typecheck and the production build
pass. No audio endpoint or machine configuration is touched.

The connected inspector now exposes the durable `safety.setPrivacyMute` latch
as an immediate, explicit safety action. It starts fail-closed as muted,
remains disabled while disconnected, reports success/failure, and routes only
through the authorized API; it does not change Windows privacy settings or
audio defaults. UI coverage is 14 tests with typecheck and production build
passing.

The selected-node inspector now supports bounded draft renaming. Empty names
and names over 120 characters are rejected locally with an actionable status,
while valid names preserve node identity, session revision, and graph edges
until the authoritative plan/commit flow. UI coverage is 34 tests with
typecheck and production build passing.

Recording rows now support bounded title editing through the authorized
`recordings.setMetadata` API. The UI updates its local row only after backend
success and explicitly reports that the audio path/content remain unchanged;
the action is disabled while disconnected. UI coverage is 17 tests with
typecheck and production build passing.

The canvas `List view` control is now functional. It presents keyboard-focusable
nodes with selection state plus an explicit connection list resolving node and
port names from the current draft; empty edges are reported distinctly. The
view remains presentation-only and cannot rewire audio. UI tests, typecheck,
and production build pass.

Connected recording rows now also offer an explicit-confirmation
`recordings.removeEntry` action. The UI removes only the library row after the
authorized backend succeeds and reports that the underlying audio file was
preserved. Disconnected mode keeps the action disabled. UI coverage is 15
tests with typecheck and production build passing.

Route inspection now presents the backend result as “Receives audio from,”
resolving node IDs to current names and displaying edge counts plus channel
maps for each reported path. It remains read-only and does not infer or alter
connections. UI tests, typecheck, and production build pass.

Connected recording rows now expose read-only recovery inspection through
`recordings.recovery`. The UI reports whether a validated lifecycle checkpoint
exists and its state, while disconnected mode keeps the action disabled; no
audio payload, file handle, or machine configuration is accessed. UI coverage
is 16 tests with typecheck and production build passing.

The session sidebar now renders the available session resources rather than
hard-coded entries. Dropdown and keyboard-focusable buttons share one selected
session state, expose `aria-current`, and show each session's revision; the
create control remains disabled until a connected backend is available. UI
typecheck, tests, and production build pass, with no lifecycle/audio action
performed by navigation.

The connected session sidebar `+` action now creates a revision-0 stopped
session through `sessions.create` using an explicit user-provided name. The
new resource is added to the local view only after the authoritative response;
offline mode keeps creation disabled, and creation never starts audio. The
shared TypeScript result contract now accurately models `{ session, state }`.
UI coverage is 18 tests with typecheck and production build passing.

The connected editor now offers session duplication through
`sessions.duplicate`. It creates a fresh revision-0 stopped resource, adds it
to the UI only after the authoritative response, and leaves lifecycle/audio
state untouched. The shared contract now models the `{ session, state }`
response; UI coverage is 19 tests with typecheck and production build passing.

The selected session name is now an editable bounded draft in the connected
inspector. It travels through the existing graph plan/commit flow, so revision
conflicts and backend validation remain authoritative; disconnected mode keeps
the field read-only. UI tests, typecheck, and production build pass.

The connected editor now offers confirmed deletion of the selected stopped
session through `sessions.delete`. The UI changes selection only after the
authoritative response; disconnected mode disables the action, and the
backend's active-resource protection remains in force. UI coverage is 20
tests with typecheck and production build passing.

The connected editor now displays the backend's persisted crash-recovery
status and exposes `recovery.clearSafeMode` only when the backend is connected
and reports an active latch. Disconnected preview includes an explicit
in-memory normal state, keeps the clear control disabled, and rejects direct
clear calls. The live adapter forwards the exact no-parameter RPC request.
UI coverage is 21 tests; typecheck and production build pass. This operation
clears recovery markers/latch state only and does not start audio or alter
machine configuration.

Diagnostics parity was also corrected: a prepared or running multi-input
worker now reports its adapter kind and exact session ID with the same status
semantics as other native workers. The focused control regression, strict
Clippy, and UI typecheck passed.

The UI status contract now includes the backend's privacy-mute persistence and
effect metadata. Connected snapshot refreshes synchronize the visible safety
latch from that authoritative value instead of relying on stale renderer
state; disconnected preview remains fail-closed and in-memory. UI coverage is
21 tests with typecheck and production build passing.

The application inventory panel now refreshes alongside reconnect and
event-triggered snapshot refreshes. This keeps the read-only Windows audio
session observations current in the editor without enabling any capture or
route mutation; disconnected mode continues to return an empty inventory.

Session list and workspace headings now use `status.get.activeSessionIds` to
label the selected and listed sessions as Running or Stopped. The UI no longer
claims a running authoritative session is stopped; disconnected preview still
uses its safe stopped fixture state.
The UI now includes a read-only Windows endpoint inventory backed by the typed
`devices.list` API. Connected refreshes request at most 500 descriptors and
render direction, sample rate, channel count, and default period; disconnected
startup remains empty and safe. The adapter does not start streams or change
endpoint configuration. UI coverage remains green at 24 tests with typecheck
and production build validation.

Typed recording, privacy, and session result contracts exposed stale UI adapter
signatures and obsolete preview/recovery field access. The adapter now consumes
the shared result variants directly; UI typecheck, all 24 Vitest tests, and the
production build pass. No audio stream or file action is invoked by this
adapter correction.

The UI now exposes guarded session lifecycle actions through the same typed
backend adapter: connected mode forwards `session.start` and `session.stop`,
while disconnected preview rejects both operations. The control is accompanied
by a forwarding regression; 25 UI tests, typecheck, and the production build
pass. The current control runtime is fake, so this slice opens no audio stream.

The UI backend adapter now also exposes explicit recording recycle forwarding
with a required confirmation value and typed preview/result variants. It is not
automatically invoked by the editor; disconnected mode rejects it, and the live
adapter regression checks only the request shape. UI coverage is 26 tests with
typecheck and production build validation; no file action was performed.

The visual editor now renders the session graph with a repository-local React
Flow canvas. It maps nodes and edges from the authoritative session draft,
highlights the selected node, and exposes fit-view, pan/zoom, and minimap
presentation controls while keeping nodes and connections non-editable. The
existing list view remains available as a keyboard-friendly alternative. UI
tests (26), TypeScript typecheck, production build, and diff checks pass; no
graph commit, stream activation, or machine configuration change is performed
by the canvas.

The library now distinguishes available built-in processors from unavailable
device/runtime capabilities. Gain, mixer, mute, and meter entries append valid
nodes with deterministic IDs to the local draft; physical input and recorder
entries are disabled with explanatory labels until their required native/runtime
integration exists. The added node never changes the authoritative revision or
edges and still requires the existing plan/commit flow. UI coverage is 28
tests; typecheck, production build, and diff checks pass.

Topology editing now has a non-drag path: the editor derives output/input
choices from the draft, creates deterministic edge IDs and bounded identity or
mono-to-stereo matrices, and reports duplicate or occupied non-mixer inputs
before a plan is submitted. The operation changes only the local draft; graph
cycle and remaining semantic validation stay in the backend. UI coverage is 30
tests with typecheck, production build, and diff checks passing.

The canvas view also exposes explicit removal controls for draft connections.
Removing an edge updates only the local candidate, reports the action to the
user, and leaves the committed graph unchanged until the existing plan/commit
step. The structured list view remains available; UI coverage is 31 tests with
typecheck, production build, and diff checks passing.

Canvas node positions are now draggable and persist separately from audio
topology under a per-session browser-local key. Malformed or unbounded stored
coordinates are discarded, persistence failures are non-fatal, and positions
are never sent to the backend graph planner. Layout persistence has two pure
regressions; UI coverage is 33 tests with typecheck and production build
passing.

The selected-node inspector now offers a reset for supported processor
parameters: gain returns to 0 dB and mute returns to off. The reset updates
only the local candidate and enters the bounded draft history; node identity,
topology, revision, and live audio remain unchanged until backend plan/commit.
UI coverage is 51 tests with typecheck and production build passing.

Session-name editing now trims valid values and rejects empty or overlong names
before they enter the local candidate. The session ID and revision remain
unchanged until the backend plan/commit flow; a pure regression covers the
normalization and 120-character bound. UI coverage is 52 tests with typecheck
and production build passing.

## Host-safe Vitest configuration (2026-09-06)

The UI test script now passes Vitest's `--configLoader runner` option. This
avoids Vite's bundled-config temporary write under `node_modules/.vite-temp`,
which is denied by the current Windows Application Control policy, while
retaining the existing TypeScript Vite configuration. The full UI suite passes
58 tests and the UI TypeScript check passes; no application or audio state is
changed.

The production `build` script also passes Vite's `--configLoader runner`, so
the bundled-config temp write is avoided. TypeScript checking and a Vite
production build directed to a disposable temporary directory pass (three
files generated). The current host still denies writing/deleting the existing
`ui/dist` tree, so a normal default-output build remains blocked by that
filesystem policy; the temporary output was removed and no audio/application
state changed.

The checked-in `tests/acceptance/m05-ui.ps1` now automates this host-safe
verification. It passed UI typecheck, all 58 Vitest tests, and a disposable
three-file production build, then removed the temporary output. The script
does not touch audio, driver, or machine configuration.

## Gain range guard (2026-09-06)

The local draft editor now mirrors the authoritative -60 to +24 dB gain range
and rejects non-finite or out-of-range gain values before they enter a draft.
Boundary and rejection regressions pass; UI typecheck and all 59 Vitest tests
pass. The single-line inspector markup now advertises the same +24 dB maximum;
backend and draft validation remain authoritative.

The M05 acceptance wrapper was requalified at clean revision `e34438f`.
TypeScript typecheck and all 59 Vitest tests passed, followed by a disposable
three-file Vite production build. Temporary output was removed and no audio,
driver, or machine configuration was changed.

The typed UI backend now forwards the existing explicit `recordings.rename`
operation with its recording ID and requested destination path. A regression
verifies the method and payload, while the disconnected backend remains
fail-closed. UI typecheck and all 60 tests pass; no recording file was changed.

The typed UI backend also forwards `recordings.reveal` without opening the
path itself. A regression verifies the method and recording ID payload using a
missing-file result, and the disconnected adapter remains fail-closed. UI
typecheck and all 61 tests pass; no file or Explorer action was performed.

The complete `tests/acceptance/m05-ui.ps1` wrapper was requalified at the
current revision. TypeScript typecheck, all 61 Vitest tests, and a disposable
three-file Vite production build passed; the temporary output was removed.
Manual visual/accessibility acceptance and native shell injection remain open.

The complete M05 acceptance was requalified at the current head. TypeScript
typecheck, all 69 UI tests, and the disposable three-file Vite production build
passed; temporary output was cleaned. Manual visual/accessibility acceptance,
native shell injection, startup registration, and live audio remain open.

## Current-tip UI qualification (2026-09-07)

At the current head, TypeScript typechecking passed, all 69 Vitest tests passed, and a
disposable Vite production build produced three files. The temporary output was removed;
this did not access audio devices, drivers, startup registration, or machine configuration.
No audio, driver, or machine configuration was changed.

The editor also renders the managed virtual-device inventory and explicit
desired-state plan/apply controls. These controls preserve the unavailable
driver warning and `deviceAdministration` boundary and never synthesize or
activate endpoints. The M05 typecheck, 61-test suite, and disposable
production build remain green.

## Explicit recording file actions (2026-09-06)

The rendered UI now includes a connected-only recording action panel for the
existing authorized `recordings.rename`, `recordings.reveal`, and
`recordings.recycle` operations. Rename accepts an explicit destination and
continues to rely on the backend's approved-directory check. Reveal reports a
missing file without opening anything, while recycle exposes a non-mutating
preview separately from the confirmation-required action. All controls are
disabled while disconnected. UI typecheck, all 61 tests, and the disposable
three-file production build pass; no recording file or machine configuration
was changed.

## Native host transport injection boundary (2026-09-07)

The UI entry point now consumes an optional preloaded
`window.__AUDIO_ROUTER_HOST__` value containing a typed `RpcTransport` and
non-empty session ID. Valid injection constructs the existing live UI backend;
missing or malformed injection fails closed to the disconnected read-only
preview. Unit coverage verifies both paths. This establishes the browser-side
injection contract without executing native shell code or registering startup
behavior. Production host ownership of the injected object and manual visual/
accessibility acceptance remain open.

## Bounded WebView2 JSON-RPC transport (2026-09-07)

The UI now includes `WebView2RpcTransport`, which adapts the host's
`chrome.webview` message surface to the existing typed `RpcTransport`
contract. Requests are wrapped as `audiorouter.rpc.request` messages and only
matching `audiorouter.rpc.response` IDs resolve them. Pending requests are
bounded at 64 by default, each request has a bounded timeout, duplicate IDs
are rejected, unrelated messages are ignored, and disposal rejects all
pending work. Four focused tests cover correlation, malformed messages,
duplicate IDs, and disposal. This is a browser-side boundary implementation;
the native WebView2 host, origin policy, and manual visual/accessibility run
remain open.

## Session inventory failure safety (2026-09-07)

Connected session inventory failures now clear the listed-session set and show
an explicit unavailable status instead of retaining disconnected demo sessions
as if they were backend resources. The disconnected preview still uses local
fixtures, and known point-in-time/created sessions remain available. M05
typecheck, 73 UI tests, and the disposable production build passed.
## 2026-09-06 — Current-tip UI acceptance

The complete M05 acceptance was rerun at the current tip. TypeScript
typechecking passed, all 63 Vitest tests passed, and the disposable Vite
production build produced three files. The temporary output was removed; this
was UI-only validation and did not access audio, drivers, or machine
configuration. Manual visual/accessibility acceptance and native shell
injection remain open.

Removed the unused legacy `VirtualDevicePanel` component so the connected UI
has one authoritative virtual-device lifecycle surface. The M05 typecheck,
all 63 Vitest tests, and disposable production build still pass.
## UI mutation idempotency completion (2026-09-06)

All connected UI mutation paths now use the shared UUID-backed idempotency-key
helper: graph commit, virtual-device apply, recording-entry removal, and
session create, duplicate, and delete. Entity IDs remain separate from retry
keys. UI typecheck, all 63 Vitest tests, and the disposable three-file Vite
production build pass; no audio, driver, recording file, or machine
configuration was accessed.

## 2026-09-14 - Telemetry inspector acceptance

The M05 acceptance wrapper requalified the current UI after the node telemetry
inspector and 1 Hz diagnostics refresh were added. Contracts/UI typecheck, all
18 Vitest files (167 tests), and a disposable Vite production build (3
artifacts) passed. The output was temporary and removed by the wrapper. Manual
WebView2/Narrator acceptance remains open; no audio endpoint or machine
configuration was accessed.
## Idempotency forwarding regression (2026-09-06)

The UI adapter test suite now explicitly asserts that retry keys reach the
shared API for recording-entry removal and session create, duplicate, and
delete operations. TypeScript typecheck, all 63 Vitest tests, and the
disposable three-file production build pass; no audio, recording file, or
machine configuration was accessed.
## Idempotency-key entropy hardening (2026-09-06)

The shared UI mutation-key helper now uses `crypto.randomUUID()` when
available, `crypto.getRandomValues()` as the browser fallback, and a
timestamp/monotonic counter only when cryptographic APIs are unavailable. The
M05 typecheck, 63-test suite, and disposable production build pass; no audio or
machine configuration was accessed.
## Direct idempotency helper coverage (2026-09-06)

The retry-key helper is now an isolated UI module with a regression verifying
operation scoping and distinct keys for repeated attempts. M05 typecheck, all
64 Vitest tests, and the disposable three-file production build pass; no audio
or machine configuration was accessed.

## Current-tip acceptance after startup API expansion (2026-09-07)

The complete M05 acceptance was rerun at the current tip. TypeScript
typechecking passed, all 69 Vitest tests passed, and the disposable Vite
production build produced three files. The temporary output was removed; this
was UI-only validation and did not access audio devices, drivers, startup
registration, or machine configuration. Manual visual/accessibility acceptance,
native shell injection, startup registration, and live audio remain open.

## Session inventory authority regression (2026-09-07)

The UI session inventory merge now gives the point-in-time `sessions.get`
snapshot precedence over a stale duplicate from `sessions.list`, while
preserving other listed sessions and locally created sessions. The regression
suite covers both cases. M05 typecheck, 73 UI tests, and the disposable
production build passed; no session was started and no audio or machine state
was changed.

## WebView2 response-shape hardening (2026-09-07)

The bounded WebView2 transport now accepts only unambiguous JSON-RPC responses:
exactly one of `result` or a finite numeric `error.code` plus string
`error.message` must be present. Malformed, ambiguous, and wrong-typed host
messages remain ignored and therefore cannot resolve a pending UI request.
M05 acceptance passed with TypeScript typechecking, 85 Vitest tests, and a
disposable three-file Vite production build. No native host, audio stream,
driver, or machine configuration was accessed.

## WebView2 outbound request bounds (2026-09-07)

The WebView2 transport now validates outbound JSON-RPC version, nonempty bounded
method names, and finite safe request IDs before posting to the native bridge.
Invalid requests are rejected locally and never cross the page/host boundary.
M05 acceptance passed with TypeScript typechecking, 86 Vitest tests, and a
disposable three-file Vite production build. No native host, audio stream,
driver, or machine configuration was accessed.

## Session identity bounds (2026-09-07)

The direct injected bridge and WebView2 session path now reject session
identities longer than 128 characters, matching the boundary's bounded-input
policy. TypeScript typechecking and all 86 Vitest tests passed. No native host,
audio stream, driver, or machine configuration was accessed.

## WebView2 origin gate (2026-09-07)

`WebView2RpcTransport` now supports an explicit bounded allowed-origin
configuration. When configured, response messages with a different or missing
origin are ignored before request correlation; the default remains compatible
with the existing injected test seam until a native shell supplies its exact
trusted origin. M05 typecheck and all 87 Vitest tests passed. Native shell
packaging and manual acceptance remain open.

The normal `main.tsx` WebView2 construction path now supplies the loaded page's
`window.location.origin` to the transport, making the origin gate active for
the live UI rather than only for callers that manually configure the class.
Malformed or originless host responses therefore remain disconnected from
pending requests. The 87-test UI suite and typecheck pass; native packaging and
manual acceptance remain open.

## Opaque-origin rejection (2026-09-07)

The WebView2 transport configuration now rejects the browser's opaque `null`
origin and invalid/empty origin bounds. The normal startup path therefore
fails closed to the disconnected preview if a trusted page origin cannot be
established, rather than treating an opaque origin as an allowlist. UI
typecheck, 87 tests, M05 acceptance, and documentation validation passed.
Native shell packaging and manual visual/accessibility acceptance remain open.

## Partial route provenance display (2026-09-07)

The route explanation contract now includes `complete`. When the backend hits
the 500-path safety ceiling, the editor labels the count as partial and does
not present the bounded list as complete provenance. UI typecheck and the
existing UI acceptance suite remain the required portable evidence; manual
visual/accessibility acceptance and native shell packaging remain open.

## Recording metadata boundary (2026-09-07)

The live UI backend adapter rejects recording title, artist, and comment values
longer than 256 Unicode characters before sending `recordings.setMetadata`.
This mirrors the storage/recording validation boundary and prevents an invalid
edit from reaching the transport. The regression uses supplementary Unicode
characters to verify code-point counting rather than JavaScript UTF-16 units.
UI tests, typecheck, build, and documentation validation are the evidence; the
native shell and manual visual/accessibility gates remain open.

## M05 UI acceptance requalification (2026-09-08)

`tests/acceptance/m05-ui.ps1` passed: TypeScript typecheck, 14 Vitest files
with 91 tests, and a temporary three-file production build. The build output
was disposable and removed by the wrapper. This validates the portable UI
contract/presentation surface; native shell packaging and manual visual,
keyboard, screen-reader, and accessibility acceptance remain open.

## UI inventory cursor consumption (2026-09-08)

The live UI adapter now consumes all bounded pages for recordings, sessions,
devices, and managed virtual devices. It preserves the legacy array response,
rejects malformed or non-advancing cursors, and stops after a 10,000-page
safety ceiling. The UI suite passed 91 tests, TypeScript
typecheck passed, and an elevated temporary Vite production build completed.
No audio endpoint or machine configuration was accessed.
## Verified application identity presentation (2026-09-08)

The UI now includes a read-only verified executable-path panel backed by the
authoritative `ApplicationInfo.executablePath` field. It shows PID/path pairs
only when the backend supplied a verified path and otherwise reports the
unavailable state; it exposes no process binding or mutation action. The
dedicated component regression covers both states. UI typecheck, 15 Vitest
files/93 tests, and a temporary production build passed. This is display and
discovery evidence only; durable process capture still requires the managed
native owner.

## Read-only recovery checkpoint panel (2026-09-11)

The UI now presents the bounded persisted recovery discovery result as a dedicated,
read-only `Recovery checkpoints` panel. It displays recording IDs and checkpoint
status, reports an unavailable/error state without inventing recovery actions, and
explicitly states that inspection does not open, repair, play, or delete audio files.
The connected-backend regression supplies an invalid checkpoint and verifies the
heading, identity, status, and safety copy. The full UI suite passed with 17 test
files/124 tests, TypeScript typecheck passed, and a disposable Vite production build
completed in an alternate output directory. No session, audio endpoint, driver, or
machine configuration was accessed. Manual Narrator, scaling, and packaged-shell
acceptance remain open.
## 2026-09-12 — Virtual-route editor surface

The connected UI now exposes the typed virtual-route list and replacement
operations through a dedicated panel. It displays the current revision and
routes, refreshes the authoritative backend state, validates non-negative
revision input and JSON-array shape locally, and sends replacement with a
unique idempotency key. Disconnected mode clears the view and disables all
mutations. UI typecheck, the 126-test suite, and diff checks passed. Backend
validation and device administration authorization remain authoritative; the
panel does not activate endpoints or change machine audio settings.

## 2026-09-14 - Selected-node telemetry

The selected-node inspector now consumes the redacted diagnostics snapshot and
renders backend-owned meter peak/RMS/clipping and dynamics gain-reduction/gate
state for the selected committed node. Draft, stopped, unprepared, and busy
stages remain explicitly unavailable rather than showing stale or invented
values. UI typecheck and the full suite passed with 18 files/167 tests. The
panel is currently snapshot-based; bounded meter event refresh and attended
WebView2/accessibility acceptance remain open. No endpoint or machine audio
configuration was accessed.

The snapshot-based limitation was narrowed on 2026-09-14: while a session is
running, the UI now refreshes only `system.diagnostics` at a bounded 1 Hz,
coalesces an in-flight request, and retains the last known values on transient
failure. It stops the timer when the session stops or backend disconnects. The
live adapter regression and 18-file/167-test UI suite pass; no durable meter
events, endpoint access, or machine audio configuration are involved.

## 2026-09-16 - Native fan-out adapter seam

The connected UI backend now exposes typed `prepareNativeOutputs`, forwarding
the explicit session generation and one-to-eight render endpoint IDs to the
shared `nativeOutputs.prepare` method. A focused backend regression verifies
the exact method and payload; the disconnected backend remains incapable of
native mutation. This adds the visible control surface but does not claim
endpoint activation, physical latency, or managed-driver qualification.

The native endpoint workspace also includes a physical render fan-out panel.
It presents up to eight exact active render endpoints, requires an explicit
positive generation, and forwards only the stopped preparation request through
the typed UI backend. UI typecheck and the full UI suite passed with 236
tests. The panel does not select defaults, alter volume/mute, or replace
backend graph and endpoint validation; attended accessibility and live audio
qualification remain open.
# M05 visual editor evidence

## 2026-09-16 - recorder drag/drop draft integration

The Recorder library entry is now draggable/clickable as an available
destination. The draft adapter creates a stopped recorder node with explicit
one-channel `in` and `out` ports, allowing graph routes to be composed before
the separate backend `recorders.create` operation attaches a file worker.
Focused draft/canvas tests passed (31 tests), the full UI suite passed (19
files, 241 tests), and UI typecheck passed. This is presentation/draft
evidence only; no recorder was armed or started.
# M05 visual editor evidence

## 2026-09-16 - virtual endpoint graph connectivity regression

Added draft-graph coverage for the complete virtual endpoint connection
direction: a virtual render-source output can connect to a physical output,
and a physical input output can connect to a virtual capture-sink input. The
focused draft/canvas suite passed 32 tests; the full UI suite passed 19 files
and 245 tests with TypeScript typecheck. This proves the UI draft adapter can
represent the routes; backend commit validation and loaded virtual endpoint
behavior remain authoritative.
# M05 visual-editor evidence

## 2026-09-16 - current UI acceptance requalification

`tests/acceptance/m05-ui.ps1` passed at the current tree: TypeScript
typecheck, 19 UI test files with 245 tests, and a temporary production build
of four files. The build output was temporary and the acceptance scope changed
no audio, driver, or machine configuration. This is presentation/editor
evidence; backend commit validation and native endpoint qualification remain
authoritative separate gates.
## 2026-09-18 - editor simplification and next signal slice

The current UI polish foregrounds the session list, left-to-right canvas,
drag/drop library, adjacent inspector, and transport/status actions. It adds
explicit canvas-node deletion, explains draft connections as uncommitted local
edits, uses an expanding canvas layout, and re-fits ReactFlow after
initialization. The shell adds a generated waveform tray icon. UI typecheck and
all 257 UI tests passed; the Tauri shell passed `cargo check`.

This remains portable/editor evidence. Tray rendering, attended scaling and
accessibility, live drag/drop, microphone audibility, and graph-native
Test Signal/meter playback remain unverified. The raw
`m00-native-loopback.ps1 -AllowLiveAudio` smoke is a separate exact VB-Cable
signal-path check and does not qualify the UI graph.

## 2026-09-18 - graph-native Test Signal and destination meters

Added the available `testSignal@1` node to the domain registry, shared
TypeScript contract, control discovery, and editor library. Its validated
parameters are frequency (20–20,000 Hz), level (-60–0 dBFS), and duration
(1–600,000 ms), with a stopped/unarmed default. The realtime compiler emits a
bounded sine source without callback allocation or blocking, resets its frame
counter at the stopped boundary, and emits silence after the configured
duration. Physical and virtual capture destinations now receive prepared
backend meter stages keyed to their authored node identity, so destination
peak/RMS/clipping observations use the existing redacted diagnostics contract.

Focused Rust tests passed: domain 66, engine 118, and control 177 with 3
guarded-live tests ignored. UI typecheck passed; the full UI suite passed 19
files/259 tests. Contract drift passed with 86 methods, 20 node kinds, 7
processors, and 20 event categories. Documentation validation passed with 56
Markdown files and 224 local links. No endpoint, driver, or persistent audio
configuration was accessed by these checks. Native plan/commit/start/meter/
stop evidence and attended UI review remain Windows-gated and unverified.

The selected-node telemetry renderer now labels the observed state as stopped,
not prepared, endpoint owned by another client, no signal, or available. It
derives ownership wording only from the backend's bounded diagnostic reason;
it does not guess a replacement endpoint or alter device state. UI typecheck,
the full 19-file/259-test suite, formatting, and diff checks passed. This is
portable presentation evidence; exact native ownership and playback remain
separate Windows acceptance gates.

The inspector parameter path was audited and corrected on 2026-09-18: it now
falls back from the built-in processor catalog to the discovered `nodeTypes`
catalog, so `testSignal@1` renders frequency, level, and duration controls and
uses the same bounded client-side validation before graph planning. The focused
catalog regression and full UI suite passed; the suite now contains 260 tests.
Backend validation remains authoritative at plan/commit time.

## 2026-09-18 - native Test Signal and destination-meter acceptance

The new explicit `tests/acceptance/m05-test-signal-native-live.ps1` wrapper
passed in the elevated authorized Windows context against the exact CABLE
capture and PD200X render endpoints. The guarded control test exercised
plan/commit, stopped endpoint preparation, session start, bounded pumping,
node-keyed destination-meter observation, session stop, and worker cleanup.
It processed 187 quanta and observed a destination peak of `-18.000 dB`.
No default device, volume, mute, privacy, driver, or persistent audio
configuration changed. This closes the existing-device native Test Signal /
meter workflow slice, but not attended accessibility, physical latency,
endurance, OS-transition reopen, managed-driver, signing, or release gates.
## 2026-09-18 - clean-commit Test Signal meter qualification

On committed tree `449ab5d9`, the guarded
`m05-test-signal-native-live.ps1 -AllowLiveAudio` acceptance passed against
the exact existing CABLE Output capture and CABLE Input render endpoints.
The graph-native Test Signal reached the destination meter with 187 processed
quanta and a destination peak of `-18.000 dB`; plan/commit, start, bounded
meter observation, stop, and cleanup all passed. This remains existing-device
and temporary-stream evidence, not attended UI or physical-latency evidence.

## 2026-09-21 - first attended visual/interaction review (real defects found)

The Windows computer-use surface remained unavailable in every prior attempt
(`apps: []`, `browsers: []`), so no attended UI evidence existed before this
entry despite repeated automated-test passes. This session had no
computer-use/screen tool either, so the user drove the review directly: the
debug shell (`src-tauri/target/debug/audiorouter-shell.exe`) was launched
against a disposable database/pipe per the documented interactive
control-plane check in `src-tauri/README.md`, with the Vite UI dev server
(`npm run dev --prefix ui`) started first so the shell's devUrl
(`http://localhost:5173`) actually resolved — an initial attempt without the
dev server running showed "can't reach this page" in the shell window; this
was a launch-sequencing mistake on the agent's part, not a product defect.

The user then interacted with the running editor (session "Gaming +
Discord", physical input → Voice gain → physical output) and reported the
following, each a genuine finding, not yet triaged into requirement IDs or
fixed:

1. **Light theme contrast/completeness bug. FIXED 2026-09-21.** Screenshot
   attached by the user (`LightTheme.png`) showed the theme selector set to
   "Light", but the top bar, left sidebar, and canvas remained dark/near-black
   with low-contrast text, while the right-hand inspector panel correctly
   rendered light. Root cause: `ui/src/App.tsx` correctly applied the
   `theme-light`/`theme-high-contrast` class to `.app-shell` all along (the
   React wiring was never broken), but a later "Audio board redesign" CSS
   pass (`ui/src/styles.css`) had re-defined `.topbar`, `.sidebar`,
   `.canvas-panel`/`.panel`, `button`/`input`/`select`, `.compact-status-panel`,
   `.session-flow-canvas`, `.canvas-library`, `.session-flow-toolbar`, the
   node-telemetry panel, and the endpoint-binding editor using hardcoded
   dark literal colors instead of the file's own `--ambient-*`/`--text-*`
   CSS custom properties, so those elements never varied regardless of the
   theme class. Fixed by (a) adding real `.theme-light`/`.theme-high-contrast`
   overrides for the `--ambient-bg`/`--ambient-surface`/`--ambient-surface-hi`/
   `--ambient-edge`/`--ambient-key`/`--ambient-shadow`/`--text-strong`/
   `--text-soft` custom properties, and (b) converting every hardcoded-literal
   rule listed above to reference those variables — variable inheritance
   means a consuming rule now repaints correctly regardless of its own
   selector specificity or position in the file, avoiding a fragile
   specificity/order fight with the pre-existing theme-scoped overrides.
   Iteratively found and fixed three further instances of the same pattern
   during user retesting: the selected-session-item sidebar highlight
   (`.session-item.selected`/`.graph-list button.selected`, hardcoded navy),
   the `.notice`/`.warning-panel` banners (hardcoded dark amber-brown, no
   light/high-contrast variant existed at all — added dedicated overrides),
   and the `.theme-picker` control (hardcoded colors plus a stacked
   label-above-select grid layout that visually misaligned it against the
   adjacent single-line "Compact status"/"Reconnect" buttons — switched to
   an inline flex layout and variable-based colors). Also fixed in the same
   pass: the duplicated `Session inventory unavailable: Session inventory
   unavailable` label (`ui/src/App.tsx`) — `formatUiError`'s fallback string
   already equals the JSX's hardcoded prefix when the caught value isn't a
   proper `Error` instance; removed the redundant prefix. User confirmed
   the fix across several rounds of live retesting via the corrected
   embedded-backend shell launch method; `npm run typecheck` and the full
   282-test suite passed after every change with no regressions. The
   `Last known backend state is stale: Event subscription failed` message
   noted alongside the original screenshot was not investigated further —
   it did not reappear during retesting and was not the user's complaint.
2. **Node connection dragging is confusing/broken for one anchor. FIXED
   2026-09-21 (same root cause as item 6 below).** Dragging from the Voice
   Gain node's right-edge top output anchor to another node's input anchor
   did nothing; only the node's bottom anchor could be dragged to make a
   connection. See item 6 for the root cause and fix — a duplicate,
   unstyled default handle was overlapping the two real ones specifically
   on the top and bottom edges, making precise clicking there unreliable.
   Not independently re-verified against this exact original repro (right
   output → left input) after the fix, but the underlying duplicate-handle
   defect it was traced to is fixed and regression-tested.
3. **Node parameter dB value cannot be typed.** Opening the Voice Gain
   node's property panel, the numeric dB value could not be selected/edited
   by clicking into it. **Retracted 2026-09-21** — this was a test-harness
   artifact, not a real defect; see the methodology-defect entry in
   [the M07 evidence file](M07-automation-recovery.md#attended-testing-methodology-defect-found-and-corrected-2026-09-21).
   With the shell launched correctly (embedded persistent backend, a real
   created session), the side panel correctly shows and allows editing the
   Gain node's dB parameter. The panel was empty during the original review
   because the diagnostic backend process used to launch the shell exits
   after a bounded number of requests by design, most likely before ever
   delivering a complete parameter-schema response.
4. **Node parameter slider (canvas mini-fader) cannot be dragged. FIXED
   2026-09-21.** The Voice Gain node's inline canvas slider showed a
   draggable-looking bar with the correct drag cursor on hover, but
   left/right mouse movement did not move the bar or change the value. Root
   cause found in `ui/src/App.tsx`: the canvas's inline parameter controls
   (`SessionFlowCanvas`'s `MiniFader`/`MiniEq`/mute-toggle) call
   `onSetNodeParameter(nodeId, name, value)` — a 3-argument contract meant
   to target an arbitrary node — but the function wired to that prop,
   `changeNodeParameter`, only accepted `(name, value)` and always operated
   on `selectedNode.id`, ignoring the rest. JavaScript silently drops extra
   call arguments rather than erroring, so `nodeId` was misread as the
   parameter name and `name` (e.g. the literal string `"gainDb"`) as the
   value, which validation then correctly rejected every time — explaining
   why dragging visibly did nothing. Fixed by splitting the function into
   `changeNodeParameterOn(nodeId, name, value)` (looks up the target node
   by the given ID rather than assuming the selection) and a
   `changeNodeParameter(name, value)` wrapper for the side panel's existing
   2-argument callers; the canvas is now wired to the former. User confirmed
   after the fix: "I can drag and change the EQ values from the node now."
   `npm run typecheck` and the full `npm test -- --run` suite (282 tests)
   both passed after the change with no regressions.
5. **Same non-editable-parameter behavior on other node types. FIXED
   2026-09-21 (same root cause as item 4).** Gate and Graphic EQ were also
   tried and exhibited the identical argument-mismatch bug for their own
   inline canvas controls (`MiniEq`'s per-band drag handlers use the same
   3-argument `onSetNodeParameter` call). Covered by the same fix as item 4;
   user confirmed EQ dragging now works in the same retest.
6. **Input/output anchor visual distinction is unclear. FIXED 2026-09-21,
   after three iterative rounds of live user feedback and one real
   duplicate-handle bug found along the way.** Nodes with both input and
   output anchors made it hard to tell which anchor was which.
   - **Round 1**: colored input handles `--vst-accent` (purple) and output
     handles `--good` (green), matching the pre-existing `.port.input`/
     `.port.output` list-view convention, and raised the resting opacity of
     unfocused handles from 0.12 to 0.42 (barely visible at rest was itself
     a likely contributor to "anchor doesn't work" reports — a target you
     can't see is hard to click precisely). User reported connecting
     purple→purple failed while green→purple worked — correct behavior, not
     a bug (a connection must run output→input; input→input is invalid by
     definition), but proved purple/green weren't legible enough for the
     user to tell apart, and that no in-app feedback explained *why* the
     invalid attempt did nothing.
   - **Discovered while investigating round 1**: purple was already used
     elsewhere for a *different* meaning — `.flow-node-vst`'s node-border
     accent marks a node as a VST plugin. Reusing it for "input connector"
     was a self-inflicted collision that added to the confusion.
   - **Round 2**: switched input to `--cyan` (matching the existing
     `flow-node-input` node-border/library-palette "input category" color
     instead of an unrelated pairing) and added a permanent legend above
     the canvas (`ui/src/App.tsx`, `.canvas-handle-legend`) plus actual
     user-facing feedback for invalid-direction attempts: added an
     `onConnectionRejected` callback prop to `SessionFlowCanvasProps`
     (`ui/src/SessionFlowCanvas.tsx`), wired in `onConnectEnd` by detecting
     `connectionState.fromHandle.type === connectionState.toHandle.type`
     when no connection resulted, surfaced via `setActionMessage` in
     `App.tsx`. Same turn, user also reported hovering an anchor made it
     "move oddly" — root cause: the hover rule's `transform: scale(1.3)`
     fought with `@xyflow/react`'s own internal positioning transform on
     the handle element; removed the transform-based hover growth entirely
     in favor of opacity/box-shadow-only feedback, which cannot conflict
     with positioning.
   - **Round 3**: user reported cyan and green "look very much the same" and
     asked for two more distinct colors. Introduced dedicated
     `--handle-input`/`--handle-output` variables using a deliberately
     high-contrast warm/cool complementary pair (blue `#3a86ff` / orange
     `#f6a928`) instead of reusing the general accent palette, since the
     goal is category-recognition at a 0.72rem dot, not an accurate but
     visually-similar hex distinction.
   - **Real bug found while verifying round 3**: user's screenshot
     (`3colors.png`) showed **3 dots on the top and bottom edges but only 2
     on left/right** of a 2-port node — a genuine defect, not a color
     perception issue, confirmed still present after a full process +
     dev-server restart (ruling out stale HMR). Root cause: the node
     objects passed to `<ReactFlow>` had no `type` field, and no
     `nodeTypes` prop was registered on the `<ReactFlow>` element, so React
     Flow silently fell back to its **built-in "default" node type** to
     render `data.label` — and that built-in type automatically adds its
     *own* implicit target/source `Handle` at Top/Bottom, on top of the
     app's own fully custom `Handle` components already rendered inside
     `data.label`. This exactly explains the observed 3-on-top/bottom,
     2-on-left/right pattern (the built-in default only adds Top/Bottom,
     never Left/Right), and very plausibly explains item 2's original "top
     anchor doesn't work" report too: three closely-packed, partially
     unstyled/overlapping targets near the top edge make precise clicking
     unreliable. Fixed by registering a trivial pass-through custom node
     component (`FlowNodeRenderer`, module-scope `NODE_TYPES` map — kept at
     module scope since recreating the `nodeTypes` object on every render
     is a documented React Flow foot-gun that forces internal remounts),
     giving every node `type: "flowNode"`, and passing `nodeTypes={NODE_TYPES}`
     to `<ReactFlow>`. User confirmed after this fix: exactly 2 dots on
     every edge, correct colors.
   - `npm run typecheck` and the full `npm test -- --run` suite (282 tests)
     passed after every round of this fix with no regressions.
7. **MP3 recording option — implemented 2026-09-21.** The Recorder node and
   action editor now offer fixed-profile MP3 (192 kbps). The backend uses the
   bundled LAME encoder on the recorder worker, supports mono/stereo 44.1/48
   kHz, persists `format: "mp3"` with `dither: false`, and rejects split
   requests. Recording and control regression tests cover bounded encoding,
   frame-boundary draining, structural inspection, factory selection, and
   persisted metadata. Release qualification still must verify the bundled
   LGPL-3.0 encoder's notices/source obligations before shipping an installer.
8. **Plugin directory scan gap — resolved 2026-09-21.** The user-mentioned
   `C:\Program Files\VSTPlugins\Reaication` path does not exist on this
   machine; the installed directory is `C:\Program Files\VSTPlugins\ReaPlugs`.
   That directory does contain real DLLs (`reacomp-standalone.dll`,
   `reagate-standalone.dll`, `reaeq-standalone.dll`, etc., confirmed present
   on disk with a read-only listing). Their `-standalone` naming is a
   plausible, not yet verified, explanation: ReaPlugs ships both
   host-loadable and standalone-wrapped binary variants, and this project's
   own validated lesson ("Plugin directory names are not format evidence",
   2026-09-08) already established that a prior ReaPlugs/ReaJS VST2
   candidate was correctly rejected here for a real PE-export/format reason,
   not a scanner bug. The shared CLI scan below now confirms the actual
   directory contents and closes this investigation as a path/name mismatch,
   not a scanner defect.

   **Investigation refresh (2026-09-21):** The user-mentioned
   `C:\Program Files\VSTPlugins\Reaication` path does not exist on this
   machine; the installed directory is `C:\Program Files\VSTPlugins\ReaPlugs`.
   Its nine DLLs include `reaeq-standalone.dll` (296,960 bytes). The existing
   read-only `m06-vst2-installed.ps1` qualification passed for that exact
   binary at 44.1, 48, and 96 kHz, including the bounded native-editor and
   supervised-timeout checks; SHA-256
   `c200e540c26ac793b43611aaceb4aa42cdd2829cdfbf0d60494716d9bdde8a7d`
   remained unchanged. This proves at least one installed candidate is a
   valid, worker-compatible VST2 effect; it does not yet prove that the
   directory scanner should admit every `-standalone` DLL or that the UI scan
   root is configured to include this directory.

   The shared CLI scan was then run against the actual absolute ReaPlugs root:
   `cargo run -p audiorouter-cli -- plugins scan --directory
   'C:\Program Files\VSTPlugins\ReaPlugs'`. It returned all nine DLLs with
   no scan errors; each was classified as x64 VST2 and
   `supportedVst2X64Gated`. This closes the scanner investigation as a
   path/name mismatch rather than a scanner defect. The UI still requires the
   user to enter the real installed root; no machine-specific default or
   automatic discovery is claimed.
9. **Feature gap: no way to add an application-capture source from the
   canvas library. FIXED 2026-09-21.** The working "Applications" panel
   (list running processes, "Add capture source" button calling the
   existing `appendApplicationCaptureNode`) already existed, but only in
   the "Full workspace" view, hidden by default in the focused canvas view
   and absent from the canvas library palette entirely — so a user starting
   from the canvas (the default view) had no visible path to it. Added an
   "Application" entry to the library's INPUTS group
   (`ui/src/SessionFlowCanvas.tsx`) that opens a new focused picker dialog
   (`ui/src/App.tsx`, mirroring the existing VST-plugin-picker dialog
   pattern: same focus trap, Escape-to-close, return-focus behavior) built
   on the same existing `applications`/`appendApplicationCaptureNode`
   logic, not a reimplementation. Iterated twice on user feedback: the
   first version listed every running process with a button per row
   (including ones with no observed audio capture, which were visibly
   present but disabled with no explanation); changed to a single dropdown
   showing only applications with an observed capture source, with a
   one-line note naming how many others are hidden and why, plus a single
   "Add capture source" action for the current dropdown selection. `npm run
   typecheck` and the full 282-test suite passed after each round; user
   confirmed the final dropdown version.
10. **Clarified: there is no "application as output" — Windows cannot
    target a specific application for render the way it can target one for
    capture. FIXED 2026-09-21.** User asked whether an application could be
    selected as an output the same way as input. Verified directly against
    `crates/domain/src/lib.rs`'s `NodeKind` enum: there is `ApplicationCapture`
    but no render/output equivalent — this is a genuine Windows/WASAPI
    platform asymmetry (an application's own audio *sessions* are
    enumerable and capturable, but there is no OS mechanism to inject audio
    into a specific running application by picking it from a list; an
    application must select its own input device). Rather than build a
    misleading picker for a capability that does not exist, clarified the
    existing correct mechanism in place: routing into a virtual endpoint
    (e.g. VB-Cable) that the target application then selects as its own
    microphone/input in its own settings — this project's "Existing virtual
    output" library entry already does exactly this. Initially pointed the
    user at "Virtual capture sink" instead, which was wrong: that entry is
    correctly, permanently disabled (`ui/src/library.ts`) pending the
    deferred, not-yet-signed AudioRouter-managed driver, a real project
    scope boundary documented throughout this plan, not a bug — its own
    tooltip already says to use "Existing virtual output" today. Split the
    shared `NO_DRIVER_NOTE` tooltip text into direction-specific
    `NO_DRIVER_INPUT_NOTE`/`NO_DRIVER_OUTPUT_NOTE` variants so the output
    entry's tooltip explicitly explains the "select the same virtual
    endpoint here and in the target application's own settings" mechanism.
    User confirmed the updated tooltip. `npm run typecheck` and the full
    282-test suite passed with no regressions.

Items 1, 2, 4, 5, 6, 9, and 10 are fixed and regression-tested (typecheck + full UI
suite); item 3 is retracted as a test-harness artifact. Items 7 and 8
remain open, not yet triaged into requirement IDs or fixed. This entry is
the attended-evidence record only. The launched shell/backend/dev-server
instances used disposable temp databases; no default device, volume, mute,
privacy, driver, signing, or persistent audio/machine configuration was
changed. The tray-specific checklist items are recorded separately in
[the M07 evidence file](M07-automation-recovery.md), including the same
test-harness methodology defect this entry's item 3 was traced to.

## 2026-09-22 signal-flow visualization implementation

Implemented UI-15 in `ui/src/SessionFlowCanvas.tsx` and `ui/src/styles.css`.
Canvas edges now show a directional moving highlight when fresh backend meter
telemetry confirms signal. Stroke width scales from bounded RMS level and
smooths through CSS transitions. Active, silent, muted, disabled, stale,
unavailable, unmetered, and faulted states have distinct styling and an
accessible edge label; reduced-motion and high-contrast styles are covered by
the existing accessibility mechanisms. The feature uses existing diagnostics
refresh and adds no audio processing or polling.

The control/engine diagnostics path does not meter every microphone,
application-capture, or processor boundary. For a single unbranched path, the
canvas can use the next downstream meter as end-to-end evidence. It stops
inferring across mixers and ambiguous fan-out. Thus it shows that signal
reaches a downstream measured point, but does not localize the exact blocking
processor inside an unmetered span. Exact per-edge fault localization would
need a separately specified backend metering extension.

Verification on Windows 11, 2026-09-22: `npm.cmd run typecheck` passed;
focused `SessionFlowCanvas.test.ts` passed 21/21; `tests/acceptance/m05-ui.ps1`
passed typecheck, 19 UI files / 284 tests, and temporary production build;
`tests/acceptance/docs.ps1` passed 58 Markdown files / 266 local links;
`tests/acceptance/m08-traceability.ps1` passed with 160 normative IDs; and
`git diff --check` passed. No live audio or attended shell check was run, so
microphone/application visual confirmation and Narrator/reduced-motion review
remain open.

## 2026-09-22 Test Signal canvas Play/Stop controls

Added separate Play and Stop buttons to each Test Signal canvas card. The
buttons use the existing authorized `session.start`/`session.stop` actions;
they start or stop the whole session, and the card says so. Play is disabled
until the current graph is committed, the enabled signal path reaches an
enabled physical output, and backend diagnostics confirm this session has a
prepared stopped endpoint worker. The card identifies `deviceAdministration`
as the endpoint-preparation requirement. This adds no renderer audio path or
permission bypass. The controls stop event propagation so they remain usable
inside React Flow and expose their action names to assistive technology.

Validation on Windows 11, 2026-09-22: `npm.cmd run typecheck` passed;
focused canvas and control tests passed (28 tests); `tests/acceptance/m05-ui.ps1`
passed typecheck, 20 UI test files/291 tests, and a temporary production build;
`tests/acceptance/docs.ps1` passed 58 Markdown files/267 local links;
`tests/acceptance/m08-traceability.ps1` passed with 160 normative IDs; and
`git diff --check` passed. `rtk` is not installed in this environment.

Live playback remains unverified. The visible session graph is configured, but
the current shell rejected `nativeEndpoints.prepare` with
`permissionDenied: DeviceAdministration`; no native worker was attached and no
audio was played. The UI controls therefore remain unavailable until an
authorized exact endpoint preparation succeeds. Attended Play/Stop, flow
animation, Narrator, reduced-motion, contrast, and scaling checks remain open.

## 2026-09-22 media source request and Test Signal usability follow-up

Audited the requested WAV/MP3 source and temporary voice take workflow. The
domain registry currently has 20 node kinds and the engine's generated
TestSignal is its only authored audio source. No WAV/MP3 decoder or source-media
API exists. The recording crate has MP3 encoding, while `recordings.preview`
only inspects metadata and there is no temporary-take-to-graph-source flow.
Implementing this correctly requires backend decode off the callback, bounded
media ownership/lifetime, temporary capture with explicit permission and
cleanup, and a graph source transport contract. The renderer must not play the
file directly because that audio would bypass the routed graph and flow meters.

As a small UI correction, reduced the Test Signal node to 184px and shortened
its inline readiness text. The existing session Play/Stop actions remain
disabled until the graph and exact endpoint are prepared. The current shell was
previously denied that preparation with `permissionDenied: DeviceAdministration`;
no authorization was changed and no audio was played.

Validation: `npm.cmd run typecheck` passed; the focused Test Signal controls
suite passed (6 tests); `git diff --check` passed. Full M05 validation and
attended audio/animation remain open. This is not evidence that WAV/MP3 playback
or temporary voice recording is implemented.

## 2026-09-22 WAV/MP3 graph source implementation

Added `audioFile` as a persisted graph input with opaque backend media IDs.
Media uploads are ordered and size bounded; the backend decodes WAV/MP3 and
resamples off the callback, and the graph source reads immutable decoded
samples with bounded atomics in the callback. The node exposes compact
Play/Stop controls; Pause/Resume, file import, descriptive status, and looping
are in the inspector. Play starts the session when needed and source transport is
node-scoped. Test Signal stays 184px wide.

Portable evidence: `cargo test -p audiorouter-domain --locked` passed (66),
the engine graph-source regression passed, `cargo test -p audiorouter-control
--locked` passed (180; 4 guarded-live ignored), the storage media
persistence/bounds regression passed, M05 UI acceptance passed typecheck and
20 files/292 tests plus temporary production build, and contract drift passed
(91 methods, 21 node kinds, 7 processors, 20 event categories). These checks
do not qualify attended device playback. `rtk` was unavailable, so documented
raw commands were used.

Temporary microphone recording remains unimplemented. On 2026-09-22 the user
explicitly authorized the local desktop shell to receive `Record` for
intentional takes in approved recording roots. `Capture` and
`DeviceAdministration` remain separate and are not included in the shell's
ordinary grant. The backend still needs a bounded recorder-to-temporary-media
workflow with cleanup and graph playback before the UI can expose a take
control. Renderer microphone capture remains disallowed. The source tests and
session-flow UI tests passed, including 23/23 canvas tests for telemetry-based
edge presentation, but no attended UI was available (`cua.getState()` returned
no apps or browsers). Exact endpoint preparation still requires
`DeviceAdministration`; no live audio or animation was observed.

## 2026-09-22 temporary voice take and visual preview follow-up

The local shell's explicitly authorized `Record` scope now supports a bounded
temporary-take flow from an already-running route's enabled Recorder node. The
backend accepts only a completed WAV with the temporary recorder identity,
checks file size/duration and WAV metadata, imports decoded audio as 24-hour
media, and removes the source recording and library row. Expired media is
excluded from reads and pruned. The UI offers a 120-second maximum take,
stop/import, and ordinary Audio File source playback after plan/commit. No
renderer microphone path was added and the shell still lacks `Capture` and
`DeviceAdministration`.

Checks on Windows 11: the locked workspace Rust suite passed; the full UI suite
passed 20 files / 294 tests; M05 acceptance passed typecheck, tests, and a
temporary production build; contract drift passed (92 methods); docs validation
passed (58 Markdown files / 267 links); M08 traceability passed (160 IDs).
The browser-only three-node harness shows simulated meter readings and clear
edge lines. It is a visual layout check, not real audio evidence. The attended
10-second sound/animation check remains open because no app is exposed by the
computer-use surface and exact endpoint preparation returned
`permissionDenied: DeviceAdministration`.

## 2026-09-22 hover and visible edge motion follow-up

After the user reported a bouncing Test Signal node and stationary dashed flow,
the canvas hover style was narrowed to a static border cue; the previous large
shadow lift made the card appear to jump. Meter-responsive width is now drawn
as a solid amber base, with an independent narrow animated dash overlay for
fresh active edges. This prevents the dash gaps from collapsing visually when
the responsive stroke becomes thick. The local harness now marks its graph as
running and uses fake input/output readings so the active edge appearance is
visible. Reduced-motion mode still turns off movement; inactive/stale/muted/
faulted paths remain static.

Verification: focused canvas/Test Signal tests passed (29); UI typecheck passed;
M05 acceptance passed 20 files / 294 tests and the temporary production build.
The Chromium harness screenshot at
`%TEMP%\audiorouter-flow-preview\flow-active-overlay.png` visibly shows the
solid level stroke plus a pale dash overlay. It is simulated visual evidence,
not live audio. The user's report confirms the prior attended interface showed
stationary lines; a subsequent live check was not possible because the
computer-use surface exposes no apps/browsers.

## 2026-09-22 Play report: canvas nodes disappeared

The user reports that all canvas nodes disappeared immediately after pressing
Play. Read-only checks found the shell process responding and the Vite page
returning HTTP 200. No matching Windows Application Error/Windows Error
Reporting event was found for the recent window. No attended app surface was
available, and the existing shell did not write persistent diagnostics; the
historical session RPC and opened database therefore could not be verified.
No graph recovery, app restart, or session mutation was attempted.

Added shell JSONL logging for session start/stop/get/list to
`%LOCALAPPDATA%\\AudioRouter\\logs\\shell.jsonl`. It captures sanitized RPC
outcome and session revision/node/edge counts, excludes graph/request contents
and audio data, and rotates at 5 MiB to one previous file. A focused regression
checks count logging and content exclusion. All 30 shell tests passed. The
shell-specific Cargo lockfile was updated after crate index access succeeded.
An isolated dev shell build passed at
`%TEMP%\\audiorouter-diagnostic-build\\debug\\audiorouter-shell.exe`; the
normal target could not be overwritten while the old shell held the executable.
This logger is not active in the already-running shell until that shell is
relaunched. No session or app restart was done. `rustfmt --check` reports
pre-existing formatting in the untouched tray-icon block; `git diff --check`
passed. New logger and test code are formatted.

## 2026-09-22 tabbed workbench, diagnostics, MCP setup, and browser E2E

Replaced the former session rail and stacked default workspace with a central
canvas and a right-side nine-tab workbench: Tools, Properties, Session, Setup,
Devices, Recording, Advanced, MCP, and Logs. Session selection/name/revision,
duplicate/delete, draft undo/redo/discard, plan, warning acknowledgement, and
commit are in Session. Device setup, recording operations, and advanced client,
startup, OS transition, graph history, and plugin scan controls are reachable
in dedicated tabs. Tab selection has an explicit cyan state and semantic
`role=tab`/`aria-selected` state. The top button now reads Show status/Hide
status; the workspace retains task tabs in either status mode.

MCP UI displays exact CLI/database/named-pipe paths from the desktop bridge,
builds copyable Codex TOML and Claude Code PowerShell setup, and shows a
redacted activity stream. It explains observer-first authorization and local
stdio transport. The browser-only preview uses marked placeholders because it
has no Tauri bridge; it does not expose a browser-to-backend transport.
Frontend keeps a bounded in-memory exception/checkpoint list. Shell and backend
write rotating JSONL RPC logs with method/outcome/timestamp and safe summaries;
MCP activity retains client/tool/outcome/safe argument field names only. High
rate bridge pumps/heartbeats are excluded; no realtime callback logging,
request parameter values, audio, or model reasoning are added. All three log
writers serialize local concurrent writes/rotation. Backend, MCP, and shell
writers acquire same-user Windows named mutexes around rotation and append;
abandoned ownership is recoverable after process exit. Caller-controlled MCP
client/tool/field names are length/count bounded; failure logs use a whitelisted
error category rather than raw error text. Shell graph summaries omit session
IDs and runtime detail.

### Verification evidence

- Windows 11 UI: `npm.cmd run typecheck` passed; `npm.cmd test` passed 20/20
  files and 294/294 tests; Playwright browser E2E passed 6/6 (latest rerun
  2026-09-23). E2E covers all
  16 exposed built-in node kinds, an 11-node multi-source/processing/output
  composition and mute edit, one draggable source-to-output connection,
  1280x720 canvas width, workspace tabs, MCP snippet expansion/copy feedback,
  and Logs. This is UI/harness coverage; it does not exercise an actual audio
  backend or prove every complex route can be connected and committed. A
  ten-edge pointer-drag experiment failed under overlapping rapid node
  placement; that brittle test was removed. Multi-source composition is
  currently asserted without committing connected edges, which remains open.
- Rust: full `cargo test --workspace --locked --quiet` passed; backend log
  redaction/category tests passed 2/2; named mutex concurrency regression
  passed; MCP activity redaction/category/bounds tests passed 3/3; stdio/backend
  MCP interop passed 3/3; Tauri shell tests passed 31/31. Separate transport
  and shell authenticated-pipe test binaries were run concurrently after the
  cross-process lock was added; both passed.
- Contracts: drift passed (92 methods, 21 node kinds, 7 processors, 20 event
  categories); documentation links passed (58 Markdown files / 272 links);
  requirement traceability passed (162 IDs); `git diff --check` passed.
- Designer screenshots at `%TEMP%\audiorouter-designer-review\`: Tools,
  Session, Properties, Setup, Devices, Recording, Advanced, MCP, Logs, dark,
  light, and high-contrast. At 1280x720, review found unstyled native endpoint
  selectors and labels running together in Devices. Shared styles now provide
  stacked labels, full-width rounded selectors, and single-column aligned
  actions; the refreshed screenshot confirms the change. Session name and
  session selection controls now use the same stacked rounded-field treatment;
  the refreshed Session screenshot was reviewed. The 1440x1000 view
  shows the wider canvas, 3x3 tab rail, visible selection, rounded input
  surfaces, and themed scrollbars. Review also caught clipped long MCP
  commands; they now wrap within the panel. A desktop-only live snippet and
  actual shell activity were not available to the browser preview.

### Limits and next evidence

No app surface was available for attended shell interaction. The currently
running legacy shell's database/session is unidentified, so it was not
restarted or mutated. The guarded live loopback/Test Signal test was not run;
no sound was started. External Codex/Claude enrollment, real UI-to-MCP tool
call/activity, Narrator, and live animated edge confirmation remain open.
The pre-fix local backend JSONL contains two malformed lines generated by
concurrent test processes before cross-process locking was implemented; the
reader ignores malformed lines, and later concurrent authenticated pipe tests
completed successfully after the named mutex change. A post-test line audit
found 35 backend lines (33 valid, 2 historical malformed), 25 valid MCP lines,
and no shell log file. New concurrent writes appended valid records; no new
malformed line appeared. These records came from test processes, not an
attended product route. The existing PID 53576
predates the shell logger, and `shell.jsonl` is not present until the updated
shell starts.
The Playwright suite does not yet implement a fully connected multi-branch
plan/commit route matrix; that remains necessary for complete graph E2E
qualification. `rtk` was unavailable, and direct PowerShell commands were
used instead. The touched CLI/transport packages and Tauri `main.rs` pass
scoped rustfmt checks. Whole-crate Tauri rustfmt still flags pre-existing
formatting in untouched `startup.rs`, `backend_supervisor.rs`, and
`os_transition_windows.rs`; they were restored after confirming the changes
were formatter-only.

## 2026-09-23 - connected draft route E2E supplement

This entry supersedes the earlier statement above that the Playwright suite
did not implement a connected multi-branch plan/commit route. The browser E2E
suite now uses `ui/route-harness.html` and a controlled fake `AppBackend`. The
route test creates Test Signal, physical input, Audio File, application
capture, and endpoint loopback sources; a mixer and gain/EQ/compressor/limiter/
mute chain; and meter, recorder, and physical output branches. It first
confirms the planner rejects the disconnected graph, uses the Setup workbench
to add 13 edges, captures the graph screenshot, then plans and commits revision
8. The controlled planner checks endpoint existence/direction, channel matrix
dimensions and finite [-2,2] coefficients, duplicate connections, the
single-incoming-edge rule outside mixers, and a connected source-to-output
path. It does not exercise the Rust graph validator, endpoint availability,
device I/O, or audio transport. Treat this as UI and draft-adapter coverage,
not physical audio acceptance.

`npm.cmd run typecheck`, `npm.cmd run test` (20 files / 294 tests), and
`npm.cmd run e2e` (7/7) passed on Windows 11. Contract drift, documentation
validation (58 Markdown files / 273 local links), M08 traceability (162
normative IDs), and `git diff --check` passed. The production UI bundle built
successfully into `%TEMP%\audiorouter-ui-build-review`; Vite could not empty the
normal `ui/dist` because a bundle was locked (`EPERM`), so the running shell was
not disturbed. Screenshot:
`%TEMP%\audiorouter-designer-review\workspace-complex-route.png`.
Visual review confirms the wider canvas and tabbed sidebar remain legible with
the 13-edge graph; Setup is scrolled partway down after adding links. No app or
browser was available through the computer-use surface, so live shell/MCP and
physical loopback remain unverified.

## 2026-09-23 - status view navigation review

The old `Show status`/full-workspace CSS expanded every legacy panel into a
long page and duplicated controls already grouped under workbench tabs. It now
shows a compact session/status strip while keeping the focused canvas and
right-side tab layout. The Playwright check verifies those legacy panels stay
hidden, the selected Session tab is preserved, and the sidebar stays visible.
Virtual-device lifecycle and route controls are included under Devices. The
full status screenshot was reviewed at
`%TEMP%\audiorouter-designer-review\workspace-status.png`; its session summary
and two actions align above the canvas and tabs. Typecheck, UI tests (294/294),
and Playwright (7/7) passed after the layout and stricter route-planner checks.
The production UI build also passed in the fresh temporary output directory
`%TEMP%\audiorouter-ui-build-review-0923b`; contract drift, docs validation
(58 Markdown files / 273 links), M08 traceability (162 IDs), and
`git diff --check` passed. A new Windows process inspection confirmed only the
legacy shell is visible and its executable path is hidden; its diagnostic log
is absent, and backend/MCP logs have not been written since 2026-09-22.

## 2026-09-23 - Audio File UI flow and MCP guide reachability

The route harness supplies a canned WAV upload contract. A Playwright flow
selects an Audio File node, uploads a small WAV-shaped fixture, verifies the
returned filename/status, toggles looping, and confirms Play and Stop remain
disabled before graph commit. This proves UI upload sequencing and draft
controls only: the fake backend returns metadata and does not parse, decode,
persist, or play the fixture. Real decoding/playback remains covered by the
engine/control/storage suites and guarded native acceptance, not by this
browser test.

The MCP workspace E2E scrolls its right-side tab content to the final setup
instructions and asserts they are visible. Screenshot review found the guide
below the initial viewport, so this makes discoverability an explicit
regression check while retaining the compact workbench. Dark, light,
high-contrast, MCP, and status screenshots were reviewed under
`%TEMP%\audiorouter-designer-review\`.

Verification on Windows 11: focused Audio File and workspace/MCP E2E passed
2/2; the simple Test Signal-to-Physical Output route and full multi-source
route also plan and commit in the fake harness; full Playwright passed 11/11.
The diagnostics E2E feeds a redacted MCP request and backend graph-commit
denial through the simulated Tauri bridge, then verifies MCP activity,
backend diagnostics, and client graph checkpoints are visible in their tabs.
UI typecheck and unit/component suite passed (294/294); production build
passed at `%TEMP%\audiorouter-ui-build-review-0923c`; contract drift, docs
validation (58 files / 273 links), traceability (162 IDs), and `git diff
--check` passed. No CUA desktop/browser surface was available; real MCP
client enrollment, current shell logs, guarded playback, and live animated
edges remain unverified. A read-only audit found 35 backend JSONL rows (33
parseable, 2 malformed historical rows), including five `graph.commit`
errors (three typed `permissionDenied`); the MCP log had 25 parseable rows,
including ten errors (six typed `toolError`); no shell log exists. The safe
log schema intentionally excludes raw runtime error messages and arguments,
so this stale evidence cannot explain the user's node disappearance. Do not
infer that those historical auth/test events describe the current session.
MCP stdio/backend interop passed 3/3; backend diagnostic privacy/concurrency
passed 3/3; MCP activity redaction/bounds passed 3/3. These are process/pipe
fixtures and do not prove an external Codex or Claude client is enrolled.

The MCP stdio process integration additionally sets a unique temporary
`LOCALAPPDATA` for the spawned CLI child, calls actual `tools/call` requests,
then parses that process's `mcp-activity.jsonl`. It verifies a tool request,
outcome/error category, safe argument field names, and absence of a nested
secret sentinel value. The test therefore validates the real stdio server and
writer together and no longer appends these integration rows to the user's
local app log. The full `mcp_stdio` target passed 3/3 on Windows; one initial
probe supplied an unsupported extra top-level field and was correctly rejected,
so it was replaced with a schema-valid `call_api` request carrying the sentinel
inside opaque params. Rustfmt check passed after formatting that assertion.

## 2026-09-23 - persistent renderer diagnostics

The client diagnostic ring now survives WebView reloads using local storage:
at most 80 entries, each capped at 320 characters. It records graph node/edge
counts and revision, plus UI error category and script basename/line. It omits
raw exception/rejection text, absolute paths, audio, parameters, and node
names. The UI labels the retention and privacy behavior. A regression dispatches
a synthetic error containing a private audio path, verifies the persisted JSON
excludes that path, unmounts/remounts the App, and verifies the prior error
category and graph checkpoint are restored. If browser storage is disabled or
full, diagnostics remain available in memory for that run.

Validation on the final sanitized code: persistent-diagnostics regression
passed; typecheck passed; production bundle passed at
`%TEMP%\audiorouter-ui-build-review-0923d`; full UI suite passed 21 files / 297
tests; Playwright passed 11/11. Focused MCP stdio/backend (3), backend
diagnostic (3), and MCP activity (3) Rust test groups passed. No shell restart,
database/session inspection beyond read-only checks, or physical audio action
was performed.

## 2026-09-23 - browser layout and Recording tab review

The user reported that the canvas/right workbench were clipped by vertical
scrolling and that Recording showed raw-looking recorder and sample-rate
controls. Reviewed the disconnected React route harness in Chromium via
Playwright at 1280x720, 1440x900, and 1920x1080. Before the fix, the page hid
overflow while the workspace extended four pixels beyond the viewport: the
header is 58px but the workspace height subtracted 54px. The Recording form's
fieldset and labels had no workbench layout rules, so controls collapsed into
inline widths. The browser review confirms the canvas and tab rail fit inside
the viewport at all three sizes, with no document scrollbar. The recorder
fields now use stacked labels, rounded full-width inputs, and grouped settings;
the default recording ID is `voice-recording` and the selected rate is clearly
`48 kHz`.

Evidence screenshot: `%TEMP%\audiorouter-designer-review\recording-tab-current.png`.
This is a browser-only preview with a simulated backend; it does not prove the
native shell's effective DPI/window sizing or live recording behavior.
Validation on Windows 11: focused layout/Recording Playwright passed; full
Playwright passed 12/12; UI typecheck passed; Vitest passed 21 files / 297
tests; production build passed to `%TEMP%\audiorouter-ui-build-review-layout`;
`git diff --check` passed.

## 2026-09-23 - compact status viewport regression and endpoint E2E coverage

The status-view browser review at 1280x720 found that enabling the status strip
expanded the document to 2403px and left the graph at its old viewport zoom.
The workspace now reserves the header/status/workspace as bounded viewport
rows, the canvas graph viewport flexes within its panel, and a significant
React Flow viewport resize triggers fit-to-view (with reduced-motion duration
disabled when requested by the OS). The 1280x720 regression checks no page
scroll, panel containment, and that every graph node remains inside the graph
viewport. It also selects Properties while compact status is open and verifies
the inspector remains reachable without growing the page.

Expanded Playwright coverage adds an existing-virtual-input -> Gain ->
existing-virtual-output route, then plans and commits it through the route
harness. It checks that virtual-render-source and virtual-capture-sink entries
remain disabled and explain the deferred managed-driver requirement. This is
UI/contract-harness evidence only; it does not qualify installed endpoint
binding or physical audio.

Visual evidence: `%TEMP%\audiorouter-designer-review\workspace-status-1280x720.png`
and `workspace-status-properties-1280x720.png`. At 1280x720, document height is
720px; the canvas and all three graph nodes are within bounds in the normal
compact-status screenshot. The property inspector has its own scroll area at
this short height, while the canvas and top-level window remain fixed.

## 2026-09-23 - attended shell handover, route compatibility, and interaction review

The user-launched review UI initially connected to an older shell's default
control pipe. A named-pipe owner check identified PID 53576; that backend
returned the obsolete production-driver message and lacked the current
desktop recording grant. After both idle shells were stopped, the relinked
`audiorouter-shell-review-20260923.exe` owned the pipe and returned the current
status and grant. The saved `desktop-session` stayed at revision 4 with two
nodes and one edge. The final relinked executable SHA256 was
`D3409349C075F078D0FDDBB5557963C2EAA585B1DE54A65374B483560FADE11B`;
its current process at this check was PID 82224. The process was launched with
`AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` after the operator's earlier enrollment.

The original Test Signal -> Gain -> Output draft exposed a real compiler
compatibility defect. The UI had created Gain with one-channel ports between
two-channel endpoints. Browser planning accepted it; native Start rejected it
as `UnsupportedTopology`. Built-in processor defaults now use two-channel
ports, and the UI explains older incompatible routes in plain terms. The
native compiler still rejects channel-changing and branching saved routes at
Start; backend plan validation for those shapes remains follow-up work.

The guarded `tests/acceptance/m05-live-test-signal.ps1` check used exact
existing VB-Cable capture/render IDs, generated a separate Test Signal ->
Gain -> Physical Output session, and pumped audio for 10.01 seconds: 597 pump
calls, 3,553 processed quanta, zero XRuns. It stopped and detached the worker;
endpoint states matched before and after. The successful session
`m05-signal-1790226441` remains saved as “10 second Test Signal to Gain to
Output” and was prepared again, stopped, on the exact VB-Cable pair in the
review shell. Three failed, agent-created probe sessions were deleted after
exact ID/name and idle checks; `desktop-session` was not changed.

An attempt to prepare the Focusrite speakers returned
`IAudioClient::Initialize(render)` HRESULT `0x8889000A` (device in use). No
stream started on that endpoint. The successful VB-Cable render check proves
the native graph and pump path but does not prove audible speaker output or
the attended WebView animation. The user can select the saved test session
and press Play; live visual and audible acceptance remains open.

The interaction slice removed failed-Play/Start tab jumps, kept Tools open
across repeated adds, made node deletion immediate and undoable with Delete,
preserved full visibility of unselected paths, added connection tooltips and
distinct pause/add/remove icons, reduced the status and tab-rail footprint,
and changed Session to one picker plus on-demand Rename and Save route. Browser
screenshots were inspected at 1280x720, 1440x900, and 1920x1080, including
`%TEMP%\audiorouter-designer-review\workspace-complex-route.png`. Computer Use
could not attach to the Windows native pipe, so no screenshot of the running
desktop shell was obtained.

Windows checks: UI typecheck passed; production UI build and forced shell
relink passed; Playwright 16/16 passed; full Vitest 22 files / 310 tests
passed; documentation validation passed (58 Markdown files / 279 local
links); `git diff --check` passed. The Playwright route fixture checks
visibility and animation using simulated backend meters; the guarded native
pump independently checks real VB-Cable endpoints. Neither substitutes for
the user's attended visual acceptance on the current desktop window.

## 2026-09-23 - occupied Physical output connection diagnosis

The user reported that a newly added Test Signal would not connect to a
Physical output. The running review shell remained responsive as PID 82224.
Its `shell.jsonl` contained routine `events.subscribe` polling but no graph
request around the report. The backend JSONL had no current rows. This is
expected for a draft connection, which is handled in the renderer before any
plan/commit RPC. The renderer's console-only connection trace was unavailable
because Computer Use exposed no attached desktop app or browser.

A read-only, elevated `sessions.get` through the shell's local control pipe
showed `desktop-session` revision 4: `desktop-input` (Physical input,
`main:output:2`) already connects to `desktop-output` (Physical output,
`main:input:2`) by enabled `desktop-edge`. `appendDraftConnection` rejected a
second edge to that non-Mixer input with `That input already has a connection`,
as required by GRAPH-02. The user's new node and attempted edge were local
draft state and cannot be reconstructed from the saved session or shell log.
No session or audio endpoint was modified during this diagnosis.

Both canvas drag and keyboard/form connection actions now share the same
draft connection path. On an occupied input, the workbench names the current
source and offers **Replace input connection**. This removes the old edge and
adds the chosen source as one local, undoable draft change; it requires an
explicit **Save route** before persistence. A client diagnostic records the
occupied-input category without node names or parameters. A Mixer remains an
explicit graph operation for combining sources.

Windows verification: UI typecheck passed; full Vitest 22 files / 310 tests
passed after updating a stale message assertion; Playwright passed 17/17,
including an occupied Headphones input, replacement and Undo; production UI
build passed after an elevated retry to replace generated `ui/dist` files;
shell `cargo build --locked` passed after cleaning only the shell package;
docs validation passed (58 Markdown files / 279 links); `git diff --check`
passed. The new side-by-side executable is
`src-tauri/target/debug/audiorouter-shell-review-20260923-connection.exe`,
SHA256 `109B4A57B1D8B72098DFBBFEE3B36066A1767D47688E65A924483E684E135759`.
The existing shell was left running to preserve the user's unsaved draft; the
new binary has not been attended-tested on the current window. The immediate
old-shell workaround is to remove the existing input-to-output edge in the
draft, connect Test Signal to the freed output input, and save after review.

## 2026-09-24 Properties sizing, output summing, and Test Signal feedback

The shell log was checked again after the reported Play attempt. It contains
polling (`events.subscribe`) but no `session.start` call at the reported time,
so the click did not reach the backend lifecycle handler. The renderer had a
second gate: Test Signal Play was disabled while any graph plan was pending,
even when the draft already matched the committed session. That stale-plan
condition is removed; a route mismatch still produces explicit feedback rather
than starting a different saved graph. A native audition of an unsaved draft
is not yet implemented; endpoint workers are compiled from a committed session
and the current API cannot prepare/start a candidate graph without persisting
it. This remains an open API/runtime task, not a passing audio result.

Properties had a short workbench row and a separate selected-node row. The
Properties tab now keeps the workbench across the full sidebar and places the
selected-node controls below its tabs with an independent scroll region. For a
second source connected to an occupied Physical Output, the editor now inserts
an explicit visible Mixer into the draft and connects both sources to it. The
operation is undoable; graph validation and the existing explicit Mixer
contract remain authoritative. Native multi-input preparation currently binds
Physical Input and Application Capture sources; Test Signal or Audio File
inside such a mix still needs native source scheduling before it can be called
playable.

Verification on Windows: UI typecheck passed; Vitest passed 22 files / 311
tests; Playwright passed 18/18 including the new output summing and full-height
Properties checks. Vite production build passed to
`%TEMP%\audiorouter-ui-review-20260924` after the running shell locked the
existing `ui/dist` asset (`EPERM`). `git diff --check` passed. No native
endpoint was started and the current shell was left running. Browser results
validate interaction/layout only, not audible output.

### 2026-09-24 candidate preview follow-up

The previous paragraph is superseded on Test Signal preview behavior. The
shared `session.start` control method now accepts an optional candidate
session. The UI validates a dirty draft with `graph.plan`; the backend checks
the candidate's stored base revision and requires both SessionControl and
GraphWrite. It compiles the candidate into an already prepared single-endpoint
native scheduler without committing or changing the saved revision. Existing
`session.stop` ends the preview. Control tests verify authorization, stale or
unprepared-route rejection, and that preview does not change persistence; the
Playwright harness verifies the UI flow and unchanged saved revision. These
tests are not evidence of audible native playback. Multi-capture workers still
use the committed graph and do not preview Test Signal or Audio File mixed with
capture sources.

Current verification: UI typecheck; Vitest 22 files / 311 tests; Playwright
19/19; `cargo test -p audiorouter-control --locked` (184 passed, 4 ignored);
Vite production build; `cargo build --locked --manifest-path
src-tauri/Cargo.toml`; docs validation (58 Markdown files / 281 local links);
and `git diff --check`. The new side-by-side shell is
`src-tauri/target/debug/audiorouter-shell-review-20260924-preview.exe`, SHA256
`FE488467E3688B4ECAC3D5C0CBE0F3B07D72BD1DE87FFF0567085CDFEAEBA8AB`. The
existing shell PID 82224 remains running with the user's current draft; this
binary has not been launched to avoid interfering with that state. Native
audible acceptance remains open.

### 2026-09-24 workspace sizing and controls follow-up

Properties was reproduced in the compact status view: the compact canvas
spanned two rows while a later Properties rule created three, causing the
canvas and sidebar to shrink. The compact Properties layout now uses one
viewport row for the canvas and sidebar; the inspector overlays only the
sidebar below its tabs. Playwright compared both column heights before and
after opening Properties at 1280x720 with less than 3 px variation. Browser
screenshots of normal and compact Properties were inspected; the selected node
controls now begin directly below the tab rail, and the document does not
scroll vertically at 1280x720, 1440x900, or 1920x1080.

The header now shows Save, Play/Stop, and stopped/starting/running audio state.
Session retains session management and undo/revert actions without exposing
plan and revision steps. For an unprepared route, Play attempts preparation
using only the exact endpoint choices already made. Missing choices and
authorization failures remain visible as action messages. The Devices tab
explains why the current adapter needs a separate capture binding for Test
Signal and presents manual preparation/recovery controls. This is a known
runtime limitation: the Test Signal plus Physical Output route still needs a
selected capture endpoint until a render-only worker is implemented. Browser
fixtures prove the UI control path only, not audible native output.

Windows checks: UI typecheck, production Vite build, Vitest 22 files / 311
tests, Playwright 19/19, and a forced shell relink passed. The review binary is
`src-tauri/target/debug/audiorouter-shell-review-20260924-workspace.exe`,
SHA256 `AB2FF89D5B1C274F65EEE7A4DEB243064D9ACBAC65FD47C994724ACF03438FF2`.
The current running shell and draft were not modified. Computer Use returned
`{"apps":[],"browsers":[]}` despite shell PID 82224 still running; no safe
desktop handover or native Play in the new shell was possible.

### 2026-09-24 attended Play failure and exact Mixer repair

The updated workspace shell PID 51844 loaded `desktop-session` revision 11
(Physical Input and Test Signal -> Mixer -> Physical Output). Local
`shell.jsonl` recorded repeated `nativeEndpoints.prepare` errors, then one
successful preparation followed by repeated `session.start` `-32602` errors.
An elevated, read-only local pipe check showed the selected exact endpoints
were CABLE Output capture and Focusrite Speakers render. A direct start of the
saved graph returned `native graph rejected: UnsupportedTopology`. No active
session or native audio was reported. The UI's prior generic message incorrectly
suggested adding a Mixer or matching channel counts, although both were present.

A separate temporary candidate containing only Test Signal -> Physical Output
started natively as generation 10 for 10 seconds, with
`system.diagnostics.audio.state=available`, then stopped. The saved graph stayed
at revision 11. This proves a native start/stop and availability report, not
that the user heard a tone; listening confirmation remains pending. Calling
`audioSources.transport` during an earlier probe returned “audio source is not
prepared,” because that API controls Audio File, while Test Signal emits on
session start. That first preview was stopped immediately.

The engine now prepares the exact saved two-source stereo shape by composing
its explicit matrices and adding a bounded Test Signal stage to the one
physical capture block before the output meter. Capture and Test Signal each
have source meters, so incoming edge activity can be attributed without
using post-Mixer output as evidence for a silent branch. It still rejects unhandled
third sources. The shell log records stable topology reason and structured
audio error kind/HRESULT/retryability without arbitrary error text. Commands
run on Windows: engine 125 passed, control 184 passed/4 ignored, shell logging
regression passed, Vitest 311 passed on a full rerun, Playwright 19 passed,
TypeScript/Vite review build passed, shell debug build passed, and
`git diff --check` passed. An initial concurrent full Vitest run had one
transient lifecycle timeout; that test passed alone and in the subsequent full
run. The new shell is
`src-tauri/target/debug/audiorouter-shell-review-20260924-mixer.exe`, SHA256
`463F79AC1E85B0356203A60FCEBA6336220991031877CED4CF6E817685B44456`.
The old shell remains open until its draft is preserved, so the new mixed route
has not yet received attended native playback evidence.

Attended continuation: the user closed the old shell, then the review shell
PID 60912 opened the same default database and loaded desktop revision 11.
`nativeEndpoints.prepare` succeeded for the selected pair. The shell log
recorded `session.start` successful as native generations 1, 2, and 3, with
matching successful `session.stop` calls; the first run stayed active for
about 24 seconds. The user explicitly reported hearing the Test Signal and
seeing an orange connection with an arrow. The exact moving-dash and
volume-width behavior is still being checked separately. A prior direct RPC
preview's silence was caused by the diagnostic script not calling the native
endpoint pump; that script's `audio.state=available` alone was not an audible
qualification. The ordinary UI does call the bounded pump while running.

## 2026-09-24 installed virtual endpoint tool clarification

The tool shelf exposed two input cards and two output cards, but each pair
created the same `physicalInput` or `physicalOutput` graph kind. The labels
made `Voicemeeter Input` (Windows playback) look selectable as an AudioRouter
capture endpoint. Consolidated each pair into one device card, and added
capture/render guidance in the tooltips, input binding picker, and quickstart.
Existing saved graph kinds and endpoint bindings are unchanged. The current
native path can take the Voicemeeter B1 capture endpoint; direct render
endpoint loopback remains unqualified in this shell.

Windows verification: `npm.cmd run typecheck` passed; `npm.cmd test` passed
22 files / 310 tests; `npm.cmd run e2e` passed 19/19 Chromium checks;
`npm.cmd run build -- --outDir dist-review-20260924-device` and
`cargo build --manifest-path src-tauri/Cargo.toml --locked` with that asset
directory passed; docs validation passed 58 files / 282 local links;
`git diff --check` passed. The first UI build failed `EPERM` creating the
review output directory and succeeded when rerun elevated. This is UI and
build evidence only; Voicemeeter B1 playback into the Focusrite has not been
attend-tested. The earlier running shell was left open and unchanged.

## 2026-09-24 Voicemeeter transition invalidated stopped worker

After Voicemeeter opened, the attended shell PID 60912 logged two
`session.start` failures with `kind=deviceInvalidated`, HRESULT `0x88890004`,
and `retryable=true`; recent `devices.list` calls succeeded. There was no
`nativeEndpoints.rebind` between the inventory refresh and those starts.
The UI had skipped preparation whenever a stopped worker was attached,
regardless of whether its WASAPI clients were invalidated or the user had
selected another exact endpoint pair. This is a likely root cause, although
the log did not identify which client rejected Start.

The UI now rebinds the attached stopped worker to the selected exact IDs
before Play, and gives an actionable invalidation message. A component
regression asserts rebind occurs before start; error formatting has a focused
regression. Windows checks: typecheck passed; Vitest 22 files / 312 tests
passed; Playwright 19/19 passed; production frontend and shell builds passed;
docs validation passed 58 files / 282 links; `git diff --check` passed.
The final side-by-side shell is
`src-tauri/target/debug/audiorouter-shell-review-20260924-rebind.exe`,
SHA256 `E21E65C8F2D12419ACABF81027BBB8892607FC29FB0F80BBF585EA01B5B99CD7`.
No live audio success is claimed for this build yet. The current shell remains
open; it is unsafe to replace it without handing over possible unsaved edits.

## 2026-09-24 route Play and Test Signal transport

The top Play previously started the route and made Test Signal emit by default.
The engine now starts Test Signal silent and exposes atomic per-source
play/stop state. `audioSources.transport` controls the prepared tone in a
running route. Canvas node Play may start a stopped route first; node Stop
stops only that tone. Route Status no longer duplicates top Play/Stop.

Windows evidence: `cargo test -p audiorouter-engine --locked --quiet` passed
125 tests; `cargo test -p audiorouter-control --locked --quiet` passed 185 tests
with 4 ignored; UI Vitest passed 312 tests; `npm.cmd run e2e` passed 20/20
Chromium checks, including separate route/source transport; contract drift
passed; production frontend and Tauri builds passed; docs validation passed
58 files / 282 links; `git diff --check` passed. The initial frontend build
was denied while creating its output directory and succeeded elevated. RTK
was unavailable. Side-by-side shell:
`src-tauri/target/debug/audiorouter-shell-review-20260924-routeplay.exe`
(SHA256 `3FEB9648FF67727E54D0C0ED790B7D6B298B0455ACA5BCBF53EFFE7BDF415BE4`).
The earlier shell was left open because its canvas may contain unsaved work.
Live audio from this build awaits an attended handoff and listening check.

## 2026-09-24 Advanced EQ from supplied visual reference

Inspected `C:\Users\miste\Downloads\EqualizerSteelseriesGG.png`: a dark
log-frequency graph, coloured filter points, a combined response trace, and
precise filter controls. The implementation promotes the existing parametric
processor as Advanced EQ while retaining `parametricEq@1` persisted identity.
The bounded audio stage, node validator, discovery catalog, and response API
now accept sixteen bands. Properties exposes point add, graph double-click,
selection, pointer drag, six filter shapes, precise frequency/gain/Q controls,
and point removal. Pass and notch filters do not expose an inapplicable gain
field. The response is supplied by the backend coefficient implementation.
The domain's per-node parameter bound increased from 64 to 96 because the
full default EQ map has 83 parameters; a focused validation regression passes.

Windows results: DSP 34, domain 67 including the full-EQ regression, engine
125, control 185 (4 ignored), UI Vitest 314, and Playwright 21/21 passed,
including an actual browser point drag; contract drift, production UI/Tauri
builds, docs validation (59 files/284 links), and `git diff --check` passed.
`cargo fmt --all -- --check` reports existing formatting diffs elsewhere in
the dirty control/engine files and was not applied globally. The inspected
[browser screenshot](advanced-eq-browser-preview.png) uses a deterministic
in-memory audio harness; its curve is simulated, while production uses
`processors.response`. The side-by-side review shell is
`src-tauri/target/debug/audiorouter-shell-review-20260924-advanced-eq.exe`
(SHA256 `4FAA8877A6B28DE9355D2CFD064F4FFE0B684565CBC9B4E2BA1346555D7DB56E`).
No attended hearing or live response check was performed. No shell was open
before launch; the updated review shell was started as PID 63692 and is
responding.
