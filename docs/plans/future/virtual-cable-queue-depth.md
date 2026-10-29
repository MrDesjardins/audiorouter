# Virtual cable: deeper driver queues (deferred)

Status (2026-10-28): **deferred by owner decision, not authorized.** The owner
chose to keep the frozen RC3 driver (0.2.4, source `d2560715`) and accept the
queue depths below, and asked that this be documented so it can be tackled
when there is more time. Context: the
[pre-purchase assurance plan](../active/virtual-cable.md#pre-purchase-assurance-plan-2026-10-28-user-request).
Requirements touched: VCAB-24, VCAB-25, VCAB-30, VDEV-12.

## What is fixed in the signed driver

Audio waits in small bounded queues between the app and Windows. They are
compiled into the kernel driver, so changing them needs a new release
candidate, a full Stage A rerun and a new Microsoft signature.

| Direction | Depth | Source |
| --- | --- | --- |
| Render (app plays into the cable) | `min(4 blocks, 32,768 samples ÷ channels)`; 4,096 frames at 8 ch once blocks reach 1,024 frames; 1,920 frames at 2 ch with 480-frame blocks | `Source/Inc/renderqueue.h` (`Capacity`) |
| Capture (app feeds the cable's recording side) | two blocks (scratch + prefetch), about 2 × block size | `Source/Main/minwavertstream.cpp`, `captureBlocks[2]` |
| Windows packet size | 1 or 2 notifications per buffer | `minwavertstream.cpp` |

Time a freeze can be absorbed at the usual 480-frame block: render 20 ms (2 ch)
and 43 ms (8 ch) at 96 kHz, 40 ms and 85 ms at 48 kHz; capture 10 ms at 96 kHz
and 20 ms at 48 kHz. Block size (`FramesPerQuantum`, up to 4,096) is chosen by
the app, so a larger block buys tolerance up to these ceilings at the cost of
extra latency, with no new signature.

## Why it was deferred

- Evidence of a problem is VM-only. In r10 a 15 ms service stall at 96 kHz
  removed 1,344 frames; the VirtualBox guest stalls far more than a real PC,
  and which direction dropped was not established.
- Deeper queues add latency, and VCAB-25 is judged against VB-Cable.
- A driver change restarts Stage A (days of VM runs) before the certificate is
  bought.
- Stage C on the real PC will show whether it matters.

## Reconsider when

- Stage C shows dropouts with counters rising on the real PC that a larger app
  block size cannot remove at acceptable latency; or
- the owner wants more margin and accepts the Stage A rerun; or
- a kernel change is needed for another reason (then bundle this with it, since
  the driver is signed once).

## Prerequisites and outline

1. Establish which direction dropped in r10 from its evidence (driver counters
   per direction), and measure stall lengths on real hardware in Stage C.
2. Decide the target tolerance per rate and channel count, and the latency
   budget against VB-Cable (DEC-14 method).
3. Model the change first (render timing model in the driver unit suite, burst
   and late-block cases, per the 2026-10-09 and 2026-10-10 lessons), then change
   depths (for example, a larger capture queue and a raised render ceiling),
   keep the callback free of allocation and waits.
4. Fresh-context driver review, WDK build, new version number, new RC, full
   Stage A, then resubmission (free within the EV certificate's validity).

## Rollback

Nothing is changed now. If this work is done later and fails, RC3 remains the
signed driver; the app's block-size setting is the interim lever.
