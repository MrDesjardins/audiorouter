#define NOMINMAX

#include <windows.h>

#include <algorithm>
#include <cstdint>
#include <filesystem>
#include <cmath>
#include <cstring>
#include <iostream>
#include <sstream>
#include <string>
#include <stdexcept>
#include <vector>

#include "pluginterfaces/base/ibstream.h"
#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"

namespace fs = std::filesystem;
using namespace Steinberg;
using GetPluginFactoryProc = IPluginFactory* (PLUGIN_API*)();

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

int wmain(int argc, wchar_t** argv) {
    if (argc != 2 && argc != 4) {
        std::wcerr << L"usage: m06-vst3-loader <plugin.vst3|binary> [--class-index <n>]\n";
        return 2;
    }

    int32 selected_class_index = -1;
    if (argc == 4) {
        if (std::wstring(argv[2]) != L"--class-index") {
            std::wcerr << L"unknown option\n";
            return 2;
        }
        try {
            const auto parsed = std::stol(argv[3]);
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
                if (inputs != 1 || outputs != 1) {
                    throw std::runtime_error("probe requires one input and output bus");
                }
                if (component->queryInterface(
                        Vst::IAudioProcessor_iid, reinterpret_cast<void**>(&processor)) !=
                    kResultOk) {
                    throw std::runtime_error("audio processor interface is missing");
                }
                Vst::BusInfo input_info{};
                Vst::BusInfo output_info{};
                if (component->getBusInfo(Vst::kAudio, Vst::kInput, 0, input_info) != kResultOk ||
                    component->getBusInfo(Vst::kAudio, Vst::kOutput, 0, output_info) !=
                        kResultOk ||
                    input_info.channelCount != output_info.channelCount ||
                    input_info.channelCount < 1 || input_info.channelCount > 2) {
                    throw std::runtime_error("unsupported audio bus layout");
                }
                const auto channels = input_info.channelCount;
                require_result("input audio bus activation",
                               component->activateBus(Vst::kAudio, Vst::kInput, 0, true));
                require_result("output audio bus activation",
                               component->activateBus(Vst::kAudio, Vst::kOutput, 0, true));
                Vst::ProcessSetup setup{};
                setup.processMode = Vst::kOffline;
                setup.symbolicSampleSize = Vst::kSample32;
                setup.maxSamplesPerBlock = 64;
                setup.sampleRate = 48000.0;
                require_result("setupProcessing", processor->setupProcessing(setup));
                require_result("component activation", component->setActive(true));
                component_active = true;
                require_result("processor activation", processor->setProcessing(true));
                processor_active = true;
                float input[2][64]{};
                float output[2][64]{};
                for (int channel = 0; channel < channels; ++channel) {
                    for (int sample = 0; sample < 64; ++sample) {
                        input[channel][sample] = 0.25f;
                    }
                }
                Vst::Sample32* input_channels[2] = {input[0], input[1]};
                Vst::Sample32* output_channels[2] = {output[0], output[1]};
                Vst::AudioBusBuffers input_bus{};
                input_bus.numChannels = channels;
                input_bus.channelBuffers32 = input_channels;
                Vst::AudioBusBuffers output_bus{};
                output_bus.numChannels = channels;
                output_bus.channelBuffers32 = output_channels;
                Vst::ProcessData data{};
                data.processMode = Vst::kOffline;
                data.symbolicSampleSize = Vst::kSample32;
                data.numSamples = 64;
                data.numInputs = 1;
                data.numOutputs = 1;
                data.inputs = &input_bus;
                data.outputs = &output_bus;
                require_result("processor process", processor->process(data));
                for (int channel = 0; channel < channels; ++channel) {
                    for (float sample : output[channel]) {
                        if (!std::isfinite(sample)) {
                            throw std::runtime_error("processor produced non-finite output");
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
                if (parameter_count <= 0) {
                    throw std::runtime_error("controller exposes no parameters");
                }
                for (int32 index = 0; index < parameter_count; ++index) {
                    Vst::ParameterInfo parameter{};
                    if (controller->getParameterInfo(index, parameter) != kResultOk) {
                        throw std::runtime_error("getParameterInfo failed");
                    }
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
                std::cout << "processed offline block: channels=" << channels
                          << " frames=64 finite=true parameters=" << parameter_count
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
