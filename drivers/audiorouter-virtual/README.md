# AudioRouter virtual audio driver prototype

This directory contains an AudioRouter-owned derivative of Microsoft's Simple
Audio Sample WDM/WaveRT virtual audio driver. The upstream source was pinned to
Windows-driver-samples commit `197ba2156a60e2b76fcd4820bae594223e91a1e9` and is
retained here with the Microsoft Public License in `LICENSE-MS-PL.txt`.

The derivative currently exposes the sample's two endpoint families (one render
speaker and one capture microphone array), with AudioRouter-owned package,
service, device, endpoint labels, and GUID identities. It is a build target for
the M03 virtual-device work, not a production driver or a claim that the full
AudioRouter bus lifecycle is implemented.

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

## Current limits and next integration work

- The prototype is x64 build-qualified; the active plan records the installed
  VS/WDK build and zero-error signability result. This is not production
  signing evidence.
- Production signing, catalog/release policy, clean-machine install,
  managed bus creation/rename/enable/
  disable/delete, and endpoint teardown are not implemented here.
- The secured control scaffold has direction-aware ownership leases and a
  bounded mapped-block ABI. Both sample WaveRT directions now call the bounded
  bridge helpers: render-source data is copied into the capture fill path and
  render DMA is published to the capture-sink path. The user-mode engine
  remains the owner of EQ, gate, compressor, limiter, delay, pitch,
  voice-chain, and meter processing. This is sample callback-wiring evidence;
  production PortCls ownership, managed bus lifecycle, loaded-driver
  transport, and measured callback behavior remain open.
- No driver installation or live endpoint test is authorized by this build
  script. Those gates require a reversible isolated-target procedure and must
  preserve the user's existing audio configuration.
