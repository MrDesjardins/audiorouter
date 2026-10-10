/* Constant-work WaveRT position arithmetic, shared with host regressions. */
#ifndef _AUDIOROUTERVIRTUAL_STREAMTIMING_H_
#define _AUDIOROUTERVIRTUAL_STREAMTIMING_H_

// Stream-owned metadata, accessed only under the position lock. SetWritePacket
// admits only the current packet before RUN, or current + 1 during RUN; hence
// two identities suffice regardless of the number of physical DMA slots.
// Never retain a borrowed DMA pointer or infer validity from a reused offset.
struct AudioRouterRenderCommits {
    ULONGLONG Packets[2];
    bool Valid[2];
    bool PacketMode;

    void Reset() {
        Packets[0] = Packets[1] = 0;
        Valid[0] = Valid[1] = false;
        PacketMode = false;
    }

    void Commit(ULONGLONG Packet) {
        const ULONG slot = static_cast<ULONG>(Packet & 1);
        Packets[slot] = Packet;
        Valid[slot] = true;
        PacketMode = true;
    }

    void InvalidateSlot(ULONG PacketNumber, ULONG Notifications) {
        if (Notifications == 0) { return; }
        for (ULONG slot = 0; slot < 2; ++slot) {
            if (Valid[slot] && static_cast<ULONG>(Packets[slot]) % Notifications ==
                    PacketNumber % Notifications) {
                Valid[slot] = false;
            }
        }
    }

    bool Contains(ULONGLONG LinearByte, ULONG PacketBytes) const {
        // Legacy write-position / polling clients do not commit packet IDs.
        if (!PacketMode) { return true; }
        if (PacketBytes == 0) { return false; }
        const ULONGLONG packet = LinearByte / PacketBytes;
        const ULONG slot = static_cast<ULONG>(packet & 1);
        return Valid[slot] && Packets[slot] == packet;
    }
};

// Compare the low 32 bits at the wire boundary, retaining the full identity
// for consumption across PacketNumber rollover. Caller bounds CurrentPacket
// to MAXLONGLONG and commits only when this signed modulo delta is zero.
__forceinline LONG AudioRouterRenderCommitDelta(
    ULONG PacketNumber, ULONGLONG CurrentPacket, bool Running)
{
    const ULONG expected = static_cast<ULONG>(CurrentPacket + (Running ? 1 : 0));
    return static_cast<LONG>(PacketNumber - expected);
}

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
