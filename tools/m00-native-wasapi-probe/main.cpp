#define NOMINMAX
#include <windows.h>
#include <audioclient.h>
#include <mmdeviceapi.h>
#include <functiondiscoverykeys_devpkey.h>
#include <iostream>
#include <iomanip>

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

int main() {
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr)) { print_hr("CoInitializeEx", hr); return 1; }

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
