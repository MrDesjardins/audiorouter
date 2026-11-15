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
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstmidicontrollers.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include "pluginterfaces/vst/ivstprocesscontext.h"
#include "host-support.h"

namespace fs = std::filesystem;
using namespace Steinberg;
using GetPluginFactoryProc = IPluginFactory* (PLUGIN_API*)();
// Windows module entry points (the SDK's module loader calls them around a
// plugin's lifetime; VSTGUI editors need InitDll for their instance handle).
using ModuleEntryProc = bool (PLUGIN_API*)();

constexpr std::size_t kMaxMessageBytes = 65 * 1024 * 1024;
constexpr std::size_t kMaxFrames = 4096;
// Keep the native descriptor/event bound aligned with the Rust worker
// contract. The bound remains finite and is enforced before queue growth.
constexpr std::size_t kMaxParameters = 4096;
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

// One MIDI channel-voice message for an instrument (worker protocol "Midi").
struct MidiEvent {
    std::size_t offset;
    uint8_t status;
    uint8_t data1;
    uint8_t data2;
};
// Aligned with MAX_MIDI_EVENTS_PER_BLOCK in the Rust worker contract.
constexpr std::size_t kMaxMidiEvents = 256;

static std::vector<MidiEvent> parse_midi_events(const std::string& json) {
    const std::string key = "\"events\":[";
    const auto array = json.find(key);
    if (array == std::string::npos) throw protocol_error("missing MIDI event array");
    auto position = array + key.size();
    std::vector<MidiEvent> events;
    while (position < json.size() && json[position] != ']') {
        if (json[position] != '{') {
            ++position;
            continue;
        }
        auto cursor = find_required(json, "\"sample_offset\":", position);
        const auto offset = parse_uint(json, cursor);
        cursor = find_required(json, "\"status\":", cursor);
        const auto status = parse_uint(json, cursor);
        cursor = find_required(json, "\"data1\":", cursor);
        const auto data1 = parse_uint(json, cursor);
        cursor = find_required(json, "\"data2\":", cursor);
        const auto data2 = parse_uint(json, cursor);
        if (offset >= kMaxFrames || status < 0x80 || status > 0xEF || data1 > 0x7F || data2 > 0x7F) {
            throw protocol_error("MIDI event is outside the bounded contract");
        }
        events.push_back({static_cast<std::size_t>(offset), static_cast<uint8_t>(status),
                          static_cast<uint8_t>(data1), static_cast<uint8_t>(data2)});
        if (events.size() > kMaxMidiEvents) {
            throw protocol_error("MIDI event count exceeds the bounded contract");
        }
        const auto close = json.find('}', cursor);
        if (close == std::string::npos) throw protocol_error("MIDI event is unterminated");
        position = close + 1;
    }
    if (position >= json.size()) throw protocol_error("MIDI event array is unterminated");
    return events;
}

// The input event list handed to an instrument's process call.
class EventList final : public Vst::IEventList {
public:
    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        return host_query<Vst::IEventList>(this, iid, object, Vst::IEventList_iid);
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    int32 PLUGIN_API getEventCount() override { return static_cast<int32>(events_.size()); }
    tresult PLUGIN_API getEvent(int32 index, Vst::Event& event) override {
        if (index < 0 || static_cast<std::size_t>(index) >= events_.size()) return kInvalidArgument;
        event = events_[static_cast<std::size_t>(index)];
        return kResultOk;
    }
    tresult PLUGIN_API addEvent(Vst::Event& event) override {
        if (events_.size() >= kMaxMidiEvents) return kOutOfMemory;
        events_.push_back(event);
        return kResultOk;
    }
    void clear() { events_.clear(); }

private:
    std::vector<Vst::Event> events_;
};

class ParameterQueue final : public Vst::IParamValueQueue {
public:
    explicit ParameterQueue(Vst::ParamID id) : id_(id) {}

    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        return host_query<Vst::IParamValueQueue>(this, iid, object, Vst::IParamValueQueue_iid);
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
    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        return host_query<Vst::IParameterChanges>(this, iid, object, Vst::IParameterChanges_iid);
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
    // The VST3 bundle layout names the folder by architecture; this worker only
    // ever loads binaries of its own bitness (a 32-bit build hosts x86-win).
#if defined(_WIN64)
    const auto contents = supplied / "Contents" / "x86_64-win";
#else
    const auto contents = supplied / "Contents" / "x86-win";
#endif
    fs::path result;
    for (const auto& entry : fs::directory_iterator(contents)) {
        if (entry.is_regular_file() && entry.path().extension() == L".vst3") {
            if (!result.empty()) throw std::runtime_error("VST3 bundle has multiple binaries");
            result = entry.path();
        }
    }
    if (result.empty()) throw std::runtime_error("VST3 bundle has no binary for this worker's architecture");
    return result;
}

static void require_result(const char* operation, tresult result) {
    if (result == kResultOk || result == kNotImplemented) return;
    std::ostringstream message;
    message << operation << " failed with VST3 result 0x" << std::hex
            << static_cast<uint32>(result);
    throw std::runtime_error(message.str());
}

// Whether `cid` is the class named by 32 upper-case hexadecimal characters,
// in the Windows (GUID) spelling the SDK writes here or in plain byte order.
static bool class_id_matches(const TUID cid, const std::string& wanted) {
    GUID guid{};
    std::memcpy(&guid, cid, sizeof(guid));
    std::ostringstream com;
    com << std::uppercase << std::hex << std::setfill('0') << std::setw(8) << guid.Data1
        << std::setw(4) << guid.Data2 << std::setw(4) << guid.Data3;
    for (const auto byte : guid.Data4) com << std::setw(2) << static_cast<unsigned>(byte);
    std::ostringstream plain;
    plain << std::uppercase << std::hex << std::setfill('0');
    for (std::size_t index = 0; index < sizeof(TUID); ++index) {
        plain << std::setw(2) << static_cast<unsigned>(static_cast<unsigned char>(cid[index]));
    }
    return com.str() == wanted || plain.str() == wanted;
}

// Optional host notifications may return false without indicating a fault.
static void require_notification(const char* operation, tresult result) {
    if (result != kResultFalse) require_result(operation, result);
}

class HostApplication final : public Vst::IHostApplication {
public:
    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        return host_query<Vst::IHostApplication>(this, iid, object, Vst::IHostApplication_iid);
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }

    tresult PLUGIN_API getName(Vst::String128 name) override {
        if (!name) return kInvalidArgument;
        const char16_t value[] = u"AudioRouter";
        std::copy(std::begin(value), std::end(value), name);
        return kResultOk;
    }

    tresult PLUGIN_API createInstance(TUID cid, TUID iid, void** object) override {
        if (!object) return kInvalidArgument;
        *object = nullptr;
        if (FUnknownPrivate::iidEqual(cid, Vst::IMessage_iid) && FUnknownPrivate::iidEqual(iid, Vst::IMessage_iid)) {
            *object = static_cast<Vst::IMessage*>(new HostMessage); return kResultOk;
        }
        if (FUnknownPrivate::iidEqual(cid, Vst::IAttributeList_iid) && FUnknownPrivate::iidEqual(iid, Vst::IAttributeList_iid)) {
            *object = static_cast<Vst::IAttributeList*>(new HostAttributes); return kResultOk;
        }
        return kNoInterface;
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

// The plugin's own delay. The host asks for it about twice a second while
// audio plays, so once the first answer is cached the pipe thread always
// answers from the cache and never waits for a UI thread that a busy plugin
// window may hold. The UI thread refreshes the cache after every request it
// serves (state, editor). After `restartComponent(kLatencyChanged)` the next
// Latency request still answers from the cache and posts the re-activation
// to the UI thread, which runs it under `g_processing_mutex` and stores the
// new value; the poll after that reports it.
static std::atomic<bool> g_latency_restart_pending{false};
static std::atomic<bool> g_latency_cached{false};
static std::atomic<uint32_t> g_latency_samples{0};
// Set when a posted re-activation failed; the audio thread then stops the
// worker so the supervisor sees the failure, as it did before the posting.
static std::atomic<bool> g_latency_restart_failed{false};
// Held by the audio thread around each Process call and by the UI thread
// around a posted latency re-activation, so the component is never
// deactivated while it processes. The audio thread waits only for the
// re-activation itself, not for a UI thread busy elsewhere.
static std::mutex g_processing_mutex;

// Which restart requests need the component switched off and on again. A
// latency change must be re-read after re-activation (SDK rule); an I/O change
// and a reload request get the same treatment so the plugin can rebuild its
// processing. Bus counts and layouts are fixed when the worker starts, so a
// plugin that changes them keeps the layout it was given.
static bool restart_needs_reactivation(int32 flags) {
    return (flags & (Vst::kLatencyChanged | Vst::kIoChanged | Vst::kReloadComponent)) != 0;
}

class ComponentHandler final : public Vst::IComponentHandler, public Vst::IComponentHandler2 {
public:
    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        if (object && FUnknownPrivate::iidEqual(iid, Vst::IComponentHandler2_iid)) {
            *object = static_cast<Vst::IComponentHandler2*>(this);
            return kResultOk;
        }
        return host_query<Vst::IComponentHandler>(this, iid, object, Vst::IComponentHandler_iid);
    }
    // IComponentHandler2. The route's saved state is taken when the editor
    // closes or the user saves, so "dirty" needs no action; group edits arrive
    // as ordinary edits; the host opens editors only on the user's request.
    tresult PLUGIN_API setDirty(TBool) override { return kResultOk; }
    tresult PLUGIN_API requestOpenEditor(FIDString) override { return kResultFalse; }
    tresult PLUGIN_API startGroupEdit() override { return kResultOk; }
    tresult PLUGIN_API finishGroupEdit() override { return kResultOk; }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    tresult PLUGIN_API beginEdit(Vst::ParamID) override { return kResultOk; }
    tresult PLUGIN_API performEdit(Vst::ParamID id, Vst::ParamValue value) override {
        g_editor_edits.push(id, value);
        return kResultOk;
    }
    tresult PLUGIN_API endEdit(Vst::ParamID) override { return kResultOk; }
    // Called by the controller, usually on the UI thread. The re-activation
    // is done between quanta by the next Latency request. kParamValuesChanged
    // needs nothing: the worker keeps no copy of the values, it reads them from
    // the controller when it saves.
    tresult PLUGIN_API restartComponent(int32 flags) override {
        if (restart_needs_reactivation(flags)) g_latency_restart_pending.store(true, std::memory_order_release);
        return kResultOk;
    }
};

class StateStream final : public IBStream {
public:
    StateStream() = default;
    StateStream(const uint8_t* data, std::size_t size) : bytes_(data, data + size) {}

    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        return host_query<IBStream>(this, iid, object, IBStream_iid);
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
               const std::vector<uint16_t>& output_channels, const std::string& class_id = {},
               bool instrument = false)
        : requested_input_channels_(input_channels), requested_output_channels_(output_channels),
          instrument_(instrument) {
        context_.sampleRate = sample_rate;
        context_.tempo = 120.0;
        context_.timeSigNumerator = 4;
        context_.timeSigDenominator = 4;
        context_.state = Vst::ProcessContext::kPlaying | Vst::ProcessContext::kTempoValid | Vst::ProcessContext::kTimeSigValid | Vst::ProcessContext::kProjectTimeMusicValid;
        // An instrument has no audio input; an effect needs at least one.
        if ((input_channels.empty() && !instrument) || output_channels.empty() ||
            input_channels.size() > 4 ||
            output_channels.size() > 4 ||
            std::accumulate(input_channels.begin(), input_channels.end(), std::size_t{0}) > 8 ||
            std::accumulate(output_channels.begin(), output_channels.end(), std::size_t{0}) > 8) {
            throw protocol_error("native VST3 bus layout is outside the bounded contract");
        }
        const auto binary = fs::absolute(resolve_binary(supplied));
        module_ = LoadLibraryExW(binary.c_str(), nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
                                                               LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
        if (!module_) throw std::runtime_error("VST3 LoadLibraryExW failed (Windows error " + std::to_string(GetLastError()) + "); install the plugin's required runtime and companion DLLs");
        if (const auto init = reinterpret_cast<ModuleEntryProc>(GetProcAddress(module_, "InitDll"))) {
            if (!init()) throw std::runtime_error("VST3 InitDll failed");
            module_initialized_ = true;
        }
        const auto get_factory = reinterpret_cast<GetPluginFactoryProc>(
            GetProcAddress(module_, "GetPluginFactory"));
        if (!get_factory) throw std::runtime_error("VST3 GetPluginFactory export is missing");
        factory_ = get_factory();
        if (!factory_) throw std::runtime_error("VST3 factory is null");
        // The requested class comes from the bundle's moduleinfo.json, which
        // is not part of the verified binary and may be stale or written in
        // the other byte order. Accept both spellings, but never replace an
        // explicitly requested class with another effect: its parameters,
        // state and audio buses can have entirely different meanings.
        std::string requested_class = class_id;
        if (!requested_class.empty()) {
            bool listed = false;
            for (int32 index = 0; index < factory_->countClasses(); ++index) {
                PClassInfo info{};
                if (factory_->getClassInfo(index, &info) == kResultOk &&
                    class_id_matches(info.cid, requested_class)) {
                    listed = true;
                    break;
                }
            }
            if (!listed) {
                throw std::runtime_error("Requested VST3 class " + requested_class +
                                         " is not in this binary; scan the plugin again and select its class");
            }
        }
        std::ostringstream class_failures;
        bool found_audio_class = false;
        bool found_instrument = false;
        IPluginFactory2* factory2 = nullptr;
        factory_->queryInterface(IPluginFactory2_iid, reinterpret_cast<void**>(&factory2));
        for (int32 index = 0; index < factory_->countClasses(); ++index) {
            PClassInfo info{};
            require_result("getClassInfo", factory_->getClassInfo(index, &info));
            if (std::strcmp(info.category, kVstAudioEffectClass) != 0) continue;
            if (!requested_class.empty() && !class_id_matches(info.cid, requested_class)) continue;
            // Instruments share the audio-module category; their
            // sub-categories say "Instrument". An effect worker skips them
            // and an instrument worker skips everything else.
            PClassInfo2 info2{};
            const bool is_instrument_class =
                factory2 && factory2->getClassInfo2(index, &info2) == kResultOk &&
                std::strstr(info2.subCategories, "Instrument") != nullptr;
            if (is_instrument_class != instrument) {
                if (is_instrument_class) found_instrument = true;
                else found_audio_class = true;
                continue;
            }
            found_audio_class = instrument ? found_audio_class : true;
            found_instrument = instrument ? true : found_instrument;
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
            if (instrument && !found_instrument) {
                throw std::runtime_error(found_audio_class
                                             ? "VST3 plugin is an effect, not an instrument"
                                             : "VST3 factory has no instrument class");
            }
            if (!instrument && !found_audio_class && found_instrument) {
                throw std::runtime_error(
                    "VST3 plugin is an instrument; add it with the Instrument tool to play it with MIDI");
            }
            if (!found_audio_class && !instrument) {
                throw std::runtime_error("VST3 factory has no audio effect class");
            }
            throw std::runtime_error("VST3 audio effect classes were unusable:" +
                                     class_failures.str());
        }
        initialized_ = true;
        TUID controller_id{};
        const auto single_controller = component_->queryInterface(Vst::IEditController_iid, reinterpret_cast<void**>(&controller_));
        const auto controller_id_result = component_->getControllerClassId(controller_id);
        if (single_controller == kResultOk && controller_) {
            // The component has already initialized this combined interface;
            // do not initialize/terminate the same object twice.
            require_result("controller component handler", controller_->setComponentHandler(&component_handler_));
        } else if (controller_id_result == kResultOk) {
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
        } else if (controller_id_result != kNotImplemented && controller_id_result != kNoInterface && controller_id_result != kResultFalse) {
            require_result("controller class id", controller_id_result);
        }
        require_result("processor interface", component_->queryInterface(
            Vst::IAudioProcessor_iid, reinterpret_cast<void**>(&processor_)));
        if (instrument_ && controller_) {
            // Optional: lets controller changes (pitch bend, CC) reach the plugin.
            controller_->queryInterface(Vst::IMidiMapping::iid,
                                        reinterpret_cast<void**>(&midi_mapping_));
        }
        // A single-stream node is mono or stereo, and so is almost every
        // effect, but not always the same way (UADx effects are stereo-only).
        // Ask for the node's layout; if the plugin keeps its own, adapt
        // between them in process().
        auto effective_inputs = input_channels;
        auto effective_outputs = output_channels;
        if (instrument && output_channels.size() == 1) {
            negotiate_main_output(output_channels[0]);
            effective_outputs[0] = plugin_output_channels_;
        } else if (input_channels.size() == 1 && output_channels.size() == 1) {
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
            require_notification("optional input bus deactivation", component_->activateBus(
                Vst::kAudio, Vst::kInput, bus, false));
        }
        const auto output_bus_count = component_->getBusCount(Vst::kAudio, Vst::kOutput);
        for (int32 bus = static_cast<int32>(output_channels.size()); bus < output_bus_count; ++bus) {
            require_notification("optional output bus deactivation", component_->activateBus(
                Vst::kAudio, Vst::kOutput, bus, false));
        }
        if (instrument && component_->getBusCount(Vst::kEvent, Vst::kInput) > 0) {
            require_notification("event input bus activation",
                                 component_->activateBus(Vst::kEvent, Vst::kInput, 0, true));
        }
        Vst::ProcessSetup setup{};
        setup.processMode = Vst::kRealtime;
        setup.symbolicSampleSize = Vst::kSample32;
        setup.maxSamplesPerBlock = static_cast<int32>(kMaxFrames);
        setup.sampleRate = sample_rate;
        // AudioRouter processes 32-bit float only. A plugin that requires
        // 64-bit samples must be refused with a plain reason instead of a
        // generic VST3 result, and never silently converted.
        if (processor_->canProcessSampleSize(Vst::kSample32) == kResultFalse) {
            throw std::runtime_error(
                "sampleFormatUnsupported: the plugin does not accept the 32-bit sample format "
                "AudioRouter uses");
        }
        const auto setup_result = processor_->setupProcessing(setup);
        if (setup_result != kResultOk && setup_result != kNotImplemented) {
            throw std::runtime_error(
                "sampleFormatUnsupported: the plugin does not accept the 32-bit sample format "
                "AudioRouter uses");
        }
        require_result("component activation", component_->setActive(true));
        active_ = true;
        require_notification("processor activation", processor_->setProcessing(true));
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
        if (midi_mapping_) midi_mapping_->release();
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
        const auto component_result = component_->getState(&component_stream);
        if (component_result != kResultFalse) require_result("VST3 component getState", component_result);
        if (controller_) {
            const auto result = controller_->getState(&controller_stream);
            if (result != kResultFalse) require_result("VST3 controller getState", result);
        }
        const auto& component_state = component_stream.bytes();
        const auto& controller_state = controller_stream.bytes();
        if (component_state.size() > UINT32_MAX || controller_state.size() > UINT32_MAX ||
            component_state.size() + controller_state.size() + 16 > kMaxStateBytes) {
            throw protocol_error("VST3 state exceeds the bounded contract");
        }
        const auto snapshot = parameter_snapshot();
        std::vector<uint8_t> state;
        state.reserve(20 + component_state.size() + controller_state.size() + snapshot.size() * 12);
        state.insert(state.end(), {'A', 'R', 'S', 'T', '2', 0, 0, 0});
        append_u32(state, static_cast<uint32_t>(component_state.size()));
        append_u32(state, static_cast<uint32_t>(controller_state.size()));
        state.insert(state.end(), component_state.begin(), component_state.end());
        state.insert(state.end(), controller_state.begin(), controller_state.end());
        // Version 2 ends with the parameter values, a fallback for plugins that
        // reject their own saved state (the host then restores the sliders).
        append_u32(state, static_cast<uint32_t>(snapshot.size()));
        for (const auto& [id, value] : snapshot) {
            append_u32(state, id);
            uint64_t bits = 0;
            std::memcpy(&bits, &value, sizeof(bits));
            append_u32(state, static_cast<uint32_t>(bits & 0xffffffffu));
            append_u32(state, static_cast<uint32_t>(bits >> 32));
        }
        if (state.size() > kMaxStateBytes) throw protocol_error("VST3 state exceeds the bounded contract");
        return state;
    }

    void restore_state(const std::vector<uint8_t>& bytes) {
        if (bytes.empty() || bytes.size() > kMaxStateBytes) {
            throw protocol_error("VST3 state is empty or exceeds the bounded contract");
        }
        std::vector<uint8_t> component_state;
        std::vector<uint8_t> controller_state;
        std::vector<std::pair<Vst::ParamID, double>> snapshot;
        const bool version1 = bytes.size() >= 16 && std::equal(bytes.begin(), bytes.begin() + 8,
                                             std::array<uint8_t, 8>{'A', 'R', 'S', 'T', '1', 0, 0, 0}.begin());
        const bool version2 = bytes.size() >= 20 && std::equal(bytes.begin(), bytes.begin() + 8,
                                             std::array<uint8_t, 8>{'A', 'R', 'S', 'T', '2', 0, 0, 0}.begin());
        if (version1 || version2) {
            const auto component_size = read_u32(bytes, 8);
            const auto controller_size = read_u32(bytes, 12);
            if (component_size > kMaxStateBytes || controller_size > kMaxStateBytes ||
                static_cast<std::size_t>(component_size) + controller_size + 16 > bytes.size()) {
                throw protocol_error("VST3 state envelope is malformed");
            }
            const std::size_t sections_end = 16 + static_cast<std::size_t>(component_size) + controller_size;
            if (version1 && sections_end != bytes.size()) throw protocol_error("VST3 state envelope is malformed");
            component_state.assign(bytes.begin() + 16, bytes.begin() + 16 + component_size);
            controller_state.assign(bytes.begin() + 16 + component_size, bytes.begin() + sections_end);
            if (version2) {
                if (sections_end + 4 > bytes.size()) throw protocol_error("VST3 state envelope is malformed");
                const auto count = read_u32(bytes, sections_end);
                if (count > kMaxParameters || sections_end + 4 + static_cast<std::size_t>(count) * 12 != bytes.size()) {
                    throw protocol_error("VST3 state envelope is malformed");
                }
                for (uint32_t index = 0; index < count; ++index) {
                    const auto at = sections_end + 4 + static_cast<std::size_t>(index) * 12;
                    const uint64_t bits = static_cast<uint64_t>(read_u32(bytes, at + 4)) |
                                          (static_cast<uint64_t>(read_u32(bytes, at + 8)) << 32);
                    double value = 0.0;
                    std::memcpy(&value, &bits, sizeof(value));
                    if (std::isfinite(value)) snapshot.emplace_back(read_u32(bytes, at), std::clamp(value, 0.0, 1.0));
                }
            }
        } else {
            component_state = bytes;
        }
        if (component_state.empty()) throw protocol_error("VST3 component state is empty");
        const bool was_active = active_;
        const bool was_processing = processing_;
        if (was_processing) {
            require_notification("VST3 processor pause for state", processor_->setProcessing(false));
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
                require_notification("VST3 processor resume after rejected state", processor_->setProcessing(true));
                processing_ = true;
            }
            // The plugin rejected its own state: put its parameters back from
            // the snapshot saved beside it, when there is one.
            if (!snapshot.empty()) {
                apply_parameter_snapshot(snapshot);
                return;
            }
            throw;
        }
        if (was_active) {
            require_result("VST3 component resume after state", component_->setActive(true));
            active_ = true;
        }
        if (was_processing) {
            require_notification("VST3 processor resume after state", processor_->setProcessing(true));
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
    Vst::IComponent* component() const { return component_; }

    uint32_t latency_samples() const { return processor_ ? processor_->getLatencySamples() : 0; }

    // The SDK applies a reported latency change only after the component is
    // deactivated and activated again. UI thread only, with
    // `g_processing_mutex` held so no Process call runs meanwhile.
    void restart_for_latency() {
        if (processing_) {
            require_notification("VST3 processor pause for latency", processor_->setProcessing(false));
            processing_ = false;
        }
        if (active_) {
            require_result("VST3 component pause for latency", component_->setActive(false));
            active_ = false;
        }
        require_result("VST3 component resume after latency", component_->setActive(true));
        active_ = true;
        require_notification("VST3 processor resume after latency", processor_->setProcessing(true));
        processing_ = true;
    }

private:
    // The controller's writable parameters and their current normalized values.
    std::vector<std::pair<Vst::ParamID, double>> parameter_snapshot() {
        std::vector<std::pair<Vst::ParamID, double>> values;
        if (!controller_) return values;
        const auto count = controller_->getParameterCount();
        if (count < 0 || count > static_cast<int32>(kMaxScannedParameters)) return values;
        for (int32 index = 0; index < count && values.size() < kMaxParameters; ++index) {
            Vst::ParameterInfo info{};
            if (controller_->getParameterInfo(index, info) != kResultOk) continue;
            if (info.flags & (Vst::ParameterInfo::kIsReadOnly | Vst::ParameterInfo::kIsProgramChange)) continue;
            const double value = controller_->getParamNormalized(info.id);
            if (std::isfinite(value)) values.emplace_back(info.id, std::clamp(value, 0.0, 1.0));
        }
        return values;
    }

    void apply_parameter_snapshot(const std::vector<std::pair<Vst::ParamID, double>>& values) {
        for (const auto& [id, value] : values) {
            if (controller_) controller_->setParamNormalized(id, value);
        }
        pending_snapshot_ = values;
    }

    // Hands the restored values to the processor with the next block.
    void drain_pending_snapshot() {
        if (pending_snapshot_.empty()) return;
        for (const auto& [id, value] : pending_snapshot_) {
            int32 queue_index = -1;
            auto* queue = changes_.addParameterData(id, queue_index);
            if (!queue) break;
            int32 point_index = -1;
            queue->addPoint(0, value, point_index);
        }
        pending_snapshot_.clear();
    }

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

    // Instrument: sets plugin_output_channels_ for the main output bus.
    void negotiate_main_output(uint16_t requested_output) {
        Vst::BusInfo output_info{};
        require_result("output bus info", component_->getBusInfo(Vst::kAudio, Vst::kOutput, 0, output_info));
        if (output_info.channelCount != requested_output) {
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
            outputs[0] = arrangement_for(requested_output);
            // A refusal is normal; the plugin then keeps its own layout.
            processor_->setBusArrangements(inputs.data(), static_cast<int32>(inputs.size()),
                                           outputs.data(), static_cast<int32>(outputs.size()));
            require_result("output bus info", component_->getBusInfo(Vst::kAudio, Vst::kOutput, 0, output_info));
        }
        if (output_info.channelCount < 1 || output_info.channelCount > 2) {
            throw std::runtime_error("native VST3 instrument main output is neither mono nor stereo (" +
                                     std::to_string(output_info.channelCount) + " out)");
        }
        plugin_output_channels_ = static_cast<uint16_t>(output_info.channelCount);
    }

    // Translates one MIDI message for the segment that starts at the current
    // cut: notes become events, pitch bend / CC / pressure become a
    // controller change when the plugin maps that controller.
    void queue_midi(const MidiEvent& message) {
        const auto kind = static_cast<uint8_t>(message.status & 0xF0);
        const auto channel = static_cast<int16>(message.status & 0x0F);
        if (kind == 0x90 || kind == 0x80) {
            Vst::Event event{};
            event.busIndex = 0;
            event.sampleOffset = 0;
            event.ppqPosition = 0.0;
            event.flags = Vst::Event::kIsLive;
            if (kind == 0x90 && message.data2 > 0) {
                event.type = Vst::Event::kNoteOnEvent;
                event.noteOn.channel = channel;
                event.noteOn.pitch = message.data1;
                event.noteOn.tuning = 0.0f;
                event.noteOn.velocity = static_cast<float>(message.data2) / 127.0f;
                event.noteOn.length = 0;
                event.noteOn.noteId = -1;
            } else {
                event.type = Vst::Event::kNoteOffEvent;
                event.noteOff.channel = channel;
                event.noteOff.pitch = message.data1;
                event.noteOff.velocity = static_cast<float>(message.data2) / 127.0f;
                event.noteOff.noteId = -1;
                event.noteOff.tuning = 0.0f;
            }
            if (events_.addEvent(event) != kResultOk) throw protocol_error("MIDI event list is full");
            return;
        }
        int32 controller = -1;
        double value = 0.0;
        if (kind == 0xB0) {
            controller = message.data1;
            value = static_cast<double>(message.data2) / 127.0;
        } else if (kind == 0xE0) {
            controller = Vst::kPitchBend;
            value = static_cast<double>(message.data1 | (message.data2 << 7)) / 16383.0;
        } else if (kind == 0xD0) {
            controller = Vst::kAfterTouch;
            value = static_cast<double>(message.data1) / 127.0;
        }
        if (controller < 0 || !midi_mapping_) return;
        Vst::ParamID id = 0;
        if (midi_mapping_->getMidiControllerAssignment(0, channel, static_cast<Vst::CtrlNumber>(controller), id) != kResultTrue) {
            return;
        }
        int32 queue_index = -1;
        auto* queue = changes_.addParameterData(id, queue_index);
        if (!queue) throw protocol_error("parameter queue limit exceeded");
        int32 point_index = -1;
        queue->addPoint(0, value, point_index);
    }

public:
    // Instrument: the block is cut at every parameter and MIDI offset; the
    // events at a cut are delivered with the segment that starts there.
    std::vector<float> process_instrument(const std::vector<float>& samples, uint16_t channels,
                                          const std::vector<ParameterEvent>& events,
                                          const std::vector<MidiEvent>& midi) {
        if (!instrument_) throw protocol_error("MIDI sent to an effect worker");
        if (samples.size() % channels != 0) throw protocol_error("interleaved sample shape is invalid");
        const auto frames = samples.size() / channels;
        if (frames == 0 || frames > kMaxFrames) throw protocol_error("frame count is invalid");
        std::vector<std::size_t> cuts{0};
        for (const auto& event : events) {
            if (event.offset >= frames) throw protocol_error("parameter offset exceeds frame count");
            cuts.push_back(event.offset);
        }
        for (const auto& message : midi) {
            if (message.offset >= frames) throw protocol_error("MIDI offset exceeds frame count");
            cuts.push_back(message.offset);
        }
        std::sort(cuts.begin(), cuts.end());
        cuts.erase(std::unique(cuts.begin(), cuts.end()), cuts.end());
        std::vector<std::vector<float>> output(plugin_output_channels_, std::vector<float>(frames));
        for (std::size_t cut = 0; cut < cuts.size(); ++cut) {
            const auto start = cuts[cut];
            const auto end = cut + 1 < cuts.size() ? cuts[cut + 1] : frames;
            for (const auto& event : events) {
                if (event.offset != start) continue;
                int32 queue_index = -1;
                auto* queue = changes_.addParameterData(event.id, queue_index);
                if (!queue) throw protocol_error("parameter queue limit exceeded");
                int32 point_index = -1;
                if (queue->addPoint(0, event.value, point_index) != kResultOk) {
                    throw protocol_error("parameter event could not be queued");
                }
            }
            for (const auto& message : midi) {
                if (message.offset == start) queue_midi(message);
            }
            std::vector<Vst::Sample32*> outputs;
            for (auto& channel : output) outputs.push_back(&channel[start]);
            Vst::AudioBusBuffers output_bus{};
            output_bus.numChannels = static_cast<int32>(outputs.size());
            output_bus.channelBuffers32 = outputs.data();
            Vst::ProcessData data{};
            data.processMode = Vst::kRealtime;
            data.symbolicSampleSize = Vst::kSample32;
            data.numSamples = static_cast<int32>(end - start);
            data.numInputs = 0;
            data.numOutputs = 1;
            data.inputs = nullptr;
            data.outputs = &output_bus;
            data.inputParameterChanges = changes_.getParameterCount() > 0 ? &changes_ : nullptr;
            data.inputEvents = events_.getEventCount() > 0 ? &events_ : nullptr;
            data.processContext = &context_;
            const auto result = processor_->process(data);
            events_.clear();
            if (result != kResultFalse) require_result("VST3 instrument process", result);
            advance_context(end - start);
            changes_ = ParameterChanges{};
        }
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

private:
    void process_segment(const std::vector<std::vector<float>>& input,
                         std::vector<std::vector<float>>& output, std::size_t start, std::size_t end,
                         const std::vector<ParameterEvent>& events, std::size_t& event_index) {
        drain_pending_snapshot();
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
        data.processContext = &context_;
        const auto result = processor_->process(data);
        if (result != kResultFalse) require_result("VST3 process", result);
        advance_context(end - start);
        changes_ = ParameterChanges{};
    }

    void process_multi_segment(
        const std::vector<std::vector<std::vector<float>>>& input,
        std::vector<std::vector<std::vector<float>>>& output, std::size_t start, std::size_t end,
        const std::vector<ParameterEvent>& events, std::size_t& event_index) {
        drain_pending_snapshot();
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
        data.processContext = &context_;
        const auto result = processor_->process(data);
        if (result != kResultFalse) require_result("VST3 multi-bus process", result);
        advance_context(end - start);
        changes_ = ParameterChanges{};
    }

    void advance_context(std::size_t frames) {
        context_.projectTimeSamples += static_cast<int64>(frames);
        context_.projectTimeMusic = context_.projectTimeSamples / context_.sampleRate * context_.tempo / 60.0;
    }
    Vst::ProcessContext context_{};
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
    std::vector<std::pair<Vst::ParamID, double>> pending_snapshot_;
    bool instrument_ = false;
    Vst::IMidiMapping* midi_mapping_ = nullptr;
    EventList events_;
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

static std::string bus_descriptors_message(Vst::IComponent* component) {
    std::ostringstream output;
    output << "{\"type\":\"Buses\",\"payload\":{";
    for (const auto direction : {Vst::kInput, Vst::kOutput}) {
        output << (direction == Vst::kInput ? "\"inputs\":[" : ",\"outputs\":[");
        const auto count = component->getBusCount(Vst::kAudio, direction);
        if (count < 1 || count > 4) throw protocol_error("unsupported VST3 audio bus count");
        bool first = true;
        for (int32 index = 0; index < count; ++index) {
            Vst::BusInfo info{};
            require_result("discovered bus info", component->getBusInfo(Vst::kAudio, direction, index, info));
            // Additional main buses are not sidechains. Keep native indices
            // even when an unsupported bus is omitted from this description.
            if (index > 0 && (direction == Vst::kOutput || info.busType != Vst::kAux)) continue;
            if ((index == 0 && info.busType != Vst::kMain) ||
                info.channelCount < 1 || info.channelCount > 2) {
                throw protocol_error("unsupported VST3 audio bus role or channel count");
            }
            if (!first) output << ',';
            first = false;
            output << "{\"nativeIndex\":" << index << ",\"name\":" << quote_vst3_title(info.name)
                   << ",\"channels\":" << info.channelCount << ",\"role\":\""
                   << (index == 0 ? "main" : "auxiliary") << "\"}";
        }
        output << ']';
    }
    output << "}}";
    return output.str();
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
        if (controller->getParameterInfo(index, info) != kResultOk) continue;
        if (!parameter_ids.insert(info.id).second) {
            continue;
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
        // One sloppy default must not cost the user every slider: bring it
        // into the normalized range instead of failing the whole list.
        const double default_value = std::isfinite(info.defaultNormalizedValue)
            ? std::clamp(info.defaultNormalizedValue, 0.0, 1.0) : 0.0;
        if (exposed++ != 0) output << ',';
        output << "{\"parameter_id\":" << info.id << ",\"title\":"
               << quote_vst3_title(info.title) << ",\"default_value\":"
               << default_value << ",\"minimum\":0,\"maximum\":1}";
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
constexpr UINT kLatencyRestartMessage = WM_APP + 6; // posted: re-activate for a latency change
constexpr UINT_PTR kParentSizeTimer = 1;
constexpr UINT kParentSizePollMs = 200;
// A UI thread that does not answer a request within this time is stuck in
// plugin code; the worker then ends so the supervisor can contain it. It is
// shorter than the host's 30 s control timeout (`WORKER_CONTROL_TIMEOUT`,
// pinned by `vst3_worker_ui_timeout_is_shorter_than_the_host_timeout`), so
// the worker reports the stuck UI thread before the host gives up on it.
constexpr UINT kUiRequestTimeoutMs = 25000;
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
            // Attach must remain asynchronous: the shell's UI thread is
            // awaiting EditorOpened and cannot service a synchronous child
            // attach yet. Display failures in its explicitly authorized parent
            // rather than leaving an unexplained blank editor window.
            const std::wstring text = L"This plugin could not open its editor (VST3 result " + std::to_wstring(result) + L"). Close this window and use the parameter controls in AudioRouter. Check the plugin's installation and licence before trying again.";
            RECT area{};
            GetClientRect(parent_, &area);
            CreateWindowExW(0, L"STATIC", text.c_str(), WS_CHILD | WS_VISIBLE | SS_LEFT,
                            16, 16, std::max(120L, area.right - 32), std::max(80L, area.bottom - 32), parent_, nullptr, GetModuleHandleW(nullptr), nullptr);
            SetWindowTextW(parent_, L"Plugin editor could not open");
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
    std::string class_id;
    std::string hash;
    uint16_t channels = 0;
    double sample_rate = 0.0;
    std::vector<uint16_t> input_buses;
    std::vector<uint16_t> output_buses;
    // Hosts an instrument played with MIDI instead of an effect.
    bool instrument = false;
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
        else if (argument == L"--class-id" && index + 1 < argc) {
            const std::wstring supplied(argv[++index]);
            result.class_id.assign(supplied.begin(), supplied.end());
            if (result.class_id.size() != 32 || !std::all_of(result.class_id.begin(), result.class_id.end(), [](unsigned char c) { return std::isxdigit(c); })) throw protocol_error("invalid VST3 class ID");
            std::transform(result.class_id.begin(), result.class_id.end(), result.class_id.begin(), [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
        }
        else if (argument == L"--plugin-sha256" && index + 1 < argc) {
            const std::wstring hash(argv[++index]);
            result.hash.assign(hash.begin(), hash.end());
        }
        else if (argument == L"--channels" && index + 1 < argc) result.channels = static_cast<uint16_t>(std::stoul(argv[++index]));
        else if (argument == L"--sample-rate" && index + 1 < argc) result.sample_rate = std::stod(argv[++index]);
        else if (argument == L"--input-buses" && index + 1 < argc) result.input_buses = parse_bus_list(argv[++index]);
        else if (argument == L"--output-buses" && index + 1 < argc) result.output_buses = parse_bus_list(argv[++index]);
        else if (argument == L"--instrument") result.instrument = true;
        else throw std::runtime_error("unknown or incomplete worker argument");
    }
    if (result.plugin.empty() || result.channels < 1 || result.channels > 2 ||
        !std::isfinite(result.sample_rate) || result.sample_rate < 8000.0 || result.sample_rate > 384000.0) {
        throw std::runtime_error("invalid native VST3 worker arguments");
    }
    if (result.input_buses.empty() != result.output_buses.empty()) {
        throw std::runtime_error("input and output bus lists must be supplied together");
    }
    if (result.instrument && !result.input_buses.empty()) {
        throw std::runtime_error("an instrument worker is single-stream");
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
    if (json == "{\"type\":\"DescribeBuses\"}") {
        return bus_descriptors_message(effect.component());
    }
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
        try { return state_message(1, effect.save_state()); }
        catch (const std::exception& error) {
            std::cerr << "VST3 state save refused: " << error.what() << '\n';
            return "{\"type\":\"Failure\",\"payload\":{\"code\":\"vst3StateSaveDeclined\"}}";
        }
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
        if (g_latency_restart_pending.exchange(false, std::memory_order_acq_rel)) {
            // The pipe thread waits for this request, so the lock is free;
            // taking it keeps the rule that a restart never overlaps Process.
            std::lock_guard<std::mutex> processing(g_processing_mutex);
            effect.restart_for_latency();
        }
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
                // State and editor requests can change the plugin's delay.
                // The pipe thread is waiting, so reading it here cannot
                // overlap Process; the next poll answers from this value.
                if (!request->shutdown && !is_latency_message(*request->json)) {
                    g_latency_samples.store(g_ui.effect->latency_samples(), std::memory_order_release);
                    g_latency_cached.store(true, std::memory_order_release);
                }
            } catch (const std::exception& error) {
                request->error = error.what();
            }
            return 0;
        }
        case kLatencyRestartMessage:
            // Posted by the pipe thread after the plugin asked for a latency
            // restart. Runs whenever this thread is free; audio keeps flowing
            // until then and waits only while the component re-activates.
            try {
                std::lock_guard<std::mutex> processing(g_processing_mutex);
                g_ui.effect->restart_for_latency();
                g_latency_samples.store(g_ui.effect->latency_samples(), std::memory_order_release);
            } catch (const std::exception& error) {
                std::cerr << "VST3 latency restart failed: " << error.what() << '\n';
                g_latency_restart_failed.store(true, std::memory_order_release);
            }
            return 0;
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

// One-way MIDI for the next Process message (instrument workers).
static bool is_midi_message(const std::string& json) {
    return json.rfind("{\"type\":\"Midi\"", 0) == 0;
}

// Reads every host message. Audio is processed here, so a busy plugin window
// never delays a quantum; everything else is answered by the UI thread.
static int audio_thread_main(const Arguments& arguments, const std::vector<uint16_t>& input_buses,
                             const std::vector<uint16_t>& output_buses, bool multi_bus) {
    AudioThreadScheduling scheduling;
    auto& effect = *g_ui.effect;
    std::vector<MidiEvent> pending_midi;
    while (true) {
        const auto payload = read_frame();
        const std::string json(payload.begin(), payload.end());
        if (is_midi_message(json)) {
            // No reply. An effect worker drops the events. An instrument
            // rejects overflow rather than evicting note-offs and risking a
            // stuck note.
            if (arguments.instrument) {
                auto more = parse_midi_events(json);
                if (more.size() > kMaxMidiEvents - std::min(kMaxMidiEvents, pending_midi.size())) {
                    throw protocol_error("pending MIDI event backlog exceeds the bounded contract");
                }
                pending_midi.insert(pending_midi.end(), more.begin(), more.end());
            }
            continue;
        }
        if (g_latency_restart_failed.load(std::memory_order_acquire)) {
            throw std::runtime_error("VST3 latency restart failed");
        }
        if (!is_audio_message(json)) {
            if (is_latency_message(json) && g_latency_cached.load(std::memory_order_acquire)) {
                // Never wait for the UI thread here: answer the last known
                // delay and hand a requested restart to the UI thread.
                if (g_latency_restart_pending.exchange(false, std::memory_order_acq_rel) &&
                    !PostMessageW(g_ui_window, kLatencyRestartMessage, 0, 0)) {
                    g_latency_restart_pending.store(true, std::memory_order_release);
                }
                write_frame(latency_message(g_latency_samples.load(std::memory_order_acquire),
                                            required_uint_field(json, "\"sample_rate_hz\":")));
                continue;
            }
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
            const auto output = [&] {
                std::lock_guard<std::mutex> processing(g_processing_mutex);
                return effect.process_buses(frames, events);
            }();
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
            const auto block_midi = std::exchange(pending_midi, {});
            const auto output = [&] {
                std::lock_guard<std::mutex> processing(g_processing_mutex);
                return arguments.instrument
                           ? effect.process_instrument(samples, arguments.channels, events, block_midi)
                           : effect.process(samples, arguments.channels, events);
            }();
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

static int self_test_host() {
    HostApplication host;
    void* queried = nullptr;
    if (host.queryInterface(Vst::IHostApplication_iid, &queried) != kResultOk || !queried) throw std::runtime_error("host QI regression");
    auto* application = static_cast<Vst::IHostApplication*>(queried);
    application->release();
    void* created = nullptr;
    TUID message_id; std::memcpy(message_id, Vst::IMessage_iid, sizeof(TUID));
    if (host.createInstance(message_id, message_id, &created) != kResultOk || !created) throw std::runtime_error("host message regression");
    auto* message = static_cast<Vst::IMessage*>(created);
    message->setMessageID("AudioRouter fixture");
    auto* attributes = message->getAttributes();
    attributes->setInt("integer", 42);
    attributes->setFloat("float", 0.5);
    attributes->setString("string", u"test");
    const uint8_t binary[]{1, 2, 255};
    attributes->setBinary("bytes", binary, sizeof(binary));
    int64 integer = 0; double number = 0; Vst::TChar text[16]{}; const void* data = nullptr; uint32 bytes = 0;
    if (attributes->getInt("integer", integer) != kResultOk || integer != 42 ||
        attributes->getFloat("float", number) != kResultOk || number != 0.5 ||
        attributes->getString("string", text, sizeof(text)) != kResultOk || text[0] != u't' ||
        attributes->getBinary("bytes", data, bytes) != kResultOk || bytes != 3 || std::memcmp(data, binary, 3)) throw std::runtime_error("host attributes regression");
    if (attributes->setBinary("oversized", binary, 1024 * 1024 + 1) != kInvalidArgument) throw std::runtime_error("unbounded attribute");
    message->release();
    StateStream stream;
    if (stream.queryInterface(IBStream_iid, &queried) != kResultOk || !queried) throw std::runtime_error("stream QI regression");
    static_cast<IBStream*>(queried)->release();
    ParameterChanges changes;
    if (changes.queryInterface(Vst::IParameterChanges_iid, &queried) != kResultOk) throw std::runtime_error("parameter QI regression");
    static_cast<Vst::IParameterChanges*>(queried)->release();
    ComponentHandler handler;
    if (handler.queryInterface(Vst::IComponentHandler_iid, &queried) != kResultOk) throw std::runtime_error("handler QI regression");
    static_cast<Vst::IComponentHandler*>(queried)->release();
    if (handler.queryInterface(Vst::IComponentHandler2_iid, &queried) != kResultOk || !queried) throw std::runtime_error("handler2 QI regression");
    auto* handler2 = static_cast<Vst::IComponentHandler2*>(queried);
    if (handler2->setDirty(1) != kResultOk || handler2->startGroupEdit() != kResultOk ||
        handler2->finishGroupEdit() != kResultOk || handler2->requestOpenEditor(Vst::ViewType::kEditor) != kResultFalse) {
        throw std::runtime_error("handler2 answers regression");
    }
    if (!restart_needs_reactivation(Vst::kLatencyChanged) || !restart_needs_reactivation(Vst::kIoChanged) ||
        !restart_needs_reactivation(Vst::kReloadComponent) || restart_needs_reactivation(Vst::kParamValuesChanged) ||
        restart_needs_reactivation(Vst::kParamTitlesChanged) || restart_needs_reactivation(0)) {
        throw std::runtime_error("restart flag regression");
    }
    g_latency_restart_pending.store(false);
    handler.restartComponent(Vst::kParamValuesChanged);
    if (g_latency_restart_pending.load()) throw std::runtime_error("a value change must not restart the component");
    handler.restartComponent(Vst::kIoChanged);
    if (!g_latency_restart_pending.exchange(false)) throw std::runtime_error("an I/O change must schedule a re-activation");
    require_notification("optional notification", kResultFalse);
    const TUID sample_class{0x01, 0x23, 0x45, 0x67, char(0x89), char(0xAB), char(0xCD), char(0xEF),
                            0x10, 0x32, 0x54, 0x76, char(0x98), char(0xBA), char(0xDC), char(0xFE)};
    if (!class_id_matches(sample_class, "67452301AB89EFCD1032547698BADCFE") ||
        !class_id_matches(sample_class, "0123456789ABCDEF1032547698BADCFE") ||
        class_id_matches(sample_class, "00000000000000000000000000000000")) {
        throw std::runtime_error("class ID spelling regression");
    }
    std::cout << "host interfaces, message attributes, bounds and optional results passed\n";
    return 0;
}

int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && std::wstring(argv[1]) == L"--self-test-host") return self_test_host();
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
            Vst3Effect effect(arguments.plugin, arguments.sample_rate,
                              arguments.instrument ? std::vector<uint16_t>{} : input_buses, output_buses,
                              arguments.class_id, arguments.instrument);
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
