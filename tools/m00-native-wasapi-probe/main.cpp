#define NOMINMAX
#include <windows.h>
#include <audioclient.h>
#include <audioclientactivationparams.h>
#include <audiopolicy.h>
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
#include <cmath>
#include <cstdio>
#include <cstdint>
#include <fstream>
#include <string>
#include <vector>
#include <algorithm>
#include <numeric>
#include <ksmedia.h>
#include <wrl.h>
#include <wrl/implements.h>

static std::wstring endpoint_name(IMMDevice* device) {
    if (!device) return L"<unknown>";
    Microsoft::WRL::ComPtr<IPropertyStore> properties;
    if (FAILED(device->OpenPropertyStore(STGM_READ, &properties))) return L"<unknown>";
    PROPVARIANT value;
    PropVariantInit(&value);
    std::wstring name = L"<unknown>";
    if (SUCCEEDED(properties->GetValue(PKEY_Device_FriendlyName, &value)) &&
        value.vt == VT_LPWSTR && value.pwszVal) {
        name = value.pwszVal;
    }
    PropVariantClear(&value);
    return name;
}

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
                  << " subformat=" << std::hex << std::setfill('0')
                  << std::setw(8) << extensible->SubFormat.Data1 << '-'
                  << std::setw(4) << extensible->SubFormat.Data2 << '-'
                  << std::setw(4) << extensible->SubFormat.Data3 << '-'
                  << std::setw(2) << static_cast<unsigned>(extensible->SubFormat.Data4[0])
                  << std::setw(2) << static_cast<unsigned>(extensible->SubFormat.Data4[1]) << '-'
                  << std::setw(2) << static_cast<unsigned>(extensible->SubFormat.Data4[2])
                  << std::setw(2) << static_cast<unsigned>(extensible->SubFormat.Data4[3])
                  << std::setw(2) << static_cast<unsigned>(extensible->SubFormat.Data4[4])
                  << std::setw(2) << static_cast<unsigned>(extensible->SubFormat.Data4[5])
                  << std::setw(2) << static_cast<unsigned>(extensible->SubFormat.Data4[6])
                  << std::setw(2) << static_cast<unsigned>(extensible->SubFormat.Data4[7])
                  << std::setfill(' ') << std::dec;
    }
    std::cout << '\n';
}

static void print_hr(const char* label, HRESULT hr) {
    std::cout << label << "=0x" << std::hex << static_cast<unsigned long>(hr)
              << std::dec << '\n';
}

static std::wstring process_image_path(DWORD process_id) {
    if (process_id == 0) return L"<system-session>";
    HANDLE process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process_id);
    if (!process) return L"<unavailable>";
    std::vector<wchar_t> path(32768);
    DWORD length = static_cast<DWORD>(path.size());
    const BOOL queried = QueryFullProcessImageNameW(process, 0, path.data(), &length);
    CloseHandle(process);
    if (!queried) return L"<unavailable>";
    return std::wstring(path.data(), length);
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

static int process_loopback_probe(DWORD target_process_id, bool read_data, bool include_target_tree,
                                  DWORD duration_ms, bool require_signal) {
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
    parameters.ProcessLoopbackParams.ProcessLoopbackMode = include_target_tree
        ? PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE
        : PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE;

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
                                long double sample_energy = 0.0;
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
                                                if (format.wBitsPerSample == 16) {
                                                    const auto* samples = reinterpret_cast<const int16_t*>(data);
                                                    const UINT64 sample_count = static_cast<UINT64>(frames) * format.nChannels;
                                                    for (UINT64 index = 0; index < sample_count; ++index) {
                                                        const long double sample = samples[index];
                                                        sample_energy += sample * sample;
                                                    }
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
                                          << " process_capture_nonzero_bytes=" << nonzero_bytes
                                          << " process_capture_sample_energy=" << static_cast<double>(sample_energy) << '\n';
                                print_hr("process_capture_stop", client->Stop());
                                print_hr("process_capture_reset", client->Reset());
                                data_ok = SUCCEEDED(read) && packet_count > 0 &&
                                          (!require_signal || nonzero_bytes > 0);
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

static int capture_data_probe(UINT target_index, DWORD duration_ms,
                              const char* output_path = nullptr,
                              bool event_driven = false) {
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
    HANDLE ready_event = nullptr;
    if (SUCCEEDED(hr) && event_driven) {
        ready_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!ready_event) hr = HRESULT_FROM_WIN32(GetLastError());
    }
    if (SUCCEEDED(hr)) hr = client->GetMixFormat(&format);
    print_hr("capture_get_mix_format", hr);
    if (SUCCEEDED(hr)) {
        print_format(format);
        DWORD stream_flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM |
                             AUDCLNT_STREAMFLAGS_NOPERSIST;
        if (event_driven) stream_flags |= AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
        hr = client->Initialize(AUDCLNT_SHAREMODE_SHARED, stream_flags,
                                1000000, 0, format, nullptr);
        print_hr("capture_initialize", hr);
        if (SUCCEEDED(hr) && event_driven) {
            hr = client->SetEventHandle(ready_event);
            print_hr("capture_set_event", hr);
        }
        if (SUCCEEDED(hr)) {
            REFERENCE_TIME default_period = 0;
            REFERENCE_TIME minimum_period = 0;
            const HRESULT period_hr = client->GetDevicePeriod(&default_period, &minimum_period);
            print_hr("capture_get_device_period", period_hr);
            if (SUCCEEDED(period_hr)) {
                std::cout << "capture_default_period_100ns=" << default_period
                          << " capture_minimum_period_100ns=" << minimum_period << '\n';
            }
        }
    }
    IAudioCaptureClient* capture = nullptr;
    if (SUCCEEDED(hr)) {
        hr = client->GetService(__uuidof(IAudioCaptureClient), reinterpret_cast<void**>(&capture));
        print_hr("capture_get_service", hr);
    }
    UINT32 packet_count = 0;
    UINT32 frame_count = 0;
    UINT64 nonzero_bytes = 0;
    UINT32 silent_packet_count = 0;
    std::ofstream capture_file;
    if (output_path) {
        capture_file.open(output_path, std::ios::binary | std::ios::trunc);
        if (!capture_file) {
            std::cout << "capture_file_open_failed=1 path=" << output_path << '\n';
            device->Release(); devices->Release(); enumerator->Release();
            return 1;
        }
    }
    if (SUCCEEDED(hr)) {
        hr = client->Start();
        print_hr("capture_start", hr);
        if (SUCCEEDED(hr)) {
            std::cout << "capture_start_tick_ms=" << GetTickCount64() << '\n';
            REFERENCE_TIME stream_latency = 0;
            const HRESULT latency_hr = client->GetStreamLatency(&stream_latency);
            print_hr("capture_get_stream_latency", latency_hr);
            if (SUCCEEDED(latency_hr)) {
                std::cout << "capture_stream_latency_100ns=" << stream_latency << '\n';
            }
            const auto deadline = std::chrono::steady_clock::now() +
                                  std::chrono::milliseconds(duration_ms);
            while (std::chrono::steady_clock::now() < deadline && SUCCEEDED(hr)) {
                UINT32 frames = 0;
                if (event_driven) {
                    const DWORD wait = WaitForSingleObject(ready_event, 50);
                    if (wait == WAIT_FAILED) {
                        hr = HRESULT_FROM_WIN32(GetLastError());
                        break;
                    }
                    if (wait == WAIT_TIMEOUT) continue;
                }
                hr = capture->GetNextPacketSize(&frames);
                if (FAILED(hr)) break;
                if (frames == 0) {
                    std::this_thread::sleep_for(std::chrono::milliseconds(2));
                    continue;
                }
                BYTE* data = nullptr;
                DWORD flags = 0;
                UINT64 position = 0;
                UINT64 timestamp = 0;
                hr = capture->GetBuffer(&data, &frames, &flags, &position, &timestamp);
                if (FAILED(hr)) break;
                ++packet_count;
                frame_count += frames;
                if ((flags & AUDCLNT_BUFFERFLAGS_SILENT) != 0) {
                    ++silent_packet_count;
                    if (capture_file) {
                        std::vector<BYTE> silence(static_cast<size_t>(frames) * format->nBlockAlign, 0);
                        capture_file.write(reinterpret_cast<const char*>(silence.data()),
                                           static_cast<std::streamsize>(silence.size()));
                    }
                } else if (data && format) {
                    const UINT64 payload_bytes = static_cast<UINT64>(frames) * format->nBlockAlign;
                    for (UINT64 byte = 0; byte < payload_bytes; ++byte) {
                        if (data[byte] != 0) ++nonzero_bytes;
                    }
                    if (capture_file) {
                        capture_file.write(reinterpret_cast<const char*>(data),
                                           static_cast<std::streamsize>(payload_bytes));
                    }
                }
                hr = capture->ReleaseBuffer(frames);
            }
            print_hr("capture_packet_read", hr);
            if (capture_file) capture_file.flush();
            std::cout << "capture_packets=" << packet_count << " capture_frames=" << frame_count
                      << " capture_silent_packets=" << silent_packet_count
                      << " capture_nonzero_bytes=" << nonzero_bytes << '\n';
            print_hr("capture_stop", client->Stop());
            print_hr("capture_reset", client->Reset());
        }
    }
    if (capture) capture->Release();
    if (ready_event) CloseHandle(ready_event);
    if (format) CoTaskMemFree(format);
    if (client) client->Release();
    device->Release(); devices->Release(); enumerator->Release();
    return SUCCEEDED(hr) && packet_count > 0 ? 0 : 1;
}

static int endpoint_inventory(EDataFlow flow, bool include_formats = false) {
    IMMDeviceEnumerator* enumerator = nullptr;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
    if (FAILED(hr)) { print_hr("inventory_enumerator", hr); return 1; }
    IMMDeviceCollection* devices = nullptr;
    hr = enumerator->EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE, &devices);
    if (FAILED(hr)) { print_hr("inventory_enum", hr); enumerator->Release(); return 1; }
    UINT count = 0;
    devices->GetCount(&count);
    std::wcout << (flow == eRender ? L"render" : L"capture") << L"_endpoint_count=" << count << L'\n';
    for (UINT index = 0; index < count; ++index) {
        IMMDevice* device = nullptr;
        if (FAILED(devices->Item(index, &device))) continue;
        LPWSTR id = nullptr;
        device->GetId(&id);
        std::wcout << (flow == eRender ? L"render" : L"capture") << L"[" << index << L"] name="
                   << endpoint_name(device) << L" id=" << (id ? id : L"<unknown>") << L'\n';
        if (include_formats) {
            IAudioClient* client = nullptr;
            WAVEFORMATEX* format = nullptr;
            HRESULT format_hr = device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr,
                                                 reinterpret_cast<void**>(&client));
            if (SUCCEEDED(format_hr)) format_hr = client->GetMixFormat(&format);
            std::cout << (flow == eRender ? "render" : "capture") << "[" << index
                      << "] mix_format=";
            print_hr("activate_or_format", format_hr);
            if (SUCCEEDED(format_hr)) print_format(format);
            if (format) CoTaskMemFree(format);
            if (client) client->Release();
        }
        CoTaskMemFree(id);
        device->Release();
    }
    devices->Release();
    enumerator->Release();
    return 0;
}

static bool render_tone(BYTE* data, UINT32 frames, const WAVEFORMATEX* format,
                        double& phase, double frequency) {
    if (!data || !format || format->nChannels == 0 || format->nSamplesPerSec == 0) return false;
    const bool is_float = format->wFormatTag == WAVE_FORMAT_IEEE_FLOAT ||
        (format->wFormatTag == WAVE_FORMAT_EXTENSIBLE &&
         format->cbSize >= sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX) &&
         reinterpret_cast<const WAVEFORMATEXTENSIBLE*>(format)->SubFormat ==
             KSDATAFORMAT_SUBTYPE_IEEE_FLOAT);
    const double step = 2.0 * 3.14159265358979323846 * frequency /
                        static_cast<double>(format->nSamplesPerSec);
    if (is_float && format->wBitsPerSample == 32) {
        auto* samples = reinterpret_cast<float*>(data);
        for (UINT32 frame = 0; frame < frames; ++frame) {
            const float sample = static_cast<float>(0.1 * std::sin(phase));
            for (UINT channel = 0; channel < format->nChannels; ++channel) {
                samples[static_cast<size_t>(frame) * format->nChannels + channel] = sample;
            }
            phase += step;
        }
        return true;
    }
    if (!is_float && format->wFormatTag == WAVE_FORMAT_PCM && format->wBitsPerSample == 16) {
        auto* samples = reinterpret_cast<SHORT*>(data);
        for (UINT32 frame = 0; frame < frames; ++frame) {
            const SHORT sample = static_cast<SHORT>(0.1 * 32767.0 * std::sin(phase));
            for (UINT channel = 0; channel < format->nChannels; ++channel) {
                samples[static_cast<size_t>(frame) * format->nChannels + channel] = sample;
            }
            phase += step;
        }
        return true;
    }
    std::memset(data, 0, static_cast<size_t>(frames) * format->nBlockAlign);
    return false;
}

static bool render_impulses(BYTE* data, UINT32 frames, const WAVEFORMATEX* format,
                            UINT64& sample_index) {
    if (!data || !format || format->nChannels == 0 || format->nSamplesPerSec < 100) return false;
    const bool is_float = format->wFormatTag == WAVE_FORMAT_IEEE_FLOAT ||
        (format->wFormatTag == WAVE_FORMAT_EXTENSIBLE &&
         format->cbSize >= sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX) &&
         reinterpret_cast<const WAVEFORMATEXTENSIBLE*>(format)->SubFormat ==
             KSDATAFORMAT_SUBTYPE_IEEE_FLOAT);
    const UINT32 interval = format->nSamplesPerSec / 100;
    std::memset(data, 0, static_cast<size_t>(frames) * format->nBlockAlign);
    UINT32 impulse_count = 0;
    if (is_float && format->wBitsPerSample == 32) {
        auto* samples = reinterpret_cast<float*>(data);
        for (UINT32 frame = 0; frame < frames; ++frame, ++sample_index) {
            if (sample_index % interval == 0) {
                for (UINT channel = 0; channel < format->nChannels; ++channel)
                    samples[static_cast<size_t>(frame) * format->nChannels + channel] = 0.5f;
                ++impulse_count;
            }
        }
    } else if (!is_float && format->wFormatTag == WAVE_FORMAT_PCM && format->wBitsPerSample == 16) {
        auto* samples = reinterpret_cast<SHORT*>(data);
        for (UINT32 frame = 0; frame < frames; ++frame, ++sample_index) {
            if (sample_index % interval == 0) {
                for (UINT channel = 0; channel < format->nChannels; ++channel)
                    samples[static_cast<size_t>(frame) * format->nChannels + channel] = 16384;
                ++impulse_count;
            }
        }
    } else {
        return false;
    }
    return impulse_count > 0;
}

static int render_data_probe(UINT target_index, DWORD duration_ms, bool tone,
                             bool impulses = false, bool event_driven = false) {
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
    HANDLE ready_event = nullptr;
    if (SUCCEEDED(hr) && event_driven) {
        ready_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!ready_event) hr = HRESULT_FROM_WIN32(GetLastError());
    }
    if (SUCCEEDED(hr)) hr = client->GetMixFormat(&format);
    print_hr("render_get_mix_format", hr);
    if (SUCCEEDED(hr)) {
        print_format(format);
        DWORD stream_flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM |
                             AUDCLNT_STREAMFLAGS_NOPERSIST;
        if (event_driven) stream_flags |= AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
        hr = client->Initialize(AUDCLNT_SHAREMODE_SHARED, stream_flags,
                                1000000, 0, format, nullptr);
        print_hr("render_initialize", hr);
        if (SUCCEEDED(hr) && event_driven) {
            hr = client->SetEventHandle(ready_event);
            print_hr("render_set_event", hr);
        }
        if (SUCCEEDED(hr)) {
            REFERENCE_TIME default_period = 0;
            REFERENCE_TIME minimum_period = 0;
            const HRESULT period_hr = client->GetDevicePeriod(&default_period, &minimum_period);
            print_hr("render_get_device_period", period_hr);
            if (SUCCEEDED(period_hr)) {
                std::cout << "render_default_period_100ns=" << default_period
                          << " render_minimum_period_100ns=" << minimum_period << '\n';
            }
        }
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
    double phase = 0.0;
    UINT64 impulse_sample_index = 0;
    bool tone_written = !tone;
    bool impulses_written = !impulses;
    if (SUCCEEDED(hr)) {
        hr = client->Start();
        print_hr("render_start", hr);
        if (SUCCEEDED(hr)) {
            std::cout << "render_start_tick_ms=" << GetTickCount64() << '\n';
            REFERENCE_TIME stream_latency = 0;
            const HRESULT latency_hr = client->GetStreamLatency(&stream_latency);
            print_hr("render_get_stream_latency", latency_hr);
            if (SUCCEEDED(latency_hr)) {
                std::cout << "render_stream_latency_100ns=" << stream_latency << '\n';
            }
            const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(duration_ms);
            while (std::chrono::steady_clock::now() < deadline) {
                if (event_driven) {
                    const DWORD wait = WaitForSingleObject(ready_event, 50);
                    if (wait == WAIT_FAILED) {
                        hr = HRESULT_FROM_WIN32(GetLastError());
                        break;
                    }
                    if (wait == WAIT_TIMEOUT) continue;
                }
                UINT32 padding = 0;
                hr = client->GetCurrentPadding(&padding);
                if (FAILED(hr)) break;
                const UINT32 available = buffer_size - padding;
                if (available != 0) {
                    BYTE* data = nullptr;
                    hr = render->GetBuffer(available, &data);
                    if (FAILED(hr)) break;
                    if (tone) tone_written = render_tone(data, available, format, phase, 997.0) || tone_written;
                    if (impulses) impulses_written = render_impulses(data, available, format, impulse_sample_index) || impulses_written;
                    hr = render->ReleaseBuffer(available, (tone || impulses) ? 0 : AUDCLNT_BUFFERFLAGS_SILENT);
                    if (FAILED(hr)) break;
                    submitted_frames += available;
                }
                std::this_thread::sleep_for(std::chrono::milliseconds(10));
            }
            print_hr("render_silent_submit", hr);
            std::cout << "render_buffer_size=" << buffer_size
                      << " render_submitted_frames=" << submitted_frames
                      << " render_tone_written=" << (tone_written ? 1 : 0)
                      << " render_impulse_written=" << (impulses_written ? 1 : 0) << '\n';
            print_hr("render_stop", client->Stop());
            print_hr("render_reset", client->Reset());
        }
    }
    if (render) render->Release();
    if (ready_event) CloseHandle(ready_event);
    if (format) CoTaskMemFree(format);
    if (client) client->Release();
    device->Release(); devices->Release(); enumerator->Release();
    return SUCCEEDED(hr) && submitted_frames > 0 && tone_written && impulses_written ? 0 : 1;
}

// Owns exactly one activated shared-mode WASAPI endpoint (device, client,
// negotiated mix format, and its audio clock). Single-threaded, single-run
// lifetime: constructed and released within one call to
// impulse_loopback_probe, never shared across threads or outlives that call.
struct LoopbackStreamState {
    IMMDevice* device = nullptr;
    // IAudioClient3 is interface-compatible with IAudioClient (it inherits
    // Initialize, Start, Stop, GetService, etc. unchanged) and additionally
    // exposes GetSharedModeEnginePeriod/InitializeSharedAudioStream, the
    // Windows 10+ low-latency shared-mode path; activating it directly lets
    // the same struct serve both the legacy and low-latency init calls.
    IAudioClient3* client = nullptr;
    WAVEFORMATEX* format = nullptr;
    IAudioClock* clock = nullptr;
    UINT64 clock_frequency = 0;
    HANDLE ready_event = nullptr;

    void release() {
        if (clock) { clock->Release(); clock = nullptr; }
        if (format) { CoTaskMemFree(format); format = nullptr; }
        if (client) { client->Release(); client = nullptr; }
        if (device) { device->Release(); device = nullptr; }
        if (ready_event) { CloseHandle(ready_event); ready_event = nullptr; }
    }
};

// event_driven registers a per-stream notification handle and initializes
// with AUDCLNT_STREAMFLAGS_EVENTCALLBACK; a small polled buffer cannot be
// serviced reliably from a single cooperative loop shared with the other
// stream (observed as tens of thousands of dropped capture frames), so the
// loopback probe runs each stream event-driven on its own thread instead.
// low_latency uses IAudioClient3::GetSharedModeEnginePeriod to discover this
// driver's minimum supported shared-mode engine period in frames, then
// InitializeSharedAudioStream with that period, instead of the legacy
// Initialize call whose small hnsBufferDuration requests Windows silently
// clamps to its own default engine period for shared streams.
static bool activate_shared_stream(IMMDeviceEnumerator* enumerator, EDataFlow flow, UINT index,
                                   LoopbackStreamState& state, const char* label,
                                   bool event_driven = false, REFERENCE_TIME buffer_100ns = 0,
                                   bool low_latency = false) {
    IMMDeviceCollection* devices = nullptr;
    HRESULT hr = enumerator->EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE, &devices);
    print_hr((std::string(label) + "_enum").c_str(), hr);
    if (FAILED(hr)) return false;
    UINT count = 0;
    devices->GetCount(&count);
    if (index >= count) {
        std::cout << label << "_index_out_of_range=" << index << " count=" << count << '\n';
        devices->Release();
        return false;
    }
    hr = devices->Item(index, &state.device);
    print_hr((std::string(label) + "_item").c_str(), hr);
    devices->Release();
    if (FAILED(hr)) return false;
    hr = state.device->Activate(__uuidof(IAudioClient3), CLSCTX_ALL, nullptr,
                                reinterpret_cast<void**>(&state.client));
    print_hr((std::string(label) + "_activate").c_str(), hr);
    if (FAILED(hr)) return false;
    hr = state.client->GetMixFormat(&state.format);
    print_hr((std::string(label) + "_get_mix_format").c_str(), hr);
    if (FAILED(hr)) return false;
    std::cout << label << "_"; print_format(state.format);
    if (event_driven) {
        state.ready_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!state.ready_event) {
            print_hr((std::string(label) + "_create_event").c_str(), HRESULT_FROM_WIN32(GetLastError()));
            return false;
        }
    }
    DWORD stream_flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST;
    if (event_driven) stream_flags |= AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
    if (low_latency) {
        UINT32 default_period_frames = 0, fundamental_period_frames = 0;
        UINT32 min_period_frames = 0, max_period_frames = 0;
        hr = state.client->GetSharedModeEnginePeriod(state.format, &default_period_frames,
                                                      &fundamental_period_frames, &min_period_frames,
                                                      &max_period_frames);
        print_hr((std::string(label) + "_get_shared_mode_engine_period").c_str(), hr);
        if (FAILED(hr)) return false;
        std::cout << label << "_engine_period_frames default=" << default_period_frames
                  << " fundamental=" << fundamental_period_frames
                  << " min=" << min_period_frames << " max=" << max_period_frames << '\n';
        // InitializeSharedAudioStream requires AUDCLNT_STREAMFLAGS_EVENTCALLBACK
        // and rejects AUTOCONVERTPCM/NOPERSIST here (AUDCLNT_E_INVALID_DEVICE_PERIOD
        // even for an in-range period) since the exact negotiated mix format is
        // passed directly with no conversion needed.
        const DWORD low_latency_flags = event_driven ? AUDCLNT_STREAMFLAGS_EVENTCALLBACK : 0;
        hr = state.client->InitializeSharedAudioStream(low_latency_flags, min_period_frames, state.format,
                                                        nullptr);
        print_hr((std::string(label) + "_initialize_shared_audio_stream").c_str(), hr);
    } else {
        // A zero buffer duration in shared mode asks WASAPI for its minimal
        // engine-period buffer instead of the large diagnostic buffer the
        // other probes in this file use; a large buffer would add its own
        // size directly to the measured round trip and defeat this
        // measurement. buffer_100ns lets a caller request a smaller
        // explicit buffer than the engine's own default-with-zero choice,
        // though the legacy Initialize call clamps this to the engine's
        // own default period regardless (see low_latency above).
        hr = state.client->Initialize(AUDCLNT_SHAREMODE_SHARED, stream_flags, buffer_100ns, 0,
                                      state.format, nullptr);
        print_hr((std::string(label) + "_initialize").c_str(), hr);
    }
    if (FAILED(hr)) return false;
    if (event_driven) {
        hr = state.client->SetEventHandle(state.ready_event);
        print_hr((std::string(label) + "_set_event").c_str(), hr);
        if (FAILED(hr)) return false;
    }
    hr = state.client->GetService(__uuidof(IAudioClock), reinterpret_cast<void**>(&state.clock));
    print_hr((std::string(label) + "_get_clock").c_str(), hr);
    if (FAILED(hr)) return false;
    hr = state.clock->GetFrequency(&state.clock_frequency);
    print_hr((std::string(label) + "_clock_frequency").c_str(), hr);
    return SUCCEEDED(hr);
}

// Extrapolates the 100ns-unit QPC time at which this stream's position was
// (or will be) zero, from one GetPosition sample. Two samples taken at the
// start and end of a run bound clock drift over that run; both anchors are
// reported so drift is visible rather than silently absorbed.
static bool clock_anchor_100ns(IAudioClock* clock, UINT64 frequency, double& anchor_100ns) {
    if (!clock || frequency == 0) return false;
    UINT64 position = 0;
    UINT64 qpc_100ns = 0;
    if (FAILED(clock->GetPosition(&position, &qpc_100ns))) return false;
    anchor_100ns = static_cast<double>(qpc_100ns) -
                  (static_cast<double>(position) / static_cast<double>(frequency)) * 1.0e7;
    return true;
}

// Single-process wired physical loopback: renders impulses to render_index
// and captures from capture_index concurrently, using each stream's own
// IAudioClock (rather than independent process-launch timestamps) to
// compute a calibrated per-impulse round-trip latency distribution. This is
// the NFR-01 measurement path; it measures the raw WASAPI/driver/cable
// round trip only, not AudioRouter's own graph-dispatch latency (tracked
// separately).
static int impulse_loopback_probe(UINT render_index, UINT capture_index, DWORD impulse_count,
                                  REFERENCE_TIME buffer_100ns = 0, bool low_latency = false) {
    IMMDeviceEnumerator* enumerator = nullptr;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
    if (FAILED(hr)) { print_hr("loopback_enumerator", hr); return 1; }

    LoopbackStreamState render_state;
    LoopbackStreamState capture_state;
    bool ok = activate_shared_stream(enumerator, eRender, render_index, render_state, "loopback_render",
                                     true, buffer_100ns, low_latency);
    if (ok) ok = activate_shared_stream(enumerator, eCapture, capture_index, capture_state, "loopback_capture",
                                        true, buffer_100ns, low_latency);
    enumerator->Release();
    if (!ok) { render_state.release(); capture_state.release(); return 1; }

    const bool capture_is_float32 = capture_state.format->wBitsPerSample == 32 &&
        (capture_state.format->wFormatTag == WAVE_FORMAT_IEEE_FLOAT ||
         (capture_state.format->wFormatTag == WAVE_FORMAT_EXTENSIBLE &&
          capture_state.format->cbSize >= sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX) &&
          reinterpret_cast<const WAVEFORMATEXTENSIBLE*>(capture_state.format)->SubFormat ==
              KSDATAFORMAT_SUBTYPE_IEEE_FLOAT));
    if (!capture_is_float32) {
        std::cout << "loopback_capture_requires_32bit_float=1\n";
        render_state.release(); capture_state.release();
        return 1;
    }

    IAudioRenderClient* render_service = nullptr;
    hr = render_state.client->GetService(__uuidof(IAudioRenderClient), reinterpret_cast<void**>(&render_service));
    print_hr("loopback_render_get_service", hr);
    IAudioCaptureClient* capture_service = nullptr;
    if (SUCCEEDED(hr)) {
        hr = capture_state.client->GetService(__uuidof(IAudioCaptureClient), reinterpret_cast<void**>(&capture_service));
        print_hr("loopback_capture_get_service", hr);
    }
    if (FAILED(hr)) {
        if (render_service) render_service->Release();
        render_state.release(); capture_state.release();
        return 1;
    }

    UINT32 render_buffer_size = 0;
    render_state.client->GetBufferSize(&render_buffer_size);
    UINT32 capture_buffer_size = 0;
    capture_state.client->GetBufferSize(&capture_buffer_size);
    REFERENCE_TIME render_default_period = 0, render_minimum_period = 0;
    render_state.client->GetDevicePeriod(&render_default_period, &render_minimum_period);
    REFERENCE_TIME capture_default_period = 0, capture_minimum_period = 0;
    capture_state.client->GetDevicePeriod(&capture_default_period, &capture_minimum_period);
    std::cout << "loopback_render_buffer_frames=" << render_buffer_size
              << " loopback_capture_buffer_frames=" << capture_buffer_size
              << " loopback_render_default_period_100ns=" << render_default_period
              << " loopback_capture_default_period_100ns=" << capture_default_period
              << " loopback_render_minimum_period_100ns=" << render_minimum_period
              << " loopback_capture_minimum_period_100ns=" << capture_minimum_period << '\n';
    REFERENCE_TIME render_stream_latency = 0, capture_stream_latency = 0;
    render_state.client->GetStreamLatency(&render_stream_latency);
    capture_state.client->GetStreamLatency(&capture_stream_latency);
    std::cout << "loopback_render_stream_latency_100ns=" << render_stream_latency
              << " loopback_capture_stream_latency_100ns=" << capture_stream_latency << '\n';
    std::cout << "loopback_render_clock_frequency=" << render_state.clock_frequency
              << " loopback_render_format_rate=" << render_state.format->nSamplesPerSec
              << " loopback_capture_clock_frequency=" << capture_state.clock_frequency
              << " loopback_capture_format_rate=" << capture_state.format->nSamplesPerSec << '\n';

    const UINT32 interval = render_state.format->nSamplesPerSec / 100; // fixed 10 ms cadence
    const DWORD render_duration_ms = static_cast<DWORD>(impulse_count) * 10;
    const DWORD capture_duration_ms = render_duration_ms + 1000;

    UINT64 impulse_sample_index = 0;
    UINT64 impulses_emitted = 0;
    UINT32 render_submitted_frames = 0;
    UINT64 capture_frames_received = 0;
    UINT64 capture_dropped_frames = 0;

    // Each detected impulse's timestamp is taken directly from
    // IAudioCaptureClient::GetBuffer's own per-packet device position and
    // QPC timestamp, rather than reconstructed from an accumulated sample
    // count; this stays correct even if a buffer overrun drops packets; the
    // dropped-frame count above is reported, not hidden, when this happens.
    std::vector<double> capture_group_qpc_100ns;
    const UINT32 channels = capture_state.format->nChannels;
    const double capture_rate = static_cast<double>(capture_state.format->nSamplesPerSec);

    std::atomic<bool> stop_requested{false};

    // A polled small buffer (~22 ms here) cannot be serviced reliably by a
    // single cooperative loop alternating between two streams; each stream
    // therefore runs event-driven on its own OS thread, woken directly by
    // WASAPI when its buffer needs service.
    std::thread render_thread([&] {
        while (!stop_requested.load(std::memory_order_relaxed)) {
            const DWORD wait = WaitForSingleObject(render_state.ready_event, 100);
            if (wait == WAIT_FAILED) break;
            if (wait == WAIT_TIMEOUT) continue;
            UINT32 padding = 0;
            if (FAILED(render_state.client->GetCurrentPadding(&padding))) continue;
            const UINT32 available = render_buffer_size - padding;
            if (available == 0) continue;
            BYTE* data = nullptr;
            if (FAILED(render_service->GetBuffer(available, &data))) continue;
            const UINT64 before = impulse_sample_index;
            render_impulses(data, available, render_state.format, impulse_sample_index);
            impulses_emitted += (impulse_sample_index / interval) - (before / interval);
            render_service->ReleaseBuffer(available, 0);
            render_submitted_frames += available;
        }
    });

    std::thread capture_thread([&] {
        bool have_capture_position = false;
        UINT64 expected_next_position = 0;
        INT64 last_hit_position = -1000000;
        while (!stop_requested.load(std::memory_order_relaxed)) {
            const DWORD wait = WaitForSingleObject(capture_state.ready_event, 100);
            if (wait == WAIT_FAILED) break;
            if (wait == WAIT_TIMEOUT) continue;
            for (;;) {
                UINT32 frames = 0;
                if (FAILED(capture_service->GetNextPacketSize(&frames)) || frames == 0) break;
                BYTE* data = nullptr;
                DWORD flags = 0;
                UINT64 position = 0;
                UINT64 timestamp = 0;
                if (FAILED(capture_service->GetBuffer(&data, &frames, &flags, &position, &timestamp))) break;
                if (have_capture_position && position > expected_next_position) {
                    capture_dropped_frames += position - expected_next_position;
                }
                have_capture_position = true;
                expected_next_position = position + frames;
                capture_frames_received += frames;
                if ((flags & AUDCLNT_BUFFERFLAGS_SILENT) == 0 && data) {
                    const auto* samples = reinterpret_cast<const float*>(data);
                    for (UINT32 frame = 0; frame < frames; ++frame) {
                        float peak = 0.0f;
                        for (UINT32 channel = 0; channel < channels; ++channel) {
                            const float sample = std::fabs(samples[static_cast<size_t>(frame) * channels + channel]);
                            if (sample > peak) peak = sample;
                        }
                        const INT64 device_frame = static_cast<INT64>(position) + frame;
                        if (peak > 0.05f && device_frame > last_hit_position + 8) {
                            capture_group_qpc_100ns.push_back(static_cast<double>(timestamp) +
                                (static_cast<double>(frame) / capture_rate) * 1.0e7);
                            last_hit_position = device_frame;
                        }
                    }
                }
                capture_service->ReleaseBuffer(frames);
            }
        }
    });

    hr = capture_state.client->Start();
    print_hr("loopback_capture_start", hr);
    std::this_thread::sleep_for(std::chrono::milliseconds(150));
    if (SUCCEEDED(hr)) hr = render_state.client->Start();
    print_hr("loopback_render_start", hr);
    double render_anchor_start = 0.0;
    clock_anchor_100ns(render_state.clock, render_state.clock_frequency, render_anchor_start);

    if (SUCCEEDED(hr)) {
        std::this_thread::sleep_for(std::chrono::milliseconds(capture_duration_ms));
    }
    stop_requested.store(true, std::memory_order_relaxed);
    SetEvent(render_state.ready_event);
    SetEvent(capture_state.ready_event);
    render_thread.join();
    capture_thread.join();

    double render_anchor_end = 0.0;
    clock_anchor_100ns(render_state.clock, render_state.clock_frequency, render_anchor_end);

    print_hr("loopback_render_stop", render_state.client->Stop());
    print_hr("loopback_capture_stop", capture_state.client->Stop());
    render_state.client->Reset();
    capture_state.client->Reset();

    std::cout << "loopback_render_submitted_frames=" << render_submitted_frames
              << " loopback_impulses_emitted=" << impulses_emitted
              << " loopback_capture_frames=" << capture_frames_received
              << " loopback_capture_dropped_frames=" << capture_dropped_frames << '\n';
    std::cout << "loopback_render_clock_drift_100ns=" << (render_anchor_end - render_anchor_start) << '\n';
    std::cout << std::fixed << std::setprecision(1)
              << "loopback_render_anchor_start_100ns=" << render_anchor_start
              << " loopback_render_anchor_end_100ns=" << render_anchor_end
              << " loopback_first_group_qpc_100ns="
              << (capture_group_qpc_100ns.empty() ? 0.0 : capture_group_qpc_100ns.front())
              << '\n';
    std::cout.unsetf(std::ios::floatfield);
    std::cout.precision(6);
    std::cout << "loopback_detected_groups=" << capture_group_qpc_100ns.size() << '\n';

    render_service->Release();
    capture_service->Release();

    const UINT64 target_pairs = std::min<UINT64>(impulse_count, impulses_emitted);
    const UINT64 pairs = std::min<UINT64>(capture_group_qpc_100ns.size(), target_pairs);
    if (target_pairs == 0 || pairs < (target_pairs * 9 / 10)) {
        std::cout << "loopback_insufficient_pairs=1 pairs=" << pairs
                  << " target_pairs=" << target_pairs << '\n';
        render_state.release(); capture_state.release();
        return 1;
    }

    // Render has no per-buffer timestamp API, so its side is measured via
    // IAudioClock. GetPosition's device position advances in this driver's
    // own native unit (empirically the byte rate reported by GetFrequency
    // here, not necessarily the format's frame rate), so frame indices are
    // converted via nBlockAlign before dividing by frequency. Capture uses
    // the WASAPI-supplied per-packet QPC timestamp directly, on the same
    // performance-counter timeline as the render anchor.
    //
    // The render anchor deliberately uses the END-of-run sample alone, not
    // an average with the start sample: `render-clock-ramp` diagnostics
    // showed this driver's render position stays at exactly 0 for a real
    // ~41-45ms engine warm-up after Start() before advancing at the steady
    // rate GetFrequency predicts (confirmed to matching sub-millisecond
    // precision once running). The start-of-run anchor is sampled during
    // that unreliable warm-up window and is therefore biased ~41-45ms
    // early; the end-of-run anchor is extrapolated backward from confirmed
    // steady-state data and is not.
    const double render_anchor = render_anchor_end;
    std::vector<double> latencies_ms;
    latencies_ms.reserve(static_cast<size_t>(pairs));
    for (UINT64 k = 0; k < pairs; ++k) {
        const double render_frame_time_100ns = render_anchor +
            (static_cast<double>(k) * interval * render_state.format->nBlockAlign /
             static_cast<double>(render_state.clock_frequency)) * 1.0e7;
        latencies_ms.push_back(
            (capture_group_qpc_100ns[static_cast<size_t>(k)] - render_frame_time_100ns) / 10000.0);
    }
    std::sort(latencies_ms.begin(), latencies_ms.end());
    auto percentile = [&](double p) {
        const size_t idx = static_cast<size_t>(std::min<double>(
            static_cast<double>(latencies_ms.size() - 1), std::floor(p * latencies_ms.size())));
        return latencies_ms[idx];
    };
    const double mean = std::accumulate(latencies_ms.begin(), latencies_ms.end(), 0.0) /
                        static_cast<double>(latencies_ms.size());
    std::cout << "loopback_pairs=" << pairs
              << " loopback_latency_min_ms=" << latencies_ms.front()
              << " loopback_latency_p50_ms=" << percentile(0.50)
              << " loopback_latency_p95_ms=" << percentile(0.95)
              << " loopback_latency_max_ms=" << latencies_ms.back()
              << " loopback_latency_mean_ms=" << mean << '\n';
    std::cout << "Scope: raw WASAPI render-write to capture-read round trip over the wired "
                 "physical cable, calibrated via the render IAudioClock and the capture "
                 "per-packet QPC timestamp on a shared performance-counter timeline; excludes "
                 "any AudioRouter graph/dispatch processing latency, which is tracked "
                 "separately.\n";

    render_state.release();
    capture_state.release();
    return 0;
}

// Captures a stream's own impulse-arrival timestamps: a persistent worker
// thread that drains packets from `service`, peak-detects impulses using
// each packet's own device position and QPC timestamp (immune to render
// warm-up bias since nothing is extrapolated from a Start()-time sample),
// and appends (device_frame, qpc_100ns) pairs to `groups`. Shared by both
// capture sides of capture_loopback_probe.
struct CaptureGroup {
    INT64 device_frame;
    double qpc_100ns;
};

static void run_capture_worker(LoopbackStreamState& state, IAudioCaptureClient* service,
                               std::atomic<bool>& stop_requested, std::vector<CaptureGroup>& groups,
                               UINT64& frames_received, UINT64& dropped_frames) {
    const UINT32 channels = state.format->nChannels;
    bool have_position = false;
    UINT64 expected_next_position = 0;
    INT64 last_hit_position = -1000000;
    while (!stop_requested.load(std::memory_order_relaxed)) {
        const DWORD wait = WaitForSingleObject(state.ready_event, 100);
        if (wait == WAIT_FAILED) break;
        if (wait == WAIT_TIMEOUT) continue;
        for (;;) {
            UINT32 frames = 0;
            if (FAILED(service->GetNextPacketSize(&frames)) || frames == 0) break;
            BYTE* data = nullptr;
            DWORD flags = 0;
            UINT64 position = 0;
            UINT64 timestamp = 0;
            if (FAILED(service->GetBuffer(&data, &frames, &flags, &position, &timestamp))) break;
            if (have_position && position > expected_next_position) {
                dropped_frames += position - expected_next_position;
            }
            have_position = true;
            expected_next_position = position + frames;
            frames_received += frames;
            if ((flags & AUDCLNT_BUFFERFLAGS_SILENT) == 0 && data) {
                const auto* samples = reinterpret_cast<const float*>(data);
                for (UINT32 frame = 0; frame < frames; ++frame) {
                    float peak = 0.0f;
                    for (UINT32 channel = 0; channel < channels; ++channel) {
                        const float sample = std::fabs(samples[static_cast<size_t>(frame) * channels + channel]);
                        if (sample > peak) peak = sample;
                    }
                    const INT64 device_frame = static_cast<INT64>(position) + frame;
                    if (peak > 0.05f && device_frame > last_hit_position + 8) {
                        groups.push_back(CaptureGroup{
                            device_frame,
                            static_cast<double>(timestamp) +
                                (static_cast<double>(frame) / static_cast<double>(state.format->nSamplesPerSec))
                                    * 1.0e7});
                        last_hit_position = device_frame;
                    }
                }
            }
            service->ReleaseBuffer(frames);
        }
    }
}

// NFR-02 measurement: mic-to-virtual-capture latency. A render stream
// (render_index) generates impulses into a physical loop feeding the "mic"
// capture endpoint (capture_a_index); a second, independent capture stream
// on the virtual capture endpoint (capture_b_index, e.g. a VB-Cable output
// an AudioRouter-routed session feeds) times the same impulses' arrival
// after engine processing. Both sides are ordinary WASAPI capture streams
// timestamped via IAudioCaptureClient::GetBuffer's own per-packet device
// position and QPC timestamp, so unlike NFR-01 there is no render-side
// IAudioClock anchor and no associated warm-up bias to correct for. The
// caller is responsible for having a live AudioRouter route already running
// from capture_a_index's physical endpoint to whatever virtual render
// endpoint feeds capture_b_index before invoking this.
static int capture_loopback_probe(UINT render_index, UINT capture_a_index, UINT capture_b_index,
                                  DWORD impulse_count) {
    IMMDeviceEnumerator* enumerator = nullptr;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
    if (FAILED(hr)) { print_hr("nfr02_enumerator", hr); return 1; }

    LoopbackStreamState render_state, capture_a_state, capture_b_state;
    bool ok = activate_shared_stream(enumerator, eRender, render_index, render_state, "nfr02_render", true);
    if (ok) ok = activate_shared_stream(enumerator, eCapture, capture_a_index, capture_a_state, "nfr02_capture_a", true);
    if (ok) ok = activate_shared_stream(enumerator, eCapture, capture_b_index, capture_b_state, "nfr02_capture_b", true);
    enumerator->Release();
    if (!ok) { render_state.release(); capture_a_state.release(); capture_b_state.release(); return 1; }

    auto is_float32 = [](const WAVEFORMATEX* format) {
        return format->wBitsPerSample == 32 &&
            (format->wFormatTag == WAVE_FORMAT_IEEE_FLOAT ||
             (format->wFormatTag == WAVE_FORMAT_EXTENSIBLE &&
              format->cbSize >= sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX) &&
              reinterpret_cast<const WAVEFORMATEXTENSIBLE*>(format)->SubFormat == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT));
    };
    if (!is_float32(capture_a_state.format) || !is_float32(capture_b_state.format)) {
        std::cout << "nfr02_capture_requires_32bit_float=1\n";
        render_state.release(); capture_a_state.release(); capture_b_state.release();
        return 1;
    }

    IAudioRenderClient* render_service = nullptr;
    hr = render_state.client->GetService(__uuidof(IAudioRenderClient), reinterpret_cast<void**>(&render_service));
    print_hr("nfr02_render_get_service", hr);
    IAudioCaptureClient* capture_a_service = nullptr;
    if (SUCCEEDED(hr)) {
        hr = capture_a_state.client->GetService(__uuidof(IAudioCaptureClient), reinterpret_cast<void**>(&capture_a_service));
        print_hr("nfr02_capture_a_get_service", hr);
    }
    IAudioCaptureClient* capture_b_service = nullptr;
    if (SUCCEEDED(hr)) {
        hr = capture_b_state.client->GetService(__uuidof(IAudioCaptureClient), reinterpret_cast<void**>(&capture_b_service));
        print_hr("nfr02_capture_b_get_service", hr);
    }
    if (FAILED(hr)) {
        if (render_service) render_service->Release();
        if (capture_a_service) capture_a_service->Release();
        render_state.release(); capture_a_state.release(); capture_b_state.release();
        return 1;
    }

    UINT32 render_buffer_size = 0;
    render_state.client->GetBufferSize(&render_buffer_size);
    const UINT32 interval = render_state.format->nSamplesPerSec / 100;
    const DWORD render_duration_ms = static_cast<DWORD>(impulse_count) * 10;
    const DWORD capture_duration_ms = render_duration_ms + 1500;

    UINT64 impulse_sample_index = 0;
    UINT64 impulses_emitted = 0;
    UINT32 render_submitted_frames = 0;
    UINT64 capture_a_frames = 0, capture_a_dropped = 0;
    UINT64 capture_b_frames = 0, capture_b_dropped = 0;
    std::vector<CaptureGroup> groups_a, groups_b;
    std::atomic<bool> stop_requested{false};

    std::thread render_thread([&] {
        while (!stop_requested.load(std::memory_order_relaxed)) {
            const DWORD wait = WaitForSingleObject(render_state.ready_event, 100);
            if (wait == WAIT_FAILED) break;
            if (wait == WAIT_TIMEOUT) continue;
            UINT32 padding = 0;
            if (FAILED(render_state.client->GetCurrentPadding(&padding))) continue;
            const UINT32 available = render_buffer_size - padding;
            if (available == 0) continue;
            BYTE* data = nullptr;
            if (FAILED(render_service->GetBuffer(available, &data))) continue;
            const UINT64 before = impulse_sample_index;
            render_impulses(data, available, render_state.format, impulse_sample_index);
            impulses_emitted += (impulse_sample_index / interval) - (before / interval);
            render_service->ReleaseBuffer(available, 0);
            render_submitted_frames += available;
        }
    });
    std::thread capture_a_thread([&] {
        run_capture_worker(capture_a_state, capture_a_service, stop_requested, groups_a,
                           capture_a_frames, capture_a_dropped);
    });
    std::thread capture_b_thread([&] {
        run_capture_worker(capture_b_state, capture_b_service, stop_requested, groups_b,
                           capture_b_frames, capture_b_dropped);
    });

    hr = capture_b_state.client->Start();
    print_hr("nfr02_capture_b_start", hr);
    if (SUCCEEDED(hr)) hr = capture_a_state.client->Start();
    print_hr("nfr02_capture_a_start", hr);
    std::this_thread::sleep_for(std::chrono::milliseconds(150));
    if (SUCCEEDED(hr)) hr = render_state.client->Start();
    print_hr("nfr02_render_start", hr);

    if (SUCCEEDED(hr)) {
        std::this_thread::sleep_for(std::chrono::milliseconds(capture_duration_ms));
    }
    stop_requested.store(true, std::memory_order_relaxed);
    SetEvent(render_state.ready_event);
    SetEvent(capture_a_state.ready_event);
    SetEvent(capture_b_state.ready_event);
    render_thread.join();
    capture_a_thread.join();
    capture_b_thread.join();

    render_state.client->Stop();
    capture_a_state.client->Stop();
    capture_b_state.client->Stop();
    render_state.client->Reset();
    capture_a_state.client->Reset();
    capture_b_state.client->Reset();

    std::cout << "nfr02_render_submitted_frames=" << render_submitted_frames
              << " nfr02_impulses_emitted=" << impulses_emitted
              << " nfr02_capture_a_frames=" << capture_a_frames
              << " nfr02_capture_a_dropped_frames=" << capture_a_dropped
              << " nfr02_capture_b_frames=" << capture_b_frames
              << " nfr02_capture_b_dropped_frames=" << capture_b_dropped << '\n';
    std::cout << "nfr02_groups_a=" << groups_a.size() << " nfr02_groups_b=" << groups_b.size() << '\n';

    render_service->Release();
    capture_a_service->Release();
    capture_b_service->Release();

    const UINT64 target_pairs = std::min<UINT64>(impulse_count, impulses_emitted);
    const UINT64 pairs = std::min<UINT64>({groups_a.size(), groups_b.size(), static_cast<size_t>(target_pairs)});
    if (target_pairs == 0 || pairs < (target_pairs * 9 / 10)) {
        std::cout << "nfr02_insufficient_pairs=1 pairs=" << pairs << " target_pairs=" << target_pairs << '\n';
        render_state.release(); capture_a_state.release(); capture_b_state.release();
        return 1;
    }

    std::vector<double> latencies_ms;
    latencies_ms.reserve(static_cast<size_t>(pairs));
    for (UINT64 k = 0; k < pairs; ++k) {
        latencies_ms.push_back(
            (groups_b[static_cast<size_t>(k)].qpc_100ns - groups_a[static_cast<size_t>(k)].qpc_100ns) / 10000.0);
    }
    std::sort(latencies_ms.begin(), latencies_ms.end());
    auto percentile = [&](double p) {
        const size_t idx = static_cast<size_t>(std::min<double>(
            static_cast<double>(latencies_ms.size() - 1), std::floor(p * latencies_ms.size())));
        return latencies_ms[idx];
    };
    const double mean = std::accumulate(latencies_ms.begin(), latencies_ms.end(), 0.0) /
                        static_cast<double>(latencies_ms.size());
    std::cout << "nfr02_pairs=" << pairs
              << " nfr02_latency_min_ms=" << latencies_ms.front()
              << " nfr02_latency_p50_ms=" << percentile(0.50)
              << " nfr02_latency_p95_ms=" << percentile(0.95)
              << " nfr02_latency_max_ms=" << latencies_ms.back()
              << " nfr02_latency_mean_ms=" << mean << '\n';
    std::cout << "Scope: mic-capture-to-virtual-capture round trip through whatever AudioRouter "
                 "route was live during this run, timestamped via each capture stream's own "
                 "per-packet QPC timestamp on a shared performance-counter timeline; requires "
                 "the caller to have started that route separately. Excludes Discord/OBS-side "
                 "network or codec delay.\n";

    render_state.release();
    capture_a_state.release();
    capture_b_state.release();
    return 0;
}

// Diagnostic only: samples IAudioClock::GetPosition repeatedly in the first
// ~300ms after Start() to see directly whether the render clock's position
// advances linearly from t=0, or stays near zero for an initial "ramp-up"
// window before locking into steady playback. Used to investigate whether
// an anchor sampled immediately after Start() is biased relative to one
// sampled after the stream has been running for a while.
static int render_clock_ramp_probe(UINT render_index, bool low_latency) {
    IMMDeviceEnumerator* enumerator = nullptr;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
    if (FAILED(hr)) { print_hr("ramp_enumerator", hr); return 1; }
    LoopbackStreamState state;
    bool ok = activate_shared_stream(enumerator, eRender, render_index, state, "ramp_render", true, 0,
                                     low_latency);
    enumerator->Release();
    if (!ok) { state.release(); return 1; }

    IAudioRenderClient* render_service = nullptr;
    hr = state.client->GetService(__uuidof(IAudioRenderClient), reinterpret_cast<void**>(&render_service));
    print_hr("ramp_get_service", hr);
    if (FAILED(hr)) { state.release(); return 1; }
    UINT32 buffer_size = 0;
    state.client->GetBufferSize(&buffer_size);

    const auto start_time = std::chrono::steady_clock::now();
    hr = state.client->Start();
    print_hr("ramp_start", hr);
    if (FAILED(hr)) { render_service->Release(); state.release(); return 1; }

    for (int sample = 0; sample < 40; ++sample) {
        WaitForSingleObject(state.ready_event, 20);
        UINT32 padding = 0;
        if (SUCCEEDED(state.client->GetCurrentPadding(&padding))) {
            const UINT32 available = buffer_size - padding;
            if (available != 0) {
                BYTE* data = nullptr;
                if (SUCCEEDED(render_service->GetBuffer(available, &data))) {
                    std::memset(data, 0, static_cast<size_t>(available) * state.format->nBlockAlign);
                    render_service->ReleaseBuffer(available, 0);
                }
            }
        }
        UINT64 position = 0, qpc_100ns = 0;
        const HRESULT position_hr = state.clock->GetPosition(&position, &qpc_100ns);
        const double elapsed_ms = std::chrono::duration<double, std::milli>(
            std::chrono::steady_clock::now() - start_time).count();
        const double implied_seconds = static_cast<double>(position) / static_cast<double>(state.clock_frequency);
        std::cout << std::fixed << std::setprecision(3)
                  << "ramp_sample=" << sample << " elapsed_ms=" << elapsed_ms
                  << " position=" << position << " implied_elapsed_s=" << implied_seconds
                  << " position_hr=0x" << std::hex << static_cast<unsigned long>(position_hr) << std::dec
                  << '\n';
    }
    std::cout.unsetf(std::ios::floatfield);
    std::cout.precision(6);

    state.client->Stop();
    state.client->Reset();
    render_service->Release();
    state.release();
    return 0;
}

static int render_session_inventory(UINT target_index) {
    IMMDeviceEnumerator* enumerator = nullptr;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
    if (FAILED(hr)) { print_hr("ownership_enumerator", hr); return 1; }
    IMMDeviceCollection* devices = nullptr;
    hr = enumerator->EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, &devices);
    if (FAILED(hr)) { print_hr("ownership_enum", hr); enumerator->Release(); return 1; }
    UINT count = 0;
    devices->GetCount(&count);
    if (target_index >= count) {
        std::cout << "ownership_index_out_of_range=" << target_index << " count=" << count << '\n';
        devices->Release(); enumerator->Release(); return 1;
    }
    IMMDevice* device = nullptr;
    hr = devices->Item(target_index, &device);
    print_hr("ownership_item", hr);
    IAudioSessionManager2* manager = nullptr;
    if (SUCCEEDED(hr)) {
        hr = device->Activate(__uuidof(IAudioSessionManager2), CLSCTX_ALL, nullptr,
                              reinterpret_cast<void**>(&manager));
        print_hr("ownership_activate_session_manager", hr);
    }
    IAudioSessionEnumerator* sessions = nullptr;
    int session_count = 0;
    if (SUCCEEDED(hr)) {
        hr = manager->GetSessionEnumerator(&sessions);
        print_hr("ownership_get_session_enumerator", hr);
    }
    if (SUCCEEDED(hr)) {
        hr = sessions->GetCount(&session_count);
        print_hr("ownership_get_session_count", hr);
    }
    if (SUCCEEDED(hr)) {
        std::cout << "ownership_session_count=" << session_count << '\n';
        for (int index = 0; index < session_count; ++index) {
            IAudioSessionControl* control = nullptr;
            HRESULT item_hr = sessions->GetSession(index, &control);
            if (FAILED(item_hr)) { print_hr("ownership_get_session", item_hr); continue; }
            IAudioSessionControl2* control2 = nullptr;
            HRESULT query_hr = control->QueryInterface(__uuidof(IAudioSessionControl2),
                                                        reinterpret_cast<void**>(&control2));
            DWORD process_id = 0;
            AudioSessionState state = AudioSessionStateExpired;
            LPWSTR display_name = nullptr;
            if (SUCCEEDED(query_hr)) {
                control2->GetProcessId(&process_id);
                control2->GetState(&state);
                control2->GetDisplayName(&display_name);
            }
            std::wcout << L"ownership_session index=" << index
                       << L" process_id=" << process_id
                       << L" state=" << static_cast<int>(state)
                       << L" image_path=" << process_image_path(process_id)
                       << L" display_name=" << (display_name ? display_name : L"<none>") << L'\n';
            CoTaskMemFree(display_name);
            if (control2) control2->Release();
            control->Release();
        }
    }
    if (sessions) sessions->Release();
    if (manager) manager->Release();
    if (device) device->Release();
    devices->Release();
    enumerator->Release();
    return SUCCEEDED(hr) ? 0 : 1;
}

static int controlled_process_attribution(DWORD duration_ms, bool include_target_tree) {
    char executable[MAX_PATH]{};
    if (GetModuleFileNameA(nullptr, executable, MAX_PATH) == 0) {
        print_hr("attribution_get_executable", HRESULT_FROM_WIN32(GetLastError()));
        return 1;
    }
    char command_line[2048]{};
    sprintf_s(command_line, "\"%s\" tone %lu", executable,
              static_cast<unsigned long>(duration_ms + 1000));
    STARTUPINFOA startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessA(nullptr, command_line, nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) {
        print_hr("attribution_create_process", HRESULT_FROM_WIN32(GetLastError()));
        return 1;
    }
    CloseHandle(process.hThread);
    const int result = process_loopback_probe(process.dwProcessId, true, include_target_tree,
                                              duration_ms, true);
    WaitForSingleObject(process.hProcess, duration_ms + 3000);
    DWORD exit_code = STILL_ACTIVE;
    GetExitCodeProcess(process.hProcess, &exit_code);
    if (exit_code == STILL_ACTIVE) {
        TerminateProcess(process.hProcess, 1);
        WaitForSingleObject(process.hProcess, 1000);
    }
    CloseHandle(process.hProcess);
    std::cout << "attribution_mode=" << (include_target_tree ? "include" : "exclude") << '\n';
    std::cout << "attribution_child_exit=" << exit_code << '\n';
    return result == 0 && exit_code == 0 ? 0 : 1;
}

static int event_endpoint_initialize_probe(EDataFlow flow, UINT target_index,
                                           const char* prefix) {
    auto label = [prefix](const char* suffix) {
        return std::string(prefix) + suffix;
    };
    IMMDeviceEnumerator* enumerator = nullptr;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator),
                                  reinterpret_cast<void**>(&enumerator));
    print_hr(label("_co_create_enumerator").c_str(), hr);
    if (FAILED(hr)) return 1;

    IMMDeviceCollection* devices = nullptr;
    hr = enumerator->EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE, &devices);
    print_hr(label("_enum").c_str(), hr);
    if (FAILED(hr)) {
        enumerator->Release();
        return 1;
    }
    UINT count = 0;
    devices->GetCount(&count);
    if (target_index >= count) {
        std::cout << prefix << "_index_out_of_range=" << target_index
                  << " count=" << count << '\n';
        devices->Release();
        enumerator->Release();
        return 1;
    }

    IMMDevice* device = nullptr;
    hr = devices->Item(target_index, &device);
    print_hr(label("_item").c_str(), hr);
    if (FAILED(hr)) {
        devices->Release();
        enumerator->Release();
        return 1;
    }
    IAudioClient* client = nullptr;
    hr = device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr,
                          reinterpret_cast<void**>(&client));
    print_hr(label("_activate").c_str(), hr);
    WAVEFORMATEX* format = nullptr;
    if (SUCCEEDED(hr)) hr = client->GetMixFormat(&format);
    print_hr(label("_get_mix_format").c_str(), hr);
    HANDLE ready_event = nullptr;
    if (SUCCEEDED(hr)) {
        ready_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!ready_event) {
            hr = HRESULT_FROM_WIN32(GetLastError());
            print_hr(label("_create_event").c_str(), hr);
        }
    }
    if (SUCCEEDED(hr)) {
        // Event-driven shared mode requires zero hnsBufferDuration. The
        // endpoint-owned mix format is passed unchanged and no conversion is
        // requested, matching the Rust adapter's production boundary.
        hr = client->Initialize(AUDCLNT_SHAREMODE_SHARED,
                                AUDCLNT_STREAMFLAGS_EVENTCALLBACK |
                                    AUDCLNT_STREAMFLAGS_NOPERSIST,
                                0, 0, format, nullptr);
        print_hr(label("_initialize_exact_zero").c_str(), hr);
    }
    if (SUCCEEDED(hr)) print_hr(label("_set_event").c_str(), client->SetEventHandle(ready_event));
    if (format) CoTaskMemFree(format);
    if (ready_event) CloseHandle(ready_event);
    if (client) client->Release();
    device->Release();
    devices->Release();
    enumerator->Release();
    return SUCCEEDED(hr) ? 0 : 1;
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
    if (argc > 1 && std::strcmp(argv[1], "event-capture") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        DWORD duration_ms = argc > 3 ? static_cast<DWORD>(std::strtoul(argv[3], nullptr, 10)) : 500;
        int result = capture_data_probe(target_index, duration_ms, nullptr, true);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "capture-file") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        DWORD duration_ms = argc > 3 ? static_cast<DWORD>(std::strtoul(argv[3], nullptr, 10)) : 200;
        const char* output_path = argc > 4 ? argv[4] : nullptr;
        int result = output_path ? capture_data_probe(target_index, duration_ms, output_path) : 1;
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "render") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        DWORD duration_ms = argc > 3 ? static_cast<DWORD>(std::strtoul(argv[3], nullptr, 10)) : 200;
        int result = render_data_probe(target_index, duration_ms, false);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "event-render") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        DWORD duration_ms = argc > 3 ? static_cast<DWORD>(std::strtoul(argv[3], nullptr, 10)) : 500;
        int result = render_data_probe(target_index, duration_ms, false, false, true);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "render-ownership") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        int result = render_session_inventory(target_index);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "event-capture-init") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        int result = event_endpoint_initialize_probe(eCapture, target_index, "event_capture");
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "event-render-init") == 0) {
        UINT target_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        int result = event_endpoint_initialize_probe(eRender, target_index, "event_render");
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "inventory") == 0) {
        int render_result = endpoint_inventory(eRender);
        int capture_result = endpoint_inventory(eCapture);
        CoUninitialize();
        return render_result != 0 ? render_result : capture_result;
    }
    if (argc > 1 && std::strcmp(argv[1], "inventory-formats") == 0) {
        int render_result = endpoint_inventory(eRender, true);
        int capture_result = endpoint_inventory(eCapture, true);
        CoUninitialize();
        return render_result != 0 ? render_result : capture_result;
    }
    if (argc > 1 && std::strcmp(argv[1], "tone") == 0) {
        DWORD duration_ms = argc > 2 ? static_cast<DWORD>(std::strtoul(argv[2], nullptr, 10)) : 1500;
        UINT target_index = argc > 3 ? static_cast<UINT>(std::strtoul(argv[3], nullptr, 10)) : 0;
        int result = render_data_probe(target_index, duration_ms, true);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "impulse") == 0) {
        DWORD duration_ms = argc > 2 ? static_cast<DWORD>(std::strtoul(argv[2], nullptr, 10)) : 1000;
        UINT target_index = argc > 3 ? static_cast<UINT>(std::strtoul(argv[3], nullptr, 10)) : 0;
        int result = render_data_probe(target_index, duration_ms, false, true);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "capture-loopback") == 0) {
        DWORD impulse_count = argc > 2 ? static_cast<DWORD>(std::strtoul(argv[2], nullptr, 10)) : 1000;
        UINT render_index = argc > 3 ? static_cast<UINT>(std::strtoul(argv[3], nullptr, 10)) : 0;
        UINT capture_a_index = argc > 4 ? static_cast<UINT>(std::strtoul(argv[4], nullptr, 10)) : 0;
        UINT capture_b_index = argc > 5 ? static_cast<UINT>(std::strtoul(argv[5], nullptr, 10)) : 0;
        int result = capture_loopback_probe(render_index, capture_a_index, capture_b_index, impulse_count);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "render-clock-ramp") == 0) {
        UINT render_index = argc > 2 ? static_cast<UINT>(std::strtoul(argv[2], nullptr, 10)) : 0;
        bool low_latency = argc > 3 && std::strcmp(argv[3], "low-latency") == 0;
        int result = render_clock_ramp_probe(render_index, low_latency);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "impulse-loopback") == 0) {
        DWORD impulse_count = argc > 2 ? static_cast<DWORD>(std::strtoul(argv[2], nullptr, 10)) : 1000;
        UINT render_index = argc > 3 ? static_cast<UINT>(std::strtoul(argv[3], nullptr, 10)) : 0;
        UINT capture_index = argc > 4 ? static_cast<UINT>(std::strtoul(argv[4], nullptr, 10)) : 0;
        REFERENCE_TIME buffer_100ns = argc > 5 ? static_cast<REFERENCE_TIME>(std::strtoull(argv[5], nullptr, 10)) : 0;
        bool low_latency = argc > 6 && std::strcmp(argv[6], "low-latency") == 0;
        int result = impulse_loopback_probe(render_index, capture_index, impulse_count, buffer_100ns, low_latency);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "process-attribution") == 0) {
        DWORD duration_ms = argc > 2 ? static_cast<DWORD>(std::strtoul(argv[2], nullptr, 10)) : 1000;
        int result = controlled_process_attribution(duration_ms, true);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && std::strcmp(argv[1], "process-attribution-exclude") == 0) {
        DWORD duration_ms = argc > 2 ? static_cast<DWORD>(std::strtoul(argv[2], nullptr, 10)) : 1000;
        int result = controlled_process_attribution(duration_ms, false);
        CoUninitialize();
        return result;
    }
    if (argc > 1 && (std::strcmp(argv[1], "process") == 0 ||
                     std::strcmp(argv[1], "process-capture") == 0 ||
                     std::strcmp(argv[1], "process-capture-exclude") == 0)) {
        DWORD target_process_id = GetCurrentProcessId();
        if (argc > 2) target_process_id = static_cast<DWORD>(std::strtoul(argv[2], nullptr, 10));
        bool read_data = std::strcmp(argv[1], "process-capture") == 0 ||
                         std::strcmp(argv[1], "process-capture-exclude") == 0;
        bool include_target_tree = std::strcmp(argv[1], "process-capture-exclude") != 0;
        DWORD duration_ms = argc > 3 ? static_cast<DWORD>(std::strtoul(argv[3], nullptr, 10)) : 500;
        int result = process_loopback_probe(target_process_id, read_data, include_target_tree,
                                            duration_ms, false);
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
