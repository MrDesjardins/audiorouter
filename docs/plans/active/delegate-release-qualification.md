# Delegation brief: app-only release qualification

Prepared 2026-11-18. Status: ready to delegate; no execution performed by this brief.

## Assignment and scope

Complete the repeatable Windows installer-lifecycle qualification for an
app-only candidate. Own the harness, VM evidence and qualification report.
Requirements: DIST-01 through DIST-08, especially DIST-02/08; NFR-16; M08.
Map each acceptance case to the exact requirements after reading the specs.
This work does not complete the separate hardware, usability or driver gates.

Follow repository AGENTS.md. This brief is a task plan, not permission to
publish, push, open a PR, purchase signing, install a driver or alter the
owner's workstation. DEC-17 permits an unsigned first app release; disclose
its trust behavior. App signing is not a prerequisite for this app-only task.

## Read before starting

Read these completely, then their relevant linked decisions/specifications:

- [Project overview](../../README.md) and [current priorities](current.md).
- [Release qualification runbook](../../operations/release-qualification.md).
- [Delivery specification](../../spec/15-delivery.md), M08 and DIST-01–08;
  [quality specification](../../spec/14-quality.md), NFR-16.
- [Historical VM results](../archived/2026-11-16-active-plan-history.md),
  particularly the November 16 installer and unattended continuation.
- [Release signing plan](release-signing-and-publication.md), for boundaries;
  [verification runbook](../../operations/verification.md).
- `tools/vm/installer-lifecycle.ps1`, `installer-lifecycle-guest.ps1`,
  `installer-lifecycle-unattended.ps1`, and relevant `tools/release/` scripts.

## Starting evidence and prerequisites

Historical evidence establishes one lifecycle run with registration, a real
session database and a generated WAV, including reinstall, downgrade, upgrade,
uninstall and reinstall. A generated WAV is not an AudioRouter recording.
Unattended saved-state restores intermittently hang in VBoxManage. A process
exit-code fix lacks a successful corrected end-to-end rerun. Do not treat
either result as current-candidate or reliable unattended evidence.

Inventory Git status, branch, HEAD, VM UUID/snapshots, tools, credentials and
retained artifacts before mutation. Preserve unrelated files. Historical VM:
`AR-ReleaseQualification-20261116`, with private signed-in standard-user
snapshot `lifecycle-ready-20261116`; confirm identities rather than trusting
names. Never use or restore the source `AR-DriverTest` VM for this assignment.
Passwords belong outside the repository and logs; request unavailable secrets
through the supported secret mechanism. A missing VM/credential blocks VM
execution, not independent harness review or regression work.

## Ordered work packages

1. **RQ-01: reconcile and freeze inputs.** Record source commit, candidate and
   previous versions, installer paths and manifest-verified hashes, Windows and
   VirtualBox versions, and exact disposable VM/snapshot identities. Use a
   clean committed source for artifact preparation. Do not clean other work
   to satisfy that guard. Update this plan with prerequisites and evidence paths.
2. **RQ-02: qualify guest-control recovery.** Reproduce the restore hang with
   bounded instrumentation. Distinguish readiness/login failures, VBoxManage
   hangs, installer completion and harness failures. Retain step start/end,
   timeout, exit status, owned process identities and redacted diagnostics on
   every exit. Fix the owning layer with a regression. Terminate only owned
   runner descendants; preserve a guest that cannot shut down for inspection.
   Do not force-reset it or retry until a failure disappears. Require three
   consecutive restore/run/shutdown passes on the corrected runner, plus an
   injected timeout proving bounded cleanup, honest failure and a successful
   subsequent clean run. Record this repeatability threshold before execution;
   it is task evidence, not a replacement for any normative reliability gate.
3. **RQ-03: protect the baseline and refusal paths.** Verify a dedicated,
   recoverable standard-user baseline, loaded user hive and host audio disabled.
   Exercise wrong VM/snapshot, wrong hash, existing staging/app directory,
   administrator account and unloaded hive guards. Unsafe targets must remain
   untouched. Keep live-session snapshots private. Snapshot restore discards
   guest state: operate only on the verified disposable target.
4. **RQ-04: complete lifecycle and preservation.** Run fresh install,
   same-version repair/reinstall, previous-version rollback, candidate upgrade,
   uninstall and reinstall. Check per-user registration/removal and exact
   installed versions/companions. Seed a session through the installed CLI.
   Create a non-private recording through AudioRouter's production recorder
   using generated test audio and an isolated guest route; verify decodability,
   frames/content and hash after every lifecycle step. Read the retained session
   with the installed CLI after each applicable step. Prove repair restores an
   intentionally removed owned companion. Record unsupported downgrade/data
   compatibility honestly; never overwrite preserved originals to force a pass.
5. **RQ-05: WebView2 and first run.** Use separate dedicated baselines with
   runtime present and absent. Prove absence before setup; do not remove the
   host runtime. Exercise bootstrap success and unavailable-network/failure
   behavior, retry/recovery, actual shell launch and companion discovery.
   Inspect interactive unsigned trust/setup behavior. Verify stopped startup,
   actionable missing-endpoint guidance and no default-device/volume changes.
   Do not infer interactive behavior from silent NSIS exit codes.
6. **RQ-06: final candidate report.** Run relevant acceptance and regression
   checks, verify artifact provenance/checksums/notices/SBOMs and inspect the
   complete diff. Update the runbook and evidence mapping with pass/fail/not-run
   per case. Keep other M08 hardware, endurance, accessibility and signing
   evidence separate. Hand off a locally reviewable branch; do not publish.

## Verification and deliverables

Use [the verification profiles](../../operations/verification.md); run docs
acceptance and `git diff --check`. Run focused harness regressions including
negative guards and the real disposable Windows lifecycle. If build/code
changes require Rust/UI checks, follow all AGENTS.md formatting, lint, Clippy,
contract and real-entry-point rules. A script parse or mocked VBoxManage test
alone cannot pass a Windows lifecycle gate.

Retain a new evidence directory per attempt with structured results and
redacted host/guest logs. Include source and artifact identity, environment,
commands, step durations, registration, database/recording checks, cleanup and
failures. Link a durable redacted summary under `docs/plans/active/evidence/`;
do not commit credentials, private recordings or opaque VM snapshots.

Done for this assignment: RQ-01–06 have reproducible evidence, or the report
explicitly identifies the remaining external gate and finishes all independent
work. Do not mark the overall release or M08 complete with missing evidence.
Report branch/commit, files, requirements, checks, limitations, exact next task
and local test instructions before requesting any PR approval.

## Coordination, risks and rollback

Own `tools/vm/installer-lifecycle*.ps1`, this plan and release evidence/runbook.
Coordinate shared release/build-script changes with the VST agent. The VST
agent supplies its exact shell/CLI/four-worker manifest; qualify the final
combined candidate again if its bytes change. Earlier artifact results do not
transfer to a rebuilt installer. Serialize heavyweight builds and VM runs.

Risks: guest-control hangs, private saved sessions, stale artifacts and
confusing generated-file retention with recorder qualification. Roll back code
with scoped commits and restore only the dedicated VM's own baseline. Keep
failed evidence. Worktree cleanup must use the AGENTS.md cleanup script.

## Execution update — 2026-11-18

RQ-01 initial inventory: checkout branch `work/release-qualification`, HEAD
`33cdf58d4f577258e9d372c58ac5d7d3230f30f4`; only this copied plan is untracked.
VirtualBox 7.2.20. Dedicated VM `AR-ReleaseQualification-20261116`, UUID
`87c31665-2022-49a2-a182-1a1ebb5e631c`, is powered off. Its current snapshot
`lifecycle-ready-20261116`, UUID `74a7f354-c6c4-4be1-ac2d-81a734c08996`, is
owned by that VM and describes a signed-in standard user; the separate
`AR-DriverTest` VM is not used. Historical candidate source `20503e066`, hash
`4ac481cac4fabb7f8e9e507f66dd65a4db74efa78369352a6d9ebcee575663c1`, and
previous 0.0.29 hash
`f2c43d115f04fdef00f55ef96862eba41d581464db24785cde5f69755ba209d8` are
historical only. Current-candidate inputs remain to be reconciled.

RQ-02 implementation sequence: add a bounded owned-process VBoxManage runner
with JSONL step timing/exit/timeout/failure records, use it for every VM
operation in the unattended wrapper, and add an injected hung-command test
that verifies bounded termination, honest failure output, and a clean next
invocation. Then run PowerShell parsing and focused regressions before any VM
restore. The lifecycle recorder requirement is not met by the existing guest
script's generated WAV; replace it with a production recorder route before
claiming DIST-02/08 acceptance.

Host Windows build 26300.9974 (26H2). Located both historical manifests and
installers in the host temp directory. The bounded runner accepted both
archived SHA-256 values. Historical attempt
`%TEMP%/ar-rq02-historical-a71cc8bc-4a19-46b1-81f0-e26676a7c4e8`
passed restore, readiness, silent lifecycle and graceful shutdown. Startup
took 13.5 s and guest lifecycle took 38.3 s. This run used the generated WAV
and preceded the companion repair check. The real intermittent hang has not
been reproduced in this run.

Windows process regression command:
`powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m08-lifecycle-process.ps1`.
Passed argument round trip, tolerated and fatal nonzero exits, launch failure,
injected timeout, descendant exit by PID and creation time, unrelated process
survival and recovery. This is host process evidence. Three consecutive
historical passes later met the repeatability threshold for that harness
revision. The real VM timeout recovery check remains open.

DIST-01/02, NFR-16 and DIST-08 have partial historical guest evidence only.
DIST-03/04 need installed shell and WebView2 scenarios. DIST-05 publication
execution remains outside this assignment. DIST-06 needs the final combined
manifest, notices and SBOMs. DIST-07 still needs interactive trust observation.

### Current checklist

- RQ-01: historical inputs verified. Current combined installer and manifest
  pending VST handoff. Preparation requires a clean committed source.
- RQ-02: three consecutive historical runs on the same harness passed.
  Injected guest timeout confirmed client exit. Marker, graceful shutdown
  and clean recovery remain open. See the [attempt record](evidence/2026-11-18-installer-process.md).
- RQ-03: wrong hash and excluded VM refusal passed. Baseline now checks
  guest login before loading a profile. Wrong snapshot and guest refusal
  cases need another disposable run.
- RQ-04: companion repair was verified on the historical installer pair.
  Production recorder fixture and decoded WAV checks added. Host WAV
  regression passes and installed recording remains unrun.
- RQ-05: optional WebView2 baseline guard and a stopped-startup check added.
  Absent baseline, network/bootstrap recovery, endpoint guidance, audio
  defaults and unsigned trust behavior are still open.
- RQ-06: historical app-only manifest verifier, docs and process regressions
  pass. Final candidate evidence remains open.

Exact next VM action: inspect the stuck release guest after timeout attempt
`936e89ef` and obtain a graceful poweroff before any restore. Then run the
marker-file timeout, clean recovery and production recorder cases.

### Implementation and evidence status

- **RQ-01 / DIST-06:** historical artifact and snapshot identification complete
  and exercised. Final combined artifact identity and provenance remain open.
- **RQ-02 / DIST-08:** host bounds, evidence and probe implemented. Local process
  suite and earlier historical runs passed. Reached marker and clean recovery
  through the newest VM harness require graceful recovery of the running guest.
- **RQ-03 / DIST-01/08, NFR-16:** wrong snapshot, existing staging/app,
  administrator and unloaded hive guards exercised using synthetic command
  output, a local cmd process, and guest profile inputs. Actual guest refusal
  cases await an independent disposable baseline.
- **RQ-04 / DIST-02/08:** recorder fixture and per-transition file checks
  implemented. Synthetic RPC framing, cancellation, cleanup and WAV
  verification passed. Installed recording and retention remain unverified.
- **RQ-05 / DIST-03/04/07/08:** present/absent inventory and guest NAT-only
  bootstrap failure/retry implemented. Synthetic runtime classification and
  link-restoration sequencing passed. Real bootstrap behavior, installed shell,
  endpoint guidance, defaults/volumes and unsigned interactive trust remain open.
- **RQ-06 / M08:** local acceptance and historical manifest verification
  exercised. Final combined package, runtime baselines and signed-off
  interactive evidence remain external dependencies.

See the [redacted process and VM evidence](evidence/2026-11-18-installer-process.md)
for the attempt IDs and results. The guest recording root is approved through
the desktop window under REC-07. The production recording case needs to
exercise that path in the disposable guest, use generated audio through the
installed app and check decoded output.

The three consecutive historical passes are complete. The full workflow
timeout still needs a reached marker, graceful shutdown, and a clean repeat.
Once the preserved guest is available again, exercise the production
recording and first-run checks through the installed desktop entry point.
Use the final combined app-only manifest for release evidence.

## 2026-11-18 preserved guest recovery

The preserved guest `AR-ReleaseQualification-20261116` was stuck in Windows
Recovery after a failed Automatic Repair, so guest control and ACPI shutdown did
nothing. Through the VM console I chose Continue's neighbour, "Turn off your PC",
and the VM reached `poweroff` without a forced power-off. Snapshots
`pre-harness-20261116` and `lifecycle-ready-20261116` are untouched.

Next action: restore `lifecycle-ready-20261116` (the current disk state is a
failed-repair guest, not a valid baseline), then rerun the full-workflow timeout
case to a reached marker with graceful shutdown and a clean repeat, followed by
the installed recorder and first-run checks. Still open: WebView2 bootstrap on
a NAT-only baseline, and the combined app-only installer with a verified manifest.

## 2026-11-18 final harness qualification and handoff

This section supersedes the "next action" paragraphs above.

Baseline: the saved-memory snapshot `lifecycle-ready-20261116` became unusable
(guest control failed with `VERR_NOT_FOUND` after every restore), and a live
snapshot of the guest wrote about 2 GB of 8 GB in 25 minutes and stalled the
VM. With the owner's approval I terminated that hung VirtualBox process. The
new baseline is the offline snapshot `lifecycle-offline-20261118`
(`45d20e3c-8066-4896-9e28-a206f9d88723`), taken from a clean shutdown. The
wrapper cold-boots it, waits for guest control, and signs the standard user in
at the console (password from the credential file, never logged) so the profile
hive is loaded. A boot stall at the VirtualBox logo happened once; the wrapper
now keeps a screenshot and names the failure.

Harness fixes found by running it: marker path without quotes and a warm-up
session before the bounded probe; marker match tolerant of a trailing space;
shutdown wait 300 s; guest and host lifecycle deadlines 400/415 s; recorder
reads `EBWebView\DevToolsActivePort`; window consent through `devices.setAccess`;
`idempotencyKey` on `sessions.play`; a stereo Recorder route (the engine
rejects mono tap-only branches); `audioSources.transport play` before
recording, because a Test Signal is silent until played (the first recording
was all zeros); the WAV verifier accepts mono or stereo.

Results on the final revision (evidence in the
[attempt record](evidence/2026-11-18-installer-process.md)):

- **RQ-02:** injected guest-control timeout `ar-rq02-timeout-327fe687`: bounded
  at 12 s, marker reached, honest failure, graceful shutdown; followed by three
  consecutive clean production-recording passes (`627dd10c`, `3d3a3b62`,
  `eb7f7c7b`). Earlier timeout runs failed for the reasons above.
- **RQ-03:** wrong snapshot UUID refused before touching the VM
  (`ar-rq03-wrongsnap-1049c80e`); wrong hash and driver VM refused earlier.
  Guest-side refusals (administrator, unloaded hive, existing staging) are
  covered only by synthetic tests.
- **RQ-04:** fresh install, repair, rollback to 0.0.29, upgrade, uninstall and
  reinstall pass with the installed app recording a generated 440 Hz tone through
  its own bridge (stereo, 48 kHz, RMS 0.089, hash retained after every step).
- **RQ-06 / DIST-06:** the VST thread's combined candidate
  `release-candidate-2026-11-18b` (source `d6c400c7b`, installer SHA-256
  `d76278cb838f824a53d899c1091ea79571fef9748224dd3d218169a7ef68814f`) passes
  `tools/release/verify-artifacts.ps1` and two full lifecycle runs
  (`ar-rq06-combined-8aec0750`, `ar-rq06-combined-7db05993`). Rebuilt bytes need
  a new qualification.
- Host: process, guards, recording and docs acceptance pass; `git diff --check` clean.

- **RQ-05 (partly met, 2026-11-19):** a runtime-absent baseline now exists
  (`lifecycle-webview-absent-v10-20261119`, built from the guest's offline
  baseline by removing WebView2 as an administrator and disabling the Edge update
  services and tasks). `-WebViewBaseline Absent -ProductionRecording
  -OfflineBootstrapRetry` (`ar-rq05-372e4035-547a-456d-a532-6e8501b1f6aa`) shows:
  with the guest NAT link disconnected, setup exits 2 and leaves no registration,
  files, database, shell or runtime; after the link returns, the same installer
  bootstraps WebView2 (157.0.4341.60) and passes install, repair, rollback,
  upgrade, uninstall and reinstall with the 440 Hz recording intact. The harness
  reports the overall run as failed on purpose: the offline exit is recorded as
  `unattributed-installer-failure` because the silent NSIS setup writes no
  bootstrap diagnostic, so attribution to the bootstrapper is not proven.

Still open: interactive unsigned-trust and setup prompts, endpoint guidance,
and default-device and volume checks (RQ-05), because VirtualBox headless has no
mouse and nobody watched the screen. A first-run "What do you want to do?"
chooser appeared in the installed shell (seen on screen only). M08 hardware,
endurance, accessibility and signing gates are untouched.

Next action: have a person observe one interactive install in the
absent-runtime baseline (GUI VirtualBox), then review the diff.
