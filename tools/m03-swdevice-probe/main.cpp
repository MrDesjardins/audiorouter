#include <windows.h>
#include <swdevice.h>

#include <algorithm>
#include <iostream>
#include <string>

#pragma comment(lib, "Cfgmgr32.lib")

namespace {

struct CreateContext {
    HANDLE complete = nullptr;
    HRESULT result = E_FAIL;
    std::wstring instance_id;
};

void CALLBACK OnCreated(
    HSWDEVICE,
    HRESULT result,
    PVOID context,
    PCWSTR device_instance_id) noexcept
{
    auto* state = static_cast<CreateContext*>(context);
    state->result = result;
    if (device_instance_id != nullptr) {
        state->instance_id = device_instance_id;
    }
    SetEvent(state->complete);
}

bool ValidInstanceId(const std::wstring& instance) {
    if (instance.empty() || instance.size() > 64) {
        return false;
    }
    return std::all_of(instance.begin(), instance.end(), [](wchar_t value) {
        return value != L'\\' && value != L'/' && value != L'\0' &&
            value >= 0x20;
    });
}

void Usage() {
    std::wcout
        << L"AudioRouter software-device probe\n"
        << L"  (default)                         print a no-side-effect plan\n"
        << L"  --create --allow-device-create   create one temporary SWD device\n"
        << L"  --instance <id>                  stable bounded instance ID\n"
        << L"  --hold-ms <0..30000>             retain the temporary handle\n";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    bool create = false;
    bool allow = false;
    std::wstring instance = L"dry-run-bus";
    DWORD hold_ms = 0;

    for (int index = 1; index < argc; ++index) {
        const std::wstring argument = argv[index];
        if (argument == L"--create") {
            create = true;
        } else if (argument == L"--allow-device-create") {
            allow = true;
        } else if (argument == L"--instance" && index + 1 < argc) {
            instance = argv[++index];
        } else if (argument == L"--hold-ms" && index + 1 < argc) {
            try {
                const unsigned long parsed = std::stoul(argv[++index]);
                if (parsed > 30000) {
                    std::wcerr << L"--hold-ms exceeds the 30000 ms bound\n";
                    return 2;
                }
                hold_ms = static_cast<DWORD>(parsed);
            } catch (...) {
                std::wcerr << L"--hold-ms must be an integer\n";
                return 2;
            }
        } else {
            Usage();
            return 2;
        }
    }

    if (!ValidInstanceId(instance)) {
        std::wcerr << L"instance ID must contain 1..64 printable characters and no slash\n";
        return 2;
    }
    if (!create) {
        std::wcout << L"dry-run: enumerator=AudioRouter parent=HTREE\\ROOT\\0 "
                       L"hardwareId=SWD\\AudioRouterVirtual instance="
                    << instance << L"\n";
        return 0;
    }
    if (!allow) {
        std::wcerr << L"device creation requires --allow-device-create\n";
        return 2;
    }

    // A double-null-terminated hardware-ID list is required by the Software
    // Device API. The handle is intentionally retained only for this bounded
    // probe; production persistence belongs to the control-plane owner.
    const wchar_t hardware_ids[] = L"SWD\\AudioRouterVirtual\0\0";
    SW_DEVICE_CREATE_INFO info = {};
    info.cbSize = sizeof(info);
    info.pszInstanceId = instance.c_str();
    info.pszzHardwareIds = hardware_ids;
    info.CapabilityFlags = SWDeviceCapabilitiesDriverRequired;
    info.pszDeviceDescription = L"AudioRouter virtual bus";

    CreateContext context;
    context.complete = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (context.complete == nullptr) {
        std::wcerr << L"CreateEvent failed: " << GetLastError() << L"\n";
        return 1;
    }
    HSWDEVICE device = nullptr;
    const HRESULT create_result = SwDeviceCreate(
        L"AudioRouter", L"HTREE\\ROOT\\0", &info, 0, nullptr,
        OnCreated, &context, &device);
    if (FAILED(create_result)) {
        std::wcerr << L"SwDeviceCreate failed: 0x" << std::hex
                   << static_cast<unsigned long>(create_result) << L"\n";
        CloseHandle(context.complete);
        return 1;
    }
    const DWORD wait_result = WaitForSingleObject(context.complete, 5000);
    if (wait_result != WAIT_OBJECT_0 || FAILED(context.result)) {
        std::wcerr << L"software-device callback failed or timed out\n";
        if (device != nullptr) {
            SwDeviceClose(device);
        }
        CloseHandle(context.complete);
        return 1;
    }
    std::wcout << L"created instance=" << context.instance_id << L"\n";
    if (hold_ms != 0) {
        Sleep(hold_ms);
    }
    // Default handle lifetime removes the temporary device on close. This
    // probe therefore cannot leave an unmanaged device behind.
    SwDeviceClose(device);
    CloseHandle(context.complete);
    std::wcout << L"temporary device closed\n";
    return 0;
}
