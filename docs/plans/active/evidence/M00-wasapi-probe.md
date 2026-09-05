# M00 WASAPI probe

## Status

The read-only endpoint inventory probe has been added at [`tools/m00-wasapi-probe`](../../../tools/m00-wasapi-probe). It uses Rust `windows` bindings and does not modify Windows defaults, start audio streams, install drivers, or write outside stdout.

## Current result

`cargo check --manifest-path tools/m00-wasapi-probe/Cargo.toml` and `cargo build --manifest-path tools/m00-wasapi-probe/Cargo.toml` pass with Rust `1.96.0` targeting `x86_64-pc-windows-msvc`; the Rust toolchain supplied a working linker path despite no `link.exe` or Visual Studio installation being on PATH.

`cargo run --manifest-path tools/m00-wasapi-probe/Cargo.toml --quiet` completed successfully on `PATRICK5080` and reported `active_endpoint_count=34`. Every enumerated endpoint returned state `0x00000001` (`DEVICE_STATE_ACTIVE`), a Windows endpoint ID, a mix format, and default/minimum device periods.

Observed capability summary: most endpoints reported 48,000 Hz, two channels, 32-bit samples, and 100,000/20,000 100-ns default/minimum periods (10 ms/2 ms). Sonar endpoints reported 96,000 Hz and eight channels. The Focusrite render endpoint reported a 30,000 100-ns minimum period (3 ms); other endpoints reported 2–3 ms minimum periods. One USB capture endpoint reported one channel at 96,000 Hz with format tag 3. These are current-device mix-format observations, not proof that every endpoint accepts every requested shared-mode format.

The probe also calls `IAudioClient::IsFormatSupported` in shared mode for 44.1/48 kHz mono/stereo IEEE-float formats. This is a capability query only: it does not initialize or run a capture/render stream and does not alter Windows configuration. On this run, all 34 endpoints returned `S_FALSE` (closest-match available) for 44.1 kHz mono and stereo; 48 kHz mono was exact (`S_OK`) on 1/34 and closest-match on 33/34; 48 kHz stereo was exact on 29/34 and closest-match on 5/34. No other HRESULTs occurred. `S_FALSE` is not a failure; the returned closest format must be inspected when implementing negotiation.

No configuration restoration was required: the probe only activated endpoint/client COM objects, queried metadata, and released them. It did not initialize an audio stream, set a format, change a default, mute, or alter a device.

## Shared-mode initialization result

The probe was extended to call `IAudioClient::Initialize` with `AUDCLNT_SHAREMODE_SHARED`, the endpoint's current mix format, `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_NOPERSIST`, and zero duration/period parameters. On the direct run, 20 of 34 endpoints initialized successfully, reported buffer sizes from 1,056 to 2,112 frames, were reset, and were released. Thirteen returned `0x80070057` (`E_INVALIDARG`) and one returned `0x8889000a` (`AUDCLNT_E_EXCLUSIVE_MODE_ONLY`). No client was started; `GetStreamLatency` therefore returned zero and is not a latency measurement.

The initialization test was non-persistent and did not modify existing configuration. It did not change default endpoints, volume, mute, exclusive-mode settings, or device properties. All activated clients were reset/released before the process exited; no restoration action was required.

It still does not establish shared-mode capture/render behavior, loopback latency, process-tree capture, or physical tone/impulse behavior.
