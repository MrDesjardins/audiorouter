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
| Actual worker/mapped-slot regressions and existing waveform/WAV/options tests | 11 passed | `cargo test -p audiorouter-windows-audio --example m03_bridge_tone`; `target/isolated-tone-worker-tests.txt` |
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

Jev was not rerun: automatic approval review previously rejected uploading
source to its external service. ASan remains unavailable with the installed
MSVC runtime, as recorded in the active plan. Neither is a passing check.

Next: stage a tool-only diagnostic for the already installed `dbf19e17` guest
driver, keep Cable B Output Listen active, stimulate Cable A Input, run only
30 seconds and review collected evidence. VCAB-24 sustained zero-error,
fidelity and latency gates remain open. Rollback is the old tone executable
in `repair-20261008-capture-tick-primed` or the clean guest snapshot.
