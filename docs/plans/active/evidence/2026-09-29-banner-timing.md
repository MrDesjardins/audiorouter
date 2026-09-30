# Banner actions and Timing layout regression — 2026-09-29

Requirements: UI-03, UI-08, UI-12 (M05). Affected build:
`target/mixer-topbar-20260928/release/audiorouter-shell.exe`.

User reproduction: Siege Advanced EQ → Mixer 1 showed an occupied EQ-input
notice with unclickable Replace and × controls. Warning/action notices overlapped
the workspace; opening Timing created a large gap above the canvas. Screenshot:
user's Downloads/2026-09-29_18-44-36.png.

The shell assumed three grid rows despite rendering several notices. Sidebar
content also contributed to automatic grid sizing. Notices now have separate
space; the workspace uses the remaining height and sidebar rows scroll. The
screenshot described Mixer → EQ, so guidance explains receiving blue input →
sending orange output. Dismiss clears pending replacement without changing the
graph. Replacement remains explicit and undoable. No audio/schema changes.

## Verification

Windows, Edge/Playwright, real App/React Flow with deterministic route harness;
no audio devices opened. Native playback remains attended.

- `npm.cmd test`: 31 files, 379 tests passed.
- `npm.cmd run typecheck`: passed; final UI build also typechecked.
- Direct Playwright with separately started Vite and temporary configuration
  (`reuseExistingServer: true`, one worker): seven targeted tests passed (18.4 s),
  covering topbar, generic output fan-out, EQ/Mixer/output and four new
  regressions. Four new regressions passed again after final guidance adjustment
  (9.1 s). Temporary configuration removed and owned Vite stopped.
- New regression: occupied EQ gesture; warning/notice geometry; ordinary
  Replace/× clicks without force; original input retained on dismissal;
  replacement Undo; EQ → Mixer in all themes. Twelve-output Timing list verifies
  unchanged canvas position/height and sidebar scrolling.
- Initial test failures: incorrect warning/Undo selectors, expecting dismissal
  to remove permanent stopped-audio status, reading moving handles after Undo.
  Corrected selectors/expectations and waited for stable handles using hover.

Visual evidence inspected at 1280 × 720:
[dark banners](2026-09-29-banners-dark.png),
[light banners](2026-09-29-banners-light.png),
[high-contrast banners](2026-09-29-banners-high-contrast.png),
[dark Timing](2026-09-29-long-timing-dark.png),
[light Timing](2026-09-29-long-timing-light.png),
[high-contrast Timing](2026-09-29-long-timing-high-contrast.png).

## Build and rollback

Passed commands:

- `npm.cmd run build -- --outDir ../target/banner-timing-20260929/ui-dist` (UI cwd).
- `cargo build --manifest-path src-tauri/Cargo.toml --release --features custom-protocol --locked --target-dir target/banner-timing-20260929`
- `cargo build --release --locked -p audiorouter-plugin-host --bin audiorouter-plugin-worker --target-dir target/banner-timing-20260929`

Final JS `index-CYKZ7a-U.js` and CSS `index-DqTlWwkw.css` asset names verified
inside the shell. UI index: 2026-09-30 01:58:29 UTC; shell: 01:59:43 UTC.
Frontend configuration temporarily selected isolated output and was restored.
Initial non-elevated output failed target-directory EPERM; elevated build
succeeded. Initial worker command incorrectly named CLI package; corrected
to plugin-host and succeeded.

Shell SHA256: `6EAF1AA3041FE2437EECC933713E765F2D55AA2B5B771289D9ACAD2DBB4E8327`.
Worker SHA256: `B9508F6834BF0EAAD0E875A1C3A313944F8FEC9D6DD70A17A83198413EC441BE`.

User's existing shell (PID 31304 when checked) remains running. Next: attended
manual acceptance of
`C:\code\audiorouter\target\banner-timing-20260929\release\audiorouter-shell.exe`.
Close old app first; keep adjacent plugin worker. Rollback: unchanged
`mixer-topbar-20260928` pair; no database migration required.
