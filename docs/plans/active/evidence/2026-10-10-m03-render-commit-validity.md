# M03 committed render packet validity repair — 2026-10-10

## Scope and reproduction

Requirements VCAB-12/20/24/27/28, VDEV-12. User requested the next engineering
step toward a stable driver before another guest retry. VM remains off with
original WAS/default backend restored; WSL and host settings are unchanged.

Source reproduction: fill a two-packet DMA ring with distinct packets A/B;
commit packets 0/1, miss packet 2, then commit packet 3. The old `ReadBytes`
reads A again at logical packet 2 without a bridge error: every callback can
be shorter than one DMA lap, and the private queue can remain empty. The
timer's subsequent ETW underrun does not increment bridge counters. This
establishes a stale-data exposure in source, not the cause of the saved Cable
A phase breaks or independent speaker scratches.

Microsoft's [SetWritePacket contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nf-portcls-iminiportwavertoutputstream-setwritepacket)
allows mitigation of circular-buffer repetition while packet clocks and
notifications continue. The [allocation contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nf-portcls-iminiportwavertstreamnotification-allocatebufferwithnotification)
defines notification counts 1/2.

## Owning repair and compatibility decision

- `streamtiming.h` contains stream-owned, constant-size commit metadata:
  two absolute logical packet identities and validity bits, at most 24 bytes.
  Existing admission allows only current before RUN or current + 1 in RUN.
- `SetWritePacket` invalidates the incoming physical slot before updating
  progress, because the OS has written payload before reporting its packet.
  This applies to rejected submissions too. Progress, admission, write-position
  update and successful tag insertion share the position lock.
- `ReadBytes` tests each consumed logical frame against those identities.
  Invalid frames become zero without reading or clearing user-owned DMA;
  each increments render `UnderrunFrames` once while a usable lease exists.
  Clock progress, notifications, valid samples and surviving-lap bounds remain.
- Init, STOP and buffer allocation/release reset metadata. PAUSE retains it.
  There are no retained DMA pointers, allocation, waits or file I/O in the
  helper. Existing timer cancellation/join ordering is preserved.
- Strict mode latches only after a successful packet commit, retaining legacy
  event/write-position and polling compatibility. Consumption before that
  first commit and clients never making commits remain outside protection;
  zero counters there do not prove producer validity.
- One-slot next commits overwrite the current slot, whose remainder becomes
  counted silence. Count-1 continuity is unqualified. EOS packet rejection
  is unchanged and remains a separate lifecycle limitation.
- Notification allocation accepts counts 1/2 and rounds cyclic buffers to
  whole frames per packet. No sample buffer growth, ABI change, host policy
  change or acceptance threshold relaxation.

## Review and verification

Fresh-context kernel review independently identified stale DMA and the
unlocked admission interval. Its patch review found no remaining source
blocker within this scope, after physical alias invalidation was included.
It requested explicit one-slot remainder coverage, now added.

- Windows host MSVC offline helper suite: **620 checks passed**, log
  `target/driver-unit-release/tests.log`. Command:
  `powershell.exe -NoProfile -ExecutionPolicy Bypass -File drivers/audiorouter-virtual/tests/build-tests.ps1`.
  New cases call production validity/admission helpers: missing commit,
  exact frame accounting in the modeled reader, unchanged clock arithmetic,
  late/ahead/duplicate admission, one/two physical slots, overwritten slot
  before progress, 32-bit wire rollover, PAUSE retention, long stalls,
  STOP/reallocation reset and legacy mode. This does not execute kernel
  `ReadBytes`; sample substitution and counter accumulation are modeled.
- `tests/acceptance/m03-driver-build.ps1 -Platform x64`: **passed**, including
  WDK compile, INF/catalog qualification, existing lifecycle refusal tests and
  the new reader/commit source guards. Log:
  `target/render-commit-x64-final-acceptance.log`.
- The same command with `-Platform ARM64`: **passed**, log
  `target/render-commit-arm64-acceptance.log`. Cross-target compile/catalog
  evidence only; no ARM64 driver loaded.
- First default HostX86 x64 WDK attempt compiled the edited source but failed
  at linking with `LNK1101: incorrect MSPDB140.DLL version`. Preserved log:
  `target/render-commit-x64-review/build.log`. The documented process-local
  Hostx64 tool selection resolved it: `PreferredToolArchitecture=x64` and the
  matching MSVC Hostx64/x64 folder first on PATH, for those build processes
  only. No Visual Studio repair or persistent environment change performed.
- `node tools/docs/validate.mjs`: **passed**, 142 Markdown files and 785 local
  links. `git diff --check`: passed.
- Jev is disabled by the earlier explicit user decision recorded in AGENTS.md.
  No Rust/UI files changed; their format/lint/Clippy checks were not rerun.

## Remaining gates and next action

Host-only validation and diff inspection are complete. Repair committed and
pushed to main as `28b989f5d8d16f20e0d1996a335c01bd7203c342`.
No guest driver was changed or loaded during this work. Full
kernel counter/waveform integration, native packet-mode activation, sustained
continuity, latency, count-1 operation and EOS remain unqualified. The saved
long scheduling losses and downstream HDA scratching remain open.

Rollback: revert the isolated source commit; preserve the existing installed
driver, clean VM snapshot, private evidence and prior bundles. A future guest
candidate must be recoverable through that clean snapshot.

## Prepared candidate and exact next guest step

Host bundle: `C:\VMs\ar-share\repair-20261010-render-commits`.
Built from the clean repair commit above, `builtAt=2026-10-10T17:08:27.6787581Z`,
test-signed x64. `tools/vm/prepare-vm-share.ps1 -Share` with that absolute path
completed using the process-local Hostx64 tool selection. Package integrity
suite passed **33 checks**, including tamper rejection, and an independent
manifest traversal verified **all 32 bundled files**. Logs:
`target/render-commit-candidate-build.log`; tamper evidence:
`target/driver-package-test-755ef67f063b440dbf73c40ef3b3c78c`.

- Signed SYS SHA-256:
  `91BBC17ADCC55482D230B932A8198A566E5C3AFC5398588C810B8CC9135D7BB6`.
- Manifest SHA-256:
  `69FD263F8005A555587BB477FE913F78D3A595EDCA77D9B58A0B88F83DFD8F67`.

No host trust, security, audio, WSL, installed driver or VM configuration was
changed. Fuzzer and native tools were built, not run. No VM process was present
at the final read-only check. The package is a candidate, not a stable release.

Next: restore the existing `03-test-signing-ready-20261007` checkpoint, boot
AR-DriverTest and open Administrator PowerShell **inside the guest**. Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'Z:\repair-20261010-render-commits\retry-smoke.ps1'
```

This existing runner copies/verifies the candidate into `C:\ar`, runs preflight
and one smoke, cleans up the driver and collects/copies evidence to the host
share. Send the output. Do not start Media Player, Listen, tone, stall, fuzzer
or a longer run during this installation check. A second clean smoke and the
focused direct waveform check remain subsequent steps after evidence review.

## First clean guest smoke — 2026-10-10 (passed)

Environment: AR-DriverTest restored from `03-test-signing-ready-20261007`,
Administrator PowerShell in the guest, no audio tools running. Command as above.
Guest clock run `20261010-102108` (preflight) to `20261010-102200` (collect).

- Runner verified 32 copied files; package identity `28b989f5…`, `dirty=false`,
  `builtAt=2026-10-10T17:08:27.6787581Z`, test-signed x64 Release.
- Preflight: 15/15 passed. Smoke: A1 baseline, A2 install, A3 endpoints and
  A14 uninstall + baseline passed. Collect: 2/2 passed.
- Install: `oem5.inf`, `ROOT\MEDIA\0000` started, no restart required, no
  default role changed. Four endpoints, all `OK`: Cable A/B Input (render,
  `{0.0.0…}`) and Cable A/B Output (capture, `{0.0.1…}`).
- Removal: `oem5.inf` removed, no restart required; before/after baseline
  JSON files are byte-identical.
- Archive on host: `C:\VMs\ar-share\evidence-20261010-102200.zip`, SHA-256
  `3CCD1A1A97151DFF39B6FE6FB9CFA0DCEAA2AA436077F5B5739670CC991EC82F`;
  extracted to ignored `target/smoke1-20261010-102200`. It also contains
  three 2026-10-07 preflight folders carried in the checkpoint's `C:\ar\evidence`;
  they are not part of this run.

Scope: installation, enumeration and cleanup only. No audio was streamed, so
packet-mode activation, waveform continuity and counters remain unqualified.
Next: the second independent clean smoke from the same checkpoint and command.

## Second clean guest smoke — 2026-10-10 (passed)

Same checkpoint restore, command and package. Guest clock `20261010-102723`
(preflight) to `20261010-102855` (collect). Preflight 15/15; A1/A2/A3/A14
passed; collect 2/2. Install `oem5.inf` on `ROOT\MEDIA\0000`, no restart,
no default change, four endpoints `OK`; removal ok without restart.

Independence: the archive contains no folder from the first run, and its
`before-baseline.json` is byte-identical to the first run's, so the checkpoint
was really restored. Its before/after baselines are also byte-identical.

Archive: `C:\VMs\ar-share\evidence-20261010-102855.zip`, SHA-256
`CC96B2122B2F8D3E3006AF009F9FB48B44322C8ECE9F515BD73408A419C36CB4`;
extracted to `target/smoke2-20261010-102855`.

Both required clean smokes pass for this candidate. Scope unchanged:
installation/cleanup only. Next: install plus status/format inventory in the
same guest session (it was left at the restored baseline), then collect.

## Install and status — 2026-10-10 (passed)

Same guest session, `C:\ar\vm-checks.ps1 -Step install`, `-Step status`,
`-Step collect`. Install 3/3 (exit 0, four endpoints, second install is an
idempotent no-op); status 2/2: helper `installed`, protocol 1.1, driver
0.1.0.0. All four active endpoints (Cable A/B Input render, Cable A/B Output
capture) support 60/60 formats, engine periods min/default/max 128/480/480
frames, mix format float32 stereo 48 kHz. Archive
`C:\VMs\ar-share\evidence-20261010-103119.zip`, SHA-256
`DDAFA3169E392AF7B75FC3CB926C30D635014C333B26C76E9BF4FA84D3CA851D`, extracted
to `target/install-20261010-103119`. The driver remains installed in the guest.
Not audio evidence.

## Direct audio r3 bundle — 2026-10-10

The existing preparer required a paired-tone base; the standard candidate
lacks the definitions-only `paired-trace-support.ps1` the runner loads.
Commit `792c5a69` adds that file when absent and records the base driver
identity in the manifest; a paired-tone base keeps its own copy.

- Recorder: `build-direct-audio.ps1` (static CRT); cargo found it current with
  the HEAD sources (last direct-audio source change `72766a66`). Import check
  shows no Visual C++ runtime DLL.
- `tests/acceptance/m03-direct-audio.ps1`: 117 orchestration checks passed;
  no audio stream or driver tool opened. Log `target/direct-audio-r3-acceptance.log`.
- Bundle `C:\VMs\ar-share\diagnostics-20261010-direct-audio-r3`: source
  `792c5a69`, driver `28b989f5` (SYS SHA-256 unchanged, `91BBC17A…7BB6`),
  35 manifest entries = the 32 base files plus `paired-trace-support.ps1`,
  `run-direct-audio.ps1`, `tools\m03_direct_audio.exe`. Manifest SHA-256
  `66D216264573EFE05D3368FE5F42A7DAB694754BED3A67151A08379B1BC67251`.

Guest step (driver already installed; Media Player and Cable B Listen off),
in Administrator PowerShell **inside the VM**:

```powershell
& {
    robocopy.exe 'Z:\diagnostics-20261010-direct-audio-r3' 'C:\ar\diagnostics-20261010-direct-audio-r3' /E /R:1 /W:1 /XF direct-*.zip
    if ($LASTEXITCODE -ge 8) { throw 'Copy failed. Stop here.' }
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\ar\diagnostics-20261010-direct-audio-r3\run-direct-audio.ps1'
    if ($LASTEXITCODE -ne 0) { throw 'Diagnostic failed. Evidence was preserved; send the output.' }
}
```

Limit (superseded by the run below): no driver counter shows whether Windows
actually calls `SetWritePacket` on these streams. A clean result shows the candidate's direct samples and counters;
a new render underrun count is a failed continuity gate to analyze, not to
suppress. Packet-mode observability would need a separate reviewed kernel change.

## Direct audio r3 result — 2026-10-10 (failed; regression analysis)

Run `direct-fb4b29850e184849af6247edb3948ffa`, guest tone folder
`20261010-103654-tone`. Archive
`C:\VMs\ar-share\diagnostics-20261010-direct-audio-r3\direct-fb4b29850e184849af6247edb3948ffa.zip`,
SHA-256 `27149CE0B72C8B8CD96A3C94C7626D51BCDD85D727D0DFC30E3409E7FD114406`,
extracted to `target/direct-r3-fb4b2985`. Startup and recorder exit 0; tone
exit 1 (error counters), both signal analyses exit 1, 166 in-signal Cable B
discontinuity/timestamp packets.

Final counters: Cable B capture underrun 126,528; Cable A render underrun
193,968 and overrun 136,368; sequence gaps, non-finite and format 0. Maximum
pump gap ~460 ms; Cable A WAV 1,303,200 frames (27.15 s of 30 s).

Per-second progress shows two separate patterns:

1. **Shared user-mode stalls** (90–460 ms, both workers at once) coincide with
   capture underrun and render overrun (bridge queue drops of whole quanta).
   This is investigation B and is much worse than in r2 (one 288 ms pause).
2. **Render underrun without stalls.** At 20–23 s worker gaps were ~20–30 ms,
   capture underrun ~0 and render overrun ~0, yet render underrun grew
   11,500–15,000 frames/s (~25–30 %). In `ReadBytes` the only render
   `UnderrunFrames` increment is the new `!committed` branch, so all 193,968
   frames are the new packet-validity rule substituting silence. This also
   proves packet mode latched (at least one `SetWritePacket` succeeded).

Waveform comparison with the same, current offline analyzer
(`m03_direct_audio analyze`, file-only):

| Run | Driver | Cable A 1-s windows clean | 10-ms blocks clean |
| --- | --- | --- | --- |
| r2 | before `28b989f5` | 28 of 30 (residual 0.00) | 2,965 of 2,973 |
| r3 | `28b989f5` | 0 of 28 (residual ~0.14) | 1,583 of 2,715 |

r3 Cable A has 446 exact-zero runs (201,968 frames ≈ the counter): 197 of
480 frames (a whole packet), others mostly 144/192 frames; the tone phase is
continuous across 347 of them, i.e. silence replaced audio in place on the
source timeline. 190 additional one-packet (480-frame) phase jumps without
zeros match bridge overrun drops from the stalls. Scratch analysis code:
session scratchpad `wav.cs` (block least-squares fit, zero runs, phase breaks).

Interpretation (proven vs. not):

- Proven: the repair's silence substitution is active and, in this run,
  removed ~15 % of Cable A audio, including in seconds without user-mode
  stalls. The previous driver delivered clean Cable A in r2.
- Not proven: whether those frames held valid data. The driver does not
  record `SetWritePacket` outcomes (accepted/late/overrun) or lateness, and
  r3's guest was far more stalled than r2's, so OS-side lateness may be real.
- Policy finding: Microsoft documents `STATUS_DATA_LATE_ERROR` as "the driver
  may optionally use some of the data from the packet". The repair instead
  discards a late packet entirely (and `InvalidateSlot` runs before
  admission, so a late or overrun submission also retires the tag of the
  packet currently transferring when its slot matches). A late write of the
  transferring packet thus becomes a full counted packet of silence where the
  previous driver played the freshly written remainder.

Not repeated: the user's run is not retried unchanged. Proposed next step
(needs a reviewed kernel change and one new candidate): record `SetWritePacket`
outcomes and lateness in reserved shared-header space, and accept a late write
of the currently transferring packet for its not-yet-consumed frames while
still returning `STATUS_DATA_LATE_ERROR` and counting the already-consumed
frames. The user approved this (2026-10-10) and asked for maximal host-side
testing before the next VM run; see the next section.

## Slot-provenance repair and host evidence — 2026-10-10

Microsoft contract re-read for this change:
[SetWritePacket](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nf-portcls-iminiportwavertoutputstream-setwritepacket)
(the OS has written the packet before the call; late packets may be partly
used; overrun data may be ignored; packet counter and notifications continue
at real-time rate),
[GetPacketCount](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nf-portcls-iminiportwavertoutputstream-getpacketcount)
(count 5 means packet 5 transfers and the OS writes 6; the OS resynchronizes
from it after a dataflow error; reset at STOP) and
[GetReadPacket](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nf-portcls-iminiportwavertinputstream-getreadpacket)
(capture: drop oldest data on overflow; the existing implementation returns
the last completed packet, which matches). Admission/return codes match the
sysvad-derived arithmetic and are unchanged.

Change (owning layer: `streamtiming.h`, `minwavertstream.cpp`, `bridgeio.h`,
`adapter.cpp`; Rust `crates/windows-audio`; tone tool and VM scripts):

- `AudioRouterRenderCommits` now records per **physical slot** the absolute
  identity of the last packet the OS wrote there (`RecordWrite`), called in
  `SetWritePacket` before `UpdatePosition`, under the position lock, for every
  call (return code independent). `Contains` requires the slot's identity to
  equal the logical packet being read. Late writes of the transferring packet
  play their unconsumed bytes; never-rewritten slots never replay; overwritten
  current slots (overrun, one-slot mode) silence their remainder. EOS payloads
  never play.
- Packet outcome counters (accepted/late/overrun) in shared-header bytes
  104–127, capability `PACKET_COUNTERS` 0x40; Rust `packet_counters()`; the
  tone tool prints `render-source packet writes (...)` and adds
  `render_packets=` to each progress line; `vm-checks.ps1` and
  `run-direct-audio.ps1` echo it. Error-counter parsing is unchanged.
- Spec 17 §5.2/§5.4 updated.

Host checks (Windows host, no driver load, no audio endpoint):

| Check | Result / evidence |
| --- | --- |
| MSVC offline bridge/helper suite with timing model | 738 passed; `target/driver-unit-provenance-tests.log` |
| x64 WDK/catalog/source acceptance (Hostx64 tools) | Passed; `target/provenance-x64-acceptance.log` |
| ARM64 WDK/catalog/source acceptance | Passed (compile-only); `target/provenance-arm64-acceptance.log` |
| `cargo fmt --check` (workspace, src-tauri) | Clean |
| Clippy workspace and src-tauri, `-D warnings` | Clean; `target/provenance-clippy*.log` |
| `cargo test -p audiorouter-windows-audio` | 122 passed (header offsets 104/112/120, capability 0x40 pinned) |
| Direct-audio orchestration acceptance | 117 passed; `target/direct-audio-provenance-acceptance.log` |
| VM script guards | 275 passed; `target/vm-guards-provenance.log` |

Offline render timing model (`renderTimingModelChecks`, production helpers,
1 ms timer, 2 × 480-frame packets, 30 s; three OS reactions: blind write with
immediate or next-wake resync, and count-first). Frames silenced although the
OS had already written them / stale frames played:

| Scenario (blind write, resync on next wake) | before repair | `28b989f5` | provenance |
| --- | --- | --- | --- |
| on time | 0 / 0 | 0 / 0 | 0 / 0 |
| late mid-packet (13.5 ms, every 5th) | 0 / 0 | **288,000** / 0 | 0 / 0 |
| jitter 0–25 ms | 0 / 455,975 | **451,876** / 0 | 0 / 0 |
| OS skips every 9th packet | 0 / 159,840 | 0 / 0 | 0 / 0 |

Across all 18 scenario × OS-model combinations the provenance rule plays no
stale frame and silences no written frame (asserted); the reference rules
reproduce both defects (asserted). This is a model of documented behavior,
not kernel timing or VM evidence; the actual OS reaction is what the new
packet counters will show.

Fresh-context kernel review (WP-04, separate read-only agent, 2026-10-10): no
blocking defect; identity resolution, `Contains` for one/two slots, ordering,
IRQL/lock path (`RecordBridgeActivity` under the position lock: QPC read and
rundown-protected interlocked adds only) and ABI were confirmed. Findings and
disposition:

1. Medium, residual: the driver cannot see the OS write before its call; a
   timer tick in between judges the slot by its previous record. Documented in
   17 §5.4. The model now separates write and call (`writeLeadUs`) and asserts
   that misjudged frames never exceed the frames consumed inside the gap. With
   an exaggerated 2 ms gap (blind write, resync on next wake): late mid-packet
   57,600 frames silenced (96 per late packet), late at boundary 20,544,
   jitter 90,426 silenced and 22,862 stale (overwrites of the transferring
   slot); on time and skipped packets remain exact. A real OS writes and calls
   on one thread, so its gap is far shorter.
2. Low: zero-gap provenance assertions are partly self-consistent; the test
   comment now says so, and the regression references plus gap bound carry
   the evidence.
3. Low: two `§` characters in the test were mis-encoded by a PowerShell 5
   round trip; restored (no other occurrence in the repository).
4. Low: packet counters cover calls while the stream publishes to the lease;
   documented in 17 §5.4 (never counted against another lease).
5. Low: one-slot caveat restored in 17 §5.4.
6. Nit: `RecordWrite` now rejects identities beyond the `MAXLONGLONG`-bounded
   count instead of overflowing; regression added.
Optional guard adopted: `RecordBridgeActivity` must follow admission and
precede the lock release in `SetWritePacket`.

After these fixes: 738 offline checks, x64 and ARM64 WDK acceptance passed
again (`target/provenance-*-acceptance.log`).

## Slot-provenance candidate and guest steps — 2026-10-10

Built from clean commit `492d8ca82cac177d792fe0561003a8e157c4fab5`
(`builtAt=2026-10-10T18:07:59.3587015Z`, test-signed x64 Release) with the
process-local Hostx64 tools; log `target/slot-provenance-candidate-build.log`.
Package checks 33 passed; all 32 manifest entries re-verified independently.

- Base: `C:\VMs\ar-share\repair-20261010-slot-provenance`; SYS SHA-256
  `10CF8E879DE85E3A924CDCC0DBA2987D00CB8328DAB9CBA870B997F3EF952381`;
  manifest SHA-256
  `97D9CD1DA8354C145E9623D68F938AE0D2EA53784FFBBA469F38835E86B128D6`.
  The packaged tone tool contains the packet-writes report.
- Direct bundle: `C:\VMs\ar-share\diagnostics-20261010-direct-audio-r4`, source
  and driver `492d8ca8`, 35 entries, manifest SHA-256
  `F0BBF409914901FAF149AC23B05C552F5D954A8D1EF5861960CB1277BA1F9236`.

Guest sequence (VirtualBox controls on the host; commands in Administrator
PowerShell inside AR-DriverTest; Media Player and Cable B Listen off):

1. Restore `03-test-signing-ready-20261007`, boot, run the first clean smoke:
   `powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'Z:\repair-20261010-slot-provenance\retry-smoke.ps1'`.
   Continue only if it ends with `Smoke passed`.
2. Power off without saving state, restore the same checkpoint, boot, and run
   one combined block: second clean smoke, install, status, collect, copy the
   r4 bundle and the 30-second direct audio test. Each step stops the block on
   failure. The exact block is in the user message of 2026-10-10 and in the
   direct audio runbook.

Expected new output: `render-source packet writes (cable-a): NativeBridgePacketCounters { accepted, late, overrun }`
next to the counters. Interpretation: late or overrun counts show Windows'
submission timing; render underrun should now be only frames consumed before
a late write. Gates are unchanged: zero error counters and clean waveforms.
