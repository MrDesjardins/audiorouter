# M00 WASAPI probe

## Status

The read-only endpoint inventory probe has been added at [`tools/m00-wasapi-probe`](../../../tools/m00-wasapi-probe). It uses Rust `windows` bindings and does not modify Windows defaults, open audio streams, install drivers, or write outside stdout.

## Current result

`cargo check --manifest-path tools/m00-wasapi-probe/Cargo.toml` and `cargo build --manifest-path tools/m00-wasapi-probe/Cargo.toml` pass with Rust `1.96.0` targeting `x86_64-pc-windows-msvc`; the Rust toolchain supplied a working linker path despite no `link.exe` or Visual Studio installation being on PATH.

`cargo run --manifest-path tools/m00-wasapi-probe/Cargo.toml --quiet` completed successfully on `PATRICK5080` and reported `active_endpoint_count=34`. Every enumerated endpoint returned state `0x00000001` (`DEVICE_STATE_ACTIVE`) and a Windows endpoint ID. The full stdout is the raw result for this run in the execution transcript; the earlier PowerShell inventory records the name-to-ID mapping.

This probe currently establishes endpoint identity/state enumeration only. It does not yet establish formats, periods, shared-mode capture/render, loopback latency, process-tree capture, or physical tone/impulse behavior.
