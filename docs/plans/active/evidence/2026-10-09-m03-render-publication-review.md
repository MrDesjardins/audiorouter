# M03 render publication review and repair — 2026-10-09

Scope: VCAB-20/24/25/27/28, VDEV-12, SEC-08; Stage A tone continuity.
User requested a thorough source repair before another VM test and confirmed
that playback was already active, with the Cable A Input meter moving.

## Evidence and findings

Read `C:\VMs\ar-share\evidence-20261008-221952.zip`, entry
`20261008-221935-tone/tone.txt`. Archive SHA-256:
`BA8A096043E41D8CEB56472886403FD788BC711065C81BB18DCA2726DF5DD074`.
The armed reader first observed sequence 2 at 18 ms, before recording any
frames, with a maximum poll gap of 2,564 microseconds. Render lost 480 frames;
capture counters were zero. The previously passing 30-second trace does not
qualify sustained reliability. See the [run history](2026-10-08-m03-render-overrun.md).

Confirmed source defects in the tree through `22046b39`:

1. `ReadBytes` can complete several quanta during one position-locked callback.
   The publisher overwrote the same shared slot for each quantum. No reader
   polling speed guarantees execution between those writes. This is sufficient
   to produce the observed first-read sequence gap, although the archive does
   not trace the exact kernel schedule of that particular failure.
2. Initial OPEN made `MappedView` visible before setting its extent and
   resetting `NextSequence`. A callback could publish while initialization was
   still in progress. Replacement OPEN already published the pointer last.
3. Shape lookup and publication were separate rundown sections without an
   expected-generation check in the publisher. Lease turnover could put old
   scratch samples into a new lease with the same dimensions.
4. Timer and position guards compared raw QPC ticks to timestamps already
   converted to 100 ns. They were only comparable when the frequencies happened
   to match; no archive evidence establishes this as the VM failure's cause.
5. Render catch-up could traverse more than one DMA ring lap, repeating bytes
   whose original samples were already overwritten. The stream destructor
   released callback dependencies before joining the notification timer.

The prior harness regression waited for a reader acknowledgement before its
second publication. It therefore could not reproduce finding 1. Pre-arming
the reader and immediately repolling after a read did not repair the driver.
Pausing playback before opening a lease is withdrawn as a workaround.

## Implementation and compatibility

- Preserve the unread shared render slot until its exact sequence is
  acknowledged; future or stale user acknowledgements cannot authorize a
  replacement. No user value controls an address, extent or queue index.
- Retain completed blocks in a fixed FIFO using the existing render stream's
  unused prefetch array: at most four blocks, fewer if the negotiated shape
  fills the array. At maximum shape there is one private slot. This adds
  24 bytes of metadata per stream, no sample allocation in any callback.
- Full FIFO replaces its oldest private block and counts its dropped frames.
  A merely busy shared slot does not count as a loss. Publication sequences
  remain contiguous after private loss; the unchanged zero-overrun gate is
  therefore mandatory as well as sequence checks.
- Service both directions each 1 ms tick, independently of packet notification
  cadence. Queue drains also run when no new DMA frames are due. STOP, invalid
  format and lease turnover invalidate pending data. Publication and counter
  updates validate the expected generation under rundown protection.
- Pause drains completed blocks without advancing DMA. EoS pads only the final
  partial block with zeros, waits for private tail delivery before signaling
  final completion, signals it once, and stops its timer. A reader that stops
  acknowledging cannot cause unbounded allocation; STOP/close retire the tail.
- Publish initial OPEN's pointer last. Join the timer before freeing stream
  dependencies or unmapping the audio buffer. Bound render catch-up copies to
  one surviving DMA lap and count discarded frames; logical time still advances.
- Keep protocol 1.1, header layout, IOCTLs and the existing client unchanged.
  No migration or driver installation was performed. Update spec 17 §5.2/5.4
  and the testing guide to describe the repaired flow control and valid
  already-playing startup case.

## Review and host verification

Environment: Windows host, VS 2026/MSVC tool directory 14.51.36231,
SDK/WDK 10.0.28000.0. All execution below is host-safe build/unit work; the
test driver was never installed or loaded on the host or in the VM.

- Fresh-context kernel review required by WP-04 identified OPEN ordering,
  generation, tail-delivery, bounded catch-up and teardown risks. Its second
  pass found premature/repeated EoS completion in the initial FIFO change;
  that was corrected before handoff. Final source review found no remaining
  source blockers. Review is not runtime timing evidence.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File drivers/audiorouter-virtual/tests/build-tests.ps1`:
  **447 checks passed**. Production FIFO/ack helpers are exercised with two
  completed blocks and no intervening reader execution, exact samples across
  128/480/4096 frames × 1/2/8 channels, delayed/future acknowledgements, queue
  overflow, generation reset and EoS tail padding. Log:
  `target/driver-unit-release/tests.log`. The adapter's kernel rundown and
  WaveRT lifecycle are covered by source guards/review, not executed by this
  user-mode test harness.
- `tests/acceptance/m03-driver-build.ps1 -Platform x64`: **passed** WDK build,
  INF/catalog qualification and source guards. Log:
  `target/render-retention-acceptance.log`. The default HostX86 linker first
  failed with `LNK1101` (debug-symbol DLL mismatch); the already-documented
  process-local `PreferredToolArchitecture=x64` and matching Hostx64/x64 tools
  first on PATH resolved it. No VS repair or machine environment edit.
  An intermediate source guard rejected partial-tail zero padding as though
  it cleared the whole scratch array; narrowed the guard and added the
  production tail-padding regression before the passing run.
- Both workspace and `src-tauri` `cargo fmt` commands and their `-- --check`
  variants: **passed**. Only a Rust acknowledgement comment changed.
- Workspace and `src-tauri` Clippy, `--all-targets --all-features -- -D warnings`:
  **passed**. Cargo emitted filesystem hard-link-cache fallback warnings;
  no Clippy findings.
- `tests/acceptance/m03-driver-build.ps1 -Platform ARM64`: **passed** with
  the same process-local host tool selection. Log:
  `target/render-retention-arm64-acceptance.log`. Compile-only cross-target
  evidence, not ARM64 runtime evidence.
- `tests/acceptance/docs.ps1`: **passed**, 136 Markdown files and 738 local
  links. `git diff --check`: **passed**.
- AddressSanitizer attempt: **not run successfully**; linker cannot find
  `clang_rt.asan_static_runtime_thunk-x86_64.lib`. Log:
  `target/driver-unit-asan/compile.log`. No compiler installation was changed.
- Requested Jev command: **no review result**; API request failed with
  `Could not reach the Jev API: fetch failed`. Not counted as a pass.

## Remaining gates and rollback

No new VM test was requested or run during this repair. No candidate was
copied over the installed VM package. Runtime continuity, end-of-playback,
lease turnover, latency, Verifier and long-run memory remain pending.
The separate 65 ms scheduling-gap capture underrun from the earlier
600-second run is not resolved by this render repair. Bounded buffering
cannot establish losslessness through arbitrary scheduling stalls.

Next task: prepare one identified candidate from the committed source;
subsequent VM qualification must cover already-running playback,
both directions and the original long-run case without relaxing counters.
Keep the existing guest and evidence until that candidate is ready. Rollback
is reverting this source change and restoring the prior VM snapshot/package;
it does not require any host security or driver change.
