# Wave Link parity candidates

Recorded 2026-10-22 at the owner's request, after comparing Elgato Wave Link
3.0–3.2.10 (Windows) with AudioRouter. These are candidates, not authorized
work, except the items the owner started on 2026-10-22 (marked
**in progress**; see the [active plan](../active/wave-link-parity.md)).
The public comparison is in `site/compare.html`.

Sources (checked 2026-10-22): [Wave Link 3.0 software overview](https://www.elgato.com/lm/en/explorer/products/wave/wave-link-3-0-software-overview/),
[Wave Link product page](https://www.elgato.com/us/en/s/wave-link),
[3.0.0 release notes](https://help.elgato.com/hc/en-us/articles/44408325264017-Elgato-Wave-Link-3-0-0-Windows-Release-Notes),
[3.1.1 release notes](https://help.elgato.com/hc/en-us/articles/46621898448017-Elgato-Wave-Link-3-1-1-Windows-Release-Notes),
[3.2 release notes](https://help.elgato.com/hc/en-us/articles/47156413046289-Elgato-Wave-Link-3-2-Windows-Release-Notes),
[Wave Link Stream Deck plugin](https://marketplace.elgato.com/product/wave-link-aa41150f-9645-4275-b064-7642ba6a17ae).

## Already covered (not gaps)

- Press-and-hold effects: the Stream Deck Toggle key has a `momentary`
  mode (on while held, restored on release).
- New-version notice: the header shows "x.y.z available" (UI-18). There is no
  automatic installer, by decision in `docs/operations/distribution.md`.
- Clipguard-style protection in software: the Limiter tool.

## Candidates

| # | Wave Link feature | AudioRouter today | Proposal | Effort and risk |
| --- | --- | --- | --- | --- |
| 1 | Listen to any mix (ear icon switches headphones to the Stream or Chat mix) | No solo/cue; rewiring needed | **Listen here** on a node or output: a temporary monitor path to the headphones, removed on stop | Backend: a temporary edge into an occupied output needs a mixer; must not touch saved revisions or protected voice paths. Medium–high |
| 2 | Virtual channels installed with the app | Needs VB-Cable; own cable in development | Already tracked by DEC-18 and the [virtual cable plan](../active/virtual-cable.md) | High (driver signing) |
| 3 | Channel × mix grid with send levels and mutes | Per-node mini faders on the canvas | Optional **Mixer view** of the same graph: sources × outputs, send level and mute per cell, edits through the same API | UI-heavy; no engine change. Medium |
| 4 | Several apps per channel, including apps not running; apps grouped by category | Implemented: browse/type a closed app's `.exe`; app groups expand into ordinary app sources feeding a Mixer (UI-23). Parent/helper tree overlap remains an open CAP-10 limitation. | Add recent-apps and app-category grouping after process-tree overlap detection can warn or deduplicate safely. | Medium |
| 5 | "Everything else" / All-audio source | One exclusion (CAP-07); several exclusions future | Desktop minus assigned apps | Blocked on a proven Windows mechanism (see the [future README](README.md)). High |
| 6 | Fades on Stream Deck "Set" actions | Step key jumps instantly | Fade over a chosen time | Each `nodes.set` is a saved plan/commit revision, so client-side fades would flood history and delay the audio pump. Needs a backend ramp or a non-revision live parameter path first. Medium |
| 7 | Soundboard clips from keys | Audio File play/pause/stop; no soundboard action | Stream Deck soundboard key bound to an Audio File node | Small–medium; see [Stream Deck extensions](stream-deck-extensions.md) |
| 8 | Stream Deck profile generated from the setup | Manual key setup | Generate a profile from the session's nodes | Medium; profile format and Marketplace rules |
| 9 | Mix-output actions (move headphones between mixes, toggle main output) | None | Pairs with #1 | Medium |
| 10 | Sound Check (record your voice and hear it through the effects) | **Implemented (UI-24):** record a temporary 10-second take from an already-routed Recorder; replay through effects on an Audio File path after saving the route. | A more integrated record-and-replay flow could remove the need to assemble an Audio File branch manually; the current flow still requires an existing Recorder branch and explicit route commit. | Medium |
| 11 | Level target zone; Auto Gain Wizard | Meter shows RMS/peak/hold, no target | **in progress**: voice target zone and a level verdict on the Meter (UI-21). Auto gain stays future (AGC row in the [future README](README.md)) | Target zone: UI only, small |
| 12 | Gain Lock; lock Windows default devices | AudioRouter never changes defaults | Optional guard that warns when another app changes a cable's level or the defaults | Needs Windows endpoint-volume notifications; medium |
| 13 | Rename, copy, paste and reorder effects across channels (3.1) | Duplicate one node | Copy a selected chain to another route; save your own chain as a preset (`presets.save` is specified in API but not implemented) | Medium |
| 14 | Hide unused devices from pickers (3.1) | Every active endpoint is listed | **in progress**: per-PC hidden devices, managed in Status › Audio devices on this PC (UI-22) | UI only, small |
| 15 | Import Wave Link 2 settings | Session import of AudioRouter files only | Import a Wave Link or Voicemeeter setup into a graph | Needs their file formats; medium |
| 16 | Voice Focus (AI noise and echo reduction) | Speech denoise; no echo cancellation | Echo cancellation for speaker users; model-based isolation is already a future row | High (licence, latency, failure policy) |

## Not pursued

- macOS and native ARM64: platform ports need separate approval (AGENTS.md);
  ARM64 is already a future row.
- Plugin marketplace with sign-in, and Elgato hardware controls (Clipguard,
  phantom power, VST Insert): tied to Elgato's own products and accounts.

## Suggested order

The not-running app picker and basic app groups (#4) were merged in PR #59.
Sound Check (#10) is implemented with the existing graph workflow; a simpler
in-panel experience remains an opportunity. Listen to any mix (#1 with #9)
remains the next larger workflow gap, followed by Mixer view (#3). The virtual
cable (#2) remains the strategic item.
