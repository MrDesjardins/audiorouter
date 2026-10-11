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

## Bundle r4 with the isolation check (VCAB-26 preparation)

Commit `9717d2b1`: probe mode `cable-isolation SECONDS RENDER CAPTURE`
(noise into one Input, every captured sample must be +0.0; peak in dBFS)
with `cable-isolation-selftest` (4 checks, including a −140 dBFS sample and
−0.0 counted as leaks). The runner adds a tone-mode session (tone on Cable B)
and records Cable A Output while noise plays into Cable A Input and into
Cable B Input. Host: `/W4` clean; cable acceptance 167 checks (new `leak`
case; mutation ignoring the probe's isolation verdict is caught). Bundle
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r4`: source `9717d2b1`,
driver `492d8ca8` (SYS SHA-256 unchanged), 36 entries, manifest SHA-256
`6B81F23A9E735B50EC13F4ED04B94D2FE2766665B8873FF69D2DEF15C7F925CC`. Use r4.

## Bundle r5 (guest-quieting fix)

In the guest, `quiet-guest.ps1 -Apply` stopped at its CPU-settle wait with
"A counter with a negative denominator value was detected" after applying
and recording every setting. The fix retries failed samples (48 host
checks); it landed in commit `1006bdad` with the docs pointing here.
Bundle `C:\VMs\ar-share\diagnostics-20261010-cable-latency-r5`, source
`1006bdad`, driver `492d8ca8` (SYS SHA-256 unchanged), manifest SHA-256
`83FA6C659A9EDA98092648FD665153A04A4DC8442C9ED16DCA88C60889619C3D`. Use r5.

## First VM run (bundle r5) — 2026-10-10, NEM VM, guest quieted

Run `latency-efde7b79506a4cc688e2c01a13d76bf9`, archive
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r5\latency-efde7b79506a4cc688e2c01a13d76bf9.zip`,
SHA-256 `7FFB85F9804FF744D66421BDE751F23BEA67337D859A56E7E2E613900F3D39FC`,
extracted to `target/latency-efde7b79`. VirtualBox under NEM (host hypervisor
on); guest quieted with `quiet-guest.ps1`.

| Check | Result |
| --- | --- |
| Bit-exact (VCAB-20 proxy, 480 frames) | **Pass**: 480,000 frames aligned and identical, 0 mismatched samples, 24,000 silence frames exact +0.0; relay forwarded 1,201, dropped 0 |
| Isolation (VCAB-26 proxy) | Cable A Output exact silence on both paths, **but inconclusive**: nothing read Cable B Output, so Cable B carried no tone (tone tool exit: "no capture consumer acknowledged the primed block"); fixed in r6 |
| Latency, 480-frame relay, default periods | p1 56.942, p50 56.943, p95 56.945, p99 93.944 ms; 980/1000 matched, 20 lost (6,240 capture frames dropped during a 142 ms VM pause); target p95 ≤ 40 ms missed |
| Latency, 128-frame relay, low-latency periods | p50 92.3, p95 126.0 ms; 82 lost; packet writes late 137, overrun 55; render underrun 14,160; target missed |

Reading: the default-mode latency is a deterministic ≈56.94 ms pipeline in
this VM: the probe's WASAPI streams were granted 1,056-frame (22 ms) buffers
on each side, plus the 480-frame bridge quantum and relay. Low-latency
periods (128 frames, 2.67 ms) do not hold under NEM timing. Neither number
is product evidence for VCAB-25 (proxy route, VM timer delays, no VB-Cable
comparison); a native or bare-metal run is required.

## Bundle r6 (isolation keeps Cable B active)

Commit `ac20975e`: `cable-isolation` takes an optional fifth argument, a
second Output to record and discard, and reports
`isolation_drain_active_frames`; the runner drains Cable B Output and passes
isolation only if Cable A Output is exact silence **and** Cable B carried
audio. Acceptance 192 checks (new `idle-b` case: silence with an idle Cable B
is inconclusive). Bundle `C:\VMs\ar-share\diagnostics-20261010-cable-latency-r6`,
source `ac20975e`, driver `492d8ca8` (SYS SHA-256 unchanged), manifest
SHA-256 `2311F75CC61D5CEB4BD1C8B0EF1C329F35E9582DE1386DCA910B4DF6C43B0AD4`.
