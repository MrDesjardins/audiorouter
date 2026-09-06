# Plugin compatibility snapshot

AudioRouter's supported plugin boundary is VST3 x64. VST2, x86, arbitrary
binary execution, and a universal commercial-plugin guarantee are outside the
current qualification.

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
return bounded identity/compatibility metadata, and do not load or execute
plugin code. Directory scan roots and candidate binaries are checked against
canonical/reparse-point boundaries. Invalid candidates remain visible as
inspection errors.

The future worker path has bounded frames, deadlines, heartbeats, shared-memory
layout checks, failure quarantine, and process cleanup. Full OS-level
filesystem/network sandboxing, native plugin execution, editor windows, and a
multi-vendor compatibility matrix remain open release work.

## Reporting a plugin result

Record the plugin format, architecture, vendor, class identity, exact host
operation, result code, and whether the test was inspection-only or offline
processing. Do not add a plugin to a live route based only on file presence or
successful metadata inspection. A failed or unavailable effect must leave a
protected voice path silent or explicitly unavailable, not silently fall back
to dry audio.

Run the reproducible local qualification with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m06-vst3-sdk.ps1
```

See [SDK setup](sdk-setup.md) and [release notes](release-notes.md) for the
toolchain and current qualification boundaries.
