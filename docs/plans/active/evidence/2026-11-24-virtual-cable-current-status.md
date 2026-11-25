# Virtual cable: current status and the one remaining stage (2026-11-24)

Scope: host-side analysis and host tooling only. No VM was started, no driver was
loaded or installed, no host driver/security setting changed, and no kernel source
was edited (the 0.2.5 package stays frozen). Requirement IDs: VCAB-20/24/25/27/28/30,
VDEV-09, SEC-08, NFR-02. Plans: [virtual cable](../virtual-cable.md),
[review fixes](../virtual-cable-review-fixes.md).

## 1. Verified state (checked at this commit, not copied from old notes)

- Driver source untouched since the freeze: `git log 7c88895c2..HEAD --
  drivers/audiorouter-virtual` is empty. Package identity matches the 2026-11-21
  gap analysis (SYS `D2C26DD9…`, CAT `12F48561…`, INF `AE3212D0…`); still **no PDB**
  for the SYS.
- Stage A on 0.2.5 is **not passed**; the 2026-11-21 gap analysis governs. Round 2
  ran 2026-11-21/22: A/B FAIL by the pre-registered rule but confounded, tagged
  pool PASS, Verifier freeze reproduced (core shows nothing stuck in the driver),
  update/remove under a live route REVIEW (helper exit 5, restart required).
- The round-2 guest wrapper exists and is host-tested: `run-stage-a.ps1 -Session
  round2` with `round2-support.ps1`, `round2-restart-render.ps1`,
  `round2-pause-watcher.ps1`, `prepare-round2-bundle.ps1` and
  `tools/vm/round2-support.tests.ps1` (host, passing). Blocks `ab, pause, live,
  pool, verifier, control, repro` exist.
- The 2026-11-22 freezes are host-load dependent: a fresh guest with no driver
  and no Verifier froze for minutes while the host had 332 MB free. Every freeze
  from now on needs the host sampler, and a night on a busy PC decides nothing.
- The old 2026-10-28 header line "next authorized non-driver implementation:
  WP-09 mixed-clock" is satisfied by later work, not pending: physical and cable
  sources compile and run together (`crates/engine/tests/source_combinations.rs`
  pairs and triples into a Mixer, `tool_combinations.rs` processors in those
  layouts), and `crates/control/src/goals.rs:86,114` defines the Mic-to-Discord
  and Game-to-Discord goals. Reimplementation would duplicate shipped work.

## 2. Review-fix items confirmed in the frozen source and its host tests

Host suite `drivers/audiorouter-virtual/tests/build-tests.ps1`: 10,915,246 checks
pass (run 2026-11-24). Rust R1 regression passes:
`cargo test -p audiorouter-windows-audio driver_owned_session_survives_a_stall_longer_than_its_lease`.

| Item | Code (file:line) | Host evidence |
| --- | --- | --- |
| 1 every IRP major function through one wrapper | `Source/Main/adapter.cpp:1155,1159,1577` | compile |
| 2 push lock inside a critical region | `Source/Main/adapter.cpp:98` | compile, Code Analysis clean (2026-11-11) |
| 3 control device at StartDevice, deleted at stop | `adapter.cpp:1212,1220,1235` | compile; VM: R2 stale-open refused twice |
| 4 owner renews its own expired lease; unbiased time | `Source/Inc/bridgeio.h:517`; `maintenanceDecisionChecks` | 10,915,246 pass |
| 5 render queue depth 8, capture stays 2 | `Source/Inc/renderqueue.h:4,38` | queue-depth checks |
| 6 frame clock from elapsed 100 ns with remainder | `Source/Inc/streamtiming.h:133`; `frameClockChecks` | 1 h drift-free 44.1/48/96 kHz, sub-ms calls |
| S1 sample encoding resolved once | `minwavertstream.cpp:251,318`; `sampleEncodingChecks` | 10,915,246 pass |
| S2 publish-side finite scan removed | `minwavertstream.cpp` publish path | 10,915,246 pass |
| S3 DMA view cleared under the position lock; R3 zero guards | `minwavertstream.cpp` `DetachDmaBuffer`/`GetReadPacket` | compile, Code Analysis clean |
| S4 unused `m_pDpc`/`m_pTimer` removed | `minwavertstream.h` | compile |
| R1 client defers the lease to the driver | `crates/windows-audio/src/lib.rs:1430,10359` | Rust regression above |
| R2 accept flag is per control device | `adapter.cpp:48,848,1235` (`AcceptingOpens`) | VM: old handle OPEN refused (1167) |
| R6 generation history, A-B-A refused, backward step allowed | `bridgeio.h:549,560`; generation checks | 10,915,246 pass |
| Probe IRPs and stale handles | `tools/m03-bridge-fuzz/bridge_fuzz.cpp:263,290` | VM: `--probe-io` PASS, old handle 1167 |

Not a quality item but noted: `tests/bridge_tests.cpp:866-871` repeats three
generation checks; harmless in the frozen tree, left untouched because the test
tree belongs to the frozen driver folder.

## 3. Requirement-to-evidence map (what a person still has to answer)

| Req | Evidence that exists | What is still missing |
| --- | --- | --- |
| VCAB-24 (zero glitches, 1 h, 8 cables) | VM eight-cable 30 min 2026-11-12: 4 unexplained events, 0.2 % impulses lost, guest user mode at ~70 % real time; crosstalk PASS | Zero-counter evidence on bare metal / native VT-x with 8 cables for 1 h (VM/NEM cannot deliver it). Stage C, signed package. |
| VCAB-25 (latency p95, VB-Cable ratio) | VM p50 36.6 ms, jitter 0.022 ms (2026-11-12); ratio never measured (VB-Cable not installed) | VB-Cable in the guest or Stage C: impulse p95/p99 and the VB-Cable ratio on the same PC; NFR-02 microphone route. |
| VCAB-27 (DPC/CPU budget) | No driver timers/DPCs for idle cables by source; no WPR numbers | Windows Performance Recorder on the reference PC: per-stream DPC average < 20 us, max < 200 us; backend <= 3 % of one core with 8 cables. Hardware. |
| VCAB-28 (pool constant, no per-stream allocation in callbacks) | Tagged driver pool PASS 2026-11-21: MNWR/MNTR/SyAu/AUDR flat at 524 KiB, 28 allocations, 0 MiB/h over 60 min; callback path allocates nothing (source) | 24 h of streaming (spec) has not been run; 60 min is the longest tagged measurement. Owner decision: a 24 h VM run or fold it into the Stage C long session. |
| VCAB-30 (stable under load) | Not run | VCAB-24 with ~80 % CPU stress and device plug/unplug, bare metal, signed package. Hardware/attended. |

## 4. Host tooling added today (agent work, tested on this PC)

Objective: make the next VM night decidable. A freeze on a busy host is not a
driver result (2026-11-22 evidence), so the run now refuses to start without host
measurements and records them per block.

- `tools/vm/host-load.ps1`: parameterised sampler (`-OutputPath`, `-Minutes` up
  to 2880, `-IntervalSeconds`, `-Once`); full UTC timestamps and six columns:
  total CPU, non-VM CPU, the VirtualBox processes' CPU and memory. One row per
  interval to the share root `C:\VMs\ar-share\host-load.csv` (guest:
  `Z:\host-load.csv`). It shares its measurement with the preflight.
- `tools/vm/round2-host-preflight.ps1`: host idle gate. 10 s measurement, WSL2
  host-process check, heaviest-process report, starts the sampler for 26 h when
  no fresh one exists, exit 0 ready / 2 refused. `-LibraryOnly` makes it
  testable. On 2026-11-24, with the release guest running, it measured total CPU
  peak 22 % but mean non-VM CPU 3.4 %, and refused on host memory (median
  5.5 GB) and WSL. The old total-CPU rule would have called that starvation; the
  new one does not.
- `tools/vm/round2-support.ps1`: `Get-HostLoadRows` (4- and 6-column),
  `Get-HostLoadWindow`, `Get-HostLoadVerdict`. CPU is Starved only for sustained
  non-VM load (mean > 20 % and at least half the samples above it, over two or
  more samples); the guest's own vCPUs do not count; memory uses the window
  median against 6000 MB; fewer than two samples is NoData, never Idle or
  Starved. The owner's 20 %/6000 MB limits are unchanged.
- `tools/vm/run-stage-a.ps1`: round 2 now runs `host-load sampler` first and
  stops with FAIL/SKIP when the CSV is absent or not growing; each block records
  `host load (<block>)`; the state file keeps `hostRows`; an unexpected restart
  records `host load (<step>, until the restart)`, positioned by row count, never
  by comparing the guest and host clocks (they differ by hours). `-PoolMinutes`
  accepts 5-1440 (2 h default) for a 24 h VCAB-28 run, and the pool block copies
  `pool-tags.csv` to the share after every sample.
- `tools/vm/round2-support.tests.ps1`: extended with both CSV formats, VM-only,
  sustained, one-spike, short-window, memory-dip, legacy and 24 h cases, the
  runner wiring, the preflight gate, and a real sampler-format run. Ends
  `All round-2 host checks passed.`
- Docs: the VM guide's round-2 section now gives the preflight/sampler commands,
  expected output, the sustained-load rule, the 24 h pool recipe and bundle r7.
  A corrupted sentence (`toolsmhost-load.ps1`, `C:VMshost-load.csv`, backslashes
  lost in an older shell edit) is fixed.

A new bundle is required for the guest (r3-r6 predate the six-column sampler and
the pool changes). `prepare-round2-bundle.ps1 -Destination
C:\VMs\ar-share\rc-0.2.5-r7` builds it from a clean committed tree and verifies
every hash; r6 is left in place as history. Done at source `97ab60910`:
`C:\VMs\ar-share\rc-0.2.5-r7`, 89 files, `MANIFEST.txt` SHA-256
`3D40FB0CB2C359BFA4988C2BC029B1589D45DB47652E43CD0459B4295A7180E7`. The bundle's
`run-stage-a.ps1` contains the gate and `-PoolMinutes` 1440, and its
`round2-support.ps1` the host-load helpers. Read-only
VM state on 2026-11-24: `AR-DriverTest` is `poweroff` with current snapshot
`02-test-signing-ready`, 8192 MB RAM, 4 vCPUs, nested VT-x off (NEM); the
release-qualification guest was running on the host, so no slot was taken. No
snapshot was taken, restored or deleted.

## 5. The one remaining ordered VM block (fail-fast)

All of it in one unattended block, from `02-test-signing-ready`, on an idle host,
after `round2-host-preflight.ps1` exits 0. Order matters:

1. `repro` (`-Blocks repro -FuzzSeconds 300 -ReproBoots 5`): does the Verifier
   freeze reproduce on an idle host? A reproducible freeze is a driver/Verifier
   defect that blocks signing; five clean boots keep the VM explanation alive but
   unproven. The per-freeze `host load` line decides.
2. If repro is clean: `ab` on a quieter design (`-Blocks ab -AbPhases 6`), then
   `live`, then `verifier`, then `pool` (the default `ab,pool,verifier,live` runs
   all four in this order). The A/B verdict now also carries its host-load line.
3. Owner decision after the numbers: revert the frame-clock hunk as a new RC
   above 0.2.6 (keep its PDB), or accept 0.2.5 and record the A/B as confounded;
   and whether helper exit 5 under a live route is acceptable for item 3.

Nothing above is a pass until its block reports PASS under an idle host-load line.
VCAB-28's 24 h pool is a separate gate: `-Blocks pool -PoolMinutes 1440` with the
26 h sampler, or fold it into the Stage C long session (owner decision).

## 6. Not run

Any VM start, driver install/load, Driver Verifier, WPR/DPC measurement, VB-Cable
install, signed package, 24 h pool run, Stage B or C, host boot/security change.
The host checks in section 4 ran on the development PC on 2026-11-24.