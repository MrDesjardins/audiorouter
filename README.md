# AudioRouter

**See your sound. Shape it. Send it anywhere.**

AudioRouter is a visual audio router for Windows 11. Draw your audio on a
canvas: plug a microphone into a noise gate, an EQ and a compressor, send the
result to Discord, monitor it in your headphones and record it, all at once.
Stream game audio from your gaming PC to your streaming PC over your home
network. Every route is a picture you can read, and every setting is live.

**[Website](https://audiorouter.org)** ·
**[Download](https://github.com/MrDesjardins/audiorouter/releases/latest)** ·
[Manual](https://audiorouter.org/manual.html) ·
[Audio tools](https://audiorouter.org/tools.html) ·
[Videos](#videos) ·
[For developers](#for-developers)

![A streamer voice chain: microphone through a noise gate, denoise, a four-point EQ, a compressor and a limiter, sent to Discord, the headphones and a recorder at the same time](docs/images/streamer-voice-chain.png)

## Get started

1. **Download** `AudioRouter_<version>_x64-setup.exe` from the
   [latest release](https://github.com/MrDesjardins/audiorouter/releases/latest).
   It is the only file you need: the app, its audio engine, the command-line
   tool and the Stream Deck plugin are all inside.
2. **Install** it. It installs for your Windows account only and needs no
   administrator rights. Windows may warn that the installer is unsigned;
   choose **More info**, then **Run anyway**.
3. **Open AudioRouter** and allow it to use your audio devices when you first
   press **Play**.

Requirements: Windows 11 x64. To send AudioRouter's sound into another app
(Discord, OBS, a game's voice chat), install a virtual cable such as
[VB-Cable](https://vb-audio.com/Cable/). You don't need one to route to your
speakers or headphones.

### Your first route in one minute

1. In **Tools**, add an **Input device** and an **Output device**.
2. Select each one and choose the real device (your microphone, your
   headphones) in **Properties**.
3. Drag from the input's orange connector to the output's blue connector.
   Add a **Noise gate** or **Advanced EQ** in between if you like.
4. Press **Play**. Speak, watch the line light up, and adjust settings live.
5. Press **Save** to keep the route. **Session → Save to file…** backs it up.

The [manual](https://audiorouter.org/manual.html) walks through plugins,
recording, application capture and two-PC streaming.

## Videos

| Video | Watch |
| --- | --- |
| AudioRouter overview (2 minutes) | [Watch on YouTube](https://youtu.be/IHitHqpi87s) |
| Your first route | [Watch on YouTube](https://youtu.be/3--AnDZZXzA) |
| Streaming audio between two PCs | Coming soon |

## What it does

- **One canvas instead of five control panels.** Sources on the left, outputs
  on the right, processing in between. The lines show where sound flows and
  pulse with its level while it plays.
- **Studio processing for your voice.** Noise gate, denoise, de-hum,
  de-click, a 16-point parametric EQ, compressor, limiter, delay, pitch and
  more: 30+ built-in [audio tools](https://audiorouter.org/tools.html).
- **Your VST plugins, safely.** Load x64 VST2 effects (for example the free
  ReaPlugs) and qualified VST3 effects, and open a plugin's own editor while
  audio plays. Each plugin runs in its own process, so a crashing plugin
  cannot take your route down with it.
- **One source, many destinations.** Send a processed microphone to Discord,
  OBS, your headphones and a recorder at the same time. Mix several sources
  into one with the Mixer.
- **Two-PC streaming built in.** Network Send and Network Receive carry audio
  between computers on your home network, with no extra software.
- **Capture one application.** Route just the game, just the browser or just
  Spotify instead of the whole desktop.
- **Record while you route**, with a recording library.
- **Sessions you can switch and move.** Keep several setups ("Streaming",
  "Podcast", "Late-night gaming"), switch between them, and save one to a
  single `.audiorouter` file to open on another PC.
- **Stream Deck keys** for toggles, privacy mute, Play/Stop, session switching
  and recording. Install the plugin from the app's **API** tab; see
  [Stream Deck controls](https://audiorouter.org/stream-deck.html).
- **Private by design.** No cloud, no account, no telemetry. AudioRouter never
  changes your Windows default devices or volumes, never switches to another
  microphone behind your back, and a failed effect on your voice path goes
  silent instead of leaking unprocessed audio.
- **Scriptable.** A local [HTTP API](https://audiorouter.org/api.html), a
  command line, and an [MCP server](https://audiorouter.org/mcp.html) for AI
  assistants, all permission-controlled.

## See it in action

**Two PCs, one stream.** The gaming PC mixes game audio with a cleaned-up
microphone and sends the mix to the streaming PC, while you keep the game in
your headphones.

![Gaming PC session: game audio and a denoised, EQ'd microphone mixed and sent to 192.168.1.50 with Network Send, plus a headphone monitor](docs/images/gaming-pc-network-send.png)

**The streaming PC receives it**, trims the level, feeds OBS through a virtual
cable, and keeps a backup recording. Light and high-contrast themes are
available too.

![Streaming PC session in the light theme: Network Receive, a trim gain, a level meter, an output to OBS and a backup recorder, with the Session file panel](docs/images/streaming-pc-light.png)

**Shape your voice precisely.** The Advanced EQ has up to 16 points you drag
on a live frequency-response curve, or type exactly.

![Advanced EQ properties: a high-pass at 90 Hz, a cut at 250 Hz, a presence boost at 3.2 kHz and a high shelf, on a frequency-response graph](docs/images/advanced-eq.png)

## Ideas to try

| Goal | Route |
| --- | --- |
| Clean voice for Discord | Microphone → Noise gate → Denoise → EQ → Compressor → Output *CABLE Input*; pick *CABLE Output* as the microphone in Discord |
| Hear yourself while streaming | Split the same chain to your headphones |
| Podcast with a safety copy | Add a Recorder branch next to the live output |
| Game audio to a second PC | Gaming PC: Game → Network Send · Streaming PC: Network Receive → Output to OBS |
| Stream only the game | Application source (the game) → Volume → Output |
| Clear Rainbow Six Siege footsteps | See the [Siege guide](https://audiorouter.org/siege.html) |
| Try an effect without risk | Toggle Bypass or Enabled on any tool while it plays |

## Help and status

- **Website and manual:** [audiorouter.org](https://audiorouter.org), the
  [basic manual](https://audiorouter.org/manual.html) and the in-depth
  [user guide](docs/operations/quickstart.md).
- **Release notes:** [Releases page](https://audiorouter.org/releases.html)
  or [GitHub Releases](https://github.com/MrDesjardins/audiorouter/releases).
- **Problems:** open an [issue](https://github.com/MrDesjardins/audiorouter/issues).
  In the app, the **Logs** tab has **Open logs folder** for the files to
  attach.

AudioRouter is an **early preview**. Known limits: the app and installer are
not signed yet; clock drift between two different devices is not corrected
(rare, regular clicks on very long sessions); clean-machine, upgrade and
uninstall testing is still open. Releases use existing virtual cables such as
VB-Cable. AudioRouter's own virtual cable is in development and not shipped
yet. See [distribution status](docs/operations/distribution.md) for details.

---

## For developers

AudioRouter is a Rust workspace (control plane, real-time engine, WASAPI
audio, plugin host, storage, transport, CLI) plus a React/TypeScript
interface in a [Tauri](https://tauri.app) desktop shell. It builds and runs
on Windows 11 x64 only.

| Folder | What is in it |
| --- | --- |
| `crates/` | Rust workspace: `domain`, `control`, `engine`, `dsp`, `windows-audio`, `plugin-host`, `storage`, `transport`, `recording`, `protocol`, `cli`, `driver-helper` |
| `src-tauri/` | Desktop shell (window, tray, embedded backend, local HTTP API) |
| `ui/` | React/TypeScript interface, unit tests (Vitest) and browser tests (Playwright) |
| `contracts/` | Generated TypeScript API contracts and drift checks |
| `tools/streamdeck/` | Stream Deck plugin |
| `drivers/` | AudioRouter virtual cable driver (in development) |
| `site/` | The [audiorouter.org](https://audiorouter.org) website, deployed by `.github/workflows/pages.yml` |
| `tools/release/` | Release build, verification and draft-publication scripts |
| `docs/` | Specifications, plans, evidence and operations guides |

### Build from source

Install on a Windows 11 x64 PC:

| Tool | Where | Notes |
| --- | --- | --- |
| Visual Studio 2022 Build Tools | [visualstudio.microsoft.com](https://visualstudio.microsoft.com/downloads/) | Select **Desktop development with C++** (MSVC and the Windows SDK) |
| Rust (rustup) | [rustup.rs](https://rustup.rs) | `rust-toolchain.toml` pins 1.96.0; rustup installs it on the first build |
| Node.js | [nodejs.org](https://nodejs.org) | 22 LTS or newer |
| Git | [git-scm.com](https://git-scm.com) | |

WebView2, which draws the interface, is already part of Windows 11. Then, in
PowerShell:

```powershell
git clone https://github.com/MrDesjardins/audiorouter.git
cd audiorouter

# The interface
npm.cmd ci --prefix contracts
npm.cmd ci --prefix ui
npm.cmd run build --prefix ui

# The app, its plugin worker and its command line, in one folder
$env:CARGO_TARGET_DIR = "$PWD\target\app"
cargo build --release --manifest-path src-tauri/Cargo.toml --features custom-protocol
cargo build --release -p audiorouter-plugin-host --bin audiorouter-plugin-worker
cargo build --release -p audiorouter-cli
```

The first build takes several minutes. The three programs end up in
`target\app\release`. To run them like an installed copy:

```powershell
$app = "$env:LOCALAPPDATA\Programs\AudioRouter"
New-Item -ItemType Directory -Force $app | Out-Null
Copy-Item target\app\release\audiorouter-shell.exe, target\app\release\audiorouter-plugin-worker.exe, target\app\release\audiorouter-cli.exe $app
```

Start `audiorouter-shell.exe` from that folder. Run one copy of AudioRouter
at a time; close it before copying a new build over it. Sessions are stored
in `%LOCALAPPDATA%\AudioRouter`.

To build the installer exactly as a release does, use
`tools\release\prepare-artifacts.ps1`; see
[release qualification](docs/operations/release-qualification.md).

### Checks before you commit

CI (`.github/workflows/ci.yml`) runs on Windows for every push to `main` and
every pull request, and fails on any of these:

```powershell
cargo fmt --all -- --check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm.cmd --prefix ui test
npm.cmd --prefix ui run e2e   # Playwright in Edge, a separate CI job
```

Tests that need real audio devices are `#[ignore]`d; run them on a PC with
endpoints using `cargo test -p audiorouter-windows-audio -- --ignored`.

When dependencies change, a Linux job checks them against `deny.toml`
(RustSec advisories, licenses, crate sources) and runs
`npm audit --omit=dev` for `ui`, `contracts` and `tools/streamdeck`; run
`cargo deny check` and
`cargo deny --manifest-path src-tauri/Cargo.toml check` locally. Dependabot
proposes weekly updates (`.github/dependabot.yml`).

Format with `cargo fmt --all` and
`cargo fmt --manifest-path src-tauri/Cargo.toml`. To format staged Rust files
automatically at commit time, run once per clone:
`git config core.hooksPath .githooks`. More in
[CONTRIBUTING.md](CONTRIBUTING.md).

### Working on the project

- **Start here:** the [documentation index](docs/README.md), the
  [active plan](docs/plans/active/current.md) and the
  [specifications](docs/spec/01-product.md).
- **AI agents:** read [AGENTS.md](AGENTS.md), the canonical agent
  instructions (do not add a case-only `agent.md` copy on Windows). Claude
  Code sessions format each edited Rust file automatically
  (`.claude/settings.json`).
- **API:** [API reference](docs/operations/api-reference.md) for the command
  line, JSON-RPC and MCP, and the [local HTTP API](docs/operations/local-http-api.md).
- **Plugins:** [plugin compatibility](docs/operations/plugin-compatibility.md).
  Only VST3 fixture and validator work needs the Steinberg VST3 SDK (building
  the app does not):
  `powershell -ExecutionPolicy Bypass -File .\tools\m06-vst3-sdk\install.ps1`
  (see [SDK setup](docs/operations/sdk-setup.md)).
- **README screenshots:** regenerate with `AUDIOROUTER_README_SHOTS=1` and
  `npx playwright test e2e/readme-screenshots.pw.ts` in `ui`, after
  `cargo build --example e2e_backend -p audiorouter-control`.
- **Website:** edit `site/`; changes deploy to audiorouter.org when they
  reach `main` (see [site/README.md](site/README.md)).

## License

Copyright (C) 2026 Patrick Desjardins.

AudioRouter is free and open-source software under the
[GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`). You may use,
study, share and modify it. If you distribute AudioRouter or a modified
version, you must publish its complete source code under the same license.

Commercial licenses that allow use outside the GPL (for example, in a
closed-source product) are available from the copyright holder; contact the
maintainer through [GitHub](https://github.com/MrDesjardins).

Releases up to and including 0.0.13 were published under the MIT License, and
those copies remain available under MIT. Third-party components keep their own
licenses: see `THIRD-PARTY-NOTICES.txt` in each release. The virtual audio
driver in `drivers/audiorouter-virtual` is derived from Microsoft sample code
and is a separate program under the Microsoft Public License
(`drivers/audiorouter-virtual/LICENSE-MS-PL.txt`).

Contributions are welcome under the terms in [CONTRIBUTING.md](CONTRIBUTING.md).
