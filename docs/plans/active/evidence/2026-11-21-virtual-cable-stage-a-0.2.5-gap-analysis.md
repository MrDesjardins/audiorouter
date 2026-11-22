# Virtual cable: Stage A on 0.2.5, gap analysis (2026-11-21)

Scope: read-only host analysis of the Stage A results already collected for driver
0.2.5 on 2026-11-11/12, plus checks on the package bytes. No VM was started, no
driver was loaded, nothing was built, no host setting changed. Requirement IDs:
VCAB-20/24/25/27/28, VDEV-09, SEC-08, NFR-02. Plans:
[virtual cable](../virtual-cable.md), [review fixes](../virtual-cable-review-fixes.md).

## 1. What "restart Stage A on 0.2.5" meant, and what the record shows

The 2026-11-10 decision ended the RC3 freeze and said Stage A restarts on 0.2.5.
That run happened on 2026-11-11/12 (`C:\VMs\ar-share\rc-0.2.5`, results
`stage-a-all-20261112-164626.zip`, `stage-a-eight-20261112-175118.zip`,
`stage-a-extras-20261112-180833.zip`). The review-fixes plan closed with "Stage A
for 0.2.5 is complete except VCAB-25, A12, A8 and the 96 kHz sweep (REVIEW) and
the post-fuzz freeze". This analysis disagrees with that summary: several gates
the same plan set for itself were never compared, so Stage A on 0.2.5 is **not
yet passed**. It needs a second, smaller VM round (section 4), not a repeat of
the whole matrix.

## 2. The package under test

| Item | Value |
| --- | --- |
| Driver | 0.2.5.0, normal build (`timingDiagnostics:false`), test-signed, tree clean |
| Built from | commit `7c88895c2` (`package.json` `gitCommit`); bundle prepared from `a2dd275e7` |
| SYS SHA-256 | `D2C26DD92C0038FDCF661159FE333F962C9C3C5FB0942A5ED825A5649EBC234A` |
| CAT SHA-256 | `12F4856101C60312D96DF2C2E7D53116C07A40C5D0CBA40756EFFF6619AF2223` |
| INF SHA-256 | `AE3212D026A5203640AA3CE05D45A5B6502DE828825D44F302731E38500F1CEA` |
| `driver-next` (0.2.6, A13) | SYS `690E1815…`, CAT `F0BBF6A1…`, INF `1E8BE17A…` |
| Source freeze | `git log 7c88895c2..origin/main -- drivers/audiorouter-virtual` is empty (checked at `66e7854d4`), and `git diff a2dd275e7 HEAD -- drivers/` was empty |
| Symbols | **No `.pdb` for the 0.2.5 SYS exists** under `C:\VMs` or `target/`. The signing guide says to keep the PDB with its SYS. The package is reproducible only up to debug identity (2026-10-28 finding), so any rebuild is a different binary. |

Task 3 of the pre-purchase plan, on these exact bytes (host, read-only):

- `infverif /v /h /rulever 25h2` (OS build 10.0.26200 rules): **INF is VALID**.
- Section scan (`dumpbin /headers`): 9 sections; `.text`, `fothk`, `PAGE`,
  `INIT` are execute-only, `.data` is read/write; **none is writable and
  executable**. DLL characteristics `0x4160` (ASLR, high-entropy ASLR, NX, CFG).
- Static evidence only. Dynamic HVCI needs Stage B or C.

## 3. Findings per open item

### 3.1 Item 6 (frame clock) revert gate: not decided

The review-fixes plan says to revert the `UpdatePosition` hunk if late packets or
silenced frames rise against r10. Nobody made the comparison. Result of doing
it on the bit-exact runs (float32 matrix, product route, 20 s each; 0.2.5 against
RC3 r10 of 2026-10-14):

| | RC3 r10 | 0.2.5 |
| --- | --- | --- |
| Runs bit-exact | 5 of 10 | 1 of 11 |
| Route `silence` blocks (sum) | 208 | 242 |
| Route `dropped` (sum) | 444 | 455 |
| `renderSequenceGaps` | 0 | 0 |
| Capture overflow | 0 | 0 |

Each set contains one run under Driver Verifier. The route counters are almost
equal (the ~18 silent blocks per route are the start-up fill before the first
render block, identical in a passing run). The failing runs have a maximum error near 1.0 and
a mismatch that continues to the end of the capture, which is consistent with one
inserted or removed block shifting the alignment (the documented stall pattern);
the block itself was not located in each run. The pass rate however moved the wrong way: one-sided Fisher
p≈0.055. At these sample sizes that is **inconclusive, not exculpatory**.

The artifacts do not contain the driver's own late/skipped/underrun counters for
these runs, so R4 ("judge by late and skipped packet counts") could not be judged
from Stage A. Closing it needs the driver counters from a paired run (section 4,
block 1).

### 3.2 Render underruns of the 2026-11-11 traced run: open

The 17:01 run (0.2.5, both directions, repeated Windows Test playback) had 6,167
render underrun frames with zero late packets, in steps of about one 10 ms
packet. The packet diagnostic that was meant to explain it never exercised render
(no playback into Cable A), and the Stage A summary does not mention it. It stays
open until a paired run on RC3 and 0.2.5 shows whether RC3 does the same.

### 3.3 Item 3 (unload without reboot): partly verified

- A13 `update` to 0.2.6.0 returned exit 0 (no restart) on the same device
  instance, but **no cable route was running** during it; the review-fixes plan
  required the update "with AudioRouter playing a cable route" because an open
  handle is exactly what used to pin the image.
- `remove` returned exit 5 once (2026-11-11, `users` session, directly after the
  second-user test) and 0 on every later run. The cause of that exit 5 was not
  recorded.
- Verdict: item 3 is **not verified** under its own condition.

### 3.4 A11 post-fuzz freeze: recurring VM pattern, not attributed

`VBox.log` of AR-DriverTest records `Guest seems to be unresponsive. Last
heartbeat received 72337 seconds ago` (`TM: Giving up catch-up attempt at a
72 336 709 850 002 ns lag`) with no state change to PAUSED or SUSPENDED. The host
System log for 2026-11-11 18:00 to 2026-11-12 19:00 holds no sleep or resume event.
The same signature (20,464 s heartbeat loss, no state change) is in
`VBox-hardening-hang-20261030.log`, which belongs to an **RC3 0.2.4** Verifier
session. So the freeze is not new with 0.2.5, which argues against a 0.2.5
regression, but its cause (hypervisor, Verifier plus fuzz, or the driver) is
unknown. The 8.7 GB core `C:\VMs\ar-dump-a11.elf` was not opened: it is a
VirtualBox ELF core and WinDbg does not open it directly. Still counts as a
blocker for calling Stage A passed: only a Verifier session that ends cleanly
two times in a row, or an explanation, clears it.

### 3.5 A8 pool (REVIEW): probably an excursion, tag-level proof missing

`pool.csv` (119 samples, 60 min): first 134.12 MiB, last 133.85 MiB, minimum
131.73, maximum 144.94. A fitted slope is 1.74 MiB/h (the summary says 2.187), but the
endpoints are flat and the range is a single mid-run excursion of about 13 MiB; RC3
r10 had a 1.81 MiB range. The measurement is whole-system non-paged pool, so it
cannot attribute anything to the driver. The driver allocates with tags `RWNM`
(`MINWAVERT_POOLTAG`), `RTNM`, `uAyS` and `RDUA` (stored in memory as `MNWR`,
`MNTR`, `SyAu`, `AUDR`); `CSaveData` and `operator new` use them too. The VM
block must read those tags with `poolmon` or `!poolused`, not the whole pool.

### 3.6 A15, 30 min, eight cables: not met

Four events stayed unexplained in 882 (second run). The pre-purchase task 4 bar is
"every FAIL attributed to a measured guest-wide stall". It is **not met**; the
earlier note calling the run "VM" is withdrawn. Part of the shortfall is a
user-mode starvation (about 70 % of real time, driver accounted every frame), but
the four events remain for Stage C.

### 3.7 VB-Cable (VCAB-25)

`stage-a-extras` failed only because VB-Cable was not installed in the guest. A
`vbcompare-*.zip` exists in the bundle, but it comes from that failed step. A
`03-before-vbcable` snapshot exists, so the VM was prepared for the install once
before. The owner installs VB-Cable (third-party installer, VM only), or the
ratio stays a Stage C item.

### 3.8 Other results, unchanged

Users/A11 second-user refusal, crash recovery, R1 (4,000 and 6,500 ms stalls ridden),
A13 update and repair, R2 stale-handle refusal and R6 generation reuse pass.
THD+N −153 dB passes (RC3 r10 was −21.5 dB REVIEW). The 96 kHz sweep (0.097 dB
against 0.05 dB) improved from 0.316 dB at r10 and stays REVIEW.

## 4. Remaining Stage A work

Owner VM block, one batch, from `02-test-signing-ready`, fail-fast, in this order.
Every step is test-signed, VM only. Nothing here is built on the host.

1. **Paired render A/B (items 3.1, 3.2).** Install RC3 (`rc-0.2.4-base`), run the
   30 s both-direction tone with continuous playback into Cable A and collect the
   driver's render underrun, late and skipped counters; restore the checkpoint,
   repeat on 0.2.5 (`rc-0.2.5`), three runs each, alternating order. Decision
   rule: if 0.2.5 shows more late packets or silenced frames than RC3 across the
   three pairs, revert the frame-clock hunk (`AudioRouterAdvanceFrames` and
   `UpdatePosition`) as a new RC; otherwise item 6 stays.
2. **Unload with a live route (item 3.3).** Start the cable route, then `update`
   to 0.2.6 and `remove` while it plays; record exit codes and whether "Driver
   did not unload" or a restart request appears.
3. **Pool by tag (item 3.5).** 60 min with the 8-cable route, `poolmon` sample
   every 30 s for the four tags plus the whole pool.
4. **Verifier session ends cleanly (item 3.4).** Standard Verifier on the SYS,
   30 min IOCTL fuzz, then a controlled restart; repeat once. Also record
   whether the guest is still responsive 60 min after the fuzz.
5. **Lease across a long pause (task 7 of the review plan).** VirtualBox guests
   rarely enter S3, so use `VBoxManage controlvm AR-DriverTest pause` for 90 s
   and `resume` as the nearest analog, with a cable route running; the route must
   ride it. This is **not** a real sleep/resume test and is recorded as such.
6. **VB-Cable ratio** (owner install) and **A12** (attended), or both deferred to
   Stage C by recorded decision.

**Prepared 2026-11-21 as session `round2`** (`tools/vm/run-stage-a.ps1 -Session round2 -AutoLogon`,
operator steps in the [VM guide](../../../operations/virtual-cable-vm-guide.md#session-round2-second-stage-a-round-on-025-overnight)).
Blocks `ab`, `live`, `pool`, `verifier` (and optional `pause`) cover items 1 to 5 above in one unattended night.
Host-tested: `tools/vm/round2-support.tests.ps1` (parse, ASCII, undefined-command scan, tone-report parsing,
the A/B rule on synthetic runs, growth fit, the pool-tag reader against the host's own table, auto-logon set and
clear on a fake registry key, the route-state check). The guest scripts themselves have **not** run in the VM.

Changes from the list above, decided before any result:

- **A/B rule recalibrated.** The first draft counted only "quiet" runs (largest service gap 40 ms or less). The
  existing tone reports showed only 1 of 9 runs under 40 ms (typical gaps 61, 78, 154, 248, 332 ms), so that filter
  would have discarded the night. The rule is now per render shape (steady stream, four short streams) on medians
  of at least 4 valid runs per driver; 0.2.5 regresses when its median underrun frames or late packets exceed RC3's
  by more than 480 frames (1 packet) and by 50 percent; a shape whose median gap exceeds 500 ms is inconclusive;
  PASS needs both shapes judged; the drivers alternate over six phases, so stalls hit both. Each run must report
  the expected driver version (`driver_version` in the tone tool's driver line) and the loaded SYS hash must match
  the package, or the run is not counted.
- **Item 2 is run by calling the helper directly** with the route verified alive and `running` just before, because
  `vm-checks update` first sets a 96 kHz format on Cable B Output, which could end the route and make the test
  meaningless. An invalid precondition is REVIEW "not counted", never PASS.
- **Pool block** reads the driver's four tags (`MNWR`, `MNTR`, `SyAu`, `AUDR`) in-guest through
  `NtQuerySystemInformation` instead of `poolmon`; PASS at most 0.25 MiB/h growth. The host has another SysVAD-derived
  driver using `MNWR`, so the tag alone proves nothing on the host; the guest has none.
- **Item 5** is only available with `-WithHostPause` and the host watcher (`round2-pause-watcher.ps1`, never run
  against a live VM); it is a `controlvm pause`, not a real S3 sleep.
- The guest signs in by itself after each planned restart with `-AutoLogon` (password stored in the guest Winlogon key
  for the run only). The PC must not sleep overnight.

## 5. Decision rules

- Blocks 1 to 4 must pass before Stage A on 0.2.5 is called passed.
- If block 1 reverts the frame clock, or any block finds a kernel defect, this is
  a **new RC**, and Stage A restarts. The new version must be above 0.2.6, because
  0.2.6 was installed in the VM and a package with the same `DriverVer` does not
  bind (the packet-diagnostic lesson).
- A new RC also needs the PDB kept next to its SYS.
- VCAB-24 stays a documented VM deviation and VCAB-25 stays indicative until
  Stage C, as before. No purchase or signing action follows from this record.

## 6. Not run

Any VM stage, any driver load, Driver Verifier, `poolmon`, the ELF core analysis,
a WDK build, CodeQL. The host checks above ran on 2026-11-21 on the development PC.
