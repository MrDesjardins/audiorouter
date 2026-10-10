# M03 cable latency diagnostic preparation — 2026-10-10

Scope: VCAB-25 preparation (impulse latency and jitter), prepared host-side
while the user was away (user request: work on non-VM gates, keep going).
No host setting, driver installation or audio endpoint was touched on the
host; the VM was not used.

## What was built

Commit `fbf5d0b6`:

- **Pass-through relay** (`crates/windows-audio/examples/m03_bridge_tone`):
  `--passthrough` republishes Cable A render-source blocks into the Cable B
  capture sink instead of the tone. Three preallocated relay buffers
  (`RELAY_BLOCKS`), lock-free queues, no allocation in the audio workers;
  silence (counted) until audio arrives; the oldest block is dropped (counted)
  if the relay backs up, so the added delay stays bounded. The capture sink is
  primed with silence in this mode. Final report line:
  `pass-through relay: forwarded=… silence=… dropped=…`.
- **Identity-coded impulse probe** (`tools/m00-native-wasapi-probe`, mode
  `cable-impulse COUNT RENDER CAPTURE [low-latency]`): impulse *k* carries the
  exact float32 amplitude (1 + k mod 16)/32; arrivals are paired to their own
  impulse within a 155 ms window, so losses cannot shift later pairs (the
  existing `impulse-loopback` mode pairs by arrival order and is kept
  unchanged for physical loopback). Reports min/p1/p50/p95/p99/max/mean,
  p99 − p1 jitter, emitted/matched/lost/corrupted/duplicate/out-of-window,
  capture dropped frames and discontinuity/timestamp-flagged packets.
  `cable-impulse-selftest` checks the pairing offline. `build.ps1
  -StaticRuntime` links `/MT` for the VM (default build unchanged).
- **Guest runner** `tools/vm/run-cable-latency.ps1`: driver status first,
  endpoint indices by exact name (exactly one Cable A Input and one Cable B
  Output), two configurations (480-frame relay / default periods, target p95
  ≤ 40 ms; 128-frame relay / low-latency periods, target ≤ 20 ms; jitter
  ≤ 2 ms; zero lost/corrupted), bounded watchdogs, owned-process cleanup,
  `result.json` with `Qualification=false`, archive of this run only.
- **Preparer** `tools/vm/prepare-cable-latency-update.ps1`: builds both tools
  with a static C runtime from a clean commit, checks imports and the probe
  self-test, copies a verified base bundle replacing only the tone tool.

## Host verification

| Check | Result |
| --- | --- |
| Probe `cable-impulse-selftest` (static build) | 138 checks pass: exact float coding, lossless 23.4 ms, losses do not shift pairs, p1/p99 jitter, corrupted/duplicate/early/late/out-of-range arrivals, 154 ms identity wrap |
| Probe imports | `ole32.dll`, `MMDevAPI.DLL`, `KERNEL32.dll` (no Visual C++ runtime) |
| Tone tool tests | 19 passed, including relay unit test and end-to-end mapped-slot pass-through (10/10 repeated runs) |
| `tests/acceptance/m03-cable-latency.ps1` | 86 checks passed; mutation (low-latency target 20 → 30 ms) is caught |
| fmt, workspace and src-tauri Clippy `-D warnings` | Clean |

Bundle `C:\VMs\ar-share\diagnostics-20261010-cable-latency`: source
`fbf5d0b6`, driver `492d8ca8` (SYS SHA-256
`10CF8E879DE85E3A924CDCC0DBA2987D00CB8328DAB9CBA870B997F3EF952381`
unchanged), 35 entries; only `tools\m03_bridge_tone.exe` differs from the
base, plus `tools\m00-probe.exe`, `run-cable-latency.ps1` and
`paired-trace-support.ps1`. Manifest SHA-256
`15783046633164DA7D5B7B4B8616C39F9502259FB351B7EA4DE9B92F53F6EF78`.
Guest command: [latency runbook](../../../operations/virtual-cable-latency.md).

## Limits

- Proxy route, not the product engine; no VB-Cable comparison (not present in
  the VM); the VM's NEM timer delays make the numbers an upper bound.
- Not run in the VM yet; no latency number is claimed.

## Bundle r2 with guest quieting

Commit `bf1cc2ba` adds the reversible `quiet-guest.ps1` (47 host checks:
apply/refuse-second-apply/exact revert, pre-existing values and exclusions
preserved, settle wait, host refusal) and copies it into the bundle. The
first bundle stays as built; use
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r2`: source `bf1cc2ba`,
driver `492d8ca8` (SYS SHA-256 unchanged), 36 entries, manifest SHA-256
`6685987B95DBE2F7752CE151DF31EA96F27D8452B633F05471D7F14878B0CE11`.

## Bundle r3 with the bit-exactness check (VCAB-20 preparation)

Commit `6c620ff1`: probe mode `cable-bitexact SECONDS RENDER CAPTURE`
(seeded noise on the 2^-24 grid, |x| ≤ 0.5, per-channel seeds; exact 32-frame
alignment; every sample compared; trailing silence must be +0.0) with
`cable-bitexact-selftest` (10 checks: grid/bounds/channel distinctness, exact
pass with lead-in, one-LSB change located, dropped 480-frame block, swapped
channels, half gain, −0.0 in silence, early capture stop). The runner adds a
third pass-through run at 480 frames and records `BitExact` in
`result.json`. Host: `/W4` compile of the probe without warnings; cable
latency acceptance 122 checks (new `not-exact` case; mutation ignoring the
probe verdict is caught). Bundle
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r3`: source `6c620ff1`,
driver `492d8ca8` (SYS SHA-256 unchanged), 36 entries, manifest SHA-256
`53BBE034142F3EBF6206EE3509BD5E981C74FC5720E04CB34FF87BFAEB40D6B6`. Use r3;
r1/r2 stay as built.
