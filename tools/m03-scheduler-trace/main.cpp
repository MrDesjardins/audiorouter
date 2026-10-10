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

// --silences: system-wide intervals in which no event of any kind (switch,
// ready, DPC, ISR, timer, process/image) was recorded on any CPU. A frozen
// virtual machine produces none; a busy or starved guest still records some.
static constexpr unsigned kMaxCpus = 256;
static bool silenceMode;
static long long silenceThreshold, firstTick, lastTick, cpuLast[kMaxCpus], cpuMaxGap[kMaxCpus];
static unsigned long long events, silences;
static long long longestSilence;
static void WINAPI Silence(EVENT_RECORD* event) {
    static const GUID header = {0x68fdd900, 0x4a3e, 0x11d1, {0x84, 0xf4, 0x00, 0x00, 0xf8, 0x04, 0x64, 0xe3}};
    if (IsEqualGUID(event->EventHeader.ProviderId, header)) return;  // logfile header, not guest activity
    const long long tick = event->EventHeader.TimeStamp.QuadPart;
    const unsigned cpu = event->BufferContext.ProcessorNumber;
    if (events++ == 0) firstTick = tick;
    else if (tick - lastTick > silenceThreshold) {
        ++silences;
        std::fprintf(output, "%lld,%lld,%lld,%u,%u\n", lastTick, tick, tick - lastTick, cpu,
            static_cast<unsigned>(event->EventHeader.EventDescriptor.Opcode));
    }
    if (events > 1 && tick - lastTick > longestSilence) longestSilence = tick - lastTick;
    if (tick > lastTick) lastTick = tick;
    if (cpu < kMaxCpus) {
        if (cpuLast[cpu] != 0 && tick - cpuLast[cpu] > cpuMaxGap[cpu]) cpuMaxGap[cpu] = tick - cpuLast[cpu];
        cpuLast[cpu] = tick;
    }
}
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

static int Silences(const wchar_t* etl, const wchar_t* csv, const wchar_t* thresholdMs) {
    wchar_t* end = nullptr;
    const long milliseconds = std::wcstol(thresholdMs, &end, 10);
    if (!end || *end != L'\0' || milliseconds < 1 || milliseconds > 600000) {
        std::fprintf(stderr, "Threshold must be 1..600000 ms.\n"); return 2;
    }
    if (GetFileAttributesW(csv) != INVALID_FILE_ATTRIBUTES) { std::fprintf(stderr, "Output exists; choose a new path.\n"); return 3; }
    EVENT_TRACE_LOGFILEW log{};
    log.LogFileName = const_cast<wchar_t*>(etl);
    log.ProcessTraceMode = PROCESS_TRACE_MODE_EVENT_RECORD | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
    log.EventRecordCallback = Silence;
    TRACEHANDLE handle = OpenTraceW(&log);
    if (handle == INVALID_PROCESSTRACE_HANDLE) { std::fprintf(stderr, "OpenTrace failed: %lu\n", GetLastError()); return 4; }
    // Raw timestamps use the session clock; only QPC (ReservedFlags 1) has PerfFreq.
    const long long frequency = log.LogfileHeader.PerfFreq.QuadPart;
    if (log.LogfileHeader.ReservedFlags != 1 || frequency <= 0) {
        std::fprintf(stderr, "Unsupported trace clock %lu.\n", log.LogfileHeader.ReservedFlags); CloseTrace(handle); return 5;
    }
    if (_wfopen_s(&output, csv, L"wb") != 0 || !output) { CloseTrace(handle); return 3; }
    std::fprintf(output, "start_qpc,end_qpc,duration_ticks,next_cpu,next_opcode\n");
    silenceMode = true;
    silenceThreshold = frequency * milliseconds / 1000;
    const ULONG result = ProcessTrace(&handle, 1, nullptr, nullptr);
    const ULONG closeResult = CloseTrace(handle);
    const bool writeError = std::ferror(output) != 0;
    const int flushResult = std::fclose(output);
    std::printf("ProcessTrace=%lu CloseTrace=%lu Events=%llu EventsLost=%lu QpcFrequency=%lld Span=%.3f s\n",
        result, closeResult, events, log.LogfileHeader.EventsLost, frequency,
        events ? static_cast<double>(lastTick - firstTick) / frequency : 0.0);
    std::printf("Silences>=%ld ms with no event on any CPU: %llu; longest system-wide silence %.3f ms\n",
        milliseconds, silences, static_cast<double>(longestSilence) * 1000.0 / frequency);
    for (unsigned cpu = 0; cpu < kMaxCpus; ++cpu) {
        if (cpuLast[cpu] != 0) std::printf("CPU %u longest gap between its events: %.3f ms\n", cpu,
            static_cast<double>(cpuMaxGap[cpu]) * 1000.0 / frequency);
    }
    return result != 0 || closeResult != 0 || log.LogfileHeader.EventsLost != 0 || writeError || flushResult != 0 || events == 0;
}

int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && std::wcscmp(argv[1], L"--self-test") == 0) return SelfTest();
    if (argc == 5 && std::wcscmp(argv[1], L"--silences") == 0) return Silences(argv[2], argv[3], argv[4]);
    if (argc < 3 || argc > 4 || (argc == 4 && std::wcscmp(argv[3], L"--raw") != 0)) {
        std::fprintf(stderr, "Usage: m03-scheduler-trace.exe input.etl output.csv [--raw] | --silences input.etl output.csv threshold_ms\n"); return 2;
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
