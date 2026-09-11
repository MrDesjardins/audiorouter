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

#define AR_BRIDGE_PROTOCOL_MAJOR 1
#define AR_BRIDGE_PROTOCOL_MINOR 0
#define AR_BRIDGE_MAX_BUS_ID_BYTES 128
#define AR_BRIDGE_MAX_CHANNELS 2
#define AR_BRIDGE_MAX_FRAMES 4096
#define AR_BRIDGE_MAX_LEASE_MS 60000

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
    USHORT Reserved;
    ULONG SampleRateHz;
    ULONG LeaseMs;
    ULONGLONG Generation;
    WCHAR BusId[AR_BRIDGE_MAX_BUS_ID_BYTES / sizeof(WCHAR)];
} AR_BRIDGE_OPEN_REQUEST, *PAR_BRIDGE_OPEN_REQUEST;

typedef struct _AR_BRIDGE_BLOCK_HEADER {
    ULONGLONG Generation;
    ULONGLONG Sequence;
    USHORT Frames;
    USHORT Channels;
    ULONG PayloadBytes;
} AR_BRIDGE_BLOCK_HEADER, *PAR_BRIDGE_BLOCK_HEADER;

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
        Request->FramesPerQuantum == 0 ||
        Request->FramesPerQuantum > AR_BRIDGE_MAX_FRAMES ||
        Request->SampleRateHz < 8000 ||
        Request->SampleRateHz > 192000 ||
        Request->LeaseMs == 0 ||
        Request->LeaseMs > AR_BRIDGE_MAX_LEASE_MS ||
        Request->Generation == 0) {
        return STATUS_INVALID_PARAMETER;
    }
    return STATUS_SUCCESS;
}

#endif
