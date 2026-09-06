#define NOMINMAX

#include <windows.h>

#include <filesystem>
#include <cmath>
#include <cstring>
#include <iostream>
#include <stdexcept>

#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"

namespace fs = std::filesystem;
using namespace Steinberg;
using GetPluginFactoryProc = IPluginFactory* (PLUGIN_API*)();

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

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::wcerr << L"usage: m06-vst3-loader <plugin.vst3|binary>\n";
        return 2;
    }

    HMODULE module = nullptr;
    IPluginFactory* factory = nullptr;
    Vst::IComponent* component = nullptr;
    Vst::IAudioProcessor* processor = nullptr;
    bool component_initialized = false;
    bool component_active = false;
    bool processor_active = false;
    try {
        const auto binary = fs::absolute(resolve_binary(argv[1]));
        module = LoadLibraryW(binary.c_str());
        if (!module) {
            throw std::runtime_error("LoadLibraryW failed");
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
                component->activateBus(Vst::kAudio, Vst::kInput, 0, true);
                component->activateBus(Vst::kAudio, Vst::kOutput, 0, true);
                Vst::ProcessSetup setup{};
                setup.processMode = Vst::kOffline;
                setup.symbolicSampleSize = Vst::kSample32;
                setup.maxSamplesPerBlock = 64;
                setup.sampleRate = 48000.0;
                if (processor->setupProcessing(setup) != kResultOk) {
                    throw std::runtime_error("setupProcessing failed");
                }
                if (component->setActive(true) != kResultOk) {
                    throw std::runtime_error("component activation failed");
                }
                component_active = true;
                if (processor->setProcessing(true) != kResultOk) {
                    throw std::runtime_error("processor activation failed");
                }
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
                if (processor->process(data) != kResultOk) {
                    throw std::runtime_error("processor process failed");
                }
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
                component->terminate();
                component_initialized = false;
                component->release();
                component = nullptr;
                std::cout << "processed offline block: channels=" << channels
                          << " frames=64 finite=true\n";
                break;
            }
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
