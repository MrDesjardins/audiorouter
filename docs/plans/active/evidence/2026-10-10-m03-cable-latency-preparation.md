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

<a id="second-vm-run-bundle-r6--2026-10-10-nem-vm-guest-quieted"></a>

## Second VM run (bundle r6) — 2026-10-10, NEM VM, guest quieted

Run `latency-6ec5fe40a5294257a05f38a997009610`, archive
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r6\latency-6ec5fe40a5294257a05f38a997009610.zip`,
SHA-256 `5121B6C2BD9EF235BB477D7E2CDA139D51DC28E9D700817C153DA278EE7376C3`
(verified on the host), extracted to `target/latency-6ec5fe40`. Driver
`0.1.0.0` installed (helper protocol 1.1), source `ac20975e`, VirtualBox under
NEM, guest still quieted from the r5 run. Runner verdict `Passed=True`
(the diagnostic completed); `Qualification=False` (proxy scope).

| Check | Result |
| --- | --- |
| Bit-exact (VCAB-20 proxy, 480 frames) | **Pass**: 480,000 frames compared, 0 mismatched samples, silence exact; relay forwarded 1,200, silence 19, dropped 0 |
| Isolation (VCAB-26 proxy) | **Pass**: Cable A Output exact silence (0 nonzero samples, peak −1000 dBFS sentinel) on both paths while Cable B Output carried audio (A Input→A Output path: 239,999 of 240,000 drained frames active; B Input→A Output path: 240,480 of 240,480) |
| Latency, 480-frame relay, default periods | p1 33.387, p50 33.389, p95 33.390, p99 33.390 ms, jitter 0.002 ms; 1,000/1,000 matched, 0 lost, 0 corrupted; packet writes 1,102 accepted, 0 late, 0 overrun; meets the default target (p95 ≤ 40 ms, jitter ≤ 2 ms) in this run |
| Latency, 128-frame relay, low-latency periods | p50 107.3, p95 150.6 ms, jitter 157.3 ms; 898 matched, 102 lost, 0 corrupted; 28,776 capture frames dropped; packet writes 3,857 accepted, 159 late, 46 overrun; render underrun 18,144; target missed |

Reading:

- The data path is correct through the proxy route: bit-exact and isolated,
  with no corrupted or duplicated impulse in either latency run.
- Default mode was again deterministic (0.002 ms spread), but its steady
  offset moved from 56.94 ms (r5) to 33.39 ms (r6) with the same 1,056-frame
  client buffers. The 23.55 ms difference (≈1,130 frames, about one client
  buffer) is a startup phase: how much each queue holds when the streams
  start, which then stays fixed for the run. One run is therefore not a
  latency figure; VCAB-25 needs repeated starts and a p95 across them.
- Low-latency periods (128 frames, 2.67 ms) still do not hold under NEM
  timing (late/overrun packet writes, capture drops), matching the traced
  timer delays. No driver conclusion follows from this VM.
- Not product evidence: proxy relay instead of the AudioRouter engine, no
  VB-Cable comparison, NEM VM timing.

Remaining for VCAB-20/25/26: the same checks through the product engine
route; repeated-start latency and the VB-Cable comparison on a native VT-x or
bare-metal Windows test PC (this PC cannot run native VT-x; see
[native VT-x session](../../../operations/virtual-cable-native-vtx-session.md)).
The guest stays quieted until `quiet-guest.ps1 -Revert` is run in the VM.
## Bundle r7 (engine route, step 1 of the single-PC path)

Commits `72aa6682` and `31f8feec`. Before this, no Cable A → engine →
Cable B path existed (WP-09 not started). Added:

- `crates/engine/tests/cable_route.rs`: a `VirtualRenderSource →
  VirtualCaptureSink` session compiled by the engine and run on
  `RealtimeScheduler` passes float32 noise on the 2^-24 grid unchanged in
  mono and stereo (3 tests). Findings: the engine processes 128-frame quanta
  of at most 2 channels (`MAX_CHANNELS`), so 8-channel cables (VCAB-20)
  cannot pass through it yet; and its channel matrix maps −0.0 to +0.0 by
  design (`mapped_mono`), the only bit difference, pinned by the test.
- `CableRouteProcessor` (`crates/windows-audio/src/cable_route.rs`):
  re-blocks bridge blocks into engine quanta with preallocated FIFOs (delay
  below one quantum) and fails closed to counted silence; 4 unit tests
  (16–4,096-frame blocks, exact ramps, missing graph, unsupported shapes).
- `m03_bridge_tone --engine` runs that processor in its render worker;
  20 example tests including a continuous, unchanged ramp through mapped
  slots. `run-cable-latency.ps1 -Route engine` uses it and records
  `Route` and an engine scope; acceptance 220 checks (new `engine-pass`).

Host checks 2026-10-10: the tests above pass; `cargo fmt --all -- --check`
clean; `cargo clippy --workspace --all-targets --all-features -- -D
warnings` clean. Not run on the host: any driver or audio stream.

Bundle `C:\VMs\ar-share\diagnostics-20261010-cable-latency-r7`, source
`31f8feec`, base `repair-20261010-slot-provenance`, driver `492d8ca8`
(SYS SHA-256 `10CF8E879DE85E3A924CDCC0DBA2987D00CB8328DAB9CBA870B997F3EF952381`,
unchanged), manifest SHA-256
`83DF4E23FED3E842CFA8C6DC77B81EB14B59D892D54B306669C379B8B38F9647`.
Guest command: the runbook block with `-Route engine`.
<a id="third-vm-run-bundle-r7-engine-route--2026-10-10-nem-vm-guest-quieted"></a>

## Third VM run (bundle r7, engine route) — 2026-10-10, NEM VM, guest quieted

Run `latency-d89ce8d3cb604e28b5353b5b9c505004`, archive
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r7\latency-d89ce8d3cb604e28b5353b5b9c505004.zip`,
SHA-256 `7CD9B22764FB1028B9D4E2CE68986F5988F585EA5CB1295FD731517A7F6D65C9`
(verified on the host), extracted to `target/latency-d89ce8d3`.
`-Route engine`, source `31f8feec`, driver `492d8ca8`.

| Check | Result |
| --- | --- |
| Bit-exact (VCAB-20, engine route, 480 frames) | **Pass**: 480,000 frames, 0 mismatched samples, max difference 0, silence exact; engine quanta processed 4,507, silent 0; relay dropped 0 |
| Isolation (VCAB-26) | **Pass** on both paths with Cable B active (239,999+ and 240,480 active frames); 0 nonzero samples |
| Latency, 480 frames, default periods | p1 41.654, p50 51.653, p95 51.654, p99 51.654 ms; **jitter 10.0 ms**; 1,000/1,000 matched, 0 lost, 0 corrupted; packet writes 1,101 accepted, 0 late; engine silent 0 |
| Latency, 128 frames, low-latency periods | p50 97.2, p95 137.5 ms, jitter 159 ms; 96 lost; packet writes 164 late, 54 overrun (NEM timing, as with the proxy) |

Reading: the engine route carries audio unchanged and isolated. The default
latency is bimodal, exactly one 480-frame block (10 ms) apart, where the
proxy route had 0.002 ms jitter. Cause (route defect, not the VM): the
re-blocking processor emitted 0, 1, 1, 2 output blocks per 480-frame input
block (a model reproduces the cycle); the capture sink takes one block per
period, so the relay depth, and the latency, toggled by a block.

Repair (host, same day): the output starts with a constant delay of
`quantum - gcd(block, quantum)` frames of silence (96 frames, 2 ms, at 480;
0 at 128) and emits at most one block per input block, so every full input
block yields exactly one output block at a constant delay. Unit tests now
assert one emission per push at 16, 128, 144, 480 and 4,096 frames, the
leading delay, and an unchanged sample stream; `reblock_delay_frames`
values are pinned. Low-latency (128-frame) blocks never had the defect;
their failure is the VM timing seen on the proxy route.
Bundle r8: `C:\VMs\ar-share\diagnostics-20261010-cable-latency-r8`, source
`33dc8661`, driver `492d8ca8` (SYS SHA-256 unchanged), manifest SHA-256
`8859D4D802211D332F356A165E135D24377EDFA6A48837831C9795AB80A1388C`.
Host checks: cable route unit tests (5), tool tests (20), latency
acceptance (220), `cargo fmt --check` and workspace Clippy clean. Same guest
command as r7 with the r8 folder.
## Fourth VM run (bundle r8, engine route) — 2026-10-10, NEM VM, guest quieted

Run `latency-ab73ed2e165a47b0931603da9184abda`, archive
`C:\VMs\ar-share\diagnostics-20261010-cable-latency-r8\latency-ab73ed2e165a47b0931603da9184abda.zip`,
SHA-256 `059F76DEEB8B55B10E689A9CF8964E93A07152C9549ECF83928C81DE0B12457D`
(verified on the host), extracted to `target/latency-ab73ed2e`.
`-Route engine`, source `33dc8661`, driver `492d8ca8`.

| Check | Result |
| --- | --- |
| Bit-exact (VCAB-20, engine route, 480 frames) | **Pass**: 480,000 frames, 0 mismatched, max difference 0, silence exact; engine silent quanta 0; relay dropped 0 |
| Isolation (VCAB-26) | **Pass** on both paths with Cable B active; 0 nonzero samples |
| Latency, 480 frames, default periods | min = p1 25.272 ms, p50 50.273, p95 50.275, max 57.273, mean 44.622; 996/1,000 matched, 4 lost, 0 corrupted |
| Latency, 128 frames, low-latency periods | p50 48.7, p95 150.0 ms; 90 lost; 166 late / 34 overrun packet writes (NEM timing, unchanged) |

Default-mode reading:

- One VM-wide stall between 4 and 5 s: the capture, render and control
  threads all recorded ~51.5 ms gaps at once; it caused the only counter
  increases (capture-sink underrun 1,536, render-source underrun 1,104 and
  overrun 1,776 frames, 1 late packet write, 1,920 probe capture frames
  dropped) and the 4 lost impulses.
- Latency had exactly two steady levels: 25.272 ms before the stall
  (≈23 % of impulses, from the mean) and 50.273–50.275 ms after it. Each
  level is flat to about 0.003 ms, so the r7 one-block toggle is gone: the
  re-blocking repair is confirmed.
- The +25 ms shift is not in the route: the tool's written − read block
  count (relay and priming) stayed 19–21 before and after the stall. The
  extra delay sits in the endpoint buffering outside the route (the probe's
  WASAPI clients have 1,056-frame, 22 ms buffers per side), which the stall
  left fuller. The same mechanism explains the proxy runs (r5 56.94 ms
  after a VM pause, r6 33.39 ms without one).

Consequences: a single end-to-end latency figure in this VM depends on
start-up and stall history by up to about one client buffer. Step 2
(repeated starts and the same-VM VB-Cable comparison) must report steady
levels per start and mark stall-shifted segments rather than one pooled
percentile.
## Bundle r9 (repeated starts and VB-Cable comparison, step 2)

Commit `a2bd36bf`. The probe reports steady latency levels in impulse order
(`cable_segments`, `cable_steady_level_ms`, up to eight `cable_segment`
lines; tolerance 1 ms from a level's first impulse; the first level lasting
50 impulses is the start's steady level). Self-test 146 checks, including
the r8 shape (two levels around a stall), emission-order independence, the
r7 one-block toggle (200 segments) and slow drift. New runner
`run-cable-latency-starts.ps1 -Route engine|vbcable` (fresh streams per
measurement, both period modes, `-Starts` 5, 400 impulses); acceptance 112
checks; the existing latency acceptance still passes 220.

Bundle `C:\VMs\ar-share\diagnostics-20261010-cable-latency-r9`, source
`a2bd36bf`, driver `492d8ca8` (SYS SHA-256 unchanged), manifest SHA-256
`8029570880076E299A52596E06CCBABF87396AE339CBE9E6B491C651458C7C4B`.
## Repeated starts, engine route (bundle r9) — 2026-10-10, NEM VM, guest quieted

Run `starts-engine-05606185287447139fc1b5db3b56311a`, archive SHA-256
`D9D6D7EB5D98A79871295BF3CDBF670599B9BE1B5FF0398675BE2CD51461A670`
(verified on the host), extracted to `target/starts-engine-05606185`.
5 starts × 2 modes, 400 impulses each; every probe and route exited 0;
engine silent quanta 0 and corrupted impulses 0 in all ten.

| Mode | Steady level per start (ms) | Median | Range | Starts that shifted | Lost |
| --- | --- | --- | --- | --- | --- |
| Default (480-frame blocks, 1,056-frame client buffers) | 33.870, 31.107, 42.024, 40.094, 35.238 (spread 0.002 each) | 35.238 | 31.1–42.0 | 1 (start 1: +10 ms later) | 63 (start 2) |
| Low-latency (128-frame blocks, 280-frame client buffers) | 150.966 in start 5 only; no 50-impulse level in starts 1–4 (25–48 segments each) | — | — | 5 | 154 |

Reading: in default mode every start settles on one flat level, and the
level varies between starts by about 11 ms (half a 22 ms client buffer),
which is the start-up buffering effect seen in r5–r8. The median, 35.2 ms,
is below the 40 ms default target; two of five starts were above 40 ms. In
low-latency mode the route never holds a level under NEM (40–66 late and
15–23 overrun packet writes per run), as on the proxy route. These are
indicative VM numbers. The VB-Cable comparison run is next.
## Repeated starts, VB-Cable (bundle r9) — 2026-10-10, NEM VM, guest quieted

VB-Cable installed by the user inside AR-DriverTest only (endpoints
`CABLE Input`, `CABLE In 16ch`, `CABLE Output`, defaults kept). Run
`starts-vbcable-5c95dde8570c48afbf2937afad07f403`, archive SHA-256
`A1A4C0FB246709A83B1064CBE1E52B554787AF70FD8993A0FB4C71A6DD5A033B`
(verified on the host), extracted to `target/starts-vbcable-5c95dde8`.
Both VB-Cable endpoints use 48 kHz float32 like the AudioRouter cables. The
runner exited with failure because start 4 (default) ended with 17 impulses
emitted, 0 matched and 39,680 capture frames dropped (probe exit 1; a VM
disturbance at start). VB-Cable's capture side ignores the low-latency
request (capture buffer 1,056 frames in both modes).

Per-start medians (p50, ms); "clean" = no lost impulse and no dropped
capture frame:

| Mode | AudioRouter engine route (r9) | VB-Cable Input → Output, no route |
| --- | --- | --- |
| Default, clean starts | 33.870*, 42.024, 40.094, 35.238 | 49.709, 51.507, 48.397 |
| Default, disturbed starts | 31.107 (63 lost) | 24.871 (32 lost), start 4 failed |
| Low-latency | no clean start (17–47 lost each, NEM) | 50.917, 54.791, 47.005, 61.388 clean; 51.194 (384 frames dropped) |

\* engine start 1 stepped from 33.870 to 43.870 ms (exactly +10.000 ms) at
impulse 301 with no lost impulse, dropped frame or route counter increase.

VB-Cable's latency alternates by about 1 ms inside a level (its own
timing), so the 1 ms segment tolerance splits it into many segments; the
per-start median is the comparable figure for both. Indicative comparison
(default periods, clean starts): the AudioRouter engine route, which
includes a route, is about 10–15 ms **lower** than VB-Cable's direct
Input → Output in this VM, so VCAB-25's "no more than 5 ms worse than
VB-Cable" holds here with margin. Low-latency periods cannot be compared:
the AudioRouter route has no clean start under NEM.

**Measurement finding (corrects the r8 reading):** VB-Cable start 1 shows a
first level of −3.9 ms, which is physically impossible. The probe dates
every impulse from one render-clock anchor taken at the end of the run, so
when the render side loses or inserts time mid-run (a VM stall), every
earlier impulse is dated wrongly by that amount. Levels before a disturbance
are therefore not reliable. In r8 the render-source lease reported 1,104
underrun frames (23 ms) at the stall, close to the 25 ms step; the earlier
conclusion that the extra delay sat in endpoint buffering is not
established. The step could be partly or entirely this dating error.
Clean single-level starts are unaffected. Next measurement repair: date
impulses from render-clock samples taken during the run (position and QPC
per period) and report render-side discontinuities, so a level step can be
attributed to the probe or to the route.
## Bundle r10 (in-run impulse dating)

Commit `a5ebd0f9`. The render thread samples `IAudioClock::GetPosition`
(stream frames and QPC) after every buffer into a preallocated vector; each
impulse is dated between the samples around its own frame
(`date_render_frame`), with the end anchor only outside the sampled range
(counted). Render stalls or skips over 2 ms are reported
(`cable_render_discontinuities`, at which impulse, how many ms), and the
old single-anchor median is printed for comparison. Pairing takes per-impulse
emission times (`pair_cable_impulses_dated`). Self-test 1,151 checks,
including the r8 shape with a true constant 50 ms and a 25 ms render
stall: the end anchor yields 25/50 ms levels, the in-run dating one 50 ms
level plus the stall report. The repeated-start runner marks clean starts
(no loss, no dropped capture frames, no render discontinuity) and reports
the median of their p50 values; acceptance 119 checks; the single-run
latency acceptance still passes 220.

Bundle `C:\VMs\ar-share\diagnostics-20261010-cable-latency-r10`, source
`a5ebd0f9`, driver `492d8ca8` (SYS SHA-256 unchanged), manifest SHA-256
`CCFADBF4D0A227313775D972CE13FEC142512BCCAF74C1A3E28F2E6E1922A394`.
## Repeated starts, both routes (bundle r10) — 2026-10-10, NEM VM, guest quieted

Archives (SHA-256 verified on the host):
`starts-engine-4918b63318ee41389be205abd51940e3.zip`
`86BA530332A2D257712BA50E3EB120A6485833A9941D94DD280207C0C465CEE7`,
`starts-vbcable-0b46ab5514504925ab84016b1d1c34bc.zip`
`5E619BC884B200C92532B36997AC1F4B6BB8C7D62B2A3A84CD339EACFB6D9C22`;
extracted to `target/starts-engine-4918b633` and `target/starts-vbcable-0b46ab55`.
All probes exited 0.

Default periods, per-start p50 (ms), in-run dating / end-anchor dating:

| Start | Engine route | VB-Cable Input → Output |
| --- | --- | --- |
| 1 | 39.27 / 39.77 (2 lost) | 54.84 / 48.36 |
| 2 | 63.90 / 40.40 (113 lost, 148 flagged) | 59.68 / 59.89 |
| 3 | 44.79 / 37.22 (4 lost) | 49.66 / 49.72 |
| 4 | **42.09 / 42.22 (clean)** | 60.64 / 60.75 |
| 5 | 39.10 / 39.32 (one 9.8 ms render step) | 44.23 / 43.58 |

Low-latency periods: the engine route lost 36–62 impulses per start
(NEM); VB-Cable lost none and measured 61–66 ms (in-run) / 51–72 ms
(anchor).

Reading:

- The two-sample stall detector misreads VB-Cable: its render position
  wobbles by 2–3 ms and snaps back (for example +2.9 then −2.9 ms one
  sample apart), so every VB-Cable start was flagged and none counted as
  clean, and two-sample interpolation carried the wobble into the dating.
  The engine route's position was smooth: its clean start had no flag, and
  both datings agreed (42.09 / 42.22 ms).
- With either dating, the engine route's default-period starts (about
  39–45 ms apart from the disturbed start 2) are not slower than VB-Cable's
  (44–61 ms). This repeats the r9 finding.
- The guest was noisier than in r9 (113 lost in engine start 2).

Repair (host, bundle r11): impulses are dated with the median per-sample
offset (QPC minus stream time) within ±0.1 s; a stall or skip is reported
only as a lasting step between the medians of 10 samples before and after
it. Raw clock readings and arrivals are saved per measurement and can be
re-analysed on the host (`cable-impulse-reanalyze`). Self-test 2,201
checks, including a VB-Cable-like wobble (no step, dating within 0.5 ms),
the r8 stall shape (one 25 ms step, every impulse more than 0.1 s from it
dated exactly) and a 20 ms forward skip; a synthetic raw file
round-trips to exactly 40.000 ms. Acceptance: repeated starts 144 checks,
single run 220.
Bundle r11: `C:\VMs\ar-share\diagnostics-20261010-cable-latency-r11`, source
`cc36d285`, driver `492d8ca8` (SYS SHA-256 unchanged), manifest SHA-256
`BFCA9B0837F1D14AE958551FF89A12217BF42DDD423F321DA8D5CEF4781C7C45`.
## Repeated starts, both routes (bundle r11) — 2026-10-10, NEM VM, guest quieted

Archives (SHA-256 verified on the host):
`starts-engine-ee54eaca00e1479b9587955163371eb9.zip`
`0353F8DF78EA212E3B1759429EE9C72C48F9B06DD8AA74B1613580EA410852E2`,
`starts-vbcable-ceb869418f6a425b9d66857abb3fe9c2.zip`
`37CE6778F05C9396CAC5B78190E0554981D59EEA47EAF2B77AD74C10990E53C6`;
extracted to `target/starts-engine-ee54eaca` and `target/starts-vbcable-ceb86941`,
with raw clock/arrival files for every measurement. All probes and routes
exited 0; engine silent quanta 0; corrupted impulses 0 everywhere.

Per-start p50 (ms, in-run median dating; end-anchor in brackets):

| Start | Engine, default | VB-Cable, default | Engine, low-latency | VB-Cable, low-latency |
| --- | --- | --- | --- | --- |
| 1 | **39.10** clean (38.89) | **46.20** clean (46.05) | 16.62, 26 lost (145.88) | 45.50, 3 steps (44.73) |
| 2 | 64.00: 34.38 then 64.45 from impulse 148, 2 lost (60.08) | **56.25** clean (56.10) | 16.73, 27 lost (136.00) | 62.81, 16 steps (25.40) |
| 3 | **43.98** clean (44.19) | **61.46** clean (61.27) | 18.22, 19 lost (135.53) | 59.60, 22 steps (39.81) |
| 4 | **38.04** clean (37.83) | **54.02** clean (54.02) | 17.34, 23 lost (81.13) | 56.67, 9 steps (49.19) |
| 5 | **36.42** clean (36.77) | **54.57** clean (53.54) | 18.13, 22 lost (3.23) | 59.15, 23 steps (31.70) |

Default periods (clean starts): engine route 36.4–44.0 ms, median of four
38.6 ms (38.04 lower middle); VB-Cable Input → Output 46.2–61.5 ms, median
54.6 ms. The AudioRouter route is about 16 ms faster than VB-Cable alone and
its median is within the 40 ms default target. The median-offset dating
fixed the r10 problem: all five default VB-Cable starts are clean, and on
clean starts in-run and end-anchor dating agree within about 1 ms.

Low-latency periods: the engine route's in-run p50 is 16.6–18.2 ms on every
start (target ≤ 20 ms), but no start is clean (19–27 lost impulses, capture
frames dropped, render steps) under NEM. The end-anchor values (3–146 ms)
are dating artefacts, so the earlier "low-latency p95 ≈ 137–150 ms"
readings (r5–r8) were mostly misdated, not route latency. VB-Cable's
low-latency render position shows real steps (3–69 ms in total per start),
so none of its starts is clean either; its p50 is 45–63 ms.

Open finding (engine start 2): the latency stepped up by 30 ms at impulse
148 and stayed there. The probe's render side was continuous (no render
step); the route counters show one late `SetWritePacket` and 1,440 extra
render-source underrun frames (30 ms) at the driver, and the probe dropped
480 capture frames. The Windows audio engine delivered render audio 30 ms
late; the driver played silence for that time and then played the late
audio as written (the r3/slot-provenance rule), so the delay persisted for
the rest of the run. Whether a cable should later shed such standing delay
(dropping audio to recover latency) is a design decision against
VCAB-24/25, not a defect fix; recorded for the latency-recovery decision.

## Bundle r12 — eight-cable sustained continuity preparation (2026-10-10)

Purpose: VCAB-24 step 3 deviation run on this one-PC VirtualBox NEM setup.
It runs four simultaneous AudioRouter engine routes (A→B, C→D, E→F, G→H)
and one identity-coded impulse train per route for 60 minutes. It is not
VCAB-24 qualification. The user decision in spec 17 remains: after this run,
first locate where the r11 standing delay is held, then design bounded,
counted latency recovery.

Source/implementation:

- Host runner `tools/vm/run-cable-continuity.ps1` and its host acceptance
  test were already committed in `24ab269a`.
- `7802c9b2` adds the runner to
  `tools/vm/prepare-cable-latency-update.ps1`, rejects a base bundle that
  already has it, documents the one-block guest procedure in
  `docs/operations/virtual-cable-latency.md`, and updates
  `docs/plans/active/virtual-cable.md`.
- Host command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
  tests/acceptance/m03-cable-continuity.ps1`. Result: **153 checks passed**;
  no audio stream, helper or driver tool opened. This exercises fake process
  boundaries, the real probe self-test and host refusal, not VM audio.
- `node tools/docs/validate.mjs`: passed, 147 Markdown files and 823 local
  links.

Bundle preparation, host PC, 2026-10-10:

```text
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/vm/prepare-cable-latency-update.ps1 -BaseBundle C:\VMs\ar-share\repair-20261010-slot-provenance -Destination C:\VMs\ar-share\diagnostics-20261010-cable-continuity-r12
```

Prepared at `C:\VMs\ar-share\diagnostics-20261010-cable-continuity-r12`
from clean source `7802c9b202b3a09c704757fa9d3dd5bac3057b92`. The bundle
manifest verifies 38 files, including `run-cable-continuity.ps1` and
`repo\tests\acceptance\m03-driver-vm-support.ps1`; manifest SHA-256:
`9318755a5c09837de1eaa491b347c159df273ceb5cd46acaa9c52709d96d644b`.
The static probe pairing self-test passed during preparation. The base and
new bundle `driver\audioroutervirtual.sys` hashes are identical:
`10cf8e879de85e3a924cdcc0dba2987d00cb8328dab9cba870b997f3ef952381`.
No driver was installed or loaded on the host, and no host setting changed.

### One-hour guest result — r12 (2026-10-10)

User ran the bundle in AR-DriverTest and copied
`continuity-e9eff1834e9a4634a761e38d8fee31ba.zip` to
`C:\VMs\ar-share\diagnostics-20261010-cable-continuity-r12`. SHA-256 verified
on the host: `6c216807e09e44f18111c3839d6434f1736d4812fddbf330032633a454bd8d83`,
matching the user's reported hash. The harness completed its 60-minute run;
exit codes passed, no impulses were corrupted, the saved cable count was
restored, and the result verdict is `unexplained-events` with
`Qualification=false`. This is a failed NEM deviation run, not VCAB-24
qualification.

| Route | Emitted | Lost | Loss runs | Capture dropped frames | Render steps | p50 latency | ≥20 ms progress intervals |
|---|---:|---:|---:|---:|---:|---:|---:|
| A→B | 359,621 | 378 | 176 | 286,944 | 67 | 61.752 ms | 57 |
| C→D | 359,621 | 345 | 173 | 283,104 | 61 | 63.029 ms | 57 |
| E→F | 359,620 | 364 | 177 | 291,168 | 62 | 62.758 ms | 53 |
| G→H | 359,618 | 369 | 179 | 290,304 | 65 | 61.951 ms | 55 |

Across routes, 1,456 of 1,438,480 emitted impulses were lost (about
0.101%); corruption and duplication were zero. The report recorded 705 loss
runs and 293 counter-increase events, with 434 of the 998 total events
classified VM-wide by the harness and 564 called unexplained. Despite that
strict label, losses cluster in the harness's nominal event dates: when bucketed into
fixed 100 ms windows, 136 of 245 occupied windows had losses on at least two
routes and 20 had losses on all four. This is evidence of shared timing
pressure, but exact cross-route synchronization needs QPC-clock dating before
attribution. It does not establish that each loss coincided with a ≥20 ms
measured gap in every route.

The raw probe clock streams also contain long gaps: 84–95 inter-clock gaps
over 20 ms per route, with maxima of 734–744 ms. That independently confirms
major pauses during the measurement, consistent with the route progress
reports' roughly 732–738 ms maximum pump gaps. This establishes scheduling
gaps, but the current output does not join each missing impulse index to the
clock gap and counter delta that caused it.

Interpretation: `Test-VmWideStall` calls an event VM-wide only when all four
one-second progress intervals contain a ≥20 ms maximum gap within ±1.5 s of
the event. Lost-event dates use launch wall time plus nominal impulse index
times 10 ms, rather than the saved render QPC clocks. Render skips, launch
uncertainty and long-run timing drift must be checked before interpreting
cross-route clusters or unexplained events. A shared burst can lose impulses on multiple routes and
still be classified unexplained if one route's sampled interval misses the
threshold/window. The harness classification is not a causal diagnosis.
Driver counters also show substantial render-source underrun/overrun and
capture-sink underruns on every route; engine processing itself reported zero
silent quanta. The run therefore does not support a driver-only or VM-only
root-cause conclusion. Preserve the counters and do not tune buffers or weaken
the event threshold from this run.

Next: inspect r12 raw impulse timing against each route's progress intervals
and packet/counter deltas, then use the already collected r11 raw data to locate
the standing delay before designing the approved latency-recovery behavior.
Do not repeat an unchanged hour-long run; if further VM evidence is needed,
first improve the report to correlate per-route impulse loss and counter deltas
with the measured pump gaps and shared cross-route loss windows.

### Work organization and offline replay (2026-10-10)

Both r11 archives were rehashed on the host and still match the SHA-256
values above. Read-only replay command (host PC):

```powershell
& 'C:\VMs\ar-share\diagnostics-20261010-cable-latency-r11\tools\m00-probe.exe' cable-impulse-reanalyze 'C:\code\audiorouter\target\starts-engine-ee54eaca\starts-engine-ee54eaca00e1479b9587955163371eb9\start2-default-raw.csv'
```

Exit 0. Reproduced 400 emitted / 398 matched / 2 lost, no corrupted or
duplicate impulses, zero render-clock discontinuities, initial level
34.379 ms for 145 impulses and level 64.446 ms from impulse 148 for 252
impulses. This reproduces the symptom; it does not locate the queued audio.
The [execution queue](../virtual-cable.md#execution-queue-after-r12-2026-10-10)
records inputs, task order, validation, rollback and the short guest-run gate.
No additional guest run is needed while these offline tasks are pending.

### QPC attribution repair and stage localization — r13 preparation (2026-10-10)

Offline review command, host PC (bundled Python 3 executable used):

```text
python tools/vm/review-cable-continuity.py C:\VMs\ar-share\diagnostics-20261010-cable-continuity-r12\continuity-e9eff1834e9a4634a761e38d8fee31ba.zip --probe target/m00-probe-cable/m00-probe.exe --driver-qpc-frequency 10000000 --output target/r12-qpc-review
```

The frequency is explicit: 10 MHz was established for this guest in the
prior scheduling trace. Progress snapshots in r12 lack normalized QPC
timestamps; the offline review anchors them to the latest driver counter
QPC and retains the existing ±1.5 s/20 ms correlation thresholds. The new
runner instead receives normalized QPC snapshots directly from the route.

| Route | Loss runs | Nearby all-four gaps (QPC) | Lost impulses without that correlation | Nominal timing drift |
|---|---:|---:|---:|---:|
| A→B | 176 | 146 | 33 | −4.808 s |
| C→D | 173 | 148 | 28 | −4.858 s |
| E→F | 177 | 149 | 30 | −4.827 s |
| G→H | 179 | 154 | 28 | −4.862 s |

All 1,456 lost impulses reproduced. The original report left 1,041 impulses
without all-route gap correlation; QPC dating leaves 119. This improves
the diagnosis, not continuity; it does not establish the cause of each loss.
The render-clock dating still assumes submitted payload frames correspond
to hardware-clock frames. Inspect that assumption before interpreting the
remaining events or sizing a recovery drop.

Stage localization from r11 engine default start 2's IEEE float32 stereo
bridge WAV (48 kHz): impulse 146, amplitude code 3/32, occurs at frame
71,104; next is impulse 148, code 5/32, at 73,024. Impulse 147 is absent.
Their spacing is 1,920 frames versus the 960 frames expected across two
impulse indices: an extra 960 frames (20 ms) already exists at Cable A's
bridge output. Later impulses retain that shift. The source WAV also has
the extra final impulse outside the probe's 400-impulse analysis window.
End-to-end dating shifts by 30 ms; this does not prove the driver holds a
30 ms queue. The new trace measures submitted payload/device positions,
source bridge observation and sink publication, including relay depth and
silence/drop counts, to locate the difference.

Changes: probe prints QPC loss/window dates and optionally stores `submit`
rows bracketing padding with clock samples. Route progress always includes
normalized QPC snapshots. Optional `--trace-route` records bounded,
preallocated render/capture impulse observations and fails on overflow;
only stopped workers' data is written. `-TraceRoute` enables both in the
short repeated-start runner and rejects missing traces. Raw write failure,
mixed clocks and incomplete loss reports fail instead of misleadingly
completing. Original latency statistics and counters are preserved.

Host validation (Windows, no endpoint/driver opened): 23 tone-tool tests;
2,203 probe self-test checks; 189 continuity-runner checks; 233 repeated-start
checks; two offline QPC regression tests; both formatting checks; workspace
and src-tauri Clippy passed. Rust emitted a filesystem hard-link cache
warning and copied cache files; no Clippy code finding. Static CRT builds
of probe/route tool passed. The first wiring check failed because trace
stdout preceded preserved progress output; output ordering was repaired
and all 23 tests passed. The first QPC fixture had PowerShell array
concatenation precedence wrong; the corrected fixture now covers the actual
QPC path and mixed-clock rejection. The no-loss sum was explicitly converted
to zero after the regression exposed an empty-sum/null mismatch.

Next: prepare a verified r13 bundle from clean identified source, keep
the signed driver byte-identical to the slot-provenance base, and give the
user the [short trace block](../../../operations/virtual-cable-latency.md#short-routesubmission-trace-after-r12).
Qualification remains false; kernel/audio recovery changes await this
measurement. Rollback: r12 tools and candidate `492d8ca8`.

Bundle r13 is now ready at
`C:\VMs\ar-share\diagnostics-20261010-cable-stage-trace-r13`, prepared from
clean source `a4949be87337384fdc47162f41fe113a9fce43ca`. Host preparation
command:

```text
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/vm/prepare-cable-latency-update.ps1 -BaseBundle C:\VMs\ar-share\repair-20261010-slot-provenance -Destination C:\VMs\ar-share\diagnostics-20261010-cable-stage-trace-r13
```

Static builds and portability/import checks passed; the manifest verifies
all 38 files. Manifest SHA-256:
`9f400d0e0ba7738d5ebece1beb3e42bd20bc858d5135d8cc3b89acade498c858`.
The packaged SYS SHA-256 matches the base:
`10cf8e879de85e3a924cdcc0dba2987d00cb8328dab9cba870b997f3ef952381`;
driver source remains `492d8ca82cac177d792fe0561003a8e157c4fab5`.
No installation or audio execution occurred on the host. Documentation
validation passed (147 Markdown files, 830 links), and `git diff --check`
passed. The exact next action is the user-run short trace block linked above,
about two minutes. Review its returned `starts-engine-*.zip` before changing
recovery or claiming improved audio continuity.

### r13 returned stage trace review — 2026-10-10

Original archive:
`C:\VMs\ar-share\diagnostics-20261010-cable-stage-trace-r13\starts-engine-49fccac967e041fc96099386a0e0b4cf.zip`.
Host `Get-FileHash` SHA-256 matches the user:
`741442A6B69C6DBC2B38F0231067934D7015DFB04A440E0BCA098236CE1BBE46`.
Working tree was clean before this documentation change. No host audio,
driver, VM configuration, security or power change was made.

| Start / mode | Lost / 400 | Median ms | p95 ms | p99−p1 ms | Before observed input | After sink publication |
|---|---:|---:|---:|---:|---:|---:|
| 1 default | 0 | 38.09 | 38.46 | 0.91 | 0 | 0 |
| 2 default | 1 | 38.01 | 38.40 | 0.95 | 0 | 1 |
| 3 default | 0 | 37.35 | 37.93 | 0.89 | 0 | 0 |
| 1 low latency | 19 | 17.24 | 18.11 | 13.54 | 5 | 14 |
| 2 low latency | 16 | 17.60 | 18.55 | 10.60 | 4 | 12 |
| 3 low latency | 17 | 16.26 | 18.91 | 11.44 | 5 | 12 |

Reproduction: extract a working copy with .NET `ZipFile.ExtractToDirectory`
to `target/r13-stage-review`; keep the original ZIP unchanged. Read each
`*-raw.csv`, `*-cable-a.route.csv`, `*-probe.txt` and source WAV. Decode
impulse codes as rounded peak × 32, compare input/publication code lists,
and reconstruct indices from their modulo-16 progression starting at code
1. Independently expand the probe's reported loss runs and compare the
missing-index lists. Inspect WAV RIFF chunks, require IEEE float32 stereo
48 kHz, and compare every nonzero pulse code with the render trace. The
local review script/output are `target/r13-stage-review/inspect.py` and
`review.txt`; bundled Python execution exited 0 after all comparisons.
An initial comparison compared loss-run starts instead of expanding their
counts; correcting that review error made every comparison agree. These
are offline original-data comparisons, not new audio execution.

Every route trace header reports `missed=0`; submission counts are
399/398/399 default and 1,483/1,486/1,488 low latency, with zero missed.
Source render pulse counts are 401/401/401 default and 396/397/396 low
latency; capture publication has exactly the same code order/counts.
Each stream includes the existing extra final impulse 400; all reported
loss counts above exclude it, matching the unchanged 400-impulse window.
Capture arrivals are 401/400/401 default and 382/385/384 low latency.

Low-latency source-missing indices are respectively
`6,8,58,62,335`; `34,146,232,361`; `79,120,131,134,230`.
Default start 2's missing impulse 102 is present at render input and at
sink publication (capture sequence 125, first-hit frame 160, QPC
101129667735). Sink underrun/overrun/sequence-gap counters remain zero in
that run; the probe reports 384 skipped capture-position frames. A zero
bridge counter is therefore insufficient evidence of delivered capture
audio. Low-latency capture-position gaps total 12,288/9,472/14,720 frames;
source underruns 9,040/7,312/10,464; late writes 83/69/90; render overruns
11/16/22 packets. Relay drops 14/7/9 blocks do not remove any observed
impulse in this sparse waveform; they must still be retained as failures.

Matched input-to-publication observation medians are 6.262/6.269/4.686 ms
default and 4.149/4.323/3.254 ms low latency, maxima under 10.007 ms.
These are worker observation intervals, not sample presentation latency.
Padding/clock sample brackets reach at most 0.434 ms; initial zero device
positions and changing submission/presentation offsets prevent treating
`device frames + padding` as an independently verified payload position.
Default levels remain stable; no +30 ms persistent transition occurred.
Low-latency discontinuity counts are 33/33/43, and jitter still fails.
No recovery size or owning kernel queue is established by this run.

Index reconstruction has a modulo-16 ambiguity if an entire 16-pulse cycle
vanishes. In these runs its missing lists match the independently dated
probe report, source WAV and trace counts. This supports the measured
stage boundaries; it does not distinguish the Cable B DMA, Windows audio
engine and diagnostic capture reader as the downstream cause.

Next task (VCAB-20/24/25/28, VDEV-12): inspect packet retention and
`GetReadPacket` delivery under late audio-engine wakeups, and the probe's
render/capture worker scheduling. Code inspection shows the probe uses
ordinary `std::thread` workers without the route tool's MMCSS registration;
this is a diagnostic scheduling risk, not proof that it caused the loss.
Model sample preservation and skipped packets before a kernel change;
prepare a focused observation/repair only when it distinguishes those
layers. Keep the current candidate and all loss counters. No new guest
command or unchanged hour run is needed now. Rollback is documentation
only; VCAB-24/25 qualification remains failed/open.
