# Claude / next-agent handoff: stabilize the virtual cable driver

## Current state (2026-10-10 evening) — read this first

This section supersedes sections 4–9 below, which describe the earlier
`28b989f5` candidate and remain as history. Details and hashes are in the
[repair record](evidence/2026-10-10-m03-render-commit-validity.md).

- **Installed in the VM:** candidate `492d8ca8` (slot provenance + packet
  outcome counters), bundle `C:\VMs\ar-share\repair-20261010-slot-provenance`.
  Two clean smokes, install/status and a 30-second direct run passed with
  zero error counters, 3,000 packets accepted on time and exact waveforms.
- **Why `28b989f5` failed (r3):** it silenced late-written render packets;
  the replacement records per-slot provenance before progress. A host timing
  model (`renderTimingModelChecks`, 738 driver checks) reproduces both the r3
  loss and the original stale replay and checks the fix.
- **Sustained runs fail from the VM, not the driver:** 300-second runs lose
  audio only during VM-wide events: all-CPU guest silences (2026-10-09 trace)
  and ~47 ms late timer delivery to every audio thread during guest
  Defender/Windows Update bursts (2026-10-10 trace). VirtualBox runs through
  NEM because WSL keeps the hypervisor on. Tools: `m03-scheduler-trace
  --silences`, `run-direct-audio.ps1 -Seconds 300 -TraceScheduling`.
- **User decisions pending:** quiet the guest (pause Windows Update,
  Defender exclusion) and/or a reversible
  [native VT-x session](../../operations/virtual-cable-native-vtx-session.md)
  or a bare-metal test PC for VCAB-24. Never change host settings yourself.
- **Prepared, not yet run:** [cable latency diagnostic](../../operations/virtual-cable-latency.md)
  (`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r2`, VCAB-25 proxy).
  Next guest action when the user returns: that one command on the installed
  driver, then review `latency-<run>.zip`.
- The user prefers host-side verification first and one combined, fail-fast
  guest block per step.

Prepared 2026-10-10 at the user's explicit request. This is the resumption
entry point for the driver investigation. Read it before suggesting another
test. It records current state and existing authorization; it does not grant
additional authority or replace AGENTS.md or the specifications.

## 1. Objective and user expectations

Converge on stable AudioRouter-owned Windows virtual cables. The user has
spent many evenings running failing VM tests and wants careful owning-layer
repairs, fewer speculative retries and a clear route to completion.

- The user explicitly requested deep code analysis and fixes before another
  test, documentation of findings, and direct commits/pushes to `main`.
- The latest code repair and its candidate package are complete. The next
  action is a clean guest installation smoke, not another code change based
  only on a guess. No result for this candidate's guest smoke has been received.
- Give complete copy/paste commands. Always identify **host PC** versus
  **inside the VM**. Do not ask the user to infer directories or arguments.
- Review each new result before increasing duration or moving to a riskier
  test. Do not repeat an unchanged failing test to gather the same evidence.
- Report what is proven and what remains uncertain. Compile success, helper
  counters and a pleasant listening impression are different evidence types.

## 2. Required reading and applicable rules

Read the current files, in this order:

1. Repository `AGENTS.md` and [documentation map](../../README.md).
2. [General active plan](current.md), especially DEC-18's current driver state.
3. [Virtual cable active plan](virtual-cable.md), including the latest active
   repair under Objective. Older dated sections contain superseded actions.
4. [Spec 17](../../spec/17-virtual-cable.md), [M03/DEC-18 in delivery](../../spec/15-delivery.md),
   and [virtual cable testing](../../operations/virtual-cable-testing.md).
5. [Latest repair and package evidence](evidence/2026-10-10-m03-render-commit-validity.md).
6. [Direct audio runbook](../../operations/virtual-cable-direct-audio.md) and
   [direct/speaker evidence](evidence/2026-10-09-m03-direct-audio-preparation.md).

Fresh-context kernel review is required by WP-04 for kernel changes. The
latest repair received that review. Re-review substantial further kernel
changes; an earlier review does not cover a new patch.

The actual local AGENTS.md disables Jev as a local check and limits the
GitHub review to the configured small-PR scope. Do not restore the older Jev
instructions pasted in chat or re-enable it without a new user request.

The task authorizes driver fixes; an old AGENTS.md introductory reference to
specification work does not cancel that explicit request. Scope still excludes
unrelated features and release claims. Preserve unrelated working-tree changes;
stage named files only. Read the tree before assuming it remains clean.

## 3. Machine boundaries and safety constraints

| Item | Verified path / state at handoff |
| --- | --- |
| Host repository | `C:\code\audiorouter`, branch `main` |
| Host shared folder | `C:\VMs\ar-share` |
| Same shared folder in guest | `Z:\` |
| Guest working folder | `C:\ar` |
| Guest identity | `AR-DriverTest` |
| VM configuration | `C:\VMs\AR-DriverTest\AR-DriverTest.vbox` |
| VM log | `C:\VMs\AR-DriverTest\Logs\VBox.log` |
| Next clean checkpoint | `03-test-signing-ready-20261007` |
| VM power state | Off at last read-only check; re-check before acting |
| Restored audio configuration | HDA, stored WAS, `useDefault=true`, no backend override |

Important: a host path such as `C:\VMs\ar-share` does **not** exist inside the
guest. Save guest output to `Z:\` to expose it on the host. Earlier confusion
about this wasted time and frustrated the user.

Mandatory boundaries:

- Never install, load, fuzz or exercise the test-signed driver on the host.
  Build/package/offline analysis are allowed; driver execution belongs in the VM.
- Preserve WSL and Hyper-V. The user explicitly refused changes that disrupt
  WSL. Do not disable the hypervisor, VBS, Memory Integrity or host Secure Boot.
- Do not change host power policy, audio defaults, certificate trust, timer
  settings, Visual Studio installation or boot settings as a workaround.
- A previous unexplained **host** bugcheck occurred during this investigation.
  Its dump did not contain an AudioRouter module. Do not claim its cause is
  established; do not repeat broad VS repair or host configuration experiments.
- The user operates native VM controls and guest commands. Earlier general
  computer-control authorization does not create unavailable mouse capabilities.
  Do not pretend to control a desktop if the current tools do not support it.
- Preserve original archives, dumps and privately recorded audio. Never commit
  private audio, private keys, API secrets, ETLs or third-party source archives.
- Do not enlarge buffers/storage, suppress counters, weaken thresholds or
  label an interrupted run clean to manufacture a pass.
- Stop only test processes owned by the current invocation. Never terminate
  unrelated AudioRouter, media, recorder or VirtualBox processes blindly.

In this Codex environment, .git/share writes required approval review, and
some CIM/System-log reads were denied. That was not evidence that the files
were missing. Claude should use its own actual permissions and report limits.

## 4. Commits and exact prepared candidate

At handoff preparation, source HEAD is `4ecb4e8b` on `main`, pushed to GitHub.

| Commit | Meaning |
| --- | --- |
| `28b989f5d8d16f20e0d1996a335c01bd7203c342` | Kernel packet-validity repair, tests and contracts |
| `4ecb4e8b` | Verified package checksums and next guest instructions |
| `5d4d1eeb` | Audio backend rollback verification and HDA skipped-transfer evidence |
| `0cc1241e` | Recorder-free scratching and virtual-clock recovery review |

This handoff will have its own documentation commit. A later documentation
HEAD does not invalidate the candidate's clean **code** source identity.

**Already built host bundle:**
`C:\VMs\ar-share\repair-20261010-render-commits`.

Inside guest: `Z:\repair-20261010-render-commits`.

Expected `driver\package.json`:

```json
{
  "version": "0.1.0",
  "driverVersion": "0.1.0.0",
  "builtAt": "2026-10-10T17:08:27.6787581Z",
  "gitCommit": "28b989f5d8d16f20e0d1996a335c01bd7203c342",
  "dirty": false,
  "platform": "x64",
  "configuration": "Release",
  "signed": "test"
}
```

- Signed SYS SHA-256:
  `91BBC17ADCC55482D230B932A8198A566E5C3AFC5398588C810B8CC9135D7BB6`.
- Manifest SHA-256:
  `69FD263F8005A555587BB477FE913F78D3A595EDCA77D9B58A0B88F83DFD8F67`.
- Package integrity: **33 checks passed**, including tamper rejection.
- Independent manifest validation: **32 files passed**. These counts describe
  different checks, not a missing file.
- No host trust, installed driver, VM configuration or security change occurred
  during packaging. Native tools/fuzzer were built and were not executed.

Do not rebuild this package merely because another agent is continuing. Verify
its existing manifest and use this identity for the next smoke.

## 5. What the latest repair actually changes

Owning files:

- `drivers/audiorouter-virtual/Source/Inc/streamtiming.h`
- `drivers/audiorouter-virtual/Source/Main/minwavertstream.cpp` and `.h`
- `drivers/audiorouter-virtual/tests/bridge_tests.cpp`
- `tests/acceptance/m03-driver-build.ps1`
- Spec 17's counter semantics and committed-render-packet section.

### Confirmed source defect

Previously `ReadBytes` consumed elapsed render DMA without checking logical
packet commits from `SetWritePacket`. If a two-slot DMA buffer contains A/B
from committed packets 0/1 and Windows misses packet 2, the reader can publish
old A again. Timely callbacks, an empty private FIFO and advancing publication
numbers do not prevent this. All bridge error counters can remain zero. The
timer's ETW underrun check runs after consumption and does not count that loss.

Also, admission used a position snapshot under the spin lock, released the
lock, validated and then reacquired it to commit. Progress could advance in
that unlocked interval.

### Implemented repair

1. Stream metadata tracks two **absolute 64-bit logical packet identities**
   with validity bits. The wire PacketNumber remains 32-bit and wraps normally.
2. Before updating position in `SetWritePacket`, invalidate tags for its incoming
   physical slot. Windows writes the payload **before** reporting it; retaining
   an old tag could otherwise classify incoming bytes as a prior packet.
   Rejected submissions also invalidate old physical-slot validity.
3. Position update, admission, write-position mutation and successful tag
   insertion share the existing position lock. Admission remains current packet
   before RUN, current + 1 during RUN.
4. `ReadBytes` uses the absolute logical byte position, including surviving-lap
   skips/wrap. Missing committed frames become zero without reading or clearing
   user-owned DMA. While a usable render lease exists, each missing frame
   increments render `UnderrunFrames`, once outside the channel loop.
5. Init, STOP and allocation/release reset metadata; PAUSE retains it. Existing
   callback cancellation/join ordering remains intact. No sample buffer growth,
   new callback allocation, waits, disk I/O or bridge ABI change.
6. Notification allocation accepts documented counts 1/2 and rounds the cyclic
   size to whole frames **per packet**, avoiding a split-frame midpoint.

### Limits that must not disappear in the next report

- Strict tracking starts only after the first **successful** `SetWritePacket`.
  Earlier consumption and legacy event/polling clients remain unchanged and
  outside this protection. Actual guest packet-mode activation is not yet proven.
- One-slot next commits reuse the current physical slot. Remaining current
  samples conservatively become counted silence. Count-1 continuity is
  **unqualified**, not silently declared glitch-free.
- `SetWritePacket` still rejects EOS packets. Earlier render FIFO/tail work
  does not establish this DDI's EOS support. It is a separate lifecycle gate.
- Host helper tests exercise production validity/admission helpers; sample
  substitution/counter accumulation are modeled, not execution of kernel
  `ReadBytes`. Source guards/review check that wiring.
- This is a confirmed source defect, **not proven attribution** of the saved
  12-second phase breaks, all long-run loss, or downstream speaker scratching.

## 6. Checks already run; do not misreport their scope

| Check | Result / evidence |
| --- | --- |
| MSVC offline bridge/helper suite | 620 passed; `target/driver-unit-release/tests.log` |
| x64 WDK/catalog/source acceptance | Passed; `target/render-commit-x64-final-acceptance.log` |
| ARM64 WDK/catalog/source acceptance | Passed; `target/render-commit-arm64-acceptance.log`; compile-only |
| Fresh-context kernel review | No remaining scoped source blocker after alias ordering and one-slot regression |
| Signed candidate build/package | Passed; `target/render-commit-candidate-build.log` |
| Tamper evidence | `target/driver-package-test-755ef67f063b440dbf73c40ef3b3c78c` |
| Documentation links/fences | Passed; latest pre-handoff run 142 Markdown files / 786 links |
| Git diff whitespace | Passed |
| Candidate guest smoke/tone | **Not run / no result received** |
| Rust/UI format, Clippy, UI checks | Not rerun: no Rust/UI source changed in this repair |
| Jev local check | Disabled by explicit user decision |

### Build trap and safe remedy

Default HostX86 linking failed with `LNK1101: incorrect MSPDB140.DLL version`.
The preserved log is `target/render-commit-x64-review/build.log`. Do not repair
Visual Studio. Existing process-local tool selection resolved the failure:

```powershell
# HOST build shell only; changes end with this process.
$env:PreferredToolArchitecture = 'x64'
$env:Path = 'C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64;' + $env:Path
```

Installed MSVC is 14.51.36231; its DLL file versions can be 14.51.36256.
WDK build used 10.0.28000.0. Do not invent a missing tool directory or change
the Rust 1.96.0 pin. Re-check installed paths if the environment changes.

## 7. Exact immediate next step: first clean smoke

Before issuing these steps, inspect new user messages/results. If the user
already completed them, consume the evidence rather than repeating the run.

Give the user these instructions:

1. In **VirtualBox on the host PC**, with AR-DriverTest off, restore the existing
   checkpoint **`03-test-signing-ready-20261007`**. This restores guest state;
   existing host evidence in `C:\VMs\ar-share` survives. Do not create a new
   checkpoint over a possibly installed/failed driver baseline.
2. Start AR-DriverTest normally. Open **Administrator PowerShell inside the VM**.
3. Keep Media Player, Cable B Listen and other audio test tools off. Paste:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'Z:\repair-20261010-render-commits\retry-smoke.ps1'
```

4. Send the final output. Do not proceed to a tone, stall, fuzzer or long run.

The existing runner is already included and reviewed. It checks VM/admin
identity, copies the candidate into `C:\ar`, verifies all manifest entries,
prints package identity, runs preflight, then one smoke. Smoke installs,
enumerates endpoints and removes the package, checking baseline restoration.
The runner collects evidence even after a smoke failure and copies the ZIP to
the share's parent: guest `Z:\`, host `C:\VMs\ar-share`.

Expected success: correct clean source identity, 32 copied files verified,
15 preflight checks pass, smoke passes, collect passes, evidence copied, then
the runner asks for a clean snapshot before the second run. A successful smoke
is installation/cleanup evidence, **not audio continuity evidence**.

If preflight fails, the runner stops before smoke and does not necessarily
collect. Inspect the specific failure, the guest transcript and current boot
state; do not change host boot settings or blindly rerun smoke. If smoke fails,
read its runner summary, install/remove reports, installation-time PnP state,
SetupAPI and CodeIntegrity evidence. Before/after cleanup snapshots alone do
not show why the driver failed while installed.

If the VM bugchecks, preserve its new minidump and event/time/package identity;
stop retries and analyze the dump. Do not confuse a guest crash with a host
crash or reuse an earlier dump as evidence for this candidate.

## 8. Following steps after the first result

### A. Verify the evidence on the host

Locate the exact newly named ZIP in `C:\VMs\ar-share`, verify the checksum if
provided, and extract bounded contents into ignored `target` scratch. Record
package source identity, run timestamp, checks, cleanup and baseline equality
in the active evidence record. Do not choose an arbitrary old ZIP by name alone.

### B. Second independent clean smoke

Only after the first smoke/evidence passes, have the user power off, restore
the **same clean checkpoint**, boot and run the **same exact command** from
section 7. Review this second archive separately. Do not run it against a
possibly contaminated baseline merely because the first helper returned 0.

### C. Install and verify active endpoints

After both clean smokes pass, use the freshly copied `C:\ar\vm-checks.ps1`
to install and check status inside the VM. Use separate commands with all
arguments; stop dependent work on failure:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\vm-checks.ps1' -Step install
```

After install passes:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\vm-checks.ps1' -Step status
```

Collect and expose the new archive as needed; collection is safe even when a
preceding check failed:

```powershell
& {
    $collectionStart = Get-Date
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\vm-checks.ps1' -Step collect
    if ($LASTEXITCODE -ne 0) { throw 'Collection failed; preserve C:\ar\evidence.' }
    $zip = Get-ChildItem -LiteralPath 'C:\ar' -Filter 'evidence-*.zip' -File |
        Where-Object { $_.LastWriteTime -ge $collectionStart } |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $zip) { throw 'No new evidence ZIP found.' }
    Copy-Item -LiteralPath $zip.FullName -Destination 'Z:\'
    Get-FileHash -LiteralPath $zip.FullName -Algorithm SHA256
}
```

Expected endpoints are A/B Input (**Render**) and A/B Output (**Capture**).
For all four active endpoints, prior candidates supported 60/60 advertised
formats with min/default/max periods 128/480/480 frames. Historical disabled
MMDevice entries must not count as active duplicates; that inventory defect
was already fixed. An install/status pass still does not qualify sound.

### D. Prepare one focused automatic waveform test

Do engineering preparation before giving the user another audio command.
The current 32-file candidate contains standard VM tools; do **not** assume
it contains `run-direct-audio.ps1` or `m03_direct_audio.exe`.

Read these current source files before assembling a diagnostic update:

- `tools/vm/build-direct-audio.ps1`
- `tools/vm/prepare-direct-audio-update.ps1`
- `tools/vm/run-direct-audio.ps1`
- `tools/vm/paired-trace-support.ps1` and `portable-tool-support.ps1`
- `crates/windows-audio/examples/m03_direct_audio.rs` and its module directory
- `tests/acceptance/m03-direct-audio.ps1`
- `tools/vm/vm-checks.ps1` and guest process-support helpers.

The existing direct runner generates Cable A input and records Cable B
directly, checking readiness before starting a bounded 30-second native tone.
It analyzes both recordings, checks counters and packet metadata, and copies
only this run's reports/recordings. It requires additional support files and
bundle integrity metadata; the prepare script expects a paired-tone base.
Do not blindly pass the new standard bundle to a base-specific assembler or
reuse an old bundle that silently selects old tools/driver metadata.

Prepare a source-identified, independently hash-verified diagnostic bundle
with the repaired driver identity preserved. Build with static runtime;
startup-check the executable in the guest before any audio. Run native-free
regressions for changes, and keep process/recording watchdogs and bounded
storage. Do not require Audacity or manual playback when the automatic source
can supply the controlled signal. Keep Cable B Listen off to isolate direct
sample quality from guest HDA/host playback. Provide the final exact command
only when the needed bundle actually exists and its dependencies are verified.

Check clean duration, 47/997-Hz continuity, generated-source agreement,
discontinuity/timestamp flags, honest counters and native packet-mode coverage.
If it fails, analyze that new evidence before another retry. Only after a
short diagnostic passes should a longer **bounded** test be proposed; inspect
existing specification gates instead of declaring 30 seconds a stable release.

## 9. Keep the three open investigations separate

### A. Driver render validity

The latest patch fixes a confirmed source exposure; guest activation and
integration are pending. If new render underruns appear, that can reflect
newly honest accounting. It is a failed continuity gate, not permission to
ignore the counter. Verify packet commits/cadence and sample position before
changing admission or resetting counters.

### B. Shared scheduling stalls and actual loss

Prior five-minute runs failed despite short runs passing. Saved guest trace
analysis found audio workers waiting roughly 28–36 ms, mostly before readiness;
once ready, they ran quickly at priority 24. Other recordings showed common
worker/control pauses near 288–317 ms. This is not proof of host sleep or one
particular driver DPC. The traced long run failed with 7,056 capture underrun
and 14,832 render overrun frames. Keep these real losses open.

Scratch/evidence entry points:

- `target/trace-tone-review-20261009-200614`: decoded analysis and thread/DPC
  reports. Longest decoded DPC ~2.754 ms, not an identified 35-ms culprit.
- `target/pair-tone-review-2efe2c4c`: paired **passing** 30-second trace. Clock
  calibration uses QPC brackets; guest UTC drift made UTC-only alignment unsafe.
- [Packet clock evidence](evidence/2026-10-09-m03-packet-clock-review.md).
- [Paired trace runbook](../../operations/virtual-cable-paired-trace.md).

The passing paired run does not attribute a long failure for which no paired
host trace exists. No mock/offline check is live timing evidence. Keep WSL.

### C. Downstream speaker scratching

An independent PCM16 44.1-kHz reference, without AudioRouter-generated samples,
also produced audible scratches. With Listen off the user still heard variable
scratches, including after recording stopped. A DirectSound comparison did not
provide reliable clean playback and has been rolled back. Do not repeat it
unchanged or claim the recorder is the sole cause.

Relevant evidence:

- WAS speaker capture `speaker-loopback-0e7000c055ba41d392abb3a902097fca.zip`:
  after a 64-frame offset, **749,260 stereo frames / 16.990023 seconds** match
  the reference exactly, including both reported noisy sections. This locates
  those audible scratches after the guest loopback sample boundary, without
  identifying the exact faulty component. The entire 30 seconds is not clean.
- DirectSound capture `speaker-loopback-98d3cd98893d4095b97504508cf77ee6.zip`:
  nominal sample progress ~35 seconds versus ~25 seconds elapsed, hitting a
  bounded recording limit. This is a timing discrepancy, not a reason to
  enlarge the recording bound.
- Whole closed-session VirtualBox log reports **11 output HDA transfers**
  skipped while a completion interrupt was pending. Input stream underruns
  are a different statistic; do not mislabel them as speaker errors. The
  output count lacks timestamps tying each skip to a scratch.
- Later overnight HostSuspend/HostResume records exist. They do not establish
  the cause of the earlier user-attended audio failures. Host System-event
  reads were denied, so no unsupported power-cause assertion should be made.

Host archives are under
`C:\VMs\ar-share\diagnostics-20261009-speaker-loopback`.
Reviewed scratch is `target/speaker-review-0e7000c0`,
`target/speaker-review-98d3cd98`, and `target/vbox-playback-review-20261009`.
The last contains the preserved closed log and selected official VirtualBox
source files. Do not execute/compile or commit the third-party archive.

The direct r2 archive
`C:\VMs\ar-share\diagnostics-20261010-direct-audio-r2\direct-4586cab5c56d429c8835b3c0eaa3008c.zip`
showed a separate common ~288-ms pause, 12,912 frames lost in both directions,
and Cable A phase breaks near 12 seconds before that later pause. Cable B
samples between the loss matched its intended tones. These observations do
not prove that the newest commit fix resolves every recorded defect.

Do not set guest Speakers to unsupported 48 kHz: observed HDA Speakers offered
16-bit 16/22.05/44.1 kHz. The independent reference uses supported 44.1 kHz;
the direct cable probe uses float32 stereo 48 kHz. Distinguish playback format
conversion from direct cable sample integrity. A format mismatch alone was
not proven to explain the scratching.

## 10. Completion gates, rollback and reporting

Current package is a test-signed VM candidate. Do not call it stable, production
signed or ready for the host. Open gates include native commit-mode coverage,
kernel waveform/counter integration, EOS, count-1 operation, sustained
continuity, DPC/pool/CPU measurements, latency, multiple active cables and
production signing/distribution.

In particular VCAB-24 requires zero discontinuities and counter increases for
one hour with eight active cables; VCAB-25 requires measured impulse latency.
Do not silently substitute the two-cable 30-second helper test for either.

For any additional code repair: update plan and contracts, add a targeted
regression, perform fresh kernel review where applicable, run owning-layer
checks, inspect/stage named diffs, commit/push and create only one new clean
source-identified candidate. Rust changes require both mandated formatting
and Clippy workflows; callback code must remain bounded and nonblocking.

Rollback of a failed guest candidate: stop dependent tests, preserve its
evidence and restore the clean guest checkpoint. Preserve the verified host
backend restoration and host settings. Do not delete old packages or overwrite
the candidate whose identity belongs to a failed recording. Source rollback
uses a new revert commit; do not rewrite shared main history.

Keep the active plan and evidence sufficient to resume without chat history.
Report requirement IDs, changed files, exact commands/environment/results,
unrun checks, open blockers and the next specific action. The user should
receive one small actionable step at a time while the agent does the evidence
review and package preparation independently.
