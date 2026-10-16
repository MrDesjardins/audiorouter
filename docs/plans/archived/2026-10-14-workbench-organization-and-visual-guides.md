# Workbench organization and visual tool guides

Date: 2026-10-14 (owner-requested UI work; final build completed 2026-10-15 UTC)

## Objective and requirements

Replace the crowded Setup area with focused Status, Virtual Cables and UI
Options tabs; move device access/version checks to Advanced; simplify Recording
empty-state and local stdio MCP setup; and provide a visual, accessible,
individual guide for every configurable tool with contextual in-app access.
Continue the compact-workspace work already authorized in this task.

Requirements: UI-01/04/05/07/09/10/11/12/15/17/18 and owner-authorized UI-19
(workbench organization and user themes) and UI-20 (visual tool guides).
Specifications: `docs/spec/09-ui.md`; existing M05/interface requirements.

## Decisions and implementation

- Workbench tabs are Tools, Properties, Timing, Session, Status, Virtual
  Cables, UI Options, Recording, Advanced, MCP, API and Logs.
- Status contains six readiness checks and the local audio-device list.
  Virtual Cables owns cable configuration and usage guidance. UI Options owns
  theme selection, user-defined palettes, JSON import/export and connection
  animation preference. Advanced contains a collapsed device-access and
  version-check section.
- User themes use a bounded versioned JSON shape with a token allowlist and
  hex-only colors. Invalid imports do not replace the active theme; export
  contains presentation data only.
- Recording's no-files state sits inside the recordings library. Local stdio
  MCP setup uses a short app-choice, naming/permission, copy-instructions and
  confirmation flow.
- The previous compact-workspace changes remain: bottom-right message popover,
  tighter Canvas header spacing, shared question-mark help, explicit Save
  affordance for topology/name changes and inline node title editing.
- Static `site/tools/` contains one guide for each of 35 supported tool kinds
  (including Group), sourced from the UI controls/defaults. The index supports
  search; guides cover 101 property anchors, examples, constraints and
  configuration actions. Contextual links are allowlisted in both UI and
  native shell. Built checkout links open the corresponding local guide;
  installed builds use the canonical public guide URL.
- Guide animations are local WAAPI illustrations with static/reduced-motion
  states, Play/Pause/Replay, keyboard controls, and offscreen pause. They
  describe concepts and make no live DSP or performance claims.
- Website source and app links are ready for review. No push, deployment, or
  public content change was made or authorized by this task.

## Changed areas

Workbench components/styles and tests under `ui/src/` and `ui/e2e/`, including
the updated `ui/e2e/virtual-cables.pw.ts` layout selector for the moved device
inventory; tool guide
link integration in `ui/src/ToolGuideLink.tsx` and native allowlist in
`src-tauri/src/tool_guides.rs`; guide content, generator, checks, and assets in
`site/`; UI contracts in `docs/spec/09-interface.md`; `site/README.md` and
`src-tauri/README.md`. The compact-workspace history and earlier header/tray
work are preserved in the active plan and prior execution record.

## Validation evidence

- `node node_modules/vitest/vitest.mjs run --configLoader runner --exclude e2e/**`
  from `ui/`: 535 tests passed in 61 files.
- Sequential Edge run in `ui/` using the local compact-check Playwright config
  for `workbench-organization.pw.ts`, `compact-workspace.pw.ts`,
  `header-canvas-usability.pw.ts`, `banner-layout.pw.ts`,
  `canvas-groups.pw.ts`, `live-bypass-draft.pw.ts`,
  `shortcut-after-save.pw.ts`, `device-access-consent.pw.ts`,
  `update-notice.pw.ts` and `site-guides.pw.ts` with `--workers=1`:
  42 passed. Coverage included all three themes, theme import/export and
  persistence, every guide offline, keyboard/reduced-motion animation
  controls, search, compact notices/save states, and contextual guide links.
- Focused `virtual-cables.pw.ts` rerun with `--workers=1`: 4 passed. The old
  layout regression looked for the device list below the cable panel; moving
  that list to Status made the selector stale. It now tracks the cable route
  section inside the Virtual Cables panel and passes for all three themes.
- `node site/check-guides.mjs`: PASS, 35 pages, 101 property anchors, 906 local
  links/assets, 35 catalog guide links, and parity with UI/native allowlists.
- Site guide browser audit: 3 cases passed for every guide offline, animation
  controls/reduced motion/mobile layout, and searchable index. Manual Edge
  inspection confirmed no JS errors and no horizontal overflow in sampled
  desktop/mobile guides.
- `npm --prefix ui run format` followed by format check: PASS. `npm --prefix ui
  run lint`: zero errors (25 existing exhaustive-deps warnings). UI
  `tsc --noEmit`: PASS. UI production Vite build: PASS.
- `cargo test --manifest-path src-tauri/Cargo.toml --locked --bin
  audiorouter-shell`: 82 passed, 4 ignored. Shell Clippy and workspace Clippy
  with `-D warnings`: PASS. Both Rust formatting checks: PASS.
- Release custom-protocol shell build with pinned locked dependencies:
  `cargo build --manifest-path src-tauri/Cargo.toml --target-dir
  src-tauri/target --release --features custom-protocol --locked`: PASS.
  The resulting executable timestamp is later than `ui/dist/index.html`.
  No running shell was stopped or launched; the rebuilt app remains ready for
  owner testing at `src-tauri/target/release/audiorouter-shell.exe`.
- No hardware audio, actual driver, assistive technology/Narrator, public
  website deployment, or in-app installed-package guide launch was qualified.

## Risks and rollback

The browser suites use their test backend and do not prove physical audio
behavior or OS accessibility. Guide illustrations are conceptual. Public
links are only live after separately authorized website publication. Revert
this task's listed UI, shell, `site/` and spec files to roll back; the local
presentation preference is optional and versioned, with no session migration.
Keep the prior shell binary as the rollback build.

## Handoff

Owner can review the rebuilt desktop executable and local site guides. The next
task is owner review and a separate decision about publishing the website.
