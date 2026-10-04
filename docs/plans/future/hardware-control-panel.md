# Hardware control panel (Raspberry Pi touch console)

Status: user-requested 2026-10-04. Option A (local-network HTTP) chosen.
Step 1, the local-network listener (HTTP-09), is implemented; see the
[active plan](../active/current.md), item 11. Everything else here is future
work and needs its own go-ahead before coding.

## Objective

A small dedicated console next to the keyboard to glance at and adjust a
playing AudioRouter session during a game without opening the desktop
window. Heavy configuration stays in the desktop app. The console must:

- show useful information for **any** session, including ones it has never
  seen, with no setup (see "Showing any session" below);
- let the user adjust a few properties quickly (touch, knobs, buttons);
- never be needed for audio to work. The PC backend owns everything; the
  panel is one more adapter (ARCH rule: UI/CLI/MCP/HTTP are adapters).

Window memory is saved because the editor can stay closed to the tray
(0.0.10 frees the WebView on close), not because the panel uses less.

## Hardware

- Raspberry Pi 4 Model B, 7" 1024×600 60 Hz capacitive touch display
  (about 154 × 86 mm visible, about 6.6 px/mm). Touch targets at least
  9 mm (60 px).
- Optional: 2–3 rotary encoders with push, 4–6 keyswitches or arcade
  buttons with LEDs, a separate large privacy-mute button with a red LED.
- 3D-printed wedge enclosure with a shaped bezel (below), USB-C power,
  ventilation near the SoC, a weighted or rubber base so turning a knob does
  not move it.

## Decisions

- **2026-10-04 (user): option A, local-network HTTP.** The Pi talks to the
  existing HTTP adapter over the home network. This approves the
  remote-control scope for: one extra HTTP listener on one private or
  link-local address of this PC that the user chooses, off at launch, same
  token and same desktop grant. Not approved yet: pairing, per-device tokens,
  a reduced "panel" grant, TLS, anything reachable from the internet, a
  second backend.
- Options B (USB gadget link) and C (USB MIDI/HID controller) are not
  pursued. C may return as a separate "MIDI learn" feature.

## Architecture

```text
 PC (Windows)                                   Raspberry Pi
 ┌──────────────────────────────┐   LAN HTTP    ┌──────────────────────────────┐
 │ backend (audio, graph, rules)│◄─────────────►│ panel agent (Node or Python) │
 │  ▲ named pipe                │  bearer token │  - holds the token in a file │
 │ HTTP adapter                 │               │  - GPIO: knobs, buttons, LEDs│
 │  127.0.0.1:17891 (always)    │               │  - polls/forwards, coalesces │
 │  192.168.x.y:17891 (opt-in)  │               │  - serves panel page locally │
 └──────────────────────────────┘               │ Chromium kiosk → 127.0.0.1   │
                                                └──────────────────────────────┘
```

Recommended: a **panel agent** on the Pi, with Chromium in kiosk mode
showing a page that the agent serves on the Pi's own loopback. Why:

- The token stays in a root-readable file on the Pi, never in browser
  storage (HTTP-03 forbids browser storage), and the agent sends no browser
  `Origin`, so the PC's exact-Host and same-origin checks stay unchanged.
- GPIO, LEDs and the screen share one process that knows the backend state.
  A button LED shows the backend's state, not the button's last press.
- Knob turns are coalesced on the Pi (see "Rate and history" below).
- Nothing on the PC changes for the prototype beyond HTTP-09.

The alternative is for the PC backend to serve a `/panel` page itself. That
keeps the panel versioned with the app, but needs a token story in the
browser (pairing) and a GPIO bridge anyway. Revisit after the prototype.

### Data the panel uses (all existing)

| Need | Call | Notes |
| --- | --- | --- |
| What is in the session | `POST /api/v1/sessions/summary` | names, kinds, readable settings, connections by name, playing, privacy mute |
| Which sessions exist / selected | `GET /api/v1/sessions`, `GET /api/v1/sessions/active` | |
| Levels | `POST /api/v1/meters/levels` | peak/RMS dBFS, clipping, gain reduction (Gate, Compressor, Limiter, Duck) |
| Health | `GET /api/v1/status`, `system.diagnostics` | run state, late gaps, plugin and device state |
| Ranges and choices | `POST /api/v1/nodes/catalog` | builds controls without knowing node kinds in advance |
| Change one thing | `nodes/set`, `nodes/toggle`, `safety/togglePrivacyMute`, `sessions/togglePlay` | by node name, idempotency key |
| What the user tweaks most | `graph.history` | ranks parameters (see below) |
| Change notifications | `events.subscribe` with a cursor | so the panel refetches the summary only after a change |

Read-only calls cost 0.1 of the HTTP budget (about 200/s), so polling levels
at 10–15 Hz is fine.

### Rate and history (needs backend work before knobs)

Every `nodes.set` is a durable, saved revision. Mutations refill at 20/s
(burst 40), and undo keeps 100 commits per session. A knob sending every
detent would fill undo with one gesture. Plan:

1. Prototype: the agent sends at most about 5 values/s while turning and the
   final value on release. Acceptable but noisy in history.
2. Proper fix (backend, new requirement): a **live gesture** method. Live
   values apply to the running graph without saving, then one commit on
   release, with a timeout that commits or reverts if the panel disappears.
   The desktop inspector sliders would benefit from the same.

## Showing any session (no prior knowledge)

Every user builds a different graph. The panel builds its screens in three
layers: **derive** a sensible default from the graph, **rank** what fits, and
let the user **curate** it. Layout is rebuilt only when the session
revision changes, never because of a meter tick (UI-17: live data does not
move things).

### 1. Derive: understand the graph from its shape

From `sessions.summary` alone:

- **Roles by kind and position.** Sources: Physical Input, Application
  Capture, Endpoint Loopback, Network Receive, Virtual Render Source, Test
  Signal, Audio File. Destinations: Physical Output, Virtual Capture Sink,
  Network Send, Recorder. Everything between is processing.
- **Strips.** One strip per source, like a mixing console channel: its
  name, a meter taken at the last metered point before a Mixer or output,
  and its main control. A Mixer becomes a **fader bank** (its
  `inputVolume:<input>` parameters, labelled by upstream name, as the API
  request builder already does).
- **Main control per kind** (falls back to Enabled/Bypass):

  | Kind | Main control | Shown |
  | --- | --- | --- |
  | Gain, Volume | level (knob/fader) | dB or % |
  | Mixer | one fader per input | fader bank page |
  | Mute | muted toggle | lit button |
  | Input Switch | A/B toggle | the two upstream names |
  | Duck | amount | live reduction |
  | Compressor, Gate, Limiter | threshold | live reduction meter |
  | EQs, Bass/Treble, Denoise, Dehum, Pitch, plugins | bypass toggle | on/off |
  | Recorder | record/stop | elapsed time, red dot |
  | Physical/virtual output | none | meter, device-missing alert |

- **Alerts** derived from status and diagnostics: privacy mute on, plugin
  failed (voice path silent by design: offer the recovery action), device
  missing, clipping latch, a playing session with silent outputs.

### 2. Rank: decide what fits on 1024×600

Score each candidate control, highest first, deterministic order on ties:

1. Safety and transport: privacy mute and Play/Stop are always present.
2. **Parameters the user actually changes**, counted from recent
   `graph.history` revisions (`node.setParameter`, `setBypass`). This is the
   best signal for an unknown session: if the user keeps adjusting "Discord
   volume" and "Footstep EQ" bypass, those are the panel's knobs.
3. Mixer inputs and Volume/Gain on source strips.
4. Nodes on paths that carry signal (from levels, sampled over a minute,
   so order does not flicker).
5. Everything else stays reachable on a "All tools" page, grouped by strip.

Names come from the user's node names (they already name things for the
canvas). Long names are truncated at a fixed width and full names appear on
long-press.

### 3. Curate: let the user correct it

- **Pin to panel** in the desktop inspector (and long-press on the panel):
  pin, unpin, reorder, assign a control to a physical knob or button.
- Stored by the backend per session as a presentation resource, like the
  canvas layout ([10 API](../../spec/10-api.md): UI layout is a separate
  presentation resource, so it creates no audio revisions), so any adapter
  can read it. Visual groups (UI-16) are local
  canvas data today. If they move to the backend, a group could become a
  panel page.
- An empty pin list means "use the derived layout", so a new session works
  immediately and curated ones stay as the user left them.

### Pages

- **Overview**: strips with meters and main controls, status bar.
- **Mixer**: fader bank of the busiest Mixer (swipe for others).
- **Focus**: one tool large (for example compressor reduction history).
- **Health**: run state, late gaps, device and plugin alerts with actions.
- Ambient: dim after idle to meters only; wake on touch or knob.

## Shaped bezel (faceplate)

The printed front can cover parts of the screen. The visible openings do not
need to be rectangles. The software therefore renders into **regions**, not
a full-screen layout:

- A **faceplate profile** (JSON, in screen pixels) lists each opening with a
  shape (rectangle, rounded rectangle, circle, arc or SVG path) and a role:
  `strip`, `meter`, `lamp`, `softKeyLabel`, `focus`, `status`, `touch`.
  The derived/ranked controls fill regions by role and priority. Anything
  outside every opening is never drawn and touch there is ignored.
- A **calibration screen** shows the openings as outlines and a grid; the
  user nudges an X/Y offset so the print and the pixels line up.
- Ideas the shape enables:
  - **Screen as a status lamp.** A translucent (white PETG/PLA) window over a
    small screen area glows the screen's colour: red when the mic is live,
    amber when muted, off when stopped. No LED wiring.
  - **Soft-key labels.** Small windows directly above physical buttons show
    each button's current assignment and state ("Footstep EQ ✓").
  - **Arched meter window** shaped like an analog VU meter.
  - **Round windows** next to each encoder showing its value ring and name.
  - **Embossed or recessed labels** printed into the bezel for fixed things
    (MIC, PLAY), the screen only shows state.
- Ship two profiles to start: `full` (whole touch screen) and `console`
  (strips + lamp + soft-key labels for 4 buttons and 3 knobs).

## Phases

1. **Done 2026-10-04:** local-network listener (HTTP-09). See active plan.
2. **Prototype (no product change):** Pi OS Lite, Chromium kiosk, panel
   agent with the token in a file, Overview page from the derived layout,
   privacy mute and Play/Stop, levels at 10 Hz. Try it in real matches.
3. **Rank by history:** add the `graph.history` scoring and the
   All tools page. Measure: on the user's session, the top 6 controls match
   what the user would pick.
4. **Backend: live gesture method and panel pins** (new requirements, spec
   and API change). Then knobs.
5. **Pairing and a reduced panel grant:** desktop shows a code, the Pi
   exchanges it for its own revocable token with only read, graph parameter
   writes, session control and privacy mute (never device administration,
   recording roots or plugin scans). Listed and revocable in the API tab.
   Start the API automatically at launch when a paired panel exists.
6. **Hardware:** faceplate profiles, calibration, GPIO buttons/encoders/LEDs,
   enclosure.

## Requirements (proposed, not yet in the specification)

- HTTP-09 (implemented): local-network listener, see
  [16 Local HTTP](../../spec/16-local-http-api.md).
- PANEL-01 derived layout from any session; PANEL-02 history ranking;
  PANEL-03 backend panel pins; PANEL-04 live gesture parameter changes;
  PANEL-05 pairing and panel grant; PANEL-06 faceplate regions and
  calibration; PANEL-07 physical controls with state-driven LEDs.

## Risks

- **Plain HTTP on the LAN.** Anyone on the network who captures the token
  controls AudioRouter (including Play/Stop and unmuting). The API tab says
  so. Pairing with per-device tokens (phase 5) limits damage; TLS needs a
  certificate story the Pi must trust.
- **Changing addresses.** DHCP can give the PC a new address; the listener
  then fails to start with a clear message. Recommend a DHCP reservation.
- **Windows Firewall** must allow AudioRouter on private networks once.
- **Unmute from the panel** is a privacy-reducing action from another
  device. Decide in phase 5 whether the panel may unmute or only mute.
- **Pi power and heat** in a closed enclosure; 3 A supply and ventilation.

## Rollback

Choose "This PC only" (or Stop API) in the API tab; the network listener is
never started at launch. Reverting HTTP-09 restores loopback-only behavior;
no stored data depends on it.
