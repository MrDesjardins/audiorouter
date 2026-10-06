/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    adapter.cpp

Abstract:

    Setup and miniport installation.  No resources are used by simple audio sample.
    This sample is to demonstrate how to develop a full featured audio miniport driver.
--*/

#pragma warning (disable : 4127)

//
// All the GUIDS for all the miniports end up in this object.
//
#define PUT_GUIDS_HERE

#include "definitions.h"
#include "bridgeio.h"
#include "endpoints.h"
#include "minipairs.h"

typedef void (*fnPcDriverUnload) (PDRIVER_OBJECT);
fnPcDriverUnload gPCDriverUnloadRoutine = NULL;
extern "C" DRIVER_UNLOAD DriverUnload;
PDEVICE_OBJECT g_BridgeControlDevice = NULL;

typedef struct _AR_BRIDGE_LEASE_STATE {
    KSPIN_LOCK Lock;
    EX_RUNDOWN_REF Rundown;
    BOOLEAN Active;
    BOOLEAN Retiring;
    BOOLEAN RundownStarted;
    // The control file object that claimed this directional lease.  It is
    // compared under Lock so another authorized handle cannot replay the
    // request identity to refresh or close the lease.
    PFILE_OBJECT OwnerFileObject;
    ULONG OwnerSessionId;
    ULONGLONG LastGeneration;
    ULONGLONG LastHeartbeat100ns;
    AR_BRIDGE_OPEN_REQUEST Request;
    PVOID SectionObject;
    PMDL LockedMdl;
    volatile PVOID MappedView;
    volatile ULONG MappedBytes;
    volatile LONG64 NextSequence;
} AR_BRIDGE_LEASE_STATE;

AR_BRIDGE_LEASE_STATE g_BridgeLeases[AR_BRIDGE_LEASE_SLOTS] = {};
// Control-plane operations only: serialized OPEN prevents the same section
// being simultaneously claimed for both directions. No WaveRT callback takes
// this mutex. Complete IRPs only after releasing it to avoid completion reentry.
EX_PUSH_LOCK g_BridgeControlLock;
struct BridgeControlGuard {
    BridgeControlGuard() { ExAcquirePushLockExclusive(&g_BridgeControlLock); }
    ~BridgeControlGuard() { ExReleasePushLockExclusive(&g_BridgeControlLock); }
};

static NTSTATUS PinBridgeView(PVOID View, ULONG Bytes, PMDL* LockedMdl)
{
    // OPEN owns a referenced, mapped section at <= APC_LEVEL. Lock its pages
    // before publishing to DISPATCH_LEVEL callbacks; a file-backed shared view
    // alone does not guarantee residency. Allocation/pinning never runs in audio.
    *LockedMdl = IoAllocateMdl(View, Bytes, FALSE, FALSE, NULL);
    if (*LockedMdl == NULL) { return STATUS_INSUFFICIENT_RESOURCES; }
    NTSTATUS status = STATUS_SUCCESS;
    __try {
        MmProbeAndLockPages(*LockedMdl, KernelMode, IoModifyAccess);
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        status = GetExceptionCode();
        IoFreeMdl(*LockedMdl);
        *LockedMdl = NULL;
    }
    return status;
}

static void SetBridgeMappedBytes(
    _In_ AR_BRIDGE_LEASE_STATE* Lease,
    _In_ ULONG Bytes)
{
    InterlockedExchange(
        reinterpret_cast<volatile LONG*>(&Lease->MappedBytes),
        static_cast<LONG>(Bytes));
}

static USHORT LoadBridgeUshort(_In_ volatile USHORT* Value)
{
    return static_cast<USHORT>(InterlockedCompareExchange16(
        reinterpret_cast<volatile SHORT*>(Value), 0, 0));
}

static ULONG LoadBridgeUlong(_In_ volatile ULONG* Value)
{
    return static_cast<ULONG>(InterlockedCompareExchange(
        reinterpret_cast<volatile LONG*>(Value), 0, 0));
}

static void StoreBridgeUshort(
    _In_ volatile USHORT* Value,
    _In_ USHORT Data)
{
    InterlockedExchange16(
        reinterpret_cast<volatile SHORT*>(Value), static_cast<SHORT>(Data));
}

static void ClearBridgeRequest(_In_ AR_BRIDGE_LEASE_STATE* Lease)
{
    // Generation is the callback's validity gate. Clear it before changing
    // the remaining request fields so a callback can only observe an invalid
    // request while the control path retires the lease.
    InterlockedExchange64(
        reinterpret_cast<volatile LONG64*>(&Lease->Request.Generation), 0);
    KeMemoryBarrier();
    Lease->Request.ProtocolMajor = 0;
    Lease->Request.ProtocolMinor = 0;
    Lease->Request.BusIdBytes = 0;
    StoreBridgeUshort(&Lease->Request.Channels, 0);
    StoreBridgeUshort(&Lease->Request.FramesPerQuantum, 0);
    StoreBridgeUshort(&Lease->Request.Direction, 0);
    Lease->Request.SampleRateHz = 0;
    Lease->Request.LeaseMs = 0;
    Lease->Request.SectionHandle = 0;
    Lease->Request.MappingBytes = 0;
    Lease->Request.Reserved2 = 0;
    RtlZeroMemory(Lease->Request.BusId, sizeof(Lease->Request.BusId));
}

static void PublishBridgeRequest(
    _In_ AR_BRIDGE_LEASE_STATE* Lease,
    _In_ const AR_BRIDGE_OPEN_REQUEST* Request)
{
    // Invalidate the old callback contract before replacing its shape. The
    // final generation store publishes the complete request after its
    // callback-visible fields are atomically installed.
    InterlockedExchange64(
        reinterpret_cast<volatile LONG64*>(&Lease->Request.Generation), 0);
    Lease->Request.ProtocolMajor = Request->ProtocolMajor;
    Lease->Request.ProtocolMinor = Request->ProtocolMinor;
    Lease->Request.BusIdBytes = Request->BusIdBytes;
    Lease->Request.SampleRateHz = Request->SampleRateHz;
    Lease->Request.LeaseMs = Request->LeaseMs;
    Lease->Request.SectionHandle = Request->SectionHandle;
    Lease->Request.MappingBytes = Request->MappingBytes;
    Lease->Request.Reserved2 = Request->Reserved2;
    RtlCopyMemory(
        Lease->Request.BusId, Request->BusId, sizeof(Lease->Request.BusId));
    StoreBridgeUshort(&Lease->Request.Channels, Request->Channels);
    StoreBridgeUshort(
        &Lease->Request.FramesPerQuantum, Request->FramesPerQuantum);
    StoreBridgeUshort(&Lease->Request.Direction, Request->Direction);
    KeMemoryBarrier();
    InterlockedExchange64(
        reinterpret_cast<volatile LONG64*>(&Lease->Request.Generation),
        static_cast<LONG64>(Request->Generation));
}

// Driver-owned counters live in the pinned, 8-byte aligned shared header.
// The caller holds rundown protection on the lease that owns `View`.
static __forceinline volatile LONG64* BridgeCounter(
    _In_ PVOID View,
    _In_ SIZE_T FieldOffset)
{
    return reinterpret_cast<volatile LONG64*>(
        static_cast<UCHAR*>(View) + AR_BRIDGE_COUNTERS_OFFSET + FieldOffset);
}

static __forceinline void AddBridgeCounter(
    _In_ PVOID View,
    _In_ SIZE_T FieldOffset,
    _In_ ULONGLONG Delta)
{
    if (Delta != 0) {
        InterlockedAdd64(BridgeCounter(View, FieldOffset), static_cast<LONG64>(Delta));
    }
}

// Runs at OPEN after the view is pinned and before any callback can see it:
// reset the counters/acknowledgement of a reused file and announce the
// negotiated sample size. The block header and payload are left untouched.
static void InitializeBridgeViewHeader(_In_ PVOID View)
{
    RtlZeroMemory(static_cast<UCHAR*>(View) + AR_BRIDGE_COUNTERS_OFFSET,
                  AR_BRIDGE_HEADER_BYTES - AR_BRIDGE_COUNTERS_OFFSET);
    *reinterpret_cast<volatile ULONG*>(
        static_cast<UCHAR*>(View) + AR_BRIDGE_SAMPLE_BYTES_OFFSET) =
        AR_BRIDGE_SAMPLE_BYTES_FLOAT64;
    KeMemoryBarrier();
}

// This helper is intentionally independent of the sample's timer callback.
// It is safe for a future PortCls callback: rundown protects the mapped view
// from CLOSE/expiry/unload, and the callback takes no lease spin lock.
NTSTATUS AudioRouterCopyLeaseBlock(
    _In_ AR_BRIDGE_LEASE_STATE* Lease,
    _In_ ULONGLONG MinimumSequence,
    _Out_writes_(DestinationCapacitySamples) DOUBLE* Destination,
    _In_ SIZE_T DestinationCapacitySamples,
    _Out_ AR_BRIDGE_BLOCK_HEADER* Header,
    _Out_ ULONG* NonFiniteSamples)
{
    if (NonFiniteSamples != NULL) {
        *NonFiniteSamples = 0;
    }
    if (Lease == NULL || Destination == NULL || Header == NULL ||
        NonFiniteSamples == NULL ||
        !ExAcquireRundownProtection(&Lease->Rundown)) {
        return STATUS_DEVICE_NOT_READY;
    }
    PVOID view = InterlockedCompareExchangePointer(&Lease->MappedView, NULL, NULL);
    // The view is detached before retirement, but the control path clears
    // MappedBytes before waiting for rundown readers. Use an interlocked load
    // so this callback-side read cannot race that teardown write.
    ULONG mappedBytes = static_cast<ULONG>(
        InterlockedCompareExchange(
            reinterpret_cast<volatile LONG*>(&Lease->MappedBytes), 0, 0));
    USHORT direction = LoadBridgeUshort(&Lease->Request.Direction);
    ULONGLONG generation = InterlockedCompareExchange64(
        reinterpret_cast<volatile LONG64*>(&Lease->Request.Generation), 0, 0);
    KeMemoryBarrier();
    NTSTATUS status = STATUS_DEVICE_NOT_READY;
    if (direction == AR_BRIDGE_DIRECTION_CAPTURE_SINK &&
        view != NULL && mappedBytes != 0 && generation != 0) {
        volatile LONG64* state = reinterpret_cast<volatile LONG64*>(
            static_cast<UCHAR*>(view) + AR_BRIDGE_STATE_OFFSET);
        ULONGLONG stateBefore = static_cast<ULONGLONG>(
            InterlockedCompareExchange64(state, 0, 0));
        if (stateBefore == 0 || (stateBefore & 1) != 0) {
            ExReleaseRundownProtection(&Lease->Rundown);
            return STATUS_DEVICE_BUSY;
        }
        status = AudioRouterCopyBridgeBlock(
            static_cast<const UCHAR*>(view), mappedBytes, generation,
            MinimumSequence, Destination, DestinationCapacitySamples, Header,
            NonFiniteSamples);
        KeMemoryBarrier();
        if (NT_SUCCESS(status) && static_cast<ULONGLONG>(
                InterlockedCompareExchange64(state, 0, 0)) != stateBefore) {
            status = STATUS_RETRY;
        }
        if (NT_SUCCESS(status)) {
            // Flow control for the single-block slot: acknowledge the block
            // just consumed so the user-mode producer can publish the next
            // one immediately, paced by this stream's clock rather than its
            // own timer. Diagnostic/pacing value only; never read back here.
            InterlockedExchange64(reinterpret_cast<volatile LONG64*>(
                static_cast<UCHAR*>(view) + AR_BRIDGE_READER_SEQUENCE_OFFSET),
                static_cast<LONG64>(Header->Sequence));
        }
    }
    ExReleaseRundownProtection(&Lease->Rundown);
    return status;
}

// Publish one render-source quantum into the mapped lease. The caller
// supplies an already-interleaved float64 buffer; this routine performs no
// allocation, waits, logging, endpoint access, or control I/O.
NTSTATUS AudioRouterPublishLeaseBlock(
    _In_ AR_BRIDGE_LEASE_STATE* Lease,
    _In_ USHORT Frames,
    _In_ USHORT Channels,
    _In_reads_(SampleCapacity) const DOUBLE* Samples,
    _In_ SIZE_T SampleCapacity)
{
    if (Lease == NULL || Samples == NULL || Frames == 0 ||
        Channels == 0 || Channels > AR_BRIDGE_MAX_CHANNELS ||
        Frames > AR_BRIDGE_MAX_FRAMES) {
        return STATUS_INVALID_PARAMETER;
    }
    SIZE_T sampleCount = static_cast<SIZE_T>(Frames) * Channels;
    if (SampleCapacity < sampleCount) {
        return STATUS_BUFFER_TOO_SMALL;
    }
    if (!ExAcquireRundownProtection(&Lease->Rundown)) {
        return STATUS_DEVICE_NOT_READY;
    }
    PVOID view = InterlockedCompareExchangePointer(&Lease->MappedView, NULL, NULL);
    // See AudioRouterCopyLeaseBlock: teardown may clear this field while a
    // rundown-protected callback is finishing against the old view.
    ULONG mappedBytes = static_cast<ULONG>(
        InterlockedCompareExchange(
            reinterpret_cast<volatile LONG*>(&Lease->MappedBytes), 0, 0));
    USHORT direction = LoadBridgeUshort(&Lease->Request.Direction);
    USHORT framesPerQuantum =
        LoadBridgeUshort(&Lease->Request.FramesPerQuantum);
    USHORT channels = LoadBridgeUshort(&Lease->Request.Channels);
    ULONGLONG generation = InterlockedCompareExchange64(
        reinterpret_cast<volatile LONG64*>(&Lease->Request.Generation), 0, 0);
    NTSTATUS status = STATUS_DEVICE_NOT_READY;
    if (direction != AR_BRIDGE_DIRECTION_RENDER_SOURCE || view == NULL ||
        framesPerQuantum != Frames ||
        channels != Channels ||
        mappedBytes < AR_BRIDGE_PAYLOAD_OFFSET + sampleCount * sizeof(DOUBLE) ||
        generation == 0) {
        ExReleaseRundownProtection(&Lease->Rundown);
        return status;
    }
    // Never let the published sequence wrap through zero.  A wrapped value
    // would look like a fresh block to a reader that has retained an older
    // minimum sequence and could make the producer appear to move backwards.
    ULONGLONG nextSequence = static_cast<ULONGLONG>(
        InterlockedCompareExchange64(&Lease->NextSequence, 0, 0));
    if (nextSequence == MAXULONGLONG) {
        ExReleaseRundownProtection(&Lease->Rundown);
        return STATUS_INTEGER_OVERFLOW;
    }
    for (SIZE_T index = 0; index < sampleCount; ++index) {
        if (Samples[index] != Samples[index] ||
            Samples[index] > 1.7976931348623157e+308 ||
            Samples[index] < -1.7976931348623157e+308) {
            ExReleaseRundownProtection(&Lease->Rundown);
            return STATUS_DATA_ERROR;
        }
    }
    volatile LONG64* state = reinterpret_cast<volatile LONG64*>(
        static_cast<UCHAR*>(view) + AR_BRIDGE_STATE_OFFSET);
    ULONGLONG current = static_cast<ULONGLONG>(
        InterlockedCompareExchange64(state, 0, 0));
    if ((current & 1) || current > MAXULONGLONG - 2) {
        ExReleaseRundownProtection(&Lease->Rundown);
        return (current & 1) ? STATUS_DEVICE_BUSY : STATUS_INTEGER_OVERFLOW;
    }
    if (InterlockedCompareExchange64(
            state, static_cast<LONG64>(current + 1),
            static_cast<LONG64>(current)) != static_cast<LONG64>(current)) {
        ExReleaseRundownProtection(&Lease->Rundown);
        return STATUS_DEVICE_BUSY;
    }
    // The consumer acknowledges each block it read in ReaderSequence. Read
    // that user-writable value exactly once; it only feeds a diagnostic
    // counter and never sizes or addresses memory.
    ULONGLONG readerSequence = static_cast<ULONGLONG>(*reinterpret_cast<volatile LONG64*>(
        static_cast<UCHAR*>(view) + AR_BRIDGE_READER_SEQUENCE_OFFSET));
    if (AudioRouterRenderBlockWasOverrun(nextSequence, readerSequence)) {
        AddBridgeCounter(view, FIELD_OFFSET(AR_BRIDGE_STREAM_COUNTERS, OverrunFrames), Frames);
    }
    ULONGLONG sequence = static_cast<ULONGLONG>(
        InterlockedIncrement64(&Lease->NextSequence));
    if (sequence == 0) {
        InterlockedExchange64(state, static_cast<LONG64>(current + 2));
        ExReleaseRundownProtection(&Lease->Rundown);
        return STATUS_INTEGER_OVERFLOW;
    }
    AR_BRIDGE_BLOCK_HEADER header = { generation, sequence, Frames, Channels,
        static_cast<ULONG>(sampleCount * sizeof(DOUBLE)) };
    RtlCopyMemory(static_cast<UCHAR*>(view) + AR_BRIDGE_HEADER_OFFSET,
                  &header, sizeof(header));
    RtlCopyMemory(static_cast<UCHAR*>(view) + AR_BRIDGE_PAYLOAD_OFFSET,
                  Samples, sampleCount * sizeof(DOUBLE));
    KeMemoryBarrier();
    InterlockedExchange64(state, static_cast<LONG64>(current + 2));
    ExReleaseRundownProtection(&Lease->Rundown);
    return STATUS_SUCCESS;
}

static void RetireBridgeResources(
    _In_ AR_BRIDGE_LEASE_STATE* Lease,
    _In_opt_ PVOID MappedView,
    _In_opt_ PVOID SectionObject,
    _In_opt_ PMDL LockedMdl,
    _In_ BOOLEAN RundownStarted)
{
    if (RundownStarted) {
        ExWaitForRundownProtectionRelease(&Lease->Rundown);
    }
    if (MappedView != NULL) {
        // Rundown has drained every callback. Zero the complete logical view
        // before unpin/unmap so another session cannot inherit retained audio.
        if (LockedMdl != NULL) {
            RtlZeroMemory(MappedView, MmGetMdlByteCount(LockedMdl));
            MmUnlockPages(LockedMdl);
            IoFreeMdl(LockedMdl);
        }
        MmUnmapViewInSystemSpace(MappedView);
    }
    if (SectionObject != NULL) {
        ObDereferenceObject(SectionObject);
    }
}

static void ReleaseLeasesOwnedByFileObject(_In_opt_ PFILE_OBJECT FileObject)
{
    if (FileObject == NULL) {
        return;
    }
    BridgeControlGuard controlGuard;

    for (ULONG index = 0; index < AR_BRIDGE_LEASE_SLOTS; ++index) {
        AR_BRIDGE_LEASE_STATE* lease = &g_BridgeLeases[index];
        PVOID sectionObject = NULL;
        PMDL lockedMdl = NULL;
        PVOID mappedView = NULL;
        BOOLEAN rundownStarted = FALSE;
        KIRQL oldIrql;

        KeAcquireSpinLock(&lease->Lock, &oldIrql);
        if ((lease->Active || lease->Retiring) &&
            lease->OwnerFileObject == FileObject) {
            sectionObject = lease->SectionObject;
            lockedMdl = lease->LockedMdl;
            lease->LockedMdl = NULL;
            mappedView = InterlockedExchangePointer(&lease->MappedView, NULL);
            rundownStarted = mappedView != NULL;
            lease->RundownStarted = rundownStarted;
            lease->Retiring = rundownStarted;
            lease->SectionObject = NULL;
            SetBridgeMappedBytes(lease, 0);
            lease->Active = FALSE;
            lease->OwnerFileObject = NULL;
            lease->OwnerSessionId = 0;
            lease->LastHeartbeat100ns = 0;
        }
        KeReleaseSpinLock(&lease->Lock, oldIrql);

        if (mappedView != NULL || sectionObject != NULL) {
            RetireBridgeResources(lease, mappedView, sectionObject, lockedMdl,
                                  rundownStarted);
            KeAcquireSpinLock(&lease->Lock, &oldIrql);
            ClearBridgeRequest(lease);
            lease->Retiring = FALSE;
            KeReleaseSpinLock(&lease->Lock, oldIrql);
        }
    }
}

static AR_BRIDGE_LEASE_STATE* BridgeLeaseForBusDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction)
{
    ULONG slotIndex = 0;
    return NT_SUCCESS(AudioRouterGetLeaseSlotIndex(
        BusIndex, Direction, &slotIndex))
        ? &g_BridgeLeases[slotIndex]
        : NULL;
}

// OPEN owns the section reference, so the maintenance requests generated by
// the mapped Rust controller may either repeat the mapping pair or omit it.
// The pair is deliberately not part of the lease identity: a user handle is
// process-relative and the kernel already holds the referenced section object.
static BOOLEAN BridgeRequestsHaveSameLeaseIdentity(
    _In_ const AR_BRIDGE_OPEN_REQUEST* Left,
    _In_ const AR_BRIDGE_OPEN_REQUEST* Right)
{
    // Compare fields explicitly so compiler/ABI padding cannot become part of
    // the protocol identity. SectionHandle and MappingBytes are intentionally
    // omitted because maintenance requests may repeat or omit that pair.
    return Left->ProtocolMajor == Right->ProtocolMajor &&
        Left->ProtocolMinor == Right->ProtocolMinor &&
        Left->BusIdBytes == Right->BusIdBytes &&
        Left->Channels == Right->Channels &&
        Left->FramesPerQuantum == Right->FramesPerQuantum &&
        Left->Direction == Right->Direction &&
        Left->SampleRateHz == Right->SampleRateHz &&
        Left->LeaseMs == Right->LeaseMs &&
        Left->Generation == Right->Generation &&
        Left->Reserved2 == Right->Reserved2 &&
        RtlCompareMemory(Left->BusId, Right->BusId,
                         sizeof(Left->BusId)) == sizeof(Left->BusId);
}

NTSTATUS AudioRouterCopyLeaseBlockForDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _In_ ULONGLONG MinimumSequence,
    _Out_writes_(DestinationCapacitySamples) DOUBLE* Destination,
    _In_ SIZE_T DestinationCapacitySamples,
    _Out_ AR_BRIDGE_BLOCK_HEADER* Header,
    _Out_ ULONG* NonFiniteSamples)
{
    AR_BRIDGE_LEASE_STATE* lease = BridgeLeaseForBusDirection(BusIndex, Direction);
    if (lease == NULL) {
        if (NonFiniteSamples != NULL) { *NonFiniteSamples = 0; }
        return STATUS_INVALID_PARAMETER;
    }
    return AudioRouterCopyLeaseBlock(
        lease, MinimumSequence, Destination, DestinationCapacitySamples,
        Header, NonFiniteSamples);
}

// Callback-side counter update: rundown-protected, no lease spin lock, no
// allocation. A missing or retiring lease simply drops the sample of activity.
NTSTATUS AudioRouterRecordLeaseActivityForDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _In_ const AR_BRIDGE_STREAM_ACTIVITY* Activity)
{
    AR_BRIDGE_LEASE_STATE* lease = BridgeLeaseForBusDirection(BusIndex, Direction);
    if (lease == NULL || Activity == NULL ||
        !ExAcquireRundownProtection(&lease->Rundown)) {
        return STATUS_DEVICE_NOT_READY;
    }
    PVOID view = InterlockedCompareExchangePointer(&lease->MappedView, NULL, NULL);
    ULONG mappedBytes = static_cast<ULONG>(
        InterlockedCompareExchange(
            reinterpret_cast<volatile LONG*>(&lease->MappedBytes), 0, 0));
    if (view == NULL || mappedBytes < AR_BRIDGE_HEADER_BYTES ||
        LoadBridgeUshort(&lease->Request.Direction) != Direction) {
        ExReleaseRundownProtection(&lease->Rundown);
        return STATUS_DEVICE_NOT_READY;
    }
    AddBridgeCounter(view, FIELD_OFFSET(AR_BRIDGE_STREAM_COUNTERS, UnderrunFrames),
                     Activity->UnderrunFrames);
    AddBridgeCounter(view, FIELD_OFFSET(AR_BRIDGE_STREAM_COUNTERS, SequenceGaps),
                     Activity->SequenceGaps);
    AddBridgeCounter(view, FIELD_OFFSET(AR_BRIDGE_STREAM_COUNTERS, NonFiniteSamples),
                     Activity->NonFiniteSamples);
    AddBridgeCounter(view, FIELD_OFFSET(AR_BRIDGE_STREAM_COUNTERS, FormatMismatches),
                     Activity->FormatMismatches);
    InterlockedExchange64(
        BridgeCounter(view, FIELD_OFFSET(AR_BRIDGE_STREAM_COUNTERS, LastDevicePosition)),
        static_cast<LONG64>(Activity->DevicePositionFrames));
    InterlockedExchange64(
        BridgeCounter(view, FIELD_OFFSET(AR_BRIDGE_STREAM_COUNTERS, LastQpcTime)),
        static_cast<LONG64>(Activity->QpcTime));
    ExReleaseRundownProtection(&lease->Rundown);
    return STATUS_SUCCESS;
}

NTSTATUS AudioRouterPublishLeaseBlockForDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _In_ USHORT Frames,
    _In_ USHORT Channels,
    _In_reads_(SampleCapacity) const DOUBLE* Samples,
    _In_ SIZE_T SampleCapacity)
{
    AR_BRIDGE_LEASE_STATE* lease = BridgeLeaseForBusDirection(BusIndex, Direction);
    return lease == NULL
        ? STATUS_INVALID_PARAMETER
        : AudioRouterPublishLeaseBlock(
            lease, Frames, Channels, Samples, SampleCapacity);
}

NTSTATUS AudioRouterGetLeaseShapeForDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _Out_ USHORT* Frames,
    _Out_ USHORT* Channels,
    _Out_ ULONG* SampleRateHz,
    _Out_ ULONGLONG* Generation)
{
    if (Frames == NULL || Channels == NULL || SampleRateHz == NULL ||
        Generation == NULL) {
        return STATUS_INVALID_PARAMETER;
    }
    AR_BRIDGE_LEASE_STATE* lease = BridgeLeaseForBusDirection(BusIndex, Direction);
    if (lease == NULL || !ExAcquireRundownProtection(&lease->Rundown)) {
        return STATUS_DEVICE_NOT_READY;
    }
    PVOID view = InterlockedCompareExchangePointer(&lease->MappedView, NULL, NULL);
    if (view == NULL ||
        LoadBridgeUshort(&lease->Request.Direction) != Direction) {
        ExReleaseRundownProtection(&lease->Rundown);
        return STATUS_DEVICE_NOT_READY;
    }
    *Frames = LoadBridgeUshort(&lease->Request.FramesPerQuantum);
    *Channels = LoadBridgeUshort(&lease->Request.Channels);
    *SampleRateHz = LoadBridgeUlong(&lease->Request.SampleRateHz);
    *Generation = InterlockedCompareExchange64(
        reinterpret_cast<volatile LONG64*>(&lease->Request.Generation), 0, 0);
    ExReleaseRundownProtection(&lease->Rundown);
    return STATUS_SUCCESS;
}

//-----------------------------------------------------------------------------
// Referenced forward.
//-----------------------------------------------------------------------------

DRIVER_ADD_DEVICE AddDevice;

NTSTATUS
StartDevice
(
    _In_  PDEVICE_OBJECT,
    _In_  PIRP,
    _In_  PRESOURCELIST
);

_Dispatch_type_(IRP_MJ_PNP)
DRIVER_DISPATCH PnpHandler;

_Dispatch_type_(IRP_MJ_CREATE)
_Dispatch_type_(IRP_MJ_CLEANUP)
_Dispatch_type_(IRP_MJ_CLOSE)
DRIVER_DISPATCH BridgeControlCreateClose;

_Dispatch_type_(IRP_MJ_DEVICE_CONTROL)
DRIVER_DISPATCH BridgeControlDeviceControl;

UNICODE_STRING g_RegistryPath;      // This is used to store the registry settings path for the driver

//-----------------------------------------------------------------------------
// Functions
//-----------------------------------------------------------------------------

static NTSTATUS CompleteBridgeIrp(
    _In_ PIRP Irp,
    _In_ NTSTATUS Status,
    _In_ ULONG_PTR Information = 0)
{
    Irp->IoStatus.Status = Status;
    Irp->IoStatus.Information = NT_SUCCESS(Status) ? Information : 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return Status;
}

static void ReleaseLeasesOwnedByFileObject(_In_opt_ PFILE_OBJECT FileObject);

NTSTATUS BridgeControlCreateClose(_In_ PDEVICE_OBJECT, _In_ PIRP Irp)
{
    if (Irp == NULL) {
        return STATUS_INVALID_PARAMETER;
    }
    PIO_STACK_LOCATION stack = IoGetCurrentIrpStackLocation(Irp);
    if (stack == NULL) {
        return CompleteBridgeIrp(Irp, STATUS_INVALID_PARAMETER);
    }
    if (stack != NULL && (stack->MajorFunction == IRP_MJ_CLEANUP ||
                          stack->MajorFunction == IRP_MJ_CLOSE)) {
        // A client can terminate without sending the close IOCTL. Release
        // only leases claimed by this file object; other bridge owners remain
        // independent and are not disturbed.
        ReleaseLeasesOwnedByFileObject(stack->FileObject);
    }
    return CompleteBridgeIrp(Irp, STATUS_SUCCESS);
}

static NTSTATUS HandleBridgeControlRequest(
    _In_ PIRP Irp,
    _Out_ ULONG_PTR* Information)
{
    if (Information == NULL) {
        return STATUS_INVALID_PARAMETER;
    }
    *Information = 0;
    if (Irp == NULL) {
        return STATUS_INVALID_PARAMETER;
    }
    PIO_STACK_LOCATION stack = IoGetCurrentIrpStackLocation(Irp);
    if (stack == NULL || stack->FileObject == NULL) {
        return STATUS_INVALID_PARAMETER;
    }
    ULONG code = stack->Parameters.DeviceIoControl.IoControlCode;
    NTSTATUS status = STATUS_INVALID_DEVICE_REQUEST;

    if (code == IOCTL_AUDIOROUTER_BRIDGE_QUERY) {
        // Read-only capability report for status/repair. METHOD_BUFFERED:
        // the I/O manager copies exactly the validated output length back.
        status = AudioRouterValidateBridgeQueryLength(
            stack->Parameters.DeviceIoControl.InputBufferLength,
            stack->Parameters.DeviceIoControl.OutputBufferLength);
        if (!NT_SUCCESS(status) || Irp->AssociatedIrp.SystemBuffer == NULL ||
            Irp->RequestorMode != UserMode) {
            return STATUS_INVALID_PARAMETER;
        }
        AudioRouterFillDriverInfo(
            static_cast<PAR_BRIDGE_DRIVER_INFO>(Irp->AssociatedIrp.SystemBuffer),
            &g_BridgeConfig);
        *Information = sizeof(AR_BRIDGE_DRIVER_INFO);
        return STATUS_SUCCESS;
    }

    if (code == IOCTL_AUDIOROUTER_BRIDGE_OPEN ||
        code == IOCTL_AUDIOROUTER_BRIDGE_CLOSE ||
        code == IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT) {
        ULONG inputBytes = stack->Parameters.DeviceIoControl.InputBufferLength;
        if (Irp->AssociatedIrp.SystemBuffer == NULL || Irp->RequestorMode != UserMode) {
            return STATUS_INVALID_PARAMETER;
        }
        status = AudioRouterValidateBridgeRequestLength(
            inputBytes, stack->Parameters.DeviceIoControl.OutputBufferLength,
            code == IOCTL_AUDIOROUTER_BRIDGE_OPEN);
        if (!NT_SUCCESS(status)) {
            return status;
        }

        PAR_BRIDGE_OPEN_REQUEST request =
            static_cast<PAR_BRIDGE_OPEN_REQUEST>(Irp->AssociatedIrp.SystemBuffer);
        status = AudioRouterValidateBridgeOpenRequest(request);
        if (NT_SUCCESS(status) && inputBytes == sizeof(AR_BRIDGE_OPEN_REQUEST_EX)) {
            // The extension is never part of the lease identity; it only
            // negotiates transport options and must be fully understood.
            status = AudioRouterValidateBridgeOpenExtension(
                &static_cast<PAR_BRIDGE_OPEN_REQUEST_EX>(
                    Irp->AssociatedIrp.SystemBuffer)->Extension);
        }
        USHORT busIndex = 0;
        if (NT_SUCCESS(status)) {
            status = AudioRouterParseCableBusId(
                request->BusId, request->BusIdBytes, &busIndex);
            if (NT_SUCCESS(status) && busIndex >= g_EnabledCableCount) {
                status = STATUS_DEVICE_NOT_CONNECTED;
            }
            // MaxLeaseMs (17 §5.5) bounds how long a silent owner can hold a
            // cable; maintenance requests carry the same identity and value.
            if (NT_SUCCESS(status) &&
                !AudioRouterLeaseWithinConfig(request->LeaseMs, &g_BridgeConfig)) {
                status = STATUS_INVALID_PARAMETER;
            }
        }
        ULONG sessionId = 0;
        if (NT_SUCCESS(status)) { status = IoGetRequestorSessionId(Irp, &sessionId); }
        // OPEN must carry a section, otherwise it could publish an active
        // lease with no mapped view and fail only when audio first arrives.
        // Maintenance requests may repeat the mapping pair (as the mapped
        // Rust controller does) or omit it; the kernel uses its retained
        // section object and does not reference a user handle again.
        if (NT_SUCCESS(status) &&
            ((code == IOCTL_AUDIOROUTER_BRIDGE_OPEN &&
              request->SectionHandle == 0) ||
             (code == IOCTL_AUDIOROUTER_BRIDGE_OPEN &&
              request->SectionHandle != 0 && request->MappingBytes == 0))) {
            status = STATUS_INVALID_PARAMETER;
        }
        if (NT_SUCCESS(status)) {
            AR_BRIDGE_LEASE_STATE* lease =
                BridgeLeaseForBusDirection(busIndex, request->Direction);
            if (lease == NULL) {
                return STATUS_INVALID_PARAMETER;
            }
            PVOID sectionObject = NULL;
            PVOID mappedView = NULL;
            PMDL lockedMdl = NULL;
            SIZE_T mappedBytes = request->MappingBytes;
            // A section is acquired only for OPEN. CLOSE and HEARTBEAT are
            // lease operations and must validate the existing identity under
            // the lease lock without touching a user handle or mapping.
            if (code == IOCTL_AUDIOROUTER_BRIDGE_OPEN &&
                request->SectionHandle != 0) {
                SIZE_T requiredBytes = AR_BRIDGE_HEADER_BYTES +
                    static_cast<SIZE_T>(request->Channels) *
                    static_cast<SIZE_T>(request->FramesPerQuantum) * sizeof(DOUBLE);
                status = AudioRouterValidateMappingBytes(request);
                if (!NT_SUCCESS(status)) {
                    // Rejected before referencing a caller-provided handle.
                } else {
                    status = ObReferenceObjectByHandle(
                        reinterpret_cast<HANDLE>(static_cast<ULONG_PTR>(request->SectionHandle)),
                        SECTION_MAP_READ | SECTION_MAP_WRITE, *MmSectionObjectType, UserMode,
                        &sectionObject, NULL);
                    if (NT_SUCCESS(status)) {
                        status = MmMapViewInSystemSpace(
                            sectionObject, &mappedView, &mappedBytes);
                        // Logical bytes are exact; Windows returns page-rounded
                        // view bytes. Bound the physical mapping separately and
                        // never expose that padding to a callback's copy length.
                        SIZE_T roundedBytes = (requiredBytes + PAGE_SIZE - 1) & ~(static_cast<SIZE_T>(PAGE_SIZE) - 1);
                        if (NT_SUCCESS(status) && (mappedBytes < requiredBytes || mappedBytes > roundedBytes)) {
                            MmUnmapViewInSystemSpace(mappedView);
                            mappedView = NULL;
                            status = STATUS_BUFFER_TOO_SMALL;
                        }
                        if (NT_SUCCESS(status)) {
                            status = PinBridgeView(mappedView, static_cast<ULONG>(requiredBytes), &lockedMdl);
                        }
                        if (!NT_SUCCESS(status)) {
                            ObDereferenceObject(sectionObject);
                            sectionObject = NULL;
                        }
                    }
                }
            }
            if (!NT_SUCCESS(status)) {
                if (mappedView != NULL) {
                    MmUnmapViewInSystemSpace(mappedView);
                }
                if (sectionObject != NULL) {
                    ObDereferenceObject(sectionObject);
                }
                return status;
            }

            PVOID oldSectionObject = NULL;
            PVOID oldMappedView = NULL;
            PMDL oldLockedMdl = NULL;
            BOOLEAN oldRundownStarted = FALSE;
            BOOLEAN publishAfterRetire = FALSE;
            KIRQL oldIrql;
            KeAcquireSpinLock(&lease->Lock, &oldIrql);
            ULONGLONG now = KeQueryInterruptTime();
            // Expiry belongs to the stored lease, never the rival request.
            // Otherwise an attacker could send LeaseMs=1 to seize a live owner.
            ULONGLONG leaseTicks = static_cast<ULONGLONG>(lease->Request.LeaseMs) * _100NS_PER_MILLISECOND;
            BOOLEAN expired = lease->Active &&
                (now - lease->LastHeartbeat100ns > leaseTicks);

            if (code == IOCTL_AUDIOROUTER_BRIDGE_OPEN) {
                status = AudioRouterValidateNextGeneration(
                    lease->LastGeneration, request->Generation);
                if (!NT_SUCCESS(status)) {
                    // Each successful OPEN needs a fresh generation, even
                    // after CLOSE, so streams can detect turnover if they
                    // were not scheduled while the lease was inactive.
                } else if ((lease->Active && !expired) || lease->Retiring) {
                    status = AudioRouterOpenOwnershipStatus(true, lease->OwnerSessionId, sessionId);
                } else {
                    // All control changes are serialized. Section identity, not
                    // handle integer, detects duplicate handles to the same object.
                    for (ULONG slot = 0; slot < AR_BRIDGE_LEASE_SLOTS; ++slot) {
                        if (&g_BridgeLeases[slot] != lease &&
                            g_BridgeLeases[slot].SectionObject == sectionObject) {
                            status = STATUS_SHARING_VIOLATION;
                            break;
                        }
                    }
                    if (!NT_SUCCESS(status)) {
                        KeReleaseSpinLock(&lease->Lock, oldIrql);
                        if (lockedMdl != NULL) { MmUnlockPages(lockedMdl); IoFreeMdl(lockedMdl); }
                        MmUnmapViewInSystemSpace(mappedView);
                        ObDereferenceObject(sectionObject);
                        return status;
                    }
                    oldSectionObject = lease->SectionObject;
                    oldLockedMdl = lease->LockedMdl;
                    lease->LockedMdl = NULL;
                    BOOLEAN priorRundownStarted = lease->RundownStarted;
                    oldMappedView = InterlockedExchangePointer(
                        &lease->MappedView, NULL);
                    oldRundownStarted = oldMappedView != NULL;
                    lease->RundownStarted = oldRundownStarted;
                    SetBridgeMappedBytes(lease, 0);
                    lease->SectionObject = NULL;
                    lease->Active = FALSE;
                    lease->OwnerFileObject = NULL;
                    lease->OwnerSessionId = 0;
                    if (oldRundownStarted) {
                        lease->Retiring = TRUE;
                        // Reserve the replacement for this file object while
                        // callbacks drain. Cleanup must be able to cancel
                        // this in-flight OPEN before it publishes a mapping.
                        lease->OwnerFileObject = stack->FileObject;
                        lease->OwnerSessionId = sessionId;
                        publishAfterRetire = TRUE;
                    } else {
                        if (priorRundownStarted) {
                            ExReInitializeRundownProtection(&lease->Rundown);
                            lease->RundownStarted = FALSE;
                        }
                        // Ownership checks passed and no callback can see this
                        // view yet: reset counters, announce float64 samples.
                        InitializeBridgeViewHeader(mappedView);
                        PublishBridgeRequest(lease, request);
                        lease->LastGeneration = request->Generation;
                        lease->OwnerFileObject = stack->FileObject;
                        lease->OwnerSessionId = sessionId;
                        lease->LastHeartbeat100ns = now;
                        lease->Active = TRUE;
                        lease->SectionObject = sectionObject;
                        lease->LockedMdl = lockedMdl;
                        lease->MappedView = mappedView;
                        SetBridgeMappedBytes(
                            lease, static_cast<ULONG>(request->MappingBytes));
                        InterlockedExchange64(&lease->NextSequence, 0);
                        sectionObject = NULL;
                        mappedView = NULL;
                        lockedMdl = NULL;
                        status = STATUS_SUCCESS;
                    }
                }
            } else if (lease->Active && !expired && !lease->Retiring &&
                       (lease->OwnerFileObject != stack->FileObject || lease->OwnerSessionId != sessionId)) {
                // A live lease belongs to the handle that opened it. Keep
                // ownership failures distinct from an expired or invalidated
                // lease so user mode can report authorization separately.
                status = STATUS_ACCESS_DENIED;
            } else if (!lease->Active || expired || lease->Retiring ||
                       lease->OwnerFileObject != stack->FileObject ||
                       lease->OwnerSessionId != sessionId ||
                       !BridgeRequestsHaveSameLeaseIdentity(
                           &lease->Request, request)) {
                // Expiry is terminal for the mapped callback view. Detach it
                // before returning the rejected maintenance request, then
                // wait for any callback reader before unmapping below.
                if (expired && lease->Active && !lease->Retiring) {
                    oldSectionObject = lease->SectionObject;
                    oldLockedMdl = lease->LockedMdl;
                    lease->LockedMdl = NULL;
                    oldMappedView = InterlockedExchangePointer(
                        &lease->MappedView, NULL);
                    oldRundownStarted = oldMappedView != NULL;
                    lease->RundownStarted = oldRundownStarted;
                    lease->SectionObject = NULL;
                    SetBridgeMappedBytes(lease, 0);
                    lease->Active = FALSE;
                    lease->OwnerFileObject = NULL;
                    lease->OwnerSessionId = 0;
                    lease->Retiring = oldRundownStarted;
                    if (!oldRundownStarted) {
                        ClearBridgeRequest(lease);
                    }
                }
                status = STATUS_INVALID_DEVICE_STATE;
            } else if (code == IOCTL_AUDIOROUTER_BRIDGE_CLOSE) {
                oldSectionObject = lease->SectionObject;
                oldLockedMdl = lease->LockedMdl;
                lease->LockedMdl = NULL;
                oldMappedView = InterlockedExchangePointer(
                    &lease->MappedView, NULL);
                oldRundownStarted = oldMappedView != NULL;
                lease->RundownStarted = oldRundownStarted;
                lease->Retiring = oldRundownStarted;
                lease->Active = FALSE;
                lease->OwnerFileObject = NULL;
                lease->OwnerSessionId = 0;
                lease->LastHeartbeat100ns = 0;
                lease->SectionObject = NULL;
                SetBridgeMappedBytes(lease, 0);
                status = STATUS_SUCCESS;
            } else {
                lease->LastHeartbeat100ns = now;
                status = STATUS_SUCCESS;
            }
            KeReleaseSpinLock(&lease->Lock, oldIrql);
            if (publishAfterRetire) {
                RetireBridgeResources(lease, oldMappedView, oldSectionObject, oldLockedMdl,
                                      oldRundownStarted);
                oldMappedView = NULL;
                oldSectionObject = NULL;
                KeAcquireSpinLock(&lease->Lock, &oldIrql);
                if (lease->Retiring &&
                    lease->OwnerFileObject == stack->FileObject &&
                    lease->RundownStarted == oldRundownStarted) {
                    ExReInitializeRundownProtection(&lease->Rundown);
                    lease->RundownStarted = FALSE;
                    InitializeBridgeViewHeader(mappedView);
                    PublishBridgeRequest(lease, request);
                    lease->LastGeneration = request->Generation;
                    lease->OwnerFileObject = stack->FileObject;
                    lease->OwnerSessionId = sessionId;
                    lease->LastHeartbeat100ns = now;
                    lease->SectionObject = sectionObject;
                    lease->LockedMdl = lockedMdl;
                    SetBridgeMappedBytes(
                        lease, static_cast<ULONG>(request->MappingBytes));
                    InterlockedExchange64(&lease->NextSequence, 0);
                    KeMemoryBarrier();
                    lease->MappedView = mappedView;
                    lease->Active = TRUE;
                    lease->Retiring = FALSE;
                    sectionObject = NULL;
                    mappedView = NULL;
                    lockedMdl = NULL;
                    status = STATUS_SUCCESS;
                } else {
                    // Cleanup or a competing owner won while the old view
                    // drained. Do not resurrect a lease for a closed handle.
                    status = STATUS_INVALID_DEVICE_STATE;
                }
                KeReleaseSpinLock(&lease->Lock, oldIrql);
            } else if (oldMappedView != NULL || oldSectionObject != NULL) {
                RetireBridgeResources(lease, oldMappedView, oldSectionObject, oldLockedMdl,
                                      oldRundownStarted);
                // `oldRundownStarted` is the state captured while holding
                // the lease lock; do not inspect the mutable lease flag after
                // releasing the lock and waiting for callback readers.
                if (oldRundownStarted) {
                    KeAcquireSpinLock(&lease->Lock, &oldIrql);
                    ClearBridgeRequest(lease);
                    lease->Retiring = FALSE;
                    KeReleaseSpinLock(&lease->Lock, oldIrql);
                }
            }
            if (mappedView != NULL) {
                if (lockedMdl != NULL) { MmUnlockPages(lockedMdl); IoFreeMdl(lockedMdl); }
                MmUnmapViewInSystemSpace(mappedView);
            }
            if (sectionObject != NULL) {
                ObDereferenceObject(sectionObject);
            }
        }
    }

    return status;
}

NTSTATUS BridgeControlDeviceControl(_In_ PDEVICE_OBJECT, _In_ PIRP Irp)
{
    if (Irp == NULL) { return STATUS_INVALID_PARAMETER; }
    NTSTATUS status;
    ULONG_PTR information = 0;
    {
        BridgeControlGuard controlGuard;
        status = HandleBridgeControlRequest(Irp, &information);
    }
    return CompleteBridgeIrp(Irp, status, information);
}

NTSTATUS CreateBridgeControlDevice(_In_ PDRIVER_OBJECT DriverObject)
{
    UNICODE_STRING deviceName;
    UNICODE_STRING dosName;
    RtlInitUnicodeString(&deviceName, AUDIOROUTER_BRIDGE_DEVICE_NAME);
    RtlInitUnicodeString(&dosName, AUDIOROUTER_BRIDGE_DOS_NAME);

    NTSTATUS status = IoCreateDeviceSecure(
        DriverObject, 0, &deviceName, FILE_DEVICE_UNKNOWN,
        FILE_DEVICE_SECURE_OPEN, FALSE,
        &AUDIOROUTER_BRIDGE_DEVICE_SDDL,
        &PID_AUDIOROUTERVIRTUAL, &g_BridgeControlDevice);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    status = IoCreateSymbolicLink(&dosName, &deviceName);
    if (!NT_SUCCESS(status)) {
        IoDeleteDevice(g_BridgeControlDevice);
        g_BridgeControlDevice = NULL;
        return status;
    }
    g_BridgeControlDevice->Flags &= ~DO_DEVICE_INITIALIZING;
    return STATUS_SUCCESS;
}

void DeleteBridgeControlDevice()
{
    BridgeControlGuard controlGuard;
    UNICODE_STRING dosName;
    RtlInitUnicodeString(&dosName, AUDIOROUTER_BRIDGE_DOS_NAME);

    for (ULONG index = 0; index < AR_BRIDGE_LEASE_SLOTS; ++index) {
        PVOID sectionObject = NULL;
        PVOID mappedView = NULL;
        PMDL lockedMdl = NULL;
        BOOLEAN rundownStarted = FALSE;
        KIRQL oldIrql;
        KeAcquireSpinLock(&g_BridgeLeases[index].Lock, &oldIrql);
        sectionObject = g_BridgeLeases[index].SectionObject;
        lockedMdl = g_BridgeLeases[index].LockedMdl;
        g_BridgeLeases[index].LockedMdl = NULL;
        mappedView = InterlockedExchangePointer(
            &g_BridgeLeases[index].MappedView, NULL);
        rundownStarted = mappedView != NULL;
        g_BridgeLeases[index].RundownStarted = rundownStarted;
        g_BridgeLeases[index].Retiring = rundownStarted;
        g_BridgeLeases[index].SectionObject = NULL;
        g_BridgeLeases[index].MappedView = NULL;
        SetBridgeMappedBytes(&g_BridgeLeases[index], 0);
        g_BridgeLeases[index].Active = FALSE;
        g_BridgeLeases[index].OwnerFileObject = NULL;
        g_BridgeLeases[index].OwnerSessionId = 0;
        g_BridgeLeases[index].LastHeartbeat100ns = 0;
        KeReleaseSpinLock(&g_BridgeLeases[index].Lock, oldIrql);

        RetireBridgeResources(&g_BridgeLeases[index], mappedView,
                              sectionObject, lockedMdl, rundownStarted);

        // A callback that acquired rundown before detachment may still be
        // reading Request. Clear the contract only after that reader has
        // drained; this is the unload equivalent of close/expiry cleanup.
        KeAcquireSpinLock(&g_BridgeLeases[index].Lock, &oldIrql);
        ClearBridgeRequest(&g_BridgeLeases[index]);
        g_BridgeLeases[index].Retiring = FALSE;
        KeReleaseSpinLock(&g_BridgeLeases[index].Lock, oldIrql);
    }
    if (g_BridgeControlDevice != NULL) {
        IoDeleteSymbolicLink(&dosName);
        IoDeleteDevice(g_BridgeControlDevice);
        g_BridgeControlDevice = NULL;
    }
}

#pragma code_seg("PAGE")
void ReleaseRegistryStringBuffer()
{
    PAGED_CODE();

    if (g_RegistryPath.Buffer != NULL)
    {
        ExFreePool(g_RegistryPath.Buffer);
        g_RegistryPath.Buffer = NULL;
        g_RegistryPath.Length = 0;
        g_RegistryPath.MaximumLength = 0;
    }
}

//=============================================================================
#pragma code_seg("PAGE")
extern "C"
void DriverUnload
(
    _In_ PDRIVER_OBJECT DriverObject
)
/*++

Routine Description:

  Our driver unload routine. This just frees the WDF driver object.

Arguments:

  DriverObject - pointer to the driver object

Environment:

    PASSIVE_LEVEL

--*/
{
    PAGED_CODE();

    DPF(D_TERSE, ("[DriverUnload]"));

    DeleteBridgeControlDevice();
    ReleaseRegistryStringBuffer();

    if (DriverObject == NULL)
    {
        goto Done;
    }

    //
    // Invoke first the port unload.
    //
    if (gPCDriverUnloadRoutine != NULL)
    {
        gPCDriverUnloadRoutine(DriverObject);
    }

    //
    // Unload WDF driver object.
    //
    if (WdfGetDriver() != NULL)
    {
        WdfDriverMiniportUnload(WdfGetDriver());
    }
Done:
    return;
}

//=============================================================================
#pragma code_seg("INIT")
__drv_requiresIRQL(PASSIVE_LEVEL)
NTSTATUS
CopyRegistrySettingsPath(
    _In_ PUNICODE_STRING RegistryPath
)
/*++

Routine Description:

Copies the following registry path to a global variable.

\REGISTRY\MACHINE\SYSTEM\ControlSetxxx\Services\<driver>\Parameters

Arguments:

RegistryPath - Registry path passed to DriverEntry

Returns:

NTSTATUS - SUCCESS if able to configure the framework

--*/

{
    // Initializing the unicode string, so that if it is not allocated it will not be deallocated too.
    RtlInitUnicodeString(&g_RegistryPath, NULL);

    g_RegistryPath.MaximumLength = RegistryPath->Length + sizeof(WCHAR);

    g_RegistryPath.Buffer = (PWCH)ExAllocatePool2(POOL_FLAG_PAGED, g_RegistryPath.MaximumLength, MINADAPTER_POOLTAG);

    if (g_RegistryPath.Buffer == NULL)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    RtlAppendUnicodeToString(&g_RegistryPath, RegistryPath->Buffer);

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg("INIT")
__drv_requiresIRQL(PASSIVE_LEVEL)
NTSTATUS
GetRegistrySettings(
    _In_ PUNICODE_STRING RegistryPath
   )
/*++

Routine Description:

    Initialize Driver Framework settings from the driver
    specific registry settings under

    \REGISTRY\MACHINE\SYSTEM\ControlSetxxx\Services\<driver>\Parameters

Arguments:

    RegistryPath - Registry path passed to DriverEntry

Returns:

    NTSTATUS - SUCCESS if able to configure the framework

--*/

{
    NTSTATUS                    ntStatus;
    PDRIVER_OBJECT              DriverObject;
    HANDLE                      DriverKey;
    RTL_QUERY_REGISTRY_TABLE    paramTable[] = {
    // QueryRoutine     Flags                                               Name                     EntryContext             DefaultType                                                    DefaultData              DefaultLength
        { NULL,   0,                                                        NULL,                    NULL,                    0,                                                             NULL,                    0}
    };

    DPF(D_TERSE, ("[GetRegistrySettings]"));

    PAGED_CODE();
    UNREFERENCED_PARAMETER(RegistryPath);

    DriverObject = WdfDriverWdmGetDriverObject(WdfGetDriver());
    DriverKey = NULL;
    ntStatus = IoOpenDriverRegistryKey(DriverObject,
                                 DriverRegKeyParameters,
                                 KEY_READ,
                                 0,
                                 &DriverKey);

    if (!NT_SUCCESS(ntStatus))
    {
        return ntStatus;
    }

    ntStatus = RtlQueryRegistryValues(RTL_REGISTRY_HANDLE,
                                  (PCWSTR) DriverKey,
                                  &paramTable[0],
                                  NULL,
                                  NULL);

    if (!NT_SUCCESS(ntStatus))
    {
        DPF(D_VERBOSE, ("RtlQueryRegistryValues failed, using default values, 0x%x", ntStatus));
        //
        // Don't return error because we will operate with default values.
        //
    }

    //
    // Dump settings.
    //

    if (DriverKey)
    {
        ZwClose(DriverKey);
    }

    return STATUS_SUCCESS;
}

#pragma code_seg("INIT")
extern "C" DRIVER_INITIALIZE DriverEntry;
extern "C" NTSTATUS
DriverEntry
(
    _In_  PDRIVER_OBJECT          DriverObject,
    _In_  PUNICODE_STRING         RegistryPathName
)
{
/*++

Routine Description:

  Installable driver initialization entry point.
  This entry point is called directly by the I/O system.

  All audio adapter drivers can use this code without change.

Arguments:

  DriverObject - pointer to the driver object

  RegistryPath - pointer to a unicode string representing the path,
                   to driver-specific key in the registry.

Return Value:

  STATUS_SUCCESS if successful,
  STATUS_UNSUCCESSFUL otherwise.

--*/
    NTSTATUS                    ntStatus;
    WDF_DRIVER_CONFIG           config;

    DPF(D_TERSE, ("[DriverEntry]"));
    ExInitializePushLock(&g_BridgeControlLock);

    // Copy registry Path name in a global variable to be used by modules inside driver.
    // !! NOTE !! Inside this function we are initializing the registrypath, so we MUST NOT add any failing calls
    // before the following call.
    ntStatus = CopyRegistrySettingsPath(RegistryPathName);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("Registry path copy error 0x%x", ntStatus)),
        Done);

    WDF_DRIVER_CONFIG_INIT(&config, WDF_NO_EVENT_CALLBACK);
    for (ULONG index = 0; index < AR_BRIDGE_LEASE_SLOTS; ++index) {
        KeInitializeSpinLock(&g_BridgeLeases[index].Lock);
        ExInitializeRundownProtection(&g_BridgeLeases[index].Rundown);
    }
    //
    // Set WdfDriverInitNoDispatchOverride flag to tell the framework
    // not to provide dispatch routines for the driver. In other words,
    // the framework must not intercept IRPs that the I/O manager has
    // directed to the driver. In this case, they will be handled by Audio
    // port driver.
    //
    config.DriverInitFlags |= WdfDriverInitNoDispatchOverride;
    config.DriverPoolTag    = MINADAPTER_POOLTAG;

    ntStatus = WdfDriverCreate(DriverObject,
                               RegistryPathName,
                               WDF_NO_OBJECT_ATTRIBUTES,
                               &config,
                               WDF_NO_HANDLE);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("WdfDriverCreate failed, 0x%x", ntStatus)),
        Done);

    ntStatus = CreateBridgeControlDevice(DriverObject);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("CreateBridgeControlDevice failed, 0x%x", ntStatus)),
        Done);
    DriverObject->MajorFunction[IRP_MJ_CREATE] = BridgeControlCreateClose;
    DriverObject->MajorFunction[IRP_MJ_CLEANUP] = BridgeControlCreateClose;
    DriverObject->MajorFunction[IRP_MJ_CLOSE] = BridgeControlCreateClose;
    DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL] = BridgeControlDeviceControl;

    //
    // Get registry configuration.
    //
    ntStatus = GetRegistrySettings(RegistryPathName);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("Registry Configuration error 0x%x", ntStatus)),
        Done);

    //
    // Tell the class driver to initialize the driver.
    //
    ntStatus =  PcInitializeAdapterDriver(DriverObject,
                                          RegistryPathName,
                                          (PDRIVER_ADD_DEVICE)AddDevice);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("PcInitializeAdapterDriver failed, 0x%x", ntStatus)),
        Done);

    //
    // To intercept stop/remove/surprise-remove.
    //
    DriverObject->MajorFunction[IRP_MJ_PNP] = PnpHandler;

    //
    // Hook the port class unload function
    //
    gPCDriverUnloadRoutine = DriverObject->DriverUnload;
    DriverObject->DriverUnload = DriverUnload;

    //
    // All done.
    //
    ntStatus = STATUS_SUCCESS;

Done:

    if (!NT_SUCCESS(ntStatus))
    {
        DeleteBridgeControlDevice();
        if (WdfGetDriver() != NULL)
        {
            WdfDriverMiniportUnload(WdfGetDriver());
        }

        ReleaseRegistryStringBuffer();
    }

    return ntStatus;
} // DriverEntry

#pragma code_seg()
// disable prefast warning 28152 because
// DO_DEVICE_INITIALIZING is cleared in PcAddAdapterDevice
#pragma warning(disable:28152)
#pragma code_seg("PAGE")
//=============================================================================
NTSTATUS AddDevice
(
    _In_  PDRIVER_OBJECT    DriverObject,
    _In_  PDEVICE_OBJECT    PhysicalDeviceObject
)
/*++

Routine Description:

  The Plug & Play subsystem is handing us a brand new PDO, for which we
  (by means of INF registration) have been asked to provide a driver.

  We need to determine if we need to be in the driver stack for the device.
  Create a function device object to attach to the stack
  Initialize that device object
  Return status success.

  All audio adapter drivers can use this code without change.

Arguments:

  DriverObject - pointer to a driver object

  PhysicalDeviceObject -  pointer to a device object created by the
                            underlying bus driver.

Return Value:

  NT status code.

--*/
{
    PAGED_CODE();

    NTSTATUS        ntStatus;
    ULONG           maxObjects;

    DPF(D_TERSE, ("[AddDevice]"));

    maxObjects = g_MaxMiniports;

    // Tell the class driver to add the device.
    //
    ntStatus =
        PcAddAdapterDevice
        (
            DriverObject,
            PhysicalDeviceObject,
            PCPFNSTARTDEVICE(StartDevice),
            maxObjects,
            0
        );

    return ntStatus;
} // AddDevice

#pragma code_seg()
NTSTATUS
_IRQL_requires_max_(DISPATCH_LEVEL)
PowerControlCallback
(
    _In_        LPCGUID PowerControlCode,
    _In_opt_    PVOID   InBuffer,
    _In_        SIZE_T  InBufferSize,
    _Out_writes_bytes_to_(OutBufferSize, *BytesReturned) PVOID OutBuffer,
    _In_        SIZE_T  OutBufferSize,
    _Out_opt_   PSIZE_T BytesReturned,
    _In_opt_    PVOID   Context
)
{
    UNREFERENCED_PARAMETER(PowerControlCode);
    UNREFERENCED_PARAMETER(InBuffer);
    UNREFERENCED_PARAMETER(InBufferSize);
    UNREFERENCED_PARAMETER(OutBuffer);
    UNREFERENCED_PARAMETER(OutBufferSize);
    UNREFERENCED_PARAMETER(BytesReturned);
    UNREFERENCED_PARAMETER(Context);

    return STATUS_NOT_IMPLEMENTED;
}

#pragma code_seg("PAGE")
NTSTATUS
InstallEndpointRenderFilters(
    _In_ PDEVICE_OBJECT     _pDeviceObject,
    _In_ PIRP               _pIrp,
    _In_ PADAPTERCOMMON     _pAdapterCommon,
    _In_ PENDPOINT_MINIPAIR _pAeMiniports
    )
{
    NTSTATUS                    ntStatus                = STATUS_SUCCESS;
    PUNKNOWN                    unknownTopology         = NULL;
    PUNKNOWN                    unknownWave             = NULL;
    PPORTCLSETWHELPER           pPortClsEtwHelper       = NULL;
#ifdef _USE_IPortClsRuntimePower
    PPORTCLSRUNTIMEPOWER        pPortClsRuntimePower    = NULL;
#endif // _USE_IPortClsRuntimePower
    PPORTCLSStreamResourceManager pPortClsResMgr        = NULL;
    PPORTCLSStreamResourceManager2 pPortClsResMgr2      = NULL;

    PAGED_CODE();

    UNREFERENCED_PARAMETER(_pDeviceObject);

    ntStatus = _pAdapterCommon->InstallEndpointFilters(
        _pIrp,
        _pAeMiniports,
        NULL,
        &unknownTopology,
        &unknownWave,
        NULL, NULL);

    if (unknownWave) // IID_IPortClsEtwHelper and IID_IPortClsRuntimePower interfaces are only exposed on the WaveRT port.
    {
        ntStatus = unknownWave->QueryInterface (IID_IPortClsEtwHelper, (PVOID *)&pPortClsEtwHelper);
        if (NT_SUCCESS(ntStatus))
        {
            _pAdapterCommon->SetEtwHelper(pPortClsEtwHelper);
            ASSERT(pPortClsEtwHelper != NULL);
            pPortClsEtwHelper->Release();
        }

#ifdef _USE_IPortClsRuntimePower
        // Let's get the runtime power interface on PortCls.
        ntStatus = unknownWave->QueryInterface(IID_IPortClsRuntimePower, (PVOID *)&pPortClsRuntimePower);
        if (NT_SUCCESS(ntStatus))
        {
            // This interface would typically be stashed away for later use.  Instead,
            // let's just send an empty control with GUID_NULL.
            NTSTATUS ntStatusTest =
                pPortClsRuntimePower->SendPowerControl
                (
                    _pDeviceObject,
                    &GUID_NULL,
                    NULL,
                    0,
                    NULL,
                    0,
                    NULL
                );

            if (NT_SUCCESS(ntStatusTest) || STATUS_NOT_IMPLEMENTED == ntStatusTest || STATUS_NOT_SUPPORTED == ntStatusTest)
            {
                ntStatus = pPortClsRuntimePower->RegisterPowerControlCallback(_pDeviceObject, &PowerControlCallback, NULL);
                if (NT_SUCCESS(ntStatus))
                {
                    ntStatus = pPortClsRuntimePower->UnregisterPowerControlCallback(_pDeviceObject);
                }
            }
            else
            {
                ntStatus = ntStatusTest;
            }

            pPortClsRuntimePower->Release();
        }
#endif // _USE_IPortClsRuntimePower

        //
        // Test: add and remove current thread as streaming audio resource.
        // In a real driver you should only add interrupts and driver-owned threads
        // (i.e., do NOT add the current thread as streaming resource).
        //
        // testing IPortClsStreamResourceManager:
        ntStatus = unknownWave->QueryInterface(IID_IPortClsStreamResourceManager, (PVOID *)&pPortClsResMgr);
        if (NT_SUCCESS(ntStatus))
        {
            PCSTREAMRESOURCE_DESCRIPTOR res;
            PCSTREAMRESOURCE hRes = NULL;
            PDEVICE_OBJECT pdo = NULL;

            PcGetPhysicalDeviceObject(_pDeviceObject, &pdo);
            PCSTREAMRESOURCE_DESCRIPTOR_INIT(&res);
            res.Pdo = pdo;
            res.Type = ePcStreamResourceThread;
            res.Resource.Thread = PsGetCurrentThread();

            NTSTATUS ntStatusTest = pPortClsResMgr->AddStreamResource(NULL, &res, &hRes);
            if (NT_SUCCESS(ntStatusTest))
            {
                pPortClsResMgr->RemoveStreamResource(hRes);
                hRes = NULL;
            }

            pPortClsResMgr->Release();
            pPortClsResMgr = NULL;
        }

        // testing IPortClsStreamResourceManager2:
        ntStatus = unknownWave->QueryInterface(IID_IPortClsStreamResourceManager2, (PVOID *)&pPortClsResMgr2);
        if (NT_SUCCESS(ntStatus))
        {
            PCSTREAMRESOURCE_DESCRIPTOR res;
            PCSTREAMRESOURCE hRes = NULL;
            PDEVICE_OBJECT pdo = NULL;

            PcGetPhysicalDeviceObject(_pDeviceObject, &pdo);
            PCSTREAMRESOURCE_DESCRIPTOR_INIT(&res);
            res.Pdo = pdo;
            res.Type = ePcStreamResourceThread;
            res.Resource.Thread = PsGetCurrentThread();

            NTSTATUS ntStatusTest = pPortClsResMgr2->AddStreamResource2(pdo, NULL, &res, &hRes);
            if (NT_SUCCESS(ntStatusTest))
            {
                pPortClsResMgr2->RemoveStreamResource(hRes);
                hRes = NULL;
            }

            pPortClsResMgr2->Release();
            pPortClsResMgr2 = NULL;
        }
    }

    SAFE_RELEASE(unknownTopology);
    SAFE_RELEASE(unknownWave);

    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS
InstallAllRenderFilters(
    _In_ PDEVICE_OBJECT _pDeviceObject,
    _In_ PIRP           _pIrp,
    _In_ PADAPTERCOMMON _pAdapterCommon
    )
{
    NTSTATUS            ntStatus;
    PENDPOINT_MINIPAIR* ppAeMiniports   = g_RenderEndpoints;

    PAGED_CODE();

    for(ULONG i = 0; i < g_EnabledCableCount; ++i, ++ppAeMiniports)
    {
        ntStatus = InstallEndpointRenderFilters(_pDeviceObject, _pIrp, _pAdapterCommon, *ppAeMiniports);
        IF_FAILED_JUMP(ntStatus, Exit);
    }

    ntStatus = STATUS_SUCCESS;

Exit:
    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS
InstallEndpointCaptureFilters(
    _In_ PDEVICE_OBJECT     _pDeviceObject,
    _In_ PIRP               _pIrp,
    _In_ PADAPTERCOMMON     _pAdapterCommon,
    _In_ PENDPOINT_MINIPAIR _pAeMiniports
)
{
    NTSTATUS    ntStatus = STATUS_SUCCESS;

    PAGED_CODE();

    UNREFERENCED_PARAMETER(_pDeviceObject);

    ntStatus = _pAdapterCommon->InstallEndpointFilters(
        _pIrp,
        _pAeMiniports,
        NULL,
        NULL,
        NULL,
        NULL, NULL);

    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS
InstallAllCaptureFilters(
    _In_ PDEVICE_OBJECT _pDeviceObject,
    _In_ PIRP           _pIrp,
    _In_ PADAPTERCOMMON _pAdapterCommon
)
{
    NTSTATUS            ntStatus;
    PENDPOINT_MINIPAIR* ppAeMiniports = g_CaptureEndpoints;

    PAGED_CODE();

    for (ULONG i = 0; i < g_EnabledCableCount; ++i, ++ppAeMiniports)
    {
        ntStatus = InstallEndpointCaptureFilters(_pDeviceObject, _pIrp, _pAdapterCommon, *ppAeMiniports);
        IF_FAILED_JUMP(ntStatus, Exit);
    }

    ntStatus = STATUS_SUCCESS;

Exit:
    return ntStatus;
}

// Read one REG_DWORD from the open hardware key. Any other type or size,
// or a missing value, reports "not present" so the caller's default wins.
static bool ReadConfigDword(_In_ HANDLE Key, _In_ PCWSTR Name, _Out_ ULONG* Value)
{
    PAGED_CODE();
    *Value = 0;
    UNICODE_STRING valueName;
    RtlInitUnicodeString(&valueName, Name);
    UCHAR storage[FIELD_OFFSET(KEY_VALUE_PARTIAL_INFORMATION, Data) + sizeof(ULONG)] = {};
    ULONG resultBytes = 0;
    NTSTATUS status = ZwQueryValueKey(Key, &valueName, KeyValuePartialInformation,
        storage, sizeof(storage), &resultBytes);
    if (!NT_SUCCESS(status)) {
        return false;
    }
    auto value = reinterpret_cast<PKEY_VALUE_PARTIAL_INFORMATION>(storage);
    if (value->Type != REG_DWORD || value->DataLength != sizeof(ULONG)) {
        return false;
    }
    *Value = *reinterpret_cast<PULONG>(value->Data);
    return true;
}

// 17 §5.5: configuration lives in this adapter's hardware key (HKR), written
// by the INF (CableCount default) and the elevated helper. Every value is
// range-checked; anything invalid falls back to the compiled default.
static AR_BRIDGE_CONFIG ReadBridgeConfig(_In_ PDEVICE_OBJECT DeviceObject)
{
    PAGED_CODE();

    AR_BRIDGE_CONFIG config;
    AudioRouterDefaultConfig(&config);
    PDEVICE_OBJECT pdo = NULL;
    HANDLE key = NULL;
    if (!NT_SUCCESS(PcGetPhysicalDeviceObject(DeviceObject, &pdo)) || pdo == NULL ||
        !NT_SUCCESS(IoOpenDeviceRegistryKey(pdo, PLUGPLAY_REGKEY_DEVICE,
            KEY_QUERY_VALUE, &key))) {
        return config;
    }
    ULONG value = 0;
    bool present = ReadConfigDword(key, L"CableCount", &value);
    config.CableCount = AudioRouterConfigValue(present, value, 1,
        ARRAYSIZE(g_RenderEndpoints), AR_CONFIG_CABLE_COUNT_DEFAULT);
    present = ReadConfigDword(key, L"MinPeriodFrames", &value);
    config.MinPeriodFrames = AudioRouterConfigValue(present, value,
        AR_CONFIG_MIN_PERIOD_FRAMES_LOW, AR_CONFIG_MIN_PERIOD_FRAMES_HIGH,
        AR_CONFIG_MIN_PERIOD_FRAMES_DEFAULT);
    present = ReadConfigDword(key, L"DefaultPeriodFrames", &value);
    config.DefaultPeriodFrames = AudioRouterConfigValue(present, value,
        AR_CONFIG_DEFAULT_PERIOD_FRAMES_LOW, AR_CONFIG_DEFAULT_PERIOD_FRAMES_HIGH,
        AR_CONFIG_DEFAULT_PERIOD_FRAMES_DEFAULT);
    present = ReadConfigDword(key, L"MaxLeaseMs", &value);
    config.MaxLeaseMs = AudioRouterConfigValue(present, value,
        AR_CONFIG_MAX_LEASE_MS_LOW, AR_BRIDGE_MAX_LEASE_MS,
        AR_CONFIG_MAX_LEASE_MS_DEFAULT);
    ZwClose(key);
    AudioRouterNormalizeConfig(&config);
    return config;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
StartDevice
(
    _In_  PDEVICE_OBJECT          DeviceObject,
    _In_  PIRP                    Irp,
    _In_  PRESOURCELIST           ResourceList
)
{
/*++

Routine Description:

  This function is called by the operating system when the device is
  started.
  It is responsible for starting the miniports.  This code is specific to
  the adapter because it calls out miniports for functions that are specific
  to the adapter.

Arguments:

  DeviceObject - pointer to the driver object

  Irp - pointer to the irp

  ResourceList - pointer to the resource list assigned by PnP manager

Return Value:

  NT status code.

--*/
    UNREFERENCED_PARAMETER(ResourceList);

    PAGED_CODE();

    ASSERT(DeviceObject);
    ASSERT(Irp);
    ASSERT(ResourceList);

    NTSTATUS                    ntStatus        = STATUS_SUCCESS;

    PADAPTERCOMMON              pAdapterCommon  = NULL;
    PUNKNOWN                    pUnknownCommon  = NULL;
    PortClassDeviceContext*     pExtension      = static_cast<PortClassDeviceContext*>(DeviceObject->DeviceExtension);

    DPF_ENTER(("[StartDevice]"));

    //
    // create a new adapter common object
    //
    ntStatus = NewAdapterCommon(
                                &pUnknownCommon,
                                IID_IAdapterCommon,
                                NULL,
                                POOL_FLAG_NON_PAGED
                                );
    IF_FAILED_JUMP(ntStatus, Exit);

    ntStatus = pUnknownCommon->QueryInterface( IID_IAdapterCommon,(PVOID *) &pAdapterCommon);
    IF_FAILED_JUMP(ntStatus, Exit);

    ntStatus = pAdapterCommon->Init(DeviceObject);
    IF_FAILED_JUMP(ntStatus, Exit);

    g_BridgeConfig = ReadBridgeConfig(DeviceObject);
    g_EnabledCableCount = g_BridgeConfig.CableCount;
    // Wave interfaces are registered below with this packet constraint, so
    // set it from the configured minimum period before any filter install.
    g_CablePacketSizeConstraints.MinPacketPeriodInHns =
        AudioRouterMinPacketPeriodHns(g_BridgeConfig.MinPeriodFrames);

    //
    // register with PortCls for power-management services
    ntStatus = PcRegisterAdapterPowerManagement( PUNKNOWN(pAdapterCommon), DeviceObject);
    IF_FAILED_JUMP(ntStatus, Exit);

    //
    // Install wave+topology filters for render devices
    //
    ntStatus = InstallAllRenderFilters(DeviceObject, Irp, pAdapterCommon);
    IF_FAILED_JUMP(ntStatus, Exit);

    //
    // Install wave+topology filters for capture devices
    //
    ntStatus = InstallAllCaptureFilters(DeviceObject, Irp, pAdapterCommon);
    IF_FAILED_JUMP(ntStatus, Exit);

Exit:

    //
    // Stash the adapter common object in the device extension so
    // we can access it for cleanup on stop/removal.
    //
    if (pAdapterCommon)
    {
        ASSERT(pExtension != NULL);
        pExtension->m_pCommon = pAdapterCommon;
    }

    //
    // Release the adapter IUnknown interface.
    //
    SAFE_RELEASE(pUnknownCommon);

    return ntStatus;
} // StartDevice

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
PnpHandler
(
    _In_ DEVICE_OBJECT *_DeviceObject,
    _Inout_ IRP *_Irp
)
/*++

Routine Description:

  Handles PnP IRPs

Arguments:

  _DeviceObject - Functional Device object pointer.

  _Irp - The Irp being passed

Return Value:

  NT status code.

--*/
{
    NTSTATUS                ntStatus = STATUS_UNSUCCESSFUL;
    IO_STACK_LOCATION      *stack;
    PortClassDeviceContext *ext;

    // Documented https://msdn.microsoft.com/en-us/library/windows/hardware/ff544039(v=vs.85).aspx
    // This method will be called in IRQL PASSIVE_LEVEL
#pragma warning(suppress: 28118)
    PAGED_CODE();

    ASSERT(_DeviceObject);
    ASSERT(_Irp);

    //
    // Check for the REMOVE_DEVICE irp.  If we're being unloaded,
    // uninstantiate our devices and release the adapter common
    // object.
    //
    stack = IoGetCurrentIrpStackLocation(_Irp);

    switch (stack->MinorFunction)
    {
    case IRP_MN_REMOVE_DEVICE:
    case IRP_MN_SURPRISE_REMOVAL:
    case IRP_MN_STOP_DEVICE:
        ext = static_cast<PortClassDeviceContext*>(_DeviceObject->DeviceExtension);

        if (ext->m_pCommon != NULL)
        {
            ext->m_pCommon->Cleanup();

            ext->m_pCommon->Release();
            ext->m_pCommon = NULL;
        }
        break;

    default:
        break;
    }

    ntStatus = PcDispatchIrp(_DeviceObject, _Irp);

    return ntStatus;
}

#pragma code_seg()
