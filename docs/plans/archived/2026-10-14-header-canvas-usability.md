# M05 maintenance — header, canvas legend and mute controls

Date: 2026-10-14. Implementation and browser verification complete; this does
not close the M05 attended accessibility or packaged-app gates.

## Objective and authorization

The owner requested four UI fixes: make Audio running/stopped look like status,
open Session when the top-left session name is clicked, collapse the canvas
legend initially, and remove the repeated running sentence. Follow-ups explicitly
requested removing Mute mic from the header and Toggle mute from the tray.
Requirements: UI-04/09/10/11/12/15/17. Initial working tree was clean.

## Implementation and decisions

- `ui/src/App.tsx` and `styles.css`: plain text plus filled/hollow dot, reserved
  status width, accessible session-title button, and no header mute button.
- `SessionFlowCanvas.tsx` and `SignalFlowPanel.tsx`: port instructions and line
  samples share one native Canvas legend disclosure, initially closed. Mouse
  and keyboard expansion remain available.
- Running sessions without action feedback have an empty, hidden message slot.
  Its fixed height remains reserved for UI-17; stopped setup guidance, errors,
  action/dismiss controls and temporary-preview information remain available.
- `src-tauri/src/main.rs`: removed the privacy menu item and its event handler.
  Backend privacy mute, the Properties control, shortcut and state reporting
  are retained. No migration, audio routing, endpoint or permission changes.
- Updated the [interface contract](../../spec/09-interface.md), shell README,
  affected component/browser checks and the new
  `ui/e2e/header-canvas-usability.pw.ts` regression.

## Verification and evidence

Environment: Windows host, Node v22.22.3, pinned Rust 1.96.0, installed Edge at
`C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe`. Commands used
the installed package CLIs directly through Node because the shell runner
failed during setup. No dependencies were installed or versions changed.

Passed checks:

- UI Prettier write and check on `src e2e`; only task files changed.
- UI ESLint: 0 errors, 25 existing reviewed hook-dependency warnings.
- `tsc --noEmit` and `vite build --configLoader runner`: pass. Vite retains its
  500-kB bundle-size warning; the new production bundle is in `ui/dist`.
- `vitest run --configLoader runner --exclude e2e/**`: 58 files, 527 passed.
  Focused App/canvas run also passed 149 tests after expectations were updated.
- `cargo test --manifest-path src-tauri/Cargo.toml --locked`: 80 passed,
  4 ignored. Ignored tests are not claimed as evidence.
- `cargo fmt --all` and shell-manifest fmt, followed by both `-- --check` runs:
  pass; the Rust diff contains only the tray removal.
- Workspace and shell `cargo clippy --all-targets --all-features -- -D warnings`
  (workspace also uses `--workspace`): pass.
- Final Edge suite: 13 passed in 17.7 seconds. Covers the new header/canvas
  regressions, top-bar status check and existing banner-layout regressions.
  Mouse/keyboard Session activation, initial legend hiding, increased canvas
  height when collapsed, status width and canvas stability through Stop/Play,
  message dismissal and actual fresh-fixture Play failure all pass.
- Dark, light and high-contrast screenshots were visually reviewed, including
  the expanded high-contrast legend. Tests write via `testInfo.outputPath`.
- Documentation links and `git diff --check`: pass.

Local logs: `%TEMP%/audiorouter-ui-fixes/` (`edge-combined.log/json`,
`ui-final.log/json`, `shell.log/json`, `clippy.log/json`,
`workspace-clippy.log/json`). Screenshots:
`%TEMP%/audiorouter-header-results/`, including each theme's running,
expanded-legend, stopped-feedback and Play-failure images. These local temporary
artifacts are not committed or guaranteed permanent; this record and the
committed checks preserve reproducible outcomes.

The final browser command selected `header-canvas-usability.pw.ts`,
`banner-layout.pw.ts` and `audio-tools.pw.ts` with grep
`header status|Play failure|top-bar audio status|sender-first Siege|long Timing|occupied input notice`,
`--workers=1 --timeout=15000`. Its temporary config used the repository config
with port 4201, the installed Edge executable, the above output directory and
an existing test-owned Vite server. The temporary config and server were removed.
To reproduce, apply those overrides to a temporary config and run the same
selection with `node node_modules/@playwright/test/cli.js test` from `ui`.

Failed experiments: the default Edge channel lookup searched a nonexistent
per-user installation; using the actual Program Files executable resolved it.
An initial new regression assumed a prepared fixture would fail on restart;
that assertion was corrected and a separate fresh-fixture failure check added.
The initial component expectation included the former text-dot character;
it was updated for the new decorative dot. Final relevant checks pass.

## Limitations, rollback and handoff

Browser fixtures simulate lifecycle and never open audio devices. The release shell and companions were subsequently rebuilt at the owner's
request (below), without launching them; actual rebuilt tray-menu observation
and packaged-app qualification remain open. No full native audio or hardware
gate is claimed. Rollback reverts only this slice's UI/shell/docs/tests; no
session migration or persisted-state reversal is needed.

Next M05 task: owner launches the rebuilt executable and verifies the tray
menu omits the toggle and the embedded UI matches this change. Check for an
existing shell before launching. The independent stability plan's next action remains
the owner's overnight app soak.


## Owner-requested desktop rebuild

The owner asked whether the changes could be tested and whether the app had
been rebuilt. No audiorouter-shell process was running at inventory. Built
without launching, changing the installer or publishing a release:

- `cargo build --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target --release --features custom-protocol --locked`: pass.
- `cargo build --manifest-path Cargo.toml -p audiorouter-cli --release --locked`: pass.
- `cargo build --manifest-path Cargo.toml -p audiorouter-plugin-host --bin audiorouter-plugin-worker --target-dir src-tauri/target --release --locked`: pass.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tools/stage-mcp-cli.ps1 -Profile release`: pass; staged/source CLI SHA-256 values match.

Test executable: `src-tauri/target/release/audiorouter-shell.exe`; companions
are alongside it. Shell modified 2026-10-14 23:52:01 UTC, after
`ui/dist/index.html` (23:41:23 UTC) and all edited UI source files. The built
JavaScript includes the new header-session-name control. SHA-256:

- `audiorouter-shell.exe`: `e9dde1de700566512939c62984d31438223e13a9a009ce4e35708e3d6588c58c`.
- `audiorouter-cli.exe`: `570533fbf1d17fc718fda65a8230de60878c01d2d924671b8ccefa9f7d4f06db`.
- `audiorouter-plugin-worker.exe`: `a3b2cf2f55475bccc06e9785c2116d151fa39a21cbe141744b422b0902c44d60`.

Rollback shell: `src-tauri/target/release/audiorouter-shell.exe.before-ui-20261014`;
the staging script also preserved its previous CLI. Rebuild logs and exit
records are in `%TEMP%/audiorouter-ui-fixes/desktop-*-rebuild.log/json`.
Human testing of the rebuilt app is next; the build alone is not attended
tray, playback or fresh-install evidence.
