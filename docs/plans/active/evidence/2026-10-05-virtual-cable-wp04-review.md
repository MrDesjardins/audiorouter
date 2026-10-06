# WP-04 independent kernel security review

Date: 2026-10-05. Reviewer: fresh delegated agent context, separate from the
implementation author. Scope: uncommitted WP-04 `bridgeio.h`, `adapter.cpp`,
their WaveRT call sites in `minwavertstream.cpp`, and the new user-mode helper
tests. Requirements: SEC-08, VDEV-07, NFR-16; WP-04 mapping, ownership,
single-fetch, cleanup and nonblocking callback scenarios.

Result: initial review **changes required**; all identified source-level
findings were addressed and follow-up review found no remaining issue in the
reviewed scope. No driver was loaded, no boot/security
settings were changed, and no VM result is claimed. Initial findings below
describe the first reviewed snapshot; follow-up review history follows them.

## Findings

1. **High — control serialization raises IRQL above the section-handle DDI's
   limit.** `adapter.cpp:59` acquires `ExAcquireFastMutex`; the guard at
   `adapter.cpp:814` encloses the call to `ObReferenceObjectByHandle` at
   `adapter.cpp:582`. Acquiring this mutex raises IRQL to APC_LEVEL, whereas
   the object-reference routine requires PASSIVE_LEVEL. Any valid mapped OPEN
   follows this path, so the newly granted interactive-user access exposes an
   invalid kernel DDI invocation; Driver Verifier qualification can fail even
   on an ordinary request. Use serialization that preserves PASSIVE_LEVEL,
   keep it outside the callbacks, and test a valid OPEN under Verifier.
   The same guard encloses unload's `IoDeleteSymbolicLink` at
   `adapter.cpp:888`; audit every control DDI's IRQL after changing the guard.
   Sources: [ExAcquireFastMutex](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-exacquirefastmutex),
   [ObReferenceObjectByHandle](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-obreferenceobjectbyhandle).

2. **Medium — stream scratch state is not invalidated or scrubbed at lease
   retirement/replacement.** `adapter.cpp:308` scrubs only the logical shared
   view. `minwavertstream.cpp:1737` reuses pending capture scratch frames;
   `minwavertstream.cpp:1885` resets render scratch offsets only when the shape
   changes, and does not zero the samples. Neither path associates its pending
   audio or retained read sequence with a lease generation. Once the direction
   defect below is repaired, replacing a lease with the same shape can retain
   old-session partial audio, and a fresh sequence can be rejected against the
   previous lease's retained sequence. This fails WP-04's explicit scratch
   cleanup condition and undermines VDEV-07 isolation. Introduce a generation
   boundary for callback-owned scratch; discard/zero pending samples and reset
   sequence state at that boundary without waiting in a callback. Regress a
   same-shape replacement while a quantum is partially consumed/assembled.

3. **Medium — existing direction guards reject both production WaveRT bridge
   operations.** `adapter.cpp:181` accepts only RENDER_SOURCE for copy, but
   capture `WriteBytes` calls copy with CAPTURE_SINK at
   `minwavertstream.cpp:1740`. `adapter.cpp:241` accepts only CAPTURE_SINK for
   publish, but render `ReadBytes` calls publish with RENDER_SOURCE at
   `minwavertstream.cpp:1853`. Both calls return DEVICE_NOT_READY; the helper
   tests never call these wrappers. This is a pre-existing integration defect,
   rather than a newly introduced memory-safety error, but blocks meaningful
   live qualification and masks the scratch-retention scenario. Correct and
   exercise the actual callback directions before claiming audio acceptance.

## Function inventory and reviewed invariants

| Function(s) | User-controlled surface and review result |
| --- | --- |
| `AudioRouterValidateBridgeOpenRequest` | Buffered OPEN/maintenance fields; bounds channels/frames/rates/lease, generation, version, reserved words, UTF-16 length, embedded NUL and trailing content. No direct user pointer. |
| `AudioRouterValidateMappingBytes` | Buffered request size; arithmetic bounded by the preceding shape validator; exact logical header plus quantum size. |
| `AudioRouterValidateBridgeBlock` | Local shared-header snapshot; bounded dimensions, payload consistency and mapped-view extent. |
| `AudioRouterCopyBridgeBlock` | User-writable section header/payload; one volatile read per header scalar and payload sample, local sizing, destination capacity, finite rejection with bounded quantum zeroing. The mapping must remain pinned/alive and callers must reject a seqlock change. |
| `AudioRouterOpenOwnershipStatus` | Session comparison used on active OPEN; held same-session conflict versus different-session denial. |
| `HandleBridgeControlRequest` | Exact METHOD_BUFFERED request/output lengths; UserMode-only requestor; typed section reference with read/write access check; exact logical size and bounded page-rounded view; MDL pin before publication; same section-object identity rejected across directions. Stored file/session identity controls maintenance; rival request cannot shorten stored expiry. IRQL finding 1 remains. |
| `PinBridgeView` | Referenced system section view; bounded MDL allocation, exception handling around probe/lock, write access pin; allocation/pinning restricted to control path. |
| `AudioRouterCopyLeaseBlock`, `AudioRouterPublishLeaseBlock` | Shared mapped bytes; rundown covers view access; atomic view/extent/shape/generation loads; seqlock is bounded with no spin retry, publish capacity/shape/sequence bounds. Direction finding 3; no control mutex, allocation, disk/network access or wait in these routines. |
| `AudioRouterCopyLeaseBlockForDirection`, `AudioRouterPublishLeaseBlockForDirection`, `BridgeLeaseForDirection` | Fixed directional slot lookup and bounded delegation; no user-derived slot index. |
| `AudioRouterGetLeaseShapeForDirection` | Rundown plus atomic immutable shape reads; output pointers are kernel caller storage; callback never takes lease/control locks. Shape does not expose generation (finding 2). |
| `SetBridgeMappedBytes`, `LoadBridgeUshort`, `StoreBridgeUshort`, `PublishBridgeRequest`, `ClearBridgeRequest` | Atomic publication/invalidation of kernel-retained request fields; generation gate and publication barrier, request reset only after view rundown. |
| `RetireBridgeResources` | Wait for callback rundown on control path; zero pinned logical view before unlock/free/unmap; dereference retained section after unmapping. User can still mutate its own mapping; zeroing is not a revocation of user mappings. |
| `ReleaseLeasesOwnedByFileObject`, `BridgeControlCreateClose` | CLEANUP/CLOSE detach only owning file's leases; serialized against OPEN/maintenance; null-safe; drain before resource release. No file object dereference after cleanup. |
| `BridgeControlDeviceControl`, `CompleteBridgeIrp` | Serialization released before IRP completion; unknown IOCTL fails; see control IRQL finding 1. |
| `CreateBridgeControlDevice`, `DeleteBridgeControlDevice` | Interactive-user ACL with secure-open flag; unload detaches/drains/scrubs/unpins/unmaps lease views before device deletion. Guard IRQL must be corrected. |
| `ReadBridgePcmSample`, `WriteBridgePcmSample`, `ClampBridgeSample`, `IsBridgePcmFormat` | Fixed PCM widths and checked frame alignment; scalar access to PortCls-managed mapped DMA, conversion and finite/clamp behavior. Higher precision/conversion changes belong to WP-05/06. |
| `CMiniportWaveRTStream::WriteBytes`, `ReadBytes`, `RefreshBridgePublishShape` | Ring bounds, scratch bounds and silence on bridge rejection; no new control locks/waits. Findings 2 and 3; scratch lifetime is separate from mapped-view rundown. |
| `AllocateBufferWithNotification`, `AllocateAudioBuffer`, `FreeBufferWithNotification`, `FreeAudioBuffer` | Inspected DMA map/unmap context: PortCls allocates/maps/frees the MDL; byte sizes aligned to validated frame size. PortCls stream teardown serialization is relied upon, not independently qualified by this static review. |

## Checks and limitations

Performed read-only `git status --short`, targeted `git diff --stat`, full
reads of the bridge helpers and relevant adapter control/callback code,
WaveRT call-site and DMA-lifetime inspection, and helper-test source review.
Read applicable repository instructions, WP-04 and specification sections
5.2–5.4. Reviewed tests cover the pure header/request helpers, guarded-page
header mutation, finite rejection and capacity canaries; they do not simulate
kernel IRQL, dispatch/session/file ownership, MDL faults, rundown races or
stream scratch reuse. No test executable or build was run by this reviewer.

This is not an audit of every inherited SysVAD property handler or the Windows
audio stack. No direct shared-header double-fetch or bounded-copy overrun was
found in the revised helper. Pinned mapping residency/rundown must still be
qualified by the VM's 30-minute IOCTL fuzzer with Driver Verifier, wrong-object
and undersized sections, concurrent handle cleanup/OPEN, and second-user lease
denial. HVCI, signing, real callback timing and cross-session behavior remain
unmeasured. WP-04 stays incomplete until those required VM gates exist.

Next task: complete the targeted generation-uniqueness review, then user-run
WP-04 VM Verifier/fuzzer/A11 ownership qualification.

## Follow-up changes and verification

The implementation replaced the control FAST_MUTEX with an executive push
lock, kept it out of WaveRT callbacks, corrected the capture-sink read and
render-source publish direction guards, and returned lease generation with
shape. The capture reader now tracks the sink generation independently and
scrubs partial scratch/reset sequence on turnover or retirement. The driver
retains the last successful generation across CLOSE/expiry and rejects reuse;
protocol §5.2 documents this per-direction contract so a stream can detect a
reopen even when it missed the inactive interval. The fuzz scenario now uses
the valid 2-channel shape requesting 1056 bytes from a genuinely 64-byte
section, reaching the mapping path.

Fresh follow-up review confirmed the undersized section request and direction/
shape pairing. It found that rejecting only the immediately previous
generation still allowed a non-adjacent reuse. The driver now enforces a
strictly increasing per-direction high-water mark, the specification records
its lifetime and reload reset, and helper tests cover 17 → 18 → 17 rejection.
The reviewer confirmed the final targeted point. After the changes, host
helper tests pass 49 checks; x64 and ARM64 WDK build acceptance pass; the
fuzzer compiles. None
of these validates kernel Verifier behavior or real callback timing. The VM
30-minute Verifier/fuzz run, concurrent cleanup/OPEN behavior, A11 second-user
denial and audio/latency measurements remain pending and block WP-04
completion.
