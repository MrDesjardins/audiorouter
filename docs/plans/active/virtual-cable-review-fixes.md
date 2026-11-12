# Active plan — virtual cable driver review fixes (2026-11-10, owner request)

Objective: act on the 2026-11-10 in-depth review of
`drivers/audiorouter-virtual` so the cable is as reliable and fast as the
design allows before the EV certificate is bought (WP-13).

Requirement IDs: VCAB-24, VCAB-25, VCAB-27, VCAB-28, VCAB-30, SEC-08,
VDEV-01/03/12, NFR-02.

## Decision record (owner, 2026-11-10)

The owner asked to "create a plan and fix everything" after the review.
**This ends the RC3 freeze** (`d2560715`). Consequences, accepted by that
request:

- The RC3 hashes in [virtual-cable.md](virtual-cable.md) no longer describe
  the next package; the next driver is a new RC (0.2.5) and needs Stage A
  again from `02-test-signing-ready`.
- The pre-purchase task 4 (final r11 bundle) must run on the new RC, not RC3.
- Host builds and host unit tests are **not** VM evidence. Nothing here is
  qualified until the VM run passes.

## Findings and decisions

| # | Finding | Decision | Tier |
| --- | --- | --- | --- |
| 1 | IRPs PortCls handles (READ, WRITE, FLUSH, security, …) reach the bridge control device, which has no PortCls extension | Route **every** major function through one wrapper that fails requests for the control device and forwards the rest to PortCls | 1 |
| 2 | `BridgeControlGuard` takes a push lock with normal kernel APCs enabled | Enter a critical region around the push lock | 1 |
| 3 | Control device created in `DriverEntry` keeps the driver loaded; VM record: update reboot "because Driver did not unload" | Create at `StartDevice`, delete at stop/remove after releasing every lease; unload deletes anything left | 1 |
| 4 | An expired owner's heartbeat destroys its own lease even with no rival; interrupt time counts sleep | Owner may renew an expired lease if nobody reclaimed it; expiry uses unbiased interrupt time | 1 |
| 5 | Render queue depth 4 and capture depth 2 are compiled in | Make the depths larger, bounded by the existing storage | 3 |
| 6 | Position advances in whole milliseconds | Advance from elapsed 100 ns units with a sub-frame carry | 2 |
| 7 | 1 ms periodic timer per running stream | Kept periodic: see "Not changed" | 2 |
| 8 | Render audio waits for the simulated DMA sweep | Not changed: see below | 3 |
| 9 | Capture latency not regulated, no fill level exported | Not changed in the kernel; see below | 3 |
| 10 | Single-slot transport | Not changed: protocol redesign, see below | 3 |
| S1 | Per-sample format GUID comparisons in the DPC | Resolve the sample encoding once per stream | 1 |
| S2 | Redundant finite scan when publishing render blocks | Removed: the only producer writes finite samples | 1 |
| S3 | `m_pDmaBuffer` cleared outside the position lock | Clear the DMA view and size under the lock | 1 |
| S4 | Unused `m_pDpc` allocation; dead EOS paths | Remove `m_pDpc`; EOS paths kept (harmless, spec task) | 1 |
| S5 | Run→Pause `KeFlushQueuedDpcs` | Kept: it guards a running timer callback | — |

### Not changed in this round, with reasons

- **7 (one-shot timer).** The 1 ms tick also drives the capture prefetch and
  render publication that the user-mode bridge depends on (§5.4 flow
  control). Packet-aligned one-shot timing would lengthen how long a
  published capture block waits. Needs a timing-model study first.
- **8 (publish at `SetWritePacket`).** Replaces the validated provenance
  model (`renderTimingModelChecks`) that fixed the r3 15 % silencing defect.
  Too risky without a new model and VM run; recorded in
  `docs/plans/future/` scope only if the owner asks.
- **9 and 10 (ring transport, fill level).** A protocol revision (new
  shared layout, Rust encoder, ABI literals, spec 17 §5). Its own milestone.
- **5** is done within the current ABI: depths grow inside the existing
  fixed storage; no new OPEN field.

## Ordered tasks

1. Tier 1 code: items 1–4, S1–S4. Rust client: no change needed for item 4
   (an owner's late heartbeat now succeeds instead of `LeaseNotActive`).
2. Spec 17 §5: owner renewal after expiry; unbiased time.
3. Tier 2: item 6, with new host checks in `tests/bridge_tests.cpp`
   (44.1 kHz frame alignment, no drift over 1 h of 1 ms ticks, sub-ms calls).
4. Tier 3: item 5 (queue depth), host test update.
5. Host checks: `tests/build-tests.ps1` (and `-SanitizeAddress`).
6. WDK build of the driver (owner's PC; ask first if a call is running).
7. VM Stage A on the new RC (owner): `run-stage-a.ps1 -Session all`, plus
   a `ReadFile`/`WriteFile`/`FlushFileBuffers` probe on the control device
   (expect errors, no bugcheck), a driver update **without reboot**, and a
   sleep/resume with a live lease.

## Validation matrix

| Change | Host evidence | VM evidence (required) |
| --- | --- | --- |
| 1 | compile | probe IRPs on `\\.\AudioRouterVirtualBridge` |
| 2 | compile | Driver Verifier session |
| 3 | compile | `update` without "Driver did not unload" |
| 4 | lease-renewal unit test | freeze > lease, route resumes without reopen |
| 6 | `positionClockChecks` | A5/A15 counters, latency p50 |
| 5 | queue-depth unit test | A16 96 kHz stall headroom |

## Risks

- Moving the control device to `StartDevice`: a client holding a handle
  across device removal gets `STATUS_DELETE_PENDING`-style errors; leases are
  released first so no mapped view outlives the device.
- Owner renewal after expiry widens the window a dead owner holds a cable
  only until a rival OPEN arrives; rivals still win an expired lease.
- Timing change (6) is covered by host models only until the VM run.

## Rollback

Every change is on branch `fix/virtual-cable-review`. RC3 remains the
reference: the share `C:\VMs\ar-share\rc-0.2.4-base` is untouched. Reverting
the branch restores the frozen source.

## Progress

### 2026-11-10 (agent, host only, branch `fix/virtual-cable-review` from `main` `feaf2108`)

Done in source (not compiled by the WDK yet):

- 1: `DriverEntry` saves PortCls's dispatch table, installs `BridgeControlOther`
  for every major function (forwards to the saved entry for PortCls
  devices, fails the control device); CREATE/CLEANUP/CLOSE/DEVICE_CONTROL keep
  their handlers; `PnpHandler` still installed last. The control device is
  identified by its own extension magic and `FILE_DEVICE_UNKNOWN`, not by the
  global pointer, so a deleted control device's late CLEANUP/CLOSE never goes
  to PortCls.
- 2: `BridgeControlGuard` enters a critical region.
- 3: `CreateBridgeControlDevice` runs at the end of `StartDevice`;
  `DeleteBridgeControlDevice` runs on STOP/REMOVE/SURPRISE_REMOVAL before the
  adapter cleanup, releases every lease first, and blocks later OPENs from
  handles that outlive it (`g_BridgeAcceptingOpens`). `DriverUnload` still
  calls it for anything left.
- 4: `AudioRouterMaintenanceDecision` (bridgeio.h) replaces the inline
  conditions; the owner renews an expired, unclaimed lease. Expiry uses
  `KeQueryUnbiasedInterruptTime`. Rust client unchanged: a renewal returns
  success where it used to return `LeaseNotActive`, and its reconnect path is
  still used for real loss.
- 5: render queue depth 4 → 8 (`AR_RENDER_QUEUE_MAX_BLOCKS`). Capture depth
  kept at 2: the capture producer publishes only after the driver's
  acknowledgement, so a deeper driver queue would only add standing latency.
- 6: `AudioRouterAdvanceFrames` (streamtiming.h) drives `UpdatePosition`
  from exact elapsed 100 ns with a fractional-frame remainder
  (`m_ulFrameRemainder` replaces the byte carry). `m_hnsElapsedTimeCarryForward`
  now holds the sub-frame remainder in hns for GetReadPacket timestamps.
- S1: `m_SampleEncoding` resolved at `Init`; Read/WriteBridgeSample switch on it.
- S2: publish-side finite scan removed (driver-private source is finite).
- S3: `DetachDmaBuffer` clears the view, size and notification count under
  the position lock before unmap in both free paths.
- S4: unused `m_pDpc` and `m_pTimer` removed.
- Spec 17 §5 item 2 (expiry), item 5 (control device lifetime/dispatch), and
  the lost-connection paragraph updated.

Host checks run:

- `drivers/audiorouter-virtual/tests/build-tests.ps1`: **pass**, 10,915,240
  checks, new `maintenanceDecisionChecks`, `sampleEncodingChecks`,
  `frameClockChecks` (1 h of 1 ms ticks drift-free at 44.1/48/96 kHz;
  irregular sub-ms calls equal one call), queue depth check. The render
  timing model table is unchanged from RC3.
- `build-tests.ps1 -SanitizeAddress`: **not run**. Link fails with
  `LNK1104 clang_rt.asan_static_runtime_thunk-x86_64.lib` (the VS install
  lacks the ASan component); environment, not code.

- WDK build (owner approved during a call, 2026-11-10):
  `build.ps1 -Version 0.2.5 -Output target\driver-0.2.5-review`: **pass**,
  0 warnings (/W4 /WX), InfVerif/signability "Errors: None, Warnings: None".
  SHA-256 prefixes: SYS `2AE9F7F842083E9E`, INF `06391F9DF91E1A1C`,
  CAT `3FD2C66BAD94F4A0` (unsigned build output, not a release candidate).
- WDK Code Analysis (`msbuild /p:RunCodeAnalysis=true`, Release x64): first
  run found C28168/C28252/C28253 (dispatch annotations, from this change),
  C6101 (`WriteBridgeSample` annotation) and C6387 (sample stream-resource
  probe). All fixed; the probe code was removed with the RuntimePower demo,
  and `InstallEndpointRenderFilters` now returns the real install status
  instead of the last probe's status. **Remaining: C26495 x4** (members not
  set in sample constructors in common.cpp, minwavert.h, minwavertstream.h,
  savedata.cpp). Kept: `operator new` allocates with `ExAllocatePool2`
  without `POOL_FLAG_UNINITIALIZED`, so every such object starts zeroed.
- Additional change from the review of this work: bridge creation failure at
  `StartDevice` is logged and **not fatal**, so a name still held by a
  delete-pending control device can never take the cables down.

Not run: any VM stage, Driver Verifier, CodeQL.

### Conditions and caveats found while implementing

- **Item 6 is the first revert candidate.** `renderTimingModelChecks` runs
  its own clock, not `UpdatePosition`, so its unchanged table says nothing
  about item 6. The old clock lagged real time by up to 1 ms; the new one by
  under a frame, so a packet completes up to 1 ms earlier and an OS write
  that landed inside the old slack can now count as late. If VM
  `PacketsLate` or silenced frames rise against r10, revert the
  `UpdatePosition` hunk and `AudioRouterAdvanceFrames` alone.
- **Item 3 "no reboot" needs the control handles closed.** A handle still
  open on the delete-pending control device keeps the image loaded. The Rust
  client drops its handle when a route closes or reconnects (workers own
  `NativeBridgeControlClient`, whose `Drop` closes it), and every request
  after the stop fails, so handles drain within the 1/2/4 s reconnect
  schedule. Run the VM update test **with AudioRouter playing a cable route**.
- Stop→start with an open handle: confirm in the VM (disable/enable the
  device while a route runs) that the bridge name is released and recreated.

### Review items not done, with reasons

| Item | Reason |
| --- | --- |
| 7 one-shot timer | The 1 ms tick also drives capture prefetch and render publication; needs a timing-model study first |
| 8 publish at `SetWritePacket` | Replaces the validated provenance model behind the r3 fix; needs a new model and a VM run |
| 9, 10 ring transport, fill level | Protocol revision (shared layout, Rust encoder, ABI literals, spec 17 §5); its own milestone |
| Capture queue depth | A deeper driver queue only adds standing latency (producer is ack-paced) |
| Per-frame divide in `Contains` | Under 1 % of a core; touches the provenance path item 8 would change |
| Scratch sized by format (~512 KB/stream) | Changes the render queue capacity arithmetic; fold into the ring redesign |
| `CSaveData` work items, INF D3 idle after 3 s | Sample behavior with no measured cost; changing the INF power keys needs its own VM check |
| Run→Pause `KeFlushQueuedDpcs` | Kept: it joins a running timer callback |

### Second review of this diff (2026-11-10, before the VM run)

| ID | Severity | Finding | Status |
| --- | --- | --- | --- |
| R1 | Medium | The driver lets the owner renew an expired lease, but the Rust client never tries: `NativeBridgeSession::heartbeat_at` (`crates/windows-audio/src/lib.rs`) returns `LeaseExpired` past its own `LeaseMs`, pinned by `native_bridge_session_expires_and_cannot_be_revived_by_a_late_heartbeat`. After a long stall or sleep the route still reconnects (1/2/4 s); the only gain today is that a rival cannot take the cable meanwhile. In the VM, sleep/resume will show a reconnect, not seamless audio. | Fixed 2026-11-11: `NativeBridgeController` calls `NativeBridgeSession::defer_lease_to_driver()` after the driver accepts OPEN; the session then keeps no local deadline and only a refused kernel HEARTBEAT ends the route (older drivers still refuse, so they still reconnect). Regression `driver_owned_session_survives_a_stall_longer_than_its_lease`. `run-cable-reconnect.ps1` now expects the route to ride the stall (no `reconnecting`, Cable B silent under 1 s, at least 1000 of 1500 impulses); `-ExpectReconnect` keeps the old verdict. Stage A extras labels renamed. VM: needs the full bundle (`prepare-cable-latency-update.ps1`, clean commit) |
| R2 (rebuilt `build.ps1 -Version 0.2.5 -Output target\driver-0.2.5-review-r2`: pass, 0 warnings; VM check: open a handle, disable/enable the device, OPEN on the old handle must fail with `STATUS_DEVICE_NOT_CONNECTED`) | Low | `g_BridgeAcceptingOpens` was one global flag, set again at restart, so a handle still open on a deleted control device could open a new lease and pin the old object and the driver image. | Fixed: the flag moved into the control device extension (`AR_BRIDGE_CONTROL_EXTENSION::AcceptingOpens`), cleared permanently at deletion; OPEN checks the flag of the handle's own device |
| R3 | Low, pre-existing | `DetachDmaBuffer` zeroes the buffer size and notification count under the position lock, but `GetReadPacket` divides by `m_ulNotificationsPerBuffer` (`minwavertstream.cpp`, `packetSize`) after releasing it. A free racing that call would divide by zero. Relies on PortCls not overlapping them, as before this change. | Fixed 2026-11-11: `GetReadPacket` snapshots size and count under the position lock and returns `STATUS_DEVICE_NOT_READY` on zero. `SetWritePacket` re-checks both after taking the lock (its earlier checks ran unlocked). `UpdatePosition` already guards a zero size. No host regression: the race is a lock-ordering property of the stream class, which the host suite does not instantiate. Code Analysis and the WDK build cover it. |
| R4 | Watch | Position now advances per frame instead of in 1 ms steps; unverified whether the host timing model covers sub-millisecond steps. Judge by Stage A late/skipped packet counts. | VM |
| R5 | Watch | The 8-block render queue delivers up to ~21 ms of held blocks after a user-mode stall instead of dropping them: expect a brief latency bump rather than a gap. | VM |

Checked correct: catch-all dispatch (non-paged, forwards the saved PortCls
routine), idempotent control-device teardown across stop / surprise removal /
removal / unload, one unbiased clock for every lease timestamp, the
maintenance decision table (identical to the old logic except owner
renew/close of its own expired lease), removal of the publish-side finite
scan, the frame clock remainder and its read-packet carry, and the
`InstallEndpointRenderFilters` return status.

## Next action

1. Owner: review the diff on `fix/virtual-cable-review` (not committed).
2. Owner: stage a test-signed share in a new folder (leave the RC3 share
   untouched for rollback):
   `powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\prepare-vm-share.ps1 -Share C:\VMs\ar-share\rc-0.2.5-review -Version 0.2.5 -NextVersion 0.2.6`.
   The package in `target\driver-0.2.5-review` is unsigned (build check
   only) and must not be used in the VM. Then run VM Stage A from
   `02-test-signing-ready` (`run-stage-a.ps1 -Session all`) against that
   share, plus the probes in task 7 and a disable/enable with an open handle.
   If the bridge fails to appear after start, disable and re-enable the
   device: nothing retries control-device creation.
3. Then resume the pre-purchase plan in [virtual-cable.md](virtual-cable.md)
   at task 4 with 0.2.5 in place of RC3.

### R1/R2 work and VM probes (2026-11-11)

- Host: `cargo test -p audiorouter-windows-audio -p audiorouter-control`
  pass (363 + 147, 15 ignored); Clippy `-D warnings` clean on both crates;
  WDK build `target\driver-0.2.5-review-r2` pass, 0 warnings.
- `m03-bridge-fuzz.exe` gained `--probe-io` (READ/WRITE/FLUSH must fail
  with `ERROR_INVALID_FUNCTION`, QUERY must still work) and
  `--probe-stale-open "<restart command>"` (R2: OPEN on a handle from
  before a device restart must fail with 1167, OPEN on a new handle must
  succeed).
- Test-signed base share: `C:\VMs\ar-share\rc-0.2.5-review` (no
  `-NextVersion`: that and the full Stage A bundle need a clean commit).
- VM: current state saved as snapshot `pre-0.2.5-review-20261111`, then
  `02-test-signing-ready` restored. Host-started guest commands run
  unelevated without the share, so the probes run from the owner's
  elevated session: `Z:\review-0.2.5-probes.ps1 -Phase install`, then
  `-Phase probes`; logs land in `C:\VMs\ar-share\review-0.2.5-*.log`.

**VM results, first probe run (2026-11-11, AR-DriverTest, `02-test-signing-ready`):**

- Install: PASS (preflight 15/15; four endpoints; second install no-op).
- Status before and after: PASS (driver 0.2.5.0, protocol 1.1, 60 formats, min period 128).
- `--probe-io`: PASS (READ/WRITE/FLUSH each `ERROR_INVALID_FUNCTION`).
- `--probe-stale-open` with `pnputil /restart-device ROOT\MEDIA\0000`: PASS.
  OPEN on the old handle was refused with 1167, and OPEN on the new handle was accepted.
  **R2 verified in the VM.**
- Fuzz 120 s: 4,250,893 calls, 0 unexpected, 2,982 opens, no crash.
  The step reads FAIL only because its "Driver Verifier active" check fails; Verifier was off for this short run.
- Tone 30 s: FAIL, `0x80070057` from `activate_prepared_section`, with 0 blocks read.
  Cause: it ran after the fuzz.
  The fuzzer's generations are `(seed ^ (index+1)<<32)|1`, with PID<<32 in the seed, so about 1e13 or more.
  The tone tool uses Unix milliseconds (about 1.8e12).
  `AudioRouterValidateNextGeneration` therefore correctly refused the OPEN.
  The lease table outlives a device restart, so this lasts until the driver unloads.
  Not a regression from this diff. The probe script now runs the tone before the fuzz.
- Minidumps: 0.

**R6 (new, low, pre-existing):** `LastGeneration` is per slot, global and monotonic.
Any client that can open the bridge can OPEN once with a near-maximum generation.
After that, every product OPEN that uses a time-based generation is refused until the driver reloads.
That is a local denial of service of the cables.
Options:
- the driver resets the floor when the owner closes, or
- the driver assigns the generation in the OPEN reply instead of trusting the client.

Not fixed in this branch; it needs a protocol decision.

**R6 update (2026-11-11, owner asked to fix):**
- **Raised to Medium.** The product client also derives generations from Unix milliseconds (`crates/control/src/cable_routes.rs`), so a backward wall-clock step (NTP, a manual change) locks every cable until reboot, with no attacker involved.
- **Fixed without a protocol change.** Streams detect turnover only by inequality (`AudioRouterStreamGenerationChanged`), so a rising value was never needed. `AudioRouterValidateNextGeneration(last, prior, requested)` now requires a nonzero value that differs from the last two generations on the slot. `AudioRouterRecordGeneration` keeps that history, in the new `PriorGeneration` lease field.
- **ABA protection.** A-B-A is still refused. Reuse after two other leases is possible, but would need a running stream to miss two whole leases and three inactive gaps. Even then the result is a bounded glitch, because every view access is checked against `MappedBytes`.
- **No change elsewhere:** IOCTL, ABI and Rust client are unchanged.
- **Regressions in `bridge_tests.cpp`:** OPEN 2^63, then 1000 succeeds; A-B-A after the huge value is refused; a backward step (999) is accepted. The old rule fails the second check.
- **Host suite:** 10,915,246 PASS. WDK `build.ps1 -Version 0.2.5 -Output target\driver-0.2.5-review-r6`: 0 warnings, 0 errors.
- **Not run:** the VM rerun of fuzz then tone, which must now pass in that order.

**C26495 (2026-11-11):** WDK Code Analysis is now clean (0 warnings).
- `CSaveData` zeroes `m_FileName`, `m_FileSync` and the spin lock in its constructor, because its destructor frees `m_FileName.Buffer`.
- `CAdapterCommon`, `CMiniportWaveRT` and `CMiniportWaveRTStream` carry a documented `#pragma warning(suppress: 26495)`. They are only created through `operator new` (`ExAllocatePool2`), and nothing passes `POOL_FLAG_UNINITIALIZED`, so they start zeroed.

### Second VM probe run (2026-11-11, owner transcript 16:48)

- Status before/after and IO refusal probe pass. Fuzz: 4,144,666 calls,
  zero unexpected, 1,735 opens; Verifier was off, so qualification still fails.
  No minidumps reported.
- Tone fails before fuzz with all reported positions and packet counts zero.
  The previous attribution to test ordering is not established for this run.
  Need `C:\ar\evidence\20261111-164815-tone\tone.txt` and process report;
  do not infer successful audio from zero error counters or a WAV's existence.
- R2 old-handle OPEN is refused with 1167 again. New-handle OPEN fails with
  87. Source inspection identifies another ordering conflict:
  `freshGeneration()` in bridge_fuzz.cpp uses uptime milliseconds * 1000,
  whereas m03_bridge_tone.rs uses Unix milliseconds. A successful tone OPEN
  raises the same slot's generation floor above the stale probe's value.
  Thus the complete R2 probe is not repeatably passing; the old-handle
  rejection itself has passed twice. Do not treat the new-handle failure
  alone as evidence of broken device recreation.
- Next: collect the raw tone text/process report and guest last boot time
  without rerunning fuzz. Then isolate the tone failure and repair probe
  generation coordination before another qualification run. No kernel or
  protocol change authorized by this diagnostic update; existing rollback
  remains the untouched RC3 share.

### Tone failure identified (2026-11-11, owner raw report)

Guest last boot: 16:47:04, before the 16:48 tone. The process ran 30.413 s,
did not time out, and reports `no capture consumer acknowledged the primed
block; enable Listen before running`. One capture block was written; no
render blocks were read, WAV has zero frames. This run reached lease
activation; it did not reproduce the first run's OPEN error. The documented
Cable B Output Listen consumer prerequisite was unmet (or not functioning).
See the VM guide's listening setup. No driver regression is established by
this result, and audio qualification remains open. Next: reboot to clear
the subsequent fuzz generation floor, configure Cable B Output listening
through VM speakers, and run the tone step alone with audio playing into
Cable A Input. Do not rerun the combined probes until their generation
coordination is repaired. Driver Verifier and R1 live-stall evidence remain
open independently.

### Third run raw report (owner supplied, 2026-11-11)

The 16:55 tone ran 80.921 s, not 30 s; watchdog did not fire. Maximum
control loop gap was 64.730305 s; both audio worker gaps were approximately
64.619 s. Heartbeat calls themselves took at most 123 us, progress 36 us,
close 21 us. Capture published 1,592 blocks. Final error is `no render
audio recorded; play the Cable A Input test during the run`. Thus capture
acknowledgements now work, while render is unexercised. The simultaneous
long gaps establish a shared interruption of the tool's work, not its
cause (VM pause/starvation, host/guest suspension, or process suspension
remain hypotheses). This is not clean-run evidence and does not isolate a
driver timing regression. Next: establish whether VM/host was paused or
suspended, inspect progress snapshots around the gap, and ensure actual
playback into Cable A before a controlled short retry. Do not change host
hypervisor/security configuration based on this run.

### Third run progress snapshots (owner supplied, 2026-11-11)

Snapshots from 1,000 through 15,041 ms show zero capture error counters,
acknowledgements keeping up with publications (1,506 written/1,505 ack at
the last snapshot), and no render activity throughout. Interval audio worker
gaps remain below 10 ms in these snapshots. Final 3,100,779 underrun frames
at 48 kHz represent 64.5995625 s, closely matching the final approximately
64.619 s worker gap. This supports underrun accumulation during the long
interruption rather than a continuously failing capture path; it does not
identify the interruption's cause or qualify a complete clean run. Owner
also confirms hearing the tone. Next: determine whether the VM/host was
paused or suspended; exercise Cable A playback explicitly on the next
controlled run. No further fuzz needed for this diagnosis.

### Host-side corroboration (2026-11-11)

Owner confirms no pause or sleep. Read the host's
`C:\VMs\AR-DriverTest\Logs\VBox.log`; preserved a copy at
`target/review-0.2.5-tone-165505-VBox.log` (local evidence, untracked).
At VM elapsed 00:02:50.595794, VirtualBox reports a
`64 617 331 372 ns lag`, then `Guest seems to be unresponsive. Last heartbeat
received 66 seconds ago`, then guest alive after `66 135 630 583 ns`.
No RUNNING-to-PAUSED transition appears in this log. This independently
corroborates the tool's 64.619 s gap, not a deliberate pause. The VM uses
NEM/NativeApi timing. These facts do not establish whether the trigger is
host scheduling, hypervisor behavior, or a guest/kernel failure. Keep the
driver under investigation; do not exonerate it or change host security
settings. Next diagnostic is bounded scheduling capture with both cable
directions active, following the existing scheduling runbook, rather than
an unchanged clean-run retry. Audio acceptance remains blocked by this gap
and unexercised Cable A playback.

### Traced both-direction run (2026-11-11, 17:01)

Collected owner ZIP `C:\VMs\ar-share\evidence-20261111-170214.zip`;
extracted under `target/review-tone-170135`. Tone exited 0 in 30.098 s,
leases closed at 30,004 ms. Capture: 3,003 blocks, all error counters zero.
Render: 1,616 blocks / 775,680 WAV frames; 1,624 accepted OS packets,
zero late/overrun packets, but 6,167 underrun frames. Qualification FAIL,
not waived because the process succeeded. Maximum pump gaps about 18.9 ms;
the earlier 64 s stall did not recur.

Progress shows repeated render position resets and idle intervals, consistent
with the requested repeated Windows Test playback. Underruns rise by roughly
473-480 frames (about one 10 ms packet), interspersed with flat counts.
Suspect stream-start/packet-boundary provenance handling; not proven and no
kernel changes made. Do not attribute these losses to the prior VM stall.

Offline trace checks on Windows host: `xperf -i <etl> -o
target/review-tone-170135/dpcisr.txt -a dpcisr -summary` succeeds.
`target/m03-scheduler-trace.exe --silences <etl>
target/review-tone-170135/silences.csv 20` succeeds: 308,455 events,
zero lost, trace span 30.605 s, one all-CPU silence 40.685 ms at QPC
5075451954, after final active-audio QPC 5071125203. No long all-CPU
silence during active audio in this trace. This does not rule out individual
thread scheduling delays. Next: examine repeated stream-start and packet
commit/timing behavior against RC3 and the frame-clock change; reproduce in
a focused model before fixing. Both directions now exercised, clean-render
and full Stage A/Verifier/stall evidence remain open. Preserve the ETL and
WAV locally; no audio is added to source control.

### Render investigation continuation (2026-11-11)

Inspected SetWritePacket, STOP/PAUSE/RUN, commit provenance, current diff
and host render model. Existing model assumes packet 0 is prefilled and
does not exercise actual PortCls stream teardown; its passing table cannot
establish the cause of the observed restart losses. SetWritePacket also
explicitly marks EOS payload unusable, rejects EOS, and ignores its length
(existing specified limitation); whether EOS occurs in these VM Test
streams is unknown. Do not assume either startup or EOS is the cause.
No change to commit provenance is justified from the scheduler ETL alone.

Windows host `powershell -NoProfile -ExecutionPolicy Bypass -File
drivers/audiorouter-virtual/tests/build-tests.ps1` passes 10,915,240 checks;
compile /W4 /WX succeeds. Evidence: target/driver-unit-release/tests.log
and compile.log. No reproduction of the VM render losses yet.
Next concrete diagnostic: use the existing AR_TIMING_DIAGNOSTICS build
path to record actual stream state/packet-write timing, compare a sustained
render stream against repeated short streams, then add a regression at the
owning boundary. A packet trace build is diagnostic only, not a release
candidate. Preserve RC3 rollback and all unrelated working-tree changes;
no commit, PR or new VM driver install performed in this continuation.

### Packet diagnostic prepared (2026-11-11, owner asked to proceed now)

Objective: isolate the traced Cable A underruns (VCAB-11/25, render timing
and lifecycle), without changing production packet acceptance. Existing
diagnostics lacked render writes, so added diagnostic-only event 8 for
SetWritePacket (status, packet number, position; Extra high32 flags/low32
EOS length) and event 9 for state transitions (Extra KSSTATE). No audio or
pointers recorded. Doubled bounded diagnostic capacity to 262,144 records
to cover a 30-second two-stream run; normal builds retain no trace storage.
The collector's new `--tone` mode runs the existing 30-second vm-checks
entry point in its existing owned job/watchdog and saves trace on failure.
New guest-only `tools/vm/run-review-packets.ps1` verifies the bundle, forces
same-version repair for diagnostic install/rollback, and collects only the
current tone reports to the share. Reboot is required after either repair
before testing, even if helper returns 0. No installed VM change performed.

Final build: `build.ps1 -Version 0.2.5 -TimingDiagnostics -TestSign
-KeepOutput -Output target/driver-0.2.5-review-packetdiag-r3`: PASS,
WDK build and catalog zero warnings/errors; test-sign verification pass.
`tests/acceptance/m03-driver-package.ps1 -Package <r3>`: 33 PASS.
Collector `/W4 /WX /MT` build and `selftest`: PASS (ABI, request bounds,
concurrent append, exhaustion, saturation and 100 stop/drain cycles).
Host bridge suite after edits: 10,915,240 PASS. PowerShell parser clean;
wrapper Install and native --tone refuse this host before device access.
`git diff --check` clean; Git separately warns an existing modified
minwavertstream.h has CRLF normalization pending, no mass formatting done.
No Rust/UI behavior edited by this diagnostic continuation.

Staged `C:\VMs\ar-share\review-0.2.5-packets`: new diagnostic driver,
static collector, existing VM support/tools, wrapper, original 0.2.5 normal
package under rollback-driver, and verified packet-manifest.csv. Same
test certificate as the prior bundle. Dirty source identity explicitly
retained in package.json; this is not a release candidate. An initial
manifest enumeration picked up its own output; regenerated excluding the
manifest and verified every entry before handoff. Earlier diagnostic r1/r2
outputs are not the staged candidate. RC3 share remains untouched.

Next: owner copies this new folder to C:\ar in VM, runs wrapper -Phase
Install, reboots, restores Listen if necessary, runs -Phase Run while
repeating Cable A Input Test. Inspect new packet CSV against render counts;
if losses reproduce, add a focused regression before changing behavior.
Rollback: same wrapper -Phase Rollback and reboot installs the bundled
original normal 0.2.5 package. Not run: guest wrapper, loaded diagnostic
driver, real packet trace, continuous-versus-short comparison, full Stage A.

### Packet diagnostic START refused (2026-11-11, 17:36)

Owner Run output: `timing IOCTL action=1 error=1`; archive
`C:\VMs\ar-share\review-packets-20261111-173633-a826c36e.zip` is present
and extracted at `target/review-packets-failed-173633`. It contains package
metadata and exit result only; no tone or driver packet trace was started.
The bundled package metadata describes staged files, not the loaded image.
Host build.log confirms /D AR_TIMING_DIAGNOSTICS on adapter.cpp and stream
sources. Inspection: normal dispatch returns STATUS_INVALID_DEVICE_REQUEST
(Win32 1) for timing IOCTL; compiled diagnostic handler's START validation
returns distinct errors for malformed input, missing privilege, active owner
or epoch overflow. Thus likely normal driver still loaded; need actual
guest service ImagePath/hash and last boot before deciding to reinstall.
Do not count this as an audio failure or blame the owner. Install phase
output has not been supplied; no proof of actual diagnostic image selection
or post-install reboot exists in the collected archive. Next: read-only
guest service-path hash versus bundled diagnostic SYS hash and boot time;
if different, inspect helper repair result/PnP binding before another run.

### Guest binary mismatch confirmed (2026-11-11)

Owner read-only service check: AudioRouterVirtual Running, ImagePath under
DriverStore `audioroutervirtual.inf_amd64_9546438da57675a0`; file SHA256
BF26C09322F3D0CEF03A29D8B32252EA6727F1054763724FA86301188A52E2DE.
Host hash comparison matches the original rc-0.2.5-review driver exactly.
Bundled packet diagnostic SHA256 is
4E98484925E041DD8799B5EEEE8F90A9BC2FDC672BBF7D8409D400959C75B08C.
Guest boot was 17:33:52.500. Reboot occurred, but service still selects the
normal binary; this explains unsupported timing IOCTL. No evidence yet why
diagnostic install did not bind. Inspected wrapper: Install uses helper
repair; backend force_same=true reaches INSTALLFLAG_FORCE, so intended
command is appropriate, but actual helper outcome is still missing.
Next: owner runs Install phase and supplies complete helper result before
another reboot/run. Do not repeat tone or infer new driver/audio failure.

### Third tone run (2026-11-11, owner summary 16:55)

Evidence directory: `C:\ar\evidence\20261111-165505-tone`. Exit 1;
capture-sink underrun_frames=3,100,779, last_device_position=4,934,906;
other capture error counters zero. Render counters/position and packet
writes all zero. The capture endpoint is now running, but this is not clean
audio evidence. Missing render activity may reflect no playback into Cable A;
the summary alone cannot establish that or explain capture underruns.
Next: inspect raw tone.txt, tone-process.json and progress text before any
new run or driver change. Compare acknowledged block counts and pump gaps;
preserve this failed run. Do not attribute underruns to VM scheduling or the
new frame clock without evidence. All audio acceptance gates remain open.

### Packet diagnostic run (2026-11-11, 18:07)

The same-version repair never bound the diagnostic driver.
`pnputil /enum-drivers` showed a single `oem5.inf` at 0.2.5.0: the diagnostic INF has the same DriverVer, so Windows kept the stored original.
Fix: `pnputil /delete-driver oem5.inf /uninstall /force`, then wrapper `-Phase Install`, then a reboot.
The loaded SHA256 after that was `4E98484925…`, the diagnostic driver.
Rollback needs the same delete step first.

Run evidence: `C:\VMs\ar-share\review-packets-20261111-180751-98495685.zip`, extracted to `target/review-packets-180751`.
Tool exit 1.

- **Render (Cable A):** no render stream ran at all.
  The trace holds 42,945 records, all direction 2 (capture), with no event 8 or 9 for render.
  Nothing was played into Cable A Input during the run, so the Cable A underrun question is still untested.
  Next run: start the Cable A Input test sound (or any playback) before the 30 s window, and keep it playing throughout.
- **Capture (Cable B):** 3,719 underrun frames.
  They fall exactly in the four progress intervals where the harness capture worker's gap reached 28 to 62 ms: at 10 s 34 ms, 17 s 31 ms, 19 s 62 ms, and 22 s 28 ms.
  In every interval with gaps of 10 ms or less the count did not move.
  The same intervals show control-loop gaps up to 78 ms.
  This matches the 2026-10-10 lesson: VirtualBox NEM stalls all guest threads.
  The cause is user-mode pump starvation, not the driver, but it is attributed by correlation only, with no scheduler trace.
  Driver Verifier and audio acceptance stay open.
