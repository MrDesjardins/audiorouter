# Plan: app-only release qualification

> **Archived 2026-11-24 — agent qualification done, RQ-05 partial; attended, hardware and Defender gates.** Open gates, including failed
> checks, are in [human-gated tasks](../active/human-gated-tasks.md); archiving
> passes none of them.

Prepared 2026-11-19. Status: executing (updated 2026-11-21). The candidate is
frozen and RQ-01 to RQ-04 have passing evidence on a fresh disposable guest with
a standard user, with one recorded deviation: a Defender exclusion for the
staging folder, because Defender quarantines the unsigned installer. RQ-05 is
partial.

**Execution state (2026-11-21):** see
[the v0.0.31 record](../active/evidence/2026-11-19-release-qualification-v0.0.31.md).
v0.0.31 is already published (source `a4e92bb05`, installer `b94e5276…3a92`),
supersedes every 0.0.30 candidate, and is the frozen candidate for RQ-01–06.
Open: the downloaded-file unsigned-publisher prompt, offline installed operation,
the v0.0.30 rollback fixture (host Defender detection), a second Windows build,
and the Defender verdict (Microsoft submission pending; signing decision in
[release signing](2026-11-24-release-signing-and-publication.md)). **Superseded in part on
2026-11-24:** see [the 0.0.32 run and the requirement map](#2026-11-24---0032-bytes-remaining-gates-and-requirement-map).
RQ-03's guest refusals passed on 2026-11-21; the older "synthetic only" sentence
below is historical. **Superseded in part on
2026-11-24:** see [the 0.0.32 run and the requirement map](#2026-11-24---0032-bytes-remaining-gates-and-requirement-map).
RQ-03's guest refusals passed on 2026-11-21; the older "synthetic only" sentence
below is historical.

## Objective and boundary

Qualify one exact Windows 11 x64 app-only installer for standard-user setup,
repair, upgrade, rollback, uninstall, state retention and first run. Produce a
reviewable requirement-to-evidence report with explicit pass/fail/blocked/not-run
results. This closes the installer workstream, not the whole M08 release gate.

Requirements: DIST-01–08, NFR-16; related offline and provenance checks NFR-15
and ENG-05. DIST-05 is a tooling/documentation review here, not a publication
exercise. DEC-17 permits an unsigned first app release; app signing is not a
prerequisite. DEC-18 driver qualification remains separate.

The requested action is preparation of this plan. Subsequent execution follows
the existing authorized priority and repository rules. No PR, publication,
purchase, driver installation or change to the owner's live audio/data is
authorized by this document. No delegation is needed to execute it.

## Read first and evidence precedence

- [Current priorities](../active/current.md), [M08](../../milestones/M08-release.md),
  [DIST requirements and decisions](../../spec/15-delivery.md), and
  [quality requirements](../../spec/14-quality.md).
- [Qualification runbook](../../operations/release-qualification.md),
  [verification profiles](../../operations/verification.md), and
  [release signing boundary](2026-11-24-release-signing-and-publication.md).
- [Existing brief and final handoff](2026-11-24-delegate-release-qualification.md) and
  [2026-11-18/19 attempt evidence](../active/evidence/2026-11-18-installer-process.md).
- `tools/vm/installer-lifecycle*.ps1`, the lifecycle acceptance tests and
  `tools/release/prepare-artifacts.ps1` / `verify-artifacts.ps1` before execution.

The final handoff supersedes earlier open-work summaries in the brief. Preserve
the historical results; do not reinterpret them as tests of a newer candidate.
Some runbook/M08 status prose predates the November runs. Use exact attempt and
artifact identities when reconciling it, without weakening milestone criteria.

## Established evidence and remaining gaps

Recorded results below were read from repository evidence, not rerun today.

- RQ-02: a reached-marker guest timeout failed honestly at 12 seconds, shut down
  gracefully, and was followed by three consecutive production-recording passes.
  Host process regressions cover owned descendants and unrelated-process survival.
- RQ-03: wrong hash, excluded driver VM and wrong snapshot refusal have real
  evidence. Administrator, unloaded hive and existing staging/app refusals have
  synthetic evidence only; actual disposable-guest cases remain open.
- RQ-04: install, companion repair, rollback to 0.0.29, upgrade, uninstall and
  reinstall retained a recording produced by the installed AudioRouter engine
  from generated 440 Hz audio. Combined candidate `release-candidate-2026-11-18b`,
  source `d6c400c7b`, installer SHA-256
  `d76278cb838f824a53d899c1091ea79571fef9748224dd3d218169a7ef68814f`,
  passed artifact verification and two lifecycle runs. These results apply only
  to those bytes and recorded environments.
- RQ-05: the absent-runtime run failed offline with setup exit 2 and no observed
  leftovers; reconnecting NAT allowed WebView2 bootstrap and the full lifecycle.
  The aggregate correctly remains **failed**: no bootstrap-specific diagnostic
  establishes the cause of the offline failure. Manual removal of WebView2 and
  disabled updater services/tasks make this a constructed failure baseline,
  not proof of an untouched clean Windows installation.
- Still open: interactive trust/setup, shortcuts and first-run usability,
  endpoint states, unchanged defaults/volumes, fully offline installed operation,
  complete uninstall residue checks, current-candidate identity and supported-OS
  coverage. Do not infer any of these from a silent setup exit.

## Prerequisites and inventory

Before mutation, record branch/HEAD and dirty paths; preserve unrelated work.
Planning checkout: `release/0.0.31` at
`73b4cfefff272a42d16dd8337afa85900f9c0196`; untracked `target-preview/`,
`target-preview2/` and `ui/tsconfig.tsbuildinfo` are unrelated and must remain.
These values are context, not a candidate selection. Reinventory at execution.

1. Select the intended source commit, version/tag and previous compatible
   package. Reconcile VST worker handoff before packaging. Use a clean committed
   source without deleting unrelated files to satisfy the clean-tree guard.
2. Locate retained installer/manifests and raw November reports; verify hashes
   and availability. If reports have expired, retain the durable summary but
   mark unavailable raw evidence and rerun the affected acceptance cases.
3. Inventory Windows builds, VirtualBox/Guest Additions versions, VM UUID,
   snapshot UUIDs, power state, logged-in account and host-audio configuration.
   Historical disposable VM UUID: `87c31665-2022-49a2-a182-1a1ebb5e631c`.
   Runtime-present offline snapshot: `lifecycle-offline-20261118`, UUID
   `45d20e3c-8066-4896-9e28-a206f9d88723`. Resolve and verify ownership of
   `lifecycle-webview-absent-v10-20261119`; its UUID must be recorded before use.
4. Provide guest credentials through the supported secret mechanism, outside
   source/logs. Prior temporary credential files were deleted. Confirm a GUI
   console and an observer are available for interactive cases. A standard user
   performs app tests; any administrator baseline preparation is recorded
   separately and is not standard-user install evidence.
5. Reserve a quiet host window and a new evidence directory for every attempt.
   Use only verified disposable guests, with host audio disabled. Never use
   `AR-DriverTest`, the owner's workstation state, or a saved-memory baseline.
6. Inventory available supported Windows 11 environments. M08 requires at least
   two supported builds plus reference PC/laptop coverage; record what the VM
   matrix covers and what remains with the broader hardware workstream. Reverify
   OS support at execution; do not claim broad coverage from build 26300 alone.

Missing credentials, a GUI observer, artifacts or an OS baseline block their
dependent cases only. Harness review and documentation can proceed independently.

## Ordered execution and acceptance cases

### RQ-01 — Freeze the candidate

Verify or prepare the app-only manifest (`includesDriver: false`) with NSIS
installer, shell, CLI, x64/x86 VST2 and VST3 workers, notices, Cargo/npm SBOMs,
release notes and checksums. Run `verify-artifacts.ps1`; require agreement of
version, architecture, source commit and intended tag. Record unsigned status.
No driver/helper payload may slip into the app-only profile. Review the manual
draft trigger and human publication boundary without uploading anything.

Exit: a frozen manifest plus verified package paths/hashes and a previous-version
pair. Any rebuilt installer invalidates package-specific lifecycle evidence and
must re-enter RQ-04/05. Do not silently select 0.0.30 or 0.0.31 from a plan/branch
name. Requirements: DIST-05/06/07, ENG-05.

### RQ-02 — Confirm bounded orchestration

Run the host process, guard and recording regressions. Confirm the current
harness still matches the revision with reached-marker timeout and three clean
runs. If it changed, repeat the injected full-workflow timeout and three
consecutive restore/run/graceful-shutdown passes. If unchanged and raw evidence
is retained, cite those passes rather than restarting completed investigations.
Keep package-specific qualification separate from harness repeatability.

Every exit must retain step timing, exit/timeout classification and cleanup
results. Kill only owned runner descendants identified by PID and creation time.
A guest that cannot shut down stays preserved for diagnosis; no force-reset or
retry-until-green loop. Exit: a bounded failure, a reached marker, graceful
shutdown and successful subsequent clean runs. Requirement: DIST-08.

### RQ-03 — Exercise refusal guards on the real disposable guest

In independent recoverable test states, exercise administrator account,
unloaded profile hive, existing staging directory and existing app directory.
Record before/after hashes or inventories proving refusal left protected
contents untouched. Retain the existing wrong-hash/VM/snapshot evidence, rerunning
if the guard changed. Do not weaken a guard merely to reach the lifecycle.
Exit: each refusal has actual target-state preservation evidence, distinguished
from synthetic tests. Requirements: DIST-01/08, NFR-16.

### RQ-04 — Run the frozen installer lifecycle

On the verified runtime-present standard-user baseline, use the unattended
wrapper with `-RestoreDisposableSnapshot -WebViewBaseline Present
-ProductionRecording` and exact manifest hashes/versions. Follow the runbook
for the complete argument set. Stop sessions deliberately before transitions.

1. Fresh install: no elevation; exact app/companion versions and paths; current
   user registration and launch shortcuts work.
2. Create a real session using the installed CLI; record generated non-private
   audio through the installed desktop bridge and production Recorder. Decode
   the WAV and check sample rate, channels, frames, tone and nonzero level.
3. Same-version repair: remove one explicitly owned companion, repair, then
   prove it was restored and resolves from the installed package.
4. Roll back to the declared previous version, then upgrade to the candidate.
   After each step read the retained session through that installed CLI and
   verify recording bytes/decodability. Record configuration compatibility;
   preserve originals if downgrade is unsupported rather than forcing a pass.
5. Uninstall: app files, shortcuts and registration removed; no stale startup
   entry, helper/broker or exposed control pipe; database/recording retained;
   external drivers/endpoints unchanged. Reinstall and read/play the retained
   fixture deliberately, without automatic startup audio.

Exit: all transitions report versions, state readability, recording hash and
cleanup evidence for the frozen package. Requirements: DIST-01/02/08, NFR-16.

### RQ-05 — Observe setup, failure recovery and first run

Use GUI VirtualBox and retain redacted observations/screenshots. Capture before
and after endpoint IDs, Windows default devices for all roles, master/endpoint
volumes and mute states using an identified read-only probe. Plan or implement
that probe before the run if none exists; a screenshot alone is insufficient.

- **Present runtime:** interactive standard-user setup, real shell/shortcut
  launch, CLI and four-worker resolution. Record every trust/setup prompt,
  elevation behavior and the unsigned release notice. Do not disable host trust
  protections. A locally copied file may lack download provenance: record its
  Mark-of-the-Web state, and do not generalize its prompts to downloaded assets.
- **Absent runtime, network available:** independently prove absence, observe
  the same installer's bootstrap and actual shell launch. Record any elevation
  and whether a standard user can complete the path. Inspect the first-run
  chooser rather than inferring success from registration alone.
- **Absent runtime, network unavailable:** disconnect only the guest NAT link;
  capture the interactive error and any verified bootstrap diagnostic. Check
  runtime, files, registry, database and processes for leftovers; reconnect and
  retry the same installer. Keep the previous unattributed failure as failed.
  If attribution remains unavailable, record a concrete diagnostic defect and
  add a regression when fixed; do not relabel it as a passing bootstrap test.
- **Endpoint states:** no usable endpoint, physical endpoint only, and an
  existing supported virtual endpoint where a dedicated fixture is available.
  Confirm accurate discovery and actionable vendor guidance, stopped startup,
  deliberate audio consent and no silent substitute input. Do not install a
  driver as a shortcut; unavailable endpoint fixtures remain blocked.
- **Installed offline operation:** with runtime already available, disable
  guest networking and exercise editing, routing/effects, generated recording,
  CLI, local MCP stdio and local discovery. Keep optional remote integrations
  off. Verify unchanged default devices, volumes and drivers after all cases.

Exit: each scenario has observed behavior and before/after evidence. Execute
the applicable installer/first-run matrix on the declared supported Windows
builds; missing environments remain explicit blockers. Requirements:
DIST-03/04/07/08, NFR-15/16. This is not physical audio or full usability evidence.

### RQ-06 — Reconcile and hand off

Run focused regressions for any changes, the applicable verification profile,
docs acceptance and `git diff --check`; inspect the complete diff. Code changes
also require the repository's formatting, lint, compile, contract and real-entry
checks. Read and resolve/justify annotations if a PR is later authorized.

Write a durable redacted report under `evidence/` with one result per case above:
requirement, scenario, status, source/package hashes, harness revision, OS/VM/
snapshot, account role, commands, duration, observed result, raw evidence path,
cleanup and remaining action. Retain failures, not just the passing retry.
Refresh stale runbook/current-plan status from measured results. Report exact
branch/commit, files, local checks, limitations and next action before any
request for PR approval. Do not create a draft release as a validation shortcut.

## Completion and release decision

Installer workstream complete means RQ-01–06 acceptance cases pass for the frozen
candidate on the declared environments with retained evidence. An external
blocker can complete an independent task's handoff, but cannot pass its acceptance
case or make this workstream complete. No unreviewed deviation is a waiver.

The final report must separately list broader M08 gates: VST compatibility and
editors, real endpoint continuity/latency, endurance/load, security/privacy,
Narrator/scaling, first-time-user study and reference PC/laptop evidence.
Driver signing gates apply only to a driver-inclusive release. App signing is
optional under DEC-17; unsigned disclosure and trust observation are required.
Publication always remains a separate human action.

## Risks, recovery and exact next action

Risks: stale candidate bytes, deleted temporary evidence, VM boot/guest-control
stalls, constructed absence mistaken for a clean machine, and silent exits
mistaken for interactive success. Mitigate with frozen manifests, retained raw
reports, offline snapshots, bounded runners and explicit case statuses.

Rollback: revert scoped documentation/tooling changes; restore only the verified
disposable guest's own offline snapshot after preserving failure evidence.
Restore the guest NAT link on every exit and report cleanup failures separately.
Never overwrite retained user data to make downgrade work. Worktree cleanup uses
the repository-mandated cleanup script only.

Coordination carried from the delegation brief: the owner of this plan owns
`tools/vm/installer-lifecycle*.ps1`, the release evidence and the qualification
runbook. Coordinate shared release/build-script changes with the VST track,
qualify a rebuilt installer again because earlier artifact results do not
transfer, and serialize heavyweight builds and VM runs. Imported historical
results stay attached to the bytes and environments recorded with them.

**Next execution action:** inventory retained candidate/manifests and raw reports,
reconcile the intended release source with the VST handoff, and record the exact
candidate plus present/absent snapshot UUIDs. Then run the remaining interactive
RQ-05 cases on those bytes; rerun RQ-04 if the selected installer differs from
the qualified combined candidate. No VM is restored during plan preparation.

### Harness repair: startup-failure diagnostic (2026-11-24)

Branch `fix/packaged-shell-windows-snapshot`, from `origin/main` `53e1de80e`.
Committed on `main` as `d13fb56a2` (2026-11-24), including the test and the
`policy` wiring below; no push or PR was made by the agent.

`tools/quality/packaged-shell-smoke.mjs` called `windows("Snapshot", …)` in the
baseline startup-failure path but never defined or imported `windows`. A shell
that died before its DevTools endpoint existed therefore reported
`windows is not defined` and lost the owned-process snapshot that RQ-02/RQ-03
rely on; the sibling qualifier had its own private copy, which is how the drift
went unnoticed. The GHA packaged-smoke job only takes the passing path, so no
hosted run reported it.

Fix: `tools/quality/packaged-shell-windows.mjs` now holds the single
PowerShell-invocation wrapper (`windows`, `windowsHelperPath`). Both the smoke
runner and `qualify-packaged-shell-smoke.mjs` import it, so a call site cannot
reference a binding that one copy dropped. `tools/quality/packaged-shell-windows.test.mjs`
covers the helper and, as a direct regression for the defect, fails on the old
file because it calls `windows()` without binding it. The test is wired into
the `policy` check in `tools/quality/checks.mjs`, so it runs on every fast and
full profile, on Windows and elsewhere (the process-snapshot cases skip off
Windows).

Checks: `node --test` over the `policy` selection (policy, verify,
quality-history-chart, packaged-shell-windows, code-size) passes 91 tests; the
new file passes its 4. Not run: a real shell launch, because the owner's app
holds the default control pipe and the plan keeps the shell off limits, and
hosted CI. The end-to-end baseline startup-failure case remains to be exercised
on the frozen candidate in a disposable guest before RQ-02/RQ-03 are re-cited
against this harness revision.

## 2026-11-24 - 0.0.32 bytes, remaining gates and requirement map

The owner allowed VM use for this session (four guest CPUs, minimal Defender
exclusions, restore the baseline afterwards). Full record:
[the 0.0.32 run](../active/evidence/2026-11-24-release-qualification-0.0.32.md). No push,
PR, tag, draft or publication was performed.

### Measured results

- **RQ-04 on 0.0.32 (passed, these bytes only).** `installer-lifecycle.ps1
  -ProductionRecording -WebViewBaseline Present` on
  `AR-ReleaseQualification-20261116` (`lifecycle-offline-20261118`, `arqual4`,
  4 CPUs) with candidate `0.0.32` SHA-256 `50bcdfc1...ceed` (source
  `0af71ca71`) and previous `0.0.29` `f2c43d11...09d8`. Every step passed
  (fresh install 6.8 s, repair, rollback, upgrade, uninstall, reinstall); a
  440 Hz stereo 48 kHz production recording, RMS 0.0890, SHA-256
  `8592C312...E011`, was retained through every transition. Evidence
  `%TEMP%\ar-rq-20261124-lifecycle-ba393b5eed604d3d9521426a8ebd2de3`. This does
  not transfer to a tagged or rebuilt 0.0.32, and the published v0.0.31
  lifecycle keeps its 2026-11-21 result.
- **Deviation: previous is 0.0.29, not 0.0.31.** Host Defender blocks reads of
  the retained v0.0.31 installer (threat `2147927549`, 2026-11-24); its
  published digest still matches. No exclusion was added (elevation was not
  worked around), so rollback used the same 0.0.29 fixture as the v0.0.31 runs.
- **RQ-02 timeout logic (passed, manual method).** The wrapper's cold-boot
  sign-in fails on this baseline: Winlogon `DefaultUserName` is `artest` with an
  expired password, so the password goes to `artest`. A wrapper attempt is
  recorded as a console-sign-in failure, not as a passing case
  (`%TEMP%\ar-rq-20261124-timeout-f8616933...`, graceful shutdown passed). With
  `arqual4` signed in manually, the harness's own bounded runner cut the hung
  guest command at 12.0 s, reached the `AR_QUAL_TIMEOUT_REACHED` marker and left
  no `VBoxManage` process (`%TEMP%\ar-rq-20261124-timeout-manual-d55e9790...`).
- **RQ-03 guest refusals are already real-guest evidence (2026-11-21).** The
  "synthetic only" statement in *Established evidence* is historical: existing
  staging, existing app directory, administrator account and the host's
  signed-in check each refused with unchanged target state
  ([v0.0.31 record, RQ-03](../active/evidence/2026-11-19-release-qualification-v0.0.31.md#rq-03-real-guest-refusal-cases-passed-2026-11-21)).
  The guest-side registry-hive assertion stays synthetic.
- **RQ-05 offline operation (passed on 0.0.32, with the harness fix).** With the
  guest NAT link off: no DNS or TCP, all six installed executables at 0.0.32,
  CLI `status`/`schema`/`session list`/`session get`, `system.describe`, MCP
  stdio `server/discover` (protocol `2026-07-28`) and a production recording
  through the installed bridge (440 Hz stereo 48 kHz, 97,152 frames, RMS
  0.0890, WAV SHA-256 `6289270C...`) all passed; no shell auto-started; the guest
  has no audio endpoints, so default-device/volume checks are not claimed.
  Evidence `%TEMP%\ar-rq-20261124-offline-fixed-be29bbd7....json`. The earlier
  failed attempt was a **stale idempotency replay**, fixed in the product and
  the harness (below), not an offline defect.
- **Absent-WebView2 on 0.0.32 (failed by design, attribution unchanged).** From
  `lifecycle-webview-absent-v10-20261119` with `arqual4`:
  `installer-lifecycle.ps1 -ProductionRecording -WebViewBaseline Absent
  -OfflineBootstrapRetry` recorded offline setup exit 2 with nothing left behind,
  then an online retry that bootstrapped WebView2 `158.0.4370.33` and passed the
  full lifecycle with the recording. The overall run stays `failed` because the
  silent offline exit has no verified bootstrap diagnostic, the same gap as
  v0.0.31. Evidence `%TEMP%\ar-rq-20261124-absent-inner-34a6ffd0...`.
- **Root cause and fix for the post-reinstall recorder failure.** A replayed
  `sessions.play` result outlived its runtime: the idempotency journal survives a
  control restart, the runtimes do not, so a cached "running" was returned while
  the summary said stopped. Reproduced on the host through
  `ControlPlane::dispatch` with a file database (failing-first
  `a_replayed_play_after_a_restart_does_not_report_a_stopped_route_as_running`).
  Fixed on both sides: the recorder harness uses a per-attempt GUID key, and
  the control plane now replays a recorded outcome verbatim for the whole
  backend lifetime and **refuses** a play/start/toggle (and record/mute)
  outcome recorded by an earlier lifetime with `idempotencyConflict` and
  "AudioRouter restarted since this request" instead of re-executing it. The
  shipped 0.0.32 bytes do not contain the product fix; the next build carries
  it. Tests cover Stop-then-retry, both toggle directions, generation, restart
  refusal, recorder start and stop replays through the dispatcher.
- **Wrapper cold-boot sign-in (fixed, proved).** The unattended wrapper now, on
  retry only, dismisses a blocking expired-password form and clears the password
  field before typing, without changing any account or security setting. A run
  on `lifecycle-offline-20261118` recorded `sign-in-reset`, sign-in,
  `guest-ready`, a 12 s injected timeout, `timeoutMarkerReached: true` and
  graceful shutdown. Evidence `%TEMP%\ar-rq-20261124-wrapperretry-702646bc....`

### Harness changes in this session

- `tests/acceptance/m08-lifecycle-unattended-guards.ps1` (new) covers the
  wrapper's preflight refusals (UUIDs, driver VM, username, versions, offline-retry
  combination, existing/missing evidence directory) and asserts the result
  records `timeoutMarkerReached`. Before it, no host test touched the wrapper.
- `installer-lifecycle-unattended.ps1` now records `timeoutMarkerReached` in
  `result.json` and names a missing marker in the failure, so RQ-02 cannot read
  as a clean timeout without a reached marker.
- `installer-lifecycle-recording.ps1` accepts `-ExpectDeviceAccess` so an
  already-consented installed shell can be recorded; the fresh-install default
  is unchanged.
- All four lifecycle host scripts are wired into the `release-tests` check
  (`tools/quality/checks.mjs`), so they run on every release-area change and in
  `--release` full runs. `m08-lifecycle-recording.ps1` also guards the new
  consent switch.
- Recorder idempotency keys are unique per attempt. Journal rows persist across
  app restarts, so fixed keys replayed an earlier Play/Record result; the guard
  in `m08-lifecycle-recording.ps1` rejects a fixed key.
- The product side of that replay defect is fixed in `crates/control`: a
  same-lifetime retry replays the recorded result verbatim, and a storage-only
  result from an earlier lifetime that claims a live runtime state is refused
  with `idempotencyConflict` ("AudioRouter restarted since this request") rather
  than re-executing the old request. `lifetime_operations` supplies the explicit
  lifetime fact; the ten-case dispatcher suite is
  `recorder_play_tests.rs`. The 0.0.32 installer does not contain this fix; the
  next build does.
- `installer-lifecycle-unattended.ps1` retries now dismiss a blocking
  expired-password form and clear the password field before typing, which makes
  its cold-boot sign-in work on the older baseline. No account or security
  setting is changed; the guard test requires both steps to be retry-only.
- Checks: `cargo test -p audiorouter-control` (519 passed, 29 ignored, including
  10 idempotency replay tests),
  `cargo test -p audiorouter-storage`, workspace and shell strict Clippy, both
  format checks, `node tools/quality/verify.mjs --checks release-tests` (8
  scripts, 27.9 s), `node tools/docs/validate.mjs` (304 files, 1,705 links),
  `git diff --check` clean. Not run: hosted CI.

### Requirement-to-evidence map

Statuses are for the named bytes; "partial" names the missing scenario.

| Requirement | Status | Evidence and exact gap |
| --- | --- | --- |
| DIST-01 per-user install | **Pass** | 2026-11-24 0.0.32 fresh install as `arqual4` (standard user, no elevation) and 2026-11-21 v0.0.31 runs; registration and six installed executables checked. |
| DIST-02 state preservation | **Pass** | Session and recording hash retained through repair, rollback, upgrade, uninstall and reinstall on 0.0.32; a production recording was produced offline after the reinstall. The post-reinstall stale-replay finding is fixed in source: a same-lifetime retry replays the recorded outcome, and an earlier-lifetime runtime result is refused with `idempotencyConflict`, with dispatcher regressions. 0.0.32 bytes need the next build for non-UI clients; the UI Play path prepares first and was unaffected. |
| DIST-03 first-run guidance | **Partial** | v0.0.31 chooser, `NO DEVICE` cards, no substitution and consent prompt observed 2026-11-21 (guest + isolated host run). Missing: endpoint vendor guidance with a real endpoint on these bytes, and a 0.0.32 interactive pass. |
| DIST-04 offline operation | **Pass for the tested surface** | 2026-11-24 offline on 0.0.32: CLI, schema, retained session read, discovery, MCP stdio and a production recording through the installed bridge. Missing: offline editing through the window itself (not exercised) and the absent-runtime diagnostic attribution. |
| DIST-05 manual publication | **Pass as tooling review** | `tools/release/test-runbook-command.ps1` and the release scripts pass; no draft/tag/release was created. Publication remains a human action. |
| DIST-06 provenance/assets | **Pass for v0.0.31; open for 0.0.32** | v0.0.31 assets, digest `b94e5276...3a92`, manifest, checksums, SBOMs and notices verified. The 0.0.32 consolidated manifest exists but there is no tag and no rebuild from a tag. |
| DIST-07 unsigned disclosure | **Partial** | Manifest and 0.0.31 notes state unsigned. The downloaded-file unsigned-publisher prompt has not been observed; v0.0.31 is quarantined by Defender. |
| DIST-08 install qualification | **Partial** | Lifecycle on v0.0.31 (four passes, exclusion deviation) and 0.0.32 (one pass); absent-runtime baseline on 0.0.32 bootstrapped and passed after the retry. Missing: interactive first run on 0.0.32, a second supported Windows build, clean Defender for v0.0.31, and the offline bootstrap attribution. |
| NFR-16 standard-user operations | **Pass** | Install, repair, rollback, upgrade, uninstall and reinstall as a standard user; administrator and unloaded-hive states refused. |
| NFR-15 offline core | **Pass for the tested surface** | CLI, schema, retained session read, local discovery, MCP stdio and a production recording all work offline on 0.0.32. |
| ENG-05 release engineering | **Pass for v0.0.31** | Verified manifest, version/architecture/source agreement and artifact checks; 0.0.32 awaits a tag rebuild. |
| M08 | **Open** | Plugins/editors, real endpoint continuity and latency, endurance, security evidence, accessibility/Narrator, participant study, reference hardware, a second Windows build and signing stay separate. DEC-17 permits unsigned app bytes. |

### Remaining open checks (moved to the owner list)

These need a person, hardware, a purchase or an owner decision; the manager
collects them in `docs/plans/active/human-gated-tasks.md`. Agent-doable work in
this plan is finished.

1. **Downloaded-file unsigned-publisher prompt and first-run trust on 0.0.32**
   (DIST-07, RQ-05): requires a person at a GUI console and a file downloaded
   with Mark-of-the-Web; the copied-file run says nothing about it.
2. **Endpoint states with real endpoints** (DIST-03): physical-only and existing
   virtual-endpoint guidance, default devices and volumes unchanged.
3. **A second supported Windows build and the reference PC/laptop pass** (M08,
   DIST-08).
4. **Defender verdict for v0.0.31 and the signing decision** (DIST-07, M08):
   Microsoft submission `eb70e424-...` pending; see
   [release signing](2026-11-24-release-signing-and-publication.md).

Optional environments, not gates now that the older guest covers the same cases:
the fresh guest's standard-user credential (its console cannot be signed in
without it), and the fresh guest's administrator password if a runtime-absent
baseline is wanted there. The absent-runtime attribution gap (silent offline
setup exit 2 with no bootstrap-specific diagnostic) stays a documented failure;
a future build could add a diagnostic, which is a product change, not a
qualification gate.

**Open product follow-up (next build, not a gate):** the idempotency-replay fix
in `crates/control` and `crates/storage` must ship in the next installer. It is
proven by host dispatcher regressions; the 0.0.32 guest bytes predate it and were
qualified with the fixed harness and the UI's prepare-then-start path.

**Rollback for this session:** the guests are powered off and restored; the new
snapshots `pre-agent-rq-20261124` were kept, and no existing snapshot, VM,
exclusion or owner data was changed. Code rollback is reverting the harness and
dispatcher commit.

**Next action:** the manager archives this plan once the human-gated list exists.
The agent-side lifecycle and harness work is complete for the 0.0.32 bytes.
