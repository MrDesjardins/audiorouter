# First-time user study: script and observation sheet (2026-11-24)

Status: **prepared, not run.** The M08 first-time-user criterion still needs
five real Windows participants. No participant was recruited or observed by
this agent; no score is claimed.

Requirement: M08 (`docs/spec/01-product.md`, "at M08, at least four of five
first-time Windows participants shall complete the primary template within ten
minutes ... Every participant shall correctly identify which sources Discord
receives"), plus UI-03, UI-09 and UI-28. Source plan: Round 28 "Clarity (3)"
and Round 30 "First-time-user walkthrough".

## What the owner needs to provide

1. A built release shell newer than `ui/dist` with the change under test
   (`tools/build-desktop.ps1`), run on a disposable database and WebView2
   profile:
   - `AUDIOROUTER_DATABASE=<temp>\study.sqlite`
   - `WEBVIEW2_USER_DATA_FOLDER=<temp>\webview`
   - no `AUDIOROUTER_ALLOW_DEVICE_ADMIN` (the participant answers the in-app
     device-access question themselves).
2. A quiet room and a screen recorder or a second observer who writes the
   sheet. Record only the app window; no participant microphone audio is
   captured or stored.
3. Ten minutes per participant plus five minutes of briefing. Five
   participants, none of whom have used AudioRouter or seen its guides.
4. The Discord desktop app installed and signed in on the test PC, and at
   least one working microphone and one pair of headphones. Virtual cables
   installed if the build under test expects the user to create them; record
   which cables were present.

## Moderator script (read verbatim)

Before starting:

> "Thank you for helping. AudioRouter routes audio on your computer. I am
> testing the app, not you. Please think aloud: say what you are looking for
> and what you expect. Use the app as you normally would. I will not answer
> questions about how to use it; if you are stuck for three minutes I will
> note it and move you on. This takes about ten minutes. The screen is being
> recorded, and no audio from your microphone is saved. You can stop at any
> time."

The task, shown on a card and read aloud:

> "Make my microphone sound cleaner and send it to Discord. Then tell me which
> source Discord receives."

Start the timer when the participant first touches the app. Stop it when the
participant says they are finished or at ten minutes, whichever comes first.

Do not:

- point at a tab, tool, goal card or device;
- name the virtual cable Discord should listen to;
- explain the canvas, the goal dialog or Properties;
- answer "is this right?" with yes or no; say "what do you expect?" instead.

Interventions allowed, each recorded with its time:

- at three minutes stuck on one step: "What have you tried? What else could you
  try?"
- at eight minutes: "You have two minutes left. Finish what you can and tell me
  what is left."

Ending questions (record answers verbatim):

1. "Which device does Discord use as its microphone, and which AudioRouter tool
   sends audio there?" (expected: the virtual cable AudioRouter renders to; the
   route's output device named for Discord or the cable.)
2. "Which part of AudioRouter changes your voice, and which part only moves
   audio?" (expected: Speech Denoise/Gate/Compressor, then the cable/output.)
3. "What was the most confusing moment?"
4. "Was there anything you expected to find and did not?"

## Observation sheet

Fill one row per participant. Times are mm:ss from first touch.

| Field | P1 | P2 | P3 | P4 | P5 |
| --- | --- | --- | --- | --- | --- |
| Participant id / date | | | | | |
| Windows experience (casual / regular / power) | | | | | |
| Audio-routing experience (none / some / expert) | | | | | |
| Build hash and AudioRouter version | | | | | |
| Microphone and headphones used | | | | | |
| Virtual cables present (names) | | | | | |
| Completed task (yes / no) | | | | | |
| Time to complete (mm:ss) | | | | | |
| Found a starting goal or template (yes / no, which) | | | | | |
| Created the mic clean-up chain unaided (yes / no) | | | | | |
| Chose the virtual cable for the Discord output (yes / no) | | | | | |
| Chose the correct Discord input, or said it must be changed (yes / no) | | | | | |
| Identified Discord's received sources (correct / partly / wrong) | | | | | |
| Hesitations (time + on-screen element + quote) | | | | | |
| Wrong turns and recoveries | | | | | |
| Errors shown by the app (text) | | | | | |
| Moderator interventions (time + kind) | | | | | |
| Verbatim quotes worth keeping | | | | | |
| Devices already bound on first launch (yes / no) | | | | | |

Hesitations to watch for (from Round 30's agent rehearsal):

- no single goal matches "clean my mic and send it to Discord";
- "Which game should play in your headphones?" appears when the participant
  did not ask about a game;
- three bindings start as "No device" and the user must choose each;
- the virtual-cable list does not say which cable Discord listens to;
- the participant expects a "Discord" device rather than a CABLE name.

## Pass criteria

- **Completion:** at least four of five participants finish the task within ten
  minutes, measured from first touch to "finished".
- **Source identification:** all five participants correctly identify the
  virtual cable Discord receives and that AudioRouter's output feeds it.
- **No coaching:** no participant receives help beyond the scripted
  interventions above; any unscripted help is a failure for that participant
  and the study needs a replacement participant.
- A failed target requires a documented UX revision and a retest; it is not
  relabeled as experienced users.

## What to record

- Append a dated result section to `docs/plans/archived/2026-11-24-ui-quality-review.md`
  with the table, pass/fail against the criteria, and the exact build hash.
- Keep recordings and sheets outside the repository; the repository gets only
  the de-identified table and quotes with participant consent.
- Do not record participant names, accounts, microphone audio or Windows
  user paths. Use P1..P5 identifiers.