# Release qualification and recovery checklist

AudioRouter is not a releasable Windows installer yet. The portable control
plane, CLI, MCP adapter, DSP, recording, and plugin-worker foundations are
implemented and tested, but native routing, the owned virtual-device driver,
production signing, packaging, and clean-machine qualification remain open.

## Current verified artifacts

The reproducible preparation flow is:

    powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\release\prepare-artifacts.ps1 -OutputDirectory <new-absolute-directory>
    powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\release\verify-artifacts.ps1 -ReleaseDirectory <prepared-directory>

Preparation requires a clean worktree, locked Cargo inputs, and a new output
directory. It produces unsigned x64 CLI and disposable-worker artifacts,
locked Cargo SBOM metadata, and checksums. It does not produce an installer or
install a driver.

## Before any installation

1. Back up the SQLite database and recording files to separate, new
   destinations using the headless runbook.
2. Stop AudioRouter sessions through the authorized control surface.
3. Confirm the package, driver, and configuration versions are compatible.
4. Review the install preview and the required elevation scope.
5. Keep unrelated audio drivers and devices out of the change set.

The current repository has no production installer or owned virtual-device
driver to install. Do not treat VB-Audio, Voicemeeter, Sonar, or another
existing virtual device as an AudioRouter release artifact.

## Recovery and uninstall expectations

If a future installer fails, retain the SQLite database and recordings, restore
only to a new validated database destination, and do not delete unrelated
audio devices. A future uninstall must preview owned endpoints, startup
registration, privileged helpers, and control-pipe cleanup separately from
recording retention. Permanent recording deletion is never an implicit
uninstall action.

## Known release blockers

- Production-signed virtual-device driver and normal Secure Boot/Memory
  Integrity qualification.
- Native end-to-end routing, latency, drift, restart, and hardware evidence.
- Signed binaries/packages, installer elevation behavior, upgrade/rollback, and
  clean-machine testing.
- Full plugin worker sandbox enforcement and the tested compatibility matrix.
- Accessibility/usability and first-time-user qualification on the declared
  reference hardware.

See the [headless runbook](headless-runbook.md), [SDK setup](sdk-setup.md), and
[M08 evidence](../plans/active/evidence/M08-release.md) for commands and
measured boundaries.
