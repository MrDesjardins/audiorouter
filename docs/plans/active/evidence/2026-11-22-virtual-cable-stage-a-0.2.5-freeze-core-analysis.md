# Analysis of the 0.2.5 Verifier freeze core (2026-11-22)

Input: `C:\VMs\ar-share\rc-0.2.5-r3\freeze-20261122\guest-core.elf` (VirtualBox ELF core of the frozen guest, 8.7 GB)
and `VBox.log`. WinDbg cannot open this format ("invalid file format"), so the core was read with Volatility 3
(2.28.2, venv `C:\VMs\vol-venv`, Windows symbols from the Microsoft symbol server) plus two small plugins kept in
`tools/vm/vol-plugins/` (`threadstate.py` thread states and a heuristic stack scan; `cpustate.py` per-CPU `KPRCB`).
Run: `vol -p tools\vm\vol-plugins -f guest-core.elf threadstate.ThreadState --pids <pid> --stacks` and
`cpustate.CpuState`. Nothing here touched the dev PC's drivers.

## What the core shows

- Windows 11 build 26100, 4 CPUs, guest clock frozen at 2026-11-22 07:47:39 UTC. `audioroutervirtual.sys` was loaded.
  `verifier.exe` and `m03-bridge-fuzz` (pid 1520, 10 threads) had started at 07:47:14, so the guest stopped about
  25 s into the fuzz.
- Every `m03-bridge-fuzz` thread is an ordinary wait (user-request waits, one console read in `condrv.sys`); **no
  thread of the process, and no thread in the system, has a frame in `audioroutervirtual.sys`** (stack-scan of the
  fuzz process and of the only running thread).
- CPUs 0, 1 and 2 were in their idle threads at IRQL 0, no DPC running. CPU 3 was running one `svchost` thread at
  IRQL 0 with a standby thread chosen and one DPC queued: a normal mid-reschedule state. No CPU at raised IRQL, no
  spinning CPU, no lock chain.
- Nearly every thread (the ones that were waiting) shows the same wait-start tick: the guest stopped as a whole, it
  did not block in one place.
- `VBox.log`: the guest reported nothing after about 01:36:38 VM uptime (the fuzz start), while the VM stayed
  `running` for another 70 minutes. Just before (01:36:24) VirtualBox logged `Guest seems to be unresponsive. Last
  heartbeat received 19 seconds ago` and then `Guest is alive`: a 19 s stall already happened seconds before the
  freeze. The host has Core Isolation (Memory Integrity) on, so the guest runs on VirtualBox's NEM backend
  (`UseNEMInstead 0`, fallback to NEM because VT-x is unavailable).

## Reading

This is not a kernel deadlock or a driver hang: nothing is stuck in or waiting on the driver, no CPU is in a
spin or a raised-IRQL state, and the whole machine stopped at once. It looks like the guest stopped being scheduled
by the hypervisor (the NEM backend); the control run below weakens this reading. That fits the three freezes on record (RC3
Verifier hang 2026-10-30, 2026-11-11, this one), all under Driver Verifier on NEM, but it is an inference: a core
taken after the stop cannot show why the host stopped running the vCPUs. Not excluded: Verifier's added load
(special pool, I/O verification) makes the NEM stall likelier without the driver being at fault.

## What would make this decisive (not run)

1. Same fuzz, same duration, Verifier on **a Microsoft driver only** (no AudioRouter driver) or Verifier off:
   if the guest still freezes, the cause is the VM and not the driver.
2. Repeat Verifier on 0.2.5 on a substrate without NEM (native VT-x or bare metal; the plan's VCAB-24 note already
   says to qualify on one of those).
3. A vCPU register dump from the core (RIP and RFLAGS.IF per vCPU) to see whether the vCPUs were halted with
   interrupts enabled; not done (the VirtualBox core's CPU notes are not parsed by Volatility).

Until then A11 stays open, with this core as evidence that the driver is not visibly stuck, not as a pass.

## Control run (2026-11-22, same day): the freeze did not reproduce without Verifier on our driver

Bundle `rc-0.2.5-r4` (source `fc5c4636c`, MANIFEST SHA-256 `CEAC95A8...BEBD52`), `-Session round2 -Blocks control`,
fresh restore of `02-test-signing-ready`, 0.2.5 loaded (SYS SHA-256 D2C26DD92C00 checked). The same IOCTL fuzz,
1800 s each:

| Part | Verifier | Result |
| --- | --- | --- |
| A | off | fuzz PASS, 10 min stall watch PASS (longest 5 s sample gap 7.5 s) |
| B | on for Microsoft's `ks.sys` and `portcls.sys` only; AudioRouter's driver loaded, not verified | fuzz PASS, 10 min stall watch PASS (7.6 s) |

No bugcheck, no dump, guest never frozen. Raw files: `C:\VMs\ar-share\rc-0.2.5-r4\control\` (zip, state, VBox.log).

What this changes:

- It does **not** show the VM is the cause. The prediction "if the guest still freezes, it is the VM" was not
  triggered: it did not freeze. The only configuration that has frozen is Driver Verifier on
  `audioroutervirtual.sys` plus the fuzz (RC3 on 2026-10-30, 0.2.5 on 2026-11-11 and 2026-11-21). That fits a
  Verifier-on-our-driver trigger, or a rare VM stall that has not hit these two runs. One clean pair of runs cannot
  tell them apart.
- The core still shows nothing stuck in the driver (above), so the open question is why a verified run stops the
  whole guest.
- Correction to the first version of this note: VirtualBox's "Guest seems to be unresponsive" heartbeat lines are
  **not** a reliable stall signal. In the control run they appeared with gaps of 77 s, 459 s and 1018 s while the
  guest's own 5 s stall watch measured at most 7.6 s. The 19 s heartbeat line before the freeze therefore does not
  support a "VM stall" reading; drop it as evidence.

Next (not run): repeat the original configuration (Verifier on `audioroutervirtual.sys`, fuzz) several times from
a fresh restore, with a short fuzz (5 min) per boot, to see whether the freeze reproduces and how soon after the
fuzz starts. A reproducible freeze at the same point is a driver or driver-plus-Verifier defect and must be fixed
before signing; no freeze in, say, five boots keeps the VM explanation alive but unproven.
