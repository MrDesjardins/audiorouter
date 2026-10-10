// Offline ETL inspection only: no session creation/control or driver access.
#include <windows.h>
#include <evntrace.h>
#include <evntcons.h>
#include <cstdio>
#include <cstring>
#include <cwchar>

struct SwitchFields {
    ULONG next, previous;
    BYTE nextPriority, previousPriority, waitReason, waitMode, previousState;
};

// Common prefix documented by Microsoft's PerfView CSwitchTraceData parser.
// ETW owns the payload for the synchronous callback. Copy checked scalars;
// never retain its pointers. Ignore appended fields in newer versions.
static bool DecodeSwitch(const void* data, USHORT length, BYTE version, SwitchFields& result) {
    if (!data || length < 24 || version < 2) return false;
    BYTE bytes[24];
    std::memcpy(bytes, data, sizeof(bytes));
    std::memcpy(&result.next, bytes, sizeof(ULONG));
    std::memcpy(&result.previous, bytes + 4, sizeof(ULONG));
    result.nextPriority = bytes[8]; result.previousPriority = bytes[9];
    result.waitReason = bytes[12]; result.waitMode = bytes[13]; result.previousState = bytes[14];
    return true;
}

static int SelfTest() {
    BYTE bytes[40]{};
    ULONG next = 0x100, previous = 0x200;
    std::memcpy(bytes, &next, 4); std::memcpy(bytes + 4, &previous, 4);
    bytes[8] = 24; bytes[9] = 8; bytes[12] = 6; bytes[13] = 0; bytes[14] = 5;
    std::memset(bytes + 24, 0xff, 16);
    SwitchFields decoded{};
    unsigned checks = 0;
#define CHECK(condition) do { if (!(condition)) { std::fprintf(stderr, "Failed line %d\n", __LINE__); return 1; } ++checks; } while (0)
    CHECK(DecodeSwitch(bytes, 24, 2, decoded));
    CHECK(DecodeSwitch(bytes, 40, 5, decoded));
    CHECK(decoded.next == next && decoded.previous == previous);
    CHECK(decoded.nextPriority == 24 && decoded.previousPriority == 8);
    CHECK(decoded.waitReason == 6 && decoded.waitMode == 0 && decoded.previousState == 5);
    CHECK(!DecodeSwitch(bytes, 23, 5, decoded));
    CHECK(!DecodeSwitch(bytes, 40, 1, decoded));
    CHECK(!DecodeSwitch(nullptr, 40, 5, decoded));
#undef CHECK
    std::printf("%u offline decoder checks pass\n", checks);
    return 0;
}

static FILE* output;
static unsigned long long switches, ready, rejected;
static void WINAPI Record(EVENT_RECORD* event) {
    static const GUID thread = {0x3d6fa8d1, 0xfe05, 0x11d0, {0x9d, 0xda, 0x00, 0xc0, 0x4f, 0xd7, 0xba, 0x7c}};
    if (!IsEqualGUID(event->EventHeader.ProviderId, thread)) return;
    const auto opcode = event->EventHeader.EventDescriptor.Opcode;
    const auto version = event->EventHeader.EventDescriptor.Version;
    const auto tick = event->EventHeader.TimeStamp.QuadPart;
    if (opcode == 36) {
        SwitchFields fields{};
        if (!DecodeSwitch(event->UserData, event->UserDataLength, version, fields)) { ++rejected; return; }
        ++switches;
        std::fprintf(output, "switch,%lld,%u,%lu,%lu,%u,%u,%u,%u,%u,%u\n", tick,
            event->BufferContext.ProcessorNumber, fields.next, fields.previous,
            fields.nextPriority, fields.previousPriority, fields.waitReason, fields.waitMode, fields.previousState, version);
    } else if (opcode == 50) {
        if (event->UserDataLength < 8 || !event->UserData) { ++rejected; return; }
        ULONG tid;
        std::memcpy(&tid, event->UserData, sizeof(tid));
        ++ready;
        std::fprintf(output, "ready,%lld,%u,%lu,0,0,0,0,0,0,%u\n", tick,
            event->BufferContext.ProcessorNumber, tid, version);
    }
}

int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && std::wcscmp(argv[1], L"--self-test") == 0) return SelfTest();
    if (argc < 3 || argc > 4 || (argc == 4 && std::wcscmp(argv[3], L"--raw") != 0)) {
        std::fprintf(stderr, "Usage: m03-scheduler-trace.exe input.etl output.csv [--raw]\n"); return 2;
    }
    const DWORD attributes = GetFileAttributesW(argv[2]);
    if (attributes != INVALID_FILE_ATTRIBUTES) { std::fprintf(stderr, "Output exists; choose a new path.\n"); return 3; }
    if (_wfopen_s(&output, argv[2], L"wb") != 0 || !output) return 3;
    std::fprintf(output, "kind,timestamp,cpu,new_tid,old_tid,new_priority,old_priority,wait_reason,wait_mode,old_state,version\n");
    EVENT_TRACE_LOGFILEW log{};
    log.LogFileName = argv[1];
    log.ProcessTraceMode = PROCESS_TRACE_MODE_EVENT_RECORD;
    if (argc == 4) log.ProcessTraceMode |= PROCESS_TRACE_MODE_RAW_TIMESTAMP;
    log.EventRecordCallback = Record;
    TRACEHANDLE handle = OpenTraceW(&log);
    if (handle == INVALID_PROCESSTRACE_HANDLE) {
        std::fprintf(stderr, "OpenTrace failed: %lu\n", GetLastError()); std::fclose(output); return 4;
    }
    const ULONG result = ProcessTrace(&handle, 1, nullptr, nullptr);
    const ULONG closeResult = CloseTrace(handle);
    const bool writeError = std::ferror(output) != 0;
    const int flushResult = std::fclose(output);
    std::printf("ProcessTrace=%lu CloseTrace=%lu CSwitch=%llu ReadyThread=%llu Rejected=%llu EventsLost=%lu\n",
        result, closeResult, switches, ready, rejected, log.LogfileHeader.EventsLost);
    return result != 0 || closeResult != 0 || rejected != 0 || log.LogfileHeader.EventsLost != 0 || writeError || flushResult != 0;
}
