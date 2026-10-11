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