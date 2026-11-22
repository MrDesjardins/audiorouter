# Stage A on 0.2.5, round 2: results (night of 2026-11-21)

Scope: `run-stage-a.ps1 -Session round2 -AutoLogon` from bundle `rc-0.2.5-r3`
(source commit `9027b730c`, MANIFEST SHA-256 `43ef5237...ada876`), in the isolated VirtualBox guest
`AR-DriverTest` restored from `02-test-signing-ready`, NEM guest. Test-signed drivers only, in the VM only.
Raw files: `C:\VMs\ar-share\rc-0.2.5-r3\` (`round2-state.json`, `round2-ab-rows.jsonl`,
`freeze-20261122\`). Nothing touched the host's drivers.

Operator notes: the snapshot's `artest` password had expired (guest date is after the snapshot), so it was
changed to a run-only value at first sign-in; every restore of the snapshot will show the same prompt.
Auto-logon worked across the planned restarts (pool and Verifier installs ended in exit 5 restarts and the run
resumed by itself).

## Block results

| Block | Result | Evidence |
| --- | --- | --- |
| ab (RC3 0.2.4 against 0.2.5) | **FAIL by the pre-registered rule, confounded** (below) | `round2-ab-rows.jsonl` |
| pool (A8, 60 min, 117 samples) | **PASS** | driver tags MNWR/MNTR/SyAu/AUDR flat at 524 KiB, 28 live allocations, growth 0 MiB/h; whole non-paged pool +6.98 MiB/h shown for comparison only; no bugcheck, no dump |
| verifier | **guest froze at the start of fuzz round 1 on 0.2.5** | below |
| live (update/remove under a route) | **not run** (never reached) | |
| pause | not requested (no `-WithHostPause`) | |

### ab

23 of 24 runs valid (one RC3 steady run invalid: 11.5 s service gap). Every run reported the expected driver
version, and RC3 bound correctly after 0.2.5 within one guest session in all phases.

| Phase | Driver | steady #1 underrun / late | steady #2 | restarts #1 | restarts #2 |
| --- | --- | --- | --- | --- | --- |
| ab1 | 0.2.5 | 0 / 0 | 0 / 0 | 1884 / 0 | 2379 / 0 |
| ab2 | RC3 | 39984 / 61 | (invalid) | 18816 / 23 | 4272 / 6 |
| ab3 | 0.2.5 | 6256 / 6 | 5223 / 4 | 5005 / 3 | 2454 / 1 |
| ab4 | RC3 | 0 / 0 | 0 / 0 | 2352 / 2 | 3888 / 4 |
| ab5 | 0.2.5 | 2778 / 3 | 2606 / 3 | 5152 / 2 | 1982 / 1 |
| ab6 | RC3 | 1440 / 1 | 960 / 1 | 1920 / 0 | 2400 / 0 |

Verdict by the rule written before any result (per shape, medians, regress if more than +480 frames or +1 packet
and more than 1.5x): steady **REGRESSED** (RC3 median 960 frames / 1 late, 0.2.5 median 2692 / 3); restarts no
regression (RC3 3144 / 3, 0.2.5 2416 / 1). So the formal outcome is FAIL and the frame-clock change (review item 6)
is the revert candidate.

Reasons not to treat that as settled: the same driver ranges from 0 to about 40,000 frames between runs
(RC3 steady 0 in ab4, 39,984 in ab2); the first steady runs after a fresh install were zero on both drivers (ab1,
ab4), so the load on the VM, not the driver, may drive most of the spread; the steady comparison is 5 RC3 against
6 0.2.5 runs with a different phase mix; and the medians differ by about 1,700 frames (35 ms) against run-to-run
spreads of tens of thousands. The rule was not changed after seeing the data. The decision (revert the frame-clock
hunk as a new RC, or repeat the comparison under a quieter design) is the owner's: task 4 of
[virtual-cable-review-fixes.md](../virtual-cable-review-fixes.md).

### verifier: guest freeze

After "Driver Verifier on" and the planned restart, step "Driver Verifier active (round 1)" passed and the
30-minute IOCTL fuzz began at about 23:47 (host clock, 2026-11-21). The state file was last written 23:47:01;
at 00:55 the guest console still showed the fuzz header with the guest clock at 11:47 PM, two screenshots 20 s
apart were byte-identical, the VirtualBox process was alive and the VM state was `running`. The guest was
frozen from the start of the fuzz for more than an hour (a 30-minute step), with no bugcheck.
This is the signature of the 2026-11-11 freeze and the 2026-10-30 RC3 Verifier hang: it now happens on 0.2.5 as
well, and this time at the start of the fuzz, not after it. A11 stays open and is no longer an RC3-only item.

Preserved before power-off (the snapshot was not restored first): `freeze-20261122\guest-core.elf` (8.7 GB
VirtualBox ELF core of the frozen guest, `VBoxManage debugvm dumpvmcore`), `VBox.log*`, `frozen-screen.png`.
The core has not been analysed. The VM was then powered off.

## What this does to Stage A on 0.2.5

Stage A on 0.2.5 is **not passed**: the A/B rule fails (confounded), the Verifier freeze reproduced, and the
live update/remove under a route (review item 3) remains unverified. Pool (A8) passed.

## Next

1. Analyse the core (`guest-core.elf`) with the debugger for the stuck threads; the Verifier freeze is the main
   open kernel question.
2. Run `-Blocks live` alone from a restored snapshot (about 15 minutes).
3. Owner decision on the frame-clock hunk (revert as a new RC above 0.2.6 with PDB kept, or a repeat A/B with a
   quieter design: fewer runs per boot, and a fresh restore between drivers).

Not run: `live`, `pause`, Verifier fuzz round 2, the controlled restart under Verifier, VB-Cable ratio, A12.
