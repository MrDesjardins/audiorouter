# M00 WASAPI probe

## Status

The read-only endpoint inventory probe has been added at [`tools/m00-wasapi-probe`](../../../tools/m00-wasapi-probe). It uses Rust `windows` bindings and does not modify Windows defaults, open audio streams, install drivers, or write outside stdout.

## Current result

`cargo check --manifest-path tools/m00-wasapi-probe/Cargo.toml` and `cargo build --manifest-path tools/m00-wasapi-probe/Cargo.toml` pass with Rust `1.96.0` targeting `x86_64-pc-windows-msvc`; the Rust toolchain supplied a working linker path despite no `link.exe` or Visual Studio installation being on PATH.

`cargo run --manifest-path tools/m00-wasapi-probe/Cargo.toml --quiet` completed successfully on `PATRICK5080` and reported `active_endpoint_count=34`. Every enumerated endpoint returned state `0x00000001` (`DEVICE_STATE_ACTIVE`), a Windows endpoint ID, a mix format, and default/minimum device periods.

Observed capability summary: most endpoints reported 48,000 Hz, two channels, 32-bit samples, and 100,000/20,000 100-ns default/minimum periods (10 ms/2 ms). Sonar endpoints reported 96,000 Hz and eight channels. The Focusrite render endpoint reported a 30,000 100-ns minimum period (3 ms); other endpoints reported 2–3 ms minimum periods. One USB capture endpoint reported one channel at 96,000 Hz with format tag 3. These are current-device mix-format observations, not proof that every endpoint accepts every requested shared-mode format.

This probe establishes endpoint identity/state and current mix-format/device-period metadata only. It does not yet establish shared-mode capture/render, loopback latency, process-tree capture, or physical tone/impulse behavior.
