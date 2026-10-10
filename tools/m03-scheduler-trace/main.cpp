// Offline ETL inspection only: no session creation/control or driver access.
#include <windows.h>
#include <evntrace.h>
#include <evntcons.h>
#include <cstdio>
#include <cstring>
#include <cwchar>
#include <algorithm>
#include <map>
#include <string>
#include <vector>

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

// --activity: per time bucket, each CPU's non-idle share, the processes using
// CPU (names from kernel process/thread rundown and start events), and for
// real-time-class threads of the named processes the longest wait before becoming ready and
// the longest ready-to-running delay. Late timers show up as long waits
// before readiness on every audio thread at once; CPU contention shows up as
// long ready-to-running delays.
namespace activity {
static const GUID kProcess = {0x3d6fa8d0, 0xfe05, 0x11d0, {0x9d, 0xda, 0x00, 0xc0, 0x4f, 0xd7, 0xba, 0x7c}};
static const GUID kThread = {0x3d6fa8d1, 0xfe05, 0x11d0, {0x9d, 0xda, 0x00, 0xc0, 0x4f, 0xd7, 0xba, 0x7c}};
static constexpr unsigned kCpus = 64;

struct Bucket {
    long long busy[kCpus]{};
    std::map<ULONG, long long> processTime;
    long long audioWait = 0, audioReady = 0;
};
static long long width, origin = -1;
static std::vector<Bucket> buckets;
static std::map<ULONG, ULONG> threadProcess;
static std::map<ULONG, std::string> processName;
static std::vector<std::string> watched;
static ULONG currentThread[kCpus];
static long long since[kCpus];
static bool seen[kCpus];
static std::map<ULONG, long long> switchedOut, readied;
static unsigned long long rejected;
// Pass 1 counts switch-ins per thread; pass 2 treats only periodic threads
// (at least kPeriodicPerSecond switch-ins per second over the trace) as audio.
static constexpr double kPeriodicPerSecond = 50.0;
static int pass = 1;
static std::map<ULONG, unsigned long long> switchIns;
static long long firstTick = -1, lastTick = -1, frequencyTicks = 1;
static bool Periodic(ULONG thread) {
    const auto count = switchIns.find(thread);
    const double span = static_cast<double>(lastTick - firstTick) / static_cast<double>(frequencyTicks);
    return count != switchIns.end() && span > 0 && static_cast<double>(count->second) / span >= kPeriodicPerSecond;
}

static Bucket& At(long long tick) {
    const size_t index = static_cast<size_t>((tick - origin) / width);
    if (index >= buckets.size()) buckets.resize(index + 1);
    return buckets[index];
}

static bool Watched(ULONG thread) {
    const auto process = threadProcess.find(thread);
    if (process == threadProcess.end()) return false;
    const auto name = processName.find(process->second);
    if (name == processName.end()) return false;
    for (const auto& candidate : watched) {
        if (_stricmp(candidate.c_str(), name->second.c_str()) == 0) return true;
    }
    return false;
}

// Account [from, to) of one thread on one CPU, split across buckets.
static void Account(unsigned cpu, ULONG thread, long long from, long long to) {
    while (from < to) {
        const long long bucketEnd = origin + ((from - origin) / width + 1) * width;
        const long long end = to < bucketEnd ? to : bucketEnd;
        Bucket& bucket = At(from);
        if (thread != 0) {
            bucket.busy[cpu] += end - from;
            const auto process = threadProcess.find(thread);
            bucket.processTime[process == threadProcess.end() ? 0xffffffffUL : process->second] += end - from;
        }
        from = end;
    }
}

// Kernel Process event (x64 or x86 pointer size): name after the SID.
static void ProcessEvent(const EVENT_RECORD* event) {
    const bool wide = (event->EventHeader.Flags & EVENT_HEADER_FLAG_64_BIT_HEADER) != 0;
    const size_t pointer = wide ? 8 : 4;
    const BYTE version = event->EventHeader.EventDescriptor.Version;
    const auto* data = static_cast<const BYTE*>(event->UserData);
    const size_t length = event->UserDataLength;
    size_t offset = pointer;  // UniqueProcessKey
    if (!data || length < offset + 16 + pointer) { ++rejected; return; }
    ULONG process;
    std::memcpy(&process, data + offset, 4);
    offset += 16 + pointer;  // ProcessId, ParentId, SessionId, ExitStatus, DirectoryTableBase
    if (version >= 4) offset += 4;  // Flags
    if (offset + 4 > length) { ++rejected; return; }
    ULONG sidMarker;
    std::memcpy(&sidMarker, data + offset, 4);
    if (sidMarker == 0) {
        offset += 4;
    } else {
        // TOKEN_USER (two pointers) followed by a SID whose sub-authority count
        // is its second byte (PerfView ProcessTraceData.SkipSID).
        const size_t token = 2 * pointer;
        if (offset + token + 2 > length) { ++rejected; return; }
        offset += token + 8 + 4 * static_cast<size_t>(data[offset + token + 1]);
    }
    if (offset >= length) { ++rejected; return; }
    std::string name;
    for (; offset < length && data[offset] != 0 && name.size() < 260; ++offset) name.push_back(static_cast<char>(data[offset]));
    processName[process] = name;
}

static void WINAPI Record(EVENT_RECORD* event) {
    const long long tick = event->EventHeader.TimeStamp.QuadPart;
    const auto& provider = event->EventHeader.ProviderId;
    const auto opcode = event->EventHeader.EventDescriptor.Opcode;
    if (IsEqualGUID(provider, kProcess) && (opcode == 1 || opcode == 3)) { ProcessEvent(event); return; }
    if (!IsEqualGUID(provider, kThread)) return;
    if ((opcode == 1 || opcode == 3) && event->UserData && event->UserDataLength >= 8) {
        ULONG process, thread;
        std::memcpy(&process, event->UserData, 4);
        std::memcpy(&thread, static_cast<const BYTE*>(event->UserData) + 4, 4);
        threadProcess[thread] = process;
        return;
    }
    if (pass == 1) {
        if (opcode == 36 && event->UserData && event->UserDataLength >= 4) {
            ULONG next;
            std::memcpy(&next, event->UserData, 4);
            ++switchIns[next];
            if (firstTick < 0) firstTick = tick;
            lastTick = tick;
        }
        return;
    }
    if (origin < 0) origin = tick;
    if (opcode == 50) {
        if (!event->UserData || event->UserDataLength < 8) { ++rejected; return; }
        ULONG thread;
        std::memcpy(&thread, event->UserData, 4);
        if (switchedOut.count(thread) && !readied.count(thread)) readied[thread] = tick;
        return;
    }
    if (opcode != 36) return;
    SwitchFields fields{};
    if (!DecodeSwitch(event->UserData, event->UserDataLength, event->EventHeader.EventDescriptor.Version, fields)) { ++rejected; return; }
    const unsigned cpu = event->BufferContext.ProcessorNumber;
    if (cpu >= kCpus) { ++rejected; return; }
    if (seen[cpu]) Account(cpu, currentThread[cpu], since[cpu], tick);
    currentThread[cpu] = fields.next;
    since[cpu] = tick;
    seen[cpu] = true;
    if (fields.previous != 0) { switchedOut[fields.previous] = tick; readied.erase(fields.previous); }
    const auto off = switchedOut.find(fields.next);
    if (off != switchedOut.end()) {
        // Only real-time-class threads (priority 16+, where MMCSS Pro Audio
        // threads run): control, disk and idle threads sleep on purpose.
        if (fields.next != 0 && fields.nextPriority >= 16 && Watched(fields.next) && Periodic(fields.next)) {
            const auto readyEntry = readied.find(fields.next);
            const long long readyAt = readyEntry == readied.end() ? tick : readyEntry->second;
            Bucket& bucket = At(tick);
            if (readyAt - off->second > bucket.audioWait) bucket.audioWait = readyAt - off->second;
            if (tick - readyAt > bucket.audioReady) bucket.audioReady = tick - readyAt;
        }
        switchedOut.erase(off);
        readied.erase(fields.next);
    }
}

static int Run(const wchar_t* etl, const wchar_t* csv, const wchar_t* bucketMs, const wchar_t* names) {
    wchar_t* end = nullptr;
    const long milliseconds = std::wcstol(bucketMs, &end, 10);
    if (!end || *end != L'\0' || milliseconds < 10 || milliseconds > 60000) { std::fprintf(stderr, "Bucket must be 10..60000 ms.\n"); return 2; }
    for (const wchar_t* cursor = names; *cursor;) {
        std::string name;
        while (*cursor && *cursor != L',') { if (*cursor > 127) { std::fprintf(stderr, "Process names must be ASCII.\n"); return 2; } name.push_back(static_cast<char>(*cursor++)); }
        if (!name.empty()) watched.push_back(name);
        if (*cursor == L',') ++cursor;
    }
    if (watched.empty()) { std::fprintf(stderr, "Name at least one process to watch.\n"); return 2; }
    if (GetFileAttributesW(csv) != INVALID_FILE_ATTRIBUTES) { std::fprintf(stderr, "Output exists; choose a new path.\n"); return 3; }
    EVENT_TRACE_LOGFILEW log{};
    ULONG result = 0, closeResult = 0;
    long long frequency = 0;
    for (pass = 1; pass <= 2; ++pass) {
        log = EVENT_TRACE_LOGFILEW{};
        log.LogFileName = const_cast<wchar_t*>(etl);
        log.ProcessTraceMode = PROCESS_TRACE_MODE_EVENT_RECORD | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
        log.EventRecordCallback = Record;
        TRACEHANDLE handle = OpenTraceW(&log);
        if (handle == INVALID_PROCESSTRACE_HANDLE) { std::fprintf(stderr, "OpenTrace failed: %lu\n", GetLastError()); return 4; }
        frequency = log.LogfileHeader.PerfFreq.QuadPart;
        if (log.LogfileHeader.ReservedFlags != 1 || frequency <= 0) { std::fprintf(stderr, "Unsupported trace clock.\n"); CloseTrace(handle); return 5; }
        width = frequency * milliseconds / 1000;
        frequencyTicks = frequency;
        result |= ProcessTrace(&handle, 1, nullptr, nullptr);
        closeResult |= CloseTrace(handle);
        if (pass == 1) rejected = 0;  // pass 2 decodes and counts every record
    }
    unsigned long long periodic = 0;
    for (const auto& entry : switchIns) if (Watched(entry.first) && Periodic(entry.first)) ++periodic;
    FILE* out = nullptr;
    if (_wfopen_s(&out, csv, L"wb") != 0 || !out) return 3;
    unsigned cpus = 0;
    for (unsigned cpu = 0; cpu < kCpus; ++cpu) if (seen[cpu]) cpus = cpu + 1;
    std::fprintf(out, "bucket_start_s,origin_qpc");
    for (unsigned cpu = 0; cpu < cpus; ++cpu) std::fprintf(out, ",cpu%u_busy_pct", cpu);
    std::fprintf(out, ",watched_max_wait_before_ready_ms,watched_max_ready_to_run_ms,top_processes\n");
    for (size_t index = 0; index < buckets.size(); ++index) {
        const Bucket& bucket = buckets[index];
        std::fprintf(out, "%.3f,%lld", static_cast<double>(index) * milliseconds / 1000.0, origin);
        for (unsigned cpu = 0; cpu < cpus; ++cpu) std::fprintf(out, ",%.1f", 100.0 * bucket.busy[cpu] / width);
        std::fprintf(out, ",%.3f,%.3f,", 1000.0 * bucket.audioWait / frequency, 1000.0 * bucket.audioReady / frequency);
        std::vector<std::pair<long long, ULONG>> top;
        for (const auto& entry : bucket.processTime) top.push_back({entry.second, entry.first});
        std::sort(top.rbegin(), top.rend());
        for (size_t rank = 0; rank < top.size() && rank < 5; ++rank) {
            const auto name = processName.find(top[rank].second);
            std::fprintf(out, "%s%s:%lu=%.1f%%", rank ? ";" : "",
                name == processName.end() ? "?" : name->second.c_str(), top[rank].second, 100.0 * top[rank].first / width);
        }
        std::fprintf(out, "\n");
    }
    const bool writeError = std::ferror(out) != 0;
    const int flushResult = std::fclose(out);
    std::printf("ProcessTrace=%lu CloseTrace=%lu Buckets=%zu Processes=%zu Threads=%zu PeriodicWatchedThreads=%llu Rejected=%llu EventsLost=%lu\n",
        result, closeResult, buckets.size(), processName.size(), threadProcess.size(), periodic, rejected, log.LogfileHeader.EventsLost);
    return result != 0 || closeResult != 0 || rejected != 0 || log.LogfileHeader.EventsLost != 0 || writeError || flushResult != 0;
}
}  // namespace activity

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
    if (argc == 6 && std::wcscmp(argv[1], L"--activity") == 0) return activity::Run(argv[2], argv[3], argv[4], argv[5]);
    if (argc < 3 || argc > 4 || (argc == 4 && std::wcscmp(argv[3], L"--raw") != 0)) {
        std::fprintf(stderr, "Usage: m03-scheduler-trace.exe input.etl output.csv [--raw] | --silences input.etl output.csv threshold_ms | --activity input.etl output.csv bucket_ms a.exe,b.exe\n"); return 2;
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
