# Editor state invariants

Requirements: UI-02/11/16/17, ENG-01/02. These invariants describe the editor's
existing ownership boundaries and guide regression tests. They do not grant
the renderer graph validation or audio authority.

## Ownership and lifetime

- The backend owns saved graph identity, revisions, parameters, connections,
  authorization and runtime state. A draft edit uses backend plan/commit before
  becoming authoritative. A graph undo cannot be replaced by a layout undo.
- Per-session local presentation owns group annotations, node coordinates,
  requested sizes and connector sides. Editing it never changes routing,
  graph revision, recording state or the Save route indicator.
- Layout history belongs to one visible session and is bounded to 20 entries
  in each direction. Switching sessions clears it; an old session snapshot or
  gesture cannot apply to another session. History snapshots own their arrays
  and nested values; later caller edits cannot modify an undo point.
- Selection, open menus, measurements and in-progress gestures are transient.
  A gesture freezes ownership and initial geometry. The app-local tool
  clipboard has window lifetime and deliberately survives session switching;
  pasted nodes receive new IDs and omit connections and sidechain references.

## Commands and restore

- Drag, resize, alignment, distribution, size matching, connector-side edits
  and Arrange use one layout history path. Undo arrange is a temporary shortcut
  to that path, with the same redo semantics. A later edit or history restore
  retires the shortcut.
- A no-op never adds a history entry or clears redo. Missing connector-side
  preferences mean the same as explicit right-source/left-target defaults.
  A real new layout edit clears redo. Changing group lock/carry options is a
  presentation setting; history does not revert the user's current lock/carry.
- Undo and redo restore only geometry/preferences for objects still present
  with the same identity. They cannot resurrect deleted nodes/edges or discard
  newer objects and their preferences. Backend-created IDs identify objects;
  presentation code must use own dictionary keys, including unusual IDs.
- A restore that would move or resize a currently locked group rejects the
  whole command and leaves both history stacks intact. Unlocking permits retry.
  Movement/size commands preserve locked frames and usable content minima.
- Membership means full enclosure by the smallest containing group, with
  stable ID tie-breaking. Strictly larger parent area prevents cycles. Moving
  a carrying ancestor translates each descendant once, preserving offsets;
  selecting a child too must not double its displacement. A locked descendant
  blocks carrying its ancestor. Resizing a frame leaves contents still.

## Failure boundaries and evidence

Optional layout storage reads/writes may throw. Null, scalar, array, malformed,
out-of-range and legacy values must not crash the editor. Keep valid entries
where possible, bound parsing/geometry, and never spread unchecked persisted
fields into geometry identities. Import validation remains backend-owned.

Executable coverage:

- [Pure history model](../../ui/src/canvasHistory.test.ts): immutable snapshots,
  session rejection, no-op/redo, atomic lock rejection, surviving/new/deleted
  identities and prototype-shaped keys. Five fixed seeds each run 400 actions
  against a separate coordinate/connector reference model. A failure prints
  seed and action prefix. Retain a minimal failing sequence as a named test.
- [Geometry](../../ui/src/canvasGeometry.test.ts): nested/overlapping groups,
  carrying, bounds, locks, alignment, size and distribution.
- [Persistence](../../ui/src/layout.test.ts) and
  [groups](../../ui/src/CanvasGroups.test.tsx): malformed/legacy data and
  per-session ownership.
- [Browser history](../../ui/e2e/canvas-layout-review.pw.ts),
  [session transitions](../../ui/e2e/canvas-layout-session.pw.ts),
  [layout](../../ui/e2e/canvas-layout.pw.ts) and
  [clipboard](../../ui/e2e/canvas-clipboard-regressions.pw.ts): real adapter and
  event interleavings. Three-theme scenarios establish browser appearance;
  packaged WebView2, Narrator and real audio remain distinct evidence.

Remaining Q2 work is tracked in the [quality plan](../plans/active/application-quality.md):
broader bounded persistence/import corpus, generated nested geometry, rotating
nightly seeds and fault/mutation demonstrations. A test count alone does not
establish those unrun gates.
