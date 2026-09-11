# M03 AudioRouter virtual-driver prototype evidence

Date: 2026-09-10
Environment: Windows x64, Visual Studio 18.9.1 (Community 2026), WDK
10.0.28000.0, PowerShell, repository `main`
Command:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\drivers\audiorouter-virtual\build.ps1 -Configuration Release -Platform x64 -KeepOutput
```

## Result

The AudioRouter-owned derivative built successfully through Utilities, Filters,
Main, Package, and Inc. The WDK signability/catalog step reported zero errors
and zero warnings and produced:

- `drivers/audiorouter-virtual/Source/Main/x64/Release/AudioRouterVirtual.sys`
- `drivers/audiorouter-virtual/Source/Main/x64/Release/AudioRouterVirtual.inf`
- `drivers/audiorouter-virtual/x64/Release/package/audioroutervirtual.cat`

The build passed `SignMode=Off`, `SkipPackageVerification=true`, and
`ApiValidator_Enable=false`. The WDK therefore did not test-sign the binary or
claim Universal-driver API qualification. Catalog generation is not production
signing evidence.

## Scope and remaining gates

This is source/build evidence for M03 VDEV-01..08 and SEC-08 only. The source
currently exposes the derivative of the upstream sample's one render and one
capture endpoint families. It does not yet implement AudioRouter bus count,
ownership lease, broker/bridge IPC, graph-to-driver callback transport, managed
create/rename/enable/disable/delete lifecycle, uninstall/restore, production
signing, or clean-machine installation. VDEV-09 and M08 signing/release gates
remain open.

No INF installation, service start, device registration, boot-policy change,
test-signing change, audio-default change, stream open, or persistent machine
configuration change occurred. The generated build directories are ignored and
disposable.

## Failed attempts and fixes

The first build attempt failed because forcing `OutDir` and `IntDir` into one
temporary directory invalidated the sample projects' relative library paths;
it also hit a WDK `InfVerif.dll` x86-load exception. The build script now keeps
project-relative outputs, uses a separate log directory, disables the known
local verifier/API-validation hooks for this build-only qualification, and
leaves installation/signing explicit future gates.
