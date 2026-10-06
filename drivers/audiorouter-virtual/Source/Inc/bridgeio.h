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
#define AR_BRIDGE_PROTOCOL_MINOR 0
#define AR_BRIDGE_MAX_BUS_ID_BYTES 128
#define AR_BRIDGE_MAX_CHANNELS 2
#define AR_BRIDGE_MAX_FRAMES 4096
#define AR_BRIDGE_MAX_LEASE_MS 60000
#define AR_BRIDGE_DIRECTION_RENDER_SOURCE 1
#define AR_BRIDGE_DIRECTION_CAPTURE_SINK 2
#define AR_BRIDGE_HEADER_BYTES 32
#define AR_BRIDGE_STATE_OFFSET 0
#define AR_BRIDGE_HEADER_OFFSET 8
#define AR_BRIDGE_PAYLOAD_OFFSET 32
#define AR_BRIDGE_MAX_PAYLOAD_BYTES \
    (AR_BRIDGE_MAX_CHANNELS * AR_BRIDGE_MAX_FRAMES * sizeof(float))

NTSTATUS AudioRouterCopyLeaseBlockForDirection(
    _In_ USHORT Direction,
    _In_ ULONGLONG MinimumSequence,
    _Out_writes_(DestinationCapacitySamples) FLOAT* Destination,
    _In_ SIZE_T DestinationCapacitySamples,
    _Out_ struct _AR_BRIDGE_BLOCK_HEADER* Header);

NTSTATUS AudioRouterPublishLeaseBlockForDirection(
    _In_ USHORT Direction,
    _In_ USHORT Frames,
    _In_ USHORT Channels,
    _In_reads_(SampleCapacity) const FLOAT* Samples,
    _In_ SIZE_T SampleCapacity);

NTSTATUS AudioRouterGetLeaseShapeForDirection(
    _In_ USHORT Direction,
    _Out_ USHORT* Frames,
    _Out_ USHORT* Channels,
    _Out_ ULONGLONG* Generation);

#define IOCTL_AUDIOROUTER_BRIDGE_OPEN \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)
#define IOCTL_AUDIOROUTER_BRIDGE_CLOSE \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)
#define IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)

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

typedef struct _AR_BRIDGE_BLOCK_HEADER {
    ULONGLONG Generation;
    ULONGLONG Sequence;
    USHORT Frames;
    USHORT Channels;
    ULONG PayloadBytes;
} AR_BRIDGE_BLOCK_HEADER, *PAR_BRIDGE_BLOCK_HEADER;

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

// Control-plane length checks are shared with user-mode regression tests.
// The request shape must already have passed the bounded OPEN validator.
__forceinline NTSTATUS AudioRouterValidateMappingBytes(const AR_BRIDGE_OPEN_REQUEST* Request)
{
    SIZE_T required = AR_BRIDGE_HEADER_BYTES + static_cast<SIZE_T>(Request->Channels) *
        Request->FramesPerQuantum * sizeof(FLOAT);
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
        static_cast<SIZE_T>(Header->Channels) * sizeof(float);
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
    _Out_writes_(DestinationCapacitySamples) FLOAT* Destination,
    _In_ SIZE_T DestinationCapacitySamples,
    _Out_ AR_BRIDGE_BLOCK_HEADER* Header
)
{
    if (View == NULL || Header == NULL || Destination == NULL ||
        ViewBytes < AR_BRIDGE_PAYLOAD_OFFSET) {
        return STATUS_INVALID_PARAMETER;
    }
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
    // a hostile writer to swap in NaN/Inf before the later copy. On rejection
    // wipe this bounded quantum so partially refreshed private audio is not used.
    for (SIZE_T index = 0; index < sampleCount; ++index) {
        FLOAT sample = *reinterpret_cast<const volatile FLOAT*>(
            View + AR_BRIDGE_PAYLOAD_OFFSET + index * sizeof(FLOAT));
        // NaN is unequal to itself; the finite bounds reject both infinities
        // without depending on CRT floating-point helpers in kernel mode.
        if (sample != sample || sample > 3.402823466e+38F ||
            sample < -3.402823466e+38F) {
            RtlZeroMemory(Destination, sampleCount * sizeof(FLOAT));
            return STATUS_DATA_ERROR;
        }
        Destination[index] = sample;
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
    if (Request->ProtocolMajor != AR_BRIDGE_PROTOCOL_MAJOR || Request->ProtocolMinor > AR_BRIDGE_PROTOCOL_MINOR) {
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
