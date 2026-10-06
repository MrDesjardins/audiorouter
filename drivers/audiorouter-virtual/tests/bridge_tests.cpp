#define AR_BRIDGE_UNIT_TEST
#include "../Source/Inc/bridgeio.h"
#include "../Source/Inc/sampleconv.h"
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
    request.MappingBytes = AR_BRIDGE_HEADER_BYTES + 128 * 2 * sizeof(DOUBLE);
    return request;
}
int main() {
    bool occupiedLeaseSlots[AR_BRIDGE_LEASE_SLOTS] = {};
    for (USHORT bus = 0; bus < AR_BRIDGE_MAX_CABLES; ++bus) {
        WCHAR busId[] = L"cable-a";
        busId[6] = static_cast<WCHAR>(L'a' + bus);
        USHORT parsedBus = 0xffff;
        require(NT_SUCCESS(AudioRouterParseCableBusId(
            busId, sizeof(busId) - sizeof(WCHAR), &parsedBus)) && parsedBus == bus,
            "each canonical cable bus id maps to its stable bus index");
        const USHORT directions[] = { AR_BRIDGE_DIRECTION_RENDER_SOURCE,
                                       AR_BRIDGE_DIRECTION_CAPTURE_SINK };
        for (USHORT direction : directions) {
            ULONG slot = AR_BRIDGE_LEASE_SLOTS;
            require(NT_SUCCESS(AudioRouterGetLeaseSlotIndex(bus, direction, &slot)) &&
                slot < AR_BRIDGE_LEASE_SLOTS && !occupiedLeaseSlots[slot],
                "each cable direction maps to a unique bounded lease slot");
            occupiedLeaseSlots[slot] = true;
        }
    }
    WCHAR unknownBus[] = L"cable-i";
    USHORT ignoredBus = 0;
    require(AudioRouterParseCableBusId(unknownBus, sizeof(unknownBus) - sizeof(WCHAR),
        &ignoredBus) == STATUS_OBJECT_NAME_NOT_FOUND, "unknown cable id rejected");
    ULONG ignoredSlot = 0;
    require(!NT_SUCCESS(AudioRouterGetLeaseSlotIndex(AR_BRIDGE_MAX_CABLES,
        AR_BRIDGE_DIRECTION_RENDER_SOURCE, &ignoredSlot)), "out-of-range bus rejected");

    const ULONG cableRates[] = { 44100, 48000, 96000 };
    const USHORT cableChannels[] = { 1, 2, 4, 6, 8 };
    unsigned supportedFormats = 0;
    for (ULONG rate : cableRates) {
        for (USHORT channels : cableChannels) {
            require(AudioRouterCableFormatSupported(rate, channels, AR_CABLE_FORMAT_PCM, 16, 16),
                "advertised PCM16 format metadata");
            require(AudioRouterCableFormatSupported(rate, channels, AR_CABLE_FORMAT_PCM, 32, 24),
                "advertised PCM24-in-32 format metadata");
            require(AudioRouterCableFormatSupported(rate, channels, AR_CABLE_FORMAT_PCM, 32, 32),
                "advertised PCM32 format metadata");
            require(AudioRouterCableFormatSupported(rate, channels, AR_CABLE_FORMAT_FLOAT32, 32, 32),
                "advertised float32 format metadata");
            supportedFormats += 4;
        }
    }
    require(supportedFormats == 60, "60 required rate/channel/format combinations");
    require(!AudioRouterCableFormatSupported(88200, 2, AR_CABLE_FORMAT_PCM, 16, 16),
        "unsupported sample rate rejected");
    require(!AudioRouterCableFormatSupported(48000, 3, AR_CABLE_FORMAT_PCM, 16, 16),
        "unsupported channel layout rejected");
    require(!AudioRouterCableFormatSupported(48000, 8, AR_CABLE_FORMAT_PCM, 24, 24),
        "packed PCM24 rejected; contract requires 24-in-32");
    require(!AudioRouterCableFormatSupported(48000, 2, AR_CABLE_FORMAT_FLOAT32, 32, 24),
        "float32 valid bits bounded");
    for (FLOAT source : { -0.0F, 1.25F, -0.375F, std::numeric_limits<FLOAT>::denorm_min() }) {
        FLOAT roundTrip = AudioRouterDoubleToFloat32(AudioRouterFloat32ToDouble(source));
        require(std::memcmp(&source, &roundTrip, sizeof(source)) == 0,
            "finite float32 bridge conversion is bit-exact without unit clamping");
    }
    require(AudioRouterFloat32ToDouble(std::numeric_limits<FLOAT>::quiet_NaN()) == 0.0,
        "float32 NaN is silenced");
    require(AudioRouterFloat32ToDouble(std::numeric_limits<FLOAT>::infinity()) == 0.0,
        "float32 infinity is silenced");
    require(AudioRouterDoubleToFloat32(std::numeric_limits<DOUBLE>::infinity()) == 0.0F,
        "out-of-range bridge value is silenced before float32 output");
    require(!AudioRouterStreamGenerationChanged(17, 17), "same lease generation retains scratch");
    require(AudioRouterStreamGenerationChanged(17, 18), "same-shape lease replacement resets scratch");
    require(AudioRouterStreamGenerationChanged(17, 0), "lease retirement resets scratch");
    require(NT_SUCCESS(AudioRouterValidateNextGeneration(0, 17)), "first nonzero generation accepted");
    require(NT_SUCCESS(AudioRouterValidateNextGeneration(17, 18)), "unique reopen generation accepted");
    require(!NT_SUCCESS(AudioRouterValidateNextGeneration(17, 17)), "reused reopen generation rejected");
    require(!NT_SUCCESS(AudioRouterValidateNextGeneration(18, 17)), "non-adjacent reused generation rejected");
    constexpr LONG pcm32RoundTripInput = 1073741889;
    require(AudioRouterDoubleToPcm32(AudioRouterPcm32ToDouble(pcm32RoundTripInput)) == pcm32RoundTripInput,
        "PCM32 sample 1073741889 round-trips exactly through double");
    require(AudioRouterPcm32ToDouble(-2147483647L - 1L) == -1.0, "PCM32 negative full scale");
    require(AudioRouterDoubleToPcm32(1.0) == 2147483647L, "PCM32 positive clamp");
    require(AudioRouterDoubleToPcm24In32(AudioRouterPcm24In32ToDouble(-2147483647L - 1L)) == -2147483647L - 1L,
        "PCM24-in-32 negative full scale round-trip");
    require(AudioRouterDoubleToPcm16(AudioRouterPcm16ToDouble(-32768)) == -32768,
        "PCM16 negative full scale round-trip");
    require(AudioRouterDoubleToPcm16(1.0) == 32767 && AudioRouterDoubleToPcm16(-1.0) == -32768,
        "PCM16 positive and negative clamp");
    require(AudioRouterDoubleToPcm16(1.0 / 65536.0) == 1 &&
        AudioRouterDoubleToPcm16(-1.0 / 65536.0) == -1,
        "PCM16 half-LSB rounds away from zero");
    require(AudioRouterDoubleToPcm24In32(1.0) == 2147483392L &&
        AudioRouterDoubleToPcm24In32(-1.0) == (-2147483647L - 1L),
        "PCM24-in-32 positive and negative clamp");
    require(AudioRouterDoubleToPcm24In32(1.0 / 16777216.0) == 256 &&
        AudioRouterDoubleToPcm24In32(-1.0 / 16777216.0) == -256,
        "PCM24-in-32 half-LSB rounds away from zero");
    require(AudioRouterDoubleToPcm32(0.5 / 2147483648.0) == 1 &&
        AudioRouterDoubleToPcm32(-0.5 / 2147483648.0) == -1,
        "PCM32 half-LSB rounds away from zero");
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
    bad = request; bad.ProtocolMinor = 0;
    require(AudioRouterValidateBridgeOpenRequest(&bad) == STATUS_REVISION_MISMATCH,
        "legacy float32 mapping rejected by precision ABI");
    bad = request; bad.ProtocolMinor++;
    require(AudioRouterValidateBridgeOpenRequest(&bad) == STATUS_REVISION_MISMATCH, "future minor mismatch");
    bad = request; bad.Channels = AR_BRIDGE_MAX_CHANNELS + 1;
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "channel bound");
    bad = request; bad.FramesPerQuantum = AR_BRIDGE_MAX_FRAMES + 1;
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "frame bound");
    bad = request; bad.SectionHandle = 0;
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenRequest(&bad)), "incomplete mapping pair");
    std::vector<ULONGLONG> aligned((AR_BRIDGE_PAYLOAD_OFFSET + AR_BRIDGE_MAX_PAYLOAD_BYTES + 7) / 8);
    auto view = reinterpret_cast<UCHAR*>(aligned.data());
    auto sharedHeader = reinterpret_cast<AR_BRIDGE_BLOCK_HEADER*>(view + AR_BRIDGE_HEADER_OFFSET);
    auto samples = reinterpret_cast<DOUBLE*>(view + AR_BRIDGE_PAYLOAD_OFFSET);
    *sharedHeader = { 1, 2, 2, 2, 32 };
    samples[0] = -0.0; samples[1] = std::numeric_limits<DOUBLE>::denorm_min(); samples[2] = 0.5; samples[3] = -0.75;
    DOUBLE destination[5] = { 7, 7, 7, 7, 123 };
    AR_BRIDGE_BLOCK_HEADER copied = {};
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, 64, 1, 1, destination, 4, &copied)), "valid copy");
    require(std::memcmp(destination, samples, 32) == 0 && destination[4] == 123, "bit exact and canary");
    require(copied.Frames == 2 && copied.Generation == 1, "returned snapshot");
    mutateHeader = true;
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, 64, 1, 1, destination, 4, &copied)), "header mutation uses local snapshot");
    require(copied.Frames == 2 && copied.Generation == 1 && destination[4] == 123, "hostile dimensions cannot resize copy");
    mutateHeader = false;
    // Two-page allocation with an inaccessible following page. The valid
    // header/payload end exactly at the boundary; hostile enlarged dimensions
    // would fault the old double-fetch copy, independently of sanitizer support.
    auto guarded = static_cast<UCHAR*>(VirtualAlloc(nullptr, 8192, 0x2000, 0x01));
    require(guarded != nullptr && VirtualAlloc(guarded, 4096, 0x1000, 0x04) != nullptr, "guard-page allocation");
    UCHAR* boundedView = guarded + 4096 - 64;
    *reinterpret_cast<AR_BRIDGE_BLOCK_HEADER*>(boundedView + AR_BRIDGE_HEADER_OFFSET) = { 1, 2, 2, 2, 32 };
    std::memcpy(boundedView + AR_BRIDGE_PAYLOAD_OFFSET, samples, 32);
    mutateHeader = true;
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(boundedView, 64, 1, 1, destination, 4, &copied)), "mutation at inaccessible page boundary");
    require(destination[4] == 123 && copied.Frames == 2, "guarded snapshot and destination bound");
    mutateHeader = false;
    require(VirtualFree(guarded, 0, 0x8000) != 0, "guard-page release");
    *sharedHeader = { 1, 2, 2, 2, 32 };
    require(AudioRouterCopyBridgeBlock(view, 63, 1, 1, destination, 4, &copied) == STATUS_BUFFER_TOO_SMALL, "short view");
    require(AudioRouterCopyBridgeBlock(view, 64, 1, 1, destination, 3, &copied) == STATUS_BUFFER_TOO_SMALL, "short destination");
    require(!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, 64, 1, 2, destination, 4, &copied)), "repeated sequence");
    sharedHeader->PayloadBytes = 31;
    require(!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, 64, 1, 1, destination, 4, &copied)), "payload mismatch");
    sharedHeader->PayloadBytes = 32;
    for (DOUBLE nonfinite : { std::numeric_limits<DOUBLE>::quiet_NaN(), std::numeric_limits<DOUBLE>::infinity(), -std::numeric_limits<DOUBLE>::infinity() }) {
        samples[3] = nonfinite;
        require(AudioRouterCopyBridgeBlock(view, 64, 1, 1, destination, 4, &copied) == STATUS_DATA_ERROR, "nonfinite rejection");
        require(destination[0] == 0 && destination[1] == 0 && destination[2] == 0 && destination[3] == 0 && destination[4] == 123, "failed quantum scrubbed");
    }
    *sharedHeader = { 1, 2, AR_BRIDGE_MAX_FRAMES, AR_BRIDGE_MAX_CHANNELS, AR_BRIDGE_MAX_PAYLOAD_BYTES };
    std::vector<DOUBLE> full(AR_BRIDGE_MAX_FRAMES * AR_BRIDGE_MAX_CHANNELS, 0.25);
    std::memcpy(samples, full.data(), AR_BRIDGE_MAX_PAYLOAD_BYTES);
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, aligned.size() * 8, 1, 1, full.data(), full.size(), &copied)), "maximum shape");
    *sharedHeader = { 1, 2, 128, 2, 128 * 2 * sizeof(DOUBLE) };
    constexpr unsigned iterations = 200000;
    auto start = std::chrono::steady_clock::now();
    for (unsigned i = 0; i < iterations; ++i) {
        if (!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, aligned.size() * 8, 1, 1, full.data(), full.size(), &copied))) return 2;
    }
    auto elapsed = std::chrono::duration<double, std::micro>(std::chrono::steady_clock::now() - start).count();
    std::printf("bridge checks passed: %u; host copy 128x2 float64: %.3f us/block (%u iterations); not kernel DPC evidence\n", checks, elapsed / iterations, iterations);
}
