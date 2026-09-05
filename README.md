# AudioRouter

AudioRouter is a planned Windows 11 application for visual audio routing, microphone processing, recording, and persistent virtual audio devices. A React/TypeScript/Vite interface, command-line client, and MCP adapter will all control the same Rust backend.

This repository currently contains specifications only. No audio engine, driver, application, or verified Windows build exists yet. “AudioRouter” is a working name.

Start with the [documentation index](docs/README.md), then the [product scope](docs/spec/01-product.md) and [reference workflows](docs/spec/02-workflows.md). Implement the [milestones](docs/spec/15-delivery.md) in order, starting with [M00: Windows feasibility](docs/milestones/M00-feasibility.md).

The first complete release must let a user send a processed microphone to Discord, send desktop audio separately to a game recorder, and hear the desired mix in headphones. The routes must be understandable on screen and fully configurable without the UI.

Development agents must read [AGENTS.md](AGENTS.md) and the [active plan](docs/plans/active/current.md). `AGENTS.md` is the canonical agent instruction file; do not create a case-only `agent.md` duplicate on Windows.
