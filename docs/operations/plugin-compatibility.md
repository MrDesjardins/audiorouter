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
- The canonical Steinberg `AGain` sample was built from the same pinned SDK;
  the official validator reported 94 passed and 0 failed, and AudioRouter's
  offline loader passed its main stereo effect class with finite processing,
  bounded parameter automation, and state round-trip. Its side-chain class
  also passed the explicit `--multi-bus` offline probe: its two-input/one-output
  layout was activated and processed with finite output. This is auxiliary-bus
  loader evidence only; the supervised realtime multi-bus worker path remains
  gated and the default one-input/one-output probe still rejects that class.

These are offline fixture results, not proof that every class or third-party
plugin is compatible with a realtime AudioRouter route. Some fixture classes
may return `E_NOTIMPL` from the optional VST3 processing lifecycle hook; the
loader tolerates only that specific result and still requires successful
processing, finite output, parameter checks, and state round-trip. Other
activation failures remain plugin-specific incompatibilities.

## Second-vendor probe

The official SDK `advanced-techniques-tutorial` was built as a disposable x64
VST3 bundle using Visual Studio 2026. Steinberg's validator reported 47 passed
and 0 failed tests, and identified the vendor as Steinberg Media Technologies,
but AudioRouter's offline loader rejected processor activation with
`0x80004001` (`E_NOTIMPL`). It is therefore recorded as an unsupported fixture,
not as a second-vendor compatibility pass. The sibling data-exchange tutorial
could not be built from the pinned checkout because its Windows source refers
to an unavailable `FDebugPrint` symbol. Both builds were disposable; no system
plugin link, plugin registration, or audio configuration was changed.

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
only: native editor window open/close, rights review, and release qualification
remain open. The repository-owned fixture additionally qualifies opaque
chunk-state save/restore and the legacy `main` export through the worker. The
host library now also has a
control-plane `EditorLifecycle` policy that separates editor open/close/failure
and deliberate retry from the processing generation. It does not create native
windows or claim editor compatibility; those remain a Windows/UI acceptance
gate.

Editor-open requests now carry a bounded opaque authorization token and the
expected owner process ID with the parent HWND. The worker rejects missing or
oversized authorization data, and the Windows editor thread verifies that the
live HWND still belongs to that process before dispatching `effEditOpen`.
`EditorParentAuthorizationIssuer` now derives the opaque capability from a
control-plane-held key, the HWND, and the owner PID; key storage and issuance
remain a control-plane/native-shell responsibility. It is not wired to the
preview WebView, so editor controls remain gated.

Worker sessions retain the latest validated plugin latency. A plugin may report
a changed bounded sample count at the negotiated sample rate; changing the
sample rate is rejected and cannot overwrite the prior value. This protects
control-plane compensation state, but does not substitute for measured
plugin-added latency or realtime graph evidence.

The VST2 host callback also reports the worker's negotiated sample rate and
block size to the legacy effect, rather than a fixed host default. This keeps
the native `effSetSampleRate`/`effSetBlockSize` setup and subsequent host
queries consistent while retaining the bounded worker boundary.

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
Callers with a known graph rate can use
`SupervisedWorkerProcess::spawn_with_sample_rate` or its verified variant;
rates are bounded to 8–192 kHz and are passed explicitly to the worker.

The repeatable native matrix currently covers all six local ReaPlugs effects
(ReaComp, ReaDelay, ReaEQ, ReaFIR, ReaGate, and ReaXcomp) at 44.1, 48, and
96 kHz (18 processing combinations). The explicitly selected installed
Pitchproof x64 VST2 binary has passed the same three processing rates.
These are compatibility observations only: they do not establish licensing
or redistribution rights, editor compatibility, or release qualification.

On 2026-09-09, the same contained-worker processing check was run directly
against the user-supplied installed ReaComp, ReaEQ, ReaDelay, and ReaGate x64
VST2 DLLs. All four passed at 44.1, 48, and 96 kHz (12 processing cases).
The check used process-local environment overrides and restored the prior
values afterward; it did not copy or register the DLLs or change machine audio
state. These results remain local compatibility evidence and do not establish
third-party rights, editor support, physical latency, or release qualification.
The remaining local ReaFIR and ReaXComp DLLs were then rechecked through the
same path at all three rates (six more cases), completing the 18-case
six-effect matrix.

Run the reproducible local qualification with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst3-sdk.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst2-reaplugs.ps1
```

An explicitly selected installed x64 VST2 effect can be qualified without
copying or registering it. The current machine also contains Pitchproof under
the system VST3 directory; inspection confirmed that its `pitchproof-x64.dll`
is VST2. Reproduce that bounded worker check with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst2-installed.ps1
```

The wrapper prints the binary SHA-256, restores any pre-existing
`AUDIOROUTER_VST2_FIXTURE` value, and never changes audio configuration. Its
processing and editor-containment checks both pass for the current fixture:
the editor enters `effEditOpen` but does not return within five seconds, so
the worker is terminated and reaped. This is contained editor-failure
evidence, not successful editor-window support. Qualification remains
fixture-specific and does not establish rights to redistribute the installed
binary.

The generic opt-in processing acceptance also attempts optional state
save/restore for a selected third-party effect. The separate deterministic
chunk-state regression is intentionally restricted to the repository-owned
fixture because it asserts that fixture's exact one-parameter transform; it is
not a generic claim about every VST2 effect's state model.

The sibling `pitchproof.dll` from the same installation is an x86 binary and
was rejected by read-only inspection as `unsupportedArchitecture`; it was not
loaded. This negative control confirms that the x64 VST2 gate does not bridge
or execute x86 plugins.

The bounded native-editor probe can be repeated with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst2-editor.ps1
```

This verifies timeout containment and worker termination for the local
fixtures; it does not qualify their native editor windows.

The VST2 wrapper runs each ignored DLL independently through the contained
worker test and restores any pre-existing `AUDIOROUTER_VST2_FIXTURE` and
`AUDIOROUTER_VST2_SAMPLE_RATE` values. For a mixed user directory containing
MIDI, utility, or streaming DLLs, pass
`-SkipIncompatibleCandidates`; each rejected x64 candidate is reported and the
remaining audio-effect candidates continue independently. The default mode
still fails on the first x64 candidate regression, so ordinary acceptance does
not hide a broken effect. The fixture directory is local-only and is not part
of source or release artifacts.

Chunk-state coverage can be run with the repository-owned fixture:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst2-state-fixture.ps1
```

The script compiles ignored x64 VST2 DLLs with `effFlagsProgramChunks`, plus a
`main`-only variant, and runs the verified worker acceptance for both. It
Processing and chunk-state restoration are covered at 44.1, 48, and 96 kHz;
the same run retains non-finite-output rejection and crash/hang containment.
It leaves no registered plugin or system audio changes.

See [SDK setup](sdk-setup.md) and [release notes](release-notes.md) for the
toolchain and current qualification boundaries.
