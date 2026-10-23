# UAD plugin support

Status: proposed, 2026-10-22 (owner request). U0 and the U3 channel
adaptation were implemented the same day on the owner's instruction ("I
want VST3 VST2 AND UAD to work"); the eight free UADx effects now process in
the app's worker. The remaining packages still need the owner's go-ahead.
Phases that change PLUG-01 (instruments) or PLUG-05 (latency changes) also
need a decision record and a matching specification update first.

## Objective

Let a Universal Audio (UA) customer use every kind of UAD processing they own
with AudioRouter, where that is technically possible. Where it is not
possible, say so in the app and the docs and offer the nearest working route.
UAD is the first vendor family we qualify on purpose. Most of the work
(choosing a plugin class, mono/stereo negotiation, latency reporting, VST3
editors, licence diagnostics) improves every third-party plugin, not just UAD.

## Requirement IDs

- PLUG-01 format boundary (x64 VST3; gated x64 VST2; instruments excluded).
- PLUG-02 discovery (class ID, vendor, version, fingerprint).
- PLUG-03 isolation, bus contract, worker pipeline delay in path latency.
- PLUG-04 state and native editors.
- PLUG-05 failure, quarantine; "unsupported latency changes".
- PLUG-06 compatibility record and licences; PLUG-07 legacy VST2.
- DSP-01 displayed values match effective values; UI-17 no layout shift.
- Protected-voice rule from `AGENTS.md`: a failed or unlicensed processor on a
  voice path produces silence, never dry audio.

## UAD product families

UA sells several kinds of processing under the UAD name. They differ in
format, in what hardware they need and in whether a host can load them at
all. Facts marked *(confirm in U1)* come from general vendor knowledge and
must be checked against a real installation before any user-facing claim.

| Family | What it is | Formats on Windows | Needs | Hostable by AudioRouter? |
| --- | --- | --- | --- | --- |
| UADx native | CPU plugins, no UA hardware | VST3, AAX, LUNA component; no VST2 (U1, 2026-10-22) | UA Connect sign-in and licence | Yes, as x64 VST3 effects (U0, 2026-10-22), with their own windows (U5, 2026-10-22); licence hint only (U6 partly) |
| UAD Spark | Subscription that unlocks UADx titles | Same binaries as UADx *(confirm with a Spark title)* | Active subscription | Same as UADx; an expired licence must be diagnosed (U6) |
| UAD-2 (DSP-powered) | Plugins that run on UAD-2 DSP chips | VST2, VST3, AAX | Apollo or UAD-2 Satellite/PCIe hardware, UA Connect or UAD software, licence | Possibly, as VST2 or VST3; needs hardware evidence (U7, U8) |
| UADx / UAD instruments | Synths and samplers (PolyMAX ships free) | VST3, AAX, LUNA | Licence | No. PLUG-01 excludes instruments and MIDI; see U11. PolyMAX presents an audio-effect class with an input bus, so scanning must filter it (G12) |
| Unison preamps and Console inserts | Processing inside Apollo's Console mixer | Not a host plugin | Apollo hardware | Not as a plugin. Route through Apollo's Windows audio devices instead (U9) |
| AAX builds | Pro Tools only | AAX | Avid host | No, and not planned: the AAX SDK requires an Avid agreement |
| LUNA extensions | Features of UA's LUNA recorder | Not a plugin | LUNA | No |

## Current state and gaps (from the code, 2026-10-22)

Discovery already covers UA's usual install locations. "Scan standard
folders" (`STANDARD_PLUGIN_FOLDERS` in `ui/src/draft.ts`) scans
`C:\Program Files\Common Files\VST3`, `...\Common Files\VST2`,
`C:\Program Files\VSTPlugins` and `C:\Program Files\Steinberg\VSTPlugins`
recursively. A `Powered Plug-Ins` subfolder below one of them is therefore
found, and VST3 bundles count as one candidate. Inspection is metadata-only.

The execution path has these gaps, most of which affect every vendor. G0,
G11 and G12 were found by the U1 run
([evidence](../active/evidence/2026-10-22-uadx-inventory.md)):

| # | Gap | Where | Effect on UAD |
| --- | --- | --- | --- |
| G0 | **Fixed 2026-10-22 (U0).** The app started `audiorouter-plugin-worker.exe` (VST2 only) for every plugin node | `crates/control/src/plugins.rs` `PluginWorkerKind` | Was: `VST2 load failed: MissingEntryPoint` for every VST3 plugin |
| G11 | **Fixed 2026-10-22 (U0).** The native VST3 worker used stdout as its protocol pipe | `isolate_protocol_pipes` in `tools/m06-vst3-worker/main.cpp` | Was: the UADx log banner read as a frame header (`TooLarge { length: 538970637 }`) |
| G12 | **Fixed 2026-10-22.** Instruments are refused when the worker starts and, since the follow-up round, filtered at scan time (`moduleinfo.json` sub-categories, UA `manifest.json` `is_synth`; compatibility `unsupportedInstrument`) | `read_plugin_metadata` in `crates/plugin-host/src/lib.rs`, worker factory check | PolyMAX leaves the Plugins list after a rescan |
| G13 | **Fixed 2026-10-22 (U0).** The worker was given the verbatim `\\?\C:\...` path from `fs::canonicalize`; UA's PACE licence check rejects it (`mixer.plugin.invalid_host_signature` in `%LOCALAPPDATA%\Universal Audio\Logs\UAPW_*.txt`) and the plugin passes audio through unchanged | `loadable_plugin_path` in `crates/plugin-host/src/lib.rs` | Was: every UADx effect output equal to its input |
| G14 | **Fixed 2026-10-22 (U0).** The worker ran with an empty environment (`env_clear`); plugins that resolve `%LOCALAPPDATA%`, `%ProgramData%` or `%TEMP%` lose their licence and settings stores | `PLUGIN_WORKER_ENVIRONMENT` in `crates/plugin-host/src/lib.rs` | A fixed list of Windows system and profile variables is passed; nothing else from the parent |
| G1 | Inspection discovers `classIds` and the UI stores `classIds[0]` on the node, but the VST3 worker takes no class argument and loads the first audio-effect class that initializes | `ui/src/draft.ts` `appendPluginPlaceholderNode`, `tools/m06-vst3-worker/main.cpp` (`parse_arguments`, factory loop) | A module with several effects (or mono and stereo variants) always gives the first one |
| G2 | **Fixed in the worker 2026-10-22.** The worker now calls `setBusArrangements` and, when the plugin keeps its own mono or stereo layout, adapts inside the worker (mono copied to both inputs; stereo output summed as 0.5·(L+R)). The adapter is not yet shown in the inspector | `negotiate_main_bus`, `process` in the worker | Every UADx effect is stereo-only and now runs on mono paths |
| G3 | **Reported 2026-10-22.** The bridge asks for `getLatencySamples` at start and after the editor closes and adds it to the stage delay the app shows. Not done: compensating parallel branches (decision 2), reacting to `restartComponent` while running | `ComponentHandler`; control-plane path latency | Plugin latency (UADx: 31–714 samples) is missing from path latency |
| G4 | **Fixed 2026-10-22.** VST3 editors open in the shell window (`IPlugView` on the worker's UI thread); WebView2 editors need the job's process limit raised while open (owner decision, SEC-07). Was: `editorUnavailable` | worker and `PluginPanels.tsx` | UAD plugins are designed around their editors; some licence prompts live there |
| G5 | **Partly fixed 2026-10-22.** A plugin that returns loud input unchanged for ~2 s is flagged (`unchangedAudio`) with a licence hint; demo and "hardware not found" states are still not distinguished | worker diagnostics, UI | An unauthorized UAD plugin looks like a silent or failed node with no reason |
| G6 | One worker process per VST3 instance; shared workers exist only for adjacent VST2 | plugin-host | Unknown whether many processes can share UAD-2 DSP; process count grows with long UAD chains |
| G7 | Fixed 128-frame engine quantum passed to the plugin | engine and worker | UAD-2 DSP plugins may need larger host blocks *(confirm in U8)* |
| G8 | Auxiliary (side-chain) buses are deactivated; runtime is single-stream | PLUG-03 runtime | Side-chain inputs on UAD compressors are unavailable |
| G9 | VST3 opaque state restore is not proven for every plugin (`-AllowStateUnsupported` exists) | 2026-09-14 state boundary | UAD presets and settings may not survive a restart |
| G10 | The VST2 adapter has no shell-plugin enumeration (`effShellGetNextPlugin`) | `crates/plugin-host/src/vst2.rs` | Only matters if a UAD VST2 build is a shell *(confirm in U1; believed not)* |

## Prerequisites

- Owner access to UAD software. U1–U6 need at least one UADx native plugin
  installed and licensed through UA Connect; a free UA title is enough if one
  is offered. U7–U8 need UAD-2 hardware (Apollo or UAD-2 Satellite), which the
  owner's PC does not currently have. Without it, UAD-2 support stays
  unqualified and must not be advertised.
- Licences stay the user's. Nothing is redistributed, downloaded or
  registered by AudioRouter. Test binaries are not committed (as with
  `third_party/vst/`).
- Plugin tests run on the host PC first (memory: host-first testing), not
  during a call (no heavy builds during calls).

## Decisions needed from the owner

1. **Hardware for UAD-2.** Borrow or buy an Apollo or Satellite, find a beta
   tester who owns one, or ship UADx only and document UAD-2 as unqualified.
   Recommendation: start with UADx (U1–U6), then take UAD-2 to a tester.
2. **Dynamic latency (changes PLUG-05).** Today a latency change is treated
   as unsupported. UAD-2 plugins report buffer-dependent latency and
   upsampled variants change it. Recommendation: accept a reported latency
   change between quanta, re-plan path delay off the realtime thread, and keep
   rejection only for changes during processing that the host was not told
   about.
3. **Instruments (changes PLUG-01).** Out of scope today. Recommendation:
   keep them out until a MIDI source node exists; U11 only records the design.
4. **Shared VST3 workers.** Needed if UAD-2 rejects many client processes.
   Recommendation: decide after U8 measures it.

## Ordered work packages

Each package lists its tests. "Windows" means the owner's PC; "hardware"
means a UAD-2 device.

### U0 — Run VST3 plugins in the app at all (G0, G11, G12) — done 2026-10-22

Implemented: control picks `audiorouter-vst3-worker.exe` (override
`AUDIOROUTER_VST3_WORKER_PATH`) for VST3 nodes; the release scripts build it
with `tools/m06-vst3-worker/build.ps1 -Release` and package it beside the
VST2 worker; the worker isolates its protocol pipes, filters parameter
descriptors (automatable, visible, writable; at most 256), refuses
instruments, and reports latency. G13 and G14 were found and fixed on the
way. Evidence: [U1 evidence, addendum](../active/evidence/2026-10-22-uadx-inventory.md#u0-results-in-the-app-worker-2026-10-22).
Still open from the original U0 tests: a repository fixture that prints to
stdout, an attended Play with a UADx effect on a real route, and the
fresh-install installer check at the next release.

Original design note: blocking for every VST3 plugin, not only UAD. Choose one: build the native
VST3 host into `audiorouter-plugin-worker.exe` (dispatch by the verified
format), or package the native VST3 worker beside it and have
`plugin_worker_path` pick the executable by format. Recommendation: package
the native worker and select it by format; it already carries the M06
containment evidence. Before any plugin code loads, the worker duplicates
its stdout handle for the protocol and points the process's stdout (CRT
fd 1 and `STD_OUTPUT_HANDLE`) at `NUL` or stderr, so plugin printing cannot
reach the pipe; apply the same to the VST2 worker. Filter instruments by
VST3 sub-category at inspection. Tests: a repository VST3 fixture that
prints to stdout on load and process; a control-plane test that a VST3 node
starts through the packaged path (the fresh-install lesson: test what the
shell really launches); muted Start plus pump with a UADx effect and
telemetry `running`; installer contents check for the new executable.

### U1 — Inventory probe (no code change to the host)

**First run done 2026-10-22** with the nine free UADx titles
([evidence](../active/evidence/2026-10-22-uadx-inventory.md)): all load
offline, process finite output and save/restore state, without a sign-in
prompt. Still open, to repeat once U0 lets the real worker run: parameter
count through the connected controller (the loader saw 0), latency, channel
layouts accepted by `setBusArrangements`, and behavior when UA Connect is
signed out. UAD-2 software was not tested (no hardware).

Method: run "Scan standard folders" and the offline loader
(`tools/m06-vst3-loader`) on every UA binary found. Record path, format,
architecture, class list, bus layout, latency, sign-in behavior. Output: a
dated evidence file and corrected facts in the family table above.

### U2 — Choose the plugin class (G1, PLUG-02)

Inspection returns every audio-effect class in a module. The scan result and
the Plugins list show one entry per class (name and vendor), and the node
stores the class ID with the fingerprint. The worker creates exactly that
class; a missing class ID is a distinct error, never a fallback to another
class. Tests: a repository-owned VST3 fixture with two classes; contract
drift check; UI test that both appear and the right one runs.

### U3 — Negotiate mono and stereo (G2)

Before activation, call `setBusArrangements` with the node's layout. If the
plugin refuses, try the plugin's own arrangement and insert an explicit
channel adapter (mono-to-stereo copy, stereo-to-mono sum with documented
gain) in the compiled path. Show the adapter in the inspector. Tests: fixture
that accepts stereo only, on a mono microphone route; sine-continuity harness
with the plugin worker kind (lesson 2026-09-26); muted Start plus pump with
plugin telemetry `running`.

### U4 — Report and compensate latency (G3, decision 2)

Read `getLatencySamples` after activation and on `kLatencyChanged`;
re-activate outside the realtime path as VST3 requires. Report the value in
node telemetry and in path latency next to the worker queue delay. Apply the
VST2 `initial_delay` the same way. Tests: fixture with a fixed and a
switchable latency; path-latency assertion; Properties panel shows it with a
fixed-width value (UI-17).

### U5 — VST3 native editors (G4, PLUG-04)

Open `IPlugView` in a worker-owned window, as VST2 editors already do:
`IPlugFrame::resizeView`, DPI awareness, `IComponentHandler` edits routed to
the controller and saved with state, close without stopping audio. Enable
"Open plugin editor" for VST3. Tests: fixture editor open/resize/close while
audio runs; UADx editor smoke test on Windows; screenshots of the inspector
in dark, light and high-contrast themes.

### U6 — Licence and hardware diagnostics (G5)

Classify, without vendor-specific hacks: worker could not create the
instance; instance created but reports no processing (silent output for a
known non-silent input after warm-up); editor-only prompt. Map them to plain
messages ("This plugin is not authorized on this PC. Open UA Connect, sign
in, then press Retry."). A protected voice path stays silent until the user
retries. Tests: UADx with UA Connect signed out, then signed in, then Retry;
no dry voice reaches the output during the failure (sine harness).

### U7 — Host block size for DSP plugins (G7, hardware)

Measure UAD-2 plugins at the 128-frame quantum. If they need larger blocks,
add a per-worker block size with a constant-delay re-blocking stage (lesson
2026-10-10: one block out per block in, pre-filled), and count its delay in
path latency. Tests: re-blocking unit tests asserting emission counts per
push; sine harness through a UAD-2 plugin; 10-minute run with zero glitches.

### U8 — UAD-2 resource behavior (G6, hardware, decision 4)

With a UAD-2 device: how many worker processes can hold DSP instances, what
the plugin does when DSP is full or the device is unplugged mid-route, and
whether AudioRouter and a DAW can share the DSP. Unplug must give silence on
protected paths and a clear message, then deliberate recovery. If many
processes fail, design a shared VST3 worker with the same containment
evidence the VST2 shared chain needed.

### U9 — Apollo Console route guide (docs only)

Unison preamps and Console inserts are applied by the Apollo before Windows
sees the audio. Document how to pick the Apollo's Windows input and virtual
output devices as AudioRouter endpoints, and the double-processing and
monitoring pitfalls. Needs an Apollo owner to confirm device names and
screenshots.

### U10 — Side-chain inputs (G8, after U3)

Wire the existing multi-bus contract (PLUG-03) into graph ownership and the
shared-memory runtime so a second node input feeds a plugin's auxiliary bus.
UAD compressors with external key inputs are the acceptance fixture.

### U11 — Instruments (design only, decision 3)

Record what a MIDI source node, note input bus and instrument worker would
need. No implementation without a PLUG-01 change.

### U12 — Compatibility record and publication

Add each tested UA title to `docs/operations/plugin-compatibility.md` with the
exact version, format, and per-check results (load, process, mono/stereo,
latency, editor, state, licence states). Update the user guide's UAD section
and the website only to what was proven.

## Validation matrix

| Check | U1 | U2 | U3 | U4 | U5 | U6 | U7 | U8 | U10 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Repository fixture unit/contract tests | | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | | ✓ |
| UI tests + three-theme screenshots | | ✓ | ✓ | ✓ | ✓ | ✓ | | | ✓ |
| Muted Start + pump, telemetry `running` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Sine-continuity harness | | | ✓ | ✓ | | ✓ | ✓ | ✓ | ✓ |
| UADx on the owner's PC | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | | | |
| UAD-2 hardware | ✓ | | | | | | ✓ | ✓ | ✓ |
| Fresh-install shell check (release only) | | | | | | | | | |

Clippy, rustfmt, UI format and lint apply to every code change.

## Risks

- UA's licensing and installer behavior can change; date every observation.
- UADx plugins may refuse to run without their editor or a sign-in prompt;
  U5 and U6 may need to land together.
- UAD-2 may not tolerate many client processes; U8 may force a shared VST3
  worker, which needs its own containment evidence.
- Without UAD-2 hardware the DSP-powered family stays unqualified; the docs
  must not imply otherwise.
- Copying vendor behavior into code (special cases by vendor name) would
  hide general bugs; fixes belong in the general host.

## Rollback

Each package is a separate change behind the existing plugin node. Reverting
a package restores the current behavior (first class, exact channel match,
no VST3 editor). Persisted class IDs from U2 must stay readable by a rollback
build: store them as an optional field that older builds ignore.

## Next action

The owner plays a UADx effect (1176) on a real route in the app and confirms
it is audible. Then, with the owner's go-ahead: U4 (feed the reported
latency into path latency), U5 (VST3 editors; UADx editors use WebView2,
which needs more than the worker job's one-process limit, so that limit
needs a reviewed decision), U6 (licence diagnostics: a signed-out UADx
plugin passes audio through, and on a voice path that must become silence)
and scan-time instrument filtering. Decision 1 (UAD-2 hardware) can wait.
