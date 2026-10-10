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
