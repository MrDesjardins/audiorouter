#pragma once

// Private diagnostic ABI. No sample data, addresses or user pointers.
// The IOCTL and storage exist only in an explicitly opted-in build.
#define AR_TIMING_IOCTL CTL_CODE(FILE_DEVICE_UNKNOWN, 0x804, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)
#define AR_TIMING_CAPACITY 131072UL
#define AR_TIMING_CHUNK 128UL
#define AR_TIMING_START 1UL
#define AR_TIMING_STOP 2UL
#define AR_TIMING_READ 3UL
#define AR_TIMING_TIMER 1
#define AR_TIMING_ADVANCE 2
#define AR_TIMING_NOTIFY 3
#define AR_TIMING_FETCH_ENTRY 4
#define AR_TIMING_FETCH_RETURN 5
#define AR_TIMING_FORMAT 6
#define AR_TIMING_RESIGNAL 7

struct AR_TIMING_REQUEST { ULONG Action; ULONG Offset; ULONG Reserved[2]; };
struct AR_TIMING_RECORD {
    ULONGLONG Sequence, Qpc, StreamId, PacketCount, LinearBytes;
    ULONG PacketNumber;
    NTSTATUS Status;
    USHORT Bus, Direction, Event, Reserved;
    ULONGLONG Extra;
};
struct AR_TIMING_RESPONSE {
    ULONG Version, Count, Total, Capacity;
    ULONGLONG QpcFrequency, Dropped, Exhausted;
    AR_TIMING_RECORD Records[AR_TIMING_CHUNK];
};
C_ASSERT(AR_TIMING_IOCTL == 0x0022E010);
C_ASSERT(sizeof(AR_TIMING_REQUEST) == 16);
C_ASSERT(sizeof(AR_TIMING_RECORD) == 64);
C_ASSERT(sizeof(AR_TIMING_RESPONSE) == 8232);

inline bool AudioRouterTimingValidRequest(const AR_TIMING_REQUEST& request) {
    return !request.Reserved[0] && !request.Reserved[1] &&
        request.Action >= AR_TIMING_START && request.Action <= AR_TIMING_READ &&
        (request.Action == AR_TIMING_READ || request.Offset == 0) &&
        request.Offset <= AR_TIMING_CAPACITY;
}

#if defined(AR_TIMING_DIAGNOSTICS)
ULONGLONG AudioRouterTimingStreamId();
void AudioRouterTimingRecord(ULONGLONG Qpc, ULONGLONG StreamId,
    ULONGLONG PacketCount, ULONGLONG LinearBytes, ULONG PacketNumber,
    NTSTATUS Status, USHORT Bus, USHORT Direction, USHORT Event, ULONGLONG Extra);
#endif
