# AudioRouter 0.1.0-dev qualification notes

This document describes the current development snapshot (2026-09-14). It is not a signed
release and must not be presented as an installable Windows audio product.

## Scope and platform

- Target: Windows 11 x64.
- Portable control, storage, DSP, recording, CLI, UI, MCP, and plugin-worker
  foundations are implemented and covered by automated tests. The UI has 188
  passing tests and includes a visual graph editor, backend-bound built-in
  processor/preset editing, explicit endpoint binding, and route provenance.
- The repository-local Steinberg VST3 SDK is pinned and verified at
  `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`.
- Native builds use Visual Studio Community 2026/MSVC 14.51.36231 and Windows
  SDK 10.0.28000.0; the installed WDK is 10.1.28000.2526.

## Verified in this qualification snapshot (2026-09-13)

- The locked Rust workspace passes its current package tests and doc-tests,
  formatting, and strict Clippy; the guarded native tests remain explicitly
  opt-in.
- M07 headless acceptance passes 33 CLI tests, 3 MCP interoperability tests,
  152 control tests (2 guarded live tests ignored), 70 plugin-host tests, 13
  worker-process tests, and 17 shell tests.
- M08 disposable artifact preparation creates and verifies unsigned x64 CLI,
  native-shell, and plugin-worker artifacts, SBOM metadata, notices, checksums,
  and a manifest, validates bounded PE headers for x64 executables, then removes
  the temporary output.
- A transient Tauri 2.11.4 NSIS bundler smoke also succeeds with `--no-sign`;
  the generated installer is removed and is not a production or installability
  qualification result.
- VST3 SDK acceptance passes 51 SDK self-tests, 1,598 official validator tests
  with 0 failures, and the offline native mda fixture loader.
- A Windows-only, gated native x64 VST2 adapter is verified with repository-owned
  `VSTPluginMain` and legacy `main` fixtures, including chunk-state restoration,
  invalid-output rejection, crash containment, and hang reaping. This does not
  grant redistribution rights or make the VST2 extension release-qualified.
- Windows endpoint and application discovery failures preserve stable audio
  categories, unsigned HRESULTs, retryability, and remediation guidance;
  `AUDCLNT_E_DEVICE_IN_USE` remains distinct from `E_INVALIDARG`.
- The native WASAPI probes qualify shared capture across 13 endpoints,
  process-loopback include/exclude and controlled attribution, silent render
  lifecycle, and endpoint timing baselines. The guarded production Rust adapter
  smoke also passes bounded capture plus zero-valued `submit_bytes` render
  submission while preserving the media-device snapshot.
- The guarded control-owned native VB-Cable route qualifies exact capture and
  render endpoint binding, processor-bearing graph activation, bounded pump
  delivery, and clean start/stop. The latest run observed 24,000 captured
  frames, 187 processed quanta, and 23,936 rendered frames; it did not change
  defaults, volume, mute, privacy, drivers, or persistent audio settings.
- The attended Tauri shell transport acceptance reaches the WebView's native
  RPC command and authenticated backend through a disposable pipe/database;
  manual visual accessibility and scaling review remains separate.

## Known limitations

- Rust capture retries the exact observed event-callback `E_INVALIDARG` with a
  fresh native-compatible polling client; busy-device and permission failures
  remain distinct and fail closed. The guarded live adapter smoke qualifies
  bounded adapter capture/render lifecycle, but is not evidence of complete
  AudioRouter graph routing.
- Production callback scheduling, physical acoustic latency, clock drift, and
  hardware/endurance qualification are incomplete. The current VB-Cable pump
  is a guarded transitional control-plane delivery path, not callback timing
  evidence.
- The managed virtual-audio driver is not included, installed, signed, or
  registered. Virtual-device lifecycle remains an honest unavailable
  capability.
- There is no production installer, signed package, upgrade/rollback proof,
  clean-machine qualification, or Secure Boot/Memory Integrity driver result.
- Plugin discovery and worker protocol protections are implemented, but full
  filesystem/network OS sandboxing, arbitrary plugin execution, and a broad
  third-party compatibility matrix remain open. The VST2 editor, rights, and
  release-qualification gates also remain open.
- Sign-in startup registration remains unverified in an attended rollback run;
  tray/background lifecycle code is covered by shell tests, while manual
  accessibility, scaling, and first-time-user qualification remain open.

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
