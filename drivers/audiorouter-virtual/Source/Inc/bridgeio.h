/*++

  AudioRouter virtual-driver broker ABI.

  This header defines the bounded request contract used by the prototype's
  secured control device. The device remains build-only until PnP/remove
  ownership, installation, and production signing are separately qualified.
--*/

#ifndef _AUDIOROUTERVIRTUAL_BRIDGEIO_H_
#define _AUDIOROUTERVIRTUAL_BRIDGEIO_H_

#if defined(AR_BRIDGE_UNIT_TEST)
#include "../../tests/km_shim.h"
#else
#include <ntddk.h>
#include <wdmsec.h>

#pragma comment(lib, "Wdmsec.lib")

// This sample's target-version headers omit these documented exports even
// though the WDK kernel library provides them.
extern "C" NTKERNELAPI NTSTATUS MmMapViewInSystemSpace(
    _In_ PVOID Section,
    _Outptr_result_bytebuffer_(*ViewSize) PVOID* MappedBase,
    _Inout_ PSIZE_T ViewSize
);
extern "C" NTKERNELAPI NTSTATUS MmUnmapViewInSystemSpace(
    _In_ PVOID MappedBase
);
// Present in ntifs.h; declare the two documented exports here rather than
// importing the entire filesystem-driver header into PortCls sample headers.
extern "C" NTKERNELAPI POBJECT_TYPE* MmSectionObjectType;
extern "C" NTKERNELAPI NTSTATUS IoGetRequestorSessionId(_In_ PIRP Irp, _Out_ PULONG SessionId);
#endif


#define AR_BRIDGE_PROTOCOL_MAJOR 1
#define AR_BRIDGE_PROTOCOL_MINOR 1
#define AR_BRIDGE_MAX_CABLES 8
#define AR_BRIDGE_LEASE_SLOTS (AR_BRIDGE_MAX_CABLES * 2)
#define AR_BRIDGE_MAX_BUS_ID_BYTES 128
#define AR_BRIDGE_MAX_CHANNELS 8
#define AR_BRIDGE_MAX_FRAMES 4096
#define AR_BRIDGE_MAX_LEASE_MS 60000
#define AR_BRIDGE_DIRECTION_RENDER_SOURCE 1
#define AR_BRIDGE_DIRECTION_CAPTURE_SINK 2
// Shared section layout (17 §5.2): seqlock state, block header, driver-owned
// stream counters, negotiated sample size, render-consumer acknowledgement,
// reserved bytes, then the payload at offset 128.
#define AR_BRIDGE_HEADER_BYTES 128
#define AR_BRIDGE_STATE_OFFSET 0
#define AR_BRIDGE_HEADER_OFFSET 8
#define AR_BRIDGE_COUNTERS_OFFSET 32
#define AR_BRIDGE_SAMPLE_BYTES_OFFSET 88
#define AR_BRIDGE_READER_SEQUENCE_OFFSET 96
#define AR_BRIDGE_PAYLOAD_OFFSET 128
#define AR_BRIDGE_SAMPLE_BYTES_FLOAT64 8
#define AR_BRIDGE_MAX_PAYLOAD_BYTES \
    (AR_BRIDGE_MAX_CHANNELS * AR_BRIDGE_MAX_FRAMES * sizeof(DOUBLE))

// Optional OPEN extension. This driver implements only float64 transport, so
// OPEN must carry the extension with FLOAT64; unknown bits are refused so a
// newer client can detect an older driver instead of being silently ignored.
#define AR_BRIDGE_OPEN_EXTENSION_BYTES 64
#define AR_BRIDGE_OPEN_FLAG_FLOAT64 0x00000001UL
#define AR_BRIDGE_OPEN_KNOWN_FLAGS AR_BRIDGE_OPEN_FLAG_FLOAT64

// QUERY capability bits. Report only what this build implements.
#define AR_BRIDGE_CAP_MULTICHANNEL 0x00000001UL
#define AR_BRIDGE_CAP_RATES_44_48_96 0x00000002UL
#define AR_BRIDGE_CAP_LOW_LATENCY_PERIODS 0x00000004UL
#define AR_BRIDGE_CAP_STREAM_COUNTERS 0x00000008UL
#define AR_BRIDGE_CAP_CONFIG_FROM_REGISTRY 0x00000010UL
#define AR_BRIDGE_CAP_SAMPLE_FLOAT64 0x00000020UL
#define AR_BRIDGE_IMPLEMENTED_CAPS \
    (AR_BRIDGE_CAP_MULTICHANNEL | AR_BRIDGE_CAP_RATES_44_48_96 | \
     AR_BRIDGE_CAP_STREAM_COUNTERS | AR_BRIDGE_CAP_SAMPLE_FLOAT64)
#define AR_BRIDGE_RATE_44100 0x00000001UL
#define AR_BRIDGE_RATE_48000 0x00000002UL
#define AR_BRIDGE_RATE_96000 0x00000004UL

// Stamped by build.ps1 -Version through MSBuild; 0.0.0.0 for unversioned
// developer builds so QUERY never reports an invented version.
#ifndef AR_DRIVER_VERSION_MAJOR
#define AR_DRIVER_VERSION_MAJOR 0
#endif
#ifndef AR_DRIVER_VERSION_MINOR
#define AR_DRIVER_VERSION_MINOR 0
#endif
#ifndef AR_DRIVER_VERSION_PATCH
#define AR_DRIVER_VERSION_PATCH 0
#endif
#ifndef AR_DRIVER_VERSION_BUILD
#define AR_DRIVER_VERSION_BUILD 0
#endif

NTSTATUS AudioRouterCopyLeaseBlockForDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _In_ ULONGLONG MinimumSequence,
    _Out_writes_(DestinationCapacitySamples) DOUBLE* Destination,
    _In_ SIZE_T DestinationCapacitySamples,
    _Out_ struct _AR_BRIDGE_BLOCK_HEADER* Header,
    _Out_ ULONG* NonFiniteSamples);

// Per-callback counter deltas from a WaveRT stream. Adding them is bounded,
// lock-free and touches only the pinned mapped view of an active lease.
typedef struct _AR_BRIDGE_STREAM_ACTIVITY {
    ULONGLONG UnderrunFrames;
    ULONGLONG SequenceGaps;
    ULONGLONG NonFiniteSamples;
    ULONGLONG FormatMismatches;
    ULONGLONG DevicePositionFrames;
    ULONGLONG QpcTime;
} AR_BRIDGE_STREAM_ACTIVITY, *PAR_BRIDGE_STREAM_ACTIVITY;

NTSTATUS AudioRouterRecordLeaseActivityForDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _In_ const AR_BRIDGE_STREAM_ACTIVITY* Activity);

NTSTATUS AudioRouterPublishLeaseBlockForDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _In_ USHORT Frames,
    _In_ USHORT Channels,
    _In_reads_(SampleCapacity) const DOUBLE* Samples,
    _In_ SIZE_T SampleCapacity);

NTSTATUS AudioRouterGetLeaseShapeForDirection(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _Out_ USHORT* Frames,
    _Out_ USHORT* Channels,
    _Out_ ULONG* SampleRateHz,
    _Out_ ULONGLONG* Generation);

#define IOCTL_AUDIOROUTER_BRIDGE_OPEN \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)
#define IOCTL_AUDIOROUTER_BRIDGE_CLOSE \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)
#define IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)
#define IOCTL_AUDIOROUTER_BRIDGE_QUERY \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x803, METHOD_BUFFERED, FILE_READ_DATA)
// The Rust client pins these exact values; METHOD_BUFFERED is part of the ABI.
C_ASSERT(IOCTL_AUDIOROUTER_BRIDGE_OPEN == 0x0022E000);
C_ASSERT(IOCTL_AUDIOROUTER_BRIDGE_CLOSE == 0x0022E004);
C_ASSERT(IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT == 0x0022E008);
C_ASSERT(IOCTL_AUDIOROUTER_BRIDGE_QUERY == 0x0022600C);

#define AUDIOROUTER_BRIDGE_DEVICE_NAME L"\\Device\\AudioRouterVirtualBridge"
#define AUDIOROUTER_BRIDGE_DOS_NAME L"\\DosDevices\\AudioRouterVirtualBridge"

// Interactive users may route audio; FILE_OBJECT and session ownership below
// remain mandatory. No world/network-user access is granted to this broker.
DECLARE_CONST_UNICODE_STRING(
    AUDIOROUTER_BRIDGE_DEVICE_SDDL,
    L"D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)"
);

typedef struct _AR_BRIDGE_OPEN_REQUEST {
    USHORT ProtocolMajor;
    USHORT ProtocolMinor;
    USHORT BusIdBytes;
    USHORT Channels;
    USHORT FramesPerQuantum;
    USHORT Direction;
    ULONG SampleRateHz;
    ULONG LeaseMs;
    ULONGLONG Generation;
    ULONGLONG SectionHandle;
    ULONG MappingBytes;
    ULONG Reserved2;
    WCHAR BusId[AR_BRIDGE_MAX_BUS_ID_BYTES / sizeof(WCHAR)];
} AR_BRIDGE_OPEN_REQUEST, *PAR_BRIDGE_OPEN_REQUEST;

typedef struct _AR_BRIDGE_OPEN_EXTENSION {
    ULONG ExtensionBytes;
    ULONG Flags;
    ULONG Reserved[14];
} AR_BRIDGE_OPEN_EXTENSION, *PAR_BRIDGE_OPEN_EXTENSION;

// OPEN/HEARTBEAT/CLOSE input is the fixed prefix, optionally followed by the
// extension. OPEN requires the extension (float64 negotiation).
typedef struct _AR_BRIDGE_OPEN_REQUEST_EX {
    AR_BRIDGE_OPEN_REQUEST Request;
    AR_BRIDGE_OPEN_EXTENSION Extension;
} AR_BRIDGE_OPEN_REQUEST_EX, *PAR_BRIDGE_OPEN_REQUEST_EX;

typedef struct _AR_BRIDGE_BLOCK_HEADER {
    ULONGLONG Generation;
    ULONGLONG Sequence;
    USHORT Frames;
    USHORT Channels;
    ULONG PayloadBytes;
} AR_BRIDGE_BLOCK_HEADER, *PAR_BRIDGE_BLOCK_HEADER;

// Written only by the driver; user mode reads them from the mapped view
// without an IOCTL. All values are monotonic within one lease.
typedef struct _AR_BRIDGE_STREAM_COUNTERS {
    ULONGLONG UnderrunFrames;
    ULONGLONG OverrunFrames;
    ULONGLONG SequenceGaps;
    ULONGLONG NonFiniteSamples;
    ULONGLONG FormatMismatches;
    ULONGLONG LastDevicePosition;
    ULONGLONG LastQpcTime;
} AR_BRIDGE_STREAM_COUNTERS, *PAR_BRIDGE_STREAM_COUNTERS;

typedef struct _AR_BRIDGE_SHARED_HEADER {
    LONG64 State;
    AR_BRIDGE_BLOCK_HEADER Block;
    AR_BRIDGE_STREAM_COUNTERS Counters;
    ULONG SampleBytes;          // driver-written at OPEN; readers require 8
    ULONG Reserved0;
    ULONGLONG ReaderSequence;   // render-source consumer acknowledgement (user mode)
    UCHAR Reserved[24];
} AR_BRIDGE_SHARED_HEADER, *PAR_BRIDGE_SHARED_HEADER;

typedef struct _AR_BRIDGE_DRIVER_INFO {
    USHORT ProtocolMajor;
    USHORT ProtocolMinor;
    USHORT DriverVersion[4];
    ULONG CableCount;
    ULONG MaxCables;
    ULONG MaxChannels;
    ULONG Capabilities;
    ULONG SupportedRates;
    ULONG MinPeriodFrames;      // 0 while LOW_LATENCY_PERIODS is not reported
    ULONG DefaultPeriodFrames;  // 0 while LOW_LATENCY_PERIODS is not reported
    ULONG Reserved[16];
} AR_BRIDGE_DRIVER_INFO, *PAR_BRIDGE_DRIVER_INFO;

// Bus IDs are protocol identities, not display names. Accept only the exact
// seven-character lowercase form and keep parsing bounded for IOCTL callers.
__forceinline NTSTATUS AudioRouterParseCableBusId(
    _In_reads_bytes_(BusIdBytes) const WCHAR* BusId,
    _In_ USHORT BusIdBytes,
    _Out_ USHORT* BusIndex)
{
    static const WCHAR prefix[] = L"cable-";
    if (BusId == NULL || BusIndex == NULL || BusIdBytes != 7 * sizeof(WCHAR)) {
        return STATUS_OBJECT_NAME_NOT_FOUND;
    }
    for (USHORT index = 0; index < 6; ++index) {
        if (BusId[index] != prefix[index]) {
            return STATUS_OBJECT_NAME_NOT_FOUND;
        }
    }
    WCHAR suffix = BusId[6];
    if (suffix < L'a' || suffix >= L'a' + AR_BRIDGE_MAX_CABLES) {
        return STATUS_OBJECT_NAME_NOT_FOUND;
    }
    *BusIndex = static_cast<USHORT>(suffix - L'a');
    return STATUS_SUCCESS;
}

__forceinline NTSTATUS AudioRouterGetLeaseSlotIndex(
    _In_ USHORT BusIndex,
    _In_ USHORT Direction,
    _Out_ ULONG* SlotIndex)
{
    if (SlotIndex == NULL || BusIndex >= AR_BRIDGE_MAX_CABLES) {
        return STATUS_INVALID_PARAMETER;
    }
    ULONG directionIndex;
    if (Direction == AR_BRIDGE_DIRECTION_RENDER_SOURCE) {
        directionIndex = 0;
    } else if (Direction == AR_BRIDGE_DIRECTION_CAPTURE_SINK) {
        directionIndex = 1;
    } else {
        return STATUS_INVALID_PARAMETER;
    }
    *SlotIndex = static_cast<ULONG>(BusIndex) * 2 + directionIndex;
    return STATUS_SUCCESS;
}

// Keep the fixed ABI fail-fast at compile time. The Rust encoder mirrors these
// offsets; changing either layout requires an explicit protocol revision.
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_OPEN_REQUEST, ProtocolMajor) == 0);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_OPEN_REQUEST, Generation) == 24);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_OPEN_REQUEST, SectionHandle) == 32);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_OPEN_REQUEST, MappingBytes) == 40);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_OPEN_REQUEST, BusId) == 48);
C_ASSERT(sizeof(AR_BRIDGE_OPEN_REQUEST) == 176);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_BLOCK_HEADER, Generation) == 0);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_BLOCK_HEADER, Sequence) == 8);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_BLOCK_HEADER, Frames) == 16);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_BLOCK_HEADER, Channels) == 18);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_BLOCK_HEADER, PayloadBytes) == 20);
C_ASSERT(sizeof(AR_BRIDGE_BLOCK_HEADER) == 24);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_OPEN_EXTENSION, Flags) == 4);
C_ASSERT(sizeof(AR_BRIDGE_OPEN_EXTENSION) == AR_BRIDGE_OPEN_EXTENSION_BYTES);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_OPEN_REQUEST_EX, Extension) == 176);
C_ASSERT(sizeof(AR_BRIDGE_OPEN_REQUEST_EX) == 240);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_SHARED_HEADER, State) == AR_BRIDGE_STATE_OFFSET);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_SHARED_HEADER, Block) == AR_BRIDGE_HEADER_OFFSET);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_SHARED_HEADER, Counters) == AR_BRIDGE_COUNTERS_OFFSET);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_SHARED_HEADER, Counters.OverrunFrames) == 40);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_SHARED_HEADER, Counters.FormatMismatches) == 64);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_SHARED_HEADER, Counters.LastQpcTime) == 80);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_SHARED_HEADER, SampleBytes) == AR_BRIDGE_SAMPLE_BYTES_OFFSET);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_SHARED_HEADER, ReaderSequence) == AR_BRIDGE_READER_SEQUENCE_OFFSET);
C_ASSERT(sizeof(AR_BRIDGE_SHARED_HEADER) == AR_BRIDGE_PAYLOAD_OFFSET);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_DRIVER_INFO, DriverVersion) == 4);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_DRIVER_INFO, CableCount) == 12);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_DRIVER_INFO, Capabilities) == 24);
C_ASSERT(FIELD_OFFSET(AR_BRIDGE_DRIVER_INFO, DefaultPeriodFrames) == 36);
C_ASSERT(sizeof(AR_BRIDGE_DRIVER_INFO) == 104);

// Classify OPEN/HEARTBEAT/CLOSE input before any field is read. The caller
// has already checked that the system buffer is present. Exact lengths only.
__forceinline NTSTATUS AudioRouterValidateBridgeRequestLength(
    _In_ ULONG InputBytes,
    _In_ ULONG OutputBytes,
    _In_ bool IsOpen)
{
    if (OutputBytes != 0) {
        return STATUS_INVALID_PARAMETER;
    }
    if (InputBytes == sizeof(AR_BRIDGE_OPEN_REQUEST_EX)) {
        return STATUS_SUCCESS;
    }
    if (InputBytes == sizeof(AR_BRIDGE_OPEN_REQUEST)) {
        // A prefix-only OPEN cannot negotiate float64; this driver has no
        // other transport. Maintenance requests may omit the extension.
        return IsOpen ? STATUS_NOT_SUPPORTED : STATUS_SUCCESS;
    }
    return STATUS_INVALID_PARAMETER;
}

__forceinline NTSTATUS AudioRouterValidateBridgeOpenExtension(
    _In_ const AR_BRIDGE_OPEN_EXTENSION* Extension)
{
    if (Extension == NULL || Extension->ExtensionBytes != AR_BRIDGE_OPEN_EXTENSION_BYTES) {
        return STATUS_INVALID_PARAMETER;
    }
    if ((Extension->Flags & ~AR_BRIDGE_OPEN_KNOWN_FLAGS) != 0 ||
        (Extension->Flags & AR_BRIDGE_OPEN_FLAG_FLOAT64) == 0) {
        return STATUS_NOT_SUPPORTED;
    }
    for (ULONG index = 0; index < ARRAYSIZE(Extension->Reserved); ++index) {
        if (Extension->Reserved[index] != 0) {
            return STATUS_NOT_SUPPORTED;
        }
    }
    return STATUS_SUCCESS;
}

__forceinline NTSTATUS AudioRouterValidateBridgeQueryLength(
    _In_ ULONG InputBytes,
    _In_ ULONG OutputBytes)
{
    return InputBytes == 0 && OutputBytes == sizeof(AR_BRIDGE_DRIVER_INFO)
        ? STATUS_SUCCESS : STATUS_INVALID_PARAMETER;
}

__forceinline void AudioRouterFillDriverInfo(
    _Out_ AR_BRIDGE_DRIVER_INFO* Info,
    _In_ ULONG EnabledCableCount)
{
    RtlZeroMemory(Info, sizeof(*Info));
    Info->ProtocolMajor = AR_BRIDGE_PROTOCOL_MAJOR;
    Info->ProtocolMinor = AR_BRIDGE_PROTOCOL_MINOR;
    Info->DriverVersion[0] = AR_DRIVER_VERSION_MAJOR;
    Info->DriverVersion[1] = AR_DRIVER_VERSION_MINOR;
    Info->DriverVersion[2] = AR_DRIVER_VERSION_PATCH;
    Info->DriverVersion[3] = AR_DRIVER_VERSION_BUILD;
    Info->CableCount = EnabledCableCount;
    Info->MaxCables = AR_BRIDGE_MAX_CABLES;
    Info->MaxChannels = AR_BRIDGE_MAX_CHANNELS;
    Info->Capabilities = AR_BRIDGE_IMPLEMENTED_CAPS;
    Info->SupportedRates = AR_BRIDGE_RATE_44100 | AR_BRIDGE_RATE_48000 | AR_BRIDGE_RATE_96000;
}

// Overrun rule for a render-source lease: the previously published block was
// lost when the consumer has not acknowledged it before the next publish.
__forceinline bool AudioRouterRenderBlockWasOverrun(
    _In_ ULONGLONG LastPublishedSequence,
    _In_ ULONGLONG ReaderSequence)
{
    return LastPublishedSequence != 0 && ReaderSequence < LastPublishedSequence;
}

// Number of blocks skipped between two consecutively consumed sequences.
__forceinline ULONGLONG AudioRouterSequenceGap(
    _In_ ULONGLONG PreviousSequence,
    _In_ ULONGLONG CurrentSequence)
{
    return PreviousSequence != 0 && CurrentSequence > PreviousSequence &&
        CurrentSequence - PreviousSequence > 1
        ? CurrentSequence - PreviousSequence - 1 : 0;
}

// Control-plane length checks are shared with user-mode regression tests.
// The request shape must already have passed the bounded OPEN validator.
__forceinline NTSTATUS AudioRouterValidateMappingBytes(const AR_BRIDGE_OPEN_REQUEST* Request)
{
    SIZE_T required = AR_BRIDGE_HEADER_BYTES + static_cast<SIZE_T>(Request->Channels) *
        Request->FramesPerQuantum * sizeof(DOUBLE);
    return Request->MappingBytes == required ? STATUS_SUCCESS : STATUS_BUFFER_TOO_SMALL;
}

__forceinline NTSTATUS AudioRouterOpenOwnershipStatus(bool Held, ULONG OwnerSession, ULONG RequestSession)
{
    if (!Held) { return STATUS_SUCCESS; }
    return OwnerSession == RequestSession ? STATUS_SHARING_VIOLATION : STATUS_ACCESS_DENIED;
}

// Pure state transition shared by the stream callback and host regressions.
// A new generation invalidates partial blocks even when the format is equal.
__forceinline bool AudioRouterStreamGenerationChanged(
    _In_ ULONGLONG PreviousGeneration,
    _In_ ULONGLONG CurrentGeneration)
{
    return PreviousGeneration != CurrentGeneration;
}

__forceinline NTSTATUS AudioRouterValidateNextGeneration(
    _In_ ULONGLONG PreviousGeneration,
    _In_ ULONGLONG RequestedGeneration)
{
    return RequestedGeneration != 0 && RequestedGeneration > PreviousGeneration
        ? STATUS_SUCCESS : STATUS_INVALID_PARAMETER;
}

// This validator is safe to call from a future callback-owned mapped-view
// reader: it performs only bounded arithmetic and scalar reads. It does not
// acquire a lock, allocate, access an endpoint, or issue an IOCTL.
__forceinline
NTSTATUS
AudioRouterValidateBridgeBlock(
    _In_ const AR_BRIDGE_BLOCK_HEADER* Header,
    _In_ ULONGLONG ExpectedGeneration,
    _In_ ULONGLONG MinimumSequence,
    _In_ SIZE_T ViewBytes
)
{
    if (Header == NULL ||
        ExpectedGeneration == 0 ||
        Header->Generation != ExpectedGeneration ||
        Header->Sequence <= MinimumSequence ||
        Header->Frames == 0 ||
        Header->Frames > AR_BRIDGE_MAX_FRAMES ||
        Header->Channels == 0 ||
        Header->Channels > AR_BRIDGE_MAX_CHANNELS) {
        return STATUS_INVALID_PARAMETER;
    }
    SIZE_T payloadBytes = static_cast<SIZE_T>(Header->Frames) *
        static_cast<SIZE_T>(Header->Channels) * sizeof(DOUBLE);
    if (payloadBytes > AR_BRIDGE_MAX_PAYLOAD_BYTES ||
        Header->PayloadBytes != payloadBytes ||
        ViewBytes < AR_BRIDGE_PAYLOAD_OFFSET + payloadBytes) {
        return STATUS_BUFFER_TOO_SMALL;
    }
    return STATUS_SUCCESS;
}

// Copy one already-mapped render-source block into a caller-owned PCM destination. This is
// deliberately a bounded helper for a future PortCls-owned callback: it does
// not acquire the lease, wait for a producer, allocate, log, or issue I/O.
// Callers must keep the mapped view alive for the duration of this operation.
__forceinline
NTSTATUS
AudioRouterCopyBridgeBlock(
    _In_ const UCHAR* View,
    _In_ SIZE_T ViewBytes,
    _In_ ULONGLONG ExpectedGeneration,
    _In_ ULONGLONG MinimumSequence,
    _Out_writes_(DestinationCapacitySamples) DOUBLE* Destination,
    _In_ SIZE_T DestinationCapacitySamples,
    _Out_ AR_BRIDGE_BLOCK_HEADER* Header,
    _Out_ ULONG* NonFiniteSamples
)
{
    if (View == NULL || Header == NULL || Destination == NULL ||
        NonFiniteSamples == NULL || ViewBytes < AR_BRIDGE_PAYLOAD_OFFSET) {
        return STATUS_INVALID_PARAMETER;
    }
    *NonFiniteSamples = 0;
    const volatile AR_BRIDGE_BLOCK_HEADER* sourceHeader =
        reinterpret_cast<const volatile AR_BRIDGE_BLOCK_HEADER*>(
            View + AR_BRIDGE_HEADER_OFFSET);
    // The writer controls this memory. Snapshot each scalar exactly once;
    // neither validation nor copy sizing may fetch the shared header again.
    // The caller pins the mapping and checks the seqlock around this call.
    AR_BRIDGE_BLOCK_HEADER snapshot = {
        sourceHeader->Generation, sourceHeader->Sequence, sourceHeader->Frames,
        sourceHeader->Channels, sourceHeader->PayloadBytes
    };
#if defined(AR_BRIDGE_UNIT_TEST)
    AudioRouterTestAfterHeaderSnapshot(View);
#endif
    NTSTATUS status = AudioRouterValidateBridgeBlock(
        &snapshot, ExpectedGeneration, MinimumSequence, ViewBytes);
    if (!NT_SUCCESS(status)) {
        return status;
    }
    SIZE_T sampleCount = static_cast<SIZE_T>(snapshot.Frames) *
        static_cast<SIZE_T>(snapshot.Channels);
    if (DestinationCapacitySamples < sampleCount) {
        return STATUS_BUFFER_TOO_SMALL;
    }
    // Single payload fetch per sample: a separate validation pass would allow
    // a hostile writer to swap in NaN/Inf before the later copy. A block with
    // any non-finite sample is rejected whole and wiped so partially refreshed
    // private audio is not used; the bounded loop still finishes so the
    // NonFiniteSamples counter reports how many samples were refused.
    ULONG nonFinite = 0;
    for (SIZE_T index = 0; index < sampleCount; ++index) {
        DOUBLE sample = *reinterpret_cast<const volatile DOUBLE*>(
            View + AR_BRIDGE_PAYLOAD_OFFSET + index * sizeof(DOUBLE));
        // NaN is unequal to itself; the finite bounds reject both infinities
        // without depending on CRT floating-point helpers in kernel mode.
        if (sample != sample || sample > 1.7976931348623157e+308 ||
            sample < -1.7976931348623157e+308) {
            ++nonFinite;
            sample = 0.0;
        }
        Destination[index] = sample;
    }
    if (nonFinite != 0) {
        RtlZeroMemory(Destination, sampleCount * sizeof(DOUBLE));
        *NonFiniteSamples = nonFinite;
        return STATUS_DATA_ERROR;
    }
    *Header = snapshot;
    return STATUS_SUCCESS;
}

// Validation is deliberately pure and bounded so the eventual dispatch path
// can reject malformed input before touching a device, mapping, or stream.
__forceinline
NTSTATUS
AudioRouterValidateBridgeOpenRequest(
    _In_ const AR_BRIDGE_OPEN_REQUEST* Request
)
{
    if (Request == NULL) {
        return STATUS_INVALID_PARAMETER;
    }
    // The float64 payload changes the mapping interpretation, so a 1.0
    // client cannot safely use this layout despite sharing the major number.
    if (Request->ProtocolMajor != AR_BRIDGE_PROTOCOL_MAJOR ||
        Request->ProtocolMinor != AR_BRIDGE_PROTOCOL_MINOR) {
        return STATUS_REVISION_MISMATCH;
    }
    if (Request->BusIdBytes == 0 ||
        Request->BusIdBytes > AR_BRIDGE_MAX_BUS_ID_BYTES ||
        (Request->BusIdBytes % sizeof(WCHAR)) != 0 ||
        Request->Channels == 0 ||
        Request->Channels > AR_BRIDGE_MAX_CHANNELS ||
        (Request->Direction != AR_BRIDGE_DIRECTION_RENDER_SOURCE &&
         Request->Direction != AR_BRIDGE_DIRECTION_CAPTURE_SINK) ||
        Request->FramesPerQuantum == 0 ||
        Request->FramesPerQuantum > AR_BRIDGE_MAX_FRAMES ||
        Request->SampleRateHz < 8000 ||
        Request->SampleRateHz > 192000 ||
        Request->LeaseMs == 0 ||
        Request->LeaseMs > AR_BRIDGE_MAX_LEASE_MS ||
        Request->Generation == 0 ||
        Request->Reserved2 != 0 ||
        ((Request->SectionHandle == 0) != (Request->MappingBytes == 0)) ||
        (Request->SectionHandle != 0 && Request->MappingBytes < AR_BRIDGE_HEADER_BYTES)) {
        return STATUS_INVALID_PARAMETER;
    }
    USHORT busIdCharacters = Request->BusIdBytes / sizeof(WCHAR);
    for (USHORT index = 0; index < busIdCharacters; ++index) {
        if (Request->BusId[index] == L'\0') {
            return STATUS_INVALID_PARAMETER;
        }
    }
    for (USHORT index = busIdCharacters;
         index < ARRAYSIZE(Request->BusId); ++index) {
        if (Request->BusId[index] != L'\0') {
            return STATUS_INVALID_PARAMETER;
        }
    }
    return STATUS_SUCCESS;
}

#endif
