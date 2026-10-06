#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <winioctl.h>
#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <functional>
#include <iterator>
#include <random>
#include <thread>
#include <vector>

namespace {
constexpr DWORD kOpen = CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA);
constexpr DWORD kClose = CTL_CODE(FILE_DEVICE_UNKNOWN, 0x801, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA);
constexpr DWORD kHeartbeat = CTL_CODE(FILE_DEVICE_UNKNOWN, 0x802, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA);
constexpr DWORD kQuery = CTL_CODE(FILE_DEVICE_UNKNOWN, 0x803, METHOD_BUFFERED, FILE_READ_DATA);
// Protocol 1.1 (bridgeio.h): 128-byte shared header, float64 payload.
constexpr DWORD kHeaderBytes = 128;
constexpr DWORD kBytes = kHeaderBytes + 128 * 2 * sizeof(double);
constexpr DWORD kShortSectionBytes = 64;
constexpr DWORD kPrefixBytes = 176;
constexpr DWORD kDriverInfoBytes = 104;

#pragma pack(push, 8)
struct Request {
    WORD major, minor, busBytes, channels, frames, direction;
    DWORD rate, leaseMs;
    ULONGLONG generation, sectionHandle;
    DWORD mappingBytes, reserved;
    wchar_t busId[64];
    // AR_BRIDGE_OPEN_EXTENSION: OPEN requires it with FLOAT64 (bit 0).
    DWORD extensionBytes, flags, extensionReserved[14];
};
#pragma pack(pop)
static_assert(offsetof(Request, extensionBytes) == kPrefixBytes, "bridge OPEN prefix ABI drift");
static_assert(sizeof(Request) == 240, "bridge OPEN extension ABI drift");

struct Stats { std::atomic<ULONGLONG> calls{0}, accepted{0}, rejected{0}, unexpected{0}, opened{0}, queried{0}; };
struct SharedSection {
    HANDLE handle = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE, 0, kBytes, nullptr);
    void* view = handle ? MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, kBytes) : nullptr;
    ~SharedSection() { if (view) UnmapViewOfFile(view); if (handle) CloseHandle(handle); }
    SharedSection(const SharedSection&) = delete;
    SharedSection& operator=(const SharedSection&) = delete;
    SharedSection() { if (view) ZeroMemory(view, kBytes); }
};
struct ShortSection {
    HANDLE handle = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE, 0, kShortSectionBytes, nullptr);
    ~ShortSection() { if (handle) CloseHandle(handle); }
};
Request makeRequest(USHORT direction, HANDLE section, ULONGLONG generation) {
    Request value{};
    value.major = 1; value.minor = 1; value.busBytes = 14;
    value.channels = 2; value.frames = 128; value.direction = direction;
    value.rate = 48000; value.leaseMs = 2000; value.generation = generation;
    value.sectionHandle = reinterpret_cast<ULONG_PTR>(section); value.mappingBytes = kBytes;
    std::memcpy(value.busId, L"cable-a", 14);
    value.extensionBytes = 64; value.flags = 1;
    return value;
}
bool expectedRejection(DWORD error) {
    // Statuses the driver returns for refused requests, as Win32 errors
    // (RtlNtStatusToDosError). Anything else fails the run for investigation.
    switch (error) {
    case ERROR_INVALID_PARAMETER: case ERROR_INSUFFICIENT_BUFFER:
    case ERROR_INVALID_HANDLE: case ERROR_ACCESS_DENIED: case ERROR_SHARING_VIOLATION:
    case ERROR_INVALID_FUNCTION: case ERROR_GEN_FAILURE: case ERROR_NOT_READY:
    case ERROR_BUSY: case ERROR_INVALID_STATE:
    case ERROR_NOT_SUPPORTED:          // unknown extension flag / prefix-only OPEN
    case ERROR_REVISION_MISMATCH:      // protocol major/minor
    case ERROR_DEVICE_NOT_CONNECTED:   // cable above the enabled count
    case ERROR_FILE_NOT_FOUND:         // unknown cable ID
    case ERROR_BAD_COMMAND:            // STATUS_INVALID_DEVICE_STATE: lease lost/replaced
        return true;                   // (a non-section handle maps to ERROR_INVALID_HANDLE)
    default:
        return false;
    }
}
void invoke(HANDLE device, DWORD code, const void* data, DWORD bytes, Stats& stats) {
    DWORD returned = 0;
    SetLastError(ERROR_SUCCESS);
    const BOOL ok = DeviceIoControl(device, code, const_cast<void*>(data), bytes,
                                    nullptr, 0, &returned, nullptr);
    ++stats.calls;
    if (ok) {
        ++stats.accepted;
        if (code == kOpen) ++stats.opened;
        return;
    }
    const DWORD error = GetLastError();
    if (expectedRejection(error)) ++stats.rejected;
    else {
        ++stats.unexpected;
        if (stats.unexpected.load() <= 16) std::fprintf(stderr, "unexpected DeviceIoControl Win32 error %lu (code 0x%08lx)\n", error, code);
    }
}

void runWorker(DWORD seconds, ULONGLONG seed, unsigned index, Stats& stats) {
    std::mt19937_64 random(seed + index * 0x9e3779b97f4a7c15ULL);
    HANDLE device = CreateFileW(L"\\\\.\\AudioRouterVirtualBridge",
        GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE,
        nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (device == INVALID_HANDLE_VALUE) {
        ++stats.unexpected;
        std::fprintf(stderr, "worker %u cannot open bridge: Win32 %lu\n", index, GetLastError());
        return;
    }
    SharedSection section;
    if (!section.handle || !section.view) {
        ++stats.unexpected;
        std::fprintf(stderr, "worker %u cannot create a section: Win32 %lu\n", index, GetLastError());
        CloseHandle(device);
        return;
    }
    const ULONGLONG end = GetTickCount64() + static_cast<ULONGLONG>(seconds) * 1000;
    const USHORT direction = (index & 1) ? 2 : 1;
    ULONGLONG generation = (seed ^ (static_cast<ULONGLONG>(index + 1) << 32)) | 1;
    Request current = makeRequest(direction, section.handle, generation);
    invoke(device, kOpen, &current, sizeof(current), stats);
    ShortSection shortSection;
    if (shortSection.handle) {
        // Valid ABI/shape but a genuinely undersized section object: kernel
        // mapping must fail cleanly before it can publish the lease.
        Request undersized = makeRequest(direction, shortSection.handle, generation + 1);
        undersized.channels = 2; // valid prototype shape, larger than 64-byte section
        undersized.frames = 128;
        undersized.mappingBytes = kHeaderBytes + undersized.channels * undersized.frames * sizeof(double);
        invoke(device, kOpen, &undersized, sizeof(undersized), stats);
    }
    const DWORD lengths[] = { 0, 1, 23, 175, 176, 177, 239, 240, 241, 255, 4096 };
    const DWORD codes[] = { kOpen, kClose, kHeartbeat, kQuery, 0, 0xffffffff };
    while (GetTickCount64() < end) {
        switch (random() % 10) {
        case 0: { // Exercise every fixed-structure boundary on all three IOCTLs.
            auto malformed = current;
            invoke(device, codes[random() % 3], &malformed, lengths[random() % std::size(lengths)], stats);
            break;
        }
        case 1: { // Invalid enums, lengths, reserved bits and hostile dimensions.
            auto malformed = current;
            DWORD bytes = sizeof(malformed);
            switch (random() % 16) {
            case 0: malformed.direction = 0; break;
            case 1: malformed.channels = 0xffff; break;
            case 2: malformed.frames = 0xffff; break;
            case 3: malformed.reserved = 1; break;
            case 4: malformed.leaseMs = 0xffffffff; break;
            case 5: malformed.rate = 0xffffffff; break;
            case 6: malformed.mappingBytes--; break;
            case 7: malformed.flags |= 1u << (1 + random() % 31); break;   // unknown flag
            case 8: malformed.flags = 0; break;                            // no FLOAT64
            case 9: malformed.extensionReserved[random() % 14] = 1; break;
            case 10: malformed.extensionBytes = static_cast<DWORD>(random()); break;
            case 11: malformed.minor = (random() & 1) ? 0 : 2; break;
            case 12: std::memcpy(malformed.busId, L"cable-z", 14); break;  // unknown cable
            case 13: std::memcpy(malformed.busId, L"cable-h", 14); break;  // likely not enabled
            case 14: bytes = kPrefixBytes; break;                          // prefix-only OPEN
            default: malformed.generation = 0; break;
            }
            invoke(device, kOpen, &malformed, bytes, stats);
            break;
        }
        case 8: { // QUERY: exact report, then hostile input/output lengths.
            std::array<unsigned char, 256> out{};
            DWORD returned = 0;
            ++stats.calls;
            if (DeviceIoControl(device, kQuery, nullptr, 0, out.data(), kDriverInfoBytes, &returned, nullptr)) {
                ++stats.accepted; ++stats.queried;
                WORD major = 0, minor = 0;
                std::memcpy(&major, out.data(), 2); std::memcpy(&minor, out.data() + 2, 2);
                if (returned != kDriverInfoBytes || major != 1 || minor != 1) {
                    ++stats.unexpected;
                    std::fprintf(stderr, "QUERY returned %lu bytes, protocol %u.%u\n", returned, major, minor);
                }
            } else if (expectedRejection(GetLastError())) { ++stats.rejected; } else { ++stats.unexpected; }
            const DWORD inBytes = static_cast<DWORD>(random() % 9);
            const DWORD outBytes = static_cast<DWORD>(random() % (out.size() + 1));
            ++stats.calls;
            if (DeviceIoControl(device, kQuery, out.data(), inBytes, out.data(), outBytes, &returned, nullptr)) {
                ++stats.accepted;
                if (inBytes != 0 || outBytes != kDriverInfoBytes) {
                    ++stats.unexpected;
                    std::fprintf(stderr, "QUERY accepted in=%lu out=%lu\n", inBytes, outBytes);
                }
            } else if (expectedRejection(GetLastError())) { ++stats.rejected; } else { ++stats.unexpected; }
            break;
        }
        case 9: { // Maintenance without the extension is valid for the owner.
            auto heartbeat = current;
            invoke(device, kHeartbeat, &heartbeat, kPrefixBytes, stats);
            break;
        }
        case 2: { // A kernel event handle is not a section object.
            HANDLE event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
            if (event) { auto bad = current; bad.sectionHandle = reinterpret_cast<ULONG_PTR>(event); invoke(device, kOpen, &bad, sizeof(bad), stats); CloseHandle(event); }
            break;
        }
        case 3: { // Neither a file handle nor an arbitrary integer is a section.
            auto bad = current; bad.sectionHandle = reinterpret_cast<ULONG_PTR>(device);
            invoke(device, kOpen, &bad, sizeof(bad), stats);
            bad.sectionHandle = static_cast<ULONG_PTR>(random());
            invoke(device, kOpen, &bad, sizeof(bad), stats);
            break;
        }
        case 4: { // Bounded random bytes and unknown IOCTLs.
            std::array<unsigned char, 256> data{};
            for (auto& byte : data) byte = static_cast<unsigned char>(random());
            invoke(device, codes[random() % std::size(codes)], data.data(), static_cast<DWORD>(random() % (data.size() + 1)), stats);
            break;
        }
        case 5: { // Same FILE_OBJECT: race a heartbeat against explicit CLOSE.
            auto heartbeat = current;
            std::thread racingClose([&] { invoke(device, kClose, &heartbeat, sizeof(heartbeat), stats); });
            invoke(device, kHeartbeat, &heartbeat, sizeof(heartbeat), stats);
            racingClose.join();
            generation += 2;
            current = makeRequest(direction, section.handle, generation);
            invoke(device, kOpen, &current, sizeof(current), stats);
            break;
        }
        case 6: { // Reusing one section for the other lease must conflict.
            auto duplicate = makeRequest(direction == 1 ? 2 : 1, section.handle, generation + 1);
            invoke(device, kOpen, &duplicate, sizeof(duplicate), stats);
            break;
        }
        default: {
            auto heartbeat = current;
            if (random() & 1) heartbeat.generation ^= 0x1000000000000000ULL;
            invoke(device, kHeartbeat, &heartbeat, sizeof(heartbeat), stats);
            if ((random() & 0x7ff) == 0) {
                generation += 2; current = makeRequest(direction, section.handle, generation);
                invoke(device, kOpen, &current, sizeof(current), stats);
            }
            break;
        }
        }
    }
    invoke(device, kClose, &current, sizeof(current), stats);
    CloseHandle(device);
}
} // namespace

int wmain(int argc, wchar_t** argv) {
    DWORD seconds = 1800;
    unsigned workers = 4;
    ULONGLONG seed = GetTickCount64() ^ (static_cast<ULONGLONG>(GetCurrentProcessId()) << 32);
    for (int i = 1; i < argc; ++i) {
        if (i + 1 >= argc) { std::fwprintf(stderr, L"Missing option value.\n"); return 2; }
        const ULONGLONG value = _wcstoui64(argv[++i], nullptr, 10);
        if (value == 0) { std::fwprintf(stderr, L"Options must be positive integers.\n"); return 2; }
        if (!wcscmp(argv[i - 1], L"--seconds") && value <= 86400) seconds = static_cast<DWORD>(value);
        else if (!wcscmp(argv[i - 1], L"--workers") && value <= 16) workers = static_cast<unsigned>(value);
        else if (!wcscmp(argv[i - 1], L"--seed")) seed = value;
        else { std::fwprintf(stderr, L"Usage: m03-bridge-fuzz.exe [--seconds 1800] [--seed N] [--workers 4]\n"); return 2; }
    }
    std::wprintf(L"Bridge IOCTL fuzz seconds=%lu workers=%u seed=%llu. Run only in the test VM with Driver Verifier enabled.\n", seconds, workers, seed);
    Stats stats;
    std::vector<std::thread> threads;
    for (unsigned i = 0; i < workers; ++i) threads.emplace_back(runWorker, seconds, seed, i, std::ref(stats));
    const ULONGLONG end = GetTickCount64() + static_cast<ULONGLONG>(seconds) * 1000;
    while (GetTickCount64() < end) {
        std::this_thread::sleep_for(std::chrono::seconds(5));
        std::wprintf(L"calls=%llu accepted=%llu rejected=%llu unexpected=%llu\n", stats.calls.load(), stats.accepted.load(), stats.rejected.load(), stats.unexpected.load());
    }
    for (auto& thread : threads) thread.join();
    std::wprintf(L"complete calls=%llu accepted=%llu rejected=%llu unexpected=%llu opened=%llu queried=%llu\n", stats.calls.load(), stats.accepted.load(), stats.rejected.load(), stats.unexpected.load(), stats.opened.load(), stats.queried.load());
    // Coverage guard: a run in which no OPEN or QUERY ever succeeds only
    // exercised rejection paths (for example after an ABI change) and must
    // not count as a pass for the lease/mapping/teardown code.
    if (stats.opened.load() == 0 || stats.queried.load() == 0) {
        std::fwprintf(stderr, L"no successful OPEN or QUERY: the lease paths were not exercised\n");
        return 1;
    }
    return stats.unexpected.load() == 0 ? 0 : 1;
}
