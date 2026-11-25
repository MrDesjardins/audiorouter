# W0: what the signed driver makes permanent

Status: **approved by the owner as recommended (2026-11-25).** Every blank Owner cell below means `yes`. Part of
[one driver, signed once](driver-sign-once.md). Read from the 0.2.5 source
(`drivers/audiorouter-virtual`); nothing was changed. "Later" means after signing, so
a new release candidate, a full Stage A and a new signature.

Fill **Owner** with `yes` (accept the recommendation), `no`, or another choice.

## A. Decide now: cannot be raised later

| # | Item | Today (source) | If left | Recommendation | Owner |
| --- | --- | --- | --- | --- | --- |
| A1 | Cables | 8 (`AR_BRIDGE_MAX_CABLES`, INF, GUIDs) | Never more than 8 | **16**, default 2; pool cost grows to about 16 MiB with all 32 streams open | decided 16 |
| A2 | Bus ID | exactly `cable-` + one letter (`AudioRouterParseCableBusId`) | Holds 26 | Keep; 16 fits | |
| A3 | Protocol version match | OPEN refuses any minor other than exactly 1.1 | An app built for a later driver cannot open this one unless it keeps speaking 1.1, and a later driver would refuse this app | **Accept a range:** same major, minor at least 1 and at most the driver's own; new features through capability bits | |
| A4 | Shared header space | 128 bytes, of which only `Reserved0` (4 bytes at offset 92) is free; counters now fill 104-127 | No room for another counter | **Grow the header to 256 bytes** before the first release. It changes the payload offset, so the app and ABI tests change with it. Minor becomes 2 | |
| A5 | Reserved request/reply fields | OPEN extension refuses unknown flags and non-zero reserved words; QUERY has 16 free words | Good already | Keep; add a test that every reserved word is refused on OPEN | |
| A6 | Capture queue | 2 blocks, 2 fixed arrays | 10 ms tolerance at 96 kHz/480 frames, the thinnest margin | **Ceiling 4 blocks, depth from the registry, default 2**, so the default latency does not change. Needs the W3 model first | |
| A7 | Render queue | up to 8 private blocks, limited to 32,768 samples in total; shared slot of 1 block | 8 channels with large blocks get few blocks | Keep 8; make depth a registry value **1-8, default 8** | |
| A8 | Shared slot (ring) | one block per direction | A ring would be a new wire layout | Spike in W3; keep one block unless it gains at least 2 ms or one more stall of tolerance. Can be dropped only before January 24 | |
| A9 | Sample rates | 44.1, 48, 96 kHz | No 88.2, 176.4 or 192 kHz ever | **Add none.** Each adds 20 formats and tests; 192 kHz doubles the DPC work and no user asked. Say so if you want 192 kHz | |
| A10 | Sample formats | 16, 24 in 32, 32-bit PCM, 32-bit float | No packed 24-bit | Keep; Windows' shared engine converts | |
| A11 | Channels | 1, 2, 4, 6, 8 | No 16-channel cable | Keep | |
| A12 | Processing modes | DEFAULT only, no RAW | Apps that demand RAW cannot get it | Keep: the cable applies no processing in DEFAULT. Prove with a RAW request test that Windows fails cleanly | |
| A13 | Streams per pin | 1 render, 1 capture | Only one exclusive-mode app; the shared engine mixes any number | Keep | |

## B. Permanent identities: any change looks like a new device

| # | Item | Today | Recommendation | Owner |
| --- | --- | --- | --- | --- |
| B1 | Hardware ID, service, device names | `ROOT\AudioRouterVirtual`, `AudioRouterVirtual`, `\Device\AudioRouterVirtualBridge` | Keep | |
| B2 | Filter names and GUIDs of cables A-H | generated from `cables.json` | Frozen; only add I-P | |
| B3 | Endpoint names | "AudioRouter Cable A Input/Output", "AudioRouter Virtual Cable" | Keep; the owner confirms the wording, also for I-P | |
| B4 | INF provider and device text | in the signed INF | Confirm spelling of publisher and product before January 24 | |
| B5 | Default endpoint format | 48 kHz, stereo, float (INF blob) | Keep; users change it in Sound settings | |
| B6 | Windows floor | 10.0.22000 | Keep | |
| B7 | ARM64 | builds, never run | Leave out of this signing | |

## C. Changeable later without a signature (registry)

| # | Item | Range and default | Recommendation | Owner |
| --- | --- | --- | --- | --- |
| C1 | `CableCount` | 1-8, default 2, read at device start | 1-16 | |
| C2 | `MinPeriodFrames` | 64-480, default 128 | Keep. **32 frames** would be the only way to go lower later; it raises timer and DPC work. Keep unless W3 shows a gain | |
| C3 | `DefaultPeriodFrames` | 128-960, **reported only** | Keep, documented as information; Windows owns the default period | |
| C4 | `MaxLeaseMs` | 500-60000 | Keep | |
| C5 | `SilenceOnStaleMs` | in the spec, **not read by the driver** | Remove from the spec; the driver already outputs silence when a block is missing | |
| C6 | Cable names | applied by the helper, not the driver | Keep | |
| C7 | Power idle settings (3 s to D3) | INF | Keep the mechanism; test resume from D3 at the idle boundary (W5 churn) | |

## D. Security and lifetime, also permanent

| # | Item | Today | Recommendation | Owner |
| --- | --- | --- | --- | --- |
| D1 | Control device access | `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)`: any interactive user | Keep; a per-app restriction is not enforceable in the driver. Confirm in the W4 review | |
| D2 | Lease limits | one owner per cable and direction, session-bound | Keep | |
| D3 | Packet number wrap | 32-bit: about 66 days at 64 frames, 1.4 years at 10 ms | Prove with a simulated clock (W4); fix in 0.3.0 if it fails | |
| D4 | Generation rule | differs from the last two | Keep | |

## E. Remove before signing

| # | Item | Finding | Recommendation | Owner |
| --- | --- | --- | --- | --- |
| E1 | `savedata.cpp`, `savedata.h` | Microsoft sample that writes audio to disk. **Not dead:** `common.cpp` still calls `InitializeWorkItems` and `DestroyWorkItems`, which allocate kernel work items at start | Remove. A signed audio driver should contain no file-writing code | |
| E2 | `micarraytopo.*`, `speakertopo.*`, `micarray*/speaker*` tables | Still compiled; the topology handlers are referenced by tables | Prove which tables the cables use, rename those, remove the rest | |
| E3 | Microsoft DDK tags and debug text | Pool tags `MNWR`, `MNTR`, `SyAu` | Keep the tags (tests use them) | |

## Order if approved

1. A3, A4, A5 together (one protocol 1.2 change, host ABI tests on both sides).
2. A1, B2, C1 (16 cables).
3. E1, E2 (removal only, no behavior change).
4. A6, A7 after the W3 model; A8 only if its spike passes.

None starts before this table is approved and D2 (the VM freeze) has an answer.
