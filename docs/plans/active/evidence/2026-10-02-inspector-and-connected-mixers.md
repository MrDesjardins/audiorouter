# Inspector isolation and connected native Mixers

Date: 2026-10-02. Windows 11/MSVC, Node 22, optimized Edge UI harness.
Requirements: UI-05/11/12, GRAPH-01/02/04/14/15, CAP-13, ARCH-04.
Release v0.0.4 publication remains paused. No VST implementation changes.

## Reproduction and repair

The unpublished 0.0.4 candidate / preceding ID-footer review left the
Compressor live view mounted when selecting EQ or Meter. The inspector heading
and Node ID changed correctly. Three-theme browser transitions reproduced it.
The NodeIdentity footer and the live editor were siblings keyed by the same
node ID. Removing the redundant outer footer key fixes reconciliation; the
footer still resets its feedback through its internal identity key. Move both
parameter-editor contexts before conditional returns to preserve hook order.

The new browser regression repeatedly switches Compressor, EQ, Meter and Gate
while a running fixture refreshes. It checks matching IDs, presence/absence of
dynamics views and timing controls, and absence of page errors. Initial slider
assertions were too strong for the fixture's empty discovery descriptors and
were corrected; the corrected regression failed in all three themes before the
key fix, and passed after it. Final production browser suite: **21 passed**.
UI typecheck/build and **443 unit tests / 44 files passed**. EQ and Meter
screenshots were inspected in dark, light and high contrast; each shows its
own appropriate inspector. Screenshots are under the local Playwright results
directory. Durable screenshots: EQ in
[dark](screenshots/2026-10-02-inspector/dark-parametricEq-properties.png),
[light](screenshots/2026-10-02-inspector/light-parametricEq-properties.png),
[high contrast](screenshots/2026-10-02-inspector/high-contrast-parametricEq-properties.png);
Meter in [dark](screenshots/2026-10-02-inspector/dark-meter-properties.png),
[light](screenshots/2026-10-02-inspector/light-meter-properties.png),
[high contrast](screenshots/2026-10-02-inspector/high-contrast-meter-properties.png).

Engine/control package suites passed: **220 control tests (8 ignored), 155
engine tests (1 private-session opt-in ignored), 33 deterministic audio tests,
7 tool route tests**, with doctests also passing. After the final readiness
refinement, library suites again pass **220 + 155**. The exact revision 214
compile-only opt-in was run separately and passed 1/1. Local logs:
`target/connected-mixers-tests.log`, `target/connected-mixers-final-unit.log`.

Patrick's reported revision 214 adds a second Mixer combining the microphone
Meter output and the game/Discord Mixer output. The previous native compiler
allowed only one convergence per connected component and rejected that route
as UnsupportedPath. Saving succeeded while activation retained the old route.
No session rewiring was performed. Current user revision subsequently became
215; the first compile-only check selected that newer graph (which compiled
but had two components, failing the fixture's one-component assertion). The
exact revision 214 history copy then passed: **3 sources, 3 outputs, 1 component**.
The database was opened read-only; private JSON copies remain outside Git.

Components with several Mixers/Input Switches now compile a topological cache
using existing nonblocking DSP gates, output rings and branch taps. Sources are
captured once, shared tools run once, and convergence stages sum only authored
cached parents using their own edge maps/input volumes. No flattening through
nonlinear tools, recursive runtime traversal, allocation or device opening is
introduced in the processing loop. Buffers/readiness are prepared and bounded
off-thread; unavailable buffers cannot replay preceding microphone quanta.
Single-convergence routes retain the existing compiler. No schema migration.

Five focused regressions pass: exact three-source/two-Mixer/three-output branch
separation and 30/100/30 game levels, one execution per shared tool, disabled and
bypassed convergence, disabled microphone, busy cache silence, privacy latch,
same-identity live replacement and generated-source transport discovery.
Initial fixture validation failed because its synthetic application omitted
required identity fields; those were supplied before testing signal behavior.
An intermediate readiness refinement failed compilation (Result propagation
inside an Option closure); corrected explicitly without discarding the error.

## Windows audio

User closed AudioRouter, Siege and integration normally before native checks.
Only exact virtual cables, generated 47 Hz tone, a stopped generated source and
the fixture's own silent hidden helper were used; no user mic/app audio captured.

Command environment: `AUDIOROUTER_LIVE_CONTINUITY=1`,
`AUDIOROUTER_CONTINUITY_SECONDS=30`, `AUDIOROUTER_CONTINUITY_TONE_HZ=47`,
`AUDIOROUTER_CONTINUITY_CHAIN=gain,parametricEq`,
`AUDIOROUTER_CONTINUITY_APPLICATION_LIVENESS=1`,
`AUDIOROUTER_CONTINUITY_PRE_MIXER=1`; then
`cargo test -p audiorouter-transport --test live_audio_continuity
live_backend_service_keeps_a_routed_tone_continuous --locked -- --ignored --nocapture`.
This uses two connected Mixers/three inputs and a separate pre-Mixer Recorder.

First 30-second run: clean reference, routed output and parsed Recorder WAV,
each **0 glitches / 0 silent runs / 0 discontinuity flags**. Concurrent heavy
deterministic tests/builds produced 249 service late gaps (max 11493 us) and
4 aggregate output underruns. This passes measured sample continuity but does
not qualify an idle scheduling result. Repeat after competing checks finish.
Local log: `target/connected-mixers-continuity.log`.

After the user closed PID 20904, final debug-profile repeat failed with a clean
reference/Recorder: routed 2 glitches and one 1200-sample silence at 0.110–0.135 s;
5 service late gaps, max 20609 us, 2 output underruns. Unchanged repeat failed
again at 18.078–18.102 s: 2 glitches/1104 silent samples, clean reference/Recorder;
12 late gaps, max 21077 us, 2 underruns. It is not limited to startup.
Logs: `target/connected-mixers-continuity-final.log`,
`target/connected-mixers-continuity-repeat.log`.

Single-convergence baseline with the same app source and Recorder, but no
pre-Mixer/generated input, also failed: 42 routed glitches and 1 Recorder glitch;
12 service late gaps, max 24413 us, 6 underruns. This does not isolate a new-cache
regression. Preserve these failures; do not qualify overall continuity from the
earlier passing run. Log: `target/connected-mixers-continuity-baseline.log`.
Next comparison uses `--release` on the exact two-Mixer fixture, matching the
review executable's optimized processing profile; no production code changes.

First optimized run is inconclusive: reference, routed result and Recorder each
contain the same 367-sample silence / two boundary glitches, at successively
delayed positions through the route. No additional routed discontinuity was
detected, but the required clean-reference gate was not met. Service: 4 late gaps,
max 19017 us, 4 aggregate underruns. Log:
`target/connected-mixers-continuity-optimized.log`. Repeat unchanged optimized
fixture before reporting a native pass; do not discard the reference failure.

Unchanged optimized repeat failed with a clean reference and Recorder: 2 routed
glitches / one 76-sample silence at 2.868–2.869 s. Service: 7 late gaps, max
18128 us, 4 aggregate underruns. Tool processing averages ~2.2 us, maxima
64.5/34.9 us; the available evidence points toward render delivery/service
scheduling, but does not prove its cause. Log:
`target/connected-mixers-continuity-optimized-repeat.log`.
**Final native continuity gate failed. Do not publish or describe the audio
repair as qualified.** Next repair task: correlate source/ring/device queue
occupancy and service/request duration against output discontinuities, isolate
render padding/recovery from synchronous control/recorder work, repair the owning
layer and repeat with a clean reference. Do not increase latency blindly or
drop failed samples from analysis. UI checks and compile-only topology evidence
remain valid; long-duration and real gameplay evidence also remain open.

## Remaining work and rollback

User explicitly requested a test executable despite the known dropout while the
repair continues, with release urgency. Prepared a separate stable test copy at
`target/reviews/release-preview-20261002/audiorouter-shell.exe`, accompanied by
the matching worker, README with launch/integration steps and known issue, and
SHA256SUMS.txt. Shell hash matches the final review below; worker SHA-256:
`864FE2B2764D8C57B831DDB6D43417A7BDD080EC731BD45DFF60C26C3444E225`.
This is an explicitly authorized experimental handoff, not a qualified release;
no published asset or user process was changed. Preserve this copy during repair.
Portable investigation confirms control dispatch and recorder draining share the
audio service thread; MMCSS and a 1 ms timer request are already configured.
These are potential stall sources, not a proven root cause. Next diagnostic slice
must distinguish control duration, audio-pump duration and scheduling/wake delay
against render ring/device occupancy without logging from the processing path.

Final custom-protocol release-mode review build passed:
`AUDIOROUTER_BUILD_ID=review-connected-mixers-20261002`,
`cargo build --manifest-path src-tauri/Cargo.toml --release --features
custom-protocol --locked`. UI bundle `index-CGAcjvR6.js`/index built at 15:57:17;
final shell built at 16:01:44. Review executable:
`target/reviews/connected-mixers-20261002/audiorouter-shell.exe`, SHA-256
`4F41049C720D9E6410915B05B013C64219B8516E83A3D846ECE08F47E77222FD`.
Matching plugin worker accompanies it; its implementation is unchanged. Final
desktop shell tests pass **44/44**, with 1 attended fixture ignored. No
installer or release publication. Documentation acceptance: 95 files/498 links;
diff whitespace check passes. The user reopened the old review as PID 20904;
user closed it normally and the final checks above were run. No second shell
was launched; the fixture used its own isolated backend and released its streams.

Engine/control/shell checks and hashed review build are complete. Native
continuity remains failed; repair render delivery/service scheduling as outlined
above before audio handoff. User review of real session/gameplay remains required.
v0.0.4 assets/tag prepared before these repairs are superseded; no publication.
Revert the engine cache/compiler extension and UI key/hook changes to roll back.
Saved session schema/bindings and API token remain unchanged. Source/output
identity edits still need Stop/Play; parameter changes can retain prepared streams.

## Recorder stall repair and combined qualification update

Optional bounded numeric timing observer identified recorder drain work at
14.9–16.8 ms during 15.1–17.1 ms service passes; control dispatch stayed below
3.5 ms. Clean reference with 32 routed glitches reproduced this in
`target/connected-mixers-recorder-phase-probe.log`.

Factory WAV/FLAC/MP3 encoders now own a dedicated bounded worker thread.
Routine service reads nonblocking progress/errors. Lifecycle completion remains
durable; pooled exhaustion counts dropped tails and marks failed playable
prefixes. Tests cover a blocked writer, nonblocking queries, overflow and parsed
WAV output. The first threaded run removed glitches but failed recording due
to sleeping between positive batches; corrected catch-up consumes bounded
batches without idle sleeps while work remains.

`target/connected-mixers-threaded-recorder-catchup.log`: optimized Windows
47 Hz/30-second connected-Mixer run passes; reference, output and WAV have zero
glitches/silence runs, no late service gaps, max service 1.1721 ms. Aggregate
output underruns 4; no universal zero-underrun or hardware-soak claim.
This pass predates the final spatial and authority changes.

Combined source checks pass UI 445, shell 44 (1 ignored), example 11,
contract drift and 24 production Edge cases. Spatial screenshots reviewed in
dark/light/high-contrast. Locked workspace rerun passes (log:
`target/release-004-combined-workspace-final.log`); final control rerun passes
224 tests with 8 ignored (`target/release-004-final-control.log`). Dropped-tail
pool regression passes separately; docs 97 files/514 links and release path-safety
checks pass. Automatic device restart
now checks caller authority; stopped restart failures report stopped activation.

Final combined native qualification remains pending: CABLE-B is now 8-channel;
stereo fixture rejects its format, alternate pin reports DeviceInUse. Spatial
loopback prepares and records, but gameplay contaminates the reference, so the
run is inconclusive. The exact contaminated temporary WAV was removed.
User requests preparation while playing; user apps and formats remain untouched.
Publication awaits a quiet final reference and exact packaged fresh-install test.
