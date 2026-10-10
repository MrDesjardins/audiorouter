/* Constant-work WaveRT position arithmetic, shared with host regressions. */
#ifndef _AUDIOROUTERVIRTUAL_STREAMTIMING_H_
#define _AUDIOROUTERVIRTUAL_STREAMTIMING_H_

// Physical-slot provenance for packet-mode render, accessed only under the
// position lock. The OS writes a packet into its physical slot
// (PacketNumber % notifications) *before* it calls SetWritePacket. A slot's
// bytes therefore belong to the last packet the OS named for that slot, and
// only to that packet: never to the packet one lap earlier or later.
// Provenance is independent of the DDI return code. A late write of the
// packet now transferring holds that packet's data and may be used ("the
// driver may optionally use some of the data from the packet"); an
// overwritten slot never replays its previous lap. Notification counts are
// limited to 1 or 2 by AllocateBufferWithNotification, so two entries suffice.
// Never retain a borrowed DMA pointer or infer validity from a reused offset.
struct AudioRouterRenderCommits {
    ULONGLONG Packets[2];   // indexed by physical slot
    bool Valid[2];
    bool PacketMode;

    void Reset() {
        Packets[0] = Packets[1] = 0;
        Valid[0] = Valid[1] = false;
        PacketMode = false;
    }

    // Record that the OS has written PacketNumber into its physical slot.
    // CurrentPacket is the logical packet before this call's progress update;
    // the 32-bit wire number resolves to the nearest 64-bit identity. Usable
    // is false when the payload must not play (unsupported EOS packet), in
    // which case the slot's previous contents are still known overwritten.
    void RecordWrite(ULONG PacketNumber, ULONGLONG CurrentPacket,
                     ULONG Notifications, bool Usable) {
        if (Notifications != 1 && Notifications != 2) { return; }
        PacketMode = true;
        const ULONG slot = PacketNumber % Notifications;
        const LONG delta = static_cast<LONG>(
            PacketNumber - static_cast<ULONG>(CurrentPacket));
        // Callers bound CurrentPacket to MAXLONGLONG; reject identities that
        // would precede the stream or pass that bound.
        const ULONGLONG maximum = ~static_cast<ULONGLONG>(0) >> 1;
        const bool representable = CurrentPacket <= maximum &&
            (delta >= 0 ? static_cast<ULONGLONG>(delta) <= maximum - CurrentPacket
                        : static_cast<ULONGLONG>(-static_cast<LONG64>(delta)) <= CurrentPacket);
        Valid[slot] = Usable && representable;
        Packets[slot] = Valid[slot]
            ? static_cast<ULONGLONG>(static_cast<LONG64>(CurrentPacket) + delta) : 0;
    }

    bool Contains(ULONGLONG LinearByte, ULONG PacketBytes, ULONG Notifications) const {
        // Legacy write-position / polling clients do not name packets.
        if (!PacketMode) { return true; }
        if (PacketBytes == 0 || (Notifications != 1 && Notifications != 2)) { return false; }
        const ULONGLONG packet = LinearByte / PacketBytes;
        const ULONG slot = static_cast<ULONG>(packet % Notifications);
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
