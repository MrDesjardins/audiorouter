// A minimal VST3 module with two effect classes: "Quiet" multiplies audio by
// 0.5 and "Loud" by 2.0. It proves that AudioRouter loads exactly the class it
// was asked for and never substitutes the other one. It also keeps a one-byte
// state per class, so a restore of one class's state cannot be mistaken for the
// other's. A test fixture, not a plugin to ship.
#include <windows.h>

#include <atomic>
#include <cstring>

#include "pluginterfaces/base/ibstream.h"
#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"

using namespace Steinberg;

namespace {

const FUID kQuietId(0x41525433, 0x51554945, 0x54000000, 0x00000001);
const FUID kLoudId(0x41525433, 0x4C4F5544, 0x00000000, 0x00000002);

class Component final : public Vst::IComponent, public Vst::IAudioProcessor {
public:
    Component(float gain, char marker) : gain_(gain), marker_(marker) {}
    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        if (FUnknownPrivate::iidEqual(iid, FUnknown_iid) ||
            FUnknownPrivate::iidEqual(iid, IPluginBase_iid) ||
            FUnknownPrivate::iidEqual(iid, Vst::IComponent_iid)) {
            *object = static_cast<Vst::IComponent*>(this);
        } else if (FUnknownPrivate::iidEqual(iid, Vst::IAudioProcessor_iid)) {
            *object = static_cast<Vst::IAudioProcessor*>(this);
        } else {
            *object = nullptr;
            return kNoInterface;
        }
        addRef();
        return kResultOk;
    }
    uint32 PLUGIN_API addRef() override { return ++references_; }
    uint32 PLUGIN_API release() override {
        const auto left = --references_;
        if (left == 0) delete this;
        return left;
    }
    tresult PLUGIN_API initialize(FUnknown*) override { return kResultOk; }
    tresult PLUGIN_API terminate() override { return kResultOk; }
    tresult PLUGIN_API getControllerClassId(TUID) override { return kNotImplemented; }
    tresult PLUGIN_API setIoMode(Vst::IoMode) override { return kResultOk; }
    int32 PLUGIN_API getBusCount(Vst::MediaType type, Vst::BusDirection) override {
        return type == Vst::kAudio ? 1 : 0;
    }
    tresult PLUGIN_API getBusInfo(Vst::MediaType type, Vst::BusDirection direction, int32 index,
                                  Vst::BusInfo& info) override {
        if (type != Vst::kAudio || index != 0) return kInvalidArgument;
        info = {};
        info.mediaType = Vst::kAudio;
        info.direction = direction;
        info.channelCount = 2;
        info.busType = Vst::kMain;
        info.flags = Vst::BusInfo::kDefaultActive;
        return kResultOk;
    }
    tresult PLUGIN_API getRoutingInfo(Vst::RoutingInfo&, Vst::RoutingInfo&) override {
        return kNotImplemented;
    }
    tresult PLUGIN_API activateBus(Vst::MediaType, Vst::BusDirection, int32, TBool) override {
        return kResultOk;
    }
    tresult PLUGIN_API setActive(TBool) override { return kResultOk; }
    // One byte: which class wrote it. A class refuses another class's state.
    tresult PLUGIN_API setState(IBStream* stream) override {
        char value = 0;
        int32 read = 0;
        if (!stream || stream->read(&value, 1, &read) != kResultOk || read != 1) return kResultFalse;
        return value == marker_ ? kResultOk : kResultFalse;
    }
    tresult PLUGIN_API getState(IBStream* stream) override {
        int32 written = 0;
        return stream ? stream->write(&marker_, 1, &written) : kResultFalse;
    }
    tresult PLUGIN_API setBusArrangements(Vst::SpeakerArrangement*, int32, Vst::SpeakerArrangement*,
                                          int32) override {
        return kResultOk;
    }
    tresult PLUGIN_API getBusArrangement(Vst::BusDirection, int32, Vst::SpeakerArrangement& arr) override {
        arr = Vst::SpeakerArr::kStereo;
        return kResultOk;
    }
    tresult PLUGIN_API canProcessSampleSize(int32 symbolicSampleSize) override {
        return symbolicSampleSize == Vst::kSample32 ? kResultTrue : kResultFalse;
    }
    uint32 PLUGIN_API getLatencySamples() override { return 0; }
    tresult PLUGIN_API setupProcessing(Vst::ProcessSetup&) override { return kResultOk; }
    tresult PLUGIN_API setProcessing(TBool) override { return kResultOk; }
    tresult PLUGIN_API process(Vst::ProcessData& data) override {
        if (data.numInputs < 1 || data.numOutputs < 1) return kResultOk;
        auto& input = data.inputs[0];
        auto& output = data.outputs[0];
        for (int32 channel = 0; channel < output.numChannels && channel < input.numChannels; ++channel) {
            for (int32 frame = 0; frame < data.numSamples; ++frame) {
                output.channelBuffers32[channel][frame] = input.channelBuffers32[channel][frame] * gain_;
            }
        }
        return kResultOk;
    }
    uint32 PLUGIN_API getTailSamples() override { return 0; }

private:
    std::atomic<uint32> references_{1};
    float gain_;
    char marker_;
};

struct ClassSpec {
    const FUID* id;
    const char* name;
    float gain;
    char marker;
};
const ClassSpec kClasses[] = {{&kQuietId, "Quiet", 0.5f, 'Q'}, {&kLoudId, "Loud", 2.0f, 'L'}};

class Factory final : public IPluginFactory2 {
public:
    tresult PLUGIN_API queryInterface(const TUID iid, void** object) override {
        if (FUnknownPrivate::iidEqual(iid, FUnknown_iid) ||
            FUnknownPrivate::iidEqual(iid, IPluginFactory_iid) ||
            FUnknownPrivate::iidEqual(iid, IPluginFactory2_iid)) {
            *object = static_cast<IPluginFactory2*>(this);
            return kResultOk;
        }
        *object = nullptr;
        return kNoInterface;
    }
    uint32 PLUGIN_API addRef() override { return 2; }
    uint32 PLUGIN_API release() override { return 1; }
    tresult PLUGIN_API getFactoryInfo(PFactoryInfo* info) override {
        *info = {};
        std::strcpy(info->vendor, "AudioRouter tests");
        return kResultOk;
    }
    int32 PLUGIN_API countClasses() override { return 2; }
    tresult PLUGIN_API getClassInfo(int32 index, PClassInfo* info) override {
        if (index < 0 || index > 1) return kInvalidArgument;
        *info = {};
        kClasses[index].id->toTUID(info->cid);
        info->cardinality = PClassInfo::kManyInstances;
        std::strcpy(info->category, kVstAudioEffectClass);
        std::strcpy(info->name, kClasses[index].name);
        return kResultOk;
    }
    tresult PLUGIN_API createInstance(FIDString cid, FIDString iid, void** object) override {
        for (const auto& spec : kClasses) {
            if (!FUnknownPrivate::iidEqual(cid, spec.id->toTUID())) continue;
            auto* component = new Component(spec.gain, spec.marker);
            const auto result = component->queryInterface(reinterpret_cast<const char*>(iid), object);
            component->release();
            return result;
        }
        return kInvalidArgument;
    }
    tresult PLUGIN_API getClassInfo2(int32 index, PClassInfo2* info) override {
        if (index < 0 || index > 1) return kInvalidArgument;
        *info = {};
        kClasses[index].id->toTUID(info->cid);
        info->cardinality = PClassInfo::kManyInstances;
        std::strcpy(info->category, kVstAudioEffectClass);
        std::strcpy(info->name, kClasses[index].name);
        std::strcpy(info->subCategories, "Fx");
        std::strcpy(info->vendor, "AudioRouter tests");
        std::strcpy(info->version, "1.0");
        std::strcpy(info->sdkVersion, "VST 3.8.1");
        return kResultOk;
    }
};

Factory g_factory;

}  // namespace

extern "C" {
__declspec(dllexport) bool InitDll() { return true; }
__declspec(dllexport) bool ExitDll() { return true; }
__declspec(dllexport) IPluginFactory* PLUGIN_API GetPluginFactory() { return &g_factory; }
}
