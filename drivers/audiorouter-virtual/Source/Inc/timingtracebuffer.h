#pragma once
#include "timingtrace.h"

// One closed bit and a writer count form a single atomic admission word.
// STOP sets the bit before waiting, so a writer can never enter behind the
// drain. Each admitted writer owns one append-only slot. No consumer reads
// until closed and drained; no slot overwrite, user pointer or allocation.
struct AudioRouterTimingBuffer {
    volatile LONG64 Gate;
    volatile LONG64 Count, Dropped, Exhausted;
    // Lifetime-cumulative, never reset; a failed admission owns no session.
    // STOP does not freeze this counter. Any nonzero value invalidates the
    // diagnostic conservatively, including failures from a previous session.
    volatile LONG64 AdmissionDropped;
    ULONGLONG StartQpc;
    AR_TIMING_RECORD Records[AR_TIMING_CAPACITY];

    void Close() { InterlockedOr64(&Gate, 0x80000000LL); }
    bool Closed() { return (InterlockedCompareExchange64(&Gate, 0, 0) & 0x80000000LL) != 0; }
    bool Drained() { return (InterlockedCompareExchange64(&Gate, 0, 0) & 0x7fffffffLL) == 0; }
    // Caller serializes control, requires closed+drained and no readers.
    void Reset() {
        RtlZeroMemory(Records, sizeof(Records));
        Count = Dropped = Exhausted = 0;
    }
    bool Open(ULONGLONG qpc) {
        StartQpc = qpc;
        LONG64 previous = InterlockedCompareExchange64(&Gate, 0, 0);
        if ((previous & ~0xffffffffLL) == 0x7fffffff00000000LL) return false;
        // Epoch prevents a paused CAS from admission across STOP/START (ABA).
        InterlockedExchange64(&Gate, (previous & ~0xffffffffLL) + 0x100000000LL);
        return true;
    }
    void Append(const AR_TIMING_RECORD& record) {
        bool admitted = false;
        for (unsigned attempt = 0; attempt != 16; ++attempt) {
            LONG64 state = InterlockedCompareExchange64(&Gate, 0, 0);
            if (state & 0x80000000LL) return;
            if ((state & 0x7fffffffLL) == 0x7fffffffLL) break;
            if (InterlockedCompareExchange64(&Gate, state + 1, state) == state) {
                admitted = true; break;
            }
        }
        if (!admitted) { InterlockedIncrement64(&AdmissionDropped); return; }
        if (record.Qpc < StartQpc) { InterlockedDecrement64(&Gate); return; }
        LONG64 index = 0;
        bool reserved = false;
        for (unsigned attempt = 0; attempt != 16; ++attempt) {
            index = InterlockedCompareExchange64(&Count, 0, 0);
            if (index >= AR_TIMING_CAPACITY) {
                InterlockedIncrement64(&Exhausted); break;
            }
            if (InterlockedCompareExchange64(&Count, index + 1, index) == index) {
                reserved = true; break;
            }
        }
        if (reserved) {
            Records[index] = record;
            Records[index].Sequence = static_cast<ULONGLONG>(index) + 1;
        } else if (index < AR_TIMING_CAPACITY) {
            InterlockedIncrement64(&Dropped);
        }
        InterlockedDecrement64(&Gate);
    }
};
