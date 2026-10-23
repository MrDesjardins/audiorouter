# Code review follow-ups after 0.0.21 — 2026-10-23

A quick review of every change from `v0.0.20` to the 0.0.21 release branch
(plugin windows, Output Switch, Sound Check, app sources and groups, hidden
devices, the dedicated audio thread, canvas layout) found nine issues. Five
were fixed before the release; the four below are recorded here so they are
not lost. None is authorized work until the owner picks it up.

## Fixed in 0.0.21

| Finding | Fix | Regression |
| --- | --- | --- |
| The plugin bridge sent its slider list with every quantum, so a knob turned in a VST3 editor snapped back within ~3 ms whenever the node had saved slider values | Sliders are sent only when their list changes (`changed_parameters`) | `slider_events_are_sent_once_per_change_so_editor_edits_survive` |
| An Output Switch side feeding a node that mixes several feeds either leaked while unselected or muted the other feeds | The multi-path compiler refuses that layout (`UnsupportedTopology`) instead of playing it wrongly | `output_switch_side_into_a_shared_mixer_is_refused_rather_than_leaked` |
| A one-output route through an Output Switch was sent to the single-endpoint worker, which rejects the switch | `needsNativePaths` selects the multi-path worker for any connected switch | `uses the multi-path worker for a route through an Output Switch` |
| The start-up latency query ended the plugin worker when the plugin could not report a delay | A worker `Failure` answer counts as no delay; pipe errors still end the worker | existing latency tests |
| Two doc comments were attached to the wrong functions (`load_plugin_state_asset`, `continuity_counts`) | Moved back | — |

## Open

### R1 — Mix an Output Switch side with other feeds (feature gap)

The fix above refuses a switch side that goes into a Mixer shared with other
sources. Supporting it needs per-feed gating inside the connected-Mixer
compiler (`crates/engine/src/connected_mixers.rs`): gate each incoming edge
before the sum, and carry inherited gates per feed instead of per node.
Prerequisite: tests that check the unselected side is silent and the other
feeds keep their level, in both switch positions and during the
break-before-make transition.

### R2 — Meters and processors after the unselected switch side still see audio

`crates/engine/src/lib.rs` applies the switch gate at the final sink, after
branch processing. A Meter, Duck key or clip indicator on the unselected side
reports live speech while its output is silent. Move the gate to the switch's
output edge (before downstream processing), or zero the branch input, and
regress with a Meter on side B while A is selected.

### R3 — Recorder taps can overflow while the control thread is blocked

Since the dedicated audio thread (`crates/control/src/audio_service.rs`),
capture and playback keep running while the control thread handles a slow
request, but recorder tap queues are still drained only by the control
thread's service pass. A request that takes hundreds of milliseconds without
`while_servicing_audio` can drop recorded audio (gaps in the WAV). Drain taps
on the audio thread or a dedicated writer thread, and qualify with the
multi-path recording lesson in AGENTS.md (parse the WAV, inject a slow
request).

### R4 — Occupied-input connections always insert a Mixer; dead replacement code

PR #60 made a connection to an occupied input add a Mixer. The old
"Replace input connection" confirmation and the reversed-drag direction help
are no longer reachable, but `connectionReplacement` state and its UI remain
in `ui/src/App.tsx`. Decide whether a backwards drag should still explain the
direction instead of silently mixing, then remove the unreachable code.

### R5 — Duplicated COM file-dialog code

`src-tauri/src/session_file_dialog.rs` `choose_executable`/`show_executable`
copy the thread, COM initialisation, dialog options and CoTaskMem
copy-then-free of `choose`/`show`. Parameterise one helper with the filter and
title so a future fix to the unsafe code cannot miss a copy.

### R7 — Edge suite flakes on hosted runners

During the 0.0.21 merge (2026-10-23) five CI runs each failed one different
Edge test that passed on rerun and locally (9/9 repeats for the first):
`compact-workspace` "debounced live autosave … in dark",
`session-workflows` "… survive backend restart" (after Delete session),
`canvas-layout-review` "new connections keep both chosen sides …" and
`plugin-inspector` "… in high-contrast" (two harness loads in one 30 s
budget; now 60 s). Find each test's wait on timing rather than state, and
consider Playwright retries with a flake report instead of whole-job reruns.

### R6 — Site comparison wording for VST2

PR #57 changed `site/compare.html` from "Load x64 VST2 and VST3 effects …
and open their editors live" to "a gated x64 VST2 adapter is available for
selected cases". The owner asked on 2026-10-22 to keep the original claims and
make them true; with 0.0.21, VST3 and UADx editors open and VST2 effects load.
The overnight session's attempt to restore the wording was blocked by the
permission check, so the owner should decide the wording.
