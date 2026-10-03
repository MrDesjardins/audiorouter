# Active plan — post-0.0.5 maintenance

Updated 2026-10-03. v0.0.6 release in preparation (user-requested); checks pass, see [0.0.6 evidence](evidence/2026-10-03-release-0.0.6.md).

## Objective and scope

Track support and remaining qualification after v0.0.5. No new implementation
is authorized by this plan. Preserve sessions, local API credentials, device
formats and published assets; no VST changes. Requirements DSP-19, GRAPH-08/14/15,
UI-05/11/12/13, AUTO-15, ARCH-04, SEC-01/10, DIST-01–08 remain traceable through
[release evidence](evidence/2026-10-02-release-0.0.5.md).

## Where things stand

[v0.0.5](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.5) is
public, explicitly unsigned/experimental/prerelease. Source and tag point to
6e8148fcfcb6b4b85c47d3e007d8149b17ca2ae0; documentation updates are pushed separately.
Locked workspace, shell, 453 UI, contracts/example and 40 production Edge checks
pass. Exact packaged first-run passes 1/1 after user app closure. Public assets
were downloaded and all hashes/provenance verified. No live cable test or device
format change in the publication turn.

## 2026-10-03 — Stable dynamics suggestion (user-requested, UI-05/UI-17)

Defect: the Compressor/Gate (and Duck) threshold suggestion box appeared and
vanished as its estimate flickered, moving the panel below it (124 px in the
Edge harness on the previous code). The user also questioned a −47 dB
compressor suggestion.

Decisions: new spec rule UI-17 (stable layout) and AGENTS.md UI convention.
The suggestion slot is always rendered at a reserved height with a waiting
text and disabled button, and keeps the last good measurement while readings
continue. The status line reserves three lines and the pill has a minimum
width. Suggestion math: the voice level is measured from readings ≥10 dB above
the noise floor (it no longer sinks when the user talks briefly); the compressor
threshold stays within 12 dB of voice peaks and in the upper half of the
noise-to-voice range, reports the reduction it actually gives ("raise Ratio
for more"), and recommends turning the microphone up when voice peaks are under
−24 dBFS. Readings are peak levels, so a quiet microphone legitimately gives a
low absolute threshold.

Verification (Windows, Edge): `npm.cmd test` 46 files/456 tests pass; `tsc --noEmit` clean; Playwright
`dynamics-editor.pw.ts`, `duck-widget.pw.ts`, `inspector-stability.pw.ts` 32
passed, including a new no-shift assertion in dark, light and high-contrast
(fails on the previous code with offsets `[526, 650]`). Screenshots reviewed
in all three themes. Not checked in the native shell or with a live microphone.
Rollback: revert the UI change; no data or contract change.

Follow-ups the same day: (1) drag-to-canvas did nothing in the desktop shell
(label shown, no preview, no drop) while Edge tests passed. Cause: Tauri's
native drag-drop handler swallows HTML5 drag events in WebView2; the main
window now calls `disable_drag_drop_handler()` (no UI uses OS file drops).
Needs attended confirmation in the shell; browser e2e cannot detect it.
(2) The Gate suggestion now keeps its closing point (threshold − hysteresis)
≥6 dB above room noise and the threshold at most halfway to the voice, and
states where the gate closes. `npm.cmd test` 457 pass; `tsc` clean;
dynamics/drag-place/duck Playwright 13 pass; release shell rebuilt with
`custom-protocol` at 13:37, embedding `index-BtvRU4mQ.js`.

## 2026-10-03 — Device picker showed a borrowed device (support report, UI-05/UI-08)

Report: a user's friend on v0.0.3 pressed Play and got "No audio started.
Choose the device for SMSL DAC, Mic Desktop Input in Properties, save, then
press Play." Their logs (discovery.jsonl, shell.jsonl; kept outside the repo)
show: a ghost render endpoint (index 37, `0xE000020B`) broke `devices.list`
before 0.0.3 and is skipped since (28 devices listed); one `graph.commit`, then
no prepare/start request, i.e. the UI refused Play because both saved device
nodes had no `endpointId`. Cause (also in 0.0.5): the Physical Input/Output
pickers displayed the remembered app-wide endpoint when the node had none, so
selecting that visible device fired no change and Save stored no device.
Workaround given: choose another device, then the correct one, Save, Play.

Fix: pickers and the Surround check use only the node's own `endpointId`; an
unbound node shows the empty choice and "No device is chosen for this node
yet. Choose one, then Save." (themed for dark/light/high contrast). Regression
`ui/e2e/device-binding.pw.ts` (3 themes) fails on the previous code (value
`render-preview` instead of empty) and passes now. `npm.cmd test` 457 pass,
`tsc` clean; device-consent, live-controls, media-and-routing, banner,
spatial and device-binding Edge suites: 20 pass, 2 fail identically without
this change (pre-existing: `live-controls` "refuse unsaved topology edits"
gets a saved message; `media-and-routing` occupied-output finds two Undo
buttons). Those two need separate investigation. Single-path routes still
play on the remembered binding when the node has none; unchanged.

## Remaining qualification and next action

Real-match Duck/failure release, native Quit process exit, combined quiet-tone
continuity, live path-change and spatial listening remain open. Full M08 clean-
machine install/upgrade/uninstall, WebView2 absence, hardware/endurance,
accessibility and signing remain open and disclosed. These are not implied by
portable tests or this experimental publication.

Exact next task: finish v0.0.6 (package, draft, exact packaged first run with
AudioRouter closed, publish, verify downloads); then user testing/support. On a
new request, plan attended/native
qualification with a quiet reference and closed competing apps. Do not run new
implementation or disturb the user's audio autonomously after this release.

## Risks, evidence and rollback

Stats.cc's undocumented feed can change; unknown state releases Duck. Independent
clock drift/queue latency remain limits. Keep v0.0.4 and compatible configuration/
recording backups for rollback; no migration or driver install. Published assets
are immutable; repairs use a new version.

Completed publication history: [execution record](../archived/2026-10-02-release-0.0.5-published.md).
