# AudioRouter

AudioRouter is a Windows 11 application under active development for visual audio routing, microphone processing, recording, and integration with existing virtual audio devices. The repository contains a Rust control/data-plane implementation, React/TypeScript/Vite UI shell, CLI, authorized MCP stdio adapter, recording/DSP processing, and isolated plugin workers.

Portable behavior and guarded Windows user-mode routing through existing
VB-Cable, Voicemeeter, and physical WASAPI endpoints are covered by tests and
live evidence, but this is not yet a release: attended UI acceptance, physical
latency/endurance, managed virtual-device driver lifecycle, production signing,
packaged installation, and clean-machine qualification remain open. Ordinary
tests deliberately do not change machine audio configuration. See the [active
evidence](docs/plans/active/current.md) and [M08 release evidence](docs/plans/active/evidence/M08-release.md) for exact boundaries.

Start with the [documentation index](docs/README.md), then the [product scope](docs/spec/01-product.md), [reference workflows](docs/spec/02-workflows.md), and [active plan](docs/plans/active/current.md). Release work follows the [milestones](docs/spec/15-delivery.md); unresolved native and signing gates are recorded rather than presented as completed.

The first complete release must let a user send a processed microphone to Discord, send desktop audio separately to a game recorder, and hear the desired mix in headphones. The routes must be understandable on screen and fully configurable without the UI.

Development agents must read [AGENTS.md](AGENTS.md) and the [active plan](docs/plans/active/current.md). `AGENTS.md` is the canonical agent instruction file; do not create a case-only `agent.md` duplicate on Windows.

## Development SDK setup

The VST3 SDK is source-distributed; the GitHub repository is the SDK and has
no separate installer. AudioRouter keeps a pinned, repository-local checkout
under the ignored `third_party/vst3sdk` directory. From PowerShell, download
and verify it with:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\m06-vst3-sdk\install.ps1
```

The native Windows work uses the Visual Studio MSVC toolchain plus the Windows
SDK/WDK installed through Visual Studio or Build Tools. See
[SDK setup](docs/operations/sdk-setup.md) for the pinned revision, required
headers, and verification commands. Setup does not install drivers, register
plugins, or change audio devices or other machine audio settings.
