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
- `node tools/docs/validate.mjs`: **passed**, 142 Markdown files and 783 local
  links before the final evidence links were added. `git diff --check`: passed.
- Jev is disabled by the earlier explicit user decision recorded in AGENTS.md.
  No Rust/UI files changed; their format/lint/Clippy checks were not rerun.

## Remaining gates and next action

Host-only validation and diff inspection are complete. Commit the isolated
repair and prepare one checksummed candidate from its clean source commit.
No guest driver was changed or loaded during this work. Full
kernel counter/waveform integration, native packet-mode activation, sustained
continuity, latency, count-1 operation and EOS remain unqualified. The saved
long scheduling losses and downstream HDA scratching remain open.

Rollback: revert the isolated source commit; preserve the existing installed
driver, clean VM snapshot, private evidence and prior bundles. A future guest
candidate must be recoverable through that clean snapshot.
