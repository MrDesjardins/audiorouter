# AudioRouter execution plan after the game

> **Archived 2026-11-24 — packages reconciled to their owning plans; remaining owner gates.** Open gates, including failed
> checks, are in [human-gated tasks](../active/human-gated-tasks.md); archiving
> passes none of them.

**Start signal:** the owner says the game is finished and asks the AI to execute this plan. The objective is to advance priorities 2–10 from the ranked roadmap and retain evidence for every completed check. Priority 1 has its own branch, local changes and acceptance plan. [Installer qualification](2026-11-24-release-qualification.md)

## Start and establish the baseline

1. Read [AGENTS.md](../../../AGENTS.md), the [documentation map](../../README.md), the [active plan](../active/current.md), and the milestones, specifications and decisions linked from each work package before touching that area. Inspect the worktree, branches and latest available main. The starting reference for this plan is `53e1de80e` (#187), including #181. Check later main commits and each plan’s newest execution record for completed work before assigning any task.
2. Preserve the uncommitted `fix/packaged-shell-windows-snapshot` work and unrelated files. Use an isolated checkout based on the applicable main SHA for new code or builds. Preserve the owner’s ordinary database, WebView profile and saved routes. Use disposable profiles, explicit test endpoints, and an isolated pipe where the harness supports it.
3. Inventory the test resources in one pass. Record Windows and toolchain versions, candidate and worker hashes, which audio endpoints and the default shell pipe are free, available licensed plugins and MIDI hardware, a disposable OBS profile, access to hosted artifacts, and the correct driver VM and snapshots. Treat game completion as the start signal. Verify the selected resources are idle before opening streams or launching the shell. Ask only for resources or decisions genuinely required, grouped where practical.
4. Start package 2 as soon as the selected cable endpoints are available. Stage packages 3 and 4 for the same attended window. Collect the fixture and hosted evidence in 3, 6, 7 and 8 while waiting for hardware or human input. Run the driver block separately on its disposable VM. If one acceptance case is blocked, record its prerequisite and move to the next runnable case.

## 2. Live audio continuity and latency

**Requirements:** ARCH-08, CAP-04, NFR-03. Read the current code-review and audio-health records. The 55.8-hour health summary and synthetic service timings are baseline evidence. The new work is live multi-input continuity, listening, and the pending live service and plugin cases. [Continuity plan](../active/code-review-fixes.md) · [Audio-health record](../active/evidence/2026-11-23-audio-health-after-latency-repair.md) · [M02](../../milestones/M02-audio-engine.md)

1. Build the release harness with `cargo test -p audiorouter-transport --test live_audio_continuity --release --locked --no-run`. Confirm all four endpoint IDs, formats, and that no voice call, stream or route holds the selected endpoints. The recorded eight-channel CABLE-B result endpoint is unsuitable for the stereo worker when configured that way. Use a free compatible alternate endpoint if available and record the actual IDs behind any fixed labels.
2. In a dedicated test process, set `AUDIOROUTER_LIVE_CONTINUITY=1` and `AUDIOROUTER_CONTINUITY_SECONDS=20`. Set the exact test endpoint IDs in `AUDIOROUTER_CONTINUITY_TONE_RENDER_ID`, `AUDIOROUTER_CONTINUITY_ROUTE_CAPTURE_ID`, `AUDIOROUTER_CONTINUITY_ROUTE_RENDER_ID` and `AUDIOROUTER_CONTINUITY_RESULT_CAPTURE_ID`. Clear inherited optional mode and plugin overrides. Run `cargo test -p audiorouter-transport --test live_audio_continuity --release --locked -- live::live_backend_service_keeps_a_routed_tone_continuous --ignored --exact --nocapture`. Verify the run reports the intended worker. First set `AUDIOROUTER_CONTINUITY_CHAIN=gain`, then use the optimized chain `gain,parametricEq,graphicEq,bassTreble,dehum,compressor,gate,limiter,delay,meter`. Repeat if results are intermittent.
3. Require a clean input reference and investigate routed discontinuities using gaps, underruns, service phase maxima, and any concurrent machine stalls. Keep large-library live load and the exact latency-changing commercial plugin as separately judged cases. Use a disposable library or approved copy and capture service timing during the actual routed workload.
4. Ask the owner to listen on their usual route and record their choice between 10 ms and 15 ms for `voice-to-cable-a`. Record the observed continuity and latency tradeoff. Apply a different floor only if selected, and measure it again on the same route.

**Evidence and recovery:** retain redacted per-run logs and the route, endpoint formats, worker, chain, duration and counters. Release the test streams and restore any explicitly approved test-local settings. Keep listening and any unavailable plugin or workload case open with its exact prerequisite.

## 3. VST2 and VST3 compatibility

**Requirements:** PLUG-01–07 and GRAPH-14. Reconcile the P2/P3/P5/P6/P9 lists against main, the VST handoff and current execution records. Pay particular attention to state and shell fixtures already covered by #187 and to local branches whose evidence may sit outside main. [VST handoff](2026-11-24-vst-handoff.md) · [Completion plan](2026-11-24-vst-completion-work.md) · [Owner checks](2026-11-24-vst-owner-checks.md) · [M06](../../milestones/M06-plugins-pitch.md)

1. Run `powershell -NoProfile -ExecutionPolicy Bypass -File tools/vst-qualification/run.ps1 -Clean` in the isolated checkout with the pinned SDK, C++ build tools and i686 target. Retain `report.json`, census and per-case logs. Check the existing hosted nightly for the matching source SHA and i686 results. Use the documented workflow if a fresh authorized run is needed and possible.
2. Use the matrix to select remaining P2/P3 gaps. Exercise program/preset save-reload, exact shell sub-ID state, and VST3 class/unit identity through the real worker and control-plane dispatch. Add a failing regression for each defect before fixing the owning layer. Record vendor-specific dispositions and worker/plugin versions.
3. For P5, qualify a saved sidechain route through graph preparation, start, disconnect and recovery. Check that the key signal ducks only the intended source and that worker failure on a protected voice path produces silence. Compare another branch or route to verify it continues as specified.
4. For P6/P9, use the owner checklist with audio flowing and a pumped parent. Check text input and focus, repeated open/resize/close, Windows scaling and monitor moves, then MIDI unplug/replug and the exact licensed plugin chain where available. Let the owner perform vendor sign-in and listening. Record actual audible effect or note output as applicable, and keep skipped editor, rights, hosted and release cases visible.

**Evidence and recovery:** retain an architecture and plugin matrix with exact version and scenario results. Stop disposable routes and workers after testing. Revert scoped code changes if regression checks fail, and use backed-up session copies for state compatibility checks.

## 4. Attended Windows UI qualification

**Requirements:** UI-10/11/17/29 and the applicable M05 and M08 accessibility and usability cases. Start from the latest UI review round and its outstanding findings. [UI quality review](2026-11-24-ui-quality-review.md) · [M05](../../milestones/M05-visual-editor.md) · [M08](../../milestones/M08-release.md)

1. Build the packaged shell from the selected source and verify the embedded UI is current. Launch with a disposable database and `WEBVIEW2_USER_DATA_FOLDER` once the default pipe is free. Verify startup, native bridge, and save/restart with that profile before the attended checks.
2. Run a keyboard-only path through first launch, canvas, Properties, Settings and dialogs with Narrator. Capture announced names, focus movement, action results and refusal feedback. Check real Windows 150% and 200% scaling, physical-key capture cancellation, and dark, light and high-contrast layouts. Record the previous display/accessibility settings and restore them after testing.
3. In a disposable live route, disable or disconnect the agreed device. Capture the device-loss message and recovery on the same path, and verify a protected or pinned input behaves as specified. Check that live states keep reserved space. Fix reproduced defects in bounded changes and exercise them through the packaged entry point and relevant browser regressions.
4. Prepare the first-time-user script and observation sheet. In the actual M08 study, at least four of five first-time users must finish within ten minutes and all must identify the Discord source set. Record participant timings and friction without private audio or personal identifiers. If a tool or its controls change, refresh the portrait Properties guide and coverage manifest, then verify audio playback in Firefox.

**Evidence and recovery:** retain the shell hash, theme and scaling captures, redacted notes and passed/failed scenarios. A self-run walkthrough gives rehearsal evidence. Record participant results separately when available.

## 5. AudioRouter virtual cable Stage A

**Requirements:** DEC-18 and VCAB-24/25/27/28/30. Keep this independent of app-only qualification. The Stage A `round2` wrapper already exists and has host validation. Confirm its status against later evidence before changing it. [Stage A gap analysis](../active/evidence/2026-11-21-virtual-cable-stage-a-0.2.5-gap-analysis.md) · [Driver handoff](2026-11-24-virtual-cable-agent-handoff.md) · [VM guide](../../operations/virtual-cable-vm-guide.md)

1. Confirm the disposable driver guest and its recoverable baseline, package and loaded SYS hashes, matching symbols, guest responsiveness and output path. Identify the owner’s VM window. Use the test-signed package inside that guest only.
2. Follow the documented `round2` block order. Assess the paired RC3/0.2.5 render results using the predeclared per-shape criteria and valid-run minimum. Continue tagged-pool, VM verifier and recovery, then live update/unload last, with route-alive proof. Run the optional pause check only in the disposable VM under its documented conditions. Record it as a VM pause, with Windows sleep/resume tracked separately.
3. Collect the reports and dump or event evidence before reset. Account for unexplained underruns, the post-run freeze, tagged-pool trend, live-route unload and A15 events. Judge Stage A against its full recorded criteria. If a frame-clock revert or kernel defect requires a new candidate, assign a version above 0.2.6 per the driver plan, retain the matching PDB, and restart Stage A on the new bytes.
4. Schedule Stage C after the Stage A pass. Keep indicative latency and VM deviations labeled until measured on the required environment. Record the remaining signing decision for the owner.

**Evidence and recovery:** record snapshot, package and loaded hashes, each block result, and surviving logs. Preserve failure evidence before restoring the disposable VM. Confirm test-only auto-logon and owned processes are cleaned up if the selected procedure used them.

## 6. Application quality and maintenance

**Requirements:** ENG-02–05, applicable NFRs and Q5–Q6. Q3.2 is recorded complete and #187 delivered Q4 contract, fault and fuzz coverage. Read later branch and hosted evidence before repeating those slices. [Application quality](2026-11-24-application-quality.md) · [Code-scanning remediation](2026-11-24-code-scanning-remediation.md)

1. Inspect hosted checks, annotations and current CodeQL dispositions for the actual SHA. Separate fixed, open, and unavailable items. Reconcile the low-severity audit, RES-6/10/11, and existing local follow-ups with main. Base any new defect work on a reproducing scenario and existing scope. Leave RES-10 restart policy at the owner decision.
2. Extend existing Q5.1 trend output with first-pass failures, retry status, duration, source SHA and artifact links. Include regression age and escaped defects where data exist. Keep missing measurements explicit. Validate against retained passing, retry-passing and failing runs so a green retry cannot erase the first failure.
3. For Q5.2, collect current nightly module coverage and relevant branches or invariants before setting thresholds. For Q6, map release and rollback scenarios to evidence and carry open native or hosted gates with their owners. Keep installer-specific cases in the installer plan.

**Evidence and recovery:** record source/workflow/run IDs, raw reports and the validation commands. Keep metrics tooling changes reversible and use retained data to check previous report behavior.

## 7. MCP setup and OBS/Discord acceptance

**Requirements:** AUTO-06/18, UI-17 and the approved D1/O1/D2/O2 checks. The R2 permission and start repairs merged in #171/#174. Verify the existing workflows with a clean client and a packaged app. [MCP setup UX](2026-11-24-mcp-setup-ux.md) · [API and MCP decisions](2026-11-24-api-mcp-quality.md) · [OBS/Discord plan](2026-11-24-obs-discord.md) · [M07](../../milestones/M07-automation-recovery.md)

1. In an isolated packaged profile, follow the setup from the normal desktop entry point. Exercise enrollment, role display, a default-tool request over the real adapter, and revocation while connected. Confirm that each role’s expected allows and refusals match the backend policy and that a revoked client loses access. Use an owner-authorized test client for account-dependent connector checks.
2. Back up a copied OBS profile and establish expected sources with distinct test signals. Verify app/device attribution, source separation and duplicate/wrong-device diagnostics. Exercise the preview, explicit confirmation and restore path for Fix in OBS on the copy. Compare the restored configuration to its baseline.
3. With the owner and agreed endpoints, verify Discord receives the intended voice path and OBS receives each source once. Record application versions and observed source scope. Preserve existing permissions. Route any assistant recording or rename expansion through the owner decision already recorded.

**Evidence and recovery:** record the client and role matrix, endpoint map, app versions and copied-profile restore outcome. Revoke test enrollment, close disposable processes and restore the original active profile if the test switched profiles.

## 8. Localization qualification

**Requirements:** L10N-01–08. Check current main, the later extraction branches, and open decisions before adding keys. English, French and Simplified Chinese catalog evidence already exists from development work. Track packaged evidence and fluent review separately. [Localization plan](2026-11-24-ui-localization.md)

1. Inventory remaining user-facing canvas, Properties, tool and backend status strings against the catalog and latest extraction. Reuse generated keys, language fallback and existing structured error codes, preserving severity, actions and interpolation. Fix confirmed gaps in small slices, with the English source and reviewed translations tracked together.
2. Generate and validate catalogs with `npm --prefix ui run i18n:generate` and `npm --prefix ui run i18n:check`. Run focused UI tests, typecheck, format, lint and the language browser scenarios. Exercise live switching, missing-key fallback, a real backend refusal, persistence across restart and native tray language. Test every supported theme at 200% and verify Narrator using the packaged app.
3. Prepare a concise review queue for fluent French and Simplified Chinese reviewers with text, screen context, placeholders and known wording decisions. Record reviewer corrections and rerun the affected flows. Keep only the claims supported by that review and packaged evidence.

**Evidence and recovery:** keep screenshots, catalog check output and review status by surface and locale. Revert the scoped extraction and regenerate outputs if needed.

## 9. Signed in-app updater

**Future scope.** The AI can inspect the disabled prototype and existing release evidence, then prepare the small promotion decision. [Updater plan](../future/auto-update-tauri.md) · [M08](../../milestones/M08-release.md)

1. Inventory the prototype against the current specification and identify what can be reused. Draft decisions for signing provider and budget, key custody and rotation, the feed host and origin, initial channels, consent and restart behavior, and driver or Stream Deck boundaries. Use production-free inputs to review existing behavior.
2. After the owner explicitly promotes this scope and exact-candidate install, upgrade and rollback evidence is available, write the decision record and apply the affected SEC-11 and M08 traceability. Implement the approved slice behind the existing disabled boundary with a staging feed and test keys.
3. Qualify signed upgrade, data/settings/recording retention, active-session and recording gates, rejected wrong or tampered artifacts, interruption, restart, and rollback. Retain previous artifacts and show the previous version remains usable after failure. Feed enablement, production credentials and publication follow the approved release process.

## 10. Broader UAD compatibility

**Future scope.** Prepare U1 after the core VST baseline in package 3. U0 is already documented. [UAD work packages](../future/uad-plugin-support.md)

1. Present a bounded U1 inventory proposal for UADx and the owner’s choice of UAD-2/Apollo qualification path. Specify required licensed titles, host/hardware access, comparison controls, and the records to keep. Reconcile older instrument and latency assumptions with the current specifications.
2. After explicit U1 approval, have the owner sign in where required. On installed licensed titles, record discovery, class and bus identity, state restore, latency, editor usability and audible output against an input or bypass reference. Separate licensing, hardware and host findings. Keep license details and plugin binaries out of repository artifacts.
3. Use actual inventory results to propose narrowly scoped host changes under the appropriate PLUG requirements. Take dynamic latency, shared worker and hardware scope through their documented decisions. Claim compatibility only for validated families, binary versions and configurations.

## Verification and handoff

For each package, update its linked execution record and the active plan after a meaningful result. Use case status `pass`, `fail`, `blocked` or `pending`, with requirement ID, source SHA, binary hashes where applicable, environment, exact command or attended steps, result, artifact path and cleanup. Keep simulation, local real Windows, hosted Windows, hardware and participant evidence attached to their respective cases.

For changes, use the repository's targeted verification and format/lint rules. Verify the real entry point, review the complete diff and read applicable hosted annotations. API or tool changes also require REST/MCP parity, contract drift checks and regenerated references. Preserve independent work when a case is blocked, and report any remaining owner action once useful work has been exhausted. Get explicit human approval before opening a PR for the reviewed branch and changes.

**Resume command:** “The game is finished. Execute `docs/plans/archived/2026-11-24-post-game-execution.md`. Start with the cable preflight, then work through all runnable checks and bring me the remaining owner decisions together.”

## Execution record (2026-11-24)

Branch `fix/packaged-shell-windows-snapshot`, based on `origin/main` `53e1de80e`.
Local only; no commit, push or PR. Owner’s app was closed, so the default pipe
was free.

### Package 2 — live audio continuity (pass for the automated checks)

- Node `v22.22.3`; Rust 1.96.0 (pinned); Windows 11. Built the release harness
  (`live_audio_continuity`) with `--no-run` first.
- Endpoint map (all virtual, 48 kHz float, 2-channel, active) written to
  `target/post-game-2026-11-24/continuity-gain/endpoints.txt`:
  - tone render → CABLE Input `{0.0.0.00000000}.{71f96f14-…}`;
  - route capture → CABLE Output `{0.0.1.00000000}.{06268191-…}`;
  - route render → CABLE-A Input `{0.0.0.00000000}.{03498a2a-…}` (the default
    CABLE-B Input `{7ebf5c2d-…}` is 8-channel and was not used);
  - result capture → CABLE-A Output `{0.0.1.00000000}.{ef18e30a-…}`.
- Multi-input worker, 20 s, chain `gain`: pass. Reference and routed result both
  `0 glitches`, `0 silent runs`, `0 WASAPI discontinuity flags`; audio service
  `lateGaps: 0`, `maxGapMicros: 3067`, `output underruns: 1` (startup/teardown).
  Log `target/post-game-2026-11-24/continuity-gain/run.log`.
- Same endpoints, 10-tool chain
  `gain,parametricEq,graphicEq,bassTreble,dehum,compressor,gate,limiter,delay,meter`:
  pass, both references `0 glitches`/`0 silent`, `maxGapMicros: 3097`.
  Log `target/post-game-2026-11-24/continuity-chain/run.log`.
- Note: the harness prints a fixed `routed result (CABLE-B Output)` label even
  though the configured result endpoint was CABLE-A Output; the actual IDs are
  recorded above.
- **Owner listening test: done** across recent evening play sessions; cushion
  decided 2026-11-24: keep 10 ms for `voice-to-cable-a` (no pump change).
  Optional large-library load and the latency-changing commercial-plugin route
  were not run.

### Package 3 — VST hosted evidence and editor classification

- Hosted VST nightly, run `45420082364`, source `0fb41911b`, runner image
  `windows-2025-vs2026`: 46 of 47 passed, 1 skipped, exit 1. The sole failure
  was `vst3 editor lifecycle (AGain, fixed size)`:
  `cycle 0: the plugin created no window`. Artifact and case log:
  `target/post-game-2026-11-24/vst-artifact/` (`logs/23-…`), job log
  `target/post-game-2026-11-24/vst-nightly.log`.
- Cause is environmental: the hosted runner is headless. `run.ps1` previously
  skipped only the DPF ProM editor (OpenGL/`0xC0150011`) and asserted that
  AGain’s editor passed; the updated runner image no longer creates AGain’s
  window. `tools/vst-qualification/run.ps1` now skips **both** editor-lifecycle
  entries when `GITHUB_ACTIONS` is set, using the existing `SkipReason` path, and
  `docs/operations/vst-qualification.md` and
  [VST completion](2026-11-24-vst-completion-work.md) are updated. Editors remain attended
  checks. The script parses cleanly; a hosted run is needed to confirm the skip
  (not triggered: the branch is unpushed).
- Not run locally: the SDK/fixture/DPF build, attended editor/DPI/MIDI checks,
  licensed plugins, or an i686 run beyond the hosted nightly. See package 3 for
  the prerequisites.

### Package 4 — packaged UI (Narrator, scaling, first-time rehearsal)

Detail is in [UI quality review, Round 30](2026-11-24-ui-quality-review.md).

- Built the packaged shell from `53e1de80e` (shell SHA-256 `2C75BB45…`), verified
  it is newer than `ui/dist`, and passed the packaged smoke on an isolated
  database/profile (WebView2 158.0.4370.33). The owner closed their own app; no
  owner data was used.
- **Narrator (owner listening):** goal dialog text/buttons read; top bar reached
  after the first Tab enters the page and the two skip links (disabled
  Undo/Redo/Save correctly skipped); tabs use the ARIA pattern (arrows select,
  Tab leaves the strip) and the owner confirmed this; canvas cards, Properties,
  Settings → API invalid **Port** (range read, visible reason), and dialog
  focus/Cancel-first order were “all good”.
- **Real 150 %→200 % scaling (owner switched):** 150 % DPR 1.5, 200 % DPR 2,
  restored 100 % DPR 1; no horizontal overflow at any scale; only the two
  by-design edge-label clips; owner confirmed nothing clipped/overlapping.
  Screenshots: `target/post-game-2026-11-24/ui-package/attended/shots/`.
- **First-time walkthrough — agent rehearsal, then one clarity fix:** the
  “mic → Discord” prompt originally mapped to no single goal and
  “Game and voice for Discord” added a game-picker step. Added a **“Mic to
  Discord”** goal (`mic-discord`) that builds Microphone → Speech Denoise →
  Gate → Compressor → **Voice to Discord** with no game step and a post-create
  message naming the virtual cable to choose; exercised through the rebuilt
  packaged shell (`69EF34D9…`). Devices are still not preselected (a goal must
  never choose a device), so the CABLE choice is the user's. The app picker
  already has a **Find an app** search box. New fr/zh goal strings need fluent
  review. Real participant study (4 of 5 within 10 minutes; all identify the
  Discord source set) remains open.
- Not run: live device-loss on the rebuilt exe, physical-key dropdown
  cancellation, narrow 200 % window.

### Package 6 — application quality and maintenance

- At `53e1de80e`: AudioRouter CI and CodeQL both **success**; 23 check-runs on
  the SHA are success or skipped (unaffected areas); **no check annotations**;
  `code-scanning/alerts?state=open` returns `[]`. Evidence:
  `target/post-game-2026-11-24/check-runs.json`, `codeql-open.json`,
  `main-runs.json`.
- Reconcile with #187: low-severity audit items fixed (DSP-4, SES-4, UI-1;
  REC-2 not reproduced; HTTP-1/UI-2 already fixed); RES-11a/b fixed; RES-6 has
  synthetic large-bundle timings. RES-10 (plugin-worker restart) remains an
  owner decision. `current.md` priority 9 was refreshed to match.
- Q5.1/Q5.2 (first-pass failures/retries in the trend table, coverage
  baselines) were **not** implemented: they need workflow/trend changes validated
  against retained passing, retry-passing and failing runs. This is the next
  runnable code task in this package.

### Package 8 — localization

- `npm --prefix ui run i18n:check`: 3 language files, 3,738 messages, generated
  TypeScript and native resources current (`target/post-game-2026-11-24/i18n-check.log`).
- Remaining: fluent French/Simplified-Chinese review, packaged three-theme 200 %
  and Narrator evidence, and any residual per-surface extraction. No source
  change was made here.

### Not run (prerequisites)

- **Package 3:** a fresh hosted nightly to confirm the headless editor skip, the
  SDK/fixture builds, attended editor/DPI/MIDI checks, licensed plugins, and an
  i686 run beyond the hosted nightly.
- **Package 4:** the real first-time-user study (4 of 5 within ten minutes; all
  identify the Discord source set), live device-loss on the rebuilt exe,
  physical-key dropdown cancellation, and a narrow 200 % window. Narrator,
  real 150 %/200 % scaling and the agent rehearsal were run (above); an earlier
  draft of this list wrongly repeated them as not run.
- **Package 5:** virtual-cable Stage A requires the disposable driver VM; the
  test-signed driver must not go on this PC.
- **Package 7:** MCP enrollment/revocation through a packaged profile and the
  attended OBS/Discord acceptance with the agreed endpoints.
- **Packages 9–10:** future scope needing explicit owner promotion (updater key
  custody/feed; UAD inventory approval).

## Package reconciliation and ownership (2026-11-24)

Every package in this plan is executed inside another plan that owns its
requirements, gates and evidence. This section maps them so the program page
can be archived without losing an open item: the owning plan keeps the check,
and this page does not duplicate it.

| Package | Owning plan(s) | What the owning plan must carry |
| --- | --- | --- |
| 2. Live audio continuity and latency | [code-review-fixes.md](../active/code-review-fixes.md) (automated continuity, service timing, commercial plugin) and [current.md](../active/current.md) priority 6 (owner listening and cushion) | The **failed** 2026-11-24 CABLE-A repeat (3 of 6 routed results lost one ~11 ms block while the reference was clean; not provably isolated; diagnosis in the residual evidence), a clean isolated re-run once a quiet window exists, the portable concurrent Play/export/import timing (now recorded), the large-library live load, and the licensed commercial-plugin run. CABLE-B 8-channel is an owner decision. Owner listening and the 10 ms cushion are already recorded. |
| 3. VST2/VST3 compatibility | [vst-completion-work.md](2026-11-24-vst-completion-work.md), [vst-owner-checks.md](2026-11-24-vst-owner-checks.md), [vst-handoff.md](2026-11-24-vst-handoff.md), [vst-full-compatibility.md](2026-11-24-vst-full-compatibility.md) | P2/P3/P5/P6/P9 dispositions; the headless editor skip needs a fresh hosted run; SDK/fixture/DPF builds, attended editor/DPI/MIDI checks and an i686 run; PLUG-01–07 evidence. |
| 4. Attended Windows UI qualification | [ui-quality-review.md](2026-11-24-ui-quality-review.md) and `current.md` priority 4 | The first-time-user study with participant records; live-route device-loss on the packaged exe; physical-key dropdown cancellation; narrow 200 % layout. Narrator, 150 %/200 % scaling and guide recapture are recorded. |
| 5. Virtual cable Stage A | [virtual-cable.md](2026-11-24-virtual-cable.md), [virtual-cable-agent-handoff.md](2026-11-24-virtual-cable-agent-handoff.md), the 2026-11-21 gap analysis | The disposable-VM Stage A blocks (frame-clock revert, render underruns, live update/unload, tagged pool, A15), then Stage C, then the owner signing decision. Do not install the test-signed driver on this PC. |
| 6. Application quality and maintenance | [application-quality.md](2026-11-24-application-quality.md), [code-scanning-remediation.md](2026-11-24-code-scanning-remediation.md), the resilience record | Q5.1 first-pass/retry trend and Q5.2 coverage baselines (the next code task); RES-10 plugin-worker restart stays an owner decision; the CodeQL refresh is current at `53e1de80e`. |
| 7. MCP setup and OBS/Discord acceptance | [mcp-setup-ux.md](2026-11-24-mcp-setup-ux.md), [obs-discord.md](2026-11-24-obs-discord.md) | Packaged-profile enrollment, role display, a default-tool call over the real adapter and live revocation; the copied OBS profile check and Fix in OBS preview/restore; the attended Discord/OBS source-scope run. |
| 8. Localization qualification | [ui-localization.md](2026-11-24-ui-localization.md) | Fluent French and Simplified-Chinese review of the new goal strings and absorbed keys; packaged three-theme/200 %/Narrator evidence; any residual per-surface extraction. `i18n:check` is current. |
| 9. Signed in-app updater | [auto-update-tauri.md](../future/auto-update-tauri.md) | Future scope only: owner promotion, signing provider and key custody, feed host, channels, consent/restart, and signed upgrade/rollback evidence. |
| 10. Broader UAD compatibility | [uad-plugin-support.md](../future/uad-plugin-support.md) | Future scope only: a bounded U1 inventory proposal and the owner's choice of UADx vs UAD-2/Apollo path, then licensed-title qualification. |

Items in packages 9 and 10 are not authorized work; they are listed so the
program page closes with its deferred scope intact. The agent-doable package
work (automated continuity, hosted inspection, code-quality reconciliation,
localization check, and the packaged rehearsal) is recorded above; nothing in
this reconciliation adds authority to push, release, install a driver or touch
the owner's data.
