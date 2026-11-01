#define NOMINMAX

#include <windows.h>
#include <bcrypt.h>
#include <werapi.h>

#include <algorithm>
#include <array>
#include <atomic>
#include <cctype>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <iomanip>
#include <iostream>
#include <numeric>
#include <fcntl.h>
#include <io.h>
#include <limits>
#include <memory>
#include <mutex>
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <thread>
#include <unordered_set>
#include <utility>
#include <vector>

#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/base/ibstream.h"
#include "pluginterfaces/gui/iplugview.h"
#include "pluginterfaces/gui/iplugviewcontentscalesupport.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivsthostapplication.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include "pluginterfaces/vst/vstspeaker.h"

namespace fs = std::filesystem;
using namespace Steinberg;
using GetPluginFactoryProc = IPluginFactory* (PLUGIN_API*)();
// Windows module entry points (the SDK's module loader calls them around a
// plugin's lifetime; VSTGUI editors need InitDll for their instance handle).
using ModuleEntryProc = bool (PLUGIN_API*)();

constexpr std::size_t kMaxMessageBytes = 4 * 1024 * 1024;
constexpr std::size_t kMaxFrames = 4096;
// Keep the native descriptor/event bound aligned with the Rust worker
// contract. The bound remains finite and is enforced before queue growth.
constexpr std::size_t kMaxParameters = 256;
// Upper bound on a plugin's declared parameter list that the worker walks to
// find its user-facing parameters.
constexpr std::size_t kMaxScannedParameters = 65536;
constexpr std::size_t kMaxStateBytes = 16 * 1024 * 1024;
constexpr uint16_t kProtocolVersion = 1;

static std::runtime_error protocol_error(const std::string& message) {
    return std::runtime_error("protocol: " + message);
}

// Protocol pipes, duplicated before any plugin code loads. The process's own
// stdin/stdout then point at NUL, so plugin printing (UADx plugins log a
// banner at load) can never be read by the host as a frame header.
static HANDLE g_protocol_input = INVALID_HANDLE_VALUE;
static HANDLE g_protocol_output = INVALID_HANDLE_VALUE;

static void isolate_protocol_pipes() {
    const auto process = GetCurrentProcess();
    if (!DuplicateHandle(process, GetStdHandle(STD_INPUT_HANDLE), process, &g_protocol_input, 0,
                         FALSE, DUPLICATE_SAME_ACCESS) ||
        !DuplicateHandle(process, GetStdHandle(STD_OUTPUT_HANDLE), process, &g_protocol_output, 0,
                         FALSE, DUPLICATE_SAME_ACCESS)) {
        throw std::runtime_error("could not duplicate worker protocol pipes");
    }
    const int null_descriptor = _open("NUL", _O_RDWR | _O_BINARY);
    if (null_descriptor == -1 || _dup2(null_descriptor, 0) != 0 || _dup2(null_descriptor, 1) != 0) {
        throw std::runtime_error("could not redirect worker stdio away from the protocol pipes");
    }
    _close(null_descriptor);
    // Plugins with their own C runtime read the Win32 standard handles.
    if (!SetStdHandle(STD_INPUT_HANDLE, reinterpret_cast<HANDLE>(_get_osfhandle(0))) ||
        !SetStdHandle(STD_OUTPUT_HANDLE, reinterpret_cast<HANDLE>(_get_osfhandle(1)))) {
        throw std::runtime_error("could not replace worker standard handles");
    }
}

static bool read_exact(void* data, std::size_t size) {
    auto* cursor = static_cast<uint8_t*>(data);
    while (size > 0) {
        DWORD read = 0;
        const auto chunk = static_cast<DWORD>(std::min<std::size_t>(size, std::size_t{1} << 20));
        if (!ReadFile(g_protocol_input, cursor, chunk, &read, nullptr) || read == 0) return false;
        cursor += read;
        size -= read;
    }
    return true;
}

static void write_all(const void* data, std::size_t size) {
    const auto* cursor = static_cast<const uint8_t*>(data);
    while (size > 0) {
        DWORD written = 0;
        const auto chunk = static_cast<DWORD>(std::min<std::size_t>(size, std::size_t{1} << 20));
        if (!WriteFile(g_protocol_output, cursor, chunk, &written, nullptr) || written == 0) {
            throw std::runtime_error("worker output write failed");
        }
        cursor += written;
        size -= written;
    }
}

static std::vector<uint8_t> read_frame() {
    uint8_t header[4]{};
    if (!read_exact(header, sizeof(header))) {
        throw std::runtime_error("worker input closed");
    }
    const auto length = static_cast<std::size_t>(header[0]) |
                        (static_cast<std::size_t>(header[1]) << 8) |
                        (static_cast<std::size_t>(header[2]) << 16) |
                        (static_cast<std::size_t>(header[3]) << 24);
    if (length == 0 || length > kMaxMessageBytes) {
        throw protocol_error("message length is outside the bounded contract");
    }
    std::vector<uint8_t> payload(length);
    if (!read_exact(payload.data(), payload.size())) {
        throw std::runtime_error("worker input frame truncated");
    }
    return payload;
}

static void write_frame(const std::string& payload) {
    if (payload.empty() || payload.size() > kMaxMessageBytes) {
        throw protocol_error("output message is outside the bounded contract");
    }
    const auto length = static_cast<uint32_t>(payload.size());
    const uint8_t header[] = {static_cast<uint8_t>(length & 0xff),
                              static_cast<uint8_t>((length >> 8) & 0xff),
                              static_cast<uint8_t>((length >> 16) & 0xff),
                              static_cast<uint8_t>((length >> 24) & 0xff)};
    std::string frame(reinterpret_cast<const char*>(header), sizeof(header));
    frame += payload;
    write_all(frame.data(), frame.size());
}

static std::size_t find_required(const std::string& json, const std::string& token,
                                 std::size_t from = 0) {
    const auto position = json.find(token, from);
    if (position == std::string::npos) {
        throw protocol_error("missing " + token);
    }
    return position + token.size();
}

static uint64_t parse_uint(const std::string& json, std::size_t& position) {
    while (position < json.size() && (json[position] == ' ' || json[position] == '\n')) {
        ++position;
    }
    const auto start = position;
    while (position < json.size() && json[position] >= '0' && json[position] <= '9') {
        ++position;
    }
    if (start == position) {
        throw protocol_error("expected unsigned integer");
    }
    try {
        return std::stoull(json.substr(start, position - start));
    } catch (const std::exception&) {
        throw protocol_error("unsigned integer is out of range");
    }
}

static double parse_number(const std::string& json, std::size_t& position) {
    while (position < json.size() && (json[position] == ' ' || json[position] == '\n')) {
        ++position;
    }
    const auto start = position;
    while (position < json.size() &&
           (std::isdigit(static_cast<unsigned char>(json[position])) ||
            json[position] == '-' || json[position] == '+' || json[position] == '.' ||
            json[position] == 'e' || json[position] == 'E')) {
        ++position;
    }
    if (start == position) {
        throw protocol_error("expected number");
    }
    char* end = nullptr;
    const auto value = std::strtod(json.substr(start, position - start).c_str(), &end);
    if (!end || !std::isfinite(value)) {
        throw protocol_error("number is non-finite or invalid");
    }
    return value;
}

static std::size_t find_array_start(const std::string& json, const std::string& key,
                                    std::size_t from = 0) {
    return find_required(json, key, from);
}

static std::vector<float> parse_samples_at(const std::string& json, std::size_t position) {
    std::vector<float> samples;
    while (position < json.size() && json[position] != ']') {
        const auto value = parse_number(json, position);
        if (!std::isfinite(value) || value < -1.0e6 || value > 1.0e6) {
            throw protocol_error("sample is outside the finite bounded range");
        }
        samples.push_back(static_cast<float>(value));
        if (samples.size() > kMaxFrames * 2) {
            throw protocol_error("sample block exceeds the bounded worker size");
        }
        while (position < json.size() && (json[position] == ' ' || json[position] == ',')) {
            ++position;
        }
    }
    if (position >= json.size() || samples.empty()) {
        throw protocol_error("sample array is empty or malformed");
    }
    return samples;
}

static std::vector<float> parse_samples(const std::string& json) {
    return parse_samples_at(json, find_array_start(json, "\"samples\":["));
}

static uint64_t required_uint_field(const std::string& json, const std::string& key);
static std::string quote_json(const std::string& value);

static std::string parse_string_field(const std::string& json, const std::string& key) {
    auto position = find_required(json, key);
    if (position >= json.size() || json[position] != '"') throw protocol_error("expected string");
    ++position;
    std::string value;
    while (position < json.size() && json[position] != '"') {
        if (json[position] == '\\' || static_cast<unsigned char>(json[position]) < 0x20) {
            throw protocol_error("unsupported string escape");
        }
        value.push_back(json[position++]);
    }
    if (position >= json.size()) throw protocol_error("unterminated string");
    return value;
}

static std::vector<uint8_t> parse_bytes(const std::string& json) {
    auto position = find_required(json, "\"bytes\":[");
    std::vector<uint8_t> bytes;
    while (position < json.size() && json[position] != ']') {
        const auto value = parse_uint(json, position);
        if (value > 255 || bytes.size() >= kMaxStateBytes) {
            throw protocol_error("state bytes exceed the bounded contract");
        }
        bytes.push_back(static_cast<uint8_t>(value));
        while (position < json.size() && (json[position] == ' ' || json[position] == ',')) ++position;
    }
    if (position >= json.size() || bytes.empty()) throw protocol_error("state bytes are empty or malformed");
    return bytes;
}

static std::string sha256_hex(const uint8_t* bytes, std::size_t size) {
    BCRYPT_ALG_HANDLE algorithm = nullptr;
    BCRYPT_HASH_HANDLE hash = nullptr;
    DWORD object_size = 0;
    DWORD result_size = 0;
    DWORD hash_size = 0;
    std::vector<uint8_t> object;
    std::vector<uint8_t> digest;
    auto cleanup = [&]() {
        if (hash) BCryptDestroyHash(hash);
        if (algorithm) BCryptCloseAlgorithmProvider(algorithm, 0);
    };
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) != 0 ||
        BCryptGetProperty(algorithm, BCRYPT_OBJECT_LENGTH, reinterpret_cast<PUCHAR>(&object_size),
                          sizeof(object_size), &result_size, 0) != 0 ||
        BCryptGetProperty(algorithm, BCRYPT_HASH_LENGTH, reinterpret_cast<PUCHAR>(&hash_size),
                          sizeof(hash_size), &result_size, 0) != 0) {
        cleanup();
        throw std::runtime_error("SHA-256 provider setup failed");
    }
    object.resize(object_size);
    digest.resize(hash_size);
    if (BCryptCreateHash(algorithm, &hash, object.data(), object_size, nullptr, 0, 0) != 0 ||
        (size != 0 && BCryptHashData(hash, const_cast<PUCHAR>(bytes), static_cast<ULONG>(size), 0) != 0) ||
        BCryptFinishHash(hash, digest.data(), hash_size, 0) != 0) {
        cleanup();
        throw std::runtime_error("SHA-256 computation failed");
    }
    cleanup();
    std::ostringstream result;
    result << std::hex << std::setfill('0');
    for (const auto byte : digest) result << std::setw(2) << static_cast<unsigned int>(byte);
    return result.str();
}

static std::string state_message(uint32_t version, const std::vector<uint8_t>& bytes) {
    std::ostringstream output;
    output << "{\"type\":\"State\",\"payload\":{\"asset\":{\"version\":"
           << version << ",\"bytes\":[";
    for (std::size_t index = 0; index < bytes.size(); ++index) {
        if (index) output << ',';
        output << static_cast<unsigned int>(bytes[index]);
    }
    output << "],\"sha256\":" << quote_json(sha256_hex(bytes.data(), bytes.size())) << "}}}";
    return output.str();
}

struct BusFrame {
    uint64_t sequence;
    uint64_t deadline;
    uint16_t channels;
    std::vector<float> samples;
};

static BusFrame parse_bus_frame(const std::string& json, std::size_t object_start) {
    const auto sequence = required_uint_field(json.substr(object_start), "\"sequence\":");
    const auto deadline = required_uint_field(json.substr(object_start), "\"deadline_tick\":");
    const auto channels = required_uint_field(json.substr(object_start), "\"channels\":");
    if (channels < 1 || channels > 2) throw protocol_error("bus frame channel count is out of range");
    const auto samples_position = find_required(json, "\"samples\":[", object_start);
    auto close = json.find(']', samples_position);
    if (close == std::string::npos) throw protocol_error("bus frame samples are unterminated");
    return {sequence, deadline, static_cast<uint16_t>(channels),
            parse_samples_at(json, samples_position)};
}

static std::vector<BusFrame> parse_bus_frames(const std::string& json) {
    const auto array = find_required(json, "\"frames\":[");
    std::vector<BusFrame> frames;
    auto position = array;
    while (position < json.size() && json[position] != ']') {
        const auto object = json.find('{', position);
        if (object == std::string::npos) throw protocol_error("bus frame object is missing");
        frames.push_back(parse_bus_frame(json, object));
        if (frames.size() > 8) throw protocol_error("bus frame count exceeds the bus limit");
        const auto samples = json.find(']', object);
        if (samples == std::string::npos) throw protocol_error("bus frame is unterminated");
        position = samples + 1;
        while (position < json.size() &&
               (json[position] == ',' || json[position] == ' ' || json[position] == '}')) {
            ++position;
        }
    }
    if (frames.empty() || position >= json.size()) throw protocol_error("bus frame array is empty");
    return frames;
}

struct ParameterEvent {
    uint32_t id;
    float value;
    std::size_t offset;
};

static std::vector<ParameterEvent> parse_parameters(const std::string& json) {
    const auto array = json.find("\"parameters\":[");
    if (array == std::string::npos) {
        throw protocol_error("missing parameter array");
    }
    auto position = array + std::string("\"parameters\":[").size();
    std::vector<ParameterEvent> events;
    while (position < json.size() && json[position] != ']') {
        const auto id_start = find_required(json, "\"parameter_id\":", position);
        auto cursor = id_start;
        const auto id = parse_uint(json, cursor);
        const auto value_start = find_required(json, "\"normalized_value\":", cursor);
        cursor = value_start;
        const auto value = parse_number(json, cursor);
        const auto offset_start = find_required(json, "\"sample_offset\":", cursor);
        cursor = offset_start;
        const auto offset = parse_uint(json, cursor);
        if (id > std::numeric_limits<uint32_t>::max() || value < 0.0 || value > 1.0 ||
            offset > kMaxFrames) {
            throw protocol_error("parameter event is outside the bounded contract");
        }
        events.push_back({static_cast<uint32_t>(id), static_cast<float>(value),
                          static_cast<std::size_t>(offset)});
        if (events.size() > kMaxParameters) {
            throw protocol_error("parameter event count exceeds the bounded contract");
        }
        position = cursor;
        const auto next = json.find_first_of("{},]", position);
        if (next == std::string::npos) {
            throw protocol_error("parameter array is malformed");
        }
        position = next;
        if (json[position] == '}') {
            ++position;
        }
        while (position < json.size() && (json[position] == ',' || json[position] == ' ')) {
            ++position;
        }
    }
    if (position >= json.size()) {
        throw protocol_error("parameter array is unterminated");
    }
    return events;
}

static uint64_t required_uint_field(const std::string& json, const std::string& key) {
    auto position = find_required(json, key);
    return parse_uint(json, position);
}

class ParameterQueue final : public Vst::IParamValueQueue {
public:
    explicit ParameterQueue(Vst::ParamID id) : id_(id) {}

    tresult PLUGIN_API queryInterface(const TUID, void** object) override {
        if (object) *object = nullptr;
        return kNoInterface;
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    Vst::ParamID PLUGIN_API getParameterId() override { return id_; }
    int32 PLUGIN_API getPointCount() override { return static_cast<int32>(points_.size()); }
    tresult PLUGIN_API getPoint(int32 index, int32& offset, Vst::ParamValue& value) override {
        if (index < 0 || static_cast<std::size_t>(index) >= points_.size()) return kInvalidArgument;
        offset = points_[static_cast<std::size_t>(index)].first;
        value = points_[static_cast<std::size_t>(index)].second;
        return kResultOk;
    }
    tresult PLUGIN_API addPoint(int32 offset, Vst::ParamValue value, int32& index) override {
        if (offset < 0 || !std::isfinite(value) || value < 0.0 || value > 1.0) {
            return kInvalidArgument;
        }
        points_.emplace_back(offset, value);
        index = static_cast<int32>(points_.size() - 1);
        return kResultOk;
    }

private:
    Vst::ParamID id_;
    std::vector<std::pair<int32, Vst::ParamValue>> points_;
};

class ParameterChanges final : public Vst::IParameterChanges {
public:
    tresult PLUGIN_API queryInterface(const TUID, void** object) override {
        if (object) *object = nullptr;
        return kNoInterface;
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    int32 PLUGIN_API getParameterCount() override { return static_cast<int32>(queues_.size()); }
    Vst::IParamValueQueue* PLUGIN_API getParameterData(int32 index) override {
        if (index < 0 || static_cast<std::size_t>(index) >= queues_.size()) return nullptr;
        return &queues_[static_cast<std::size_t>(index)];
    }
    Vst::IParamValueQueue* PLUGIN_API addParameterData(const Vst::ParamID& id,
                                                       int32& index) override {
        for (std::size_t i = 0; i < queues_.size(); ++i) {
            if (queues_[i].getParameterId() == id) {
                index = static_cast<int32>(i);
                return &queues_[i];
            }
        }
        if (queues_.size() >= kMaxParameters) return nullptr;
        queues_.emplace_back(id);
        index = static_cast<int32>(queues_.size() - 1);
        return &queues_.back();
    }

private:
    std::vector<ParameterQueue> queues_;
};

static fs::path resolve_binary(const fs::path& supplied) {
    if (!fs::is_directory(supplied)) return supplied;
    const auto contents = supplied / "Contents" / "x86_64-win";
    fs::path result;
    for (const auto& entry : fs::directory_iterator(contents)) {
        if (entry.is_regular_file()) {
            if (!result.empty()) throw std::runtime_error("VST3 bundle has multiple binaries");
            result = entry.path();
        }
    }
    if (result.empty()) throw std::runtime_error("VST3 bundle has no x64 binary");
    return result;
}

static void require_result(const char* operation, tresult result) {
    if (result == kResultOk || result == kNotImplemented) return;
    std::ostringstream message;
    message << operation << " failed with VST3 result 0x" << std::hex
            << static_cast<uint32>(result);
    throw std::runtime_error(message.str());
}

class HostApplication final : public Vst::IHostApplication {
public:
    tresult PLUGIN_API queryInterface(const TUID, void** object) override {
        if (object) *object = nullptr;
        return kNoInterface;
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }

    tresult PLUGIN_API getName(Vst::String128 name) override {
        if (!name) return kInvalidArgument;
        const char16_t value[] = u"AudioRouter";
        std::copy(std::begin(value), std::end(value), name);
        return kResultOk;
    }

    tresult PLUGIN_API createInstance(TUID, TUID, void** object) override {
        if (object) *object = nullptr;
        return kNotImplemented;
    }
};

// Parameter edits made in the plugin's own window. The UI thread adds them;
// the audio thread hands them to the processor with the next quantum, as a
// DAW does. One pending value per parameter, bounded like host events.
class EditorEditQueue final {
public:
    void push(Vst::ParamID id, Vst::ParamValue value) {
        if (!std::isfinite(value)) return;
        value = std::clamp(value, 0.0, 1.0);
        std::lock_guard<std::mutex> lock(mutex_);
        for (auto& edit : edits_) {
            if (edit.first == id) {
                edit.second = value;
                return;
            }
        }
        if (edits_.size() < kMaxParameters) edits_.emplace_back(id, value);
    }

    std::vector<std::pair<Vst::ParamID, Vst::ParamValue>> take() {
        std::lock_guard<std::mutex> lock(mutex_);
        return std::exchange(edits_, {});
    }

    // Put back edits that did not fit in a quantum. A newer edit of the same
    // parameter, made meanwhile, is kept.
    void restore(const std::vector<std::pair<Vst::ParamID, Vst::ParamValue>>& leftover) {
        std::lock_guard<std::mutex> lock(mutex_);
        for (const auto& edit : leftover) {
            const bool newer = std::any_of(edits_.begin(), edits_.end(),
                                           [&](const auto& pending) { return pending.first == edit.first; });
            if (!newer && edits_.size() < kMaxParameters) edits_.push_back(edit);
        }
    }

private:
    std::mutex mutex_;
    std::vector<std::pair<Vst::ParamID, Vst::ParamValue>> edits_;
};

static EditorEditQueue g_editor_edits;

// The plugin's own delay. The host asks for it about twice a second, so the
// pipe thread answers from this cache instead of waiting for a UI thread that
// a busy plugin window may hold. `restartComponent(kLatencyChanged)` and every
// other UI request (state, editor) invalidate it; the next Latency request
// then goes to the UI thread, which re-activates the component if the plugin
// asked for it and stores the fresh value.
static std::atomic<bool> g_latency_restart_pending{false};
static std::atomic<bool> g_latency_cached{false};
static std::atomic<uint32_t> g_latency_samples{0};

class ComponentHandler final : public Vst::IComponentHandler {
public:
    tresult PLUGIN_API queryInterface(const TUID, void** object) override {
        if (object) *object = nullptr;
        return kNoInterface;
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    tresult PLUGIN_API beginEdit(Vst::ParamID) override { return kResultOk; }
    tresult PLUGIN_API performEdit(Vst::ParamID id, Vst::ParamValue value) override {
        g_editor_edits.push(id, value);
        return kResultOk;
    }
    tresult PLUGIN_API endEdit(Vst::ParamID) override { return kResultOk; }
    // Called by the controller, usually on the UI thread. Only a latency
    // change needs host action; the SDK requires a re-activation before the
    // new value applies, done between quanta by the next Latency request.
    tresult PLUGIN_API restartComponent(int32 flags) override {
        if (flags & Vst::kLatencyChanged) g_latency_restart_pending.store(true, std::memory_order_release);
        return kResultOk;
    }
};

class StateStream final : public IBStream {
public:
    StateStream() = default;
    StateStream(const uint8_t* data, std::size_t size) : bytes_(data, data + size) {}

    tresult PLUGIN_API queryInterface(const TUID, void** object) override {
        if (object) *object = nullptr;
        return kNoInterface;
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }

    tresult PLUGIN_API read(void* data, int32 count, int32* read_count = nullptr) override {
        if (count < 0 || (!data && count != 0)) return kInvalidArgument;
        const auto available = bytes_.size() > cursor_ ? bytes_.size() - cursor_ : 0;
        const auto amount = std::min<std::size_t>(available, static_cast<std::size_t>(count));
        if (amount) std::memcpy(data, bytes_.data() + cursor_, amount);
        cursor_ += amount;
        if (read_count) *read_count = static_cast<int32>(amount);
        // As the SDK's MemoryStream: a short read at the end succeeds and
        // reports the count. Plugins that read their state in chunks until a
        // short read (UADx) reject the whole state on kResultFalse.
        return kResultOk;
    }

    tresult PLUGIN_API write(void* data, int32 count, int32* written_count = nullptr) override {
        if (count < 0 || (!data && count != 0)) return kInvalidArgument;
        const auto amount = static_cast<std::size_t>(count);
        if (cursor_ > kMaxStateBytes || amount > kMaxStateBytes - cursor_) return kOutOfMemory;
        if (cursor_ + amount > bytes_.size()) bytes_.resize(cursor_ + amount);
        if (amount) std::memcpy(bytes_.data() + cursor_, data, amount);
        cursor_ += amount;
        if (written_count) *written_count = count;
        return kResultOk;
    }

    tresult PLUGIN_API seek(int64 offset, int32 mode, int64* result = nullptr) override {
        int64 base = 0;
        if (mode == kIBSeekCur) base = static_cast<int64>(cursor_);
        else if (mode == kIBSeekEnd) base = static_cast<int64>(bytes_.size());
        else if (mode != kIBSeekSet) return kInvalidArgument;
        if (offset < -base || base + offset < 0 || base + offset > static_cast<int64>(kMaxStateBytes)) {
            return kInvalidArgument;
        }
        cursor_ = static_cast<std::size_t>(base + offset);
        if (result) *result = static_cast<int64>(cursor_);
        return kResultOk;
    }

    tresult PLUGIN_API tell(int64* position) override {
        if (!position) return kInvalidArgument;
        *position = static_cast<int64>(cursor_);
        return kResultOk;
    }

    const std::vector<uint8_t>& bytes() const { return bytes_; }

private:
    std::vector<uint8_t> bytes_;
    std::size_t cursor_ = 0;
};

class Vst3Effect final {
public:
    Vst3Effect(const fs::path& supplied, double sample_rate,
               const std::vector<uint16_t>& input_channels,
               const std::vector<uint16_t>& output_channels)
        : requested_input_channels_(input_channels), requested_output_channels_(output_channels) {
        if (input_channels.empty() || output_channels.empty() || input_channels.size() > 4 ||
            output_channels.size() > 4 ||
            std::accumulate(input_channels.begin(), input_channels.end(), std::size_t{0}) > 8 ||
            std::accumulate(output_channels.begin(), output_channels.end(), std::size_t{0}) > 8) {
            throw protocol_error("native VST3 bus layout is outside the bounded contract");
        }
        const auto binary = fs::absolute(resolve_binary(supplied));
        module_ = LoadLibraryExW(binary.c_str(), nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
                                                               LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
        if (!module_) throw std::runtime_error("VST3 LoadLibraryExW failed");
        if (const auto init = reinterpret_cast<ModuleEntryProc>(GetProcAddress(module_, "InitDll"))) {
            if (!init()) throw std::runtime_error("VST3 InitDll failed");
            module_initialized_ = true;
        }
        const auto get_factory = reinterpret_cast<GetPluginFactoryProc>(
            GetProcAddress(module_, "GetPluginFactory"));
        if (!get_factory) throw std::runtime_error("VST3 GetPluginFactory export is missing");
        factory_ = get_factory();
        if (!factory_) throw std::runtime_error("VST3 factory is null");
        std::ostringstream class_failures;
        bool found_audio_class = false;
        bool found_instrument = false;
        IPluginFactory2* factory2 = nullptr;
        factory_->queryInterface(IPluginFactory2_iid, reinterpret_cast<void**>(&factory2));
        for (int32 index = 0; index < factory_->countClasses(); ++index) {
            PClassInfo info{};
            require_result("getClassInfo", factory_->getClassInfo(index, &info));
            if (std::strcmp(info.category, kVstAudioEffectClass) != 0) continue;
            // Instruments share the audio-module category; their
            // sub-categories say "Instrument". AudioRouter has no MIDI input.
            PClassInfo2 info2{};
            if (factory2 && factory2->getClassInfo2(index, &info2) == kResultOk &&
                std::strstr(info2.subCategories, "Instrument") != nullptr) {
                found_instrument = true;
                continue;
            }
            found_audio_class = true;
            if (factory_->createInstance(info.cid, Vst::IComponent_iid,
                                         reinterpret_cast<void**>(&component_)) != kResultOk) {
                class_failures << " class " << index << " (" << info.name
                                << "): component creation rejected;";
                continue;
            }
            const auto initialize_result = component_->initialize(&host_application_);
            if (initialize_result != kResultOk) {
                class_failures << " class " << index << " (" << info.name
                                << "): initialize returned 0x" << std::hex
                                << static_cast<uint32>(initialize_result) << ";";
                component_->release();
                component_ = nullptr;
                continue;
            }
            const auto input_bus_count = component_->getBusCount(Vst::kAudio, Vst::kInput);
            const auto output_bus_count = component_->getBusCount(Vst::kAudio, Vst::kOutput);
            if (input_bus_count < static_cast<int32>(input_channels.size()) ||
                output_bus_count < static_cast<int32>(output_channels.size())) {
                class_failures << " class " << index << " (" << info.name
                                << "): has " << input_bus_count << "/" << output_bus_count
                                << " buses, requested " << input_channels.size() << "/"
                                << output_channels.size() << ";";
                component_->terminate();
                component_->release();
                component_ = nullptr;
                continue;
            }
            break;
        }
        if (factory2) factory2->release();
        if (!component_) {
            if (!found_audio_class && found_instrument) {
                throw std::runtime_error(
                    "VST3 plugin is an instrument; AudioRouter hosts audio effects only");
            }
            if (!found_audio_class) {
                throw std::runtime_error("VST3 factory has no audio effect class");
            }
            throw std::runtime_error("VST3 audio effect classes were unusable:" +
                                     class_failures.str());
        }
        initialized_ = true;
        TUID controller_id{};
        const auto controller_id_result = component_->getControllerClassId(controller_id);
        if (controller_id_result == kResultOk) {
            const auto controller_result = factory_->createInstance(
                controller_id, Vst::IEditController_iid,
                reinterpret_cast<void**>(&controller_));
            if (controller_result == kResultOk) {
                const auto initialize_result = controller_->initialize(&host_application_);
                if (initialize_result == kResultOk) {
                    controller_initialized_ = true;
                    require_result("controller component handler", controller_->setComponentHandler(
                        &component_handler_));
                    component_->queryInterface(Vst::IConnectionPoint::iid,
                                                reinterpret_cast<void**>(&component_connection_));
                    controller_->queryInterface(Vst::IConnectionPoint::iid,
                                                reinterpret_cast<void**>(&controller_connection_));
                    if (component_connection_ && controller_connection_ &&
                        component_connection_->connect(controller_connection_) == kResultTrue &&
                        controller_connection_->connect(component_connection_) == kResultTrue) {
                        components_connected_ = true;
                    }
                } else {
                    controller_->release();
                    controller_ = nullptr;
                }
            }
        } else if (controller_id_result != kNotImplemented && controller_id_result != kNoInterface) {
            require_result("controller class id", controller_id_result);
        }
        require_result("processor interface", component_->queryInterface(
            Vst::IAudioProcessor_iid, reinterpret_cast<void**>(&processor_)));
        // A single-stream node is mono or stereo, and so is almost every
        // effect, but not always the same way (UADx effects are stereo-only).
        // Ask for the node's layout; if the plugin keeps its own, adapt
        // between them in process().
        auto effective_inputs = input_channels;
        auto effective_outputs = output_channels;
        if (input_channels.size() == 1 && output_channels.size() == 1) {
            negotiate_main_bus(input_channels[0], output_channels[0]);
            effective_inputs[0] = plugin_input_channels_;
            effective_outputs[0] = plugin_output_channels_;
        }
        for (std::size_t bus = 0; bus < input_channels.size(); ++bus) {
            Vst::BusInfo info{};
            require_result("input bus info", component_->getBusInfo(
                Vst::kAudio, Vst::kInput, static_cast<int32>(bus), info));
            if (info.channelCount != effective_inputs[bus]) {
                throw std::runtime_error("native VST3 input bus " + std::to_string(bus) +
                                         " declares " + std::to_string(info.channelCount) +
                                         " channels, layout requests " +
                                         std::to_string(input_channels[bus]));
            }
            require_result("input bus activation", component_->activateBus(
                Vst::kAudio, Vst::kInput, static_cast<int32>(bus), true));
        }
        for (std::size_t bus = 0; bus < output_channels.size(); ++bus) {
            Vst::BusInfo info{};
            require_result("output bus info", component_->getBusInfo(
                Vst::kAudio, Vst::kOutput, static_cast<int32>(bus), info));
            if (info.channelCount != effective_outputs[bus]) {
                throw std::runtime_error("native VST3 output bus channels do not match layout");
            }
            require_result("output bus activation", component_->activateBus(
                Vst::kAudio, Vst::kOutput, static_cast<int32>(bus), true));
        }
        // A plugin may expose optional side-chain or auxiliary buses. The
        // bounded transport carries the requested main buses only; explicitly
        // disable the remainder so the component and ProcessData agree.
        const auto input_bus_count = component_->getBusCount(Vst::kAudio, Vst::kInput);
        for (int32 bus = static_cast<int32>(input_channels.size()); bus < input_bus_count; ++bus) {
            require_result("optional input bus deactivation", component_->activateBus(
                Vst::kAudio, Vst::kInput, bus, false));
        }
        const auto output_bus_count = component_->getBusCount(Vst::kAudio, Vst::kOutput);
        for (int32 bus = static_cast<int32>(output_channels.size()); bus < output_bus_count; ++bus) {
            require_result("optional output bus deactivation", component_->activateBus(
                Vst::kAudio, Vst::kOutput, bus, false));
        }
        Vst::ProcessSetup setup{};
        setup.processMode = Vst::kRealtime;
        setup.symbolicSampleSize = Vst::kSample32;
        setup.maxSamplesPerBlock = static_cast<int32>(kMaxFrames);
        setup.sampleRate = sample_rate;
        require_result("setupProcessing", processor_->setupProcessing(setup));
        require_result("component activation", component_->setActive(true));
        active_ = true;
        require_result("processor activation", processor_->setProcessing(true));
        processing_ = true;
    }

    ~Vst3Effect() {
        if (processing_) processor_->setProcessing(false);
        if (active_) component_->setActive(false);
        if (components_connected_) {
            component_connection_->disconnect(controller_connection_);
            controller_connection_->disconnect(component_connection_);
        }
        if (component_connection_) component_connection_->release();
        if (controller_connection_) controller_connection_->release();
        if (processor_) processor_->release();
        if (controller_) {
            if (controller_initialized_) controller_->terminate();
            controller_->release();
        }
        if (component_) {
            if (initialized_) component_->terminate();
            component_->release();
        }
        if (factory_) factory_->release();
        if (module_initialized_) {
            if (const auto exit = reinterpret_cast<ModuleEntryProc>(GetProcAddress(module_, "ExitDll"))) exit();
        }
        if (module_) FreeLibrary(module_);
    }

    std::vector<uint8_t> save_state() {
        StateStream component_stream;
        StateStream controller_stream;
        require_result("VST3 component getState", component_->getState(&component_stream));
        if (controller_) require_result("VST3 controller getState", controller_->getState(&controller_stream));
        const auto& component_state = component_stream.bytes();
        const auto& controller_state = controller_stream.bytes();
        if (component_state.size() > UINT32_MAX || controller_state.size() > UINT32_MAX ||
            component_state.size() + controller_state.size() + 16 > kMaxStateBytes) {
            throw protocol_error("VST3 state exceeds the bounded contract");
        }
        std::vector<uint8_t> state;
        state.reserve(16 + component_state.size() + controller_state.size());
        state.insert(state.end(), {'A', 'R', 'S', 'T', '1', 0, 0, 0});
        append_u32(state, static_cast<uint32_t>(component_state.size()));
        append_u32(state, static_cast<uint32_t>(controller_state.size()));
        state.insert(state.end(), component_state.begin(), component_state.end());
        state.insert(state.end(), controller_state.begin(), controller_state.end());
        return state;
    }

    void restore_state(const std::vector<uint8_t>& bytes) {
        if (bytes.empty() || bytes.size() > kMaxStateBytes) {
            throw protocol_error("VST3 state is empty or exceeds the bounded contract");
        }
        std::vector<uint8_t> component_state;
        std::vector<uint8_t> controller_state;
        if (bytes.size() >= 16 && std::equal(bytes.begin(), bytes.begin() + 8,
                                             std::array<uint8_t, 8>{'A', 'R', 'S', 'T', '1', 0, 0, 0}.begin())) {
            const auto component_size = read_u32(bytes, 8);
            const auto controller_size = read_u32(bytes, 12);
            if (component_size > kMaxStateBytes || controller_size > kMaxStateBytes ||
                static_cast<std::size_t>(component_size) + controller_size + 16 != bytes.size()) {
                throw protocol_error("VST3 state envelope is malformed");
            }
            component_state.assign(bytes.begin() + 16, bytes.begin() + 16 + component_size);
            controller_state.assign(bytes.begin() + 16 + component_size, bytes.end());
        } else {
            component_state = bytes;
        }
        if (component_state.empty()) throw protocol_error("VST3 component state is empty");
        const bool was_active = active_;
        const bool was_processing = processing_;
        if (was_processing) {
            require_result("VST3 processor pause for state", processor_->setProcessing(false));
            processing_ = false;
        }
        if (was_active) {
            require_result("VST3 component pause for state", component_->setActive(false));
            active_ = false;
        }
        try {
            // The SDK's host order: the component takes its state first, then
            // the controller mirrors it with setComponentState.
            StateStream component_stream(component_state.data(), component_state.size());
            require_result("VST3 component setState", component_->setState(&component_stream));
            if (controller_) {
                StateStream controller_component_stream(component_state.data(), component_state.size());
                require_result("VST3 controller component state", controller_->setComponentState(
                    &controller_component_stream));
            }
            if (!controller_state.empty()) {
                if (!controller_) throw protocol_error("VST3 controller state is unavailable");
                StateStream controller_stream(controller_state.data(), controller_state.size());
                require_result("VST3 controller setState", controller_->setState(&controller_stream));
            }
        } catch (...) {
            // A plugin may reject an otherwise valid host snapshot. Restore
            // the lifecycle before reporting the capability limitation so a
            // subsequent quantum is not processed by a half-paused instance.
            if (was_active) {
                require_result("VST3 component resume after rejected state", component_->setActive(true));
                active_ = true;
            }
            if (was_processing) {
                require_result("VST3 processor resume after rejected state", processor_->setProcessing(true));
                processing_ = true;
            }
            throw;
        }
        if (was_active) {
            require_result("VST3 component resume after state", component_->setActive(true));
            active_ = true;
        }
        if (was_processing) {
            require_result("VST3 processor resume after state", processor_->setProcessing(true));
            processing_ = true;
        }
    }

    std::vector<float> process(const std::vector<float>& samples, uint16_t channels,
                               const std::vector<ParameterEvent>& events) {
        if (samples.size() % channels != 0) throw protocol_error("interleaved sample shape is invalid");
        const auto frames = samples.size() / channels;
        if (frames == 0 || frames > kMaxFrames) throw protocol_error("frame count is invalid");
        // Adapt the node's channels to the plugin's main bus: a mono node
        // feeds both inputs of a stereo-only effect, and a stereo node is
        // summed at half gain into a mono-only one (and the reverse on output).
        std::vector<std::vector<float>> input(plugin_input_channels_, std::vector<float>(frames));
        std::vector<std::vector<float>> output(plugin_output_channels_, std::vector<float>(frames));
        for (std::size_t frame = 0; frame < frames; ++frame) {
            const float* host = &samples[frame * channels];
            if (plugin_input_channels_ == channels) {
                for (uint16_t channel = 0; channel < channels; ++channel) input[channel][frame] = host[channel];
            } else if (plugin_input_channels_ == 2) {
                input[0][frame] = host[0];
                input[1][frame] = host[0];
            } else {
                input[0][frame] = 0.5f * (host[0] + host[1]);
            }
        }
        std::vector<ParameterEvent> ordered = events;
        std::stable_sort(ordered.begin(), ordered.end(),
                         [](const auto& left, const auto& right) { return left.offset < right.offset; });
        std::size_t start = 0;
        std::size_t event_index = 0;
        while (event_index < ordered.size()) {
            const auto offset = ordered[event_index].offset;
            if (offset >= frames) throw protocol_error("parameter offset exceeds frame count");
            process_segment(input, output, start, offset, ordered, event_index);
            start = offset;
        }
        process_segment(input, output, start, frames, ordered, event_index);
        std::vector<float> result(samples.size());
        for (std::size_t frame = 0; frame < frames; ++frame) {
            for (uint16_t channel = 0; channel < channels; ++channel) {
                float sample;
                if (plugin_output_channels_ == channels) {
                    sample = output[channel][frame];
                } else if (plugin_output_channels_ == 2) {
                    sample = 0.5f * (output[0][frame] + output[1][frame]);
                } else {
                    sample = output[0][frame];
                }
                if (!std::isfinite(sample)) throw std::runtime_error("VST3 produced non-finite output");
                result[frame * channels + channel] = sample;
            }
        }
        return result;
    }

    std::vector<std::vector<float>> process_buses(
        const std::vector<BusFrame>& frames, const std::vector<ParameterEvent>& events) {
        if (frames.size() != requested_input_channels_.size()) {
            throw protocol_error("native VST3 input bus count does not match layout");
        }
        if (frames.empty()) throw protocol_error("native VST3 input bus set is empty");
        const auto frame_count = frames.front().samples.size() / frames.front().channels;
        if (frames.front().channels != requested_input_channels_[0] || frame_count == 0 ||
            frame_count > kMaxFrames || frames.front().samples.size() % frames.front().channels != 0) {
            throw protocol_error("native VST3 main bus shape is invalid");
        }
        for (std::size_t bus = 0; bus < frames.size(); ++bus) {
            if (frames[bus].sequence != frames.front().sequence ||
                frames[bus].deadline != frames.front().deadline ||
                frames[bus].channels != requested_input_channels_[bus] ||
                frames[bus].samples.size() / frames[bus].channels != frame_count ||
                frames[bus].samples.size() % frames[bus].channels != 0) {
                throw protocol_error("native VST3 input buses do not share quantum identity");
            }
        }
        std::vector<std::vector<std::vector<float>>> input;
        std::vector<std::vector<std::vector<float>>> output;
        for (std::size_t bus = 0; bus < frames.size(); ++bus) {
            input.emplace_back(requested_input_channels_[bus], std::vector<float>(frame_count));
            for (std::size_t index = 0; index < frames[bus].samples.size(); ++index) {
                input[bus][index % requested_input_channels_[bus]][index / requested_input_channels_[bus]] =
                    frames[bus].samples[index];
            }
        }
        for (const auto channels : requested_output_channels_) {
            output.emplace_back(channels, std::vector<float>(frame_count));
        }
        std::vector<ParameterEvent> ordered = events;
        std::stable_sort(ordered.begin(), ordered.end(),
                         [](const auto& left, const auto& right) { return left.offset < right.offset; });
        std::size_t start = 0;
        std::size_t event_index = 0;
        while (event_index < ordered.size()) {
            const auto offset = ordered[event_index].offset;
            if (offset >= frame_count) throw protocol_error("parameter offset exceeds frame count");
            process_multi_segment(input, output, start, offset, ordered, event_index);
            start = offset;
        }
        process_multi_segment(input, output, start, frame_count, ordered, event_index);
        std::vector<std::vector<float>> result;
        for (std::size_t bus = 0; bus < output.size(); ++bus) {
            result.emplace_back(output[bus].size() * frame_count);
            for (std::size_t index = 0; index < result.back().size(); ++index) {
                const auto sample = output[bus][index % output[bus].size()][index / output[bus].size()];
                if (!std::isfinite(sample)) throw std::runtime_error("VST3 produced non-finite output");
                result.back()[index] = sample;
            }
        }
        return result;
    }

    Vst::IEditController* controller() const { return controller_; }

    uint32_t latency_samples() const { return processor_ ? processor_->getLatencySamples() : 0; }

    // The SDK applies a reported latency change only after the component is
    // deactivated and activated again. UI thread, between quanta: the pipe
    // thread waits for this request, so no Process call runs meanwhile.
    void restart_for_latency() {
        if (processing_) {
            require_result("VST3 processor pause for latency", processor_->setProcessing(false));
            processing_ = false;
        }
        if (active_) {
            require_result("VST3 component pause for latency", component_->setActive(false));
            active_ = false;
        }
        require_result("VST3 component resume after latency", component_->setActive(true));
        active_ = true;
        require_result("VST3 processor resume after latency", processor_->setProcessing(true));
        processing_ = true;
    }

private:
    static void append_u32(std::vector<uint8_t>& output, uint32_t value) {
        output.push_back(static_cast<uint8_t>(value & 0xff));
        output.push_back(static_cast<uint8_t>((value >> 8) & 0xff));
        output.push_back(static_cast<uint8_t>((value >> 16) & 0xff));
        output.push_back(static_cast<uint8_t>((value >> 24) & 0xff));
    }

    static uint32_t read_u32(const std::vector<uint8_t>& input, std::size_t offset) {
        return static_cast<uint32_t>(input[offset]) |
               (static_cast<uint32_t>(input[offset + 1]) << 8) |
               (static_cast<uint32_t>(input[offset + 2]) << 16) |
               (static_cast<uint32_t>(input[offset + 3]) << 24);
    }

    static Vst::SpeakerArrangement arrangement_for(uint16_t channels) {
        return channels == 1 ? Vst::SpeakerArr::kMono : Vst::SpeakerArr::kStereo;
    }

    // Sets plugin_input_channels_/plugin_output_channels_ for the main bus.
    void negotiate_main_bus(uint16_t requested_input, uint16_t requested_output) {
        Vst::BusInfo input_info{};
        Vst::BusInfo output_info{};
        require_result("input bus info", component_->getBusInfo(Vst::kAudio, Vst::kInput, 0, input_info));
        require_result("output bus info", component_->getBusInfo(Vst::kAudio, Vst::kOutput, 0, output_info));
        if (input_info.channelCount != requested_input || output_info.channelCount != requested_output) {
            std::vector<Vst::SpeakerArrangement> inputs(
                static_cast<std::size_t>(component_->getBusCount(Vst::kAudio, Vst::kInput)));
            std::vector<Vst::SpeakerArrangement> outputs(
                static_cast<std::size_t>(component_->getBusCount(Vst::kAudio, Vst::kOutput)));
            for (std::size_t bus = 0; bus < inputs.size(); ++bus) {
                processor_->getBusArrangement(Vst::kInput, static_cast<int32>(bus), inputs[bus]);
            }
            for (std::size_t bus = 0; bus < outputs.size(); ++bus) {
                processor_->getBusArrangement(Vst::kOutput, static_cast<int32>(bus), outputs[bus]);
            }
            inputs[0] = arrangement_for(requested_input);
            outputs[0] = arrangement_for(requested_output);
            // A refusal is normal; the plugin then keeps its own layout.
            processor_->setBusArrangements(inputs.data(), static_cast<int32>(inputs.size()),
                                           outputs.data(), static_cast<int32>(outputs.size()));
            require_result("input bus info", component_->getBusInfo(Vst::kAudio, Vst::kInput, 0, input_info));
            require_result("output bus info", component_->getBusInfo(Vst::kAudio, Vst::kOutput, 0, output_info));
        }
        if (input_info.channelCount < 1 || input_info.channelCount > 2 ||
            output_info.channelCount < 1 || output_info.channelCount > 2) {
            throw std::runtime_error("native VST3 main bus is neither mono nor stereo (" +
                                     std::to_string(input_info.channelCount) + " in, " +
                                     std::to_string(output_info.channelCount) + " out)");
        }
        plugin_input_channels_ = static_cast<uint16_t>(input_info.channelCount);
        plugin_output_channels_ = static_cast<uint16_t>(output_info.channelCount);
    }

    void process_segment(const std::vector<std::vector<float>>& input,
                         std::vector<std::vector<float>>& output, std::size_t start, std::size_t end,
                         const std::vector<ParameterEvent>& events, std::size_t& event_index) {
        while (event_index < events.size() && events[event_index].offset == start) {
            int32 queue_index = -1;
            auto* queue = changes_.addParameterData(events[event_index].id, queue_index);
            if (!queue) throw protocol_error("parameter queue limit exceeded");
            int32 point_index = -1;
            if (queue->addPoint(static_cast<int32>(events[event_index].offset - start),
                                events[event_index].value, point_index) != kResultOk) {
                throw protocol_error("parameter event could not be queued");
            }
            ++event_index;
        }
        if (start == end) return;
        std::vector<Vst::Sample32*> inputs;
        std::vector<Vst::Sample32*> outputs;
        for (auto& channel : input) inputs.push_back(const_cast<float*>(&channel[start]));
        for (auto& channel : output) outputs.push_back(&channel[start]);
        Vst::AudioBusBuffers input_bus{};
        input_bus.numChannels = static_cast<int32>(inputs.size());
        input_bus.channelBuffers32 = inputs.data();
        Vst::AudioBusBuffers output_bus{};
        output_bus.numChannels = static_cast<int32>(outputs.size());
        output_bus.channelBuffers32 = outputs.data();
        Vst::ProcessData data{};
        data.processMode = Vst::kRealtime;
        data.symbolicSampleSize = Vst::kSample32;
        data.numSamples = static_cast<int32>(end - start);
        data.numInputs = 1;
        data.numOutputs = 1;
        data.inputs = &input_bus;
        data.outputs = &output_bus;
        data.inputParameterChanges = changes_.getParameterCount() > 0 ? &changes_ : nullptr;
        require_result("VST3 process", processor_->process(data));
        changes_ = ParameterChanges{};
    }

    void process_multi_segment(
        const std::vector<std::vector<std::vector<float>>>& input,
        std::vector<std::vector<std::vector<float>>>& output, std::size_t start, std::size_t end,
        const std::vector<ParameterEvent>& events, std::size_t& event_index) {
        while (event_index < events.size() && events[event_index].offset == start) {
            int32 queue_index = -1;
            auto* queue = changes_.addParameterData(events[event_index].id, queue_index);
            if (!queue) throw protocol_error("parameter queue limit exceeded");
            int32 point_index = -1;
            if (queue->addPoint(static_cast<int32>(events[event_index].offset - start),
                                events[event_index].value, point_index) != kResultOk) {
                throw protocol_error("parameter event could not be queued");
            }
            ++event_index;
        }
        if (start == end) return;
        std::vector<std::vector<Vst::Sample32*>> input_channels;
        std::vector<std::vector<Vst::Sample32*>> output_channels;
        std::vector<Vst::AudioBusBuffers> input_buses;
        std::vector<Vst::AudioBusBuffers> output_buses;
        for (std::size_t bus = 0; bus < input.size(); ++bus) {
            input_channels.emplace_back();
            for (auto& channel : input[bus]) input_channels.back().push_back(const_cast<float*>(&channel[start]));
            Vst::AudioBusBuffers buffers{};
            buffers.numChannels = static_cast<int32>(input[bus].size());
            buffers.channelBuffers32 = input_channels.back().data();
            input_buses.push_back(buffers);
        }
        for (std::size_t bus = 0; bus < output.size(); ++bus) {
            output_channels.emplace_back();
            for (auto& channel : output[bus]) output_channels.back().push_back(&channel[start]);
            Vst::AudioBusBuffers buffers{};
            buffers.numChannels = static_cast<int32>(output[bus].size());
            buffers.channelBuffers32 = output_channels.back().data();
            output_buses.push_back(buffers);
        }
        Vst::ProcessData data{};
        data.processMode = Vst::kRealtime;
        data.symbolicSampleSize = Vst::kSample32;
        data.numSamples = static_cast<int32>(end - start);
        data.numInputs = static_cast<int32>(input_buses.size());
        data.numOutputs = static_cast<int32>(output_buses.size());
        data.inputs = input_buses.data();
        data.outputs = output_buses.data();
        data.inputParameterChanges = changes_.getParameterCount() > 0 ? &changes_ : nullptr;
        require_result("VST3 multi-bus process", processor_->process(data));
        changes_ = ParameterChanges{};
    }

    HMODULE module_ = nullptr;
    bool module_initialized_ = false;
    HostApplication host_application_;
    ComponentHandler component_handler_;
    IPluginFactory* factory_ = nullptr;
    Vst::IComponent* component_ = nullptr;
    Vst::IAudioProcessor* processor_ = nullptr;
    Vst::IEditController* controller_ = nullptr;
    Vst::IConnectionPoint* component_connection_ = nullptr;
    Vst::IConnectionPoint* controller_connection_ = nullptr;
    bool initialized_ = false;
    bool controller_initialized_ = false;
    bool components_connected_ = false;
    bool active_ = false;
    bool processing_ = false;
    std::vector<uint16_t> requested_input_channels_;
    std::vector<uint16_t> requested_output_channels_;
    uint16_t plugin_input_channels_ = 0;
    uint16_t plugin_output_channels_ = 0;
    ParameterChanges changes_;
};

static std::string quote_json(const std::string& value) {
    std::string result = "\"";
    for (const auto character : value) {
        switch (character) {
            case '\\': result += "\\\\"; break;
            case '"': result += "\\\""; break;
            case '\b': result += "\\b"; break;
            case '\f': result += "\\f"; break;
            case '\n': result += "\\n"; break;
            case '\r': result += "\\r"; break;
            case '\t': result += "\\t"; break;
            default:
                if (static_cast<unsigned char>(character) < 0x20) {
                    constexpr char hex[] = "0123456789abcdef";
                    const auto byte = static_cast<unsigned char>(character);
                    result += "\\u00";
                    result.push_back(hex[byte >> 4]);
                    result.push_back(hex[byte & 0x0f]);
                } else {
                    result.push_back(character);
                }
                break;
        }
    }
    result.push_back('"');
    return result;
}

static std::string quote_vst3_title(const Vst::String128& title) {
    std::string value;
    for (const auto code_unit : title) {
        if (code_unit == 0) break;
        const auto character = static_cast<unsigned int>(code_unit);
        value.push_back(character >= 0x20 && character <= 0x7e
                            ? static_cast<char>(character)
                            : '?');
    }
    if (value.empty()) value = "Parameter";
    return quote_json(value);
}

static std::string parameter_descriptors_message(Vst::IEditController* controller) {
    if (!controller) return "{\"type\":\"Parameters\",\"payload\":{\"descriptors\":[]}}";
    const auto count = controller->getParameterCount();
    if (count < 0 || count > static_cast<int32>(kMaxScannedParameters)) {
        throw protocol_error("VST3 parameter count exceeds the bounded scan");
    }
    std::ostringstream output;
    std::unordered_set<Vst::ParamID> parameter_ids;
    std::size_t exposed = 0;
    output << "{\"type\":\"Parameters\",\"payload\":{\"descriptors\":[";
    for (int32 index = 0; index < count; ++index) {
        Vst::ParameterInfo info{};
        require_result("VST3 parameter info", controller->getParameterInfo(index, info));
        if (!parameter_ids.insert(info.id).second) {
            throw protocol_error("VST3 parameter IDs are not unique");
        }
        // Many plugins (UADx among them) declare hundreds of internal,
        // hidden or read-only parameters. Expose only the user-automatable
        // ones, up to the worker contract's descriptor bound.
        constexpr int32 kNotUserFacing = Vst::ParameterInfo::kIsHidden |
                                         Vst::ParameterInfo::kIsReadOnly |
                                         Vst::ParameterInfo::kIsProgramChange;
        if (!(info.flags & Vst::ParameterInfo::kCanAutomate) || (info.flags & kNotUserFacing)) {
            continue;
        }
        if (exposed == kMaxParameters) break;
        if (!std::isfinite(info.defaultNormalizedValue) ||
            info.defaultNormalizedValue < 0.0 || info.defaultNormalizedValue > 1.0) {
            throw protocol_error("VST3 parameter default is outside the normalized contract");
        }
        if (exposed++ != 0) output << ',';
        output << "{\"parameter_id\":" << info.id << ",\"title\":"
               << quote_vst3_title(info.title) << ",\"default_value\":"
               << info.defaultNormalizedValue << ",\"minimum\":0,\"maximum\":1}";
    }
    output << "]}}";
    return output.str();
}

static std::string processed_message(uint64_t sequence, uint64_t deadline, uint16_t channels,
                                     const std::vector<float>& samples) {
    std::ostringstream output;
    output << "{\"type\":\"Processed\",\"payload\":{\"frame\":{";
    output << "\"sequence\":" << sequence << ",\"deadline_tick\":" << deadline
           << ",\"channels\":" << channels << ",\"samples\":[";
    output << std::setprecision(9);
    for (std::size_t index = 0; index < samples.size(); ++index) {
        if (index) output << ',';
        output << samples[index];
    }
    output << "]}}}";
    return output.str();
}

// A plugin without a Windows editor view answers editor requests with this
// bounded code; the host reports it as "no editor" and keeps processing.
static constexpr const char* kEditorUnavailableMessage =
    "{\"type\":\"Failure\",\"payload\":{\"code\":\"editorUnavailable\"}}";

static constexpr const char* kEditorParentInvalidMessage =
    "{\"type\":\"Failure\",\"payload\":{\"code\":\"editorParentInvalid\"}}";

static constexpr const char* kStateUnsupportedMessage =
    "{\"type\":\"Failure\",\"payload\":{\"code\":\"StateUnsupported\"}}";

static constexpr const char* kNoEditorMessage =
    "{\"type\":\"Editor\",\"payload\":{\"has_editor\":false,\"width\":0,\"height\":0}}";

static constexpr const char* kEditorClosedMessage = "{\"type\":\"EditorClosed\"}";

// Messages to the worker's UI thread, which owns every controller and view
// call (VST3 requires them on the thread that created the plugin).
constexpr UINT kUiRequestMessage = WM_APP + 1;      // lParam: UiRequest*, answered synchronously
constexpr UINT kHostParametersMessage = WM_APP + 2; // lParam: owned HostParameterValues*
constexpr UINT kAttachEditorMessage = WM_APP + 3;   // wParam: editor generation
constexpr UINT kReleaseEditorsMessage = WM_APP + 4;
constexpr UINT kAudioStoppedMessage = WM_APP + 5;
constexpr UINT_PTR kParentSizeTimer = 1;
constexpr UINT kParentSizePollMs = 200;
// A UI thread that does not answer a request within this time is stuck in
// plugin code; the worker then ends so the supervisor can contain it.
constexpr UINT kUiRequestTimeoutMs = 15000;
constexpr int32 kMaxEditorSide = 4096;

using HostParameterValues = std::vector<std::pair<Vst::ParamID, Vst::ParamValue>>;

static bool window_owned_by(HWND window, uint32_t process_id) {
    if (!window || !IsWindow(window)) return false;
    DWORD owner = 0;
    return GetWindowThreadProcessId(window, &owner) != 0 && owner == process_id;
}

// Hosts a plugin's own editor (IPlugView) inside the window the desktop shell
// authorized for it. UI thread only. Opening answers the host at once and
// attaches the view in a posted message, and closing detaches it the same
// way: the shell's window thread can be waiting on the host while the
// request is answered, and the plugin's child window would otherwise wait on
// that thread.
class EditorHost final : public IPlugFrame {
public:
    EditorHost(Vst::IEditController* controller, HWND ui_window)
        : controller_(controller), ui_window_(ui_window) {}

    ~EditorHost() {
        retire();
        release_retired();
    }

    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        if (!object) return kInvalidArgument;
        if (FUnknownPrivate::iidEqual(iid, IPlugFrame_iid) ||
            FUnknownPrivate::iidEqual(iid, FUnknown_iid)) {
            *object = static_cast<IPlugFrame*>(this);
            return kResultOk;
        }
        *object = nullptr;
        return kNoInterface;
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }

    tresult PLUGIN_API resizeView(IPlugView* view, ViewRect* rect) override {
        if (!view || view != view_ || !rect || !valid_size(*rect)) return kInvalidArgument;
        fit_parent(*rect);
        view->onSize(rect);
        last_client_width_ = rect->getWidth();
        last_client_height_ = rect->getHeight();
        return kResultTrue;
    }

    std::string describe() {
        ViewRect rect{};
        bool sized = false;
        if (view_) {
            sized = view_->getSize(&rect) == kResultOk;
        } else {
            IPlugView* view = create_view();
            if (!view) return kNoEditorMessage;
            sized = view->getSize(&rect) == kResultOk;
            view->release();
        }
        const bool fits = sized && valid_size(rect);
        return "{\"type\":\"Editor\",\"payload\":{\"has_editor\":true,\"width\":" +
               std::to_string(fits ? rect.getWidth() : 0) + ",\"height\":" +
               std::to_string(fits ? rect.getHeight() : 0) + "}}";
    }

    std::string open(uint64_t parent_window, uint32_t parent_process_id, const std::string& token) {
        const auto parent = reinterpret_cast<HWND>(static_cast<uintptr_t>(parent_window));
        if (token.empty() || parent_window == 0 || parent_process_id == 0 ||
            !window_owned_by(parent, parent_process_id)) {
            return kEditorParentInvalidMessage;
        }
        if (view_ && parent != parent_) retire();
        if (!view_) {
            view_ = create_view();
            if (!view_) return kEditorUnavailableMessage;
            view_->setFrame(this);
            parent_ = parent;
            parent_process_id_ = parent_process_id;
            attached_ = false;
            ++generation_;
            PostMessageW(ui_window_, kAttachEditorMessage, static_cast<WPARAM>(generation_), 0);
        }
        return "{\"type\":\"EditorOpened\",\"payload\":{\"parent_window\":" +
               std::to_string(parent_window) + "}}";
    }

    std::string close() {
        if (view_) {
            retire();
            PostMessageW(ui_window_, kReleaseEditorsMessage, 0, 0);
        }
        return kEditorClosedMessage;
    }

    void attach(uint64_t generation) {
        if (!view_ || attached_ || generation != generation_) return;
        if (!window_owned_by(parent_, parent_process_id_)) {
            retire();
            release_retired();
            return;
        }
        IPlugViewContentScaleSupport* scale = nullptr;
        if (view_->queryInterface(IPlugViewContentScaleSupport_iid,
                                  reinterpret_cast<void**>(&scale)) == kResultOk && scale) {
            const auto dpi = GetDpiForWindow(parent_);
            if (dpi > 0) scale->setContentScaleFactor(static_cast<float>(dpi) / 96.0f);
            scale->release();
        }
        const auto result = view_->attached(reinterpret_cast<void*>(parent_), kPlatformTypeHWND);
        if (result != kResultOk) {
            std::cerr << "VST3 editor attach failed with VST3 result 0x" << std::hex
                      << static_cast<uint32>(result) << std::dec << '\n';
            retire();
            release_retired();
            return;
        }
        attached_ = true;
        ViewRect rect{};
        if (view_->getSize(&rect) == kResultOk && valid_size(rect)) {
            fit_parent(rect);
            last_client_width_ = rect.getWidth();
            last_client_height_ = rect.getHeight();
        }
        SetTimer(ui_window_, kParentSizeTimer, kParentSizePollMs, nullptr);
    }

    // The user can resize the shell's window; a resizable editor follows it.
    void follow_parent_size() {
        if (!view_ || !attached_) return;
        if (!IsWindow(parent_)) {
            retire();
            release_retired();
            return;
        }
        RECT client{};
        if (!GetClientRect(parent_, &client)) return;
        const auto width = static_cast<int32>(client.right - client.left);
        const auto height = static_cast<int32>(client.bottom - client.top);
        if (width == last_client_width_ && height == last_client_height_) return;
        last_client_width_ = width;
        last_client_height_ = height;
        if (width < 1 || height < 1 || view_->canResize() != kResultTrue) return;
        ViewRect rect(0, 0, std::min(width, kMaxEditorSide), std::min(height, kMaxEditorSide));
        view_->checkSizeConstraint(&rect);
        view_->onSize(&rect);
    }

    void release_retired() {
        for (auto& [view, attached] : retired_) {
            if (attached) view->removed();
            view->setFrame(nullptr);
            view->release();
        }
        retired_.clear();
    }

private:
    static bool valid_size(const ViewRect& rect) {
        return rect.getWidth() >= 1 && rect.getWidth() <= kMaxEditorSide && rect.getHeight() >= 1 &&
               rect.getHeight() <= kMaxEditorSide;
    }

    IPlugView* create_view() {
        if (!controller_) return nullptr;
        IPlugView* view = controller_->createView(Vst::ViewType::kEditor);
        if (view && view->isPlatformTypeSupported(kPlatformTypeHWND) != kResultTrue) {
            view->release();
            view = nullptr;
        }
        return view;
    }

    void retire() {
        KillTimer(ui_window_, kParentSizeTimer);
        if (view_) retired_.emplace_back(view_, attached_);
        view_ = nullptr;
        attached_ = false;
        parent_ = nullptr;
        parent_process_id_ = 0;
        ++generation_;
    }

    // Size the shell's window so its client area fits the editor. Posted
    // (SWP_ASYNCWINDOWPOS): the shell's window thread may be busy.
    void fit_parent(const ViewRect& rect) {
        if (!valid_size(rect) || !IsWindow(parent_)) return;
        RECT frame{0, 0, rect.getWidth(), rect.getHeight()};
        const auto style = static_cast<DWORD>(GetWindowLongPtrW(parent_, GWL_STYLE));
        const auto extended = static_cast<DWORD>(GetWindowLongPtrW(parent_, GWL_EXSTYLE));
        const auto dpi = GetDpiForWindow(parent_);
        if (!AdjustWindowRectExForDpi(&frame, style, FALSE, extended, dpi > 0 ? dpi : 96)) return;
        SetWindowPos(parent_, nullptr, 0, 0, frame.right - frame.left, frame.bottom - frame.top,
                     SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS);
    }

    Vst::IEditController* controller_;
    HWND ui_window_;
    IPlugView* view_ = nullptr;
    bool attached_ = false;
    HWND parent_ = nullptr;
    uint32_t parent_process_id_ = 0;
    uint64_t generation_ = 0;
    int32 last_client_width_ = 0;
    int32 last_client_height_ = 0;
    std::vector<std::pair<IPlugView*, bool>> retired_;
};

static void append_bus_list(std::ostringstream& output, const std::vector<uint16_t>& buses) {
    output << '[';
    for (std::size_t index = 0; index < buses.size(); ++index) {
        if (index) output << ',';
        output << buses[index];
    }
    output << ']';
}

static std::string hello_buses_message(const std::string& hash,
                                       const std::vector<uint16_t>& inputs,
                                       const std::vector<uint16_t>& outputs) {
    std::ostringstream message;
    message << "{\"type\":\"HelloBuses\",\"payload\":{\"protocol_version\":1,\"plugin_sha256\":"
            << quote_json(hash) << ",\"layout\":{\"input_channels\":";
    append_bus_list(message, inputs);
    message << ",\"output_channels\":";
    append_bus_list(message, outputs);
    message << "}}}";
    return message.str();
}

static std::string processed_buses_message(
    uint64_t sequence, uint64_t deadline, const std::vector<uint16_t>& inputs,
    const std::vector<uint16_t>& outputs, const std::vector<std::vector<float>>& frames) {
    std::ostringstream message;
    message << "{\"type\":\"ProcessedBuses\",\"payload\":{\"layout\":{\"input_channels\":";
    append_bus_list(message, inputs);
    message << ",\"output_channels\":";
    append_bus_list(message, outputs);
    message << "},\"frames\":[";
    message << std::setprecision(9);
    for (std::size_t bus = 0; bus < frames.size(); ++bus) {
        if (bus) message << ',';
        message << "{\"sequence\":" << sequence << ",\"deadline_tick\":" << deadline
                << ",\"channels\":" << outputs[bus] << ",\"samples\":[";
        for (std::size_t index = 0; index < frames[bus].size(); ++index) {
            if (index) message << ',';
            message << frames[bus][index];
        }
        message << "]}";
    }
    message << "]}}";
    return message.str();
}

struct Arguments {
    fs::path plugin;
    std::string hash;
    uint16_t channels = 0;
    double sample_rate = 0.0;
    std::vector<uint16_t> input_buses;
    std::vector<uint16_t> output_buses;
};

static std::vector<uint16_t> parse_bus_list(const std::wstring& value) {
    std::vector<uint16_t> buses;
    std::size_t start = 0;
    while (start <= value.size()) {
        const auto end = value.find(L',', start);
        const auto part = value.substr(start, end == std::wstring::npos ? end : end - start);
        if (part.empty()) throw std::runtime_error("bus list contains an empty entry");
        const auto channels = std::stoul(part);
        if (channels < 1 || channels > 2) throw std::runtime_error("bus channels must be 1 or 2");
        buses.push_back(static_cast<uint16_t>(channels));
        if (buses.size() > 4) throw std::runtime_error("bus count exceeds four");
        if (end == std::wstring::npos) break;
        start = end + 1;
    }
    if (buses.empty()) throw std::runtime_error("bus list is empty");
    return buses;
}

static Arguments parse_arguments(int argc, wchar_t** argv) {
    Arguments result;
    for (int index = 1; index < argc; ++index) {
        const std::wstring argument(argv[index]);
        if (argument == L"--plugin-path" && index + 1 < argc) result.plugin = argv[++index];
        else if (argument == L"--plugin-sha256" && index + 1 < argc) {
            const std::wstring hash(argv[++index]);
            result.hash.assign(hash.begin(), hash.end());
        }
        else if (argument == L"--channels" && index + 1 < argc) result.channels = static_cast<uint16_t>(std::stoul(argv[++index]));
        else if (argument == L"--sample-rate" && index + 1 < argc) result.sample_rate = std::stod(argv[++index]);
        else if (argument == L"--input-buses" && index + 1 < argc) result.input_buses = parse_bus_list(argv[++index]);
        else if (argument == L"--output-buses" && index + 1 < argc) result.output_buses = parse_bus_list(argv[++index]);
        else throw std::runtime_error("unknown or incomplete worker argument");
    }
    if (result.plugin.empty() || result.channels < 1 || result.channels > 2 ||
        !std::isfinite(result.sample_rate) || result.sample_rate < 8000.0 || result.sample_rate > 384000.0) {
        throw std::runtime_error("invalid native VST3 worker arguments");
    }
    if (result.input_buses.empty() != result.output_buses.empty()) {
        throw std::runtime_error("input and output bus lists must be supplied together");
    }
    return result;
}


// A request the audio thread hands to the UI thread and waits for.
struct UiRequest {
    const std::string* json = nullptr;
    std::string reply;
    std::string error;
    bool shutdown = false;
};

struct UiContext {
    Vst3Effect* effect = nullptr;
    EditorHost* editor = nullptr;
};

static UiContext g_ui;
static HWND g_ui_window = nullptr;
static std::atomic<bool> g_audio_stopped{false};

static bool is_latency_message(const std::string& json) {
    return json.find("\"type\":\"Latency\"") != std::string::npos;
}

static std::string latency_message(uint32_t samples, uint64_t sample_rate) {
    return "{\"type\":\"Latency\",\"payload\":{\"samples\":" + std::to_string(samples) +
           ",\"sample_rate_hz\":" + std::to_string(sample_rate) + "}}";
}

// Every request except audio processing. Runs on the UI thread.
static std::string handle_ui_request(const std::string& json, bool& shutdown) {
    auto& effect = *g_ui.effect;
    auto& editor = *g_ui.editor;
    if (json == "{\"type\":\"Shutdown\"}") {
        shutdown = true;
        return {};
    }
    if (json == "{\"type\":\"DescribeEditor\"}") return editor.describe();
    if (json == "{\"type\":\"DescribeParameters\"}") {
        return parameter_descriptors_message(effect.controller());
    }
    if (json.find("\"type\":\"EditorOpen\"") != std::string::npos) {
        const auto parent_window = required_uint_field(json, "\"parent_window\":");
        const auto parent_process_id = required_uint_field(json, "\"parent_process_id\":");
        if (parent_process_id > std::numeric_limits<uint32_t>::max()) {
            return kEditorParentInvalidMessage;
        }
        return editor.open(parent_window, static_cast<uint32_t>(parent_process_id),
                           parse_string_field(json, "\"authorization_token\":"));
    }
    if (json == "{\"type\":\"EditorClose\"}") return editor.close();
    if (json.find("\"type\":\"StateSave\"") != std::string::npos) {
        return state_message(1, effect.save_state());
    }
    if (json.find("\"type\":\"StateRestore\"") != std::string::npos) {
        const auto version = required_uint_field(json, "\"version\":");
        if (version == 0 || version > std::numeric_limits<uint32_t>::max()) {
            throw protocol_error("state version is outside the bounded contract");
        }
        const auto bytes = parse_bytes(json);
        try {
            effect.restore_state(bytes);
        } catch (const std::exception& error) {
            const std::string diagnostic(error.what());
            if (diagnostic.find("setState failed with VST3 result") != std::string::npos ||
                diagnostic.find("controller setState failed with VST3 result") != std::string::npos ||
                diagnostic.find("controller component state failed with VST3 result") != std::string::npos) {
                return kStateUnsupportedMessage;
            }
            throw;
        }
        return state_message(static_cast<uint32_t>(version), bytes);
    }
    if (is_latency_message(json)) {
        const auto sample_rate = required_uint_field(json, "\"sample_rate_hz\":");
        if (g_latency_restart_pending.exchange(false, std::memory_order_acq_rel)) effect.restart_for_latency();
        const auto samples = effect.latency_samples();
        g_latency_samples.store(samples, std::memory_order_release);
        g_latency_cached.store(true, std::memory_order_release);
        return latency_message(samples, sample_rate);
    }
    throw protocol_error("expected Process or Shutdown");
}

static LRESULT CALLBACK ui_window_proc(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
    switch (message) {
        case kUiRequestMessage: {
            auto* request = reinterpret_cast<UiRequest*>(lparam);
            try {
                request->reply = handle_ui_request(*request->json, request->shutdown);
            } catch (const std::exception& error) {
                request->error = error.what();
            }
            return 0;
        }
        case kHostParametersMessage: {
            std::unique_ptr<HostParameterValues> values(reinterpret_cast<HostParameterValues*>(lparam));
            // Keep the plugin's own window in step with AudioRouter's sliders.
            if (auto* controller = g_ui.effect->controller()) {
                for (const auto& [id, value] : *values) controller->setParamNormalized(id, value);
            }
            return 0;
        }
        case kAttachEditorMessage:
            try {
                g_ui.editor->attach(static_cast<uint64_t>(wparam));
            } catch (const std::exception& error) {
                std::cerr << "VST3 editor attach failed: " << error.what() << '\n';
            }
            return 0;
        case kReleaseEditorsMessage:
            g_ui.editor->release_retired();
            return 0;
        case WM_TIMER:
            if (wparam == kParentSizeTimer) g_ui.editor->follow_parent_size();
            return 0;
        case kAudioStoppedMessage:
            return 0;
        default:
            return DefWindowProcW(window, message, wparam, lparam);
    }
}

static HWND create_ui_window() {
    WNDCLASSEXW window_class{};
    window_class.cbSize = sizeof(window_class);
    window_class.lpfnWndProc = ui_window_proc;
    window_class.hInstance = GetModuleHandleW(nullptr);
    window_class.lpszClassName = L"AudioRouterVst3WorkerUi";
    if (!RegisterClassExW(&window_class)) throw std::runtime_error("could not register the worker UI window");
    const auto window = CreateWindowExW(0, window_class.lpszClassName, L"", 0, 0, 0, 0, 0, HWND_MESSAGE,
                                        nullptr, window_class.hInstance, nullptr);
    if (!window) throw std::runtime_error("could not create the worker UI window");
    return window;
}

static bool is_audio_message(const std::string& json) {
    return json.rfind("{\"type\":\"Process", 0) == 0;
}

// Reads every host message. Audio is processed here, so a busy plugin window
// never delays a quantum; everything else is answered by the UI thread.
static int audio_thread_main(const Arguments& arguments, const std::vector<uint16_t>& input_buses,
                             const std::vector<uint16_t>& output_buses, bool multi_bus) {
    auto& effect = *g_ui.effect;
    while (true) {
        const auto payload = read_frame();
        const std::string json(payload.begin(), payload.end());
        if (!is_audio_message(json)) {
            const bool latency = is_latency_message(json);
            if (latency && g_latency_cached.load(std::memory_order_acquire) &&
                !g_latency_restart_pending.load(std::memory_order_acquire)) {
                write_frame(latency_message(g_latency_samples.load(std::memory_order_acquire),
                                            required_uint_field(json, "\"sample_rate_hz\":")));
                continue;
            }
            // State and editor requests can change the plugin's delay.
            if (!latency) g_latency_cached.store(false, std::memory_order_release);
            UiRequest request;
            request.json = &json;
            DWORD_PTR ignored = 0;
            if (!SendMessageTimeoutW(g_ui_window, kUiRequestMessage, 0, reinterpret_cast<LPARAM>(&request),
                                     SMTO_NORMAL, kUiRequestTimeoutMs, &ignored)) {
                // The UI thread is stuck inside plugin code and still owns
                // `request`; end the process rather than return.
                std::cerr << "native VST3 worker stopped: the plugin UI thread did not answer\n";
                std::cerr.flush();
                TerminateProcess(GetCurrentProcess(), 1);
            }
            if (!request.error.empty()) throw std::runtime_error(request.error);
            if (request.shutdown) return 0;
            write_frame(request.reply);
            continue;
        }
        auto events = parse_parameters(json);
        HostParameterValues host_values;
        for (const auto& event : events) {
            auto existing = std::find_if(host_values.begin(), host_values.end(),
                                         [&](const auto& value) { return value.first == event.id; });
            if (existing == host_values.end()) {
                host_values.emplace_back(event.id, event.value);
            } else {
                existing->second = event.value;
            }
        }
        // Edits from the plugin's window go first; a host event for the same
        // parameter in this quantum wins, so that edit is dropped. A quantum
        // carries at most kMaxParameters parameters (the processor's queue
        // limit); edits that do not fit wait for the next quantum.
        auto edits = g_editor_edits.take();
        std::vector<ParameterEvent> editor_events;
        std::vector<std::pair<Vst::ParamID, Vst::ParamValue>> deferred;
        std::size_t room = kMaxParameters - std::min(kMaxParameters, host_values.size());
        for (const auto& edit : edits) {
            const bool host_sets_it = std::any_of(host_values.begin(), host_values.end(),
                                                  [&](const auto& value) { return value.first == edit.first; });
            if (host_sets_it) continue;
            if (room == 0) {
                deferred.push_back(edit);
                continue;
            }
            --room;
            editor_events.push_back(ParameterEvent{edit.first, static_cast<float>(edit.second), 0});
        }
        if (!deferred.empty()) g_editor_edits.restore(deferred);
        events.insert(events.begin(), editor_events.begin(), editor_events.end());
        if (multi_bus) {
            if (json.find("\"type\":\"ProcessBuses\"") == std::string::npos) {
                throw protocol_error("expected ProcessBuses or Shutdown");
            }
            const auto frames = parse_bus_frames(json);
            const auto output = effect.process_buses(frames, events);
            write_frame(processed_buses_message(frames.front().sequence, frames.front().deadline,
                                                input_buses, output_buses, output));
        } else {
            if (json.find("\"type\":\"Process\"") == std::string::npos) {
                throw protocol_error("expected Process or Shutdown");
            }
            const auto sequence = required_uint_field(json, "\"sequence\":");
            const auto deadline = required_uint_field(json, "\"deadline_tick\":");
            const auto channels = required_uint_field(json, "\"channels\":");
            if (channels != arguments.channels || channels > 2) throw protocol_error("channel mismatch");
            const auto samples = parse_samples(json);
            const auto output = effect.process(samples, arguments.channels, events);
            write_frame(processed_message(sequence, deadline, arguments.channels, output));
        }
        if (!host_values.empty()) {
            auto values = std::make_unique<HostParameterValues>(std::move(host_values));
            if (PostMessageW(g_ui_window, kHostParametersMessage, 0, reinterpret_cast<LPARAM>(values.get()))) {
                values.release();
            }
        }
    }
}

int wmain(int argc, wchar_t** argv) {
    try {
        // Third-party plugin code runs in this disposable process. A plugin
        // crash must terminate the worker for the supervisor to quarantine it;
        // Windows Error Reporting UI would otherwise block unattended runs.
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
        // SetErrorMode covers the legacy fault dialog, while WER can still
        // queue a consent/reporting UI on supported Windows builds. Disable
        // all WER UI in this disposable worker so a plugin fault always
        // reaches the supervisor as process termination.
        if (FAILED(WerSetFlags(WER_FAULT_REPORTING_NO_UI))) {
            throw std::runtime_error("could not disable Windows Error Reporting UI");
        }
        // Plugin editors size themselves in physical pixels inside the
        // shell's DPI-aware window.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // This thread is the plugin's UI thread; editors (some built on
        // WebView2) expect a single-threaded COM apartment.
        if (FAILED(OleInitialize(nullptr))) throw std::runtime_error("could not initialize COM");
        isolate_protocol_pipes();
        const auto arguments = parse_arguments(argc, argv);
        const auto input_buses = arguments.input_buses.empty()
                                     ? std::vector<uint16_t>{arguments.channels}
                                     : arguments.input_buses;
        const auto output_buses = arguments.output_buses.empty()
                                      ? std::vector<uint16_t>{arguments.channels}
                                      : arguments.output_buses;
        int exit_code = 1;
        {
            Vst3Effect effect(arguments.plugin, arguments.sample_rate, input_buses, output_buses);
            const auto multi_bus = !arguments.input_buses.empty();
            if (multi_bus) {
                write_frame(hello_buses_message(arguments.hash, input_buses, output_buses));
            } else {
                write_frame("{\"type\":\"Hello\",\"payload\":{\"protocol_version\":1,\"plugin_sha256\":" +
                            quote_json(arguments.hash) + ",\"channels\":" +
                            std::to_string(arguments.channels) + "}}");
            }
            const auto ready = read_frame();
            const std::string ready_json(ready.begin(), ready.end());
            if (ready_json != "{\"type\":\"Ready\"}") throw protocol_error("expected Ready");
            g_ui_window = create_ui_window();
            EditorHost editor(effect.controller(), g_ui_window);
            g_ui.effect = &effect;
            g_ui.editor = &editor;
            std::thread audio([&] {
                try {
                    exit_code = audio_thread_main(arguments, input_buses, output_buses, multi_bus);
                } catch (const std::exception& error) {
                    std::cerr << "native VST3 worker stopped: " << error.what() << '\n';
                }
                g_audio_stopped.store(true, std::memory_order_release);
                PostMessageW(g_ui_window, kAudioStoppedMessage, 0, 0);
            });
            // A plugin may post WM_QUIT itself; only the audio thread ends
            // this loop.
            while (!g_audio_stopped.load(std::memory_order_acquire)) {
                MSG message{};
                const auto result = GetMessageW(&message, nullptr, 0, 0);
                if (result == -1) break;
                if (result == 0) continue;
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            audio.join();
            g_ui.editor = nullptr;
        }
        OleUninitialize();
        return exit_code;
    } catch (const std::exception& error) {
        std::cerr << "native VST3 worker stopped: " << error.what() << '\n';
        return 1;
    }
}
