#define NOMINMAX

#include <windows.h>

#include <algorithm>
#include <cstdint>
#include <filesystem>
#include <cmath>
#include <cstring>
#include <iostream>
#include <optional>
#include <sstream>
#include <string>
#include <stdexcept>
#include <vector>

#include "pluginterfaces/base/ibstream.h"
#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include "pluginterfaces/vst/vsttypes.h"

namespace fs = std::filesystem;
using namespace Steinberg;
using GetPluginFactoryProc = IPluginFactory* (PLUGIN_API*)();

template <std::size_t N>
static std::string bounded_ascii(const Vst::TChar (&value)[N]) {
    std::string result;
    result.reserve(N);
    for (const auto character : value) {
        if (character == 0) {
            break;
        }
        result.push_back(character >= 32 && character <= 126
                             ? static_cast<char>(character)
                             : '?');
    }
    return result;
}

class MemoryStream final : public IBStream {
public:
    tresult PLUGIN_API queryInterface(const TUID, void** object) override {
        if (object) {
            *object = nullptr;
        }
        return kNoInterface;
    }

    uint32 PLUGIN_API addRef() override { return ++references; }

    uint32 PLUGIN_API release() override {
        const auto remaining = --references;
        if (remaining == 0) {
            delete this;
        }
        return remaining;
    }

    tresult PLUGIN_API read(void* buffer, int32 bytes, int32* bytes_read) override {
        if (bytes < 0 || !buffer) {
            return kInvalidArgument;
        }
        const auto available = data.size() - std::min(position, data.size());
        const auto count = std::min<std::size_t>(available, static_cast<std::size_t>(bytes));
        std::memcpy(buffer, data.data() + position, count);
        position += count;
        if (bytes_read) {
            *bytes_read = static_cast<int32>(count);
        }
        return kResultOk;
    }

    tresult PLUGIN_API write(void* buffer, int32 bytes, int32* bytes_written) override {
        if (bytes < 0 || (bytes > 0 && !buffer)) {
            return kInvalidArgument;
        }
        const auto count = static_cast<std::size_t>(bytes);
        if (position > data.size() - std::min(position, data.size())) {
            data.resize(position);
        }
        if (position + count > data.size()) {
            data.resize(position + count);
        }
        std::memcpy(data.data() + position, buffer, count);
        position += count;
        if (bytes_written) {
            *bytes_written = bytes;
        }
        return kResultOk;
    }

    tresult PLUGIN_API seek(int64 offset, int32 mode, int64* result) override {
        int64 base = 0;
        if (mode == IBStream::kIBSeekCur) {
            base = static_cast<int64>(position);
        } else if (mode == IBStream::kIBSeekEnd) {
            base = static_cast<int64>(data.size());
        } else if (mode != IBStream::kIBSeekSet) {
            return kInvalidArgument;
        }
        if (offset < -base || base + offset < 0) {
            return kInvalidArgument;
        }
        position = static_cast<std::size_t>(base + offset);
        if (result) {
            *result = static_cast<int64>(position);
        }
        return kResultOk;
    }

    tresult PLUGIN_API tell(int64* position_out) override {
        if (!position_out) {
            return kInvalidArgument;
        }
        *position_out = static_cast<int64>(position);
        return kResultOk;
    }

    std::size_t size() const { return data.size(); }

private:
    uint32 references = 1;
    std::vector<uint8_t> data;
    std::size_t position = 0;
};

class SingleParameterQueue final : public Vst::IParamValueQueue {
public:
    explicit SingleParameterQueue(Vst::ParamID id) : id(id) {}

    tresult PLUGIN_API queryInterface(const TUID, void** object) override {
        if (object) {
            *object = nullptr;
        }
        return kNoInterface;
    }

    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    Vst::ParamID PLUGIN_API getParameterId() override { return id; }
    int32 PLUGIN_API getPointCount() override { return has_point ? 1 : 0; }

    tresult PLUGIN_API getPoint(int32 index, int32& sample_offset,
                                Vst::ParamValue& value) override {
        if (index != 0 || !has_point) {
            return kInvalidArgument;
        }
        sample_offset = offset;
        value = normalized_value;
        return kResultOk;
    }

    tresult PLUGIN_API addPoint(int32 sample_offset, Vst::ParamValue value,
                                int32& index) override {
        if (sample_offset < 0 || !std::isfinite(value) || value < 0.0 || value > 1.0) {
            return kInvalidArgument;
        }
        offset = sample_offset;
        normalized_value = value;
        has_point = true;
        index = 0;
        return kResultOk;
    }

private:
    Vst::ParamID id;
    int32 offset = 0;
    Vst::ParamValue normalized_value = 0.0;
    bool has_point = false;
};

class SingleParameterChanges final : public Vst::IParameterChanges {
public:
    tresult PLUGIN_API queryInterface(const TUID, void** object) override {
        if (object) {
            *object = nullptr;
        }
        return kNoInterface;
    }

    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
    int32 PLUGIN_API getParameterCount() override { return queue ? 1 : 0; }

    Vst::IParamValueQueue* PLUGIN_API getParameterData(int32 index) override {
        return index == 0 && queue ? &*queue : nullptr;
    }

    Vst::IParamValueQueue* PLUGIN_API addParameterData(const Vst::ParamID& id,
                                                       int32& index) override {
        if (queue) {
            return nullptr;
        }
        queue.emplace(id);
        index = 0;
        return &*queue;
    }

private:
    std::optional<SingleParameterQueue> queue;
};

static fs::path resolve_binary(const fs::path& supplied) {
    if (!fs::is_directory(supplied)) {
        return supplied;
    }
    const auto contents = supplied / "Contents" / "x86_64-win";
    fs::path result;
    for (const auto& entry : fs::directory_iterator(contents)) {
        if (entry.is_regular_file()) {
            if (!result.empty()) {
                throw std::runtime_error("bundle contains multiple x64 binaries");
            }
            result = entry.path();
        }
    }
    if (result.empty()) {
        throw std::runtime_error("bundle contains no x64 binary");
    }
    return result;
}

static void require_result(const char* operation, tresult result) {
    if (result == kResultOk) {
        return;
    }
    std::ostringstream message;
    message << operation << " failed with VST3 result 0x" << std::hex
            << static_cast<uint32>(result);
    throw std::runtime_error(message.str());
}

static void require_processing_result(const char* operation, tresult result) {
    // VST3 permits a processor to leave this optional lifecycle hook at the
    // SDK default. The pinned Steinberg AGain sample does so and still
    // processes valid audio; kNotImplemented must not be treated as failed
    // activation, while all other failures remain fatal.
    if (result == kResultOk || result == kNotImplemented) {
        return;
    }
    require_result(operation, result);
}

int wmain(int argc, wchar_t** argv) {
    if (argc < 2 || argc > 5) {
        std::wcerr << L"usage: m06-vst3-loader <plugin.vst3|binary> [--class-index <n>] [--multi-bus]\n";
        return 2;
    }

    int32 selected_class_index = -1;
    bool allow_multi_bus = false;
    for (int argument_index = 2; argument_index < argc; ++argument_index) {
        if (std::wstring(argv[argument_index]) == L"--multi-bus") {
            allow_multi_bus = true;
            continue;
        }
        if (std::wstring(argv[argument_index]) != L"--class-index" ||
            argument_index + 1 >= argc ||
            std::wstring(argv[argument_index + 1]) == L"--multi-bus") {
            std::wcerr << L"unknown option\n";
            return 2;
        }
        try {
            const auto parsed = std::stol(argv[++argument_index]);
            if (parsed < 0 || parsed > INT32_MAX) {
                throw std::out_of_range("class index");
            }
            selected_class_index = static_cast<int32>(parsed);
        } catch (const std::exception&) {
            std::wcerr << L"class index must be a non-negative integer\n";
            return 2;
        }
    }

    HMODULE module = nullptr;
    IPluginFactory* factory = nullptr;
    Vst::IComponent* component = nullptr;
    Vst::IAudioProcessor* processor = nullptr;
    Vst::IEditController* controller = nullptr;
    bool component_initialized = false;
    bool component_active = false;
    bool processor_active = false;
    bool processed_audio_effect = false;
    try {
        const auto binary = fs::absolute(resolve_binary(argv[1]));
        if (!fs::is_regular_file(binary)) {
            throw std::runtime_error("resolved plugin binary is not a regular file");
        }
        constexpr DWORD load_library_search_dll_load_dir = 0x00000100;
        constexpr DWORD load_library_search_default_dirs = 0x00001000;
        module = LoadLibraryExW(
            binary.c_str(), nullptr,
            load_library_search_dll_load_dir | load_library_search_default_dirs);
        if (!module) {
            throw std::runtime_error("LoadLibraryExW with restricted search paths failed");
        }
        const auto get_factory = reinterpret_cast<GetPluginFactoryProc>(
            GetProcAddress(module, "GetPluginFactory"));
        if (!get_factory) {
            throw std::runtime_error("GetPluginFactory export is missing");
        }
        factory = get_factory();
        if (!factory) {
            throw std::runtime_error("GetPluginFactory returned null");
        }

        PFactoryInfo factory_info{};
        if (factory->getFactoryInfo(&factory_info) != kResultOk) {
            throw std::runtime_error("getFactoryInfo failed");
        }
        const auto classes = factory->countClasses();
        if (classes <= 0) {
            throw std::runtime_error("factory exposes no classes");
        }
        std::wcout << L"factory loaded: " << binary << L"\n";
        std::cout << "classes: " << classes << "\n";
        for (int32 index = 0; index < classes; ++index) {
            PClassInfo info{};
            if (factory->getClassInfo(index, &info) != kResultOk) {
                throw std::runtime_error("getClassInfo failed");
            }
            std::cout << "class[" << index << "] category=" << info.category
                      << " name=" << info.name << "\n";
        }
        for (int32 index = 0; index < classes; ++index) {
            PClassInfo info{};
            if (factory->getClassInfo(index, &info) != kResultOk) {
                throw std::runtime_error("getClassInfo failed");
            }
            if (selected_class_index >= 0 && index != selected_class_index) {
                continue;
            }
            if (std::strcmp(info.category, kVstAudioEffectClass) == 0) {
                if (factory->createInstance(
                        info.cid, Vst::IComponent_iid, reinterpret_cast<void**>(&component)) !=
                    kResultOk) {
                    throw std::runtime_error("component createInstance failed");
                }
                if (component->initialize(nullptr) != kResultOk) {
                    throw std::runtime_error("component initialize failed");
                }
                component_initialized = true;
                const auto inputs = component->getBusCount(Vst::kAudio, Vst::kInput);
                const auto outputs = component->getBusCount(Vst::kAudio, Vst::kOutput);
                if ((!allow_multi_bus && (inputs != 1 || outputs != 1)) ||
                    inputs < 1 || inputs > 4 || outputs < 1 || outputs > 4) {
                    throw std::runtime_error("probe requires one input and output bus");
                }
                if (component->queryInterface(
                        Vst::IAudioProcessor_iid, reinterpret_cast<void**>(&processor)) !=
                    kResultOk) {
                    throw std::runtime_error("audio processor interface is missing");
                }
                std::vector<Vst::BusInfo> input_infos(static_cast<std::size_t>(inputs));
                std::vector<Vst::BusInfo> output_infos(static_cast<std::size_t>(outputs));
                for (int32 bus = 0; bus < inputs; ++bus) {
                    if (component->getBusInfo(Vst::kAudio, Vst::kInput, bus,
                                              input_infos[static_cast<std::size_t>(bus)]) !=
                        kResultOk ||
                        input_infos[static_cast<std::size_t>(bus)].channelCount < 1 ||
                        input_infos[static_cast<std::size_t>(bus)].channelCount > 2) {
                        throw std::runtime_error("unsupported input audio bus layout");
                    }
                    require_result("input audio bus activation",
                                   component->activateBus(Vst::kAudio, Vst::kInput, bus, true));
                }
                for (int32 bus = 0; bus < outputs; ++bus) {
                    if (component->getBusInfo(Vst::kAudio, Vst::kOutput, bus,
                                              output_infos[static_cast<std::size_t>(bus)]) !=
                        kResultOk ||
                        output_infos[static_cast<std::size_t>(bus)].channelCount < 1 ||
                        output_infos[static_cast<std::size_t>(bus)].channelCount > 2) {
                        throw std::runtime_error("unsupported output audio bus layout");
                    }
                    require_result("output audio bus activation",
                                   component->activateBus(Vst::kAudio, Vst::kOutput, bus, true));
                }
                Vst::ProcessSetup setup{};
                setup.processMode = Vst::kOffline;
                setup.symbolicSampleSize = Vst::kSample32;
                setup.maxSamplesPerBlock = 64;
                setup.sampleRate = 48000.0;
                require_result("setupProcessing", processor->setupProcessing(setup));
                require_result("component activation", component->setActive(true));
                component_active = true;
                require_processing_result("processor activation", processor->setProcessing(true));
                processor_active = true;
                TUID controller_id{};
                if (component->getControllerClassId(controller_id) != kResultOk ||
                    factory->createInstance(
                        controller_id, Vst::IEditController_iid,
                        reinterpret_cast<void**>(&controller)) != kResultOk) {
                    throw std::runtime_error("controller createInstance failed");
                }
                if (controller->initialize(nullptr) != kResultOk) {
                    throw std::runtime_error("controller initialize failed");
                }
                const auto parameter_count = controller->getParameterCount();
                constexpr int32 max_parameter_descriptors = 256;
                if (parameter_count < 0 || parameter_count > max_parameter_descriptors) {
                    throw std::runtime_error("parameter descriptor count exceeds bounded contract");
                }
                SingleParameterChanges parameter_changes;
                if (parameter_count > 0) {
                    Vst::ParameterInfo parameter{};
                    if (controller->getParameterInfo(0, parameter) != kResultOk) {
                        throw std::runtime_error("getParameterInfo failed");
                    }
                    int32 queue_index = -1;
                    auto* queue = parameter_changes.addParameterData(parameter.id, queue_index);
                    if (!queue || queue_index != 0 ||
                        queue->addPoint(0, 0.5, queue_index) != kResultOk) {
                        throw std::runtime_error("parameter change construction failed");
                    }
                }
                std::vector<std::vector<std::vector<float>>> input_storage;
                std::vector<std::vector<std::vector<float>>> output_storage;
                std::vector<std::vector<Vst::Sample32*>> input_channels;
                std::vector<std::vector<Vst::Sample32*>> output_channels;
                std::vector<Vst::AudioBusBuffers> input_buses;
                std::vector<Vst::AudioBusBuffers> output_buses;
                for (const auto& info : input_infos) {
                    input_storage.emplace_back();
                    input_channels.emplace_back();
                    for (int32 channel = 0; channel < info.channelCount; ++channel) {
                        input_storage.back().emplace_back(64, 0.25f);
                        input_channels.back().push_back(input_storage.back().back().data());
                    }
                    input_buses.emplace_back();
                    input_buses.back().numChannels = info.channelCount;
                    input_buses.back().channelBuffers32 = input_channels.back().data();
                }
                for (const auto& info : output_infos) {
                    output_storage.emplace_back();
                    output_channels.emplace_back();
                    for (int32 channel = 0; channel < info.channelCount; ++channel) {
                        output_storage.back().emplace_back(64, 0.0f);
                        output_channels.back().push_back(output_storage.back().back().data());
                    }
                    output_buses.emplace_back();
                    output_buses.back().numChannels = info.channelCount;
                    output_buses.back().channelBuffers32 = output_channels.back().data();
                }
                Vst::ProcessData data{};
                data.processMode = Vst::kOffline;
                data.symbolicSampleSize = Vst::kSample32;
                data.numSamples = 64;
                data.numInputs = inputs;
                data.numOutputs = outputs;
                data.inputs = input_buses.data();
                data.outputs = output_buses.data();
                data.inputParameterChanges = parameter_count > 0 ? &parameter_changes : nullptr;
                require_result("processor process", processor->process(data));
                for (const auto& bus : output_storage) {
                    for (const auto& channel : bus) {
                        for (const float sample : channel) {
                            if (!std::isfinite(sample)) {
                            throw std::runtime_error("processor produced non-finite output");
                            }
                        }
                    }
                }
                processor->setProcessing(false);
                processor_active = false;
                component->setActive(false);
                component_active = false;
                processor->release();
                processor = nullptr;
                MemoryStream state;
                if (component->getState(&state) != kResultOk || state.size() == 0) {
                    throw std::runtime_error("component getState returned no data");
                }
                const auto state_bytes = state.size();
                if (state.seek(0, IBStream::kIBSeekSet, nullptr) != kResultOk ||
                    component->setState(&state) != kResultOk) {
                    throw std::runtime_error("component state round trip failed");
                }
                for (int32 index = 0; index < parameter_count; ++index) {
                    Vst::ParameterInfo parameter{};
                    if (controller->getParameterInfo(index, parameter) != kResultOk) {
                        throw std::runtime_error("getParameterInfo failed");
                    }
                    if (!std::isfinite(parameter.defaultNormalizedValue) ||
                        parameter.defaultNormalizedValue < 0.0 ||
                        parameter.defaultNormalizedValue > 1.0) {
                        throw std::runtime_error("parameter descriptor has invalid default");
                    }
                    std::cout << "parameter[" << index << "] id=" << parameter.id
                              << " title=" << bounded_ascii(parameter.title)
                              << " default=" << parameter.defaultNormalizedValue
                              << " step_count=" << parameter.stepCount
                              << " flags=" << parameter.flags << "\n";
                    const auto original = controller->getParamNormalized(parameter.id);
                    if (!std::isfinite(original) || original < 0.0 || original > 1.0) {
                        throw std::runtime_error("parameter returned invalid normalized value");
                    }
                    require_result("parameter write", controller->setParamNormalized(parameter.id, 0.5));
                    const auto updated = controller->getParamNormalized(parameter.id);
                    if (!std::isfinite(updated) || updated < 0.0 || updated > 1.0) {
                        throw std::runtime_error("parameter automation returned invalid value");
                    }
                    require_result("parameter restore",
                                   controller->setParamNormalized(parameter.id, original));
                }
                controller->terminate();
                controller->release();
                controller = nullptr;
                component->terminate();
                component_initialized = false;
                component->release();
                component = nullptr;
                std::cout << "processed offline block: input_buses=" << inputs
                          << " output_buses=" << outputs
                          << " frames=64 finite=true parameters=" << parameter_count
                          << " parameter_descriptors=" << parameter_count
                          << " automation=verified state_bytes=" << state_bytes
                          << " class_index=" << index << " class_name=" << info.name << "\n";
                processed_audio_effect = true;
                break;
            }
        }
        if (!processed_audio_effect) {
            throw std::runtime_error("factory exposes no compatible audio effect");
        }
        factory->release();
        factory = nullptr;
        FreeLibrary(module);
        return 0;
    } catch (const std::exception& error) {
        if (processor_active) {
            processor->setProcessing(false);
        }
        if (processor) {
            processor->release();
        }
        if (controller) {
            controller->terminate();
            controller->release();
        }
        if (component_active) {
            component->setActive(false);
        }
        if (component) {
            if (component_initialized) {
                component->terminate();
            }
            component->release();
        }
        if (factory) {
            factory->release();
        }
        if (module) {
            FreeLibrary(module);
        }
        std::cerr << "VST3 loader failed: " << error.what() << "\n";
        return 1;
    }
}
