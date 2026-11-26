# Plan: one driver, signed once, built to last

Status: **accepted 2026-11-25**; the owner answered decisions 1, 2 and 5 (section 4). No
driver source has changed. This plan extends the archived
[virtual cable plan](../archived/2026-11-24-virtual-cable.md); owner-run tasks stay in
[human-gated tasks](human-gated-tasks.md) (D1-D6, B1) and are cited by ID.

## 1. Objective

Enter signing (OSSign, earliest **2027-03-05**, [B1](human-gated-tasks.md#b1-signing-certificate-identity-and-key-storage))
with a driver we do not expect to change: more cables, the best performance we can
prove, and a stated statistical certainty about long running. All kernel changes go
into **one** release candidate, **0.3.0**, because any edit restarts Stage A anyway.

Requirements: VCAB-01/02/05/20-30, VDEV-01/04/07/09/10/12, SEC-08, NFR-02/16,
ENG-02/04. Design: [spec 17](../../spec/17-virtual-cable.md).

## 2. Where we really stand

| Belief | Evidence on 2026-11-25 |
| --- | --- |
| "Faster than VB-Cable" | **Indicative only.** VM, r11, older driver, default periods, few clean starts: 38.6 ms against 54.6 ms median ([r9-r11](evidence/2026-10-10-m03-cable-latency-preparation.md)). AudioRouter was not slower even with a route in the path. Low-latency periods had no clean start. Never measured on 0.2.5, never on real hardware. The VCAB-25 low-latency target is 20 ms p95. |
| "Over 8 hours of driver tests overnight" | **Not yet.** The 8-hour soak (C3) tests the *app* on VB-Cable, and is open. The longest tagged driver memory run is 60 min (pass). The eight-cable run is 30 min with 4 unexplained events ([gap analysis](evidence/2026-11-21-virtual-cable-stage-a-0.2.5-gap-analysis.md)). The 24 h pool run (VCAB-28) has not been done. |
| "More cables cost nothing" | Money: none. Engineering: a new RC, memory, tests (section 5, W2). |
| "0.2.5 is nearly done" | Stage A is **not passed**: Verifier freeze (D2), frame-clock A/B (D3), helper exit 5 when updating or removing under a live route. |

The VM cannot prove timing: the guest ran at about 70 % of real time with eight
cables and freezes under host load. This is the largest risk to "sign once",
and decision 2 below accepts it for now.

## 3. Principles

1. **Raise ceilings now, choose values later.** A fixed limit that costs only static
   tables is set generously; a value that might need tuning is read from the registry
   within a fixed range. The app then changes without a new signature.
2. **Freeze behavior, not ambition.** A kernel change is accepted only for a defect, a
   limit that cannot be raised later, or a measured gain. No speculative features.
3. **Every claim has a number and a confidence bound** (section 6).
4. **No unexplained event.** A failure is fixed, or attributed by trace, or it blocks signing.
5. One change per commit, with host tests; one revert must be enough to drop it.

## 4. Owner decisions

| # | Decision | Recommendation |
| --- | --- | --- |
| 1 | Maximum cables | **Decided 2026-11-25: 16** (A-P), default 2. Same one-letter bus ID; final for this driver. |
| 2 | Real hardware **before** signing | **Decided 2026-11-25: use the VM for now**, so an agent can run the long tests. No test PC is bought. Consequence: timing results before signing are indicative (section 6). Ask again at the January 24 freeze, with the campaign data so far; a test PC stays the only way to get binding timing before signing. |
| 3 | Frame-clock change (D3) | Decide from a quiet, repeated A/B; on a tie, keep the simpler arithmetic with the better host proof. |
| 4 | Update or removal while audio plays asks for a restart (exit 5) | Find out why in W1. Accept it if Windows holds the image because an app has an endpoint open, since then no driver can avoid it. |
| 5 | Freeze date | **Decided 2026-11-25: 2027-01-24.** Features not ready by then are dropped. |

## 5. Work packages

Order: W0 and W1 start now; W2-W4 host work runs in parallel, but **no driver source
changes before W0 is approved and D2 has an answer**.

### W0. Irreversibility inventory (agent, host, no VM)

Result: [inventory](driver-irreversibility-inventory.md), approved 2026-11-25. Driver edits still wait for D2.

List every compile-time or wire-level limit, with cost now, cost later, who may want
more, and a recommendation. Start from:

| Item | Today | Initial view |
| --- | --- | --- |
| Cables / lease slots | 8 / 16 | 16 / 32 |
| Cable bus ID | one letter, exactly 7 characters | keep |
| Channels | 8 | keep; 16 multiplies payload and tests |
| Rates, formats | 44.1/48/96 kHz; 16/24/32-bit, float | **decide 88.2, 176.4 and 192 kHz**: cheap to list, but each adds tests |
| Frames per quantum | 4096 | keep |
| Render queue | up to 8 blocks, shared slot of 1 block | decide, see W3 |
| Capture queue | 2 blocks | **decide**: 10 ms at 96 kHz is the thinnest margin ([deferred study](../future/virtual-cable-queue-depth.md)); its trigger "a kernel change for another reason" is now met |
| Queue depth setting | compiled | candidate: registry value inside a compiled ceiling |
| Minimum period | 64-480 frames, default 128 | keep |
| Protocol | 1.1, OPEN flags/reserved, QUERY `Reserved[16]`, header reserved to 128 bytes | prove unknown flags are refused and that reserved space suffices for counters wanted in W5 |
| `SilenceOnStaleMs`, `DefaultPeriodFrames`, cable names | not read / reported only | confirm intended |
| Sample-code leftovers | spec 5.1 says remove; `savedata.cpp`, `micarraytopo.cpp`, `speakertopo.cpp` are still compiled | prove unreachable, then remove |
| Windows floor | 22000 (Windows 11) | keep; no Windows 10 |
| ARM64 | builds only | keep out of this signing unless you ask |

Exit: the owner marks each row, the spec records the decisions (VCAB-01, the VDEV-01
deviation, section 1 non-goals, VCAB-24 wording).

### W1. Clear the open blockers (VM, owner D1 first)

1. D2: freeze reproduction, 5 boots on an idle host. Also run the same steps with **no
   driver** as a control, so a freeze can be attributed. Analyse `guest-core.elf`
   (owned by the agent; needs a converter, not yet tried).
2. D3: frame-clock A/B, repeated with fresh restore between drivers and fewer runs per boot.
3. Exit-5 cause: which handle holds the image. Tests with no endpoint open, with a WASAPI
   client only, and with an AudioRouter route only.
4. The four unexplained eight-cable events: reproduce with the scheduler trace, or state
   plainly that only hardware can answer.

Exit: D2 clean, or a defect fixed in 0.3.0. Nothing else is built on an unexplained freeze.

### W2. More cables (kernel, app, tools)

Change together, from one source of truth:

- Driver: `AR_BRIDGE_MAX_CABLES`, the endpoint enum, `DEFINE_CABLE_PAIR` and its
  tables, the INF (generated from `tools/m03-inf-gen/cables.json`, **new GUIDs for I-P,
  A-H GUIDs and filter names unchanged**), registry range of `CableCount`.
- App: `MAX_CABLES` in `driver-helper`; `(1..=8)` in `virtual_cable_shell.rs`;
  `cable-[a-h]` in `api_output_schema.rs` (three places), `cable_routes.rs` and
  `m03_cable_session.rs`; "Cable A to Cable H" texts; UI, MCP and REST text; guides;
  `max_cables: 8` and `MaxCables == 8` tests. Use one domain constant, pinned against
  the C header by a test, and limit by `min(app maximum, QUERY MaxCables)` so an older
  driver still works.
- Scaling audit: the lease-table walks found so far in `adapter.cpp` are handle cleanup,
  OPEN and shutdown, and stream lookup is by index; confirm none is on the per-millisecond path. Idle cables must cost no timer or DPC.
- Memory: each open stream holds two 256 KiB nonpaged arrays, so 16 cables streaming
  both ways is about **16 MiB** (8 MiB today). Measure; shrink if a smaller bound is safe.

Tests: `1, 2, 8, 9, 15, 16` cables (off-by-one and letter boundaries), every letter
routes to itself and no other (crosstalk VCAB-26 over all pairs), count up and down with
apps attached, endpoint settings kept for the cables that stay, reboot, upgrade
0.2.5 to 0.3.0 and back, a device count of 17 refused, boot and device-restart time
recorded, Discord and OBS lists. The VM runs 16 at once for
stability and memory; its continuity result is indicative until Stage C.

Gate (the owner chose 16): if 16 fails the memory, boot or restart bound, report to the owner
and propose a lower number; never ship an untested one.

### W3. Best performance (measure, change, measure)

1. **Method first.** One harness, same PC, same session, interleaved order, at least 20
   fresh starts per product and mode: AudioRouter cable with a route, VB-Cable CABLE
   Input to Output, VB-Cable A/B, default and low-latency periods, idle and 80 % CPU.
   Report p50/p95/p99, jitter (p99-p1), drops, CPU and DPC time, with confidence intervals.
   Existing: `run-cable-latency-starts.ps1`, `run-cable-vbcompare.ps1`.
2. **Budget the path.** Split latency into Windows engine, driver render, bridge slot,
   app, bridge slot, driver capture, Windows engine. Publish what the driver can and
   cannot improve; Windows' shared engine is most of the remainder.
3. **Candidates** (each accepted only with a measured gain and no continuity loss):
   capture queue depth; queue ceilings configurable; a multi-block shared ring instead
   of the single slot; event-driven wake instead of the client's timing wait; 1 ms service
   tick against a period-aligned tick; per-stream timers against one shared timer at
   many cables; DPC cost per stream.
4. **Targets:** VCAB-25 p95 at most 20 ms low-latency and 40 ms default, jitter at most
   2 ms; **stretch: strictly below VB-Cable in every mode** (spec allows +5 ms); VCAB-27
   DPC below 20 us mean and 200 us max; bit-exact VCAB-20; no added distortion.

The shared ring is the biggest one-way door: it changes the wire layout, allowed for
free only before the first release. **Spike it on the host against the timing model
first**; keep the single slot if the measured benefit is under 2 ms or one extra
stall of tolerance.

### W4. Correctness and security hardening (aimed at never changing again)

- Static: CodeQL (Microsoft driver must-fix suite), Code Analysis, SDV or its
  replacement, `infverif /h`, HVCI section scan, on every driver commit in CI.
- Dynamic in the VM: standard Verifier plus IOCTL fuzz, low-resources simulation,
  systematic concurrency where supported, special pool, pool tracking, DMA checks.
- Structured IOCTL and shared-memory fuzzing with a **hostile user-mode writer** on the
  section (double fetch, sequence abuse, unmapping during use, handle close races).
- Fault injection: kill the app at every state, revoke leases, session switch, second
  user, low memory, surprise removal, sleep during streaming.
- Independent fresh-context review of the complete driver, twice: at RC and after the last fix.
- HLK audio tests in the VM where they run; record which need hardware. Attestation does not need them.
- Arithmetic proofs for time and position: 64-bit wrap, 100 days uptime, QPC extremes,
  32-bit packet wire number wrap (about 1.4 years at 10 ms packets, but about
  **66 days** at the 64-frame minimum period); host tests drive them with simulated clocks.

### W5. Stability certainty (the long-run campaign)

**Two separate tracks** in the same VM, judged differently:

| Track | Where | Examples |
| --- | --- | --- |
| Churn (load tolerant, counts) | VM, nightly | open/close leases, start/stop streams, rate, format and channel changes, count changes, app kill and restart, install/update/remove, reboot, default-device changes, Verifier fuzz |
| Time (real-time) | VM, unattended nights, host idle | 2 then 16 cables, idle and 80 % CPU, sleep/resume, device plug/unplug, game-like load |

Every run writes one JSON record: SYS hash, git commit, machine, duration, uptime,
stream-hours, host-load verdict, per-counter deltas (`UnderrunFrames`, `OverrunFrames`,
`SequenceGaps`, `NonFiniteSamples`, `FormatMismatches`, packet late/overrun), tone
discontinuities, tagged pool first/last/slope, handles, DPC max, bugchecks, verdict
(`pass`, `fail`, `invalid` with reason). An invalid run adds **zero** hours and stays visible.
A scoreboard script aggregates by SYS hash, and **any driver change resets the score**.

Build in order: (1) record schema and aggregator with tests; (2) churn runner over the
existing VM tools; (3) time runner with the continuity and latency checks; (4) nightly
schedule, one unattended batch, host preflight gate; (5) weekly owner summary.

## 6. Metrics and gates

For zero failures in *T* valid hours the 95 % upper bound on the failure rate is 3/T
(exact Poisson, `-ln 0.05 / T`). Count **machine hours**, not stream-hours, because streams
on one machine share failures.

| Gate | Required | What it proves at 95 % confidence |
| --- | --- | --- |
| G1 Churn | 10,000 lease and 10,000 stream cycles, 1,000 app kills, 300 count changes, 100 install/update/remove, 100 reboots, 0 failures | Failure below 0.03 % per lease/stream cycle, 0.3 % per kill, 1 % per count change, 3 % per install or reboot |
| G2 Verifier | 10 consecutive sessions end cleanly, 20 h fuzz total | About 26 % per session upper bound; stated honestly as weak |
| G3 Memory | 72 h at maximum cables, tagged pool slope at most 0.25 MiB/h and flat steps | VCAB-28, three times the 24 h requirement |
| G4 Continuity | Target 300 valid hours in the VM, zero unattributed glitch or counter increase, including one 72 h run and eight nights of at least 8 h | Under 1 event per 100 h |
| G5 Pause/resume | 100 VM pause/resume cycles while streaming, 0 failures | Under 3 % per cycle |
| G6 Load | G4 conditions at 80 % CPU, 24 h | VCAB-30 |
| G7 Performance | W3 targets, VB-Cable comparison on the same PC | VCAB-25/27 |
| G8 Quality | bit-exact matrix 3 rates x 1/2/8 ch x each letter, THD+N, sweep, isolation | VCAB-20-23/26 |

**VM rule for the time gates (decision 2).** An hour counts only when the host-load verdict is
idle and no audio was lost. A stall that the cable absorbed without loss stays valid (the soak records stall
intervals but runs no scheduler trace for hundreds of hours). A failure inside a measured guest-wide
stall makes that hour *invalid*, never a pass; a failure outside one is a driver failure until
explained. The scoreboard reports valid, invalid and failed hours side by side, because a high
invalid share could hide defects: **above 20 % invalid, the gate is inconclusive**.
G4, G6 and G7 may therefore end `VM-indicative`, not `passed`; DPC and absolute latency are
not claimed at all from the VM. G5 becomes pause/resume plus guest reboot cycles; real sleep
waits for Stage C. Stage C on the owner's PC with the signed package stays the binding check,
so a Stage C miss can still mean a second signing.

**Feasibility (owner schedule, 2026-11-25).** Weekdays 22:30 to 15:00 is 16.5 h, 82.5 h a week, about
412 h in the five weeks from January 24 to February 28. 300 valid hours fit if at least 73 % of that time is
valid and nothing resets the score. G1 and G2 use the weekends. The longest gap with the
evenings free is Friday 22:30 to Monday 15:00, 64.5 h, so the 72 h run **needs the owner to give up one evening**;
proposal: G3 and G4 share that single 72 h run at maximum cables. Control: [driver soak](../../operations/virtual-cable-soak.md).

**Main risk to G4.** None of the last ten recorded VM continuity runs (10-60 min, four routes, 2026-10-10 to
2026-11-12) was clean: the best were vm-correlated, most had unexplained events, and one 0.2.5 run delivered only
70 % of real time. They predate the idle-host gate and the two-route load. The first rehearsal nights on 0.2.5 decide
whether the VM can produce valid hours at all; if they cannot, decision 2 (real hardware) must be taken again
before the freeze, not after. Rehearsal hours never count for 0.3.0. If fewer hours are reached, report the bound
for those hours (100 h: under 3 per 100 h; 200 h: under 1.5).

Failure policy: first failure stops the campaign for that gate; fix, new version,
reset the score. At most two resets before re-planning, because more means the design,
not the luck, is wrong.

## 7. Schedule (targets)

| Dates | Work |
| --- | --- |
| now - Dec 7 | W0; D1; D2; W5 schema and scoreboard; W3 harness |
| Dec 7 - Dec 31 | W1 ends; W2 and W4 static work on host; W3 spikes; churn runner |
| Jan 1 - Jan 23 | 0.3.0 built (PDB kept), host and VM Stage A subset, independent review |
| **Jan 24** | **Freeze 0.3.0**; hash recorded; scoreboard starts at zero |
| Jan 24 - Feb 28 | Full Stage A, G1-G8 campaign nightly; second review |
| Mar 1 - Mar 4 | Evidence bundle, reproducible build from a clean clone, go/no-go |
| **Mar 5** | Apply to OSSign ([checklist](../future/M03-driver-signing.md#ossign-reply-2026-11-25-ticket-991393)) |

If G4 needs more hours, or a reset lands after Feb 10, the application slips; the rule is
quality first.

OSSign builds from public source, so signed bytes will differ from ours:

- build from a clean public clone with a pinned WDK/compiler, documented so a third party can repeat it;
- compare sections and imports, keep the PDB with the SYS, map every signed function to the tested source;
- run a Stage A subset on **OSSign's test build** before submission, and Stage B/C on the signed package;
- ask OSSign what a later resubmission costs (no cost figure is known).

## 8. Validation matrix

| Change | Host (agent) | VM (agent after D1) | Stage C, after signing | Owner |
| --- | --- | --- | --- | --- |
| W2 cables | unit, INF generator, ABI pin, UI/REST/MCP checks | enumerate, route, count, upgrade | 16-cable continuity | Discord/OBS look |
| W3 performance | timing model, sim clocks | indicative | binding | reads the table |
| W4 hardening | CodeQL, Code Analysis, model tests | Verifier, fuzz, HLK where possible | HLK rest | - |
| W5 stability | schema/aggregator tests | churn and time tracks | repeat time track | leaves PC idle at night |

Required by AGENTS.md: REST and MCP text stays true, guide pages and their videos follow
any changed control, three-theme check of the Setup cable picker, `source_combinations`
and `tool_combinations` suites run for 9-16 cables.

## 9. Risks

| Risk | Response |
| --- | --- |
| A freeze is a real driver defect | W1 first; it blocks everything |
| Bare-metal numbers arrive only after signing (accepted for now) | VM time gates, deep timing models, registry-tunable values in W0/W3, revisit at the freeze |
| The plan adds changes faster than it removes risk | Freeze date; drop, never extend |
| Verifier or NEM noise hides a defect | Control runs without the driver; host-load gate |
| OSSign declines or asks for changes | B1 fallback (own EV) reuses the same RC |
| Attestation policy changes | Ask in writing; HLK results keep WHQL open |
| More cables slow Windows audio enumeration for every user | Default stays 2; measure only, cap offered by the app |

## 10. Rollback

0.2.5 sources (`7c88895c2`) stay in Git history and buildable (tag it only if you want one). Each W2-W4 change is its own commit
and can be reverted before the freeze; after it, only defects are fixed, as 0.3.1. The
app reads the driver's QUERY `MaxCables`, so a new app keeps working with a 0.2.5 driver and its eight
cables. The reverse is not safe yet: rolling back from more than 8 cables to 0.2.5 leaves only cables A-B
(inventory row A14, open, to be settled before the freeze).
Users without the driver keep VB-Cable.

## 11. Progress and next action

2026-11-25 (host only; no VM started, no driver source changed):

- W0: [irreversibility inventory](driver-irreversibility-inventory.md) **approved by the owner as recommended (2026-11-25)**; the owner also gives up one evening for the shared G3/G4 72 h run.
- W5 step 1 and the time runner: `tools/vm/soak*.ps1`, `soak.cmd`, `prepare-soak-bundle.ps1`; bundle
  `C:\VMs\ar-share\rc-0.2.5-soak` (0.2.5, rehearsal, uncommitted scripts). `run-cable-continuity.ps1` gained
  `-RouteCount 2|4` (default 4, unchanged).
- Checks run on the host, Windows PowerShell 5.1, all pass: `soak.tests.ps1`, `soak-guest.tests.ps1` (the real loop
  with stub audio tools), `m03-cable-continuity.ps1` (202), `round2-support.tests.ps1`. The real `soak.cmd start` was
  run and correctly refused (9.9 GB free, WSL running). **Not run:** anything inside the VM.
  `tests/acceptance/m03-cable-quality.ps1` fails on this PC (`IO.Compression.ZipFile` not loaded), also without these changes.

The soak is fixed at eight cables and 2 or 4 routes; W2 must extend it to 16 before any 0.3.0 hour counts.

**W2, app side** (branch `feat/cables-16-app`, 2026-11-25, no driver source changed):

- One app maximum, `audiorouter_domain::MAX_CABLES = 16`, with `cable_bus_index`/`cable_bus_id`
  for `cable-a`..`cable-p`; a test keeps `AR_BRIDGE_MAX_CABLES` (still 8) between 8 and 16. Make it
  an equality in the driver commit.
- **Safety finding:** the 0.2.5 driver reads an out-of-range `CableCount` as 2, so writing 9 would
  silently remove cables C-H. The helper now takes the driver's capacity from the bridge QUERY
  `MaxCables` (8 when the bridge does not answer, never more). `set-cables` refuses a larger count
  before any write, and the status reports the count the driver really uses. Install and update
  wait for the new driver's QUERY before they count cables.
- `configure --name` refuses a cable beyond the driver capacity too.
- `virtualCable.status` gains `maxCables` (schema, contracts, API reference, MCP text). The Setup
  picker offers 1 to `maxCables`. The shell and the schemas accept up to 16, and spec 17 VCAB-01
  says sixteen. Section 5.1 of the spec (driver tables) changes with the driver commit.
- Checks (host, 2026-11-25): `cargo test --workspace --no-fail-fast` (all pass after the fixes;
  `siege_round` feed test failed once and passed on two reruns, unrelated), Clippy for the workspace
  and `src-tauri`, both `cargo fmt --check`, `src-tauri` cable tests, UI typecheck, format and lint,
  `VirtualCablePanel.test.tsx` (22), `e2e/virtual-cables.pw.ts` in the three themes (4), contract
  drift, docs validation, `m03-driver-vm-guards.ps1` (547).
- Not exercised: cables 9-16 on a real driver (none exists yet). The INF generator,
  `m03-inf-gen.ps1` and `m03-driver-build.ps1` still count 8 and change with the driver commit.
  A direct bridge OPEN of `cable-i` against 0.2.5 reports "driver unavailable", not "cable not
  enabled"; only the route compiler opens bridges, and it checks QUERY first.

**W2, driver** (branch `feat/driver-16-cables`, 2026-11-25; the owner asked to raise the driver
to 16 now, ahead of D2):

- `AR_BRIDGE_MAX_CABLES = 16`. Cables I-P were added to the endpoint enum, the minipair
  tables and the generated INF, with new GUIDs. A-H keep their GUIDs, filter names and endpoint
  names, and the INF diff only adds lines. `C_ASSERT`s tie the tables to the constant, and the
  domain test now requires `MAX_CABLES` to equal the header value.
- Host checks: driver unit tests pass (10,915,271 checks). The ASan variant cannot link on this PC
  (`clang_rt.asan_static_runtime_thunk` missing). `m03-inf-gen.ps1` passes (160 interfaces),
  `infverif` reports the INF valid, the domain tests pass, and the soak tests pass.
  `m03-driver-build.ps1` was already stale after `c183b09c0`. Two of its checks were repaired, and
  it still fails at a check of `adapter.cpp`, which this change did not touch.
- VM share `C:\VMs\ar-share\rc-0.2.7-r2` (commit `c6ed3b79b`) holds a test-signed driver 0.2.7
  and 0.2.8 for the upgrade step, both built from the same source. The guest `cables` step now
  goes 2 -> 8 -> 9 -> 16 -> refused 17 -> 2 and checks that the A-H endpoint IDs are kept from
  8 to 16. The 0.2.5-pinned Stage A bundle does not apply. The 16-cable soak bundle still needs
  `m00-probe.exe` from an audio-tools update.
- **Not run:** anything inside the VM. Blocked by D1.

Owner: D1, then the first rehearsal night. Agent: read its records, then the churn runner.
Rollback: delete the soak files and revert `-RouteCount`; nothing else depends on them.
No pull request, purchase or driver edit is authorized here.
