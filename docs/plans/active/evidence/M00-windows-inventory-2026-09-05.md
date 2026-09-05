# M00 Windows inventory — 2026-09-05

## Scope

Read-only native Windows inventory for M00 tasks 1, 5, and 6. No defaults, drivers, audio streams, or user files were changed. This is machine inventory, not capture/render or latency evidence.

## Machine and OS

| Field | Result |
| --- | --- |
| Host | `PATRICK5080` |
| OS | Microsoft Windows 11 Home |
| Version/build | `10.0.26200`, build `26200` |
| Architecture | 64-bit |
| Manufacturer/model | CyberPowerPC / GamingPC |
| System type | x64-based PC |
| Memory | `33,579,425,792` bytes reported (~31.3 GiB) |
| PowerShell | Windows PowerShell `5.1.26100.9168` |

## Portable/native toolchain visible

| Tool | Result |
| --- | --- |
| Rust | `1.96.0 (ac68faa20 2026-05-25)` |
| Cargo | `1.96.0 (30a34c682 2026-05-25)` |
| Node | `v22.22.3` |
| npm | `10.9.8` |
| MSVC `cl.exe` | Not found on PATH |
| MSBuild | Not found on PATH |
| CMake | Not found on PATH |
| .NET SDK | `dotnet --version` did not load an SDK; no usable SDK reported |
| Windows SDK | `10.0.26100.0` and older SDK directories present under `C:\Program Files (x86)\Windows Kits\10` |
| WDK | Not established; no WDK-specific installation or build environment identified |
| `signtool.exe` | Present in Windows SDK at `bin\10.0.26100.0\x64\signtool.exe`, but not on PATH |

Visual Studio presence was not established. A developer command prompt or installed-component query is still required before a driver or C++ probe can be built. The SDK supplies supporting tools such as `midl.exe`, `rc.exe`, and `signtool.exe`, but their presence does not establish a usable MSVC/WDK build environment.

## Audio hardware and existing virtual devices

The following Windows media devices were reported as `OK`:

- Focusrite USB Audio
- USB Digital Audio (generic USB audio)
- PD200X Podcast Microphone
- NVIDIA High Definition Audio
- Realtek High Definition Audio
- NVIDIA Virtual Audio Device (Wave Extensible) (WDM)
- VB-Audio Voicemeeter VAIO
- VB-Audio Virtual Cable
- SteelSeries Sonar Virtual Audio Device

The following media devices were reported as `Unknown` and require a separate availability check:

- ATEM Mini
- Blackmagic Design
- Microsoft Streaming Quality Manager Proxy
- Microsoft Streaming Clock Proxy

Enumerated endpoint examples include Focusrite `Speakers` and `Analogue 1 + 2`, PD200X `Microphone`/`Speakers`, VB-Audio `CABLE Input`/`CABLE Output`, Voicemeeter inputs/outputs, and SteelSeries Sonar endpoints. Endpoint instance IDs were captured in the terminal output for this run; they are intentionally not treated as AudioRouter-stable identities.

## Command and access record

The inventory used elevated read-only PowerShell commands:

```powershell
Get-CimInstance Win32_OperatingSystem
Get-CimInstance Win32_ComputerSystem
Get-CimInstance Win32_SoundDevice
Get-PnpDevice -Class AudioEndpoint
Get-PnpDevice -Class Media
Get-Command rustc.exe,cargo.exe,node.exe,npm.cmd,cl.exe,msbuild.exe,cmake.exe,dotnet.exe,signtool.exe
```

The same WMI/PnP queries returned `Access denied` (`0x80041003`) in the restricted non-elevated shell. The successful result above required the approved elevated read-only invocation. `npm.ps1` was blocked by execution policy; `npm.cmd` was used instead.

## M00 conclusion

This machine is a usable Windows inventory/reference candidate, and it has both physical USB audio hardware and interim third-party virtual audio devices. It does not provide evidence for CAP-03/04/05/06/07/08, NFR-01/02/03, or VDEV-02/09: no WASAPI probe, process-loopback probe, controlled tone/impulse harness, physical loopback measurement, driver prototype, signing route, or exact format/period manifest has been run. The existing virtual devices are interim-only under the specification and cannot satisfy the managed-driver gate.
