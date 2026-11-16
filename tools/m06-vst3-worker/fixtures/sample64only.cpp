// A minimal VST3 effect that only processes 64-bit samples. It exists to prove
// that AudioRouter's worker refuses a plugin that does not accept the 32-bit
// float format, with a plain reason, instead of converting or loading it.
// It is a test fixture, not a plugin to ship.
#include <windows.h>

#include <atomic>
#include <cstring>

#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"

using namespace Steinberg;

namespace {

// 1 AudioRouter fixture class id, not a registered plugin.
const FUID kClassId(0x41525833, 0x32424954, 0x464F5552, 0x54595F31);

class Component final : public Vst::IComponent, public Vst::IAudioProcessor {
public:
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
    tresult PLUGIN_API setState(IBStream*) override { return kResultOk; }
    tresult PLUGIN_API getState(IBStream*) override { return kResultOk; }
    tresult PLUGIN_API setBusArrangements(Vst::SpeakerArrangement*, int32, Vst::SpeakerArrangement*,
                                          int32) override {
        return kResultOk;
    }
    tresult PLUGIN_API getBusArrangement(Vst::BusDirection, int32, Vst::SpeakerArrangement& arr) override {
        arr = Vst::SpeakerArr::kStereo;
        return kResultOk;
    }
    // The point of the fixture: only 64-bit samples are accepted.
    tresult PLUGIN_API canProcessSampleSize(int32 symbolicSampleSize) override {
        return symbolicSampleSize == Vst::kSample64 ? kResultTrue : kResultFalse;
    }
    uint32 PLUGIN_API getLatencySamples() override { return 0; }
    tresult PLUGIN_API setupProcessing(Vst::ProcessSetup& setup) override {
        return setup.symbolicSampleSize == Vst::kSample64 ? kResultOk : kResultFalse;
    }
    tresult PLUGIN_API setProcessing(TBool) override { return kResultOk; }
    tresult PLUGIN_API process(Vst::ProcessData&) override { return kResultOk; }
    uint32 PLUGIN_API getTailSamples() override { return 0; }

private:
    std::atomic<uint32> references_{1};
};

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
    int32 PLUGIN_API countClasses() override { return 1; }
    tresult PLUGIN_API getClassInfo(int32 index, PClassInfo* info) override {
        if (index != 0) return kInvalidArgument;
        *info = {};
        kClassId.toTUID(info->cid);
        info->cardinality = PClassInfo::kManyInstances;
        std::strcpy(info->category, kVstAudioEffectClass);
        std::strcpy(info->name, "Sample64Only");
        return kResultOk;
    }
    tresult PLUGIN_API createInstance(FIDString cid, FIDString iid, void** object) override {
        if (!FUnknownPrivate::iidEqual(cid, kClassId.toTUID())) return kInvalidArgument;
        auto* component = new Component();
        const auto result = component->queryInterface(reinterpret_cast<const char*>(iid), object);
        component->release();
        return result;
    }
    tresult PLUGIN_API getClassInfo2(int32 index, PClassInfo2* info) override {
        if (index != 0) return kInvalidArgument;
        *info = {};
        kClassId.toTUID(info->cid);
        info->cardinality = PClassInfo::kManyInstances;
        std::strcpy(info->category, kVstAudioEffectClass);
        std::strcpy(info->name, "Sample64Only");
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
