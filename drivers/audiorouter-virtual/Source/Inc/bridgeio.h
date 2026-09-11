/*++

  AudioRouter virtual-driver broker ABI.

  This header intentionally defines the bounded request contract only. The
  current prototype does not create a control device or expose these IOCTLs;
  registration must arrive with a secured device ACL and PnP/remove ownership.
--*/

#ifndef _AUDIOROUTERVIRTUAL_BRIDGEIO_H_
#define _AUDIOROUTERVIRTUAL_BRIDGEIO_H_

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

#define IOCTL_AUDIOROUTER_BRIDGE_OPEN \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)
#define IOCTL_AUDIOROUTER_BRIDGE_CLOSE \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)
#define IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT \
    CTL_CODE(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)

#define AUDIOROUTER_BRIDGE_DEVICE_NAME L"\\Device\\AudioRouterVirtualBridge"
#define AUDIOROUTER_BRIDGE_DOS_NAME L"\\DosDevices\\AudioRouterVirtualBridge"

// Only LocalSystem and built-in Administrators may open the broker endpoint.
// Do not replace this with a world-readable WDK convenience SDDL.
DECLARE_CONST_UNICODE_STRING(
    AUDIOROUTER_BRIDGE_DEVICE_SDDL,
    L"D:P(A;;GA;;;SY)(A;;GA;;;BA)"
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

// This validator is safe to call from a future callback-owned mapped-view
// reader: it performs only bounded arithmetic and scalar reads. It does not
// acquire a lock, allocate, access an endpoint, or issue an IOCTL.
__forceinline
NTSTATUS
AudioRouterValidateBridgeBlock(
    _In_ const AR_BRIDGE_BLOCK_HEADER* Header,
    _In_ ULONGLONG ExpectedGeneration,
    _In_ SIZE_T ViewBytes
)
{
    if (Header == NULL ||
        ExpectedGeneration == 0 ||
        Header->Generation != ExpectedGeneration ||
        Header->Sequence == 0 ||
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

// Copy one already-mapped block into a caller-owned PCM destination. This is
// deliberately a bounded helper for a future PortCls-owned callback: it does
// not acquire the lease, wait for a producer, allocate, log, or issue I/O.
// Callers must keep the mapped view alive for the duration of this operation.
__forceinline
NTSTATUS
AudioRouterCopyBridgeBlock(
    _In_ const UCHAR* View,
    _In_ SIZE_T ViewBytes,
    _In_ ULONGLONG ExpectedGeneration,
    _Out_writes_(DestinationCapacitySamples) FLOAT* Destination,
    _In_ SIZE_T DestinationCapacitySamples,
    _Out_ AR_BRIDGE_BLOCK_HEADER* Header
)
{
    if (View == NULL || Header == NULL || Destination == NULL ||
        ViewBytes < AR_BRIDGE_PAYLOAD_OFFSET) {
        return STATUS_INVALID_PARAMETER;
    }
    const AR_BRIDGE_BLOCK_HEADER* sourceHeader =
        reinterpret_cast<const AR_BRIDGE_BLOCK_HEADER*>(
            View + AR_BRIDGE_HEADER_OFFSET);
    NTSTATUS status = AudioRouterValidateBridgeBlock(
        sourceHeader, ExpectedGeneration, ViewBytes);
    if (!NT_SUCCESS(status)) {
        return status;
    }
    SIZE_T sampleCount = static_cast<SIZE_T>(sourceHeader->Frames) *
        static_cast<SIZE_T>(sourceHeader->Channels);
    if (DestinationCapacitySamples < sampleCount) {
        return STATUS_BUFFER_TOO_SMALL;
    }
    for (SIZE_T index = 0; index < sampleCount; ++index) {
        FLOAT sample = 0.0F;
        RtlCopyMemory(
            &sample,
            View + AR_BRIDGE_PAYLOAD_OFFSET + index * sizeof(FLOAT),
            sizeof(FLOAT));
        // NaN is unequal to itself; the finite bounds reject both infinities
        // without depending on CRT floating-point helpers in kernel mode.
        if (sample != sample || sample > 3.402823466e+38F ||
            sample < -3.402823466e+38F) {
            return STATUS_DATA_ERROR;
        }
        Destination[index] = sample;
    }
    *Header = *sourceHeader;
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
    if (Request == NULL ||
        Request->ProtocolMajor != AR_BRIDGE_PROTOCOL_MAJOR ||
        Request->BusIdBytes == 0 ||
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
        ((Request->SectionHandle == 0) != (Request->MappingBytes == 0)) ||
        (Request->SectionHandle != 0 && Request->MappingBytes < AR_BRIDGE_HEADER_BYTES)) {
        return STATUS_INVALID_PARAMETER;
    }
    return STATUS_SUCCESS;
}

#endif
