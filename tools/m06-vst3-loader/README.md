# M06 VST3 loader probe

This native probe is a non-audio loading gate for the pinned local VST3 SDK.
It loads one explicit x64 VST3 bundle, calls its GetPluginFactory export,
enumerates factory class metadata, instantiates and initializes the first audio
component to inspect its bus counts, then terminates/releases it and unloads
the module. It also processes one bounded offline stereo block, checks that
the output is finite, initializes the associated controller, and exercises
normalized parameter set/readback while restoring the original values. It does
bounded controller descriptor discovery (parameter ID, ASCII-safe title,
normalized default, step count, and flags) and rejects catalogs larger than
the worker contract's 256-entry limit. It does
an in-memory component state save/restore, and does not create an editor, open
an audio device, or alter machine configuration. A valid effect with no
automatable parameters is accepted; automation checks run when parameters are
exposed.

Build and run from the repository root:

    .\tools\m06-vst3-loader\build.ps1
    .\tools\m06-vst3-loader\m06-vst3-loader.exe .\third_party\vst3sdk-build\VST3\Release\mda-vst3.vst3
    .\tools\m06-vst3-loader\m06-vst3-loader.exe .\third_party\vst3sdk-build\VST3\Release\mda-vst3.vst3 --class-index 4
    Remove-Item .\tools\m06-vst3-loader\m06-vst3-loader.exe, .\tools\m06-vst3-loader\m06-vst3-loader.obj

The executable and object file are intentionally ignored/generated and must
not be committed.

The optional class index makes plugin-specific compatibility results
reproducible. The class list printed by the probe identifies the selected
audio-effect index; a controller or missing index fails closed. The optional
`--multi-bus` flag enables the bounded offline probe for effects exposing up to
four mono/stereo audio buses per direction. Without that flag, non-single-bus
effects are rejected. The probe supplies every declared bus to `process`,
checks every output for finite samples, and does not imply worker or realtime
multi-bus scheduling.
