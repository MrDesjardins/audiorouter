# AudioRouter 0.1.0-dev qualification notes

This document describes the current development snapshot. It is not a signed
release and must not be presented as an installable Windows audio product.

## Scope and platform

- Target: Windows 11 x64.
- Portable control, storage, DSP, recording, CLI, UI, MCP, and plugin-worker
  foundations are implemented and covered by automated tests.
- The repository-local Steinberg VST3 SDK is pinned and verified at
  `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`.
- Native builds use Visual Studio Community 2026/MSVC 14.51.36231 and Windows
  SDK 10.0.28000.0; the installed WDK is 10.1.28000.2526.

## Verified in this qualification snapshot

- The locked Rust workspace passes 466 unit/integration tests, all doc-tests,
  formatting, and strict Clippy.
- M07 headless acceptance passes 25 CLI tests, 2 MCP interoperability tests,
  87 control tests, 39 plugin-host tests, 8 worker-process tests, and strict
  Clippy.
- M08 disposable artifact preparation creates and verifies unsigned x64 CLI
  and plugin-worker artifacts, SBOM metadata, notices, checksums, and a
  manifest, then removes the temporary output.
- VST3 SDK acceptance passes 51 SDK self-tests, 1,598 official validator tests
  with 0 failures, and the offline native mda fixture loader.
- The native WASAPI probes qualify shared capture across 13 endpoints,
  process-loopback include/exclude and controlled attribution, silent render
  lifecycle, and endpoint timing baselines. The guarded production Rust adapter
  smoke also passes bounded capture plus zero-valued `submit_bytes` render
  submission while preserving the media-device snapshot.

## Known limitations

- Rust capture retries the exact observed event-callback `E_INVALIDARG` with a
  fresh native-compatible polling client; busy-device and permission failures
  remain distinct and fail closed. The guarded live adapter smoke qualifies
  bounded adapter capture/render lifecycle, but is not evidence of complete
  AudioRouter graph routing.
- Realtime graph scheduling, physical acoustic latency, clock drift, and
  hardware/endurance qualification are incomplete.
- The managed virtual-audio driver is not included, installed, signed, or
  registered. Virtual-device lifecycle remains an honest unavailable
  capability.
- There is no production installer, signed package, upgrade/rollback proof,
  clean-machine qualification, or Secure Boot/Memory Integrity driver result.
- Plugin discovery and worker protocol protections are implemented, but full
  filesystem/network OS sandboxing, arbitrary plugin execution, and a broad
  third-party compatibility matrix remain open.
- Sign-in startup, tray/background lifecycle, and manual accessibility and
  first-time-user qualification remain open.

## Safety and recovery

The acceptance commands are configuration-safe: they do not change default
devices, volume, mute, privacy settings, drivers, or endpoint state. See the
[development quickstart](quickstart.md), [headless runbook](headless-runbook.md),
and [release qualification checklist](release-qualification.md) for commands,
backup/restore expectations, and recovery boundaries.

Do not install a third-party virtual cable and describe it as an AudioRouter
driver. Do not treat an unsigned manifest as publication-ready. Production
signing, driver installation, and any audio-stream experiment require separate
authorization and the appropriate isolated test environment.
