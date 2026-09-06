# SDK and native toolchain setup

AudioRouter uses two different SDK categories:

## Steinberg VST3 SDK

The VST3 SDK is source-distributed; the GitHub repository is the SDK itself,
not an installer. AudioRouter keeps the pinned checkout in the ignored local
directory `third_party/vst3sdk` so it is not committed into this repository.

The expected checkout is:

```text
third_party/vst3sdk
revision: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
```

To download or repair the checkout, run the repository-local setup script from
PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\m06-vst3-sdk\install.ps1
```

The script is idempotent, initializes the SDK's submodules, and verifies the
pinned revision and required headers. It refuses to replace a different
checkout unless `-Force` is supplied. The SDK is source-distributed, so this
is the supported project-local installation boundary; it does not register a
system-wide SDK or install anything into Windows.

It includes the SDK submodules and is built with the project-local portable
CMake cache under `third_party/vst3sdk-build`. If CMake is not on PATH, use the
ignored `third_party/cmake-4.4.0/bin/cmake.exe` cache already provisioned for
the repository. The official validator and the sample `mda-vst3` bundle have
been run successfully. AudioRouter supports the
VST3 x64 boundary; VST2 and x86 plugins are not part of this setup.

## Windows SDK and WDK

The native WASAPI and future driver work use the Windows SDK and WDK installed
through Visual Studio/Build Tools. The verified host installation provides
Windows SDK `10.0.28000.0` and WDK `10.1.28000.2526`, including MSVC, kernel
headers/libraries, and `signtool.exe`.

These tools are build dependencies only. Setup and validation do not install a
driver, register a plugin, change default devices, or modify volume, mute,
privacy, or other persistent audio settings.

## Verification

Read-only verification of the local VST3 checkout:

```powershell
Test-Path third_party\vst3sdk\CMakeLists.txt
git -C third_party\vst3sdk rev-parse HEAD
```

Native build scripts locate Visual Studio and the Windows SDK without requiring
global PATH changes. The VST3 loader accepts optional `-SdkInclude` and
`-Output` overrides, while default discovery selects the newest installed
MSVC toolset and Windows SDK containing the required headers. See
`tools/m00-native-wasapi-probe/build.ps1` for the native probe and
`docs/plans/active/evidence/M06-vst3-sdk.md` for measured SDK validation
evidence.

For a repeatable local acceptance run, use:

```powershell
powershell -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst3-sdk.ps1
```

The script verifies the pinned revision, builds the local SDK sample when
needed, runs the official validator and offline loader, and removes generated
loader outputs. `-SkipBuild` is available when the existing Release build is
already known to be current.
