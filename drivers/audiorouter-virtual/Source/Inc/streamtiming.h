/* Constant-work WaveRT position arithmetic, shared with host regressions. */
#ifndef _AUDIOROUTERVIRTUAL_STREAMTIMING_H_
#define _AUDIOROUTERVIRTUAL_STREAMTIMING_H_

__forceinline ULONGLONG AudioRouterCompletedPackets(
    ULONGLONG LinearBytes, ULONG BufferBytes, ULONG Notifications)
{
    const ULONG packetBytes = Notifications == 0 ? 0 : BufferBytes / Notifications;
    return packetBytes == 0 ? 0 : LinearBytes / packetBytes;
}

// Caller validates CompletedPackets > 0 and the multiplication range.
__forceinline ULONGLONG AudioRouterCompletedPacketStart(
    ULONGLONG CompletedPackets, ULONG PacketBytes)
{
    return (CompletedPackets - 1) * PacketBytes;
}

struct AudioRouterDmaWindow {
    ULONG Offset;
    ULONG Bytes;
    ULONG SkippedBytes;
};

// Only the latest DMA lap still exists after a delayed callback. Keep full
// logical clock progress separately; process at most two physical segments.
__forceinline AudioRouterDmaWindow AudioRouterSurvivingDmaWindow(
    ULONGLONG LinearBytes, ULONG Displacement, ULONG BufferBytes)
{
    AudioRouterDmaWindow result = {};
    if (BufferBytes == 0) { return result; }
    result.Bytes = Displacement > BufferBytes ? BufferBytes : Displacement;
    result.SkippedBytes = Displacement - result.Bytes;
    const ULONGLONG offset = LinearBytes % BufferBytes;
    result.Offset = static_cast<ULONG>(
        (offset + result.SkippedBytes % BufferBytes) % BufferBytes);
    return result;
}

#endif
