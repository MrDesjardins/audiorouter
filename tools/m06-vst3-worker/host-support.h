#pragma once
#include <map>
#include <variant>
#include <avrt.h>
#include "pluginterfaces/vst/ivstattributes.h"
#include "pluginterfaces/vst/ivstmessage.h"

class AudioThreadScheduling final {
    HANDLE handle_ = nullptr;
public:
    AudioThreadScheduling() {
        DWORD index = 0;
        handle_ = AvSetMmThreadCharacteristicsW(L"Pro Audio", &index);
        if (handle_) AvSetMmThreadPriority(handle_, AVRT_PRIORITY_HIGH);
    }
    ~AudioThreadScheduling() { if (handle_) AvRevertMmThreadCharacteristics(handle_); }
    AudioThreadScheduling(const AudioThreadScheduling&) = delete;
    AudioThreadScheduling& operator=(const AudioThreadScheduling&) = delete;
};

// Stack-owned host interfaces live until every plugin interface is released.
// Heap messages/attribute lists implement COM ownership independently.
template<class Interface>
static Steinberg::tresult host_query(Interface* self, const Steinberg::TUID iid,
                                     void** object, const Steinberg::TUID supported) {
    if (!object) return Steinberg::kInvalidArgument;
    *object = nullptr;
    if (!Steinberg::FUnknownPrivate::iidEqual(iid, supported) &&
        !Steinberg::FUnknownPrivate::iidEqual(iid, Steinberg::FUnknown_iid)) return Steinberg::kNoInterface;
    *object = self;
    self->addRef();
    return Steinberg::kResultOk;
}

class HostAttributes final : public Steinberg::Vst::IAttributeList {
    using Value = std::variant<Steinberg::int64, double, std::u16string, std::vector<uint8_t>>;
    std::map<std::string, Value> values_;
    std::atomic<Steinberg::uint32> references_{1};
    bool valid(AttrID id) const { return id && std::strlen(id) <= 256 && (values_.count(id) || values_.size() < 256); }
    template<class T> Steinberg::tresult get(AttrID id, T& value) {
        if (!id) return Steinberg::kInvalidArgument;
        const auto found = values_.find(id);
        if (found == values_.end()) return Steinberg::kResultFalse;
        const auto* stored = std::get_if<T>(&found->second);
        if (!stored) return Steinberg::kResultFalse;
        value = *stored;
        return Steinberg::kResultOk;
    }
public:
    Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID iid, void** object) override {
        return host_query<Steinberg::Vst::IAttributeList>(this, iid, object, Steinberg::Vst::IAttributeList_iid);
    }
    Steinberg::uint32 PLUGIN_API addRef() override { return ++references_; }
    Steinberg::uint32 PLUGIN_API release() override { const auto remaining = --references_; if (!remaining) delete this; return remaining; }
    Steinberg::tresult PLUGIN_API setInt(AttrID id, Steinberg::int64 value) override {
        if (!valid(id)) return Steinberg::kInvalidArgument; values_[id] = value; return Steinberg::kResultOk;
    }
    Steinberg::tresult PLUGIN_API getInt(AttrID id, Steinberg::int64& value) override { return get(id, value); }
    Steinberg::tresult PLUGIN_API setFloat(AttrID id, double value) override {
        if (!valid(id)) return Steinberg::kInvalidArgument; values_[id] = value; return Steinberg::kResultOk;
    }
    Steinberg::tresult PLUGIN_API getFloat(AttrID id, double& value) override { return get(id, value); }
    Steinberg::tresult PLUGIN_API setString(AttrID id, const Steinberg::Vst::TChar* value) override {
        if (!valid(id) || !value) return Steinberg::kInvalidArgument;
        std::size_t length = 0;
        while (length < 65536 && value[length]) ++length;
        if (length == 65536) return Steinberg::kInvalidArgument;
        values_[id] = std::u16string(reinterpret_cast<const char16_t*>(value), length);
        return Steinberg::kResultOk;
    }
    Steinberg::tresult PLUGIN_API getString(AttrID id, Steinberg::Vst::TChar* value, Steinberg::uint32 bytes) override {
        if (!value || bytes < sizeof(*value)) return Steinberg::kInvalidArgument;
        std::u16string stored;
        const auto result = get(id, stored);
        if (result != Steinberg::kResultOk) return result;
        const auto count = std::min(stored.size(), bytes / sizeof(*value) - 1);
        std::copy_n(stored.data(), count, value); value[count] = 0;
        return Steinberg::kResultOk;
    }
    Steinberg::tresult PLUGIN_API setBinary(AttrID id, const void* data, Steinberg::uint32 bytes) override {
        if (!valid(id) || bytes > 1024 * 1024 || (!data && bytes)) return Steinberg::kInvalidArgument;
        std::vector<uint8_t> value(bytes);
        if (bytes) std::memcpy(value.data(), data, bytes);
        values_[id] = std::move(value); return Steinberg::kResultOk;
    }
    Steinberg::tresult PLUGIN_API getBinary(AttrID id, const void*& data, Steinberg::uint32& bytes) override {
        data = nullptr; bytes = 0;
        if (!id) return Steinberg::kInvalidArgument;
        const auto found = values_.find(id);
        if (found == values_.end()) return Steinberg::kResultFalse;
        const auto* value = std::get_if<std::vector<uint8_t>>(&found->second);
        if (!value) return Steinberg::kResultFalse;
        data = value->data(); bytes = static_cast<Steinberg::uint32>(value->size()); return Steinberg::kResultOk;
    }
};

class HostMessage final : public Steinberg::Vst::IMessage {
    std::string id_;
    HostAttributes* attributes_ = new HostAttributes;
    std::atomic<Steinberg::uint32> references_{1};
public:
    ~HostMessage() { attributes_->release(); }
    Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID iid, void** object) override {
        return host_query<Steinberg::Vst::IMessage>(this, iid, object, Steinberg::Vst::IMessage_iid);
    }
    Steinberg::uint32 PLUGIN_API addRef() override { return ++references_; }
    Steinberg::uint32 PLUGIN_API release() override { const auto remaining = --references_; if (!remaining) delete this; return remaining; }
    Steinberg::FIDString PLUGIN_API getMessageID() override { return id_.c_str(); }
    void PLUGIN_API setMessageID(Steinberg::FIDString id) override { id_ = id ? std::string(id).substr(0, 256) : ""; }
    Steinberg::Vst::IAttributeList* PLUGIN_API getAttributes() override { return attributes_; }
};
