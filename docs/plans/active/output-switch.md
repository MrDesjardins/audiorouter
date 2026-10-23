# Output Switch — active implementation plan (2026-10-22)

## Objective

Add an Output Switch routing node with one mono/stereo input and two named
outputs, A and B. It sends the input to exactly the selected output side. The
user can switch sides in Properties and through the existing generic node
toggle, including Stream Deck. Add the tool to the public catalog and generate
a visual guide.

- Requirements: GRAPH-01/03/08/09/17, DSP-21, UI-05/07/20/25, AUTO-15.
- Branch: `feature/output-switch`, based on `origin/main` (`e6e575ff`) in
  `C:\code\audiorouter\.worktrees\output-switch`.
- Prerequisites: Input Switch has a persisted A/B parameter and live toggle.
  Existing graph execution supports output fan-out but its processor compiler
  represents a node with one shared output block; Output Switch needs explicit
  output-port routing semantics. The public tool catalog is generated from
  `site/guide-content/processing.json` and `site/generate-guides.mjs`.

## Decisions

- The node has one `in` input and two `a`/`b` output ports with matching channel
  counts. `selected` defaults to `a`; only edges leaving the selected port
  carry audio.
- Either side may fan out to multiple destinations. A and B are exclusive:
  switching never sends audio to both sides. An unconnected selected side is
  silent and never falls back.
- To avoid a click without leaking a transition copy to both destinations, the
  old side fades down to silence, then the new side fades up (10 ms each).
- `selected` is an ordinary persisted graph parameter. Generic `nodes.toggle`
  and Stream Deck two-choice handling should work without a new RPC.
- This is graph routing, not temporary monitoring: it switches where one
  signal is sent, but does not select an arbitrary mix to hear in headphones.
- No PR until the owner tests the local build and approves the exact branch
  and changes.

## Ordered tasks

1. Add DSP-21, GRAPH-17 and UI-25 contracts; update this plan, `current.md`,
   and the Wave Link parity list.
2. Add `outputSwitch` to domain registry, defaults, validation, backend catalog
   and versioned contracts; preserve compatibility with existing sessions.
3. Implement selected-port routing in the engine, with a bounded lock-free
   break-before-make transition that survives live graph replacement.
4. Extend graph validation and topology compilation for one input and two
   outputs; reject malformed directions, widths and connection shapes.
5. Add tool library/draft support, canvas visuals, accessible inspector
   controls, labels and connection names.
6. Verify generic `nodes.toggle`/Stream Deck discovery for A/B; add coverage
   where existing two-choice tests do not cover the new node.
7. Regress exclusivity, switching while playing, disconnected sides, fan-out,
   invalid graphs, disable/bypass, persistence, undo and existing routes.
8. Add an Output Switch visual guide and complete tool-catalog entry; generate
   local site assets and run site checks. Do not publish.
9. Run Rust format, Clippy and relevant tests; UI format/lint/typecheck/full
   tests/build and Edge checks in dark/light/high contrast; docs/site
   acceptance and complete diff review.
10. Build the local Windows shell and sidecars in a task-specific target
    directory; give the owner the full path and wait for local testing before
    committing, pushing or opening a PR.

## Validation matrix

| Check | Acceptance | Evidence |
| --- | --- | --- |
| Domain/control | Defaults, selected-side validation, catalog ports and generic toggle discovery | Unit and control tests |
| Engine | Selected output only; old-side ramp reaches silence before new-side ramp; missing side silent; fan-out works independently | Deterministic blocks and compiled graph tests |
| UI | Add/connect/save/undo; switch A/B while session runs; accessible selection and stable layout | Vitest and Edge route harness |
| Website | Output Switch appears in all-tools catalog and guide index; page explains and illustrates port behavior | Guide generation and site acceptance |
| Build/quality | Rust fmt/Clippy/tests; UI format/lint/typecheck/tests/build; Windows shell and sidecars | Local commands; CI after owner-approved PR |
| Owner acceptance | Toggle a real route between Headphones and OBS; selected endpoint receives audio and inactive endpoint stays silent | Owner's Windows devices and route |

## Risks

- Port-aware routing can affect compilers that consume graph output ports;
  regression coverage must include existing fan-out, Mixer, Input Switch,
  Recorder, Network Send and physical/virtual outputs.
- Break-before-make gives a brief silent interval. It preserves exclusive
  routing and avoids transiently sending audio to OBS when Headphones is
  selected. Measure the transition and prove there is no overlap.
- Existing sessions remain valid. The new node uses type/schema version 1 and
  needs no migration.
- A missing selected output stays silent; never select a fallback.
- Site changes are local generated assets only; publication is separately
  authorized.

## Rollback

Revert the scoped domain, control, engine, contract, UI, spec, guide/catalog,
and tests together. No migration, driver change, installer, publication, push
or PR is included.

## Progress

- Implementation is complete on `feature/output-switch`. The local Windows
  shell and sidecars are built at
  `C:\code\audiorouter\.worktrees\output-switch\src-tauri\target\debug`.
- Domain, control and engine coverage passes, including A/B exclusivity,
  break-before-make switching, a disconnected selected side remaining silent,
  and routing a connected Mixer convergence to only the selected output.
  Workspace and `src-tauri` Clippy passed with `-D warnings`; both Rust format
  checks pass. Domain/control tests passed (339 passed, 9 ignored), and the
  full engine suite passed before the final regression-only extension; the
  extended transition regression and connected-Mixer regression both pass.
- UI verification on latest main (`e6e575ff`): format check, lint (0 errors; 25 pre-existing
  `exhaustive-deps` warnings), typecheck using the branch contract source,
  production build, and Vitest (644 passed). Edge workflow and theme checks
  report success for Output Switch add/save/reload and dark/light/high-contrast
  rendering. In both Edge runs, all tests reported success but the process
  remained alive during cleanup and was stopped manually.
- Stream Deck tests pass (26 tests); its TypeScript typecheck passes using the
  UI's installed TypeScript compiler. Site guide generation and acceptance
  pass (37 tool pages, 140 property anchors, 917 local links/assets); site
  quality checks pass. The visual guide is generated locally and not published.
- Local shell build and plugin worker build pass. Staged CLI hash matches its
  build output. The repository worktree's `node_modules` are junctions to the
  main checkout, so UI typecheck/build explicitly mapped the contracts import
  to this branch's source; the temporary mapping has been removed.
- User acceptance is still pending: launch
  `C:\code\audiorouter\.worktrees\output-switch\src-tauri\target\debug\audiorouter-shell.exe`
  and test a real source toggling between Headphones and OBS. The plugin worker
  and CLI sidecars are in the same directory. This build does not establish
  real endpoint/audio behavior. Commit `834a327d` records the implementation
  on the local feature branch; nothing has been pushed and no PR has been
  opened.
- The owner requested a local-main merge. It is deferred because the shared
  `main` worktree is 32 commits behind `origin/main` and has extensive
  uncommitted edits in overlapping files. Preserve that worktree. Once those
  edits are committed or otherwise cleared by their owners, update local main
  and fast-forward it to `feature/output-switch` (main is currently an
  ancestor). Do not push or open a PR without the owner's approval.
- Next: owner tests the built app and reports any issues; then complete the
  local-main fast-forward once the shared main worktree is safe.
