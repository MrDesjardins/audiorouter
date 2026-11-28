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

- `origin/feat/multichannel-output` (`53afc3ce1`, then `e7783dfae`), which
  already contains `origin/docs/site-device-wording`,
  `origin/feat/mcp-rename-and-recording` and
  `origin/improve/ui-accessibility-app-qualification`. The second merge adds
  A4 for the single-endpoint and application workers (`205892499`), A3 default
  bus-list tools (`62551f1b2`), the A4 live-continuity plan (`0145e83f2`) and
  the A3 release note (`6b8728299`).
- `origin/main` (`d912424c2`, #199, then `fe6a08b40`, #200). The candidate
  already contained an equivalent recovery tree and the frontend port; the
  later branch adds the harness fixes, the A4 plan text and 0.0.33 notes.

The candidate's earlier branch is left untouched for byte comparison. No
unrelated untracked files or owner profiles are used.

## Decisions and ordered tasks

1. Merge `origin/feat/multichannel-output` into `t3/integrate-next-app-candidate`
   as it advanced, keeping the approved A4/A3 decisions, #199 wording and both
   plan additions.
2. Correct the A4 implementation notes to the code actually merged (2–32 float32
   channels in every worker; channel 0/1 widening, not channel-mask positions).
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
7. Commit the audited source, plan and evidence changes and open one PR for
   review. PR #207 now targets `main` and contains the merged #201/#203/#205 work.
   Do not tag, publish or merge it.

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
- Not claimed: attended audio/device, CABLE-A continuity, attended 7.1 Play, the
  application worker live, a hardware check of channels 3–8, Windows scaling,
  Narrator, tray/dialog language, Defender/MOTW, clean machine, second PC or
  signing. The 7.1 continuity gate passed in #206; the remaining items stay
  human/other-thread gates.

## Results

Integration commit `2323072e3` (current `main`, including #201, #203, #205,
aa95a6394 and the #206 docs/evidence merge) is clean and locally qualified. #206
changed documentation/specification text only, so the qualified candidate bytes
are unchanged. Both formatting and Clippy runs are
clean; 64 workspace test targets (1,683 passed, 0 failed), 186 `src-tauri` tests,
1,210 UI tests and 35 Stream Deck tests passed. Contract drift (157 methods),
documentation validation (333 files), site/CSP tests, UI hooks, quality policy
tests and `release-tests` passed. One full run hit a transport timing flake
(`busy_pipe_wait`); it passed 3/3 in isolation and in the complete rerun, and is
reported without a product change. The aa95 spacing change was checked separately
through UI lint/format/typecheck, 1,210 UI tests and its 3/3 Edge recording-opt-in
case with the new minimum-gap assertion.

The app-only release preparation produced a complete unsigned 0.0.33 package
(`AudioRouter_0.0.33_x64-setup.exe`, SHA-256
`405ae6bf6f941d5e1c92318b1afd44d4e1cc427e12b863976dab3ec2141a7583`) with the CLI,
four workers, tunnel runtime, Stream Deck package, sound library, SBOMs and
checksums. The manifest verifier passed. The qualified shell is
`0f319c482028034d2a9c15c068deda8fcf7f1c251e0f766c8aced55b7dbcf43e`.

`fresh_install_shell` passed on that shell with a disposable database and
WebView profile, and all eleven packaged-shell qualification cases passed. The
final baseline used a clean tree, denied device access, produced no uncaught UI
errors and exited both owned shells with code 0. The installer was not installed;
its VM lifecycle, attended audio/device checks and every human gate above remain
open.

Evidence: [2026-11-27 app-only candidate integration](evidence/2026-11-27-app-candidate-integration.md).

## Risks and rollback

The merge touches plan text and the A4 output path. A bad widening would affect
only multichannel output devices; mono and non-float devices stay refused, and
the exact-width path is unchanged. Revert this branch's merge commit to return to
the `candidate/0.0.33` tree. Disposable profiles and databases keep qualification
away from the owner's route and consent. The previous candidate executables are
retained under `target/candidate-0.0.33/bytes/` with recorded SHA-256 values.

## Evidence and next action

Next: hand the candidate to the owner for attended testing. The 7.1 continuity
evidence is merged in #206; attended 7.1 Play and the application worker live
path remain open, the frontend thread owns attended accessibility, and
publication/signing/PR approval remain separate.
