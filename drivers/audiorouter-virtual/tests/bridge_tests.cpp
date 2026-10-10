#define AR_BRIDGE_UNIT_TEST
#include "../Source/Inc/bridgeio.h"
#include "../Source/Inc/sampleconv.h"
#include "../Source/Inc/capturequeue.h"
#include "../Source/Inc/renderqueue.h"
#include "../Source/Inc/streamtiming.h"
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
// Same FIFO and ack decision used by WaveRT/adapter. The producer completes
// two quanta in one callback: the reader cannot run until that callback ends.
// This is deliberately unlike the old harness test that waited for each ack.
static void renderBurstChecks() {
    for (ULONG frames : {128UL, 480UL, 4096UL}) {
        for (ULONG channels : {1UL, 2UL, 8UL}) {
            AudioRouterRenderQueue queue = {};
            queue.Reset(frames, channels, 7);
            std::vector<DOUBLE> storage(AR_BRIDGE_MAX_CHANNELS * AR_BRIDGE_MAX_FRAMES);
            std::vector<DOUBLE> input(frames * channels);
            std::vector<DOUBLE> shared(frames * channels);
            ULONGLONG published = 0, ack = 0, activeGeneration = 7;
            auto publish = [&](const DOUBLE* samples, ULONG count, ULONGLONG generation) {
                if (generation != activeGeneration || !AudioRouterRenderSlotAvailable(published, ack)) {
                    return false;
                }
                std::memcpy(shared.data(), samples, count * sizeof(DOUBLE));
                ++published;
                return true;
            };
            for (ULONG block = 1; block <= 2; ++block) {
                for (ULONG index = 0; index < input.size(); ++index) {
                    input[index] = block + index / 65536.0;
                }
                require(!queue.Push(storage.data(), input.data()), "two-quantum callback retains both blocks");
                queue.Drain(storage.data(), publish);
            }
            require(published == 1 && queue.Count == 1, "second completion cannot replace unread shared block");
            for (ULONG block = 1; block <= 2; ++block) {
                bool exact = published == block;
                for (ULONG index = 0; index < shared.size(); ++index) {
                    exact = exact && shared[index] == block + index / 65536.0;
                }
                require(exact, "batched render samples delivered exactly in order");
                ack = published;
                queue.Drain(storage.data(), publish);
            }
            require(queue.Count == 0, "ack plus empty DMA tick drains tail");
            // An arbitrary future user acknowledgement must not permit a write.
            ack = ~ULONGLONG(0);
            require(!queue.Push(storage.data(), input.data()), "queue accepts while ack is hostile");
            queue.Drain(storage.data(), publish);
            require(queue.Count == 1 && published == 2, "future acknowledgement does not bypass flow control");
            ack = published;
            activeGeneration = 8;
            queue.Drain(storage.data(), publish);
            require(queue.Count == 1, "old queued generation cannot publish into replacement lease");
            queue.Reset(frames, channels, activeGeneration);
            require(queue.Count == 0, "generation reset makes old samples unreachable");

            // Overflow drops exactly the oldest private quantum; it never
            // overwrites the current shared block or silently clears counters.
            ULONG drops = 0;
            for (ULONG block = 1; block <= queue.Capacity + 2; ++block) {
                for (auto& sample : input) { sample = block; }
                if (queue.Push(storage.data(), input.data())) { ++drops; }
            }
            require(drops == 2 && queue.Count == queue.Capacity, "bounded FIFO reports each dropped block once");
            ULONG expected = 3;
            queue.Drain(storage.data(), [&](const DOUBLE* samples, ULONG count, ULONGLONG generation) {
                bool exact = generation == 8;
                for (ULONG i = 0; i < count; ++i) { exact = exact && samples[i] == expected; }
                require(exact, "overflow retains FIFO order of surviving samples");
                ++expected;
                return true;
            });
            require(queue.Count == 0, "drain iterations are bounded by capacity");
            require(queue.Capacity >= 1 && queue.Capacity <= 4 &&
                queue.Capacity * queue.SamplesPerBlock <= storage.size(), "all negotiated shapes fit existing storage");
            queue.Clear();
            require(queue.Count == 0, "STOP drops valid lengths without replay");
            for (auto& sample : input) { sample = 0.25; }
            AudioRouterPadRenderTail(input.data(), frames - 1, frames, channels);
            bool tailExact = true;
            for (ULONG i = 0; i < input.size(); ++i) {
                tailExact = tailExact && input[i] == (i < (frames - 1) * channels ? 0.25 : 0.0);
            }
            require(tailExact, "EoS preserves valid samples and zeroes only the partial block tail");
        }
    }
    AudioRouterRenderQueue invalid = {};
    invalid.Reset(0xffffffff, 8, 1);
    require(invalid.Capacity == 0, "invalid queue dimensions are bounded before multiplication");
    invalid.Reset(480, 2, 0);
    require(invalid.Capacity == 0, "missing lease invalidates queue");
}

static void packetClockChecks() {
    const ULONG frameBytes = 8;
    const ULONG packetBytes = 480 * frameBytes;
    const ULONG bufferBytes = 2 * packetBytes;
    require(AudioRouterCompletedPackets(0, bufferBytes, 2) == 0, "no completed packet at startup");
    require(AudioRouterCompletedPackets(packetBytes - frameBytes, bufferBytes, 2) == 0, "partial packet not reported complete");
    require(AudioRouterCompletedPackets(packetBytes, bufferBytes, 2) == 1, "first full packet completes once");
    require(AudioRouterCompletedPacketStart(1, packetBytes) == 0, "first capture packet timestamp starts at byte zero");
    require(AudioRouterCompletedPackets(163 * 48ULL * frameBytes, bufferBytes, 2) == 16, "163 ms delayed callback completes sixteen packets, not one");
    require(AudioRouterCompletedPackets(895 * 48000ULL * frameBytes, bufferBytes, 2) == 89500, "895 s pause resynchronizes packet count in constant work");
    const ULONGLONG wrappedPackets = 0x100000001ULL;
    require(AudioRouterCompletedPackets(wrappedPackets * packetBytes, bufferBytes, 2) == wrappedPackets, "packet clock retains high word after ULONG wrap");
    require(AudioRouterCompletedPacketStart(wrappedPackets, packetBytes) == 0x100000000ULL * packetBytes, "capture packet start remains correct after ULONG wrap");
    require(AudioRouterCompletedPackets(123, 0, 2) == 0 && AudioRouterCompletedPackets(123, 12, 0) == 0, "unconfigured packet clock rejects division by zero");
    const ULONG displacements[] = { 0, 8, packetBytes, bufferBytes, 163 * 48 * frameBytes, 895 * 48000 * frameBytes, ULONG_MAX };
    const ULONGLONG positions[] = { 0, 8, bufferBytes - 8, ~0ULL };
    for (ULONG displacement : displacements) {
        for (ULONGLONG position : positions) {
            const auto window = AudioRouterSurvivingDmaWindow(position, displacement, bufferBytes);
            require(window.Bytes <= bufferBytes, "capture callback processes at most one physical DMA lap");
            require(static_cast<ULONGLONG>(window.Bytes) + window.SkippedBytes == displacement, "skipped DMA bytes remain accounted as loss");
            require((static_cast<ULONGLONG>(window.Offset) + window.Bytes) % bufferBytes ==
                (position % bufferBytes + static_cast<ULONGLONG>(displacement)) % bufferBytes, "bounded DMA traversal preserves the full logical end position");
            require(window.Offset < bufferBytes, "surviving DMA window stays within buffer at wrap");
        }
    }
    require(AudioRouterSurvivingDmaWindow(1, ULONG_MAX, 0).Bytes == 0, "zero DMA size yields empty window");
    ULONGLONG ticks = 0;
    const ULONGLONG hundredDaysHns = 100ULL * 86400 * 10000000 + 1234567;
    require(AudioRouterHnsToQpc(hundredDaysHns, 10000000, &ticks) && ticks == hundredDaysHns, "capture timestamp remains valid after 100 days at 10 MHz");
    require(AudioRouterHnsToQpc(hundredDaysHns, 50000000, &ticks) && ticks == hundredDaysHns * 5, "capture timestamp retains fractional seconds at 50 MHz");
    require(AudioRouterHnsToQpc(1, 1000000, &ticks) && ticks == 0, "QPC conversion truncates sub-tick fraction");
    require(AudioRouterHnsToQpc(~0ULL, 10000000, &ticks) && ticks == ~0ULL, "representable QPC boundary does not overflow intermediate multiplication");
    require(!AudioRouterHnsToQpc(~0ULL, 20000000, &ticks), "truly overflowing QPC output is rejected");
    require(!AudioRouterHnsToQpc(1, 0, &ticks), "zero QPC frequency is rejected");
    require(!AudioRouterHnsToQpc(9999999, ~0ULL, &ticks), "unrepresentable fractional multiplication fails closed");
    require(!AudioRouterHnsToQpc(19999999, ~0ULL, &ticks), "large whole and fractional clock overflow fails closed");
    require(!AudioRouterHnsToQpc(10000001, ~0ULL - 42, &ticks), "overflow when adding valid whole and fractional ticks is rejected");
}

static void renderCommitChecks() {
    constexpr ULONG packetBytes = 16;
    AudioRouterRenderCommits commits = {};
    commits.Reset();
    require(commits.Contains(0, 0, 2), "polling remains outside packet validity mode");
    require(commits.Contains(32, packetBytes, 2), "legacy event mode preserved before first write");
    require(AudioRouterRenderCommitDelta(0, 0, false) == 0, "prefill admits packet zero");
    commits.RecordWrite(0, 0, 2, true);
    commits.RecordWrite(1, 0, 2, true);
    const DOUBLE dma[2] = {0.25, -0.5};
    ULONGLONG missingFrames = 0;
    for (ULONGLONG packet = 0; packet < 4; ++packet) {
        if (packet == 3) {
            require(AudioRouterRenderCommitDelta(3, 2, true) == 0, "next packet admitted after missed packet");
            commits.RecordWrite(3, 2, 2, true);
        }
        for (ULONG frame = 0; frame < 4; ++frame) {
            const bool valid = commits.Contains(packet * packetBytes + frame * 4, packetBytes, 2);
            const DOUBLE sample = valid ? dma[packet % 2] : 0.0;
            missingFrames += valid ? 0 : 1;
            require(sample == (packet == 2 ? 0.0 : dma[packet % 2]),
                    "missed OS write cannot replay the previous DMA lap");
        }
    }
    require(missingFrames == 4, "only missing producer frames count as underruns");
    require(AudioRouterCompletedPackets(64, 32, 2) == 4,
            "missing producer data does not stop packet clock");
    require(AudioRouterRenderCommitDelta(2, 2, true) < 0, "late packet reported late");
    require(AudioRouterRenderCommitDelta(4, 2, true) > 0, "ahead packet reported overrun");
    require(AudioRouterRenderCommitDelta(3, 2, true) == 0, "duplicate admission matches existing contract");

    // A late write of the packet now transferring holds that packet's data:
    // its unconsumed bytes play although the DDI reports STATUS_DATA_LATE_ERROR.
    commits.Reset();
    commits.RecordWrite(5, 5, 2, true);
    require(AudioRouterRenderCommitDelta(5, 5, true) < 0, "late current packet still reported late");
    require(commits.Contains(5 * packetBytes + 8, packetBytes, 2), "late current packet remainder is usable");
    require(!commits.Contains(3 * packetBytes + 8, packetBytes, 2), "late write never revives the previous lap");
    // Overwriting the transferring slot (too far ahead) retires its remainder.
    commits.Reset();
    commits.RecordWrite(5, 4, 2, true);
    require(commits.Contains(5 * packetBytes, packetBytes, 2), "current packet valid before overwrite");
    commits.RecordWrite(7, 5, 2, true);
    require(!commits.Contains(5 * packetBytes + 8, packetBytes, 2), "overrun write silences overwritten current remainder");
    require(commits.Contains(7 * packetBytes, packetBytes, 2), "overrun payload belongs only to its named packet");

    for (ULONG notifications : {1UL, 2UL}) {
        commits.Reset();
        commits.RecordWrite(1, 1, notifications, true);
        commits.RecordWrite(1 + notifications, 1, notifications, false);
        require(!commits.Contains(packetBytes, packetBytes, notifications),
                "unusable (EOS) write still retires the overwritten slot");
        require(!commits.Contains((1 + notifications) * packetBytes, packetBytes, notifications),
                "unusable (EOS) payload never plays");
    }
    commits.Reset();
    commits.RecordWrite(0, 0, 1, true);
    commits.RecordWrite(1, 0, 1, true);
    require(!commits.Contains(8, packetBytes, 1),
            "one-slot next write silences overwritten current remainder");
    require(commits.Contains(packetBytes + 8, packetBytes, 1),
            "one-slot next payload belongs only to next logical packet");
    require(sizeof(commits) <= 24, "packet metadata is fixed and bounded");
    commits.Reset();
    commits.RecordWrite(0xffffffffUL, 0xfffffffeULL, 2, true);
    require(commits.Contains(0xffffffffULL * packetBytes, packetBytes, 2),
            "ULONG_MAX is a valid packet identity rather than a sentinel");
    require(AudioRouterRenderCommitDelta(0, 0xffffffffULL, true) == 0,
            "wire packet number wraps while logical packet identity continues");
    commits.RecordWrite(0, 0xffffffffULL, 2, true);
    require(commits.Contains(0x100000000ULL * packetBytes, packetBytes, 2), "wrapped wire number resolves to next identity");
    require(!commits.Contains(0, packetBytes, 2), "wrapped slot cannot revive startup packet");
    require(commits.Contains(0xffffffffULL * packetBytes + 8, packetBytes, 2),
            "pause preserves partially consumed current packet");
    const auto window = AudioRouterSurvivingDmaWindow(16, 16000, 32);
    require(!commits.Contains(16 + window.SkippedBytes, packetBytes, 2),
            "long stall cannot revive historical packet identities");
    commits.Reset();
    commits.RecordWrite(0xffffffffUL, 0, 2, true);
    require(!commits.Valid[1] && commits.PacketMode, "identity before stream start is unrepresentable and unusable");
    commits.Reset();
    const ULONGLONG saturated = ~0ULL >> 1;
    commits.RecordWrite(static_cast<ULONG>(saturated) + 1, saturated, 2, true);
    require(!commits.Valid[0] && !commits.Valid[1], "identity beyond the saturated packet count is unusable");
    commits.RecordWrite(static_cast<ULONG>(saturated), saturated, 2, true);
    require(commits.Contains(saturated, 1, 2), "saturated packet count itself remains representable");
    commits.Reset();
    commits.RecordWrite(0, 0, 3, true);
    require(!commits.PacketMode, "unsupported notification count records nothing");
    commits.Reset();
    require(!commits.PacketMode && !commits.Valid[0] && !commits.Valid[1],
            "STOP and buffer replacement discard all packet identities");
    commits.RecordWrite(0, 0, 2, true);
    require(!commits.Contains(0, 0, 2), "packet mode fails closed with invalid packet size");
    require(!commits.Contains(0, packetBytes, 0), "packet mode fails closed without notifications");
}

// Offline WaveRT render timing model. The OS writes a packet into its slot,
// then (after a configurable write-to-call gap) calls SetWritePacket; the
// 1 ms timer consumes DMA and notifies once per completed packet. Each
// consumed frame is classified against what the slot physically holds.
// Production helpers decide validity, packet clock and admission; the OS
// reaction (resync via packet count on an error) follows the GetPacketCount
// documentation. This is a model, not kernel timing evidence.
// With a zero gap the provenance assertions largely restate that RecordWrite
// resolves identities and slots like the model's own ground truth; the
// meaningful checks are the reference regressions (the 28b989f5 rule silences
// written audio, no validity replays stale laps), the on-time lossless check
// and the gap bound: the driver cannot see a write before its call, so only
// frames consumed inside that gap may be stale or wrongly silenced.
enum class RenderPolicy { Legacy, CommitAfterProgress, SlotProvenance };
// How the modeled OS reacts to its notification and to a dataflow error.
// The documentation fixes only that it resynchronizes from the packet count.
enum class OsModel { BlindRetryNow, BlindRetryNextWake, CountFirst };
struct RenderSimResult {
    ULONGLONG Correct = 0, Stale = 0, SilencedValid = 0, SilencedMissing = 0;
    ULONG Accepted = 0, Late = 0, Overrun = 0, DeferredCalls = 0;
};
// The 28b989f5 rule, retained only as the regression reference.
struct CommitAfterProgressReference {
    ULONGLONG Packets[2] = {}; bool Valid[2] = {}; bool PacketMode = false;
    void Invalidate(ULONG packetNumber, ULONG notifications) {
        for (ULONG slot = 0; slot < 2; ++slot) {
            if (Valid[slot] && static_cast<ULONG>(Packets[slot]) % notifications == packetNumber % notifications) { Valid[slot] = false; }
        }
    }
    void Commit(ULONGLONG packet) { Packets[packet & 1] = packet; Valid[packet & 1] = true; PacketMode = true; }
    bool Contains(ULONGLONG byte, ULONG packetBytes) const {
        if (!PacketMode) { return true; }
        const ULONGLONG packet = byte / packetBytes;
        return Valid[packet & 1] && Packets[packet & 1] == packet;
    }
};
template <typename Delay, typename Skip>
static RenderSimResult simulateRender(RenderPolicy policy, OsModel osModel, ULONG notifications,
                                      ULONG totalMs, Delay osDelayUs, Skip osSkips, ULONG writeLeadUs) {
    constexpr ULONG frameBytes = 8, framesPerMs = 48, packetFrames = 480;
    const ULONG packetBytes = packetFrames * frameBytes;
    const ULONG bufferBytes = packetBytes * notifications;
    RenderSimResult result;
    LONG64 slotHolds[2] = {-1, -1};
    ULONGLONG linear = 0, lastNotified = 0;
    AudioRouterRenderCommits commits = {}; commits.Reset();
    CommitAfterProgressReference reference;
    auto counter = [&]() { return AudioRouterCompletedPackets(linear, bufferBytes, notifications); };
    auto consume = [&](ULONGLONG toByte) {
        for (; linear < toByte; linear += frameBytes) {
            const ULONGLONG packet = linear / packetBytes;
            const bool truth = slotHolds[packet % notifications] == static_cast<LONG64>(packet);
            const bool valid = policy == RenderPolicy::Legacy ? true :
                policy == RenderPolicy::SlotProvenance ? commits.Contains(linear, packetBytes, notifications)
                                                       : reference.Contains(linear, packetBytes);
            if (valid) { if (truth) { ++result.Correct; } else { ++result.Stale; } }
            else { if (truth) { ++result.SilencedValid; } else { ++result.SilencedMissing; } }
        }
    };
    bool running = false;
    ULONGLONG nowByte = 0;
    auto osWrite = [&](ULONGLONG packet) {
        slotHolds[static_cast<ULONG>(packet) % notifications] = static_cast<LONG64>(packet);
    };
    // The driver side of SetWritePacket, in the production order. Returns the
    // DDI admission delta (0 accepted, <0 late, >0 overrun).
    auto ddiSetWritePacket = [&](ULONGLONG packet) {
        const ULONG wire = static_cast<ULONG>(packet);
        if (policy == RenderPolicy::SlotProvenance) { commits.RecordWrite(wire, counter(), notifications, true); }
        if (policy == RenderPolicy::CommitAfterProgress) { reference.Invalidate(wire, notifications); }
        if (running) { consume(nowByte); }
        const ULONGLONG current = counter();
        const LONG delta = AudioRouterRenderCommitDelta(wire, current, running);
        if (delta == 0) {
            ++result.Accepted;
            if (policy == RenderPolicy::CommitAfterProgress) { reference.Commit(current + (running ? 1 : 0)); }
        } else if (delta < 0) { ++result.Late; } else { ++result.Overrun; }
        return delta;
    };
    struct Event { ULONGLONG TimeUs; bool Call; ULONGLONG Packet; };
    std::vector<Event> events;
    ULONGLONG osNext = 0;
    ULONGLONG nowUs = 0;
    auto afterCall = [&](ULONGLONG packet, LONG delta) {
        if (delta == 0) { osNext = packet + 1; return; }
        if (running) { consume(nowByte); }
        osNext = counter() + 1;
    };
    auto osWake = [&]() {
        if (osModel == OsModel::CountFirst) {
            // Query the count, never write behind or beyond count + 1.
            if (running) { consume(nowByte); }
            const ULONGLONG target = counter() + (running ? 1 : 0);
            if (osNext > target) { return; }
            osNext = target;
        }
        const ULONGLONG packet = osNext;
        if (osSkips(packet)) { osNext = packet + 1; return; }
        osWrite(packet);
        if (running && writeLeadUs != 0) {
            // The call arrives later; timer ticks may consume in between.
            ++result.DeferredCalls;
            osNext = packet + 1;   // provisional until the call's result
            events.push_back({nowUs + writeLeadUs, true, packet});
            return;
        }
        const LONG delta = ddiSetWritePacket(packet);
        afterCall(packet, delta);
        if (delta != 0 && osModel == OsModel::BlindRetryNow && !osSkips(osNext)) {
            const ULONGLONG retry = osNext;
            osWrite(retry);
            afterCall(retry, ddiSetWritePacket(retry));
        }
    };
    // Prefill before RUN: the current packet (0).
    osWake();
    running = true;
    // Packet 0 is transferring after RUN, so the OS writes packet 1 at once.
    osWake();
    ULONG notificationIndex = 0;
    for (ULONG ms = 1; ms <= totalMs; ++ms) {
        // OS events due within this millisecond happen at their exact time,
        // earliest first.
        for (;;) {
            size_t earliest = events.size();
            for (size_t i = 0; i < events.size(); ++i) {
                if (events[i].TimeUs <= ms * 1000ULL &&
                    (earliest == events.size() || events[i].TimeUs < events[earliest].TimeUs)) { earliest = i; }
            }
            if (earliest == events.size()) { break; }
            const Event event = events[earliest];
            events.erase(events.begin() + static_cast<std::ptrdiff_t>(earliest));
            nowUs = event.TimeUs;
            nowByte = event.TimeUs * framesPerMs / 1000 * frameBytes;
            if (event.Call) {
                const LONG delta = ddiSetWritePacket(event.Packet);
                // A newer write already superseded this call's resync point;
                // a real OS writes and calls on one thread, never interleaved.
                if (delta != 0 && osNext == event.Packet + 1) { afterCall(event.Packet, delta); }
            } else {
                osWake();
            }
        }
        nowUs = ms * 1000ULL;
        nowByte = static_cast<ULONGLONG>(ms) * framesPerMs * frameBytes;
        consume(nowByte);   // 1 ms timer DPC
        if (counter() > lastNotified) {
            lastNotified = counter();
            events.push_back({ms * 1000ULL + osDelayUs(notificationIndex), false, 0});
            ++notificationIndex;
        }
    }
    return result;
}
static void renderTimingModelChecks() {
    struct Scenario { const char* name; ULONG notifications; ULONG (*delayUs)(ULONG); bool (*skip)(ULONGLONG); bool producerMisses; };
    static ULONG seed;
    const Scenario scenarios[] = {
        {"on-time 1 ms", 2, [](ULONG) -> ULONG { return 1000; }, [](ULONGLONG) { return false; }, false},
        {"late at boundary 10 ms", 2, [](ULONG i) -> ULONG { return i % 7 == 3 ? 10000 : 1000; }, [](ULONGLONG) { return false; }, true},
        {"late mid-packet 13.5 ms", 2, [](ULONG i) -> ULONG { return i % 5 == 2 ? 13500 : 1000; }, [](ULONGLONG) { return false; }, true},
        {"jitter 0-25 ms", 2, [](ULONG) -> ULONG { seed = seed * 1103515245u + 12345u; return (seed >> 8) % 25000; }, [](ULONGLONG) { return false; }, true},
        {"OS skips packets", 2, [](ULONG) -> ULONG { return 1000; }, [](ULONGLONG p) { return p % 9 == 4; }, true},
        {"one-slot on-time", 1, [](ULONG) -> ULONG { return 1000; }, [](ULONGLONG) { return false; }, true},
    };
    const OsModel osModels[3] = {OsModel::BlindRetryNow, OsModel::BlindRetryNextWake, OsModel::CountFirst};
    const char* osNames[3] = {"blind, retry now", "blind, retry on wake", "count first"};
    const ULONG leads[2] = {0, 2000};
    std::printf("render timing model (30 s, 480-frame packets; frames; lead = OS write-to-call gap):\n");
    std::printf("  %-24s %-21s %5s %-18s %9s %7s %9s %9s %5s %5s %5s\n", "scenario", "OS model", "lead",
                "policy", "correct", "stale", "silValid", "silMiss", "acc", "late", "over");
    for (ULONG lead : leads)
    for (int os = 0; os < 3; ++os)
    for (const auto& scenario : scenarios) {
        RenderSimResult results[3];
        const RenderPolicy policies[3] = {RenderPolicy::Legacy, RenderPolicy::CommitAfterProgress, RenderPolicy::SlotProvenance};
        const char* names[3] = {"no validity (pre)", "28b989f5 commit", "slot provenance"};
        for (int p = 0; p < 3; ++p) {
            seed = 12345;
            results[p] = simulateRender(policies[p], osModels[os], scenario.notifications, 30000,
                                        scenario.delayUs, scenario.skip, lead);
            const auto& r = results[p];
            std::printf("  %-24s %-21s %5lu %-18s %9llu %7llu %9llu %9llu %5lu %5lu %5lu\n", scenario.name, osNames[os],
                        lead, names[p], r.Correct, r.Stale, r.SilencedValid, r.SilencedMissing, r.Accepted, r.Late, r.Overrun);
        }
        const auto& fixed = results[2];
        require(fixed.Correct + fixed.SilencedMissing + fixed.Stale + fixed.SilencedValid == 30000ULL * 48,
                "every consumed frame classified once");
        if (lead == 0) {
            require(fixed.Stale == 0, "slot provenance never plays a stale lap");
            require(fixed.SilencedValid == 0, "slot provenance never silences data the OS already reported");
        } else {
            // Only frames consumed between an OS write and its call can be
            // misjudged: at most one DPC per started millisecond of the gap.
            const ULONGLONG window = static_cast<ULONGLONG>(fixed.DeferredCalls) * (lead / 1000 + 1) * 48;
            require(fixed.Stale + fixed.SilencedValid <= window, "write-to-call gap bounds provenance error");
        }
        if (!scenario.producerMisses && lead == 0) {
            for (const auto& r : results) {
                require(r.Correct == 30000ULL * 48 && r.Late == 0 && r.Overrun == 0, "on-time producer is lossless under every policy");
            }
        }
        if (!scenario.producerMisses) {
            require(fixed.Correct == 30000ULL * 48, "on-time producer is lossless with slot provenance, gap or not");
        }
    }
    // Regression evidence: the late-packet rule of 28b989f5 silenced written
    // audio, and the original no-validity reader replayed stale laps.
    // An OS that queries the count first never writes behind it, so only the
    // blind-write models can expose the reference's late-write loss.
    for (OsModel os : {OsModel::BlindRetryNow, OsModel::BlindRetryNextWake}) {
        seed = 12345;
        const auto jitterReference = simulateRender(RenderPolicy::CommitAfterProgress, os, 2, 30000,
            scenarios[3].delayUs, scenarios[3].skip, 0);
        require(jitterReference.SilencedValid > 0, "reference reproduces silenced late-written audio");
    }
    seed = 12345;
    const auto boundaryReference = simulateRender(RenderPolicy::CommitAfterProgress, OsModel::BlindRetryNextWake, 2,
        30000, scenarios[1].delayUs, scenarios[1].skip, 0);
    require(boundaryReference.SilencedValid > 0, "reference silences a packet written at its boundary");
    const auto skipLegacy = simulateRender(RenderPolicy::Legacy, OsModel::BlindRetryNextWake, 2, 30000,
        scenarios[4].delayUs, scenarios[4].skip, 0);
    require(skipLegacy.Stale > 0, "reference reproduces stale replay without validity");
}

int main() {
    renderCommitChecks();
    renderTimingModelChecks();
    packetClockChecks();
    renderBurstChecks();
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
    require(AudioRouterRenderSlotAvailable(0, 0), "first render block can publish");
    require(AudioRouterRenderSlotAvailable(5, 5), "acknowledged block can be replaced");
    require(!AudioRouterRenderSlotAvailable(5, 4), "unread block cannot be replaced");
    require(!AudioRouterRenderSlotAvailable(5, 6), "future acknowledgement cannot authorize replacement");
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
