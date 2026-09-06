#define NOMINMAX
#include <windows.h>
#include <audioclient.h>
#include <audioclientactivationparams.h>
#include <mmdeviceapi.h>
#include <functiondiscoverykeys_devpkey.h>
#include <iostream>
#include <iomanip>
#include <condition_variable>
#include <chrono>
#include <mutex>
#include <thread>
#include <atomic>
#include <cstring>
#include <cstdlib>
#include <wrl.h>
#include <wrl/implements.h>

static void print_format(const WAVEFORMATEX* format) {
    if (!format) return;
    std::cout << "rate=" << format->nSamplesPerSec
              << " channels=" << format->nChannels
              << " bits=" << format->wBitsPerSample
              << " tag=0x" << std::hex << format->wFormatTag << std::dec
              << " block=" << format->nBlockAlign
              << " avg=" << format->nAvgBytesPerSec
              << " cbSize=" << format->cbSize;
    if (format->wFormatTag == WAVE_FORMAT_EXTENSIBLE &&
        format->cbSize >= sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX)) {
        const auto* extensible = reinterpret_cast<const WAVEFORMATEXTENSIBLE*>(format);
        std::cout << " mask=0x" << std::hex << extensible->dwChannelMask << std::dec
                  << " validBits=" << extensible->Samples.wValidBitsPerSample
                  << " subformat=" << std::hex << extensible->SubFormat.Data1 << std::dec;
    }
    std::cout << '\n';
}

static void print_hr(const char* label, HRESULT hr) {
    std::cout << label << "=0x" << std::hex << static_cast<unsigned long>(hr)
              << std::dec << '\n';
}

class ProcessLoopbackHandler final
    : public Microsoft::WRL::RuntimeClass<Microsoft::WRL::RuntimeClassFlags<Microsoft::WRL::ClassicCom>,
                                          Microsoft::WRL::FtmBase,
                                          IActivateAudioInterfaceCompletionHandler> {
public:
    explicit ProcessLoopbackHandler(std::mutex& mutex, std::condition_variable& ready,
                                    HRESULT& activation, bool& completed,
                                    Microsoft::WRL::ComPtr<IUnknown>& activated)
        : mutex_(mutex), ready_(ready), activation_(activation), completed_(completed),
          activated_(activated) {}

    HRESULT STDMETHODCALLTYPE ActivateCompleted(
        IActivateAudioInterfaceAsyncOperation* operation) override {
        HRESULT activation = E_FAIL;
        IUnknown* activated = nullptr;
        if (operation) {
            HRESULT activate_result = E_FAIL;
            HRESULT get_result = operation->GetActivateResult(&activate_result, &activated);
            activation = FAILED(get_result) ? get_result : activate_result;
            if (SUCCEEDED(activation) && activated) activated_.Attach(activated);
            else if (activated) activated->Release();
        }
        {
            std::lock_guard<std::mutex> lock(mutex_);
            activation_ = activation;
            completed_ = true;
        }
        ready_.notify_one();
        return S_OK;
    }

private:
    std::mutex& mutex_;
    std::condition_variable& ready_;
    HRESULT& activation_;
    bool& completed_;
    Microsoft::WRL::ComPtr<IUnknown>& activated_;
};

static int process_loopback_probe(DWORD target_process_id, bool read_data, DWORD duration_ms) {
    std::mutex mutex;
    std::condition_variable ready;
    HRESULT activation = E_FAIL;
    bool completed = false;
    Microsoft::WRL::ComPtr<IUnknown> activated;
    auto handler = Microsoft::WRL::Make<ProcessLoopbackHandler>(
        mutex, ready, activation, completed, activated);
    if (!handler) return 1;

    AUDIOCLIENT_ACTIVATION_PARAMS parameters{};
    parameters.ActivationType = AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK;
    parameters.ProcessLoopbackParams.TargetProcessId = target_process_id;
    parameters.ProcessLoopbackParams.ProcessLoopbackMode =
        PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE;

    PROPVARIANT property{};
    PropVariantInit(&property);
    property.vt = VT_BLOB;
    property.blob.cbSize = sizeof(parameters);
    property.blob.pBlobData = static_cast<BYTE*>(CoTaskMemAlloc(sizeof(parameters)));
    if (!property.blob.pBlobData) {
        return 1;
    }
    std::memcpy(property.blob.pBlobData, &parameters, sizeof(parameters));

    IActivateAudioInterfaceAsyncOperation* operation = nullptr;
    HRESULT hr = ActivateAudioInterfaceAsync(
        VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK, __uuidof(IAudioClient), &property,
        handler.Get(), &operation);
    print_hr("process_activate_async", hr);
    bool data_ok = !read_data;
    if (SUCCEEDED(hr)) {
        std::unique_lock<std::mutex> lock(mutex);
        if (!ready.wait_for(lock, std::chrono::seconds(5), [&] { return completed; })) {
            std::cout << "process_callback=timeout\n";
        } else {
            print_hr("process_activate_result", activation);
            if (SUCCEEDED(activation)) {
                Microsoft::WRL::ComPtr<IAudioClient> client;
                HRESULT query = activated.As(&client);
                print_hr("process_query_audio_client", query);
                if (SUCCEEDED(query)) {
                    WAVEFORMATEX format{};
                    format.wFormatTag = WAVE_FORMAT_PCM;
                    format.nChannels = 2;
                    format.nSamplesPerSec = 44100;
                    format.wBitsPerSample = 16;
                    format.nBlockAlign = format.nChannels * format.wBitsPerSample / 8;
                    format.nAvgBytesPerSec = format.nSamplesPerSec * format.nBlockAlign;
                    HANDLE ready_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
                    HRESULT initialize = client->Initialize(
                        AUDCLNT_SHAREMODE_SHARED,
                        AUDCLNT_STREAMFLAGS_LOOPBACK |
                            AUDCLNT_STREAMFLAGS_EVENTCALLBACK |
                            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
                        0, 0, &format, nullptr);
                    print_hr("process_initialize_44100_pcm", initialize);
                    if (SUCCEEDED(initialize) && ready_event) {
                        print_hr("process_set_event_handle", client->SetEventHandle(ready_event));
                        if (read_data) {
                            Microsoft::WRL::ComPtr<IAudioCaptureClient> capture;
                            HRESULT service = client.As(&capture);
                            print_hr("process_get_capture_service", service);
                            if (SUCCEEDED(service)) {
                                HRESULT start = client->Start();
                                print_hr("process_capture_start", start);
                                UINT32 packet_count = 0;
                                UINT32 frame_count = 0;
                                UINT32 silent_packet_count = 0;
                                UINT64 nonzero_bytes = 0;
                                HRESULT read = start;
                                if (SUCCEEDED(start)) {
                                    const auto deadline = std::chrono::steady_clock::now() +
                                        std::chrono::milliseconds(duration_ms);
                                    while (std::chrono::steady_clock::now() < deadline) {
                                        WaitForSingleObject(ready_event, 50);
                                        while (true) {
                                            UINT32 frames = 0;
                                            read = capture->GetNextPacketSize(&frames);
                                            if (FAILED(read) || frames == 0) break;
                                            BYTE* data = nullptr;
                                            DWORD flags = 0;
                                            UINT64 position = 0;
                                            UINT64 timestamp = 0;
                                            read = capture->GetBuffer(&data, &frames, &flags,
                                                                      &position, &timestamp);
                                            if (FAILED(read)) break;
                                            ++packet_count;
                                            frame_count += frames;
                                            const UINT64 packet_bytes = static_cast<UINT64>(frames) * format.nBlockAlign;
                                            if ((flags & AUDCLNT_BUFFERFLAGS_SILENT) != 0) {
                                                ++silent_packet_count;
                                            } else if (data) {
                                                for (UINT64 index = 0; index < packet_bytes; ++index) {
                                                    if (data[index] != 0) ++nonzero_bytes;
                                                }
                                            }
                                            read = capture->ReleaseBuffer(frames);
                                            if (FAILED(read)) break;
                                        }
                                        if (FAILED(read)) break;
                                    }
                                }
                                print_hr("process_capture_packet_read", read);
                                std::cout << "process_capture_packets=" << packet_count
                                          << " process_capture_frames=" << frame_count
                                          << " process_capture_silent_packets=" << silent_packet_count
                                          << " process_capture_nonzero_bytes=" << nonzero_bytes << '\n';
                                print_hr("process_capture_stop", client->Stop());
                                print_hr("process_capture_reset", client->Reset());
                                data_ok = SUCCEEDED(read) && packet_count > 0;
                            }
                        } else {
                            print_hr("process_reset", client->Reset());
                        }
                    }
                    if (ready_event) CloseHandle(ready_event);
                }
            }
        }
        if (operation) operation->Release();
    }
    PropVariantClear(&property);
    return SUCCEEDED(hr) && completed && SUCCEEDED(activation) && data_ok ? 0 : 1;
}

static int capture_data_probe(UINT target_index, DWORD duration_ms) {
    IMMDeviceEnumerator* enumerator = nullptr;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
    if (FAILED(hr)) { print_hr("capture_enumerator", hr); return 1; }
    IMMDeviceCollection* devices = nullptr;
    hr = enumerator->EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE, &devices);
    if (FAILED(hr)) { print_hr("capture_enum", hr); enumerator->Release(); return 1; }
    UINT count = 0;
    devices->GetCount(&count);
    if (target_index >= count) {
        std::cout << "capture_index_out_of_range=" << target_index << " count=" << count << '\n';
        devices->Release(); enumerator->Release(); return 1;
    }
    IMMDevice* device = nullptr;
    hr = devices->Item(target_index, &device);
    print_hr("capture_item", hr);
    if (FAILED(hr)) { devices->Release(); enumerator->Release(); return 1; }
    IAudioClient* client = nullptr;
    hr = device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr,
                          reinterpret_cast<void**>(&client));
    print_hr("capture_activate", hr);
    WAVEFORMATEX* format = nullptr;
    if (SUCCEEDED(hr)) hr = client->GetMixFormat(&format);
    print_hr("capture_get_mix_format", hr);
    if (SUCCEEDED(hr)) {
        print_format(format);
        hr = client->Initialize(AUDCLNT_SHAREMODE_SHARED,
                                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST,
                                1000000, 0, format, nullptr);
        print_hr("capture_initialize", hr);
    }
    IAudioCaptureClient* capture = nullptr;
    if (SUCCEEDED(hr)) {
        hr = client->GetService(__uuidof(IAudioCaptureClient), reinterpret_cast<void**>(&capture));
        print_hr("capture_get_service", hr);
    }
    UINT32 packet_count = 0;
    UINT32 frame_count = 0;
    if (SUCCEEDED(hr)) {
        hr = client->Start();
        print_hr("capture_start", hr);
        if (SUCCEEDED(hr)) {
            std::this_thread::sleep_for(std::chrono::milliseconds(duration_ms));
            while (true) {
                UINT32 frames = 0;
                hr = capture->GetNextPacketSize(&frames);
                if (FAILED(hr) || frames == 0) break;
                BYTE* data = nullptr;
                DWORD flags = 0;
                UINT64 position = 0;
                UINT64 timestamp = 0;
                hr = capture->GetBuffer(&data, &frames, &flags, &position, &timestamp);
                if (FAILED(hr)) break;
                ++packet_count;
                frame_count += frames;
                hr = capture->ReleaseBuffer(frames);
                if (FAILED(hr)) break;
            }
            print_hr("capture_packet_read", hr);
            std::cout << "capture_packets=" << packet_count << " capture_frames=" << frame_count << '\n';
            print_hr("capture_stop", client->Stop());
            print_hr("capture_reset", client->Reset());
        }
    }
    if (capture) capture->Release();
    if (format) CoTaskMemFree(format);
    if (client) client->Release();
    device->Release(); devices->Release(); enumerator->Release();
    return SUCCEEDED(hr) && packet_count > 0 ? 0 : 1;
}

static int render_data_probe(UINT target_index, DWORD duration_ms) {
    IMMDeviceEnumerator* enumerator = nullptr;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
    if (FAILED(hr)) { print_hr("render_enumerator", hr); return 1; }
    IMMDeviceCollection* devices = nullptr;
    hr = enumerator->EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, &devices);
    if (FAILED(hr)) { print_hr("render_enum", hr); enumerator->Release(); return 1; }
    UINT count = 0;
    devices->GetCount(&count);
    if (target_index >= count) {
        std::cout << "render_index_out_of_range=" << target_index << " count=" << count << '\n';
        devices->Release(); enumerator->Release(); return 1;
    }
    IMMDevice* device = nullptr;
    hr = devices->Item(target_index, &device);
    print_hr("render_item", hr);
    if (FAILED(hr)) { devices->Release(); enumerator->Release(); return 1; }
    IAudioClient* client = nullptr;
    hr = device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr,
                          reinterpret_cast<void**>(&client));
    print_hr("render_activate", hr);
    WAVEFORMATEX* format = nullptr;
    if (SUCCEEDED(hr)) hr = client->GetMixFormat(&format);
    print_hr("render_get_mix_format", hr);
    if (SUCCEEDED(hr)) {
        print_format(format);
        hr = client->Initialize(AUDCLNT_SHAREMODE_SHARED,
                                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST,
                                1000000, 0, format, nullptr);
        print_hr("render_initialize", hr);
    }
    IAudioRenderClient* render = nullptr;
    UINT32 buffer_size = 0;
    if (SUCCEEDED(hr)) {
        hr = client->GetBufferSize(&buffer_size);
        print_hr("render_get_buffer_size", hr);
        hr = client->GetService(__uuidof(IAudioRenderClient), reinterpret_cast<void**>(&render));
        print_hr("render_get_service", hr);
    }
    UINT32 submitted_frames = 0;
    if (SUCCEEDED(hr)) {
        hr = client->Start();
        print_hr("render_start", hr);
        if (SUCCEEDED(hr)) {
            const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(duration_ms);
            while (std::chrono::steady_clock::now() < deadline) {
                UINT32 padding = 0;
                hr = client->GetCurrentPadding(&padding);
                if (FAILED(hr)) break;
                const UINT32 available = buffer_size - padding;
                if (available != 0) {
                    BYTE* data = nullptr;
                    hr = render->GetBuffer(available, &data);
                    if (FAILED(hr)) break;
                    hr = render->ReleaseBuffer(available, AUDCLNT_BUFFERFLAGS_SILENT);
                    if (FAILED(hr)) break;
                    submitted_frames += available;
                }
                std::this_thread::sleep_for(std::chrono::milliseconds(10));
            }
            print_hr("render_silent_submit", hr);
            std::cout << "render_buffer_size=" << buffer_size
                      << " render_submitted_frames=" << submitted_frames << '\n';
            print_hr("render_stop", client->Stop());
            print_hr("render_reset", client->Reset());
        }
    }
    if (render) render->Release();
    if (format) CoTaskMemFree(format);
    if (client) client->Release();
    device->Release(); devices->Release(); enumerator->Release();
    return SUCCEEDED(hr) && submitted_frames > 0 ? 0 : 1;
}

int main(int argc, char** argv) {
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr)) { print_hr("CoInitializeEx", hr); return 1; }

    if (argc > 1 && std::strcmp(argv[1], "capture") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        DWORD duration_ms = argc > 3 ? static_cast<DWORD>(std::strtoul(argv[3], nullptr, 10)) : 200;
        int result = capture_data_probe(target_index, duration_ms);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "render") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        DWORD duration_ms = argc > 3 ? static_cast<DWORD>(std::strtoul(argv[3], nullptr, 10)) : 200;
        int result = render_data_probe(target_index, duration_ms);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && (std::strcmp(argv[1], "process") == 0 ||
                     std::strcmp(argv[1], "process-capture") == 0)) {
        DWORD target_process_id = GetCurrentProcessId();
        if (argc > 2) target_process_id = static_cast<DWORD>(std::strtoul(argv[2], nullptr, 10));
        bool read_data = std::strcmp(argv[1], "process-capture") == 0;
        DWORD duration_ms = argc > 3 ? static_cast<DWORD>(std::strtoul(argv[3], nullptr, 10)) : 500;
        int result = process_loopback_probe(target_process_id, read_data, duration_ms);
        CoUninitialize();
        return result;
    }

    IMMDeviceEnumerator* enumerator = nullptr;
    hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                          __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
    if (FAILED(hr)) { print_hr("CoCreateInstance", hr); CoUninitialize(); return 1; }

    IMMDeviceCollection* devices = nullptr;
    hr = enumerator->EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE, &devices);
    if (FAILED(hr)) { print_hr("EnumAudioEndpoints", hr); enumerator->Release(); CoUninitialize(); return 1; }

    UINT count = 0;
    devices->GetCount(&count);
    std::cout << "capture_endpoint_count=" << count << '\n';
    for (UINT index = 0; index < count; ++index) {
        IMMDevice* device = nullptr;
        if (FAILED(devices->Item(index, &device))) continue;
        LPWSTR id = nullptr;
        device->GetId(&id);
        std::wcout << L"endpoint[" << index << L"]=" << (id ? id : L"<unknown>") << L'\n';
        CoTaskMemFree(id);

        IAudioClient* client = nullptr;
        hr = device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr,
                              reinterpret_cast<void**>(&client));
        print_hr("activate", hr);
        if (SUCCEEDED(hr)) {
            WAVEFORMATEX* format = nullptr;
            hr = client->GetMixFormat(&format);
            print_hr("get_mix_format", hr);
            if (SUCCEEDED(hr)) {
                std::cout << "mix_"; print_format(format);
                WAVEFORMATEX* closest = nullptr;
                HRESULT support = client->IsFormatSupported(AUDCLNT_SHAREMODE_SHARED, format, &closest);
                print_hr("is_format_supported", support);
                if (closest) { std::cout << "closest_"; print_format(closest); CoTaskMemFree(closest); }
                // This is deliberately the only stream operation: Initialize allocates no
                // running stream and does not start or read audio.
                HRESULT initialized = client->Initialize(
                    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_NOPERSIST,
                    1000000, 0, format, nullptr);
                print_hr("initialize_100ms_nopersist", initialized);
                if (SUCCEEDED(initialized)) client->Reset();
                CoTaskMemFree(format);
            }
            client->Release();
        }
        device->Release();
    }
    devices->Release();
    enumerator->Release();
    CoUninitialize();
    return 0;
}
