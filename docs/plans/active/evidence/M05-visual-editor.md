# M05 visual editor evidence

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
