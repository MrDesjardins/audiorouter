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

Added Vitest fake-backend coverage in `ui/src/backend.test.ts`: four tests
verify disconnected read-only snapshots, empty event cursors, stale snapshot
retention after a failed refresh, revision-preserving node drafts, and unknown
node rejection. `npm run test`, typecheck, production build, and high-severity
audit all pass.
