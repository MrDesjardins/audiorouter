# M00 WASAPI probe

## 2026-09-08 - Latest guarded requalification reconciliation

The latest guarded runs supersede the endpoint counts from earlier runs while
preserving those historical observations. `m00-native-live.ps1` passed all 13
capture and 18 render endpoints at 100 ms, with one occupied render correctly
classified. `m00-native-format-inventory.ps1` passed read-only activation and
`GetMixFormat` inspection for 31 endpoints, including 48 kHz 32-bit
extensible mono/stereo and 96 kHz 32-bit mono/eight-channel formats.

The selected VB-Audio pair also passed event-driven capture/render at 200 ms
(10,080 capture and 14,400 submitted render frames), endpoint loopback at 500
ms capture/800 ms tone (72,054 nonzero payload bytes), and digital impulse
correlation (97/100 groups, zero p95 spacing error, estimated onset 72.21 ms).
The controlled process attribution and exclusion runs passed at 500 ms with
21,609 frames/77,823 nonzero bytes and 22,050 frames respectively. The Rust
process-loopback include/exclude run passed at 250 ms with 89/93 scheduler
blocks and zero rejected packets, XRuns, and queue overruns. Every wrapper
stopped/reset streams, removed temporary artifacts where applicable, and
verified unchanged media state. These are current shared/event/process-loopback
and digital-correlation observations; calibrated physical latency, actual PID
reuse, managed-driver callback routing, and production signing remain open.

## 2026-09-08 - Longer impulse correlation requalification

The guarded native impulse wrapper was rerun with 1,000 impulses on the
explicitly named VB-Audio virtual-cable pair. It detected 998 groups, with p95
spacing error of 0 frames and estimated onset of 52.88 ms. Capture/render
teardown, temporary cleanup, and media-state preservation passed. The onset is
an uncalibrated digital-correlation estimate, not physical acoustic latency;
managed-driver callback and production gates remain open.

## 2026-09-08 - Impulse child-process ownership hardening

The impulse acceptance wrapper now launches its concurrent capture and impulse
children through directly owned `.NET Process` instances. This gives the
harness reliable exit-code reads, bounded redirected output collection, and
explicit kill/reap cleanup if setup or analysis fails. The requalified
100-impulse run detected 95 groups with p95 spacing error of 0 frames; stream
teardown, temporary cleanup, and media-state preservation passed. This improves
harness failure detection only and does not convert the uncalibrated onset into
physical-latency evidence.

## 2026-09-08 - Current process restart identity requalification

`cargo test -p audiorouter-windows-audio --locked -- --nocapture` passed all
30 tests, including the Windows-only bounded-helper restart regression. The
test observed PID, executable, and creation timestamp, rejected the terminated
process as stale, and observed a replacement helper. This run did not produce
an actual PID reuse event, so actual reuse remains unclaimed. The test is
read-only with respect to audio endpoints and machine configuration.

## 2026-09-08 - Capture fallback HRESULT regression

The Windows-audio unit suite now directly verifies that capture initialization
fallback is limited to exact Win32 `E_INVALIDARG` (`0x80070057`). The predicate
does not retry `AUDCLNT_E_DEVICE_IN_USE` (`0x8889000A`) or access denied
(`0x80070005`), preserving those as distinct failures. The 30-test
`audiorouter-windows-audio` suite, formatting, and strict Clippy pass. This is
an offline error-classification test; it opens no stream and changes no audio
configuration.

## 2026-09-08 - Digital impulse loopback correlation

The guarded `m00-native-impulse.ps1 -AllowLiveAudio -ImpulseCount 1000` run
emitted a bounded impulse train through the existing VB-Audio virtual cable
and captured the return. It detected 996 of 1,000 impulse groups with a p95
spacing error of 0 frames and an estimated onset of 75.33 ms. The wrapper
verified capture/render lifecycle, unchanged media-device state, and removed
the temporary executable, object, raw capture, and logs. This is digital
propagation/cadence evidence only; the estimated onset is not calibrated
physical acoustic latency and does not close the managed-driver gate.

The guarded `m00-rust-process-live.ps1 -AllowLiveAudio -DurationMilliseconds
250` acceptance passed both Rust asynchronous process-loopback modes. Include
converted 10,584 source frames at 44.1 kHz into 11,392 48 kHz engine frames
and 89 exact quanta; exclude converted 11,025 into 11,904 and 93 quanta.
Both modes reported zero rejected packets and zero scheduler XRuns, input or
output overruns/underruns, and the wrapper verified unchanged media state.
This strengthens the Rust adapter boundary but does not close production
driver callback timing or physical acoustic latency.

The guarded `m00-native-event-live.ps1 -AllowLiveAudio -DurationMilliseconds
500` acceptance passed over the selected VB-Audio pair. Event-driven capture
read 24,480 frames and silent render submitted 28,800 frames; start/stop/reset
all succeeded and the media-device snapshot remained unchanged. This is
event-lifecycle evidence only and does not close production-driver ownership
or physical-latency gates.

## 2026-09-08 - Guarded native live acceptance

Using the installed Visual Studio/Windows SDK/WDK toolchain, the explicit
`-AllowLiveAudio` acceptance wrapper built a disposable native probe and
passed `m00-native-live.ps1 -DurationMilliseconds 100` across all 13 active
capture and 21 render endpoints. Every capture completed bounded start/stop/
reset; every usable render completed silent start/submission/stop/reset, and
one endpoint was correctly reported as occupied. The wrapper compared the
media-device snapshot before and after and removed the temporary executable
and object file.

The same guarded wrapper passed `m00-native-process-live.ps1
-DurationMilliseconds 500`. A disposable child emitted a deterministic tone,
the selected process tree was captured for 19,845 frames with 70,847 nonzero
payload bytes, and the child exited cleanly. These results provide controlled
process-attribution data-path evidence, not physical acoustic latency or
managed-driver routing evidence. Defaults, volume, mute, privacy, driver,
signing, startup, and other persistent audio configuration were unchanged.

The complementary `m00-native-process-exclude-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500` run also passed. It selected the explicit exclude-
target-process-tree mode, captured 22,050 frames, verified clean child exit,
and found the media-device snapshot unchanged. Include and exclude modes are
therefore both exercised natively; this does not establish physical acoustic
latency or managed-driver routing.

The read-only `m00-native-format-inventory.ps1` acceptance then passed across
34 active endpoints. It verified activation, `GetMixFormat`, and complete
format descriptors for every endpoint, including 48 kHz mono/stereo
extensible float, 96 kHz mono PCM, and 96 kHz eight-channel extensible float
variants. The media-device snapshot was unchanged and temporary native
artifacts were removed. This supports format negotiation only, not universal
format acceptance or latency.

## Status

The read-only endpoint inventory probe has been added at [`tools/m00-wasapi-probe`](../../../../tools/m00-wasapi-probe). It uses Rust `windows` bindings and does not modify Windows defaults, start audio streams, install drivers, or write outside stdout.

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

It still does not establish complete shared-mode capture/render behavior,
loopback latency, controlled process-tree attribution, or physical
tone/impulse behavior. The later native process-loopback results in this file
provide bounded process-tree activation and packet-read evidence.

## Process-loopback and driver follow-up

The historical planning sentence immediately below predates the native
implementation and is superseded by the native activation and data-path
results later in this section. The checked-in probe covers both process-tree
include and exclude modes; controlled attribution and physical latency remain
unclaimed.

The next capture probe cannot reuse endpoint activation. Microsoft’s application-loopback sample activates `IAudioClient` asynchronously through `ActivateAudioInterfaceAsync`, using `VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK` and a blob containing `AUDIOCLIENT_ACTIVATION_PARAMS`; the process-tree mode supports either include or exclude for one target process tree and requires Windows 10 build 20348 or later. The host build 26200 meets the documented OS minimum, but this probe has not yet been implemented or run. See the [official sample](https://github.com/microsoft/Windows-classic-samples/tree/main/Samples/ApplicationLoopback) and [`ActivateAudioInterfaceAsync`](https://learn.microsoft.com/en-us/windows/win32/api/mmdeviceapi/nf-mmdeviceapi-activateaudiointerfaceasync).

The process-loopback implementation and bounded include/exclude data-path results are documented below. The remaining evidence gap is controlled per-process tone attribution and physical latency; those are not implied by activation or nonzero packet reads. See the [official sample](https://github.com/microsoft/Windows-classic-samples/tree/main/Samples/ApplicationLoopback) and [`ActivateAudioInterfaceAsync`](https://learn.microsoft.com/en-us/windows/win32/api/mmdeviceapi/nf-mmdeviceapi-activateaudiointerfaceasync).

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

The host now has Visual Studio Community 2026 `18.9.2`, MSVC `14.51.36231`, Windows SDK `10.0.28000.2526` (headers/libs under `10.0.28000.0`), and WDK `10.1.28000.2526`. The reproducible build entry point is [`tools/m00-native-wasapi-probe/build.ps1`](../../../../tools/m00-native-wasapi-probe/build.ps1). It uses explicit toolchain paths and does not modify environment or audio settings.

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

## 2026-09-06 — Reproducible native build

The checked-in `tools/m00-native-wasapi-probe/build.ps1` script compiled
`main.cpp` successfully with the installed Visual Studio Community 2026 MSVC
toolchain and Windows SDK/WDK libraries. This was a build-only verification;
the generated executable was not run and no endpoint was opened. The temporary
executable and object file were removed after compilation.
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

A fresh Rust process path using the hardcoded endpoint ID directly (without a
preceding endpoint enumeration) produced the same `E_INVALIDARG`. This rules
out enumeration/reopen lifetime as the cause; the Rust COM initialization
boundary remains the unresolved difference from the successful C++ call.

A temporary direct `IAudioClient` vtable invocation from Rust, bypassing the
generated `Initialize` method wrapper while using the same ABI argument types,
also returned `E_INVALIDARG`. The diagnostic code was removed after the run.
This rules out a defect limited to the generated method wrapper; the remaining
difference is the Rust process/runtime ABI context or its interaction with the
Windows binding, and the native C++ shim remains the verified fallback boundary.

The checked-in native probe was rebuilt again with Visual Studio Community 2026,
MSVC, and Windows SDK/WDK 10.0.28000.0 after the decision-register update.
Compilation succeeded; the generated executable and object were removed without
running the probe or opening an audio endpoint. This is toolchain evidence only,
and does not change the unresolved Rust live-open or physical-latency status.

At clean revision `d495b9e`, the same compile-only build completed successfully
using the installed Visual Studio Community 2026/MSVC and Windows SDK/WDK
toolchain. `main.exe` and `main.obj` were removed after verification. The
executable was not run, no stream was started or read, and no driver or audio
configuration was changed.

The checked-in `tests/acceptance/m00-native-build.ps1` now reproduces this
compile-only check with a unique temporary executable and `finally` cleanup. It
refuses to overwrite a pre-existing generated object and reports the scope as
compile-only; no audio stream, driver, signing mode, or machine configuration
action is performed.

The compile-only acceptance was rerun at clean revision `904c115` with the
installed Visual Studio Community 2026/MSVC and Windows SDK/WDK toolchain.
`main.cpp` compiled successfully, the temporary executable/object outputs were
cleaned, and the probe was not executed. No audio stream, driver, signing mode,
or machine configuration action occurred.

The compile-only acceptance was requalified again at the current revision using
the installed Visual Studio Community 2026/MSVC and Windows SDK/WDK toolchain.
This remains build evidence only: the probe was not executed, and no audio
stream, driver, signing mode, or machine configuration was touched.

The same compile-only acceptance was rerun at the current tip with the
installed VS2026/MSVC and Windows SDK/WDK toolchain. `main.cpp` compiled
successfully and the temporary executable/object outputs were cleaned. The
probe was not executed, so this does not claim live audio evidence.

## Live shared-capture qualification (2026-09-07)

With explicit authorization for controlled live testing, the native probe was
built with the installed VS2026/MSVC and Windows SDK/WDK toolchain and run
against the host's 13 enumerated capture endpoints. Each endpoint accepted
shared-mode initialization, started, delivered packets, stopped, and reset;
the bounded 100 ms sweep returned no `E_INVALIDARG` and no
`AUDCLNT_E_DEVICE_IN_USE`. The first endpoint exposed a 48 kHz, two-channel,
32-bit IEEE-float extensible format (`mask=0x3`) and delivered 4,800 frames.

The earlier 500 ms capture on that endpoint independently delivered 10 packets
and 4,800 frames. The event-driven capture initialization and event-handle
registration also returned `S_OK`. A post-test media-device snapshot matched
the pre-test snapshot: all ten listed media devices remained present and `OK`.
The probe stopped and reset every stream and did not alter defaults, volume,
mute, privacy, drivers, signing, or startup configuration.

This qualifies the native reference path and rules out device ownership as a
general explanation for the previous Rust `E_INVALIDARG`. It does not yet
qualify the Rust adapter's live-open boundary or production realtime latency.

## Rust adapter live qualification (2026-09-07)

The repository Rust WASAPI probe was then run against the same host. It
enumerated 34 active endpoints, including 13 capture endpoints. Every capture
endpoint reported success for the original, extensible, and float capture
initialization variants (`capture_original_hresult=0x0`,
`capture_extensible_hresult=0x0`, and `capture_float_hresult=0x0`). The
compatibility path therefore succeeds in the actual Rust process; the earlier
`E_INVALIDARG` was resolved by the checked-in event-first, polling-fallback
implementation. The fallback remains narrowly gated to the exact
`E_INVALIDARG` result and does not mask device-in-use or unrelated failures.

The Rust probe's separate render sweep had one endpoint classified as
`AUDCLNT_E_EXCLUSIVE_MODE_ONLY`; this is a render capability result and not a
capture failure. A final read-only post-test query found ten present media
devices and no device outside `OK`. No defaults, volume, mute, privacy,
drivers, signing, or startup configuration were changed.

## Process-loopback capture read (2026-09-07)

The native probe also performed a bounded 500 ms process-loopback capture read
for the current test process, without generating a tone. Asynchronous
activation, audio-client query, 44.1 kHz PCM initialization, event registration,
capture-service acquisition, start, packet reads, stop, and reset all returned
success. The read collected 50 packets and 22,050 frames, with 15,217 nonzero
bytes and nonzero sample energy; no silent packets were observed. The final
media-device state remained ten present devices, all `OK`. This qualifies the
loopback data path only; process attribution across a deliberately generated
source and physical output latency remain separate gates.
## 2026-09-06 — Current-tip compile qualification

The compile-only native probe acceptance was rerun at the current tip with
Visual Studio Community 2026, MSVC 14.51.36231, and the installed Windows
SDK/WDK toolchain. `main.cpp` compiled successfully and generated outputs were
cleaned. The probe was not executed; no audio stream, driver, signing mode, or
machine configuration was touched.
## Current-tip compile-only qualification (2026-09-06)

The checked-in native probe compiled successfully with the installed Visual
Studio Community 2026/MSVC and Windows SDK/WDK toolchain. The acceptance
script removed its temporary executable/object outputs; the probe was not
executed, so no audio stream, driver, signing mode, or machine configuration
was touched.

## Physical-loopback prerequisite probe (2026-09-07)

The native `tone` command now accepts an optional render endpoint index while
retaining endpoint 0 as its default, allowing a physical output to be selected
without changing the system default. The selected Focusrite analogue capture
endpoint (local capture index 10) initialized, started, delivered 10 packets /
4,800 frames over 500 ms, and stopped/reset successfully. The corresponding
Focusrite render endpoint (local render index 5) returned
`AUDCLNT_E_DEVICE_IN_USE` (`0x8889000A`) during initialization, so the probe
emitted no tone and no physical latency measurement was attempted. The before
and after media inventory was unchanged and the temporary executable was
removed. An available output route is required before the 1,000-impulse
physical-loopback gate can run.

## Occupied Focusrite render attribution (2026-09-07)

The read-only `render-ownership 5` diagnostic enumerated two sessions on the
Focusrite render endpoint. One is the system audio service; the other is
active process ID 35536, `voicemeeterpro.exe` at
`C:\Program Files (x86)\VB\Voicemeeter\voicemeeterpro.exe`. This explains the
matching `AUDCLNT_E_DEVICE_IN_USE` returned by the render initialization
probe. The process was only inspected and was not terminated; no Voicemeeter
routing, defaults, volume, mute, or other machine setting was changed.
The diagnostic now reports the executable image path directly through
`QueryFullProcessImageNameW`, so this attribution does not require a separate
shell process inspection.

The result is reproducible with the opt-in
`tests/acceptance/m00-native-live.ps1 -AllowLiveAudio` wrapper. It discovers
both directional endpoint counts, requires successful capture start/stop/reset
for every capture endpoint, accepts only the specifically classified
`AUDCLNT_E_DEVICE_IN_USE` render result, and compares the media snapshot before
and after. It is excluded from ordinary CI because it opens bounded live
streams.

The wrapper was rerun at the current head for 100 ms and passed with 13 active
capture endpoints and 21 active render endpoints; one render endpoint was
classified as the known occupied case. The directional count is discovered at
runtime (an earlier run had 18 render endpoints), and the media-device
identity/state snapshot remained unchanged.

## Silent shared-render lifecycle (2026-09-07)

The native probe completed a bounded 200 ms shared-render lifecycle on an
active endpoint using its 48 kHz, two-channel, 32-bit extensible mix format.
Activation, format retrieval, initialization, buffer/service acquisition,
start, silent submission, stop, and reset all returned success. The stream
submitted 14,400 silent frames from a 4,800-frame buffer. A post-test media
snapshot remained ten present devices, all `OK`; no audible tone or persistent
audio configuration change was made.

## Controlled process-tree attribution (2026-09-07)

The native probe launched its temporary child tone process and captured that
child's process loopback for 500 ms. Asynchronous activation, capture
initialization, event registration, capture start, packet reads, stop, and
reset all succeeded. The read collected 50 packets and 22,050 frames with no
silent packets, 76,661 nonzero bytes, and sample energy of `2.02077e+11`. The
child exited with code 0 and the temporary probe process was gone after the
run. The media-device snapshot remained ten present devices, all `OK`. This
qualifies process-tree inclusion and non-silent attribution; physical output
latency and restart/PID-reuse behavior remain open.

## Endpoint timing baseline (2026-09-07)

The probe now records `GetDevicePeriod` and `GetStreamLatency` after
initialization/start. On the qualified endpoint, the 48 kHz capture stream
reported a 10 ms default period, 3 ms minimum period, and
`GetStreamLatency=0` (100-ns units). The silent render stream reported a 10 ms
default period, 2 ms minimum period, a 4,800-frame buffer, and
`GetStreamLatency=0`. Both streams ran for 200 ms and completed stop/reset.
These are endpoint API timing values, not acoustic speaker-to-microphone
latency; the physical loopback measurement remains open. The post-test media
snapshot contained ten present devices, all `OK`.

This supersedes the earlier compile-only and “implementation-ready but
unverified” entries in this evidence file. Those entries remain as historical
test records; the current qualification is the bounded live result above.

## Process restart identity regression (2026-09-07)

The Windows-audio test suite now launches a bounded `cmd.exe` helper, observes
its PID, executable, and Windows creation timestamp through the real process
inventory, verifies the binding, terminates and reaps it, and confirms the old
identity cannot bind afterward. A replacement helper is also observed with a
creation identity. The regression passes as part of 16 Windows-audio tests.
The host did not reuse the PID during this run, so this proves stale-binding
rejection across restart but does not claim an actual PID-reuse event.

## Extensible format diagnostics (2026-09-07)

The probe now reports the WAVEFORMATEXTENSIBLE capture channel mask and
subformat GUID alongside the existing format fields. The packed Windows
structure is read with an explicit unaligned copy. The compile-only check
passes; the probe remains unexecuted, so this adds diagnostic capability
without claiming live audio evidence.

## Shared-mode buffer default (2026-09-07)

The probe now passes zero for the shared-mode buffer duration so the engine
selects the period rather than imposing a one-second capture request. This is
the required value for shared event-driven streams and the minimum-latency
choice for other shared streams, but a nonzero sufficiently large shared-mode
buffer is also valid. The change removes one diagnostic variable; it does not
confirm the cause of `E_INVALIDARG`. The probe remains compile-checked only.

## Current-tip compile-only qualification (2026-09-07)

The checked-in native probe compiled successfully with the installed Visual
Studio Community 2026/MSVC and Windows SDK/WDK toolchain. The acceptance
script removed its temporary executable/object outputs; the probe was not
executed, so no audio stream, driver, signing mode, or machine configuration
was touched.

## Virtual-cable render-to-capture loopback (2026-09-07)

The probe now exposes friendly names in its read-only endpoint inventory and
polls capture packets throughout the bounded interval. This avoids mistaking
an already-drained shared-mode buffer for silence. An authorized 1,000 ms
capture on `CABLE Output (VB-Audio Virtual Cable)` ran concurrently with a
1,500 ms generated tone on `CABLE Input (VB-Audio Virtual Cable)`. The render
stream wrote a tone and submitted 76,800 frames; capture observed 99 packets,
47,520 frames, zero silent packets, and 200,532 nonzero payload bytes. Both
streams returned successful start/stop/reset results. The media-device
identity/state snapshot matched before and after, and no endpoint default,
volume, mute, privacy, driver, signing, or startup setting was changed.

This is a digital virtual-cable data-path qualification, not an acoustic
speaker-to-microphone latency measurement. Physical latency remains open.

The run is reproducible with the opt-in
`tests/acceptance/m00-native-loopback.ps1 -AllowLiveAudio` wrapper. The wrapper
resolves both virtual endpoints by friendly name instead of hard-coding their
enumeration positions, validates the complete bounded lifecycle, requires
nonzero captured payload, and compares the media identity/state snapshot.

## USB speaker/microphone signal-path smoke (2026-09-07)

The available `Speakers (PD200X Podcast Microphone)` render endpoint was
checked read-only first and had no active application owner. An authorized
bounded run then opened it together with `Microphone (PD200X Podcast
Microphone)` capture. The render stream initialized and wrote a 700 ms tone,
submitting 37,920 frames; the 1,000 ms mono capture stream initialized and
observed 100 packets/48,000 frames with 73,521 nonzero payload bytes. All
start/stop/reset calls returned success and the media-device identity/state
snapshot was unchanged. No endpoint default, volume, mute, privacy, driver,
signing, or startup setting was changed.

This is only a physical signal-path smoke result: ambient/input content is not
separated from the generated tone, and no impulse timestamps or acoustic
round-trip distribution were measured. The required 1,000-impulse
speaker-to-microphone latency gate therefore remains open.

The same wrapper can reproduce the USB smoke by selecting
`-RenderFriendlyName 'Speakers (PD200X Podcast Microphone)'`
and `-CaptureFriendlyName 'Microphone (PD200X Podcast Microphone)'`. It keeps
the endpoint lookup name-based and preserves the same bounded duration,
nonzero-payload, media-snapshot, and cleanup checks. This remains signal-path
smoke evidence only; it does not replace the 1,000-impulse acoustic test.

The wrapper is generic over the explicitly selected friendly-name pair, with
the VB-Audio cable retained as the default. A current run passed for both the
cable and the PD200X USB pair.

## Impulse-train correlation harness (2026-09-07)

The native probe now supports `impulse` (a deterministic 10 ms impulse train)
and `capture-file` (bounded raw packet capture). The opt-in
`tests/acceptance/m00-native-impulse.ps1 -AllowLiveAudio` wrapper resolves
friendly-name endpoint pairs, defaults to 1,000 impulses, validates lifecycle,
correlates captured peaks, reports p95 inter-impulse spacing error and an onset
estimate, compares the media snapshot, and removes temporary executable,
object, log, and raw-capture files. The onset estimate is deliberately not
treated as the required calibrated physical p95 latency gate.

The virtual-cable run passed correlation with 996/1,000 detected groups, zero
p95 spacing error, and a 76.92 ms estimated onset. Running the same analyzer on
the PD200X pair detected zero groups. A bounded follow-up captured 95,520 mono
frames and 146,220 nonzero bytes, but the maximum absolute float sample was
only `2.15e-6`, with no samples above `0.001`. The render lifecycle succeeded,
but no measurable impulse returned to the microphone, so the current physical
setup cannot qualify the required acoustic latency distribution. The result
is recorded as an explicit failed/unqualified gate; the threshold was not
lowered to turn ambient/noise data into a latency pass.

## Current-head digital impulse requalification (2026-09-07)

The authorized VB-Audio digital loopback was rerun from the current head with
the pinned 1,000-impulse, 10 ms configuration. The analyzer detected 997/1,000
groups, measured zero-frame p95 inter-impulse spacing error, and estimated a
63.73 ms digital onset. The media identity/state snapshot was unchanged and
the wrapper removed its temporary executable, object, logs, and raw capture.
This confirms repeatable digital correlation only; it does not promote the
estimate to calibrated speaker-to-microphone latency evidence.

## Current-head physical impulse requalification (2026-09-07)

The authorized 1,000-impulse analyzer was rerun on the available PD200X
speaker/microphone pair. It detected 0/1,000 impulse groups, below the required
900-group threshold, so the physical acoustic-latency gate remains
unqualified. The wrapper performed its media identity/state comparison and
temporary-output cleanup; the threshold was not lowered and no persistent
audio configuration was changed.

## Current-head native lifecycle requalification (2026-09-07)

The authorized bounded native live wrapper was rerun from the current head.
It discovered 13 capture endpoints and 21 render endpoints, classified one
render endpoint as occupied, and completed the 100 ms shared-capture and
silent-render lifecycle checks successfully. The media inventory was unchanged
and temporary executable/object/log outputs were removed. This remains stream
lifecycle evidence; it does not qualify physical latency or driver behavior,
and no default, volume, mute, privacy, startup, or other machine audio setting
was changed.

## Event-driven capture data path (2026-09-07)

The native probe now exposes an opt-in `event-capture` command. It creates an
event, initializes shared capture with `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`,
registers the event before starting, waits for event notifications, drains
available packets, and keeps the event alive through stop/reset. An authorized
500 ms run on capture endpoint 0 completed initialization, event registration,
start, 50 packets/24,000 frames, stop, and reset successfully. The selected
endpoint returned zero nonzero payload bytes during this interval, so this
qualifies event-driven lifecycle and packet draining only; it does not claim
signal propagation, physical latency, or native realtime graph scheduling.

## Event-driven render submission (2026-09-07)

The native probe also exposes an opt-in `event-render` command. It initializes
shared render with `AUDCLNT_STREAMFLAGS_EVENTCALLBACK`, registers the event,
waits for render availability, submits only `AUDCLNT_BUFFERFLAGS_SILENT`
buffers, and retains the event through stop/reset. An authorized 500 ms run on
render endpoint 0 completed initialization, event registration, start, 28,320
submitted silent frames, stop, and reset successfully. This qualifies the
event-driven render lifecycle and bounded submission path only; it does not
claim audible routing, physical latency, or driver integration.
## 2026-09-07 — Lifecycle regression after format-identity hardening

The authorized `m00-native-live.ps1 -AllowLiveAudio -DurationMilliseconds 100`
wrapper was rerun after endpoint enumeration began preserving extensible channel
mask and subformat identity. It discovered 13 capture endpoints and 21 render
endpoints; all bounded capture and silent-render lifecycle checks passed, with
one render endpoint correctly classified as occupied. The media-device snapshot
was unchanged and the temporary executable was removed. Defaults, volume, mute,
privacy, drivers, signing, startup registration, and other persistent audio
configuration were not changed.

## Repeatable event-path acceptance (2026-09-07)

`tests/acceptance/m00-native-event-live.ps1` provides guarded,
friendly-name-selected event qualification. It builds the native probe into an
isolated temporary executable, runs `event-capture` and `event-render` on the
selected VB-Audio pair, requires complete event lifecycles, compares the media
snapshot, and removes the executable and generated object. The authorized
500 ms run captured 24,480 frames and submitted 28,800 silent render frames.
This is repeatable event-path evidence; it does not claim audible routing,
physical latency, driver behavior, or persistent configuration changes.

## Current-head process-tree exclusion (2026-09-07)

The authorized `process-capture-exclude` probe was rerun against the current
test process. Process-loopback activation, 44.1 kHz PCM initialization, event
registration, capture start, and cleanup all returned success; the stream read
50 packets/22,050 frames with 15,217 nonzero payload bytes and no silent
packets. Temporary outputs were removed. This qualifies the documented single
target-tree exclusion mode only; arbitrary multi-process exclusion, PID reuse,
and production route supervision remain separate gates.

## Current-head controlled process attribution (2026-09-07)

The authorized `process-attribution` probe was rerun with a disposable child
tone. Asynchronous process-loopback activation, 44.1 kHz PCM initialization,
event registration, capture start, packet reads, and stop/reset all succeeded.
The child exited with code 0; capture observed 49 packets/21,609 frames, zero
silent packets, and 76,370 nonzero payload bytes. The temporary executable was
removed. This confirms controlled include-tree data attribution only; PID reuse,
physical latency, and production supervisor route restart remain separate
gates.

## Repeatable controlled-attribution acceptance (2026-09-07)

`tests/acceptance/m00-native-process-live.ps1` now builds the native probe in
an isolated temporary path, runs the disposable-child `process-attribution`
scenario, requires successful asynchronous activation, capture lifecycle,
child exit, and positive payload, compares the media snapshot, and removes
generated outputs. The authorized 500 ms run captured 22,050 frames and 78,114
nonzero bytes. This makes the include-tree evidence repeatable without claiming
PID reuse, arbitrary exclusion sets, physical latency, or production restart
supervision.

## Read-only endpoint format inventory (2026-09-07)

The native probe now provides a separate `inventory-formats` command so the
stable name/ID inventory remains script-compatible while each endpoint's
`IAudioClient::GetMixFormat` result can be inspected. The authorized run
successfully queried all 21 active render and 13 active capture endpoints. It
found a genuine available rate difference: CABLE Output capture is 48 kHz
float32 stereo, while SteelSeries Sonar Aux render is 96 kHz float32 eight
channel. This identifies a candidate for differing-rate resampler testing;
format inspection is read-only and did not open a stream or change device
configuration.

The first guarded route attempt with that pair was blocked before inventory by
Windows Application Control when launching its newly generated temporary Rust
executable. No stream opened; existing-rate route evidence remains valid.

## Repeatable endpoint-format acceptance (2026-09-07)

`tests/acceptance/m00-native-format-inventory.ps1` now builds the native probe
with a per-run temporary object, runs `inventory-formats`, requires one
successful `GetMixFormat` result for every discovered endpoint, snapshots media
identity/state, and removes both generated files. The authorized read-only run
passed for all 34 active endpoints. The distinct formats were 48 kHz mono,
48 kHz stereo, 96 kHz mono, and 96 kHz eight-channel float/PCM mix formats.
No audio stream was opened and no machine configuration changed.

The format printer now emits the complete `WAVEFORMATEXTENSIBLE` subtype GUID
alongside the channel mask and valid-bit count. The current endpoint set
reports `00000003-0000-0010-8000-00aa00389b71` for its float32 extensible
formats; the 96 kHz mono endpoint remains legacy IEEE-float PCM and is printed
without extensible-only fields.
## Controlled process attribution requalification (2026-09-07)

The guarded `m00-native-process-live.ps1 -AllowLiveAudio -DurationMilliseconds
500` wrapper passed at the current head. It created only a disposable child,
verified child exit, process-loopback lifecycle, and 21,609 captured frames with
76,370 nonzero payload bytes, then confirmed the media-device identity/state
snapshot was unchanged and removed the exact generated executable/object. This
is controlled process-tree evidence; PID reuse and physical acoustic latency
remain unqualified. No existing process was terminated and no audio setting was
changed.

## Digital loopback requalification (2026-09-07)

The guarded `m00-native-loopback.ps1 -AllowLiveAudio` wrapper passed over the
existing VB-Audio Virtual Cable. A bounded tone/capture run produced 219,572
nonzero payload bytes over 1,000 ms capture and 1,500 ms tone windows. The
wrapper verified unchanged media-device identity/state and exact temporary
cleanup. It changed no defaults, volume, mute, privacy, driver, signing,
startup, or other machine audio configuration. This qualifies digital signal
propagation only; physical acoustic latency remains unqualified.

## Digital loopback requalification (2026-09-08)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-loopback.ps1 -AllowLiveAudio
-CaptureDurationMilliseconds 1000 -ToneDurationMilliseconds 1500`.

The explicitly selected existing VB-Audio endpoints passed the bounded render
and capture lifecycle, producing 209,982 nonzero payload bytes during the
1,000 ms capture window while the 1,500 ms tone window ran. The wrapper
verified unchanged media-device identity/state and removed the exact temporary
executable, logs, and object. Defaults, volume, mute, privacy, drivers,
signing, startup, and other machine audio configuration were unchanged. This
is digital cable propagation evidence only; it does not qualify physical
acoustic latency, managed virtual-device ownership, or production signing.
## Bounded live endpoint lifecycle (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-live.ps1 -AllowLiveAudio -DurationMilliseconds 250`.

The authorized native check passed for 13 shared capture endpoints and 21
render endpoints. It opened each client only for the bounded test duration,
used silent render buffers, and stopped/resets clients during cleanup; one
already-occupied render endpoint was recognized and handled. The acceptance
script reported that defaults, volume, mute, privacy, drivers, signing, and
startup configuration were unchanged. This is lifecycle evidence, not physical
latency, process attribution, or production-driver evidence.

## Bounded live endpoint lifecycle requalification (2026-09-08)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-live.ps1 -AllowLiveAudio -DurationMilliseconds
250`.

The current native probe exercised 13 active capture and 21 active render
endpoints. All bounded capture and silent-render lifecycle checks passed; one
occupied render endpoint was recognized as the expected
`AUDCLNT_E_DEVICE_IN_USE` case. The wrapper stopped/reset clients, verified
the media-device snapshot, and reported defaults, volume, mute, privacy,
drivers, signing, and startup configuration unchanged. This remains endpoint
lifecycle evidence, not physical acoustic latency or managed-driver callback
evidence.
## Event-driven live lifecycle (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-event-live.ps1 -AllowLiveAudio
-DurationMilliseconds 250`.

The event-mode check passed on the existing `CABLE Input (VB-Audio Virtual
Cable)` render and `CABLE Output (VB-Audio Virtual Cable)` capture endpoints,
reporting 16,800 submitted render frames and 12,480 capture frames. Render
buffers were silent, clients were stopped/reset by the harness, and the script
reported defaults, volume, mute, privacy, drivers, signing, and startup
configuration unchanged. This is event lifecycle evidence, not physical
latency or production-driver evidence.
## Bounded virtual-cable signal path (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-loopback.ps1 -AllowLiveAudio
-CaptureDurationMilliseconds 250 -ToneDurationMilliseconds 250`.

The selected existing VB-Audio virtual cable passed the signal-path check:
the 250 ms render tone produced 6,056 nonzero captured payload bytes. The
harness verified capture/tone start-stop-reset lifecycle, compared media-device
identity/state before and after, and removed temporary probe files. It reported
defaults, volume, mute, privacy, drivers, signing, and startup configuration
unchanged. This is signal-path evidence, not calibrated physical-latency or
production-driver evidence.
## Bounded process-scoped loopback (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-process-live.ps1 -AllowLiveAudio
-DurationMilliseconds 250`.

The disposable process-tree loopback check passed, capturing 10,584 frames and
32,907 nonzero bytes. The harness verified process activation, child exit,
capture stop/reset, media-device identity/state stability, and temporary-file
cleanup. It reported no persistent audio configuration change. This is
process-attribution evidence only; it does not close physical latency,
cross-process exclusion, or production-driver gates.
## Bounded impulse correlation (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-impulse.ps1 -AllowLiveAudio -ImpulseCount 50`.

The selected existing VB-Audio virtual cable produced 51 detected groups for
50 expected impulses, with p95 spacing error of 471 frames and estimated onset
of 26.27 ms. Temporary raw capture/log files were removed after the run. This
is bounded software signal-correlation evidence only; it is not the calibrated
acoustic p95 latency gate.
## Process restart and PID-reuse binding regression (2026-09-07)

Command: `cargo test -p audiorouter-windows-audio
restarted_process_cannot_inherit_a_stale_binding --locked -- --nocapture`.

The focused Windows test passed. It launches bounded disposable helper
processes, compares executable and creation-time identity, and rejects stale
binding inheritance after restart. No audio device or persistent machine
configuration is accessed. This is deterministic process-identity evidence;
full reboot, Windows Audio service restart, and multi-user transition evidence
remain open.

## Process-loopback exclusion mode (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-process-exclude-live.ps1 -AllowLiveAudio
-DurationMilliseconds 250`.

The controlled native harness now launches a disposable child tone process and
captures with `PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE`. Activation,
capture start/stop/reset, child exit, and media-device identity/state checks are
required. This validates the supported exclusion mode and lifecycle, but does
not claim a cross-process rejection threshold because unrelated system audio is
not controlled by this fixture. The run passed with 11,025 captured frames.
Temporary binaries are removed and no persistent audio configuration is changed.
The include-tree regression was also rerun after the harness change: it passed
with 10,584 frames and 34,368 nonzero bytes.

## Rust process-loopback adapter (2026-09-07)

The production `audiorouter-windows-audio` crate now exposes
`ProcessLoopbackCapture` with explicit include/exclude target-tree mode. It
uses `ActivateAudioInterfaceAsync`, transfers the callback-owned COM reference
without crossing a Rust thread boundary as a COM object, initializes the
supported event-driven 44.1 kHz stereo PCM shape, copies packets into bounded
caller storage, and stops/resets deterministically. `PROPVARIANT` ownership is
left to its Windows binding destructor; a temporary live run caught and fixed
the double-free that otherwise caused `STATUS_HEAP_CORRUPTION`.

The guarded Rust acceptance passed both modes for 250 ms: each reported 24
packets and 10,584 frames, followed by unchanged media-device identity/state.
This is process-loopback API/lifecycle evidence, not a claim of controlled
cross-process signal rejection or physical latency.

## Native event-driven lifecycle recheck (2026-09-08)

The bounded native event acceptance passed for 300 ms on the explicitly named
VB-Audio endpoints: 14,400 capture frames and 19,200 silent render frames.
Both event-driven clients returned successful initialize, event-handle, start,
stop, and reset results. The media-device snapshot was unchanged. This is
shared-mode lifecycle evidence only and does not establish callback deadline,
physical latency, managed-driver lifecycle, or production signing.

## Long digital impulse correlation recheck (2026-09-08)

The authorized 1,000-impulse VB-Audio cable acceptance detected 997 groups,
with p95 inter-group spacing error of 0 frames and estimated onset of 67.27 ms.
The capture/render processes completed their lifecycle and temporary artifacts
were removed. This is digital signal-correlation evidence only; the estimated
onset is not calibrated physical p95 latency and does not satisfy NFR-01/02/03.

## Native digital signal-path recheck (2026-09-08)

The bounded native tone/loopback acceptance passed with the explicitly
selected VB-Audio endpoints: a 1,500 ms render tone produced 213,082 nonzero
capture payload bytes during the 1,000 ms capture window. Capture and render
lifecycle cleanup succeeded and defaults, volume, mute, privacy, drivers,
signing, startup configuration, and media state were unchanged. This confirms
digital signal transfer only; it is not calibrated physical latency or
managed-driver/application compatibility evidence.

## Maximum digital impulse correlation (2026-09-08)

The maximum 2,000-impulse VB-Audio correlation acceptance passed with 1,997
detected groups and zero p95 inter-group spacing error. Estimated onset was
77.77 ms. Temporary capture/impulse artifacts were removed and the selected
endpoint lifecycle completed cleanly. This remains digital timing evidence;
the onset is not calibrated physical latency and does not satisfy the hardware
latency gate.

## Native process-loopback exclusion recheck (2026-09-08)

The bounded native exclusion-mode acceptance passed for 250 ms and captured
11,025 frames while excluding a disposable child process tree from the
selected process-loopback source. Child cleanup, stream stop/reset, and the
media-state snapshot completed successfully. This validates the API mode and
lifecycle only; it does not establish a full cross-process rejection
threshold, production-driver behavior, or physical latency.

## Native process attribution recheck (2026-09-08)

The bounded native process-attribution acceptance passed for 500 ms using a
disposable child and selected process tree: 21,609 capture frames and 77,823
nonzero payload bytes were observed. The child exited, capture lifecycle
completed, temporary artifacts were removed, and the media snapshot was
unchanged. This validates controlled process-tree attribution only; it does
not prove exclusion thresholds, reboot/PID-reuse behavior beyond the focused
regression, or physical latency.

## USB speaker/microphone signal-path requalification (2026-09-08)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-loopback.ps1 -AllowLiveAudio
-CaptureDurationMilliseconds 1000 -ToneDurationMilliseconds 1500
-RenderFriendlyName "Speakers (PD200X Podcast Microphone)"
-CaptureFriendlyName "Microphone (PD200X Podcast Microphone)"`.

The current inventory-selected PD200X render/capture pair passed the bounded
signal-path lifecycle and produced 129,225 nonzero capture payload bytes. The
wrapper verified exact temporary cleanup and unchanged endpoint identity/state;
defaults, volume, mute, privacy, drivers, signing, and startup configuration
were unchanged. This is a physical-path signal smoke only: ambient input was
not separated from the tone and no calibrated acoustic impulse distribution or
physical p95 latency claim is made.

## USB acoustic impulse attempt (2026-09-08)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-native-impulse.ps1 -AllowLiveAudio -ImpulseCount 1000
-RenderFriendlyName "Speakers (PD200X Podcast Microphone)"
-CaptureFriendlyName "Microphone (PD200X Podcast Microphone)"`.

The explicitly selected PD200X pair completed the bounded attempt but the
analyzer detected 0 of 1,000 impulse groups and therefore rejected the run
against its 90% threshold. The wrapper removed the generated executable,
capture/log artifacts, and object; no endpoint identity/state or audio setting
changed. This negative result confirms that the current physical return path
is not measurable by this thresholded harness; the calibrated acoustic
latency gate remains unqualified and the threshold was not weakened.

## Disposable native build isolation (2026-09-08)

`tools/m00-native-wasapi-probe/build.ps1` now derives an implicit object path
beside a custom `-Output` executable, while preserving `main.obj` for the
default source-directory output and honoring explicit `-Object` overrides.
The compile acceptance asserts that custom-output builds do not leave an
object in the repository tool directory and that the adjacent temporary
object is cleaned. Native compile acceptance, PowerShell parsing, formatting,
documentation validation, and diff checks pass. This changes only temporary
build-artifact placement; it does not open audio or alter machine state.

All six native live acceptance wrappers that use custom executable outputs now
also remove the adjacent implicit object generated by the build script. The
21-script PowerShell parse check passed, and the bounded VB-Audio loopback
acceptance passed with 4,414 nonzero payload bytes; its temporary executable,
object, logs, and repository-side artifacts were absent after cleanup. No
audio configuration changed.

The same wrapper cleanup change was requalified on both process-loopback paths
on 2026-09-08. The attribution run captured 10,584 frames and 32,907
nonzero bytes; the exclusion run captured 11,025 frames while excluding the
disposable child tree. Both completed child/stream cleanup, left no adjacent
temporary object or repository `main.obj`, and changed no persistent audio
configuration. This remains controlled process-loopback evidence, not a full
cross-process isolation threshold or actual PID-reuse observation.

The event-driven wrapper was also requalified on 2026-09-08: the selected
VB-Audio endpoints completed the 250 ms lifecycle with 12,480 capture frames
and 16,320 silent render frames. The adjacent object and repository `main.obj`
were absent after cleanup, and the wrapper reported unchanged audio settings.
This is shared-mode event lifecycle evidence only.

The impulse wrapper was requalified after the custom-output build change on
2026-09-08 using 50 impulses over the existing VB-Audio cable. It detected
45 groups with zero p95 spacing error and an estimated 82.54 ms digital onset.
The executable, adjacent object, raw capture, and logs were all absent after
cleanup; no repository object or audio configuration remained changed. This
is bounded digital correlation, not calibrated physical acoustic latency.

## Native process attribution requalification (2026-09-08)

`tests/acceptance/m00-native-process-live.ps1 -AllowLiveAudio
-DurationMilliseconds 750` completed against the current native probe. The
disposable child/process-tree route captured 33,075 frames and 121,582
nonzero payload bytes, then stopped and released its streams successfully.
The wrapper reported unchanged persistent audio configuration. This strengthens
controlled process-tree attribution evidence only; it is not PID-reuse,
physical-latency, managed-driver callback, or production isolation evidence.

The paired exclusion run completed with the same 750 ms bound and 33,075
captured frames. It excluded the disposable child process tree, released the
native streams, and reported no persistent configuration change. This
confirms the exclusion API/lifecycle path; it does not establish a quantitative
cross-process isolation threshold or PID-reuse protection.

## Native event lifecycle requalification (2026-09-08)

`tests/acceptance/m00-native-event-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500` passed on the explicitly selected VB-Audio pair:
24,480 capture frames and 28,320 silent render frames were processed. Both
streams completed their event-driven lifecycle and cleanup; defaults, volume,
mute, privacy, driver, signing, and startup state were unchanged. This is
shared-mode event lifecycle evidence, not production callback or latency proof.

## Native digital impulse requalification (2026-09-08)

`tests/acceptance/m00-native-impulse.ps1 -AllowLiveAudio -ImpulseCount 100`
passed on the explicitly selected VB-Audio cable. The analyzer detected 97 of
100 impulse groups, with zero p95 spacing error and an estimated digital onset
of 69.13 ms. Temporary executable/object/raw/log artifacts were cleaned up,
and persistent audio configuration was unchanged. This is digital return-path
correlation only; it is not calibrated acoustic latency or production-driver
callback evidence.

## Native tone/loopback signal-path requalification (2026-09-08)

`tests/acceptance/m00-native-loopback.ps1 -AllowLiveAudio
-CaptureDurationMilliseconds 1000 -ToneDurationMilliseconds 1500` passed on
the explicitly selected VB-Audio pair. The native capture lifecycle recorded
216,970 nonzero payload bytes while the render tone lifecycle completed
successfully. Both streams were stopped/reset and persistent defaults, volume,
mute, privacy, driver, signing, and startup configuration were unchanged.
This proves digital signal-path propagation only; calibrated physical latency,
managed-driver ownership, and production callback timing remain open.
