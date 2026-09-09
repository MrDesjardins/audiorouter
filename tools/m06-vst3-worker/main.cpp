#define NOMINMAX

#include <windows.h>

#include <algorithm>
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
#include <optional>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"

namespace fs = std::filesystem;
using namespace Steinberg;
using GetPluginFactoryProc = IPluginFactory* (PLUGIN_API*)();

constexpr std::size_t kMaxMessageBytes = 4 * 1024 * 1024;
constexpr std::size_t kMaxFrames = 4096;
constexpr std::size_t kMaxParameters = 64;
constexpr uint16_t kProtocolVersion = 1;

static std::runtime_error protocol_error(const std::string& message) {
    return std::runtime_error("protocol: " + message);
}

static std::vector<uint8_t> read_frame() {
    uint8_t header[4]{};
    if (!std::cin.read(reinterpret_cast<char*>(header), sizeof(header))) {
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
    if (!std::cin.read(reinterpret_cast<char*>(payload.data()),
                       static_cast<std::streamsize>(payload.size()))) {
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
    std::cout.write(reinterpret_cast<const char*>(header), sizeof(header));
    std::cout.write(payload.data(), static_cast<std::streamsize>(payload.size()));
    std::cout.flush();
    if (!std::cout) {
        throw std::runtime_error("worker output write failed");
    }
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
        const auto get_factory = reinterpret_cast<GetPluginFactoryProc>(
            GetProcAddress(module_, "GetPluginFactory"));
        if (!get_factory) throw std::runtime_error("VST3 GetPluginFactory export is missing");
        factory_ = get_factory();
        if (!factory_) throw std::runtime_error("VST3 factory is null");
        for (int32 index = 0; index < factory_->countClasses(); ++index) {
            PClassInfo info{};
            require_result("getClassInfo", factory_->getClassInfo(index, &info));
            if (std::strcmp(info.category, kVstAudioEffectClass) != 0) continue;
            if (factory_->createInstance(info.cid, Vst::IComponent_iid,
                                         reinterpret_cast<void**>(&component_)) != kResultOk) {
                continue;
            }
            if (component_->initialize(nullptr) != kResultOk) {
                component_->release();
                component_ = nullptr;
                continue;
            }
            if (component_->getBusCount(Vst::kAudio, Vst::kInput) !=
                    static_cast<int32>(input_channels.size()) ||
                component_->getBusCount(Vst::kAudio, Vst::kOutput) !=
                    static_cast<int32>(output_channels.size())) {
                component_->terminate();
                component_->release();
                component_ = nullptr;
                continue;
            }
            break;
        }
        if (!component_) throw std::runtime_error("VST3 bundle has no audio effect class");
        initialized_ = true;
        TUID controller_id{};
        require_result("controller class id", component_->getControllerClassId(controller_id));
        require_result("controller creation", factory_->createInstance(
            controller_id, Vst::IEditController_iid,
            reinterpret_cast<void**>(&controller_)));
        require_result("controller initialize", controller_->initialize(nullptr));
        controller_initialized_ = true;
        for (std::size_t bus = 0; bus < input_channels.size(); ++bus) {
            Vst::BusInfo info{};
            require_result("input bus info", component_->getBusInfo(
                Vst::kAudio, Vst::kInput, static_cast<int32>(bus), info));
            if (info.channelCount != input_channels[bus]) {
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
            if (info.channelCount != output_channels[bus]) {
                throw std::runtime_error("native VST3 output bus channels do not match layout");
            }
            require_result("output bus activation", component_->activateBus(
                Vst::kAudio, Vst::kOutput, static_cast<int32>(bus), true));
        }
        require_result("processor interface", component_->queryInterface(
            Vst::IAudioProcessor_iid, reinterpret_cast<void**>(&processor_)));
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
        if (module_) FreeLibrary(module_);
    }

    std::vector<float> process(const std::vector<float>& samples, uint16_t channels,
                               const std::vector<ParameterEvent>& events) {
        if (samples.size() % channels != 0) throw protocol_error("interleaved sample shape is invalid");
        const auto frames = samples.size() / channels;
        if (frames == 0 || frames > kMaxFrames) throw protocol_error("frame count is invalid");
        std::vector<std::vector<float>> input(channels, std::vector<float>(frames));
        std::vector<std::vector<float>> output(channels, std::vector<float>(frames));
        for (std::size_t index = 0; index < samples.size(); ++index) {
            input[index % channels][index / channels] = samples[index];
        }
        std::vector<ParameterEvent> ordered = events;
        std::stable_sort(ordered.begin(), ordered.end(),
                         [](const auto& left, const auto& right) { return left.offset < right.offset; });
        std::size_t start = 0;
        std::size_t event_index = 0;
        while (event_index < ordered.size()) {
            const auto offset = ordered[event_index].offset;
            if (offset >= frames) throw protocol_error("parameter offset exceeds frame count");
            process_segment(input, output, channels, start, offset, ordered, event_index);
            start = offset;
        }
        process_segment(input, output, channels, start, frames, ordered, event_index);
        std::vector<float> result(samples.size());
        for (std::size_t index = 0; index < result.size(); ++index) {
            const auto sample = output[index % channels][index / channels];
            if (!std::isfinite(sample)) throw std::runtime_error("VST3 produced non-finite output");
            result[index] = sample;
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

private:
    void process_segment(const std::vector<std::vector<float>>& input,
                         std::vector<std::vector<float>>& output, uint16_t channels,
                         std::size_t start, std::size_t end,
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
        for (uint16_t channel = 0; channel < channels; ++channel) {
            inputs.push_back(const_cast<float*>(&input[channel][start]));
            outputs.push_back(&output[channel][start]);
        }
        Vst::AudioBusBuffers input_bus{};
        input_bus.numChannels = channels;
        input_bus.channelBuffers32 = inputs.data();
        Vst::AudioBusBuffers output_bus{};
        output_bus.numChannels = channels;
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
    IPluginFactory* factory_ = nullptr;
    Vst::IComponent* component_ = nullptr;
    Vst::IAudioProcessor* processor_ = nullptr;
    Vst::IEditController* controller_ = nullptr;
    bool initialized_ = false;
    bool controller_initialized_ = false;
    bool active_ = false;
    bool processing_ = false;
    std::vector<uint16_t> requested_input_channels_;
    std::vector<uint16_t> requested_output_channels_;
    ParameterChanges changes_;
};

static std::string quote_json(const std::string& value) {
    std::string result = "\"";
    for (const auto character : value) {
        if (character == '\\' || character == '"') result.push_back('\\');
        result.push_back(character);
    }
    result.push_back('"');
    return result;
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

int wmain(int argc, wchar_t** argv) {
    try {
        if (_setmode(_fileno(stdin), _O_BINARY) == -1 ||
            _setmode(_fileno(stdout), _O_BINARY) == -1) {
            throw std::runtime_error("could not set worker pipes to binary mode");
        }
        const auto arguments = parse_arguments(argc, argv);
        const auto input_buses = arguments.input_buses.empty()
                                     ? std::vector<uint16_t>{arguments.channels}
                                     : arguments.input_buses;
        const auto output_buses = arguments.output_buses.empty()
                                      ? std::vector<uint16_t>{arguments.channels}
                                      : arguments.output_buses;
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
        while (true) {
            const auto payload = read_frame();
            const std::string json(payload.begin(), payload.end());
            if (json == "{\"type\":\"Shutdown\"}") return 0;
            const auto events = parse_parameters(json);
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
        }
    } catch (const std::exception& error) {
        std::cerr << "native VST3 worker stopped: " << error.what() << '\n';
        return 1;
    }
}
