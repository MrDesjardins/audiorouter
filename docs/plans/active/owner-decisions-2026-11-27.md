# Owner decisions 2026-11-27: API and MCP flexibility

## Objective and authority

On 2026-11-27 the owner answered the open A decisions: "A14, A1: Yes, A2: Yes,
A4: Unclear, I do have on this PC Cable-A and Cable-B but we need AudioRouter to
be working for all PC which may have different virtual cables. A12, A13,
A15-A18, fine. The idea is to have as much flexibility for people with API and
MCP." "Fine" accepts each default. Earlier in the same request the owner also
asked for the UI port, the 0.0.33 candidate and nightly upkeep (current-plan
priorities 3–5). No driver work, PR, release or publication is authorized by
this plan; a PR still needs explicit approval.

## Requirements

AUTO-06/10/15/18, R2-1a, R2-4, SEC-02, API-07, ARCH-08, NFR-03, UI-17, L10N-01.

## Decisions

| ID | Decision | Done in |
| --- | --- | --- |
| A1 | Option (a): per-client recording opt-in, off by default, set in the window | This branch; decision record in `docs/spec/13-security.md` and AUTO-18 |
| A2 | `sessions.rename` with the default MCP tool `rename_session` (Editor) | This branch |
| A4 | Option (a), chosen 2026-11-27 after the investigation below: stereo routes play into the front left/right of any multichannel output device | Its own branch, stacked on this one |
| A12, A13, A15–A18 | Defaults kept | Recorded in the human-gated table |
| A14 | Site and guides use the app's words ("input device", "output device") | Separate site branch |

A3 (bus lists expert-only): on 2026-11-27 the owner asked what "expert-only"
meant. Explained: the two read-only lists were offered to assistants only with
`--advanced-tools`, REST already served them, and changing buses needs device
administration either way. The owner then answered **yes, make them default
tools**. Done on this branch: both are in `DEFAULT_MCP_TOOLS` with plain
descriptions, their exclusions are removed, and
`an_observer_reads_the_virtual_bus_lists_through_default_mcp_tools` calls both
through `mcp_tool_call` as an Observer.

## Tasks and status

1. Done: record the answers in [human-gated tasks](human-gated-tasks.md).
2. Done: A2. `sessions.rename` is a task-shaped edit (`simple.rs`). It trims the
   name, refuses empty names, names with control characters and names over 120
   characters, and commits a new revision through plan/commit, so undo works and
   audio is not interrupted. The current name returns `unchanged: true` without a
   revision. It is GraphWrite, so Editor and Operator can rename. Wired into
   `API_METHODS`, the dispatcher, the parameter allowlist, schemas, contracts, the
   retry contract cases, the API reference and `site/assets/api-methods.json`.
3. Done: A1. The backend part covers storage, `grant_for_client`, the
   `clients.authorize` parameter and the `clients.list` field. The four recording
   tools join the default MCP list, and the MCP server instructions and recipe 3
   now say they need the opt-in. The window's Connected clients panel gains the
   box and a "may record" tag, in three languages, and the window's MCP
   permission help, the Cowork guide and the site MCP page say so too.
4. Done: full local checks (below) and the panel in three themes. The first
   screenshots showed the toggle and the Authorize button on one line, so the
   note now sits under the toggle and the button has its own row.
5. Site wording (A14) goes on its own branch.
6. Done: A4 investigation; the owner chose option (a) on 2026-11-27. Implementation is a separate branch.
7. 0.0.33 candidate: once #199, the UI port and this branch are on `main`, or
   from a local integration if the owner wants to test first.

## A4 investigation: virtual cables other than stereo

Findings (2026-11-27, code reading):

- Capture already copes: `adapt_native_paths_session` widens or folds a
  route to the endpoint's channel count, and 5.1/7.1 capture is supported.
- Output does not. `output_format_supported` (`crates/control/src/lib.rs`)
  accepts only stereo IEEE float32, so Play refuses any route whose output is
  a 4-, 6- or 8-channel endpoint ("output fan-out endpoints must be stereo
  IEEE float32"). `SharedRender::submit_bytes` accepted a narrower stride but
  copied it packed, which would have scrambled the channels.
- Cables differ by PC: the owner's CABLE-B Input is set to 7.1 on purpose, and
  other virtual-cable products may default to more than two channels (not
  verified here). Such a cable cannot be an output today.

Options for the owner:

| Option | What changes | Cost and risk |
| --- | --- | --- |
| (a) Recommended | Write the stereo route into the front left/right channels of any float32 render endpoint and leave the other channels silent; same for 16/24-bit PCM through the existing conversion | Small, contained in the output pump; needs 4/6/8-channel tests, `source_combinations` coverage and a continuity run in a quiet window |
| (b) | Full multichannel output routes (route channels map to endpoint channels) | Large: graph, UI, MCP and recorder changes |
| (c) | Keep stereo-only and explain it in the device picker | No engine work; such cables stay unusable |

Either way, list every endpoint's channel count in `list_devices` so an
assistant can see why a device is refused. **Owner choice (2026-11-27): option (a).**

## A4 option (a): implementation plan (branch `feat/multichannel-output`)

Scope: every worker Play can choose. The multi-input output fan-out serves
multi-device routes; the UI picks the single-endpoint worker
(`nativeEndpoints.prepare`) for a simple one-input, one-output route and the
application worker for a single app route. A first version covered only the
fan-out; the single-endpoint bridge (stereo graph and resamplers whatever the
device width) and the application worker check were added before handoff.

1. `SharedRender` records the stream's float32 flag and byte stride.
   `submit_bytes` keeps the exact-width copy, and a narrower float32 source into
   a wider float32 stream is written frame by frame into the device buffer: the
   source channels fill the first channels, every other channel exactly 0.0.
   Windows orders standard masks front-left/right first, so a stereo route lands
   on front L/R; the first two channels are used without a mask. The expansion
   is a pure helper on borrowed slices: no allocation, no pump or ring changes,
   frame counts unchanged, so the drift controller and its queue math are
   untouched.
2. `output_format_supported` accepts float32 render devices with 2 to 32
   channels at 8–192 kHz (mono still refused). The fan-out refusal message and
   `setup_check` follow it. Spatial (binaural) capture rules are unchanged.
3. The device list reports each device's channel count where it does not
   already, so a script or assistant can see the layout.
4. Docs: the API reference and the device guidance say what a multichannel
   output plays.

Verification: unit tests of the helper (4, 6 and 8 channels, masks with and
without front left/right, exact-width passthrough, refused partial or non-float
input); `setup_check_tests` flipped deliberately; the drift simulation suite
and the windows-audio, control and engine suites; both Clippy runs. Hardware
proof needs the continuity harness through CABLE-B set to 7.1, which plays into
a cable the owner uses live: it waits for a quiet window, and A4 is not done
before it passes.

Rollback: revert the branch; devices with more than two channels are refused
at Play again, as before.

Status 2026-11-27: steps 1, 2 and 4 done; step 3 needed nothing, because
`devices.list` already reports `format.channels` for every device and the UI
labels it. The limit is 2–32 float channels (`MAX_RENDER_CHANNELS`, one value
for every worker). Local results: both `cargo fmt` checks and both Clippy runs
clean; `audiorouter-windows-audio` (170, including the drift simulation suite,
the widening tests and the multichannel endpoint-bridge test) and
`audiorouter-control` (530) passed, including the flipped `setup_check` tests.
The qualification audit also ran the whole `audiorouter-engine` suite,
including `source_combinations` (6/6) and `tool_combinations` (6/6), and docs
validation (331 files). It found and corrected three stale statements that still
refused a multichannel output — the setup-check contract in
`docs/spec/09-interface.md`, the quickstart's setup-check section, and the
`nativeOutputs.prepare` text in the API reference. Evidence:
[multichannel output audit](evidence/2026-11-27-multichannel-output.md). A fresh
release shell was built from this branch for the attended gate (SHA-256
`5D06A08C…`).

Live qualification, 2026-11-27, owner's quiet window, Windows 11 dev PC.
CABLE-B Input reported 8 channels, float32, 48 kHz. Harness
`live_backend_service_keeps_a_routed_tone_continuous` (997 Hz tone, CABLE Output
→ Gain → CABLE-B Input, result read from CABLE-B Output):

| Worker | Length | Reference glitches | Routed glitches | Result |
| --- | --- | --- | --- | --- |
| multi-path (default) | 30 s | 0 | 0 | pass, peak 0.0500 |
| multi-path | 60 s | 0 | 0 | pass (one 37 ms late audio pass absorbed) |
| single endpoint | 30 s | not recorded | 4 | fail |
| single endpoint | 30 s ×3 | 0 | 0 | pass |
| single endpoint | 30 s | 2 (1 silent run) | 4 | system hiccup: reference also broken, 18 ms late pass |

The routed tone arrived at full level with zero glitches in every run whose
reference was clean, so the stereo route reaches the 7.1 cable's front left/right
intact. The first endpoint failure kept no reference figures, so it cannot be
attributed; it did not recur in three runs. Not checked: that channels 3–8 are
silent on hardware (CABLE-B Output is read as stereo; the unit tests cover it),
the application worker live, and an attended Play in the built app.

## Validation

- `sessions_rename_saves_a_revision_through_dispatch_and_respects_roles`
  (dispatcher: trim, replay, unchanged, refusals, Observer denied, Editor
  allowed).
- `rename_session_tool_renames_the_open_session_and_undo_restores_it` (default
  MCP tool through `mcp_tool_call`, undo, Observer refused).
- `client_recording_opt_in_is_off_by_default_and_round_trips` (storage).
- `recording_opt_in_is_per_client_off_by_default_and_cleared_by_revocation`
  (dispatcher, restart, role change, revocation, self-grant refused).
- `real_mcp_pipe_honors_the_recording_opt_in_and_its_live_removal` (real backend
  process, real MCP sidecar with default tools, live allow, disallow and revoke).
- `lists known clients and authorizes a new one` (UI: off by default, the exact
  request sent, the tag, reset after authorization).
- `cargo fmt` (both), strict Clippy (workspace and `src-tauri`), control, CLI,
  storage and transport tests, `src-tauri` tests, contract drift, docs validation,
  UI typecheck, lint, format and unit tests, and screenshots of the panel in the
  dark, light and high-contrast themes.

## Risks and rollback

### PR #201 browser spacing regression (2026-11-27)

CI run 46111602040 failed four panel-spacing cases: the new recording
permission explanation sat only 1 px above Authorize client. Use the existing
panel action-row class to reserve its standard spacing; no permission behavior
changes. The recording-opt-in browser suite now asserts an 8 px minimum gap in
all three themes. Verification uses that suite and the existing panel-spacing
suite on Windows Edge, plus UI typecheck, format and strict lint. Rollback:
revert this spacing change; no stored data changes.

Windows dev PC results: panel-spacing plus recording-opt-in 11/11 passed;
the added minimum-gap assertion then passed 3/3 in dark/light/high contrast.
Screenshots inspected in all three themes. UI typecheck, format and strict lint
passed. Both PR check annotations were read: each reports exit code 1 from
the failed browser job and its dependent gate. The browser console also reports
Node's conflicting NO_COLOR/FORCE_COLOR environment and React Flow attribution
warnings; neither caused this failure, and this spacing repair changes neither
environment flags nor licensing/attribution behavior. No guide control or audio
behavior changed.

A client given the opt-in can start recordings without the user pressing
Record. That is the owner's explicit choice; it is per client, off by default
and visible as "may record". The flag lives in `control_settings`, so an older
build ignores it (fail-closed). Rollback: revert this branch. Stored
`clientRecording:*` rows are then ignored, and `sessions.rename` disappears from
discovery.

## Evidence

2026-11-27, Windows 11 dev PC, Rust 1.96.0: both `cargo fmt -- --check` clean;
Clippy `-D warnings` clean (workspace and `src-tauri`); every workspace test
target passed, `src-tauri` 186 passed; contract drift (157 methods) and docs
validation (326 files) passed; UI lint, format, i18n check and typecheck clean;
UI unit tests 1204/1204; `ui/e2e/clients-recording-opt-in.pw.ts` 3/3 (dark,
light, high contrast; screenshots looked at). Not run: the built shell exe with
an MCP client given the opt-in (the real-pipe test covers the backend and
sidecar).
