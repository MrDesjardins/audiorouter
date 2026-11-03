# UX-05 Workbench accessibility slice

Current status (2026-11-02): source integrated as `9831785f` in parent
`bf99cf97`. The gaming-time restrictions below describe the completed child
assignment; parent qualification now follows the final consolidation request
in [current.md](current.md). Narrator, attended keyboard/scaling and visual
gates remain open. This child performs no native build or integration.

## Objective and authority

Implement standard keyboard tab navigation for UI-11/12/17 under M05/M08.
Owner explicitly authorized this isolated implementation while gaming. Never
contact the live backend, profiles or native app; no native builds or audio work.
Source: read-only parent UX improvement plan dated 2026-11-02, UX-05.

## Prerequisites and decisions

Reviewed repository instructions, documentation map, current execution record,
M05/M08, interface requirements and delivery decisions. Existing tabs have no
roving index or keyboard handler. Properties is a separate inspector overlay;
preserve its layout and backend authority. No API or contract changes.

## Ordered work and verification

1. Add tab IDs, panel relationships, roving focus and Left/Right/Home/End.
2. Preserve focus on tabs during activation and on existing controls during
   telemetry/error rerenders; do not steal focus on external navigation.
3. Add focused mock keyboard regression tests; typecheck promptly.
4. Audit narrow/scaled CSS; change only demonstrated source constraints.
5. Format, lint, inspect diff and commit this slice only.

Validation: single-worker Workbench mock tests, UI typecheck, formatting and
lint in this worktree. No live data or native artifacts. Narrator, attended
keyboard build/save/play/stop/export/recovery, 1280x720 at 200%, three-theme
visual qualification and reduced-motion attended gates remain unrun.

## Risks, rollback and next action

Properties lives outside Workbench; integration must retain a valid association.
Mock DOM cannot prove scaling or screen-reader behavior. Revert this slice's
commit to roll back; no storage migration. Bounded source checks are recorded below.

## Implementation and evidence — 2026-11-02

Implemented automatic Left/Right activation with wrap, Home/End, one tab stop,
stable IDs and labelled focusable panels. The real Properties overlay (node,
group and multi-selection variants) is associated rather than its hidden
placeholder. Because it precedes tabs in DOM order, Tab enters that panel and
Shift+Tab from the panel returns to Properties. Surviving controls retain focus;
removing a focused Workbench control restores focus to the selected tab.

Source audit: tab buttons forced height 2.4rem with overflow hidden, a concrete
clipping constraint for wrapped labels. Retain the minimum height and allow
wrapping/height growth on deliberate resizing. Existing sidebar overflow and
900px stacked layout remain. No wider scaling claims follow from this audit.

Windows source-only checks: UI typecheck and typecheck:all passed; format and
zero-warning lint passed. Workbench single-worker tests passed 4/4; adjacent
ApiPanel, CanvasGroups and CanvasLayoutInspector tests passed 11/11 (14 total
with the earlier 3-test Workbench run). git diff --check passed. PowerShell npm
shim initially rejected execution policy; npm.cmd worked. Dependency junction
reads the installed parent node_modules; Vite cache explicitly stays inside
this worktree. No native builds, backend calls, app launch or profile reads.

Integration: changes include small panel-semantic edits in ApiPanel,
PropertiesPanel, CanvasGroups and CanvasLayoutInspector plus shared
workbenchPanel.ts; retain these when merging overlapping slices. No REST/MCP
change. No processing-tool control changed, so guide recordings are unaffected.
All attended/visual gates listed above remain unrun. Next: parent performs
disposable-profile three-theme/scaling and Narrator
qualification when the owner is ready. This does not complete M05/M08.
