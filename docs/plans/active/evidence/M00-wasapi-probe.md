# M00 WASAPI probe

## Status

The read-only endpoint inventory probe has been added at [`tools/m00-wasapi-probe`](../../../tools/m00-wasapi-probe). It uses Rust `windows` bindings and does not modify Windows defaults, start audio streams, install drivers, or write outside stdout.

## Current result

`cargo check --manifest-path tools/m00-wasapi-probe/Cargo.toml` and `cargo build --manifest-path tools/m00-wasapi-probe/Cargo.toml` pass with Rust `1.96.0` targeting `x86_64-pc-windows-msvc`; the Rust toolchain supplied a working linker path despite no `link.exe` or Visual Studio installation being on PATH.

`cargo run --manifest-path tools/m00-wasapi-probe/Cargo.toml --quiet` completed successfully on `PATRICK5080` and reported `active_endpoint_count=34`. Every enumerated endpoint returned state `0x00000001` (`DEVICE_STATE_ACTIVE`), a Windows endpoint ID, a mix format, and default/minimum device periods.

Observed capability summary: most endpoints reported 48,000 Hz, two channels, 32-bit samples, and 100,000/20,000 100-ns default/minimum periods (10 ms/2 ms). Sonar endpoints reported 96,000 Hz and eight channels. The Focusrite render endpoint reported a 30,000 100-ns minimum period (3 ms); other endpoints reported 2–3 ms minimum periods. One USB capture endpoint reported one channel at 96,000 Hz with format tag 3. These are current-device mix-format observations, not proof that every endpoint accepts every requested shared-mode format.

The probe calls `IAudioClient::IsFormatSupported` in shared mode for 44.1/48 kHz mono/stereo IEEE-float formats. This capability query does not start a stream or alter Windows configuration. On this run, all 34 endpoints returned `S_FALSE` (closest-match available) for 44.1 kHz mono and stereo; 48 kHz mono was exact (`S_OK`) on 1/34 and closest-match on 33/34; 48 kHz stereo was exact on 29/34 and closest-match on 5/34. No other HRESULTs occurred. `S_FALSE` is not a failure; the returned closest format must be inspected when implementing negotiation.

No configuration restoration was required: the probe only activated endpoint/client COM objects, queried metadata, and released them. It did not initialize an audio stream, set a format, change a default, mute, or alter a device.

## Shared-mode initialization result

The probe was extended to call `IAudioClient::Initialize` with `AUDCLNT_SHAREMODE_SHARED`, the endpoint's current mix format, `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST`, and zero duration/period parameters. On the direct run, 20 of 34 endpoints initialized successfully, reported buffer sizes from 1,056 to 2,112 frames, were reset, and were released. Thirteen returned `0x80070057` (`E_INVALIDARG`) and one returned `0x8889000a` (`AUDCLNT_E_EXCLUSIVE_MODE_ONLY`). No client was started; `GetStreamLatency` therefore returned zero and is not a latency measurement.

The initialization test was non-persistent and did not modify existing configuration. It did not change default endpoints, volume, mute, exclusive-mode settings, or device properties. All activated clients were reset/released before the process exited; no restoration action was required.

The probe then performed a render-only start/stop smoke test. Of 21 render endpoints, 20 initialized, started with no submitted audio, stopped, reset, and released successfully. One render endpoint failed initialization with `AUDCLNT_E_EXCLUSIVE_MODE_ONLY`. The 13 capture endpoints were never started, so this run captured no microphone or desktop audio; their initialization failures remained `E_INVALIDARG` as above.

To isolate the capture failure, the probe was rerun with direction-specific initialization: render endpoints used the non-persistent auto-conversion path, while capture endpoints used no stream flags and requested 100 ms, zero-duration, and finally the 20 ms buffer used by Microsoft’s shared capture sample. Capture initialization still returned `E_INVALIDARG` on all 13 endpoints for every duration. Microsoft documents that shared-mode initialization should accept the same endpoint mix format returned by `GetMixFormat`; therefore this is a reproducible probe limitation/result requiring endpoint-specific format validation and capture-client diagnostics, not evidence that Windows capture is unsupported. See the [Microsoft GetMixFormat documentation](https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-getmixformat) and [shared capture sample](https://github.com/microsoft/Windows-classic-samples/blob/main/Samples/Win7Samples/multimedia/audio/CaptureSharedTimerDriven/WASAPICapture.cpp).

The current fresh-client/no-flag/minimum-period run also records a representative capture mix descriptor: format tag `65534` (`WAVE_FORMAT_EXTENSIBLE`), 48,000 Hz, two channels, 32 bits/container, block align 8, average 384,000 bytes/sec, and `cbSize=22`. These fields are structurally consistent with an extensible format. The failure remains `E_INVALIDARG`, not `AUDCLNT_E_DEVICE_IN_USE`; shared mode is intended to allow other user-mode clients. The next diagnostic is to preserve and validate the full extensible channel mask/subformat and compare a minimal native capture client against this binding.

The latest event-only retry uses `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`, the endpoint-reported minimum period, a fresh client-owned mix format, and a private unnamed event handle. It also returns `E_INVALIDARG` on all 13 capture endpoints. Host checks found `Audiosrv` and `AudioEndpointBuilder` running, microphone consent `Allow` for both user and machine policy stores, and no AppPrivacy deny policy. These results make ordinary client contention or microphone privacy denial unlikely; they do not yet prove the exact driver-specific capture format accepted by the endpoint.

The follow-up one-second/no-flag retry still returns `E_INVALIDARG` on all 13 capture endpoints, so the result is not explained by the earlier 20 ms duration or event-callback setup. A three-way fresh-client format comparison also produced the same result for every capture endpoint: (1) the raw `GetMixFormat` pointer, (2) a copied full `WAVEFORMATEXTENSIBLE` value with its embedded `Format` pointer, and (3) a constructed IEEE-float `WAVEFORMATEX` using the endpoint's channel/rate fields. The 96 kHz mono capture endpoint fails all three variants too. This rules out the probe's earlier format-pointer lifetime/copy hypothesis and makes ordinary shared-client contention an insufficient explanation for the observed `E_INVALIDARG`.

This test changed no persistent configuration and required no restoration. It does not prove audible rendering, capture data flow, loopback latency, process-tree capture, or physical tone/impulse behavior.

The probe also tested endpoint loopback initialization on render endpoints using a fresh client and `LOOPBACK|AUTOCONVERTPCM|NOPERSIST`. Twenty of 21 render endpoints accepted the loopback client and were reset/released without starting or reading it. The same one endpoint that rejected normal shared initialization returned `AUDCLNT_E_EXCLUSIVE_MODE_ONLY` for loopback. This is positive endpoint-loopback initialization evidence, not loopback audio or latency evidence.

It still does not establish shared-mode capture/render behavior, loopback latency, process-tree capture, or physical tone/impulse behavior.

## Process-loopback and driver follow-up

The next capture probe cannot reuse endpoint activation. Microsoft’s application-loopback sample activates `IAudioClient` asynchronously through `ActivateAudioInterfaceAsync`, using `VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK` and a blob containing `AUDIOCLIENT_ACTIVATION_PARAMS`; the process-tree mode supports either include or exclude for one target process tree and requires Windows 10 build 20348 or later. The host build 26200 meets the documented OS minimum, but this probe has not yet been implemented or run. See the [official sample](https://github.com/microsoft/Windows-classic-samples/tree/main/Samples/ApplicationLoopback) and [`ActivateAudioInterfaceAsync`](https://learn.microsoft.com/en-us/windows/win32/api/mmdeviceapi/nf-mmdeviceapi-activateaudiointerfaceasync).

The driver gate remains unresolved. Microsoft describes SYSVAD as a source sample for a proprietary WDM audio device, not a finished AudioRouter bus driver or a production-signed redistributable. The host now has Visual Studio Community 18.9.2/MSVC 14.51.36231 and Windows SDK 10.0.26100 tools, but this repository does not yet contain a SYSVAD-derived driver project. No driver build, install, or signing claim is made. See [Microsoft sample audio drivers](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/sample-audio-drivers).

## Native capture cross-check

On 2026-09-05, `tools/m00-native-wasapi-probe/main.cpp` was compiled with the installed Visual Studio Developer Command Prompt (`cl /std:c++17 /EHsc`) and SDK libraries, then run read-only on `PATRICK5080`. It enumerated the same 13 active capture endpoints observed by the Rust probe. Every endpoint returned `S_OK` for `IAudioClient::Activate`, `GetMixFormat`, and `IsFormatSupported(AUDCLNT_SHAREMODE_SHARED, mixFormat)`, including the full extensible channel mask/subformat. Every endpoint returned `0x80070057` (`E_INVALIDARG`) from native `IAudioClient::Initialize` with `AUDCLNT_STREAMFLAGS_NOPERSIST` and a 100 ms buffer. The native result independently reproduces the failure, so it is not caused solely by the Rust Windows binding or by another client holding the endpoint.

The native harness did not call `Start`, `GetBuffer`, or `Read`; it changed no default endpoint, volume, mute, privacy policy, or persistent device setting. The executable and object file were removed after the run. The remaining diagnostic is endpoint/driver-specific initialization behavior; `E_INVALIDARG` is distinct from `AUDCLNT_E_DEVICE_IN_USE`, and shared mode normally permits concurrent clients. A production capture adapter still needs a successful native initialization path and subsequent no-default-change stream test.

A Rust scaffold for the documented asynchronous process-loopback activation ABI compiles, including the activation blob and completion-handler shape. Runtime invocation is intentionally disabled: Windows returned from the activation call and entered the callback, but the generated Rust COM result/interface handoff corrupted the probe before a trustworthy activation HRESULT could be collected. This is a binding-harness failure, not audio feasibility evidence. The scaffold is therefore not counted as a process-loopback pass; the reliable follow-up is the official native C++ sample in an installed Visual Studio/WDK environment.

The earlier Rust-scaffold limitation above is superseded by the native C++ activation and data-path results below. Both process-tree include and exclude modes now have successful activation and 500 ms read evidence; the remaining gap is controlled per-process tone attribution, not basic process-loopback implementation.

### Native process-loopback result (2026-09-05)

The native harness now uses an agile WRL `FtmBase` completion handler and correctly distinguishes `GetActivateResult` from the callback method HRESULT. `ActivateAudioInterfaceAsync` for the current Explorer process returned `S_OK`; the callback retrieved the activation result and queried `IAudioClient` successfully. Shared-mode initialization with 44.1 kHz stereo PCM plus `LOOPBACK|EVENTCALLBACK|AUTOCONVERTPCM` returned `S_OK`, and `SetEventHandle` returned `S_OK`. The harness did not start or read the stream, so this is activation/initialization evidence rather than captured-audio or latency evidence. This supersedes the earlier minimal-harness runtime result; the Rust path remains disabled pending an equivalent COM interop fix.

### Native process-loopback data result (2026-09-05)

The probe now has an opt-in `process-capture [pid] [milliseconds]` mode. Against the current PowerShell process for 500 ms, asynchronous activation, 44.1 kHz shared loopback initialization, capture service lookup, `Start`, event-driven reads, `Stop`, and `Reset` all returned success. It read 50 packets totaling 22,050 frames; no packets were marked silent and 15,217 nonzero payload bytes were observed. Samples were not retained. This is process-tree loopback data-flow evidence, not a latency or end-to-end routed-output claim. The temporary executable/object were removed after the run, and no defaults, volume, mute, privacy policy, or driver state changed.

The same harness now accepts `process-capture-exclude [pid] [milliseconds]`, selecting `PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE` explicitly. Against the current PowerShell process for 500 ms it also completed activation, initialization, event-driven capture, and cleanup successfully, reading 50 packets/22,050 frames with 15,217 nonzero payload bytes. These runs establish both API mode paths, but do not yet prove process-tree attribution against a controlled per-process tone source.

### Native probe rerun (2026-09-05)

Using the installed Visual Studio/WDK toolchain and the checked-in build script, the probe ran without changing machine audio configuration. Endpoint 0 completed shared capture start/read/stop/reset with 10 packets and 4,800 frames over 200 ms. Process-loopback include and exclude modes each completed activation, initialization, 20 packet reads, stop, and reset with 8,820 frames and nonzero payload bytes (6,064 and 27,569 respectively). The temporary executable and object file were removed afterward. This strengthens the native data-path evidence but still does not establish controlled per-process tone attribution or physical latency.
## 2026-09-05 native toolchain correction

The host now has Visual Studio Community 2026 `18.9.2`, MSVC `14.51.36231`, Windows SDK `10.0.28000.2526` (headers/libs under `10.0.28000.0`), and WDK `10.1.28000.2526`. The reproducible build entry point is [`tools/m00-native-wasapi-probe/build.ps1`](../../../tools/m00-native-wasapi-probe/build.ps1). It uses explicit toolchain paths and does not modify environment or audio settings.

The native C++ probe was built successfully and run on the current Windows user session. For all 13 active capture endpoints, native `Activate`, `GetMixFormat`, shared-mode `IsFormatSupported`, and shared-mode `IAudioClient::Initialize` with `AUDCLNT_STREAMFLAGS_NOPERSIST` returned success. This corrects the earlier Rust-only `E_INVALIDARG` result: the native path demonstrates that capture client initialization is available on this machine and that ordinary device contention was not the cause of the Rust probe failures.

The same executable's process-loopback path, targeting the current Explorer process tree, returned success for `ActivateAudioInterfaceAsync`, the completion result, `QueryInterface(IAudioClient)`, 44.1 kHz PCM shared-mode initialization with loopback/event/autoconvert flags, and `SetEventHandle`. The client was reset and released without `Start`, `GetBuffer`, or audio reads. This proves activation/configuration, not audible process capture or latency.

The native probe now includes an opt-in `process-attribution [milliseconds]` mode. It creates a short-lived child process, renders a deterministic 997 Hz tone through that child, captures the child's process tree via application loopback, and bounds child lifetime/cleanup. The mode compiled successfully with the installed Visual Studio/Windows SDK toolchain. It was intentionally not executed in this run because the controlled source emits an audible signal through the active render endpoint; therefore controlled attribution remains implementation-ready but unverified, and no machine audio configuration was changed.

Read-data modes now also report accumulated 16-bit sample energy. Only the controlled-attribution mode requires nonzero payload bytes; ordinary capture accepts valid silent packets, preserving the distinction between silence and stopped/unavailable sources. The change compiled successfully with temporary executable/object outputs removed afterward.

The opt-in `capture 0 200` command then exercised the native capture data path on active capture endpoint 0. `Start`, ten packet reads, and 4,800 frames over 200 ms returned success; samples were counted and discarded, followed by successful `Stop` and `Reset`. This confirms native shared-mode capture data flow for that endpoint without persisting audio or changing the user's configuration. It does not establish all-endpoint behavior, process-tree capture data, render-to-capture routing, or physical/loopback latency.

The probe source also contains an opt-in silent `render [endpoint-index] [milliseconds]` path that compiles against the same toolchain and submits only `AUDCLNT_BUFFERFLAGS_SILENT` buffers before stopping/resetting. Runtime execution of the newly built unsigned binary was blocked by the host's Windows Application Control policy, so render data-path success is not claimed. No security policy or signing setting was changed to bypass this block.

No endpoint was started, no buffer was read, no default device/volume/mute/format setting was changed, and no driver was installed or loaded.

## 2026-09-06 — Installed toolchain native data-path validation

Using the installed Visual Studio Community 2026, Windows SDK 10.0.28000.0,
and WDK toolchain, the checked-in build script compiled the native probe.
Endpoint 0 completed shared capture activation, initialization, start, ten
packet reads totaling 4,800 frames over 200 ms, stop, and reset. Process
loopback then completed asynchronous activation, 44.1 kHz shared
initialization, start, 50 packet reads totaling 22,050 frames over 500 ms,
stop, and reset; no packets were marked silent and 15,217 nonzero payload
bytes were observed. These are successful native data-path checks, not
physical routing or latency evidence.

The temporary executable and object file were removed after testing. The
probe did not change defaults, volume, mute, privacy policy, driver state, or
other persistent machine audio configuration.

The probe also completed a 200 ms silent render run on endpoint 0. Shared
initialization, buffer acquisition, start, submission of 13,920 frames using
`AUDCLNT_BUFFERFLAGS_SILENT`, stop, and reset all returned success. No tone
was generated, and no endpoint defaults or persistent audio settings changed.
## 2026-09-06 — Native endpoint-format cross-check

The installed Visual Studio/Windows SDK toolchain rebuilt the native probe, and
the read-only inventory path ran against all 13 active capture endpoints. Every
endpoint returned success for activation, `GetMixFormat`, shared-mode
`IsFormatSupported` using that exact mix format, and
`IAudioClient::Initialize` with `AUDCLNT_STREAMFLAGS_NOPERSIST`. No endpoint was
started or read, and the generated executable/object were removed afterward.
This confirms the earlier `E_INVALIDARG` was specific to the prior requested
format/flag combination, not ordinary client contention; full application
capture routing and latency evidence remain open.

## 2026-09-06 — Event-driven exact-format initialization

The native probe was extended with `event-capture-init [endpoint-index]` to
match the corrected Rust adapter boundary. On capture endpoint 0, activation,
`GetMixFormat`, shared `IAudioClient::Initialize` with the exact endpoint
format, `AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_NOPERSIST`,
zero buffer duration, and `SetEventHandle` all returned success. The client was
never started and no packet was read. This directly validates the request shape
that previously produced `E_INVALIDARG`; data-path, process attribution, and
latency evidence remain separate gates.

The same read-only event-initialization mode was then run for all 13 active
capture endpoints. Every endpoint returned success for exact-format shared
`Initialize` with `EVENTCALLBACK | NOPERSIST`, zero duration, and
`SetEventHandle`. No endpoint was started and no packet was read. This removes
ordinary endpoint contention as an explanation for the earlier `E_INVALIDARG`
result for this adapter request shape; data attribution and latency remain
open.

The generalized probe was also run across all 18 active render endpoints. 17
accepted the same exact-format event-driven initialization and event-handle
setup. One endpoint returned `AUDCLNT_E_DEVICE_IN_USE` (`0x8889000A`) while
the remaining endpoints returned success; no endpoint was started or rendered
to. This is a distinct, correctly classified busy-device result and does not
reintroduce the earlier capture `E_INVALIDARG` finding.

## 2026-09-06 — Rust adapter discrepancy isolation

A Windows-gated Rust integration attempt against the same first capture
endpoint (the endpoint ID and ordering matched the native probe) returned
`E_INVALIDARG` from the Rust `IAudioClient::Initialize` call. Before that call,
Rust `IAudioClient::IsFormatSupported` returned `S_OK` for the same
`GetMixFormat()` pointer. This narrows the remaining issue to the Rust COM/ABI
initialization boundary or its request marshalling, rather than endpoint
enumeration, format support, or ordinary device contention. The temporary
failing test and diagnostics were removed; the stable adapter suite remains
portable/metadata-only until this live-open discrepancy is resolved.

Further isolation tried a caller-owned copy of the complete 40-byte
`WAVEFORMATEXTENSIBLE` payload and separately removed the event and conversion
flags while retaining shared initialization. Both Rust variants still returned
`E_INVALIDARG`, so the failure is not caused by the endpoint allocation,
event-handle creation, or the `AUTOCONVERTPCM` flag. Temporary diagnostics were
removed after the run; the next fix must address the generated COM call/ABI
boundary or replace that binding path with a verified native shim.

A final control variation used a stack-owned simple PCM/IEEE-float
`WAVEFORMATEX` with the endpoint's observed 48 kHz, two-channel, 32-bit shape;
Rust `Initialize` still returned `E_INVALIDARG`. This rules out the
`WAVEFORMATEXTENSIBLE` payload as the determining factor. No temporary test
code remains in the adapter, and the native C++ reference path remains the
only live-initialization implementation qualified on this host.
