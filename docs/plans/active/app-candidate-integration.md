# Next app-only candidate integration (2026-11-27)

## Objective and authority

Owner asked for the next non-driver app-only candidate: audit and integrate the
approved API/MCP work, frontend port, recovery repairs, website wording and the
A4 multichannel-output change, then build and qualify complete fresh unsigned
app-only bytes with isolated data and WebView profiles. No driver change, PR,
tag, publication, endurance soak or change to the owner's running app is
authorized. Preserve the previous candidate bytes.

## Requirements and prerequisites

M05/M08 and AUTO-06/10/15/18, API-07, A4/ARCH-08, L10N-01, UI-02/04/05/08/10/11/12/17,
DIST-02/03/06/07/08, NFR-04/05/06/14/15 and Q6.4. Read the active plan, A1/A2/A4
and A5–A9 decisions, recovery repairs, the UI qualification record and its
runbooks.

Inputs audited against `candidate/0.0.33` (`29bbb6b74`) before merging:

- `origin/feat/multichannel-output` (`53afc3ce1`), which already contains
  `origin/docs/site-device-wording` (`4f8f20d8f`),
  `origin/feat/mcp-rename-and-recording` (`ba2350941`) and
  `origin/improve/ui-accessibility-app-qualification` (`c27f6cfd1`).
- `origin/main` (`d912424c2`, #199). The candidate already contained an
  equivalent recovery tree; the later branches add two harness fixes, the A4
  plan text and the 0.0.33 release notes.

The candidate's earlier branch is left untouched for byte comparison. No
unrelated untracked files or owner profiles are used.

## Decisions and ordered tasks

1. Merge `origin/feat/multichannel-output` into `t3/integrate-next-app-candidate`,
   keeping the approved A4 decision, #199 wording and both plan additions.
2. Correct the A4 implementation notes to the code actually merged (2–32 float32
   channels; channel 0/1 widening, not channel-mask positions).
3. Run the full portable checks and the complete desktop release build; verify
   the embedded UI is newer than its source and the executable reports 0.0.33.
4. Build the app-only unsigned NSIS package through
   `tools/release/prepare-artifacts.ps1 -AppOnly`, which stages the CLI, both
   x64 workers, both x86 workers, the tunnel runtime and the Stream Deck package.
5. Qualify the complete bytes with isolated data and WebView profiles:
   `fresh_install_shell` and the packaged-shell qualifier's eleven cases. Keep
   the owner's app and profiles out of every run.
6. Record results in a unique evidence file, update this plan and add the
   verified status to `current.md` only after the checks and build finish.
7. Commit only the audited source, plan and evidence changes. Do not tag, publish
   or open a PR.

## Validation matrix and evidence

- Merge audit: ancestry/diff inspection; prior candidate bytes copied aside.
- Formatting: both `cargo fmt -- --check` runs.
- Rust: workspace Clippy `-D warnings`, `src-tauri` Clippy, all workspace tests and
  `src-tauri` tests with the pinned 1.96.0 toolchain.
- Contracts/API: `contracts check:drift` (157 methods) and the existing dispatcher,
  MCP and recovery regressions.
- UI: lint, Prettier check, i18n check, typecheck and unit tests.
- Site/docs: documentation validation, CSP check and site tests.
- Automation: UI-hook regressions, quality policy tests and the `release-tests`
  profile (including the lifecycle harness).
- Build: `tools/build-desktop.ps1` and the app-only release preparation; hashes,
  timestamps and unsigned status.
- App qualification: fresh database/profile launch and the packaged-shell
  qualifier baseline, restart, bridge, readiness, error, cleanup and cancellation
  cases.
- Not claimed: attended audio/device, CABLE-A continuity, 7.1 output continuity,
  Windows scaling, Narrator, tray/dialog language, Defender/MOTW, clean machine,
  second PC or signing. Those remain human/other-thread gates.

## Risks and rollback

The merge touches plan text and the A4 output path. A bad widening would affect
only multichannel output devices; mono and non-float devices stay refused, and
the exact-width path is unchanged. Revert this branch's merge commit to return to
the `candidate/0.0.33` tree. Disposable profiles and databases keep qualification
away from the owner's route and consent. The previous candidate executables are
retained under `target/candidate-0.0.33/bytes/` with recorded SHA-256 values.

## Evidence and next action

Evidence: [2026-11-27 app-only candidate integration](evidence/2026-11-27-app-candidate-integration.md).

Next: hand the candidate to the owner for attended testing; the multichannel
thread supplies the quiet-window continuity evidence, the frontend thread owns
attended accessibility, and publication/signing/PR approval remain separate.
