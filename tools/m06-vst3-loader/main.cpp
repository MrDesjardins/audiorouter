#define NOMINMAX

#include <windows.h>

#include <filesystem>
#include <iostream>
#include <stdexcept>

#include "pluginterfaces/base/ipluginbase.h"

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
        factory->release();
        factory = nullptr;
        FreeLibrary(module);
        return 0;
    } catch (const std::exception& error) {
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
