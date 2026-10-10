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

// Split whole seconds from the remainder before scaling to QPC ticks. A
// direct HNS * Frequency overflows at ~51 hours for a 10 MHz clock even
// though its correctly divided result still fits. Caller supplies Ticks.
__forceinline bool AudioRouterHnsToQpc(
    ULONGLONG Hns, ULONGLONG Frequency, ULONGLONG* Ticks)
{
    const ULONGLONG maximum = ~static_cast<ULONGLONG>(0);
    const ULONGLONG seconds = Hns / 10000000;
    const ULONGLONG remainder = Hns % 10000000;
    if (Frequency == 0 || seconds > maximum / Frequency ||
        (remainder != 0 && Frequency > maximum / remainder)) {
        return false;
    }
    const ULONGLONG wholeTicks = seconds * Frequency;
    const ULONGLONG fractionalTicks = remainder * Frequency / 10000000;
    if (fractionalTicks > maximum - wholeTicks) { return false; }
    *Ticks = wholeTicks + fractionalTicks;
    return true;
}

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
