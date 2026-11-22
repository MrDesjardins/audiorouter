# Plan: app-only release qualification

Prepared 2026-11-19. Status: executing (updated 2026-11-21). The candidate is
frozen and RQ-01 to RQ-04 have passing evidence on a fresh disposable guest with
a standard user, with one recorded deviation: a Defender exclusion for the
staging folder, because Defender quarantines the unsigned installer. RQ-05 is
partial.

**Execution state (2026-11-21):** see
[the v0.0.31 record](evidence/2026-11-19-release-qualification-v0.0.31.md).
v0.0.31 is already published (source `a4e92bb05`, installer `b94e5276…3a92`),
supersedes every 0.0.30 candidate, and is the frozen candidate for RQ-01–06.
Open: the downloaded-file unsigned-publisher prompt, offline installed operation,
the v0.0.30 rollback fixture (host Defender detection), a second Windows build,
and the Defender verdict (Microsoft submission pending; signing decision in
[release signing](release-signing-and-publication.md)).

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

- [Current priorities](current.md), [M08](../../milestones/M08-release.md),
  [DIST requirements and decisions](../../spec/15-delivery.md), and
  [quality requirements](../../spec/14-quality.md).
- [Qualification runbook](../../operations/release-qualification.md),
  [verification profiles](../../operations/verification.md), and
  [release signing boundary](release-signing-and-publication.md).
- [Existing brief and final handoff](delegate-release-qualification.md) and
  [2026-11-18/19 attempt evidence](evidence/2026-11-18-installer-process.md).
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

**Next execution action:** inventory retained candidate/manifests and raw reports,
reconcile the intended release source with the VST handoff, and record the exact
candidate plus present/absent snapshot UUIDs. Then run the remaining interactive
RQ-05 cases on those bytes; rerun RQ-04 if the selected installer differs from
the qualified combined candidate. No VM is restored during plan preparation.
