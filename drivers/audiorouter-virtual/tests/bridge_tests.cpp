#define AR_BRIDGE_UNIT_TEST
#include "../Source/Inc/bridgeio.h"
#include "../Source/Inc/sampleconv.h"
#include "../Source/Inc/capturequeue.h"
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
// Offline model: the producer refills one tick after an acknowledgement.
// A notification arriving 1 ms late must not exhaust a prefetched queue.
// It uses the same queue methods as WaveRT; it is not kernel timing evidence.
static ULONG captureTimerUnderruns(bool serviceEveryTick) {
    AudioRouterCaptureQueue queue = {};
    queue.Reset();
    ULONG lastTick = 0;
    ULONG publishAt = 0;
    ULONGLONG published = 1;
    bool ready = true;
    ULONG underruns = 0;
    for (ULONG tick = 1; tick <= 100; ++tick) {
        if (!ready && tick >= publishAt) { ++published; ready = true; }
        const bool notification = tick == 10 || tick == 21 ||
            (tick >= 30 && tick % 10 == 0);
        if (!serviceEveryTick && !notification) { continue; }
        auto prefetch = [&]() {
            queue.Prefetch([&](ULONG, ULONGLONG) {
                if (!ready) { return false; }
                ready = false;
                publishAt = tick + 1;
                return queue.Commit(480, published);
            });
        };
        prefetch();
        ULONG due = (tick - lastTick) * 48;
        lastTick = tick;
        while (due != 0 && queue.Available() != 0) {
            const ULONG take = due < queue.Available() ? due : queue.Available();
            queue.Consume(take);
            due -= take;
            prefetch();
        }
        underruns += due;
    }
    return underruns;
}
int main() {
    const ULONG clockRates[] = { 44100, 48000, 96000 };
    const USHORT frameSizes[] = { 2, 4, 8, 12, 16, 24, 32 };
    for (ULONG rate : clockRates) {
        for (USHORT frameBytes : frameSizes) {
            if (frameBytes == 0) { std::exit(1); }
            ULONGLONG carry = 0;
            ULONGLONG total = 0;
            bool aligned = true;
            for (ULONG tick = 0; tick < 1000; ++tick) {
                const ULONGLONG numerator = static_cast<ULONGLONG>(rate) * frameBytes + carry;
                const ULONGLONG bytes = AudioRouterFrameAlignedByteCount(numerator, frameBytes);
                aligned = aligned && bytes % frameBytes == 0;
                total += bytes;
                carry = numerator % (1000ULL * frameBytes);
            }
            require(aligned && total == static_cast<ULONGLONG>(rate) * frameBytes && carry == 0,
                "one-millisecond DMA updates preserve full frames and exact one-second sample count");
        }
    }
    require(AudioRouterFrameAlignedByteCount(1234, 0) == 0, "zero frame size fails closed");
    require(captureTimerUnderruns(false) != 0,
        "notification-only capture service reproduces jitter starvation");
    require(captureTimerUnderruns(true) == 0,
        "per-tick capture service builds a reserve without counted silence");
    AudioRouterCaptureQueue queue = {};
    queue.Reset();
    require(queue.Count == 0 && queue.Available() == 0, "capture starts empty");
    require(queue.Commit(480, 1), "capture commits current block");
    require(queue.Consume(128) && queue.Available() == 352, "partial read retains samples");
    require(queue.Commit(480, 2), "prefetch while current block has unread samples");
    require(queue.Count == 2 && queue.Head == 0 && queue.Offset == 128,
        "publication does not replace unread current block");
    require(!queue.Commit(480, 3), "capture memory bounded to two blocks");
    require(queue.Consume(352) && queue.Head == 1 && queue.Available() == 480,
        "FIFO promotion preserves whole prefetched block");
    require(!queue.Commit(480, 2) && !queue.Commit(480, 1), "duplicates and old blocks refused");
    require(queue.Commit(128, 3) && queue.Tail() == 1, "freed slot reused without shifting samples");
    require(!queue.Consume(481) && queue.Available() == 480, "oversized drain leaves queue intact");
    require(queue.Consume(480) && queue.Consume(128) && queue.Count == 0,
        "batched callback drains both blocks exactly once");
    require(!queue.Consume(1) && queue.Available() == 0, "missing block requires counted silence");
    require(!queue.Commit(0, 4) && !queue.Commit(AR_BRIDGE_MAX_FRAMES + 1, 4),
        "invalid frame lengths cannot become valid audio");
    require(queue.Commit(480, 4) && queue.Consume(48), "partial block before boundary");
    queue.Clear();
    require(queue.Available() == 0 && queue.Count == 0 && !queue.Commit(480, 4),
        "stop or unusable format discards audio without replaying acknowledged block");
    queue.Reset();
    require(queue.Commit(128, 1) && queue.Offset == 0,
        "generation turnover permits new sequence without old samples");
    // A busy publisher performs no Commit: good queued samples remain valid.
    require(queue.Consume(64) && queue.Available() == 64 && queue.Sequence == 1,
        "busy publication preserves partial queued audio");
    queue.Reset();
    require(queue.Count == 0 && queue.Frames[0] == 0 && queue.Frames[1] == 0,
        "teardown invalidates both buffers with bounded metadata writes");
    DOUBLE queueSamples[2][4] = {};
    ULONG fetches = 0;
    queue.Prefetch([&](ULONG slot, ULONGLONG sequence) {
        ++fetches;
        for (ULONG frame = 0; frame < 4; ++frame) {
            queueSamples[slot][frame] = static_cast<DOUBLE>(sequence * 4 + frame);
        }
        return queue.Commit(4, sequence + 1);
    });
    require(fetches == 2 && queue.Count == 2, "prefetch copies at most two blocks");
    queue.Prefetch([&](ULONG, ULONGLONG) { ++fetches; return false; });
    require(fetches == 2, "full queue does not read or overwrite shared slot");
    for (ULONG frame = 0; frame < 8; ++frame) {
        require(queueSamples[queue.Head][queue.Offset] == static_cast<DOUBLE>(frame),
            "prefetched samples drain in order without duplication");
        require(queue.Consume(1), "sample frame consumed once");
        // Simulate publication in progress after the first block is freed.
        queue.Prefetch([&](ULONG, ULONGLONG) { ++fetches; return false; });
    }
    require(queue.Available() == 0 && fetches == 7,
        "busy publication makes one attempt per call and leaves no stale samples");
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
    // Protocol 1.1 OPEN extension, request lengths and QUERY (17 §5.2).
    require(AudioRouterValidateBridgeRequestLength(240, 0, true) == STATUS_SUCCESS,
        "OPEN with extension has the exact accepted length");
    require(AudioRouterValidateBridgeRequestLength(176, 0, true) == STATUS_NOT_SUPPORTED,
        "prefix-only OPEN cannot negotiate float64 and is refused distinctly");
    require(AudioRouterValidateBridgeRequestLength(176, 0, false) == STATUS_SUCCESS,
        "maintenance request may omit the extension");
    for (ULONG length : { 0u, 175u, 177u, 239u, 241u, 4096u }) {
        require(AudioRouterValidateBridgeRequestLength(length, 0, true) == STATUS_INVALID_PARAMETER &&
            AudioRouterValidateBridgeRequestLength(length, 0, false) == STATUS_INVALID_PARAMETER,
            "inexact bridge request length rejected before field access");
    }
    require(AudioRouterValidateBridgeRequestLength(240, 8, true) == STATUS_INVALID_PARAMETER,
        "OPEN output buffer rejected");
    AR_BRIDGE_OPEN_EXTENSION extension = {};
    extension.ExtensionBytes = AR_BRIDGE_OPEN_EXTENSION_BYTES;
    extension.Flags = AR_BRIDGE_OPEN_FLAG_FLOAT64;
    require(NT_SUCCESS(AudioRouterValidateBridgeOpenExtension(&extension)), "FLOAT64 extension accepted");
    auto badExtension = extension; badExtension.Flags = 0;
    require(AudioRouterValidateBridgeOpenExtension(&badExtension) == STATUS_NOT_SUPPORTED,
        "extension without FLOAT64 refused: no silent float32 transport");
    badExtension = extension; badExtension.Flags |= 0x2;
    require(AudioRouterValidateBridgeOpenExtension(&badExtension) == STATUS_NOT_SUPPORTED,
        "unknown extension flag refused, never ignored");
    for (ULONG word = 0; word < ARRAYSIZE(extension.Reserved); ++word) {
        badExtension = extension; badExtension.Reserved[word] = 1;
        require(AudioRouterValidateBridgeOpenExtension(&badExtension) == STATUS_NOT_SUPPORTED,
            "nonzero reserved extension word refused");
    }
    badExtension = extension; badExtension.ExtensionBytes = 60;
    require(AudioRouterValidateBridgeOpenExtension(&badExtension) == STATUS_INVALID_PARAMETER,
        "extension size field must match");
    require(!NT_SUCCESS(AudioRouterValidateBridgeOpenExtension(nullptr)), "null extension");
    require(NT_SUCCESS(AudioRouterValidateBridgeQueryLength(0, sizeof(AR_BRIDGE_DRIVER_INFO))),
        "QUERY exact output length");
    require(!NT_SUCCESS(AudioRouterValidateBridgeQueryLength(4, sizeof(AR_BRIDGE_DRIVER_INFO))) &&
        !NT_SUCCESS(AudioRouterValidateBridgeQueryLength(0, sizeof(AR_BRIDGE_DRIVER_INFO) - 1)) &&
        !NT_SUCCESS(AudioRouterValidateBridgeQueryLength(0, sizeof(AR_BRIDGE_DRIVER_INFO) + 1)),
        "QUERY inexact lengths rejected");
    // Registry configuration (17 §5.5): defaults, ranges, normalization.
    AR_BRIDGE_CONFIG config;
    AudioRouterDefaultConfig(&config);
    require(config.CableCount == 2 && config.MinPeriodFrames == 128 &&
        config.DefaultPeriodFrames == 480 && config.MaxLeaseMs == 60000,
        "compiled configuration defaults");
    require(AudioRouterConfigValue(false, 300, 64, 480, 128) == 128, "missing value uses default");
    require(AudioRouterConfigValue(true, 300, 64, 480, 128) == 300, "in-range value used");
    require(AudioRouterConfigValue(true, 64, 64, 480, 128) == 64 &&
        AudioRouterConfigValue(true, 480, 64, 480, 128) == 480, "range bounds are inclusive");
    require(AudioRouterConfigValue(true, 63, 64, 480, 128) == 128 &&
        AudioRouterConfigValue(true, 481, 64, 480, 128) == 128 &&
        AudioRouterConfigValue(true, 0xffffffff, 64, 480, 128) == 128,
        "out-of-range value falls back to the default");
    AR_BRIDGE_CONFIG inverted = config;
    inverted.MinPeriodFrames = 400; inverted.DefaultPeriodFrames = 128;
    AudioRouterNormalizeConfig(&inverted);
    require(inverted.DefaultPeriodFrames == 400, "default period never below the minimum");
    require(AudioRouterMinPacketPeriodHns(128) == 26666 && AudioRouterMinPacketPeriodHns(480) == 100000 &&
        AudioRouterMinPacketPeriodHns(64) == 13333, "packet period in 100 ns units at 48 kHz");
    require(AudioRouterLeaseWithinConfig(2000, &config) && AudioRouterLeaseWithinConfig(60000, &config),
        "lease within the cap accepted");
    AR_BRIDGE_CONFIG tightLease = config; tightLease.MaxLeaseMs = 500;
    require(!AudioRouterLeaseWithinConfig(501, &tightLease) && AudioRouterLeaseWithinConfig(500, &tightLease) &&
        !AudioRouterLeaseWithinConfig(0, &tightLease), "configured lease cap enforced");

    AR_BRIDGE_DRIVER_INFO info;
    std::memset(&info, 0xcd, sizeof(info));
    AR_BRIDGE_CONFIG reported = config; reported.CableCount = 3; reported.MinPeriodFrames = 256;
    AudioRouterFillDriverInfo(&info, &reported);
    require(info.ProtocolMajor == 1 && info.ProtocolMinor == 1 && info.CableCount == 3 &&
        info.MaxCables == 8 && info.MaxChannels == 8, "QUERY reports protocol and limits");
    require((info.Capabilities & AR_BRIDGE_CAP_SAMPLE_FLOAT64) &&
        (info.Capabilities & AR_BRIDGE_CAP_STREAM_COUNTERS) &&
        (info.Capabilities & AR_BRIDGE_CAP_LOW_LATENCY_PERIODS) &&
        (info.Capabilities & AR_BRIDGE_CAP_CONFIG_FROM_REGISTRY),
        "QUERY reports the implemented capabilities");
    require(info.SupportedRates == 7 && info.MinPeriodFrames == 256 && info.DefaultPeriodFrames == 480,
        "QUERY rates and configured period limits");
    bool reservedClear = true;
    for (ULONG word : info.Reserved) { reservedClear = reservedClear && word == 0; }
    require(reservedClear, "QUERY never leaks stale reserved bytes");
    require(IOCTL_AUDIOROUTER_BRIDGE_OPEN == 0x0022E000 && IOCTL_AUDIOROUTER_BRIDGE_QUERY == 0x0022600C,
        "IOCTL codes are METHOD_BUFFERED and match the Rust client");
    require(!AudioRouterRenderBlockWasOverrun(0, 0), "first render block is never an overrun");
    require(!AudioRouterRenderBlockWasOverrun(5, 5), "acknowledged block is not an overrun");
    require(AudioRouterRenderBlockWasOverrun(5, 4), "unread block replaced is an overrun");
    require(AudioRouterSequenceGap(0, 9) == 0, "first consumed block has no gap");
    require(AudioRouterSequenceGap(4, 5) == 0, "consecutive blocks have no gap");
    require(AudioRouterSequenceGap(4, 8) == 3, "skipped blocks counted");
    require(AudioRouterSequenceGap(8, 4) == 0, "regression is not a gap");

    constexpr SIZE_T kSmallView = AR_BRIDGE_PAYLOAD_OFFSET + 32;
    ULONG nonFinite = 0;
    std::vector<ULONGLONG> aligned((AR_BRIDGE_PAYLOAD_OFFSET + AR_BRIDGE_MAX_PAYLOAD_BYTES + 7) / 8);
    auto view = reinterpret_cast<UCHAR*>(aligned.data());
    auto sharedHeader = reinterpret_cast<AR_BRIDGE_BLOCK_HEADER*>(view + AR_BRIDGE_HEADER_OFFSET);
    auto samples = reinterpret_cast<DOUBLE*>(view + AR_BRIDGE_PAYLOAD_OFFSET);
    *sharedHeader = { 1, 2, 2, 2, 32 };
    samples[0] = -0.0; samples[1] = std::numeric_limits<DOUBLE>::denorm_min(); samples[2] = 0.5; samples[3] = -0.75;
    DOUBLE destination[5] = { 7, 7, 7, 7, 123 };
    AR_BRIDGE_BLOCK_HEADER copied = {};
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, kSmallView, 1, 1, destination, 4, &copied, &nonFinite)), "valid copy");
    require(std::memcmp(destination, samples, 32) == 0 && destination[4] == 123, "bit exact and canary");
    require(copied.Frames == 2 && copied.Generation == 1, "returned snapshot");
    mutateHeader = true;
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, kSmallView, 1, 1, destination, 4, &copied, &nonFinite)), "header mutation uses local snapshot");
    require(copied.Frames == 2 && copied.Generation == 1 && destination[4] == 123, "hostile dimensions cannot resize copy");
    mutateHeader = false;
    // Two-page allocation with an inaccessible following page. The valid
    // header/payload end exactly at the boundary; hostile enlarged dimensions
    // would fault the old double-fetch copy, independently of sanitizer support.
    auto guarded = static_cast<UCHAR*>(VirtualAlloc(nullptr, 8192, 0x2000, 0x01));
    require(guarded != nullptr && VirtualAlloc(guarded, 4096, 0x1000, 0x04) != nullptr, "guard-page allocation");
    UCHAR* boundedView = guarded + 4096 - kSmallView;
    *reinterpret_cast<AR_BRIDGE_BLOCK_HEADER*>(boundedView + AR_BRIDGE_HEADER_OFFSET) = { 1, 2, 2, 2, 32 };
    std::memcpy(boundedView + AR_BRIDGE_PAYLOAD_OFFSET, samples, 32);
    mutateHeader = true;
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(boundedView, kSmallView, 1, 1, destination, 4, &copied, &nonFinite)), "mutation at inaccessible page boundary");
    require(destination[4] == 123 && copied.Frames == 2, "guarded snapshot and destination bound");
    mutateHeader = false;
    require(VirtualFree(guarded, 0, 0x8000) != 0, "guard-page release");
    *sharedHeader = { 1, 2, 2, 2, 32 };
    require(AudioRouterCopyBridgeBlock(view, kSmallView - 1, 1, 1, destination, 4, &copied, &nonFinite) == STATUS_BUFFER_TOO_SMALL, "short view");
    require(AudioRouterCopyBridgeBlock(view, kSmallView, 1, 1, destination, 3, &copied, &nonFinite) == STATUS_BUFFER_TOO_SMALL, "short destination");
    require(!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, kSmallView, 1, 2, destination, 4, &copied, &nonFinite)), "repeated sequence");
    sharedHeader->PayloadBytes = 31;
    require(!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, kSmallView, 1, 1, destination, 4, &copied, &nonFinite)), "payload mismatch");
    sharedHeader->PayloadBytes = 32;
    for (DOUBLE nonfinite : { std::numeric_limits<DOUBLE>::quiet_NaN(), std::numeric_limits<DOUBLE>::infinity(), -std::numeric_limits<DOUBLE>::infinity() }) {
        samples[3] = nonfinite;
        require(AudioRouterCopyBridgeBlock(view, kSmallView, 1, 1, destination, 4, &copied, &nonFinite) == STATUS_DATA_ERROR, "nonfinite rejection");
        require(destination[0] == 0 && destination[1] == 0 && destination[2] == 0 && destination[3] == 0 && destination[4] == 123, "failed quantum scrubbed");
        require(nonFinite == 1, "rejected sample counted");
    }
    samples[0] = std::numeric_limits<DOUBLE>::quiet_NaN();
    samples[3] = std::numeric_limits<DOUBLE>::infinity();
    require(AudioRouterCopyBridgeBlock(view, kSmallView, 1, 1, destination, 4, &copied, &nonFinite) == STATUS_DATA_ERROR &&
        nonFinite == 2, "every non-finite sample in a rejected block is counted");
    samples[0] = 0.0; samples[3] = -0.75;
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, kSmallView, 1, 1, destination, 4, &copied, &nonFinite)) &&
        nonFinite == 0, "clean block reports no non-finite samples");
    *sharedHeader = { 1, 2, AR_BRIDGE_MAX_FRAMES, AR_BRIDGE_MAX_CHANNELS, AR_BRIDGE_MAX_PAYLOAD_BYTES };
    std::vector<DOUBLE> full(AR_BRIDGE_MAX_FRAMES * AR_BRIDGE_MAX_CHANNELS, 0.25);
    std::memcpy(samples, full.data(), AR_BRIDGE_MAX_PAYLOAD_BYTES);
    require(NT_SUCCESS(AudioRouterCopyBridgeBlock(view, aligned.size() * 8, 1, 1, full.data(), full.size(), &copied, &nonFinite)), "maximum shape");
    *sharedHeader = { 1, 2, 128, 2, 128 * 2 * sizeof(DOUBLE) };
    constexpr unsigned iterations = 200000;
    auto start = std::chrono::steady_clock::now();
    for (unsigned i = 0; i < iterations; ++i) {
        if (!NT_SUCCESS(AudioRouterCopyBridgeBlock(view, aligned.size() * 8, 1, 1, full.data(), full.size(), &copied, &nonFinite))) return 2;
    }
    auto elapsed = std::chrono::duration<double, std::micro>(std::chrono::steady_clock::now() - start).count();
    std::printf("bridge checks passed: %u; host copy 128x2 float64: %.3f us/block (%u iterations); not kernel DPC evidence\n", checks, elapsed / iterations, iterations);
}
