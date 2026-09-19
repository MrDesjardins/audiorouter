# AudioRouter 0.1.0-dev qualification notes

This document describes the current development snapshot (2026-09-17). It is not a signed
release and must not be presented as an installable Windows audio product.

## Current scope boundary — 2026-09-17

This development track relies on an existing VB-Cable pair, Voicemeeter, and
physical WASAPI endpoints for user-mode routing and tool integration. The
AudioRouter-owned kernel driver is deliberately deferred because production
signing and trusted installation requirements are not currently available.
Driver, PortCls, signed-package, and clean-machine gates remain open and are
not implied by the VB-Cable evidence below.

## Scope and platform

- Target: Windows 11 x64.
- Portable control, storage, DSP, recording, CLI, UI, MCP, and plugin-worker
  foundations are implemented and covered by automated tests. The UI has 257
  passing tests and includes a visual graph editor, backend-bound built-in
  processor/preset editing, explicit endpoint binding, and route provenance.
- The repository-local Steinberg VST3 SDK is pinned and verified at
  `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`.
- Native builds use Visual Studio Community 2026/MSVC 14.51.36231 and Windows
  SDK 10.0.28000.0; the installed WDK is 10.1.28000.2526.

## Current requalification refresh — 2026-09-17

The current tree requalification supersedes stale counts in older snapshot
paragraphs: M05 passes typecheck, 19 UI test files/257 tests, and a disposable
production build; M07 passes 36 CLI, 3 MCP, 177 control tests with 3 guarded
live tests ignored, 70 plugin-host, and 13 worker-process tests; and the full
locked workspace passes its package suites and doc-tests. The explicit
VB-Cable/Focusrite-to-VB-Cable/DELL multi-route also passed with 47,040
captured and 47,232 rendered frames. These are development evidence only. M08 artifact
preparation is currently blocked by the required clean Git worktree, and no
driver, signing, accessibility, physical-latency, sandbox, or publication
claim is implied.
The disposable unsigned NSIS smoke was rerun in the elevated build context,
produced a 4,888,262-byte x64 bundle, verified manifest integrity, and
removed the bundle afterward. This strengthens packaging evidence only; the
clean-tree artifact flow still refuses the current dirty checkout.

The guarded application-capture lifecycle also passed for the observed
Voicemeeter and Zoom process identities in both include and exclude modes,
including same-process worker restart and unchanged media-device state. The
refreshed Windows shell suite passed all 29 tests for notification mapping,
bounded delivery, safe-mode recovery, startup ownership, and tray authority.
These results do not promote real OS power-transition delivery, native reopen,
or universal process-loopback compatibility.
The latest exact-endpoint native run also toggled privacy mute during active
processing and verified the control state transitions, while the current Zoom
process passed exclude-mode capture with worker restart. No defaults, volume,
mute, privacy, or persistent audio configuration was changed by these checks.

## Verified in this qualification snapshot (2026-09-16)

The guarded integrated acceptance chain passed at commit `1e9c60ce`, including
the x64/ARM64 source gates, 234 UI tests, unsigned artifact/NSIS smoke,
traceability, and documentation validation. The generated installer was
4,760,908 bytes and was removed after the disposable smoke test.

- The locked Rust workspace passes its current package tests and doc-tests,
  formatting, and strict Clippy; the guarded native tests remain explicitly
  opt-in.
- M07 headless acceptance passes 36 CLI tests, 3 MCP interoperability tests,
  173 control tests (2 guarded live tests intentionally ignored), 70
  plugin-host tests, 13 worker-process tests, and 26 shell tests. The shell
  supervisor persists crash markers and keeps a stopped control plane
  available in safe mode.
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
  delivery, and clean start/stop. The latest 500 ms run observed 23,520
  captured frames, 23,424 rendered frames, and 183 processed quanta. It did
  not change defaults, volume, mute, privacy, drivers, or persistent audio
  settings.
- Shell-owned backend recovery persists crash markers and keeps a stopped
  control plane available after the safe-mode threshold, allowing an
  authorized operator to inspect and clear the latch without reopening native
  endpoints.
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
- Sign-in startup registration is verified at the native helper boundary, but
  an attended rollback run, manual accessibility, scaling, and first-time-user
  qualification remain open.

The native startup helper's HKCU Run boundary was requalified on 2026-09-14
with an opt-in enable/disable round trip. The test temporarily created only
the current user's `AudioRouter` value, verified ownership, removed it, and
confirmed the value was absent afterward. The native helper is qualified; an
attended shell/accessibility and clean-machine startup review is still
required before release claims.

The same guarded test was reattempted on 2026-09-17 in the current host and
was denied by the host's HKCU registry policy (`WIN32_ERROR(5)`) before any
write. A read-only query confirmed that the `AudioRouter` value was absent;
this is an environment validation blocker, not evidence of a successful current
round trip.

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
