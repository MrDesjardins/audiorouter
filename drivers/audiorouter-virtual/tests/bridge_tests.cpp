#define AR_BRIDGE_UNIT_TEST
#include "../Source/Inc/bridgeio.h"
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <limits>
#include <vector>

extern "C" __declspec(dllimport) void* __stdcall VirtualAlloc(void*, SIZE_T, unsigned long, unsigned long);
extern "C" __declspec(dllimport) int __stdcall VirtualFree(void*, SIZE_T, unsigned long);

static bool mutateHeader = false;
void AudioRouterTestAfterHeaderSnapshot(const UCHAR* view) {
    if (mutateHeader) {
        auto header = reinterpret_cast<AR_BRIDGE_BLOCK_HEADER*>(const_cast<UCHAR*>(view) + AR_BRIDGE_HEADER_OFFSET);
        header->Frames = 65535;
        header->Channels = 65535;
        header->PayloadBytes = 0xffffffff;
        header->Generation = 999;
    }
}
static unsigned checks = 0;
static void require(bool success, const char* name) {
    if (!success) { std::fprintf(stderr, "FAILED: %s\n", name); std::exit(1); }
    ++checks;
}
static AR_BRIDGE_OPEN_REQUEST validRequest() {
    AR_BRIDGE_OPEN_REQUEST request = {};
    request.ProtocolMajor = AR_BRIDGE_PROTOCOL_MAJOR;
    request.ProtocolMinor = AR_BRIDGE_PROTOCOL_MINOR;
    request.BusIdBytes = 14;
    std::memcpy(request.BusId, L"cable-a", 14);
    request.Channels = 2; request.FramesPerQuantum = 128;
    request.Direction = AR_BRIDGE_DIRECTION_RENDER_SOURCE;
    request.SampleRateHz = 48000; request.LeaseMs = 2000;
    request.Generation = 1; request.SectionHandle = 123;
    request.MappingBytes = AR_BRIDGE_HEADER_BYTES + 128 * 2 * sizeof(FLOAT);
    return request;
}
int main() {
    require(!AudioRouterStreamGenerationChanged(17, 17), "same lease generation retains scratch");
    require(AudioRouterStreamGenerationChanged(17, 18), "same-shape lease replacement resets scratch");
    require(AudioRouterStreamGenerationChanged(17, 0), "lease retirement resets scratch");
    require(NT_SUCCESS(AudioRouterValidateNextGeneration(0, 17)), "first nonzero generation accepted");
    require(NT_SUCCESS(AudioRouterValidateNextGeneration(17, 18)), "unique reopen generation accepted");
    require(!NT_SUCCESS(AudioRouterValidateNextGeneration(17, 17)), "reused reopen generation rejected");
    require(!NT_SUCCESS(AudioRouterValidateNextGeneration(18, 17)), "non-adjacent reused generation rejected");
    require(!AudioRouterStreamGenerationChanged(17, 17), "same lease generation retains scratch");
    require(AudioRouterStreamGenerationChanged(17, 18), "same-shape lease replacement resets scratch");
    require(AudioRouterStreamGenerationChanged(17, 0), "lease retirement resets scratch");
    require(!AudioRouterStreamGenerationChanged(17, 17), "same lease generation retains scratch");
    require(AudioRouterStreamGenerationChanged(17, 18), "same-shape lease replacement resets scratch");
    require(AudioRouterStreamGenerationChanged(17, 0), "lease retirement resets scratch");
    auto request = validRequest();
    require(NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&request)), "valid OPEN");
    require(NT_SUCCESS(AudioRouterValidateMappingBytes(&request)), "exact logical mapping");
    auto shortMapping = request; shortMapping.MappingBytes--;
    require(!NT_SUCCESS(AudioRouterValidateMappingBytes(&shortMapping)), "short logical mapping");
    shortMapping.MappingBytes += 2;
    require(!NT_SUCCESS(AudioRouterValidateMappingBytes(&shortMapping)), "oversized logical mapping");
    require(AudioRouterOpenOwnershipStatus(true, 1, 1) == STATUS_SHARING_VIOLATION, "same-session held lease conflict");
    require(AudioRouterOpenOwnershipStatus(true, 1, 2) == STATUS_ACCESS_DENIED, "different-session held lease denial");
    require(NT_SUCCESS(AudioRouterOpenOwnershipStatus(false, 1, 2)), "released lease can transfer");
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(nullptr)), "null OPEN");
    auto bad = request; bad.BusId[2] = 0;
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "embedded NUL");
    bad = request; bad.BusId[7] = L'x';
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "nonzero trailing bus data");
    bad = request; bad.Reserved2 = 1;
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "reserved bits");
    bad = request; bad.ProtocolMajor++;
    require(AudioRouterValidateBridgeOpenRequest(&bad) == STATUS_REVISION_MISMATCH, "major mismatch");
    bad = request; bad.ProtocolMinor++;
    require(AudioRouterValidateBridgeOpenRequest(&bad) == STATUS_REVISION_MISMATCH, "minor mismatch");
    bad = request; bad.Channels = AR_BRIDGE_MAX_CHANNELS + 1;
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "channel bound");
    bad = request; bad.FramesPerQuantum = AR_BRIDGE_MAX_FRAMES + 1;
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "frame bound");
    bad = request; bad.SectionHandle = 0;
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "incomplete mapping pair");
    std::vector<ULONGLONG> aligned((AR_BRIDGE_PAYLOAD_OFFSET + AR_BRIDGE_MAX_PAYLOAD_BYTES + 7) / 8);
    auto view = reinterpret_cast<UCHAR*>(aligned.data());
    auto sharedHeader = reinterpret_cast<AR_BRIDGE_BLOCK_HEADER*>(view + AR_BRIDGE_HEADER_OFFSET);
    auto samples = reinterpret_cast<FLOAT*>(view + AR_BRIDGE_PAYLOAD_OFFSET);
    *sharedHeader = { 1, 2, 2, 2, 16 };
    samples[0] = -0.0f; samples[1] = std::numeric_limits<FLOAT>::denorm_min(); samples[2] = 0.5f; samples[3] = -0.75f;
    FLOAT destination[5] = { 7, 7, 7, 7, 123 };
    AR_BRIDGE_BLOCK_HEADER copied = {};
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, 48, 1, 1, destination, 4, &copied)), "valid copy");
    require(std::memcmp(destination, samples, 16) == 0 && destination[4] == 123, "bit exact and canary");
    require(copied.Frames == 2 && copied.Generation == 1, "returned snapshot");
    mutateHeader = true;
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, 48, 1, 1, destination, 4, &copied)), "header mutation uses local snapshot");
    require(copied.Frames == 2 && copied.Generation == 1 && destination[4] == 123, "hostile dimensions cannot resize copy");
    mutateHeader = false;
    // Two-page allocation with an inaccessible following page. The valid
    // header/payload end exactly at the boundary; hostile enlarged dimensions
    // would fault the old double-fetch copy, independently of sanitizer support.
    auto guarded = static_cast<UCHAR*>(VirtualAlloc(nullptr, 8192, 0x2000, 0x01));
    require(guarded != nullptr && VirtualAlloc(guarded, 4096, 0x1000, 0x04) != nullptr, "guard-page allocation");
    UCHAR* boundedView = guarded + 4096 - 48;
    *reinterpret_cast<AR_BRIDGE_BLOCK_HEADER*>(boundedView + AR_BRIDGE_HEADER_OFFSET) = { 1, 2, 2, 2, 16 };
    std::memcpy(boundedView + AR_BRIDGE_PAYLOAD_OFFSET, samples, 16);
    mutateHeader = true;
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(boundedView, 48, 1, 1, destination, 4, &copied)), "mutation at inaccessible page boundary");
    require(destination[4] == 123 && copied.Frames == 2, "guarded snapshot and destination bound");
    mutateHeader = false;
    require(VirtualFree(guarded, 0, 0x8000) != 0, "guard-page release");
    *sharedHeader = { 1, 2, 2, 2, 16 };
    require(AudioRouterCopyBridgeBlock(view, 47, 1, 1, destination, 4, &copied) == STATUS_BUFFER_TOO_SMALL, "short view");
    require(AudioRouterCopyBridgeBlock(view, 48, 1, 1, destination, 3, &copied) == STATUS_BUFFER_TOO_SMALL, "short destination");
    require(!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, 48, 1, 2, destination, 4, &copied)), "repeated sequence");
    sharedHeader->PayloadBytes = 15;
    require(!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, 48, 1, 1, destination, 4, &copied)), "payload mismatch");
    sharedHeader->PayloadBytes = 16;
    for (FLOAT nonfinite : { std::numeric_limits<FLOAT>::quiet_NaN(), std::numeric_limits<FLOAT>::infinity(), -std::numeric_limits<FLOAT>::infinity() }) {
        samples[3] = nonfinite;
        require(AudioRouterCopyBridgeBlock(view, 48, 1, 1, destination, 4, &copied) == STATUS_DATA_ERROR, "nonfinite rejection");
        require(destination[0] == 0 && destination[1] == 0 && destination[2] == 0 && destination[3] == 0 && destination[4] == 123, "failed quantum scrubbed");
    }
    *sharedHeader = { 1, 2, AR_BRIDGE_MAX_FRAMES, AR_BRIDGE_MAX_CHANNELS, AR_BRIDGE_MAX_PAYLOAD_BYTES };
    std::vector<FLOAT> full(AR_BRIDGE_MAX_FRAMES * AR_BRIDGE_MAX_CHANNELS, 0.25f);
    std::memcpy(samples, full.data(), AR_BRIDGE_MAX_PAYLOAD_BYTES);
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, aligned.size() * 8, 1, 1, full.data(), full.size(), &copied)), "maximum shape");
    *sharedHeader = { 1, 2, 128, 2, 128 * 2 * sizeof(FLOAT) };
    constexpr unsigned iterations = 200000;
    auto start = std::chrono::steady_clock::now();
    for (unsigned i = 0; i < iterations; ++i) {
        if (!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, aligned.size() * 8, 1, 1, full.data(), full.size(), &copied))) return 2;
    }
    auto elapsed = std::chrono::duration<double, std::micro>(std::chrono::steady_clock::now() - start).count();
    std::printf("bridge checks passed: %u; host copy 128x2 float32: %.3f us/block (%u iterations); not kernel DPC evidence\n", checks, elapsed / iterations, iterations);
}
