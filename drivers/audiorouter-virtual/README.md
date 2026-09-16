# AudioRouter virtual audio driver prototype

This directory contains an AudioRouter-owned derivative of Microsoft's Simple
Audio Sample WDM/WaveRT virtual audio driver. The upstream source was pinned to
Windows-driver-samples commit `197ba2156a60e2b76fcd4820bae594223e91a1e9` and is
retained here with the Microsoft Public License in `LICENSE-MS-PL.txt`.

The derivative currently exposes the sample's two endpoint families as
`AudioRouter - Desktop In` (render) and `AudioRouter - Voice Chat` (capture),
with AudioRouter-owned package, service, device, endpoint labels, and GUID
identities. It is a build target for
the M03 virtual-device work, not a production driver or a claim that the full
AudioRouter bus lifecycle is implemented.

The INF now also declares the `SWD\\AudioRouterVirtual` hardware-ID match used
by the planned Software Device API provisioner. This is only a driver-package
matching prerequisite: the repository does not yet create software devices,
persist their returned PnP instance IDs, or activate this package on the
current machine.

## Build-only workflow

From an x64 Developer PowerShell or an elevated PowerShell session with Visual
Studio and the WDK installed, run:

```powershell
.\build.ps1 -Configuration Release -Platform x64 -KeepOutput
```

The script locates MSBuild, verifies the WDK driver targets, builds the solution,
and reports the disposable package directory. It never installs the INF,
registers a device, changes test-signing or boot policy, changes an audio
default, or starts a driver service.

`UPSTREAM-README.md` preserves the source sample's original documentation. Its
deployment instructions are intentionally not an AudioRouter installation
procedure; do not use them on a development workstation.

## Explicit package lifecycle

After producing a package, an administrator may use the guarded lifecycle
entrypoint below on an isolated test system:

```powershell
.\manage.ps1 -Install -AllowDriverInstall -Inf .\path\to\AudioRouterVirtual.inf
.\manage.ps1 -Uninstall -AllowDriverInstall -Inf .\path\to\AudioRouterVirtual.inf
```

Installation records the exact `oem*.inf` name returned by `pnputil`; uninstall
requires that state file and removes only that package. Both switches are
required, the INF must be inside this driver package directory, and ambiguous
or missing state fails closed. State publication is staged in the same
directory; if it fails after installation, the script attempts an automatic
package rollback and reports if that compensation fails. The script does not change test-signing,
Secure Boot, HVCI, audio defaults, or endpoint selections. Do not run it on the
daily workstation until the isolated-target procedure and rollback evidence
are approved.

## Current limits and next integration work

- The prototype is x64 and ARM64 compile-qualified; the active plan records
  the installed VS/WDK build and zero-error signability result. This is not
  production signing evidence.
- Production signing, catalog/release policy, clean-machine qualification,
  managed bus creation/rename/enable/
  disable/delete, and endpoint teardown are not implemented here.
- The secured control scaffold has direction-aware ownership leases and a
  bounded mapped-block ABI. Both sample WaveRT directions now call the bounded
  bridge helpers: render-source data is copied into the capture fill path and
  render DMA is published to the capture-sink path. The user-mode engine
  remains the owner of EQ, gate, compressor, limiter, delay, pitch,
  voice-chain, and meter processing. This is sample callback-wiring evidence;
  production PortCls ownership, managed bus lifecycle, loaded-driver
  transport, and measured callback behavior remain open. The inherited
  diagnostic file writer is not invoked from the realtime callback because it
  takes locks and queues work items; bridge publication remains independent of
  that opt-in sample diagnostic feature.
- No driver installation or live endpoint test is authorized by this build
  script. Those gates require a reversible isolated-target procedure and must
  preserve the user's existing audio configuration.
