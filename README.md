# AudioRouter

AudioRouter is a Windows 11 application under active development for visual audio routing, microphone processing, recording, and persistent virtual audio devices. The repository contains a Rust control/data-plane foundation, React/TypeScript/Vite UI shell, CLI, authorized MCP stdio adapter, recording/DSP primitives, and isolated plugin-worker protocol groundwork.

Portable behavior is covered by automated tests, but this is not yet a release: native end-to-end audio routing, managed virtual-device driver lifecycle, production signing, packaged installation, and hardware/endurance qualification remain open. Ordinary tests deliberately do not change machine audio configuration. See the [active evidence](docs/plans/active/current.md) and [M08 release evidence](docs/plans/active/evidence/M08-release.md) for exact boundaries.

Start with the [documentation index](docs/README.md), then the [product scope](docs/spec/01-product.md), [reference workflows](docs/spec/02-workflows.md), and [active plan](docs/plans/active/current.md). Release work follows the [milestones](docs/spec/15-delivery.md); unresolved native and signing gates are recorded rather than presented as completed.

The first complete release must let a user send a processed microphone to Discord, send desktop audio separately to a game recorder, and hear the desired mix in headphones. The routes must be understandable on screen and fully configurable without the UI.

Development agents must read [AGENTS.md](AGENTS.md) and the [active plan](docs/plans/active/current.md). `AGENTS.md` is the canonical agent instruction file; do not create a case-only `agent.md` duplicate on Windows.
