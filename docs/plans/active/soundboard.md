# Active plan: Soundboard tool and sound library (2026-10-29)

Owner request, 2026-10-29: build a Soundboard tool (nine sounds, canvas buttons,
Stream Deck trigger through the API) with a library of about a hundred free short
sounds, a searchable picker, and the user's own sounds. "I want you to start."

- Specification and behavior: [Soundboard tool](../future/soundboard-tool.md)
  and [Soundboard sound library](../future/soundboard-sound-library.md). Those
  documents stay the detailed source; this plan tracks execution.
- Authorized: the tool, its API, UI, Stream Deck action, the bundled sound
  library and picker, and website guide pages for the tool.
- Not authorized: in-app global keyboard shortcuts (step 2 of the key-binding
  section), a ready-made Stream Deck profile, a downloadable sound pack, streaming
  of long files, attribution-licensed sounds. Each needs a separate decision.
- No pull request is opened without the owner's approval (AGENTS.md).

## Owner decisions (2026-10-29)

1. Built-in sounds are CC0, public domain or original sounds made for AudioRouter
   only. No attribution licenses in version one.
2. The library is bundled in the installer. No download and no network access.
3. User sounds: 120 seconds per slot and a total decoded budget for the board,
   shown at upload. No streaming of longer files in version one.

## Requirement IDs

SND-01 to SND-14 (proposed in the Soundboard tool plan; added to `docs/spec/`
in task 1). Touches UI-17, UI-20, UI-11, API/CLI/MCP parity and the audio
callback rules.

## Prerequisites and constraints

- Compile-heavy work (Rust builds, test suites, Playwright) waits while the
  owner's virtual cable VM session is running, because host load causes false
  audio-loss failures in the VM. Documentation, scripts and sourcing can proceed.
- A Freesound API key (free; see "Getting Freesound access" in the library plan) is needed only if Freesound is used for sourcing;
  Kenney, OpenGameArt, Wikimedia and generated sounds need nothing from the owner.
- Windows evidence is required for audio continuity, installer contents and the
  fresh-install run. Stream Deck hardware is required for the attended key test.

## Ordered tasks

1. Spec: add SND requirements, the tool catalog entry, API methods, the tool guide
   requirement and a decision record. Update the future plans to point here.
2. Sourcing and format spike (no Rust build): build the target list, collect and
   record candidates, write the generator for original sounds, compare WAV, MP3
   and FLAC for size and clean starts, choose normalization numbers by listening.
3. Domain: `NodeKind::Soundboard`, slot parameters, validation, `API_METHODS`.
4. Engine: prepared stage, voices, lock-free command queue, mixing and limiter;
   add to the tool-combination suite.
5. Control plane, API, CLI, MCP, HTTP; regenerate the website API catalog.
6. Library: manifest, checks in CI, installer resource folder, notices file,
   `soundLibrary.list` and preview read.
7. UI: card buttons, Properties editor, picker; three-theme screenshots and
   layout-shift sampling.
8. Stream Deck action and its tests.
9. Website: tool guide page and the Stream Deck page.
10. Qualification: audio continuity, fresh-install run, installer check, attended
    Stream Deck test, owner listening review.

## Validation matrix

See the validation tables in the two linked plans. Record each result here with
command, environment, result and evidence path as it is run.

## Risks

See the linked plans. Additional execution risks: a long implementation spanning
engine, UI, API and plugin must be kept in reviewable commits (one task each), and
the library licensing record must be complete before any sound is committed.

## Rollback

Each task is a separate commit on `feature/soundboard`. Reverting the branch
removes the tool, the API methods, the library and the pages. A saved session
containing a Soundboard needs the placeholder decision from the Soundboard plan
before release.

## Progress

- 2026-10-29: owner decisions recorded; plan promoted from `future/`. No code yet.
- 2026-10-29: library target changed to up to about 100 (about 80 is fine) with many
  variations of favorites (fart 10 to 12, claps, ta-da); Bodily humor category and
  content rules added; Freesound access steps written.

- 2026-10-29 sourcing, first pass (no key, host-light):
  - Wikimedia Commons API (CC0/public-domain, 6 s or less): mostly spoken-word
    recordings; usable hits are a few doorbells and two roosters. Secondary source.
  - OpenGameArt (CC0 sound-effect filter): almost nothing for fart, clap, applause
    or air horn; some fanfare and laugh packs to inspect. Secondary source.
  - kenney.nl returns 403 to scripted downloads; the packs need a manual download
    by the owner (not circumvented). Freesound needs the owner's API key.
  - `tools/soundboard/generate-original.mjs`: 33 original sounds (9 fart variations,
    3 ta-da, 3 air horns, rimshot, sad horn, buzzer, doorbell, coin, etc.), seeded and
    byte-identical across runs, all at most 3.3 s. Unreviewed by ear. Level check:
    peak is matched at -3 dBFS but average level varies from about -4 to -19 dB, so
    the library needs RMS or loudness matching as well as a peak ceiling.
- 2026-10-29 gap review added to the Soundboard plan: v1 additions per-slot level,
  play modes (loop, toggle, hold), random-variant slots, progress on the button,
  drag and drop, recently used, trigger cooldown (SND-15 to SND-20; owner to confirm).

- 2026-10-29 sourcing closed for now (owner: add more later). The library is
  committed: `resources/sound-library/` holds 105 sounds as mono 44.1 kHz FLAC with
  `library.json` (source, author, URL, license, SHA-256 each) and `NOTICES.md` (7.8 MB;
  hashes verified). Format spike result for 105 sounds: WAV 14.4 MB, FLAC 7.7 MB,
  MP3 2.7 MB with a 25 ms encoder start delay. Decision: FLAC, which needs the
  decoder's `flac` feature (to enable and verify in task 4). The picks, license
  evidence and tools are in `tools/soundboard/` (see its README); the raw sources are
  backed up at `C:\audiorouter-assets\sound-sourcing`.
- 2026-10-29 build start: Rust builds wait for the owner's VM session to finish
  (VM `AR-DriverTest` was still running); code is written first.

- 2026-10-28 code written, NOT yet compiled or tested (the owner's VM pool run was
  still in progress, so no cargo, tsc, vitest or Playwright ran on the host):
  - Domain: `NodeKind::Soundboard` (37 kinds), slot parameters `slotNMediaId`,
    `FileName`, `Caption`, `GainDb` (-24 to +6), `Mode` (once, loop, toggle, hold),
    `Set` (a category id or up to 8 sound refs), bounded validation and a unit test;
    API methods `soundboard.trigger`, `soundboard.stop`, `soundboard.status`,
    `soundLibrary.list`, `soundLibrary.read`.
  - Engine: `crates/engine/src/soundboard.rs` (16 voices, bounded command queue,
    5 ms stop fade, 50 ms trigger gap, random sets that never repeat, soft limiter) with
    unit tests; symphonia `flac` feature added (Cargo.lock will change on first build).
  - Control: `soundboard.rs` (API), `sound_library.rs` (manifest, SHA-256 check,
    base64 read, decoded-budget loading of `lib:` and user sounds).
  - UI: canvas card, Properties editor, picker, live status store, API client,
    Stream Deck Sound action and inspector, website guide, API catalog, Stream Deck page,
    spec UI-25.
  - Known gaps: a library category in `slotNSet` is accepted but not yet expanded (lists
    work); drag and drop (SND-19) is not built; the installer does not yet copy
    `resources/sound-library` next to the program; `tool_combinations.rs` and an
    engine route test for Soundboard are not written; no jsdom tests for the new UI.

- 2026-10-29 first compile and static checks (host, `-j2`, VM idle; the pool run had
  passed). The scripted patches had duplicated lines in `simple.rs`, `audio_media.rs`,
  `native_dispatch.rs`, `native_paths.rs`, `status.rs` and `App.tsx`; every one was a
  compile error and is fixed (commits 660b83dd and the UI fix after it). Also fixed: the
  registry and API array lengths (37, 136), a `json!` repeat, an unused import, a missing
  `soundboard` key in `draft.test.ts`.
  Results: `cargo fmt` both workspaces clean; `cargo clippy --workspace --all-targets
  --all-features -D warnings` clean; same for `src-tauri`; domain 78, engine lib 172,
  control 299 and `tool_combinations` 4 tests pass; `ui` typecheck:all, lint and prettier
  clean, vitest 743/743; `tools/streamdeck` typecheck clean, 27 tests pass;
  `site/check-guides.mjs` passes. Not run: Playwright, any live audio, the installer.
  `tool_combinations.rs` `TOOLS` lists processors only, so a source needs no entry.
  The installer resource list now includes `sound-library` in both release configs; the
  built installer has not been inspected.

- 2026-10-29 owner attended test of the built exe and Stream Deck plugin found five
  defects, each fixed with a regression test that fails without the fix:
  1. A Mixer route containing a Soundboard was refused (`connected_mixers.rs` kept its
     own source list): `source_combinations.rs` now compiles and runs every source kind
     alone, in a fan-out, in every Mixer pair and in every connected-Mixer triple, and
     classifies `NodeKind` with an exhaustive `match`.
  2. Microphone + Soundboard in a Mixer went to the single-endpoint chain, which mixes
     only a Test Signal with one input (`UnsupportedTopology`): `needsNativePaths` now
     sends an Audio File or Soundboard that shares a route to the multi-path worker
     (`draft.test.ts`).
  3. A missing device gave a message but no mark: the nodes Play needs now get a red
     dashed outline and a "Choose a device" tag.
  4. The Stream Deck Sound key's `mode` was rejected as an unknown parameter by the
     allowlist in `authorization.rs`; a test now walks every method's schema against the
     allowlist (it also found `devices.list includeInactive` and `nativeBridges.prepare
     leaseMs`), and a dispatcher test sends every documented Soundboard parameter.
  5. Stream Deck: per-key behavior (slot setting, once, loop until pressed again, hold),
     a key name, and slot names in the inspector; `sound-settings.ts` holds the logic
     and `test/sound.test.ts` tests it. A slot's Loop now stops on the next press.
  Process finding: the owner reported that these were handed over untested. AGENTS.md
  now has a "Done means exercised" rule.
  Pre-PR checks (host, 2026-10-29): `cargo fmt` both workspaces clean; Clippy
  `-D warnings` clean for the workspace and `src-tauri`; `cargo test --workspace` and
  `src-tauri` pass; UI format, lint, typecheck:all and 749 vitest tests pass; Stream
  Deck typecheck and 33 tests pass; `site/check-guides.mjs` passes. Attended by the
  owner: Play on Mixer routes, the Sound key (slot setting, mode choices after the fix
  not yet re-confirmed by the owner). Not run: Playwright, the continuity harness, a
  fresh-install run of the installer, the three-theme screenshots, listening review of
  every library sound.

## Next action

### 2026-10-30: Soundboard / Mixer 4 investigation

Owner reports both Play controls produce no Cable-A audio. Requirements GRAPH-12,
GRAPH-15 and UI-25/SND trigger parity. Working tree was initially clean. Read the
live database through read-only SQLite and retained a private session copy in
ignored `target/soundboard-owner-session.json`; revision 156 wires Soundboard 1
and the processed mic into Mixer 4, then Cable-A. No saved route was changed.
Windows live RPC accepted slot 1 and 3 triggers; slot 3 reported playing/progress,
Mixer 4 and Cable-A reported identical peaks (-24.41 dBFS on the first sample),
while Soundboard 1 incorrectly reported -120 dBFS. This establishes app-side
delivery, not receipt or audibility in Discord/Siege or the cable capture device.

Cause of the confirmed meter defect: generated inputs use silent pacing packets,
and the input meter observed those rather than the rendered Soundboard block.
Fix both connected output-cache and ordinary input-chain paths to observe the
rendered board. Regression extends the connected-Mixer test to trigger and
assert nonzero output and source meter. A separate opt-in private-session test
loads the three actual bundled FLACs and renders 100 quanta to Cable-A; it passed
on Windows with the saved revision before the meter fix, proving the wiring
already delivers the sound. Existing combination tests previously asserted only
finite outputs, so did not establish audible generator output.

Validation: focused engine trigger/render/meter test passes on Windows. Next:
run source/tool combinations, saved-route regression and format/Clippy; inspect
diff. Remaining investigation: Windows cable capture/receiving app audibility.
No replacement executable is handed over yet. Rollback: revert only these meter
and regression changes; no migration, device change or driver work.

Local validation completed on Windows: focused trigger/render/meter test (1),
source combinations (4), tool combinations (4, including allocation check),
opt-in saved-route test (1) all pass. `cargo clippy -p audiorouter-engine
--all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check`
pass. Live RPC slot 3 showed playing/progress and matching Mixer 4/Cable-A
levels while the Soundboard meter stayed silent, confirming the meter defect
in the running 0.0.25 executable. User asked which receiving app is being used;
await that clarification before attributing downstream silence. An unrelated
concurrent edit appeared in `ui/src/styles.css`; preserved unchanged by this task.

Owner clarification: microphone audio works; Soundboard -> Mixer 4 has no flow
animation while Mixer 4 -> Cable-A animates. Confirmed UI data path:
`edgeMeterDb` in `ui/src/SessionFlowCanvas.tsx` prefers the source node's RMS;
`edgeSignalForConnection` marks values at or below -60 dB silent. The erroneous
-120 dB Soundboard source meter therefore suppresses exactly the reported edge
animation. The backend meter correction supplies this same UI data path; no UI
animation change is needed. Visual confirmation in a rebuilt shell remains open.

Owner requests rebuild and complete regression coverage. Extend source combinations
to press Soundboard Play and require nonzero output and source meter in single,
fan-out, every source pair and every connected-Mixer triple. Add UI edge-state
coverage for idle board/live Mixer, active board/live Mixer and return to idle.
Run focused regression with meter fix removed to qualify failure, restore fix,
then workspace/shell checks, UI checks and fresh embedded-UI release build.
Stage a separate local folder with matching sidecars/library; preserve the running
app and unrelated stylesheet edits. Rollback is the existing local-test executable.

Rebuild/regression results (2026-10-30, Windows, Rust 1.96.0, Node 22.22.3):

- Qualified the regression by temporarily removing rendered-meter observations:
  source-combination tests failed in single, pair and connected-Mixer layouts
  with `Soundboard meter measured silence`. Restored the fix and all pass.
  Added exact observed-frame counts, which caught double counting in the first
  correction; pacing packets are now excluded from this meter. No allocations,
  locks or Arc clones were added to the processing path.
- `cargo test --workspace --locked` passed (explicit hardware/private fixtures
  remain ignored). After the final frame-count correction, engine lib tests
  passed 176/1 ignored, source combinations 4/4, tool combinations 4/4 and
  the explicit saved revision 156 FLAC playback/meter test 1/1 passed.
- Shell tests passed 85/4 ignored. Workspace and shell Clippy with all targets,
  all features and `-D warnings` passed. Both rustfmt check commands and
  `git diff --check` passed.
- UI full suite passed 758 tests/83 files, including both actual Play controls
  through their controller (already running and start-before-trigger), and the
  source-specific edge animation state. Typecheck:all, lint and format:check pass.
- Edge `soundboard-flow.pw.ts`: 3/3 pass, dark/light/high-contrast, idle -> active
  -> idle source meter with an independently active Mixer/output, unchanged card
  bounds. Inspected all screenshots in
  `target/soundboard-flow-evidence/`. This uses simulated telemetry, while the
  engine regressions render actual PCM. Initial harness attempts hit a shared
  port, a type annotation error and a decoration-matching selector; corrected
  them, used private port 4197, and reran successfully. No shared server stopped.
- `npm.cmd run build` refreshed `ui/dist`. The optimized custom-protocol shell
  linked successfully, but Cargo's final copy to `src-tauri/target/release/`
  was blocked by the owner's running executable. Copied the freshly linked
  `release/deps/audiorouter_shell.exe` to the isolated staging folder and verified
  identical SHA-256. Its 23:36:36 timestamp is newer than the final engine source
  (23:33:56) and embedded UI (23:33:26). CLI/plugin worker were rebuilt in release;
  unchanged native VST3/tunnel runtimes and the current sound library are staged.
- Build: `target/local-test-soundboard-meter-20261030/audiorouter-shell.exe`,
  version 0.0.24 (current checkout version, not a new release). Checksums are in
  that folder's `checksums.json`. Shell SHA-256:
  `EEA1CE9403BB689318FB4A36095EFEDCD11D9F04AE2D9E035F3F691407CBCECE`.
- Ran that exact staged shell at Medium/non-admin integrity against a private
  real backend, disposable database and WebView2 profile. Soundboard canvas and
  Play button loaded through the embedded assets/native bridge, zero observed
  page errors. Evidence: `target/soundboard-shell-smoke-1793428692889/result.json`
  and `rebuilt-shell.png`; token report `target/soundboard-smoke-token.json`.
  Initial probe looked for DevToolsActivePort one directory too high; corrected
  it to WebView2's EBWebView directory and reran. Owned test processes cleaned up.
  No devices opened, user session changed, or running owner app stopped.
- Tooling notices: Vite warns about a 621 kB application chunk; splitting it is
  outside this meter correction. Edge development harness emitted React Flow's
  attribution notice, and inherited NO_COLOR/FORCE_COLOR produced a runner
  notice; no attribution or colour configuration was changed. Node SQLite used
  only for read-only diagnosis/disposable smoke setup reports experimental API.

The staged build was handed to the owner for the saved-route check. No installer,
release or full browser suite is claimed. Remaining broader Soundboard feature
qualifications follow below.

Owner acceptance (2026-10-30): "Working, please commit in the appropriate branch
and open a PR." The owner confirmed the rebuilt fix and explicitly authorized
opening its PR. Branch `fix/soundboard-source-meter` starts at current main
(`b33fa54b`), including the already merged 0.0.25 release. Revalidate on that base,
commit only the engine/meter tests, UI regressions and this plan; exclude the
concurrent `ui/src/styles.css` edit. Next action: hosted Windows CI and human
review after opening the approved PR. No release or installer is authorized by
this acceptance.

Pre-PR revalidation on the updated 0.0.25 base passed: full workspace and shell
tests, both all-target/all-feature Clippy runs with warnings denied, both Rust
format/check runs, UI format/lint/typecheck and all 758 tests. Documentation
acceptance passed (213 Markdown files, 1,141 links), and the complete staged diff
was inspected. Reproducible logs are in `target/soundboard-pr-*.log`; the earlier
three-theme Edge, private-session audio and rebuilt-shell evidence remains above.

Remaining, in order: library category expansion for `slotNSet`; drag and drop (SND-19);
remaining editor/picker interactions beyond the now-covered Play controls;
inspect a built installer for `sound-library` (and `verify-artifacts.ps1` expectations);
three-theme screenshots with layout-shift sampling; live continuity, fresh-install and
attended Stream Deck qualification (task 10), which need the owner. The Stage A
`hardening` session still needs the owner to run it in the VM.
