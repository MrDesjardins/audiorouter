# Plugin compatibility snapshot

AudioRouter's release-qualified plugin boundary remains VST3 x64. A gated
Windows-only VST2 x64 worker adapter is available for explicitly selected
user-installed audio-effect DLLs; x86, arbitrary binary execution, and a
universal commercial-plugin guarantee remain outside qualification.

## Verified fixture

The repository-local Steinberg VST3 SDK is pinned at
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`. Its bundled `mda-vst3` fixture was
built and checked with the installed Visual Studio 2026 toolchain:

- 68 factory classes were enumerated.
- The official validator reported 1,598 passed and 0 failed.
- The offline loader processed the `mda Ambience` audio-effect class with a
  finite stereo block, verified bounded parameter automation, and round-tripped
  component state.
- Additional offline loader checks passed for `mda BeatBox`, `mda Combo`, and
  `mda Delay`.

These are offline fixture results, not proof that every class or third-party
plugin is compatible with a realtime AudioRouter route. Some fixture classes
reject processor activation with `E_NOTIMPL`; that result is surfaced as a
plugin-specific incompatibility rather than converted into success.

## Inspection and execution boundary

`plugins scan` and `plugins inspect` accept explicitly selected absolute paths,
return bounded identity/compatibility metadata, including best-effort VST3
vendor, version, and class IDs read from `moduleinfo.json`, and do not load or
execute plugin code. Directory scan roots and candidate binaries are checked
against canonical/reparse-point boundaries. Invalid candidates remain visible
as inspection errors. Missing or malformed optional module metadata leaves
those fields empty and does not turn a binary into a compatibility claim.

The worker path has bounded frames, deadlines, heartbeats, shared-memory layout
checks, failure quarantine, and process cleanup. Full OS-level
filesystem/network sandboxing and a multi-vendor compatibility matrix remain
open release work. The native VST2 adapter has passed the local six-binary
ReaPlugs worker matrix for bounded load, processing, parameters, latency, state
capability, editor capability discovery, and shutdown. This is fixture evidence
only: native editor window open/close, chunk-capable state, rights review, and
release qualification remain open. The host library now also has a
control-plane `EditorLifecycle` policy that separates editor open/close/failure
and deliberate retry from the processing generation. It does not create native
windows or claim editor compatibility; those remain a Windows/UI acceptance
gate.

Worker sessions retain the latest validated plugin latency. A plugin may report
a changed bounded sample count at the negotiated sample rate; changing the
sample rate is rejected and cannot overwrite the prior value. This protects
control-plane compensation state, but does not substitute for measured
plugin-added latency or realtime graph evidence.

When a worker rejects a latency update, the process adapter returns the
worker's structured protocol failure so callers can quarantine or retry it
according to the normal failure policy.

Worker failure codes are limited to 128 bytes; empty or oversized codes are
rejected before they can enter the process boundary or diagnostic state.

The worker protocol also carries opaque, integrity-checked state assets. State
payloads are capped at 512 KiB, and the disposable worker supports explicit
restore/save round trips. This validates transport and persistence boundaries;
vendor-specific VST3 state serialization still requires the native plugin host.
Callers with a known schema can use `restore_state_for_version` to reject a
version mismatch before sending state to the worker.
State version `0` is invalid at both the asset and worker-wire boundaries.

Worker parameter descriptors are bounded to 256 entries and 128-byte titles;
IDs must be unique and normalized defaults/ranges must be finite and within
`0..=1`. The disposable worker reports an empty catalog in its ordinary mode
and a two-entry `test-fixtures` catalog for process-level wire regression.
Native plugin parameter mapping remains separate and is not implied by the
fixture.

The opt-in `test-fixtures` Cargo feature adds deterministic worker modes for
crash, hang, malformed output, a non-empty parameter catalog, and bounded
dynamic-latency updates. The process tests prove bounded reaping,
timeout kill, supervisor containment, and reader-side rejection. These modes are test fixtures only;
they are excluded from ordinary builds and do not represent third-party VST3
execution or full OS filesystem/network sandboxing.

## Reporting a plugin result

Record the plugin format, architecture, vendor, class identity, exact host
operation, result code, and whether the test was inspection-only or offline
processing. Do not add a plugin to a live route based only on file presence or
successful metadata inspection. A failed or unavailable effect must leave a
protected voice path silent or explicitly unavailable, not silently fall back
to dry audio.

Before a worker launch, callers can invoke `PluginIdentity::verify_current` with
the approved configured roots. It rechecks the exact canonical path, binary
fingerprint, format, architecture, and size, and fails closed if the file was
replaced or moved outside the grant. It never selects a substitute path.
`SupervisedWorkerProcess::spawn_verified` composes that check with supervised
worker creation for callers that need one launch operation.

Run the reproducible local qualification with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst3-sdk.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst2-reaplugs.ps1
```

The bounded native-editor probe can be repeated with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst2-editor.ps1
```

This verifies timeout containment and worker termination for the local
fixtures; it does not qualify their native editor windows.

The VST2 wrapper runs each ignored DLL independently through the contained
worker test and restores any pre-existing `AUDIOROUTER_VST2_FIXTURE` value.
The fixture directory is local-only and is not part of source or release
artifacts.

Chunk-state coverage can be run with the repository-owned fixture:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst2-state-fixture.ps1
```

The script compiles an ignored x64 VST2 DLL with `effFlagsProgramChunks`, runs
the verified worker acceptance, and leaves no registered plugin or system
audio changes.

See [SDK setup](sdk-setup.md) and [release notes](release-notes.md) for the
toolchain and current qualification boundaries.
