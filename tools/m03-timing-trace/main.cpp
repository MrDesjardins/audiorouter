#include <windows.h>
#include <winternl.h>
#include <cstdio>
#include <cstdlib>
#include <memory>
#include <thread>
#include <vector>
#include <string>
#include "../../drivers/audiorouter-virtual/Source/Inc/timingtracebuffer.h"

static int SelfTest() {
    for (ULONG action = 0; action != 5; ++action) {
        for (ULONG offset : {0UL, 1UL, AR_TIMING_CAPACITY, AR_TIMING_CAPACITY + 1}) {
            AR_TIMING_REQUEST request = { action, offset, {} };
            bool expected = action >= AR_TIMING_START && action <= AR_TIMING_READ &&
                (action == AR_TIMING_READ || offset == 0) && offset <= AR_TIMING_CAPACITY;
            if (AudioRouterTimingValidRequest(request) != expected) return 9;
            request.Reserved[0] = 1;
            if (AudioRouterTimingValidRequest(request)) return 10;
            request.Reserved[0] = 0; request.Reserved[1] = 1;
            if (AudioRouterTimingValidRequest(request)) return 11;
        }
    }
    auto buffer = std::make_unique<AudioRouterTimingBuffer>();
    buffer->Close(); buffer->Reset();
    AR_TIMING_RECORD record = {}; record.Qpc = 100;
    buffer->Append(record);
    if (buffer->Count != 0) return 1;
    buffer->Open(101); buffer->Append(record);
    if (buffer->Count != 0) return 2;
    record.Qpc = 102;
    std::vector<std::thread> threads;
    for (unsigned t = 0; t != 8; ++t) threads.emplace_back([&]() {
        for (unsigned n = 0; n != 10000; ++n) buffer->Append(record);
    });
    for (auto& thread : threads) thread.join();
    buffer->Close();
    if (!buffer->Drained() || buffer->Count + buffer->Dropped + buffer->AdmissionDropped != 80000) return 3;
    for (LONG64 n = 0; n < buffer->Count; ++n)
        if (buffer->Records[n].Sequence != static_cast<ULONGLONG>(n) + 1 || buffer->Records[n].Qpc != 102) return 4;
    buffer->Reset(); buffer->Open(103); record.Qpc = 104;
    for (unsigned n = 0; n != AR_TIMING_CAPACITY + 7; ++n) buffer->Append(record);
    buffer->Close();
    if (!buffer->Drained() || buffer->Count != AR_TIMING_CAPACITY || buffer->Exhausted != 7) return 5;
    LONG64 oldState = buffer->Gate;
    buffer->Reset(); buffer->Open(105);
    if (InterlockedCompareExchange64(&buffer->Gate, oldState + 1, oldState) == oldState) return 6;
    record.Qpc = 104; buffer->Append(record);
    if (buffer->Count) return 7;
    for (unsigned cycle = 0; cycle != 100; ++cycle) {
        threads.clear(); record.Qpc = 106;
        for (unsigned t = 0; t != 4; ++t) threads.emplace_back([&]() {
            for (unsigned n = 0; n != 1000; ++n) buffer->Append(record);
        });
        buffer->Close();
        while (!buffer->Drained()) std::this_thread::yield();
        // No record/counter mutation after drain; closed callers can still run.
        LONG64 count = buffer->Count;
        for (auto& thread : threads) thread.join();
        if (buffer->Count != count) return 8;
        buffer->Reset(); buffer->Open(105);
    }
    buffer->Close();
    buffer->Gate = 0x7fffffff80000000LL;
    if (buffer->Open(107) || !buffer->Closed()) return 12;
    std::puts("Timing ABI/request bounds, concurrent append, exhaustion, epoch saturation, old QPC and 100 stop/drain cycles pass.");
    return 0;
}

static bool Control(HANDLE device, ULONG action, ULONG offset, AR_TIMING_RESPONSE& response) {
    AR_TIMING_REQUEST request = { action, offset, {} }; DWORD returned = 0;
    if (!DeviceIoControl(device, AR_TIMING_IOCTL, &request, sizeof(request),
        &response, sizeof(response), &returned, nullptr)) {
        std::fprintf(stderr, "timing IOCTL action=%lu error=%lu\n", action, GetLastError()); return false;
    }
    return returned == sizeof(response) && response.Version == 1 &&
        response.Capacity == AR_TIMING_CAPACITY && response.Total <= AR_TIMING_CAPACITY &&
        response.Count <= AR_TIMING_CHUNK && response.QpcFrequency > 0;
}
static std::wstring Quote(const std::wstring& text) {
    // All inputs are paths; reject quotes instead of interpreting command text.
    if (text.find(L'"') != std::wstring::npos) return L"";
    return L"\"" + text + L"\"";
}
int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && wcscmp(argv[1], L"selftest") == 0) return SelfTest();
    if (argc != 3) { std::fprintf(stderr, "Usage: m03-timing-trace <bundle> <evidence directory>\n"); return 1; }
    wchar_t computer[MAX_COMPUTERNAME_LENGTH + 1]; DWORD computerChars = ARRAYSIZE(computer);
    DWORD marker = GetFileAttributesW(L"C:\\ar\\IS_TEST_VM");
    if (!GetComputerNameW(computer, &computerChars) || _wcsicmp(computer, L"AR-DriverTest") != 0 ||
        marker == INVALID_FILE_ATTRIBUTES || (marker & FILE_ATTRIBUTE_DIRECTORY)) {
        std::fprintf(stderr, "Guest-only: AR-DriverTest required. No device opened.\n"); return 1;
    }
    HANDLE token;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &token)) return 1;
    TOKEN_PRIVILEGES privileges = {}; privileges.PrivilegeCount = 1;
    if (!LookupPrivilegeValueW(nullptr, L"SeLoadDriverPrivilege", &privileges.Privileges[0].Luid)) { CloseHandle(token); return 1; }
    privileges.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
    SetLastError(ERROR_SUCCESS);
    BOOL adjusted = AdjustTokenPrivileges(token, FALSE, &privileges, 0, nullptr, nullptr);
    DWORD privilegeError = GetLastError(); CloseHandle(token);
    if (!adjusted || privilegeError != ERROR_SUCCESS) { std::fprintf(stderr, "Elevated guest privilege required.\n"); return 1; }
    HANDLE device = CreateFileW(L"\\\\.\\AudioRouterVirtualBridge", GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE, nullptr, OPEN_EXISTING, 0, nullptr);
    if (device == INVALID_HANDLE_VALUE) { std::fprintf(stderr, "device error=%lu\n", GetLastError()); return 1; }
    AR_TIMING_RESPONSE response = {};
    if (!Control(device, AR_TIMING_START, 0, response)) { CloseHandle(device); return 1; }
    std::wstring bundle = argv[1], evidence = argv[2];
    wchar_t system[MAX_PATH]; UINT systemLength = GetSystemDirectoryW(system, ARRAYSIZE(system));
    int result = 1;
    if (systemLength && systemLength < ARRAYSIZE(system)) {
        std::wstring powershell = std::wstring(system) + L"\\WindowsPowerShell\\v1.0\\powershell.exe";
        std::wstring command = Quote(powershell) + L" -NoProfile -ExecutionPolicy Bypass -File " +
            Quote(bundle + L"\\run-cable-latency-starts.ps1") +
            L" -Route engine -Starts 1 -Impulses 1000 -TraceRoute -PeriodMode low-latency";
        SECURITY_ATTRIBUTES security = { sizeof(security), nullptr, TRUE };
        HANDLE out = CreateFileW((evidence + L"\\measurement.txt").c_str(), GENERIC_WRITE, FILE_SHARE_READ,
            &security, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
        HANDLE err = CreateFileW((evidence + L"\\measurement-stderr.txt").c_str(), GENERIC_WRITE, FILE_SHARE_READ,
            &security, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
        HANDLE input = CreateFileW(L"NUL", GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE,
            &security, OPEN_EXISTING, 0, nullptr);
        HANDLE job = CreateJobObjectW(nullptr, nullptr);
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION limit = {};
        limit.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        bool ready = out != INVALID_HANDLE_VALUE && err != INVALID_HANDLE_VALUE && input != INVALID_HANDLE_VALUE && job &&
            SetInformationJobObject(job, JobObjectExtendedLimitInformation, &limit, sizeof(limit));
        STARTUPINFOW startup = {}; startup.cb = sizeof(startup);
        startup.dwFlags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW; startup.wShowWindow = SW_HIDE;
        startup.hStdOutput = out; startup.hStdError = err; startup.hStdInput = input;
        PROCESS_INFORMATION process = {};
        if (ready && CreateProcessW(powershell.c_str(), command.data(), nullptr, nullptr, TRUE,
            CREATE_SUSPENDED | CREATE_NO_WINDOW, nullptr, bundle.c_str(), &startup, &process)) {
            if (AssignProcessToJobObject(job, process.hProcess) && ResumeThread(process.hThread) != static_cast<DWORD>(-1)) {
                if (WaitForSingleObject(process.hProcess, 120000) == WAIT_OBJECT_0) {
                    DWORD exitCode; if (GetExitCodeProcess(process.hProcess, &exitCode)) result = static_cast<int>(exitCode);
                }
            }
            // On failure or watchdog, kill only the job's owned process tree.
            if (result != 0) { TerminateJobObject(job, 124); TerminateProcess(process.hProcess, 124); }
            WaitForSingleObject(process.hProcess, 10000);
            CloseHandle(process.hThread); CloseHandle(process.hProcess);
        }
        if (job) CloseHandle(job);
        if (out != INVALID_HANDLE_VALUE) CloseHandle(out);
        if (err != INVALID_HANDLE_VALUE) CloseHandle(err);
        if (input != INVALID_HANDLE_VALUE) CloseHandle(input);
    }
    if (!Control(device, AR_TIMING_STOP, 0, response)) result = 1;
    else {
        const ULONG total = response.Total;
        std::printf("timing_records=%lu qpc_frequency=%llu dropped_including_lifetime_admission=%llu exhausted=%llu\n",
            response.Total, response.QpcFrequency, response.Dropped, response.Exhausted);
        FILE* file = nullptr;
        if (_wfopen_s(&file, (evidence + L"\\driver-timing.csv").c_str(), L"wb") || !file) result = 1;
        else {
            std::fputs("sequence,qpc,stream_id,packet_count,linear_bytes,packet_number,status,bus,direction,event,extra\n", file);
            for (ULONG offset = 0; offset < total; ) {
                if (!Control(device, AR_TIMING_READ, offset, response) || response.Total != total || !response.Count ||
                    response.Count > total - offset) { result = 1; break; }
                for (ULONG n = 0; n < response.Count; ++n) {
                    const auto& r = response.Records[n];
                    if (r.Sequence != static_cast<ULONGLONG>(offset) + n + 1) result = 1;
                    std::fprintf(file, "%llu,%llu,%llu,%llu,%llu,%lu,0x%08lx,%hu,%hu,%hu,%llu\n",
                        r.Sequence,r.Qpc,r.StreamId,r.PacketCount,r.LinearBytes,r.PacketNumber,
                        static_cast<ULONG>(r.Status),r.Bus,r.Direction,r.Event,r.Extra);
                }
                offset += response.Count;
            }
            if (std::ferror(file)) result = 1;
            if (std::fclose(file)) result = 1;
        }
        if (!total || response.Dropped || response.Exhausted) result = 1;
    }
    // Closing the owning handle stops/drains/scrubs even on any error above.
    CloseHandle(device);
    return result;
}
