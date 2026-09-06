# M06 VST3 loader probe

This native probe is a non-audio loading gate for the pinned local VST3 SDK.
It loads one explicit x64 VST3 bundle, calls its GetPluginFactory export,
enumerates factory class metadata, instantiates and initializes the first audio
component to inspect its bus counts, then terminates/releases it and unloads
the module. It does not process samples, create an editor, open an audio
device, or alter machine configuration.

Build and run from the repository root:

    .\tools\m06-vst3-loader\build.ps1
    .\tools\m06-vst3-loader\m06-vst3-loader.exe .\third_party\vst3sdk-build\VST3\Release\mda-vst3.vst3
    Remove-Item .\tools\m06-vst3-loader\m06-vst3-loader.exe, .\tools\m06-vst3-loader\m06-vst3-loader.obj

The executable and object file are intentionally ignored/generated and must
not be committed.
