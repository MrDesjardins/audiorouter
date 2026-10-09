# Isolated tone harness — 2026-10-08

Scope: WP-06, VCAB-21–24 test tooling and ownership/shutdown support. This
implements the user's approved harness design after the failed
`evidence-20261008-205521.zip` run. The driver, ABI, private queue capacity and
zero-error acceptance criteria are unchanged. No test driver was loaded on
the host.

## Changes

- Prepare and prime mapped sessions before activating broker leases. Audio
  workers signal readiness after their Windows priority setup succeeds.
- Separate capture producer and render consumer threads. Each owns one
  direction and polls its mapped slot; neither performs console output,
  heartbeat IOCTLs, WAV serialization or disk writes. Capture publishes only
  after acknowledgement, with no overwrite fallback when no consumer is open.
- Main thread owns the control handles, sends heartbeat requests and prints
  progress. Windows handles were not made `Send` with unsafe code.
- Recording worker receives preallocated float64 blocks via bounded queues.
  There are 64 buffers, at most 16 MiB at the maximum negotiated block size.
  An exhausted pool, sequence gap in a clean run, malformed mapping, disk
  failure or worker panic causes a harness failure. Deliberate stall runs
  report sequence gaps separately and retain raw driver counters.
- End-of-run capture service continues until native lease deactivation. Both
  leases deactivate before mapping flush, file cleanup, console summaries or
  joins. Render drains the stable final publication; recording drains queued
  blocks and patches the WAV header. No counter resets or subtraction.
- Diagnostics report maximum capture/render pump gaps, heartbeat/progress
  durations, lease close duration, WAV append/finish durations and render
  sequence gaps. Priority is `THREAD_PRIORITY_HIGHEST`, not a realtime process
  class or an MMCSS guarantee. Timer resolution has matched RAII cleanup.
- Missing capture acknowledgement or an empty render recording is an explicit
  setup failure, rather than a successful empty WAV.

## Host verification

Windows host, pinned Rust 1.96.0. These are mapped user-mode and tooling
checks, **not kernel/runtime continuity or latency evidence**.

| Check | Result | Evidence/command |
| --- | --- | --- |
| Actual worker/mapped-slot regressions and existing waveform/WAV/options tests (initial harness) | 11 passed | `cargo test -p audiorouter-windows-audio --example m03_bridge_tone`; `target/isolated-tone-worker-tests.txt` |
| Post-guest teardown fixes and worker regressions | 12 passed | `cargo test -p audiorouter-windows-audio --example m03_bridge_tone`; `target/isolated-tone-worker-tests-fix1.txt` |
| Repeated full tone suite | 10 consecutive runs passed (110 test executions) | `target/isolated-tone-repeat-tests.txt` |
| Bridge/session/ABI/mapping tests, including absent-device prepared activation cleanup | 30 passed | `cargo test -p audiorouter-windows-audio --lib native_bridge`; `target/isolated-tone-bridge-tests.txt` |
| Native stdout/stderr, exit codes, strict complete final counters | Passed | `tests/acceptance/m03-vm-inventory-output.ps1`; `target/isolated-tone-output-check.txt` |
| Documentation links/fences | 134 files, 725 links passed | `tests/acceptance/docs.ps1`; `target/isolated-tone-docs-check.txt` |
| VM baseline guards and read-only default-role snapshot | 275 checks passed | `tests/acceptance/m03-driver-vm-guards.ps1`; `target/isolated-tone-guards.txt` |
| Formatting | Both workspace and shell checks passed | `cargo fmt --all -- --check`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` |
| Focused Clippy | Passed | `cargo clippy -p audiorouter-windows-audio --all-targets --all-features -- -D warnings`; `target/isolated-tone-clippy.txt` |
| Guest retry script | Syntax checked; execution requires guest | PowerShell parser, `tools/vm/retry-isolated-tone.ps1` |

The isolated worker regression holds the actual recording append callback
while the control thread is blocked for 160 ms. A mapped peer consumes capture
and publishes render blocks concurrently. Both audio workers advance by at
least ten blocks during that blockage. The test then parses the resulting
float64 WAV and verifies all 30 blocks in order, alongside exact generated
capture samples. It models blocked control work; it does not execute real
driver heartbeat requests on the host.

Other regressions cover exhausted recording storage, disk errors and returned
buffer ownership, cancellation before activation, stable final render drain,
rejection of stale generations, and a real 120 ms producer sleep remaining
visible in pump diagnostics without replaying sequences.

## Limits and next action

The earlier VM failure's precise cause is still unproved. Isolation prevents
short recording/control stalls from directly pausing audio service, but does
not guarantee Windows scheduling. A control stall beyond the two-second
broker lease can still expire ownership. The bounded recording pool cannot
absorb an indefinitely blocked disk. Its exhaustion must remain a failure.

### First isolated guest attempt exposed a teardown-order defect

Archive `evidence-20261008-211936.zip`; tone run `20261008-211905-tone`.
Installed-driver status passed (protocol 1.1, driver 0.1.0.0, four endpoints,
all 60 formats). During the 30-second run the capture acknowledgement advanced
to 2910 and the device position advanced; the tone worker published 3003
blocks. Render recorded 1271 blocks / 610080 WAV frames, so Cable A Input was
stimulated. Progress snapshots showed zero runtime error counters.

The tool then exited 1 with `render mapping: SampleSizeMismatch`. Its final
counter snapshots incorrectly showed zero device positions because the driver
resets its mapping header on CLOSE. Code review also found `close()` removed
the owned mapping files before the scoped audio workers joined. Fixed shutdown
now snapshots counters before CLOSE, deactivates both leases, stops and joins
audio workers and drains the recording queue while mapping files are still
owned, then removes the files. A retired format header is accepted only during
shutdown; the same error remains fatal during an active lease. Host tone tests
now number 12; all passed after this fix. The guest attempt does not qualify
continuity, but confirms both leases opened and real render audio was recorded.
A fresh 30-second result is still required.

### Corrected 30-second guest run passes

Archive `evidence-20261008-212408.zip`, SHA-256
`A17719B5480B1DCE86CB7AD2028D505FD8A1F465BDC5BABA25C6DF3A0AAEE923`
(5,559,146 bytes). Tone summary has all four checks passing and collect has
both checks passing. The final five error counters are zero in each direction;
capture ended at device position 86,769,696, render at 69,648. Capture
published 3003 blocks and the driver acknowledged through sequence 2910 before
close. Render produced 1019 blocks (489,120 frames). There were no harness
render sequence gaps. Capture and render maximum pump gaps were 5853 us and
5744 us; maximum heartbeat, progress output and lease close were 244 us,
611 us and 27 us. WAV is stereo 48 kHz IEEE float32, 489120 frames (10.19 s),
with nonzero signal in both channels: RMS 0.1811/0.1691, peak 0.9851/0.9794.
Render publications stopped advancing near 20 seconds while capture continued;
the run log has audio activity but does not establish continuous Cable A test
playback for all 30 seconds.

This verifies the harness teardown fix and a clean short VM run. Proceed to the
10-minute tone run; keep Cable B Listen active, check that the test tone is
audible, and use Cable A Input Test several times during the run so the WAV
continues to capture render-side samples. Continue to stop before deliberate
stall/8-channel runs until the 10-minute trace has been reviewed. The longer
run still needs zero driver error counters, and no VM latency measurement has
yet been made. Evidence is from guest `AR-DriverTest` running the already
installed test-signed driver; no driver build changed in this harness pass.

Jev was not rerun: automatic approval review previously rejected uploading
source to its external service. ASan remains unavailable with the installed
MSVC runtime, as recorded in the active plan. Neither is a passing check.

The initial tool-only bundle at
`C:\VMs\ar-share\diagnostics-20261008-isolated-tone` used clean source
`bac0cc0de611636b61946c9c35c743bfb7e41639`, staged at
`2026-10-09T04:17:12.3464104Z`. Its execution caught the teardown defect above;
The original executable has now been replaced by the teardown fix. Bundle
metadata: clean commit `966a83b6dd26bf6cd47c5beca5d059a3a9d1a59c`, staged
`2026-10-09T04:22:46.2570174Z`; tool SHA-256
`E54533B9E011488DC19F6BA8D948AE035AC2B6706ED775D5500039365D621043`.
The check script hash matches its committed copy. The unchanged installed
driver hash is `692C013CF9727985FE804F4020388108D1A568ECF8FC0D99B9B525771E39D646`.
No driver files were copied. Release build and expected `--help` launch
passed; logs: `target/isolated-tone-release-build-fix1.txt` and
`target/isolated-tone-release-launch-fix1.txt`.

Next: run the replaced tool-only diagnostic once for 30 seconds on the already
installed `dbf19e17` guest. Keep Cable B Output Listen active, stimulate
Cable A Input and review the collected evidence.
VCAB-24 sustained zero-error, fidelity and latency gates remain open. Rollback
is the old tone executable in `repair-20261008-capture-tick-primed` or the
clean guest snapshot.
