#ifndef AUDIOROUTER_RENDER_QUEUE_H
#define AUDIOROUTER_RENDER_QUEUE_H

// Fixed-shape wire blocks require silent padding at EoS. Bounds are checked
// before multiplication; the caller owns a full maximum-sized scratch array.
inline void AudioRouterPadRenderTail(DOUBLE* samples, ULONG validFrames, ULONG frames, ULONG channels) {
    if (samples == NULL || validFrames > frames || frames > AR_BRIDGE_MAX_FRAMES ||
        channels == 0 || channels > AR_BRIDGE_MAX_CHANNELS) { return; }
    RtlZeroMemory(samples + validFrames * channels,
                  (frames - validFrames) * channels * sizeof(DOUBLE));
}

// Private metadata, serialized with DMA by the stream position lock. Storage
// is the stream's existing nonpaged prefetch array (unused on render streams).
// No user-controlled pointer, allocation, wait, or unbounded retry is involved.
struct AudioRouterRenderQueue {
    ULONG Head;
    ULONG Count;
    ULONG Capacity;
    ULONG SamplesPerBlock;
    ULONGLONG Generation;

    void Clear() { Head = Count = 0; }
    void Reset(ULONG frames, ULONG channels, ULONGLONG generation) {
        Clear();
        Generation = generation;
        SamplesPerBlock = 0;
        Capacity = 0;
        if (generation == 0 || frames == 0 || frames > AR_BRIDGE_MAX_FRAMES ||
            channels == 0 || channels > AR_BRIDGE_MAX_CHANNELS) { return; }
        SamplesPerBlock = frames * channels;
        Capacity = (AR_BRIDGE_MAX_CHANNELS * AR_BRIDGE_MAX_FRAMES) / SamplesPerBlock;
        if (Capacity > 4) { Capacity = 4; }
    }
    // Append a complete block, replacing the oldest *private* block when full.
    // Return true only for an actual loss. The shared unread slot stays intact.
    bool Push(DOUBLE* storage, const DOUBLE* samples) {
        if (Capacity == 0) { return true; }
        const bool dropped = Count == Capacity;
        if (dropped) { Head = (Head + 1) % Capacity; --Count; }
        const ULONG tail = (Head + Count) % Capacity;
        RtlCopyMemory(storage + tail * SamplesPerBlock, samples,
                      SamplesPerBlock * sizeof(DOUBLE));
        ++Count;
        return dropped;
    }
    template<class Publish>
    void Drain(const DOUBLE* storage, Publish publish) {
        // At most four publications, even if an active reader acknowledges
        // concurrently. A busy/missing lease retains the pending block.
        for (ULONG attempt = 0; attempt < Capacity && Count != 0; ++attempt) {
            if (!publish(storage + Head * SamplesPerBlock, SamplesPerBlock, Generation)) {
                break;
            }
            Head = (Head + 1) % Capacity;
            --Count;
        }
    }
};

#endif
