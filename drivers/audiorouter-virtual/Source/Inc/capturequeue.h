#ifndef AUDIOROUTER_CAPTURE_QUEUE_H
#define AUDIOROUTER_CAPTURE_QUEUE_H

// Private WaveRT state, serialized by the owning stream's position lock.
// Samples live in two fixed nonpaged stream arrays. Reset changes validity
// only; every committed sample is overwritten by a validated bridge copy.
struct AudioRouterCaptureQueue {
    ULONG Frames[2];
    ULONG Offset;
    ULONG Head;
    ULONG Count;
    ULONGLONG Sequence;

    void Clear() {
        Frames[0] = Frames[1] = 0;
        Offset = Head = Count = 0;
    }
    void Reset() { Clear(); Sequence = 0; }
    ULONG Tail() const { return (Head + Count) % 2; }
    ULONG Available() const { return Count == 0 ? 0 : Frames[Head] - Offset; }
    template<class Fetch>
    void Prefetch(Fetch fetch) {
        // Fetch commits only a validated copy. Busy publication leaves all
        // valid lengths untouched; the caller can still drain queued audio.
        for (ULONG attempt = 0; attempt < 2 && Count < 2; ++attempt) {
            if (!fetch(Tail(), Sequence)) { break; }
        }
    }
    bool Commit(ULONG frames, ULONGLONG sequence) {
        if (Count == 2 || frames == 0 || frames > AR_BRIDGE_MAX_FRAMES ||
            sequence <= Sequence) {
            return false;
        }
        Frames[Tail()] = frames;
        ++Count;
        Sequence = sequence;
        return true;
    }
    bool Consume(ULONG frames) {
        if (frames == 0 || frames > Available()) { return false; }
        Offset += frames;
        if (Offset == Frames[Head]) {
            Frames[Head] = 0;
            Head = (Head + 1) % 2;
            Offset = 0;
            --Count;
        }
        return true;
    }
};

#endif
