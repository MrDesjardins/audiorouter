# 2026-09-15 - live built-in processing chain validation

## 2026-09-15 - native lifecycle requalification after M03 harness fix

The authorized `m02-control-native-live.ps1 -AllowLiveAudio` acceptance
rediscovered the exact existing VB-Cable endpoints and completed the bounded
500 ms control-owned lifecycle. It captured 24,000 frames, processed 187
quanta, and rendered 23,936 frames. Start/stop/detach cleanup passed, with no
persistent endpoint or machine-audio state change. This refreshes existing
VB-Cable user-mode evidence; it does not qualify the prototype driver or
physical latency.

The authorized ignored control test
`guarded_live_native_endpoint_session_lifecycle_uses_one_control_plane` was
run at the current head with the exact existing VB-Cable capture and render
endpoint IDs. The prepared native graph included the built-in EQ, gate,
compressor, pitch-shift, and limiter chain before the output. The 500 ms
session completed with 24,000 captured frames, 187 processed graph quanta,
and 23,936 rendered frames; the test passed.

The test stopped the session and detached the worker on completion. No default
device, volume, mute, privacy, driver, signing, startup, or persistent audio
configuration changed. This qualifies built-in processor graph execution in
the existing user-mode VB-Cable route; production-driver callback and
subjective/transfer-function audio-quality gates remain open.

# 2026-09-15 - timing-qualified Rust adapter route

The guarded `m02-rust-adapter-route-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500` acceptance passed against the exact existing
VB-Cable endpoints. It captured 24,000 frames, processed 187 graph blocks,
and routed 23,936 frames. Processing telemetry reported 4,482,600 ns total,
168,600 ns maximum, 187 histogram samples, and zero deadline misses or
deadline lateness; the 128-frame deadline was 2,666,667 ns. Endpoint media
identity/state remained unchanged and all temporary clients were stopped.

# 2026-09-15 - differing-rate route requalification

The earlier documented 96 kHz capture identity was no longer active; an
exact-ID attempt failed closed with `0x80070490` (`no active capture endpoint`)
before opening a stream. A fresh read-only inventory found active 96 kHz mono
capture `{0.0.1.00000000}.{9f27c735-b82c-47a3-a962-56ea1e3774f3}` and active
48 kHz render `{0.0.0.00000000}.{1869e2ef-82c1-4602-a35a-be804a32112a}`.

The guarded route then passed for 500 ms using those exact IDs: 48,000 capture
frames, 23,936 scheduler/routed frames, 187 graph blocks, a 1,333,334 ns
128-frame deadline, 2,421,200 ns total processing time, and zero deadline
misses/lateness. Temporary clients were stopped and endpoint/media state was
unchanged. This is shared-mode cross-rate evidence, not independent-clock,
physical-latency, or managed-driver evidence.

# 2026-09-15 - repeated VB-Cable bridge and control-route qualification

The guarded `m02-rust-adapter-bridge-live.ps1` acceptance passed two 500 ms
cycles against the exact existing VB-Cable pair. Each cycle negotiated 48 kHz
stereo and reported 24,480 captured frames, 191 processed quanta, 24,448
rendered frames, zero dropped frames, zero scheduler xruns, and zero deadline
misses. Temporary recording and stream resources were removed and the media
snapshot was unchanged before and after both cycles.

The guarded `m02-control-route-live.ps1` acceptance also passed for 500 ms:
generation 1, 49 packets, 23,520 captured frames, 183 processed quanta, 23,424
rendered frames, one successful start/stop/reset, and one rejected post-stop
pump. It stopped and detached the worker, and verified unchanged media-device
identity/state. These are user-mode VB-Cable qualifications; they do not claim
production driver installation, PortCls ownership, or physical latency.

# 2026-09-15 - two-way endpoint resampling boundary

`WasapiSchedulerBridge::new_for_endpoints_at_graph_rate` now supports an
explicit internal graph rate independent of the selected endpoint rates. The
native endpoint control path uses the engine's 48 kHz internal rate; capture
packets are converted into that graph rate and processed graph blocks are
converted back to the render endpoint rate. Both streaming FIFOs retain phase,
repair non-finite input, stay allocation-free after construction, and reset at
stream boundaries. The regression
`scheduler_bridge_converts_graph_output_to_a_different_render_rate` passed,
and strict Clippy passed for Windows audio and control. This is portable
adapter evidence; live cross-rate hardware and production driver/PortCls
qualification remain open.

## 2026-09-15 - live VB-Cable validation after two-way conversion

The authorized `tests/acceptance/m02-rust-adapter-bridge-live.ps1
-AllowLiveAudio -DurationMilliseconds 500 -Cycles 1` run passed after the
two-way implementation was pushed. Against the exact existing VB-Cable pair
at 48 kHz stereo, it processed 24,480 captured frames, 191 graph quanta/tap
calls, and 24,448 rendered frames with zero non-finite tap samples, dropped
render frames, scheduler XRuns, or deadline misses. The 78-test Windows-audio
suite also passed. Temporary streams and recording were removed, and media
identity/state remained unchanged. This does not qualify the production
AudioRouter driver, PortCls transport, or physical latency.

# Native adapter route requalification (2026-09-14)

## 2026-09-14 - verified application-capture lifecycle

The guarded ignored test
`guarded_live_native_application_worker_lifecycle` qualified the native
process-loopback path against the exact observed `voicemeeterpro.exe` identity
(PID, creation-time value, and verified executable path) and the active CABLE
Input render endpoint. The bounded test completed two start/pump/stop cycles
on the same worker, followed by session-owned shutdown and worker detachment.
Temporary process environment values were cleared after the run. No endpoint
default, volume, mute, driver, signing, startup, or persistent audio
configuration changed. This is same-process restart evidence; process
replacement/rebind endurance plus production-driver transport remain open.

The reproducible wrapper
`tests/acceptance/m02-control-application-live.ps1` now requires the process
identity explicitly, discovers only one exact active CABLE Input render
endpoint by default, snapshots media devices, restores process environment
values, and verifies unchanged media state in both normal and cleanup paths.
It was exercised with the same Voicemeeter identity and passed.

## 2026-09-14 - lifecycle-order regression and rerun

The control-owned probe initially exposed an unsafe teardown-order mismatch:
it attempted to stop the native endpoint worker while its session was still
running, and the newly enforced control boundary rejected the request. The
probe was corrected to stop the recorder, stop the session (the authoritative
native-worker shutdown boundary), capture lifecycle telemetry, and then detach
the stopped worker.

The authorized rerun through
`tests/acceptance/m02-control-route-live.ps1 -AllowLiveAudio` used the exact
active CABLE Output capture and CABLE Input render IDs. It passed with 50
packets, 24,000 captured frames, 187 processed quanta, 23,936 rendered frames,
and a 95,788-byte recording. Start/stop/reset each succeeded once, and the
deliberate stale-generation pump was rejected. Before/after media snapshots
matched, and process environment plus temporary recording state were restored.
No endpoint default, volume, mute, privacy, driver, signing, startup, or
persistent audio configuration changed.

## 2026-09-13 - current-head control-owned VB-Cable lifecycle

The authorized ignored control test
`guarded_live_native_endpoint_session_lifecycle_uses_one_control_plane` was
run through `tests/acceptance/m02-control-native-live.ps1 -AllowLiveAudio`
against the exact active VB-Cable capture/render pair. The 500 ms route
captured 24,000 frames, processed 187 graph quanta, and rendered 23,936
frames; the same control plane completed start, bounded pumping, and stop.

The wrapper restored its process environment and temporary worker state. No
endpoint default, volume, mute, privacy, driver, signing, startup, or
persistent machine-audio setting changed. This is current-head user-mode
route evidence, not managed-driver, calibrated physical-latency, or drift-soak
evidence.

## 2026-09-13 - five-cycle adapter bridge endurance

The authorized `m02-rust-adapter-bridge-live.ps1 -AllowLiveAudio
-DurationMilliseconds 2000 -Cycles 5` run used the exact existing VB-Cable
capture/render pair. All five cycles independently completed at 48 kHz
stereo with 201 packets, 96,480 captured frames, 753 processed quanta and
tap calls, and 96,384 rendered frames. Each cycle reported zero non-finite tap
samples, dropped render frames, scheduler XRuns, and deadline misses; each
recording contained 25,072 bytes.

The harness stopped and removed every temporary stream and recording and the
before/after media-device state was unchanged. This is stronger bounded
user-mode bridge lifecycle evidence, but it does not establish calibrated
physical latency, clock-drift soak, managed-driver ownership, or production
signing.

## 2026-09-13 - built-in processing chain with pitch

The authorized ignored control test
`guarded_live_native_endpoint_session_lifecycle_uses_one_control_plane` was
run against the exact existing VB-Cable capture/render pair after extending
its backend-owned graph with the built-in `Pitch` node at +2 semitones. The
live chain is now EQ, Gate, Compressor, Pitch, and Limiter between the native
capture and render nodes. The bounded 500 ms run captured 24,000 frames,
processed 187 graph quanta, and rendered 23,936 frames; start, pumping, and
stop all passed.

The harness restored process environment and temporary worker state, and the
before/after media identity/state remained unchanged. This proves native
attachment of the in-house pitch processor in the user-mode route. It does
not establish production-driver ownership, calibrated physical latency, or
long-duration pitch artifact quality; those gates remain open under DSP-06.

## 2026-09-13 - current-tip two-second adapter route

The authorized `m02-rust-adapter-route-live.ps1 -AllowLiveAudio
-DurationMilliseconds 2000` run used explicit VB-Cable endpoint IDs and
completed at 48 kHz with the 128-frame graph quantum. It captured 96,480
frames, processed 753 graph blocks, scheduled and routed 96,384 frames, and
observed zero deadline misses or lateness. Total processing time was
16,902,500 ns with a 108,400 ns maximum; the timing histogram accounted for
all 753 samples.

The harness confirmed the media snapshot remained unchanged and removed its
temporary probe artifacts. No endpoint default, volume, mute, privacy, driver,
signing, startup, or persistent machine-audio setting changed. This is
bounded user-mode endurance evidence, not a physical-latency or long-duration
hardware soak qualification.

## 2026-09-13 - current-tip three-cycle bridge lifecycle

The authorized `m02-rust-adapter-bridge-live.ps1 -AllowLiveAudio
-DurationMilliseconds 300 -Cycles 3` run used explicit exact IDs for the
existing VB-Cable capture/render pair. All three cycles independently reported
48 kHz stereo, 14,880 captured frames, 116 processed quanta, 116 tap calls,
14,848 rendered frames, and 25,072 recording-file bytes. Every cycle reported
zero non-finite tap samples, dropped render frames, scheduler XRuns, and
deadline misses.

The harness compared media-device identity/state after each cycle and removed
all temporary streams and recordings. No default endpoint, volume, mute,
privacy, driver, signing, startup, or persistent machine-audio setting
changed. This strengthens user-mode bridge lifecycle evidence; physical
latency/drift soak, managed-driver ownership, and production release gates
remain open.

## 2026-09-13 - current-tip Rust adapter route

The authorized `m02-rust-adapter-route-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500` run used the exact existing VB-Cable pair and
completed with 48,000 Hz capture/render, a 128-frame graph quantum, and a
2,666,667 ns graph deadline. It captured 24,000 frames, processed 187 graph
blocks, scheduled and routed 23,936 frames, recorded 4,246,900 ns total
processing time with a 57,500 ns maximum, and observed zero deadline misses or
deadline lateness. The 32-bucket timing histograms accounted for all samples.

The harness compared media-device identity/state before and after, removed its
temporary probe binaries, and confirmed no default endpoint, volume, mute,
privacy, driver, signing, startup, or persistent machine-audio setting
changed. This is user-mode adapter timing evidence, not physical latency,
long-run drift/soak, managed-driver, or production-signing evidence.

## 2026-09-13 - current-tip guarded application-loopback lifecycle

The explicitly authorized ignored control test
`guarded_live_native_application_worker_lifecycle_uses_one_control_plane`
completed against the existing `voicemeeterpro.exe` process (PID 34568), using
its exact executable path and creation-time identity plus the active
`CABLE Input (VB-Audio Virtual Cable)` render endpoint. The test validated the
selected process identity before opening the stopped worker, started the
control-owned process-loopback route, pumped for the bounded 500 ms interval,
then stopped and detached it successfully.

The process was not terminated or reconfigured. All temporary environment
variables were restored after the run, and no endpoint default, volume, mute,
privacy, driver, startup, or persistent machine-audio setting changed. This
is native application-capture evidence; process restart recovery, physical
latency/drift, production driver ownership, signing, and attended UI gates
remain open.

## 2026-09-13 - current-tip guarded VB-Cable lifecycle

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-control-native-live.ps1 -AllowLiveAudio`.

The explicitly authorized run selected the active `CABLE Output (VB-Audio
Virtual Cable)` capture and `CABLE Input (VB-Audio Virtual Cable)` render
endpoints, then completed the same-process 500 ms control-owned lifecycle.
It reported 24,000 captured frames, 187 processed quanta, and 23,936 rendered
frames. The test restored all process environment variables and temporary
worker state. No default endpoint, volume, mute, privacy, driver, startup, or
persistent machine-audio setting was changed.

This is current Windows user-mode VB-Cable route evidence and does not close
the managed AudioRouter driver, PortCls ownership, signing, installer,
physical-latency, or attended UI gates.

## 2026-09-13 - physical render ownership diagnostic and VB-Cable fallback

Read-only endpoint inventory identified the exact active pair used for this
qualification: `CABLE Output (VB-Audio Virtual Cable)` capture and
`Speakers (Focusrite USB Audio)` render, both 48 kHz/stereo. The authorized
bounded command
`tests/acceptance/m02-control-native-live.ps1 -AllowLiveAudio` failed during
render activation with `AUDCLNT_E_DEVICE_IN_USE` (`0x8889000A`). The backend
reported `deviceInUse` with retry guidance, preserving the ownership conflict
as a distinct diagnostic rather than translating it to `E_INVALIDARG`.

The exact existing VB-Cable capture/render loopback was immediately
requalified afterward and passed: 24,000 captured frames, 187 processed
quanta, and 23,936 rendered frames. The harness restored its process
environment and temporary worker state. No Windows default, volume, mute,
driver, or persistent audio configuration changed. The physical-output
qualification remains open until the competing render stream is released or
another exact active physical endpoint is deliberately selected.

## 2026-09-13 - repeat VB-Cable lifecycle run

The authorized `m02-control-native-live.ps1 -AllowLiveAudio` run completed
against the exact active VB-Cable pair: 24,480 captured frames, 191 processed
quanta, and 24,448 rendered frames. The control-owned graph performed the
bounded start/stop lifecycle and restored temporary worker/environment state.
No Windows default, volume, mute, driver, or persistent audio configuration
was changed. This is native user-mode route evidence; it does not qualify the
managed AudioRouter driver or physical latency.

## 2026-09-13 - VB-Cable-first selection and guarded route

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-control-native-live.ps1 -AllowLiveAudio`.

The harness read the active endpoint inventory, selected the exact
`CABLE Output (VB-Audio Virtual Cable)` capture and `CABLE Input (VB-Audio
Virtual Cable)` render IDs, and ran the control-owned native lifecycle for
500 ms. The graph delivered 24,000 captured frames, 187 processed quanta, and
23,936 rendered frames. The built-in EQ, gate, compressor, and limiter chain
was active in the graph. The harness restored its process environment and
temporary worker state; no endpoint defaults, volume, mute, driver, or other
persistent machine-audio configuration changed.

This is guarded user-mode existing-endpoint evidence. It does not qualify the
managed driver, production signing, physical latency, or unattended visual
WebView gate.

## 2026-09-13 - guarded lifecycle with the built-in voice chain

The ignored Windows control test was extended to the full one-channel
processor chain `Live EQ` → `Live Gate` → `Live Compressor` → `Live Limiter`.
The exact active CABLE Output capture and CABLE Input render endpoints were
prepared stopped, started through the control plane, pumped for 500 ms, and
stopped cleanly. The wrapper reported 23,520 captured frames, 183 processed
quanta, and 23,424 rendered frames. Its environment and temporary worker state
were restored/removed; no persistent audio configuration changed.

The chain uses bounded parameters from the authoritative processor catalog:
EQ 1 kHz peaking/Q1/-6 dB, Gate -45 dB/60 dB/4:1, Compressor -18 dB/3:1,
and Limiter -1 dBFS/5 ms lookahead. This proves native control-owned graph
delivery for the in-house voice processors, not acoustic response measurement,
production callback timing, managed-driver ownership, signing, or physical
latency qualification.

## 2026-09-14 - guarded Rust process-loopback include/exclude requalification

Command:
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m00-rust-process-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500`

Both process-loopback modes passed through the Rust adapter and bounded graph
resampler. Include mode captured 22,050 source frames at 44.1 kHz and emitted
23,936 engine frames at 48 kHz across 187 quantum blocks; exclude mode
captured 21,609 source frames and emitted 23,296 engine frames across 182
blocks. Both runs reported generation 1 and zero scheduler xruns, input/output
overruns, or input/output underruns. Streams stopped/reset and the media
snapshot remained unchanged. This is user-mode process-loopback and
resampling evidence, not arbitrary protected-app capture, physical latency,
or production-driver evidence.

## 2026-09-13 - guarded lifecycle with built-in EQ and Gate

The ignored Windows control test was extended to use a disposable one-channel
`Live EQ` (`band0Enabled=true`, peaking at 1,000 Hz, Q 1, gain -6 dB) before
the existing `Live Gate`. The exact active CABLE Output capture and CABLE
Input render endpoints were prepared stopped, the graph was started and
pumped for 500 ms, and then stopped through the same `ControlPlane`. The
wrapper reported 24,000 captured frames, 187 processed quanta, and 23,936
rendered frames. Environment variables and the temporary worker state were
restored/removed; no persistent audio configuration changed.

This proves that both the built-in EQ and Gate stages can be compiled into the
native control-owned graph delivery path. It does not prove calibrated acoustic
frequency-response measurement, managed-driver ownership, PortCls integration,
signing, or physical latency.

## 2026-09-13 - guarded control-owned native lifecycle

The ignored Windows control test
`guarded_live_native_endpoint_session_lifecycle_uses_one_control_plane` was
run with explicit opt-in and the existing pair:

- capture: `CABLE Output (VB-Audio Virtual Cable)`
- render: `CABLE Input (VB-Audio Virtual Cable)`

The test enumerated and exact-matched both active IDs, prepared stopped WASAPI
clients, started the session through the same `ControlPlane`, pumped the graph
for 500 ms, and then stopped the session. The checked-in wrapper reported
24,000 captured frames, 187 processed graph quanta, 23,936 rendered frames,
one successful native start, and one successful native stop. It used no durable
database and restored the temporary opt-in environment variables after the
run. No defaults, volume, mute, privacy, or other persistent audio
configuration was changed.

This is control-owned native worker and processed graph-delivery evidence. It
does not prove managed virtual-driver ownership, PortCls integration, signing,
or physical acoustic latency.

## 2026-09-13 - guarded lifecycle with built-in Gate

The same ignored live test now uses a validated one-channel Gate node between
the exact capture and render endpoints, with threshold `-45 dB`, range `60 dB`,
hysteresis `3 dB`, ratio `4:1`, attack `5 ms`, hold `50 ms`, and release
`150 ms`. The existing CABLE pair was opened only after exact active-endpoint
matching. Native start, 500 ms bounded pumping, and stop passed with 24,000
captured frames, 187 processed quanta, and 23,936 rendered frames.

The test graph is disposable and in-memory; the wrapper restores its opt-in
environment. This proves that a processor-bearing graph reaches native runtime
activation and delivery. It does not claim acoustic transformation measurement,
managed-driver ownership, PortCls integration, signing, or physical latency.

## Guarded same-process control lifecycle harness

The control crate now contains an ignored Windows test,
`guarded_live_native_endpoint_session_lifecycle_uses_one_control_plane`.
It requires `AUDIOROUTER_ALLOW_LIVE_AUDIO=1`,
`AUDIOROUTER_CAPTURE_ENDPOINT_ID`, and `AUDIOROUTER_RENDER_ENDPOINT_ID`, then
enumerates and exact-matches both active endpoint descriptors before opening
stopped clients. It starts and stops the native session through the same
`ControlPlane` instance and checks native runtime/lifecycle telemetry. The
test is never run by ordinary workspace acceptance; it is a deliberate live
qualification command:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m02-control-native-live.ps1 -AllowLiveAudio
```

The harness owns no durable database and does not select defaults or alter
volume/mute/privacy state. Production PortCls ownership and managed-driver
activation remain separate gates.

## Current-tip guarded Rust adapter route

Command:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m02-rust-adapter-route-live.ps1 -AllowLiveAudio -DurationMilliseconds 500
```

The run used the existing explicitly selected VB-Audio pair: `CABLE Output`
capture to `CABLE Input` render. It captured 24,000 frames, processed 187
graph blocks, scheduled 23,936 frames, and routed 23,488 frames. Processing
time was 4,046,800 ns total with a 55,400 ns maximum; all 187 deadline samples
had zero deadline misses and zero lateness. The wrapper verified unchanged
media-device identity/state and removed its temporary executable/object.

This is shared-mode existing-endpoint adapter evidence only. It does not prove
managed virtual-driver ownership, production callback timing, physical
acoustic latency, signing, or installer behavior.

## Bridge identity encoding contract

The shared `AudioBridgeHello` validator now bounds both its portable UTF-8
identity and the UTF-16 byte representation required by the fixed native
request buffer. This prevents a 65-character ASCII identity (or an equivalent
UTF-16 expansion) from passing portable validation and failing only at the
Windows encoder boundary. Protocol and focused Windows-audio tests passed; no
driver was installed or loaded and no audio endpoint or persistent machine
configuration was changed.

The shared validator also rejects embedded NULs, matching the kernel's fixed
buffer identity validation and preventing ambiguous native bus identifiers.

The process-loopback activation payload is owned by a Windows COM-task-memory
RAII guard after asynchronous activation starts. Normal completion and
immediate activation failure release the blob; only the bounded timeout path
intentionally retains the async lifetime to prevent a use-after-free.

## Explicit CABLE adapter route

The bounded post-reboot `m02-rust-adapter-route-live.ps1` acceptance passed
against the explicitly selected existing CABLE Input/Output endpoints. The
48 kHz route captured 24,000 frames, processed 187 graph blocks, scheduled
and routed 23,936 frames, and reported zero deadline misses. Processing-time
and deadline-lateness histograms were complete and internally consistent.
The runner removed its temporary native probe outputs and verified unchanged
media identity/state; it did not change defaults, volume, mute, privacy,
drivers, signing, startup, or other persistent machine-audio configuration.

## Post-reboot system-selected adapter smoke

The authorized bounded adapter smoke check passed for 500 ms on the existing
system-selected capture/render endpoints at 48 kHz. It captured 24,480 frames,
processed 191 graph blocks, rendered 25,152 zero-valued frames, and reported
zero xruns, input/output overruns, and deadline misses. The probe stopped and
reset both streams and verified unchanged media-device identity/state. Its
`route=false` result is intentional because the default render endpoint is not
the explicitly selected CABLE route.

This is shared-mode user-space lifecycle evidence only; it does not qualify
physical acoustic latency, a loaded managed driver, or routed playback through
the default endpoint.

## Post-reboot CABLE bridge cycle

One authorized 500 ms cycle through freshly enumerated CABLE endpoints passed
with 24,480 captured frames, 191 processed quanta, 191 tap calls, and 24,448
rendered frames. Tap samples were finite; dropped render frames, scheduler
xruns, and deadline misses were all zero. The probe stopped and removed its
temporary streams/recording and verified unchanged media-device identity/state.

This strengthens digital user-space bridge evidence only. It does not qualify
physical acoustic latency, a loaded managed driver, or PortCls callback
ownership.

## Session shutdown ownership correction

The control-plane session stop path now treats an attached native endpoint
worker as part of the session's owned runtime resources. After recorder
finalization succeeds, it stops the worker before stopping the runtime. A stop
failure is surfaced with the existing structured `AudioError` mapping, but the
runtime is still transitioned to stopped so a client cannot continue pumping a
session that the control plane reports as stopped. Recorder failures still
precede this boundary and leave the session available for deliberate recovery.

Verification: control tests (125) and doctests passed, strict control Clippy,
workspace formatting, and `git diff --check` passed. This is a control-plane
lifecycle regression fix; it does not qualify a loaded virtual driver or alter
machine audio settings.

## Session deletion ownership boundary

The control plane now treats an attached native endpoint worker as transient
session ownership during deletion. A running worker blocks deletion explicitly;
a stopped worker is cleared only after the durable and in-memory session
removals succeed. Persistence failure consequently preserves the worker and
its binding for deliberate retry, while successful deletion cannot leave stale
native taps attached to a missing session.

Verification: control tests (125), strict control Clippy, formatting,
`git diff --check`, and documentation validation passed. No endpoint or driver
was opened or modified.

The administrator-authorized Rust adapter route was rerun against the named
CABLE endpoints for 500 ms. Negotiation selected 48,000 Hz on both sides with
the 128-frame graph quantum and a 2,666,667 ns graph deadline. It captured
24,480 frames, processed 191 graph blocks, and routed 24,448 frames. Processing
time totaled 4,083,600 ns with a 57,300 ns maximum and a 65,536 ns p99.9 upper
bound; deadline misses and deadline lateness were both zero.

The control-owned route was also rerun with the exact endpoint identities: 50
packets, 24,000 captured frames, 187 processed quanta, 23,936 rendered frames,
4,140 recording bytes, one successful start/stop/reset, and one rejected stale
pump. The worker stopped and detached cleanly. Before/after media snapshots
matched; no default, volume, mute, privacy, driver, signing, startup, or
persistent machine-audio setting changed. This is real shared-mode adapter and
control-lifecycle evidence, not production virtual-driver or physical-latency
qualification.

The system-selected Rust adapter smoke path was also rerun for 500 ms. It
passed at 48 kHz with 51 capture packets, 24,480 captured frames, 191 graph
blocks, 24,448 scheduler frames, zero xruns, input/output overruns, or deadline
misses, and a 65,536 ns p99.9 processing-time upper bound. Its telemetry
correctly reported `route=false`: the existing default render endpoint differs
from the explicitly selected CABLE route. This is default-endpoint lifecycle
and processing evidence, not a routed-playback claim; streams stopped/reset
and media state was unchanged.

The maximum five-cycle Rust adapter bridge soak then passed at 48 kHz with
one-second cycles. Every cycle captured 48,480 frames, processed 378 quanta
and tap calls, and rendered 48,384 frames. All five reported zero non-finite
tap samples, dropped frames, scheduler xruns, and deadline misses. Temporary
streams and recordings were stopped/removed after each cycle and media state
remained unchanged. This is repeated user-space lifecycle evidence, not the
required long-duration or loaded-driver soak.

At the two-second control-route bound, the exact endpoint pair produced 200
packets, 96,000 captured frames, 750 processed quanta, and 96,000 rendered
frames. Start/stop/reset each succeeded once and one stale-generation pump was
rejected. The worker detached cleanly and media state remained unchanged.

Two additional 500 ms Rust adapter bridge cycles passed against the exact CABLE
pair. Each cycle captured 24,480 frames, processed 191 quanta/tap calls, and
rendered 24,448 frames. Both reported zero non-finite tap samples, dropped
frames, scheduler xruns, or deadline misses. The Rust process-loopback include
and exclude modes also passed at 44.1 kHz source to 48 kHz engine conversion:
10,584 and 11,025 source frames became 11,392 and 11,904 engine frames, with
zero rejected packets and xruns. All streams were stopped/reset and media
state remained unchanged. These checks strengthen M02 user-space evidence but
do not substitute for managed-driver callback or physical-latency validation.

# M02 audio adapter groundwork

## Negotiated same-rate graph activation

On 2026-09-15, the native endpoint bridge began retaining the validated
capture/render sample rate and the control plane began preparing the native
graph at that rate. This fixes the prior unconditional 48 kHz activation for
validated 44.1 kHz endpoint pairs. Cross-rate capture/render conversion is
still gated behind a separate bounded-resampling implementation; endpoint
format mismatches continue to fail before activation. Focused control and
Windows-audio tests passed (165 control tests, 74 Windows-audio tests; two
control tests remain intentionally ignored because they require explicit live
audio authorization).

The complete guarded acceptance chain was requalified on 2026-09-15 with the
installed VS 18/WDK 10.0.28000 toolchain. The project driver compiled with
signability and catalog generation, and the read-only native inventory found
31 endpoints. The chain passed all portable, UI, VST3, VST2, headless, release,
traceability, and documentation stages. It deliberately did not install or
load a driver or change machine audio configuration. Cross-rate resampling and
physical-latency qualification remain open.

On 2026-09-15, `InterleavedStreamingResampler` was wired into the native
endpoint bridge's capture path. The bridge now accepts differing capture and
render rates after strict channel/format validation, selects the render rate
as graph rate, and converts captured float32 packets before scheduling. Its
76-test suite covers phase retention, bounded admission, finite-sample repair,
atomic underflow silence, reset, and 44.1↔48 kHz endpoint-rate selection.
The render endpoint remains at the graph rate in this slice, and live
cross-rate device evidence is still required.

## Guarded VB-Cable route

On 2026-09-15, the authorized `m02-control-route-live.ps1` harness ran for
500 ms against the exact existing VB-Cable bindings (`CABLE Output` capture
and `CABLE Input` render). Both negotiated 48 kHz stereo. The control-owned
worker reported generation 1, 50 packets, 24,000 captured frames, 187
processed quanta, 23,936 rendered frames, a 95,788-byte temporary WAV, one
start/stop/reset success, and one deliberate stale-generation rejection.
The before/after media identity and state snapshots matched. The worker was
stopped and detached; defaults, volume, mute, privacy, drivers, signing,
startup settings, and endpoint registration were unchanged. This validates
the existing-endpoint control route only and does not qualify the production
driver, capture-rate conversion on hardware, or physical latency.

The same exact 48 kHz stereo pair was independently requalified on 2026-09-15
through `m02-control-native-live.ps1`: 24,000 captured frames, 187 processed
quanta, and 23,936 rendered frames completed in 500 ms. The lower
`m02-rust-adapter-bridge-live.ps1` harness completed one cycle with 24,960
captured/rendered frames, 195 tap calls, zero non-finite samples, zero XRuns,
zero deadline misses, and zero dropped frames. Both harnesses verified
unchanged media identity/state and cleanup. These runs provide 48 kHz
existing-endpoint evidence; they do not prove 44.1 kHz hardware negotiation,
cross-rate live conversion, production driver activation, or physical latency.

## 2026-09-08 - Differing-rate route requalification

The guarded `m02-rust-adapter-route-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500` run passed on the explicitly selected VB-Audio
pair: 96 kHz capture to 48 kHz render, 47,040 capture frames, 183 graph
blocks, 23,424 scheduler frames, and 23,424 routed frames. The negotiated
128-frame graph deadline was 1,333,334 ns; processing p99.9 was 65,536 ns,
with zero deadline misses and zero deadline lateness. Media-state snapshots
and temporary cleanup passed. This remains shared-mode user-space evidence,
not managed-driver callback, independent-clock, or physical-latency evidence.

## 2026-09-08 - Portable liveness and recovery boundary audit

The Windows adapter's recovery boundary was re-audited against CAP-06,
CAP-11, and CAP-12. Exact endpoint recovery refreshes the notification-backed
inventory and reopens only the persisted endpoint ID, direction, and mix
format; missing, changed, or stale bindings fail closed before activation.
Transient device-invalidation and audio-service failures use a bounded retry
helper, while invalid-argument, access, and other non-transient failures are
not retried. Process restart resolution requires exactly one
case-insensitive executable match with a creation timestamp, and the binding
check rejects a stale PID/name/time tuple.

Portable regressions cover retry success, retry-attempt bounds, non-transient
rejection, endpoint identity validation, and restarted-process stale-binding
rejection. This confirms the safe recovery policy without substituting another
endpoint or changing defaults. Full sleep/resume, Windows Audio service
restart, reboot, user-session transition, and an actual PID-reuse occurrence
remain native lifecycle gates and are not claimed by this audit.

## 2026-09-08 - Rust adapter and route requalification

The guarded `m02-rust-adapter-live.ps1 -AllowLiveAudio -DurationMilliseconds
500` run passed against the existing endpoints: 24,480 capture frames became
191 generation-1 graph blocks and 24,448 scheduler frames; 25,536 silent
render frames were submitted. Processing-time p99.9 upper bound was 65,536 ns,
with zero xruns and deadline misses. The explicitly selected VB-Audio route
also passed for 500 ms with 24,000 capture frames, 187 graph blocks, and
23,936 routed frames; its processing-time p99.9 bound was 32,768 ns, with zero
deadline misses. Both wrappers verified stream stop/reset, temporary cleanup,
and unchanged media state. These are user-mode adapter and digital-route
results, not managed-driver callback timing or physical acoustic latency.

## 2026-09-08 - Differing-rate Rust route

The guarded route was requalified with explicit endpoint IDs after the native
format inventory identified a valid pair: 96 kHz mono capture into 48 kHz
stereo render. The Rust adapter consumed 48,000 capture frames, produced 187
fixed graph blocks and 23,936 routed frames, and reported a 16,384 ns
processing-time p99.9 upper bound with zero deadline misses. Endpoint/media
identity and cleanup checks passed. This closes the available differing-rate
resampler smoke on the current machine, but not clock-drift qualification,
physical acoustic latency, or managed-driver ownership.

## 2026-09-07 — Rust capture compatibility mode

The Windows adapter now exposes an explicit `SharedCapture::open_polling`
path. It uses the native-qualified shared-mode request (exact endpoint mix
format, `AUDCLNT_STREAMFLAGS_NOPERSIST`, and a bounded buffer duration) and
polls packet availability, while the event-driven request remains the primary
`open` behavior. This addresses the observed distinction between successful native
non-event initialization and the Rust event-callback `E_INVALIDARG` path
without silently weakening the event-driven contract. The adapter package's
15 tests and strict Clippy pass; no live stream was opened for this change.

The normal `SharedCapture::open` path now retries that polling request only
when event-callback initialization returns exactly `E_INVALIDARG`. Busy-device,
permission, and endpoint-loss errors are not retried or relabeled. The retry
uses a fresh COM client, so a failed initialization cannot leave a partially
configured client in use. This is a compatibility implementation, not live
Rust stream qualification.

## 2026-09-05 — Read-only endpoint adapter

Added `crates/windows-audio` as the first reusable Windows adapter boundary. It explicitly owns COM initialization/uninitialization, enumerates active capture and render endpoints, copies the COM-owned `WAVEFORMATEX` metadata before freeing it, and returns endpoint ID, direction, shared-mode periods, sample rate, channels, bits, and format tag. It also provides shared capture/render lifecycle wrappers with exact endpoint selection, bounded event-driven initialization, owned event handles, start/stop/reset, timeout waits, and packet/buffer operations that release device buffers immediately.

The control plane now uses this adapter for `devices.list`, returning active endpoint IDs, direction, state, format, and period metadata. The adapter also provides an identity-preserving metadata snapshot diff for added, removed, and changed endpoints; it is a polling helper and does not silently rebind a missing device. `status.get` reports device discovery as available while full audio remains unavailable because the realtime graph and routing are not implemented. `apps.list` returns bounded process identities and, on Windows, the read-only audio-session observations described below.

Endpoint topology notifications are now registered through an RAII `IMMNotificationClient` subscription. Every callback only sets an atomic dirty flag; the control plane must consume that flag and resnapshot, so callbacks never enumerate, allocate, lock, or rebind streams.

`SharedCapture::next_packet_into` now provides a bounded data-copy boundary into caller-owned storage, handles silent packets as zeroes, validates the requested bytes-per-frame and destination capacity, and releases every WASAPI packet before returning. This is an adapter primitive, not yet an end-to-end realtime graph.

`SharedRender::submit_bytes` provides the matching bounded output boundary for caller-owned interleaved bytes. It rejects partial frames, limits writes to available device capacity, and releases the WASAPI render buffer. The normal adapter tests remain non-invasive; no render stream was started.

The adapter now classifies retained Windows HRESULTs into stable `AudioFailureKind` values, including device contention versus invalid argument and exclusive-only behavior. Six adapter tests pass. This allows control diagnostics to distinguish the earlier `E_INVALIDARG` condition from `AUDCLNT_E_DEVICE_IN_USE` without changing the underlying error.

Application discovery now attempts `PROCESS_QUERY_LIMITED_INFORMATION` and records an optional process creation timestamp alongside PID and executable name. This is sufficient identity material for a PID-reuse check without exposing command lines or paths. The adapter's `bind_application` path enforces the identity tuple; controlled process-loopback tone attribution remains open.

`bind_application` now enforces that identity material before a future process-loopback activation: PID, executable name, and creation timestamp must all match, otherwise binding is rejected. The Windows identity test passes without opening an audio stream; the native loopback harness remains a separate data-path implementation.

The validator rejects missing creation-time metadata explicitly, preventing PID/name-only matches from being accepted. Seven adapter tests and strict adapter Clippy pass.

The adapter now also exposes a restart resolver for persisted executable
selectors. It refuses to rebind when no executable matches, when multiple
case-insensitive matches exist, or when the sole match lacks a creation
timestamp; only one verified candidate is returned. Portable regression
coverage exercises all four outcomes. This is identity-only evidence: no
audio stream is opened, and controlled process-tree attribution and native
restart/PID-reuse runtime evidence remain open.

PID-bound verification now compares executable names case-insensitively, while
still requiring the observed creation timestamp and PID. This keeps live
rebinding consistent with the restart resolver and avoids false identity
changes caused only by Windows name casing; no audio stream is opened.

Control serialization now exposes creation timestamps as decimal strings rather than JSON numbers, preserving the full `u64` identity across TypeScript clients. Control tests and contract typechecking pass; no audio operation is involved.

Buffer-capacity failures are now classified separately as `BufferConstraint`, while invalid frame sizes remain `InvalidArgument`. Seven adapter tests and strict adapter Clippy pass.

`EndpointMonitor` now owns an initial active-endpoint snapshot and refreshes it only after an `IMMNotificationClient` notification, returning identity-preserving diffs. Its read-only startup/poll test passes; stream rebinding and graph recovery remain separate work. Eight adapter tests and strict adapter Clippy pass.

Verification on the Windows 11 host:

```powershell
cargo test -p audiorouter-windows-audio
```

Passed 5 tests, including live active-endpoint enumeration, endpoint snapshot diffing, and unknown capture/render endpoint rejection tests. The wrapper's real capture start/packet/stop path is covered by the separately run native diagnostic; the ordinary workspace suite does not open the user's microphone. The adapter does not change defaults, volume, mute, driver state, or other persistent configuration.

This does not yet satisfy M02. Graph activation/routing, end-to-end realtime buffer transfer, latency measurement, dual-device drift validation, process-tree capture data, and failure recovery remain open. The native diagnostic separately provides the current capture and process-loopback activation/data evidence.

The process-tree capture clause above is superseded by the later native probe evidence: include and exclude modes have successful data-path reads. Remaining M02 gaps are controlled attribution, graph-to-device activation, end-to-end scheduling, latency, dual-device drift validation, failure recovery, and driver lifecycle.

The native probe now provides an opt-in controlled-attribution harness: a bounded child process renders a deterministic tone while the parent captures only that process tree and verifies child cleanup. The implementation builds with the installed Windows toolchain, but runtime execution is intentionally pending because it emits an audible test tone; this does not yet constitute attribution evidence.

The Windows application inventory now supplements process identities with a read-only `IAudioSessionManager2` snapshot across active render and capture endpoints. `applications.list` reports active/inactive session counts, separate render/capture session counts, display names, and capture sessions observed for each PID. It does not open or start an audio stream, alter endpoint state, or claim that an unobserved protected/background capture is impossible. Process-loopback attribution and capture capability beyond the observed snapshot remain open.

Session enumeration is fail-soft at the individual session and endpoint
manager boundaries: an aggregate session, inaccessible endpoint, or transient
session disappearance is skipped while the rest of the process inventory is
retained. The adapter regression still verifies nonzero process identities and
consistent session-count bounds.

The process inventory also excludes Windows' PID-0 system pseudo-process so
every returned application satisfies the API's positive-PID schema constraint.

Results are sorted case-insensitively by executable and then PID, making
read-only UI/CLI refreshes deterministic even though Toolhelp enumeration
order is not guaranteed.
The policy has both a synthetic regression and a live Windows snapshot check.

## Current adapter status (2026-09-07)

The earlier introductory read-only wording in this report is superseded by
the later production-adapter smoke evidence below. `SharedCapture` and
`SharedRender` now provide explicit bounded stream ownership, event/polling
delivery, caller-owned packet copies, and lifecycle cleanup; metadata and
application/session discovery remain read-only. The guarded adapter-to-engine
and virtual-cable runs are the current runtime evidence. Native graph-to-device
scheduling, dual-device drift, failure recovery, and measured physical latency
remain open.

## 2026-09-11 — Endpoint state enumeration boundary

The adapter now provides `enumerate_endpoint_states`, which requests the
Windows all-state endpoint collection and returns only opaque ID, direction,
and mapped state. Active, disabled, unplugged, and not-present values are
recognized; unknown numeric values are preserved instead of being treated as
active. The path does not activate clients or request formats, so unavailable
devices cannot make discovery fail merely because they cannot be opened.

The focused Windows-audio suite passed 56 tests with strict Clippy,
formatting, and diff checks. The control response remains active-only pending
the format-optional all-state schema and notification-driven refresh slice.

The all-state implementation was rechecked after correcting the Windows
projection of `DEVICE_STATEMASK_ALL` to the generated `DEVICE_STATE` newtype.
The focused Windows-audio suite passed 56 tests, including active/disabled,
not-present/unplugged, and unknown-value mapping. This remains a metadata
adapter result only; control-plane all-state response and notification refresh
are not yet claimed.

## 2026-09-11 — Explicit default-role observations

The Windows adapter now exposes `enumerate_default_endpoint_bindings`, which
queries `IMMDeviceEnumerator::GetDefaultAudioEndpoint` for console,
multimedia, and communications roles in both render and capture directions.
The operation only reads opaque endpoint IDs; it does not activate a client,
start a stream, change defaults, or reserve an endpoint. Missing role
assignments are omitted rather than substituted.

`devices.list` includes the resulting `defaultRoles` array, with matching
schema and TypeScript contract support. This is an observation for an explicit
follow-default binding, not permission to replace a pinned endpoint. Focused
Windows-audio (55) and control (106) tests plus strict Clippy passed; no
endpoint stream or persistent machine audio configuration changed.

## 2026-09-11 — Bounded endpoint friendly names

The active endpoint adapter now reads `PKEY_Device_FriendlyName` through a
read-only property store and returns a bounded display string alongside the
opaque ID and direction. Property conversion failures use `Unknown audio
endpoint`; names are never used for binding or replacement. `devices.list`,
its schema, the shared TypeScript contract, and the UI inventory expose the
presentation value. Focused Windows-audio (55), control (106), contracts/UI
typechecks, UI (121), and strict Clippy passed without opening a stream or
changing persistent audio configuration. Disabled/unplugged enumeration and
notification-driven refresh remain open.

The rebind failure path was hardened so endpoint objects are discarded after
stop attempts even when one stop reports an error; repeated stop also resets
staged bridge audio. The focused 54-test Windows-audio suite, strict Clippy,
formatting, and diff checks passed. No endpoint was opened by the checks.

The bounded packet-drain entry point was also hardened to reject stopped
workers even when the requested packet budget is zero, keeping all pump APIs
consistent with the fail-closed lifecycle contract. The focused 54-test suite,
strict Clippy, formatting, and diff checks passed.

The complete administrator-authorized safe acceptance chain was requalified at
`830152dc` after this worker implementation. M00-M08 project, portable DSP,
UI, plugin, headless, release, traceability, and documentation checks passed;
13 run-owned temporary children were removed. This does not upgrade the
worker to managed-driver callback or production endpoint evidence.

## Bounded endpoint-worker packet drain (2026-09-10)

`WasapiEndpointWorker` now drains a caller-selected packet budget per event
wake, capped at 64 packets. It stops on an empty packet read, aggregates
telemetry with saturating arithmetic, and keeps event waits, endpoint
activation, stop, and rebind outside the pump. The tap/deadline path uses the
same bound and never retries an invalidated endpoint. The focused
Windows-audio suite passed 54 tests, strict package Clippy passed, and
formatting/diff checks passed. No endpoint was opened by these checks; managed
driver ownership and production callback timing remain open.

## 2026-09-06 — Prepared session activation boundary

`RuntimeProcessor::activate_session` now compiles a complete session candidate
before publishing its immutable runtime graph. If validation or supported-topology
preparation fails, no publication occurs and the previously active generation
continues processing. A regression uses a valid fixture followed by an invalid
matrix and verifies that generation and output remain on the original graph.
The engine suite passes 40 tests with strict Clippy. This is a portable
publication/rollback boundary; WASAPI scheduling, device-resource activation,
and live routing remain open.

## 2026-09-06 — Cross-milestone regression

The full locked Rust workspace passed, including the 40-test engine suite and
all adapter, control, storage, recording, plugin-worker, transport, CLI/MCP,
and doc-test targets. Contracts/UI typechecks, 23 UI tests, and the production
Vite build also passed. This confirms regression health for the portable
boundaries; live graph-to-device scheduling, physical latency, and driver
lifecycle remain open.

The Windows adapter now includes the numeric HRESULT in displayed audio errors
while retaining stable failure classification. A regression confirms
`0x80070057` is reported as `InvalidArgument` and remains visible in the
diagnostic text. This improves investigation of the unresolved Rust
`IAudioClient::Initialize` discrepancy; it does not claim that discrepancy is
fixed and does not open or start an audio stream.

Shared capture and render initialization now wrap failures with the operation
name (`IAudioClient::Initialize(capture)` or `(render)`) while preserving the
underlying HRESULT and `AudioFailureKind`. The Windows-audio regression covers
the formatted invalid-argument path; this remains diagnostic hardening only,
not a fix or evidence of a started stream.

## Production Rust adapter smoke (2026-09-07)

The standalone M00 probe now has an explicit `adapter-smoke` mode that uses the
production `SharedCapture` and `SharedRender` types. On the current host it
opened the first active capture and render endpoints, started both streams,
read capture packets into a preallocated caller-owned buffer, submitted only
silent render buffers, and stopped/reset both clients. A 500 ms run collected
51 capture packets/24,480 frames/195,840 bytes and submitted 25,536 silent
render frames. This qualifies the Rust adapter's bounded stream data path and
cleanup, but not graph-to-device scheduling or audible end-to-end routing.
The final media snapshot remained ten present devices, all `OK`.

The live run is now reproducible through
`tests/acceptance/m02-rust-adapter-live.ps1 -AllowLiveAudio`. The wrapper
requires an explicit bounded duration, snapshots media-device identity/state
before and after, requires positive capture and silent-render counts, and is
not invoked by ordinary CI or non-live acceptance.

The guarded wrapper was executed with `-AllowLiveAudio -DurationMilliseconds
200` and passed: 21 capture packets/10,080 frames/80,640 bytes and 11,136
silent render frames were observed, and the before/after media-device snapshot
was identical.

The smoke path was then tightened to exercise `SharedRender::submit_bytes`
with a preallocated all-zero caller buffer. The guarded 200 ms acceptance
passed again with 21 capture packets/10,080 frames/80,640 bytes and 10,656
zero-valued render frames; the media-device identity/state snapshot remained
identical. This validates the caller-owned render-copy path without generating
an audible signal.

## Adapter-to-engine block smoke (2026-09-07)

The opt-in smoke was extended to feed the captured 32-bit samples into the
portable `RealtimeScheduler` in fixed 128-frame blocks. A guarded 200 ms live
run processed 8,064 capture frames through the scheduler while submitting
11,136 zero-valued render frames through `SharedRender::submit_bytes`; capture
and render clients stopped/reset successfully and the media-device snapshot
remained identical. This qualifies bounded adapter-to-engine ownership and
block processing, not complete graph activation, audible routing, or physical
latency.

The same run reported one start attempt/success, one stop attempt/success,
one successful stream reset, and zero rejected pumps. These are bounded
control-thread lifecycle counters; they are not a production callback timing
claim. A deliberately stale generation pump was rejected before packet drain;
the control rejection counter reported exactly one rejection, and the valid
route remained at generation 1.

## Rust process-loopback requalification (2026-09-08)

The guarded Rust process-loopback acceptance passed both include and exclude
modes at 250 ms. Include mode converted 10,584 source frames to 11,392 engine
frames across 89 scheduler blocks; exclude mode converted 11,025 source frames
to 11,904 engine frames across 93 blocks. Both runs used the documented 44.1
kHz source and 48 kHz engine, with zero rejected packets, XRuns, input/output
overruns, or underruns. The wrapper verified unchanged media state after
teardown. This is asynchronous process-loopback adapter evidence only; it does
not establish managed-driver routing, physical latency, or PID reuse.

The packet adaptation was then tightened to carry partial WASAPI packets across
boundaries in a fixed staging buffer. The latest guarded 300 ms run consumed 30
packets (14,400 capture frames), processed 14,336 complete 128-frame scheduler
frames, retained the final 64-frame remainder at bounded shutdown, and
submitted 15,936 zero-valued render frames. Counts vary with scheduling during
the bounded window; this validates packet-to-quantum carry rather than assuming
packet/quantum alignment.

The smoke now publishes a prepared generation-1 graph containing the portable
0.5x gain stage. It verifies that every returned scheduler block belongs to
that generation and contains only finite samples. A guarded 300 ms run passed
with 32 packets/15,360 capture frames, 120 graph blocks/15,360 processed
frames, no pending remainder, and 16,032 zero-valued render frames. The media
device identity/state snapshot remained identical; this qualifies graph
activation and finite-output handling at the adapter boundary, not audible
routing or physical latency.

## Rust adapter-to-render route smoke (2026-09-07)

The standalone probe now has an explicit `adapter-route` mode that selects a
capture and render endpoint by their discovered opaque endpoint IDs, requires
matching 32-bit channel/rate metadata, feeds capture packets through the
generation-1 0.5x gain graph, and submits the processed caller-owned frames to
the render client. The guarded VB-Audio cable run passed for 500 ms with
24,480 capture frames, 24,448 scheduler frames, and 24,448 routed render
frames. The media-device identity/state snapshot was unchanged.

The reproducible wrapper is
`tests/acceptance/m02-rust-adapter-route-live.ps1 -AllowLiveAudio`; it resolves
friendly names through the native inventory probe, removes its temporary probe
and generated object, and never changes defaults, volume, mute, privacy,
drivers, signing, or startup configuration. This qualifies the digital
adapter-to-render data path, not physical acoustic latency, arbitrary format
conversion, or production graph lifecycle.

The route carry queue is bounded to 64 fixed 128-frame blocks (approximately
170 ms for stereo float32); if render capacity cannot catch up, the route
fails closed rather than dropping a processed block or allocating without a
limit.

The route now supports explicit mono-to-stereo duplication and stereo-to-mono
averaging for compatible 32-bit endpoints, plus fixed-quantum linear
sample-rate conversion into preallocated graph blocks. Three pure mapping
regressions and strict probe Clippy pass; the existing stereo VB-Audio route
also passed again with 24,480 captured, 24,448 scheduled, and 23,904 routed
frames. Cross-block clock-drift correction and hardware synchronization remain
separate open gates.

The route uses the existing preallocated linear resampler when the selected
capture and render rates differ, preserving the fixed 128-frame graph quantum.
It does not yet maintain a cross-block fractional phase or feed FIFO occupancy
back into `DriftController`; those remain native scheduler work.

The endpoint selector has two additional pure regressions: an opaque ID must
match the requested direction, and an omitted ID may select only within the
requested direction. The standalone probe tests, strict Clippy, and locked
compile check pass.

## Fail-closed endpoint binding resolution (2026-09-07)

The Windows-audio adapter now exposes `resolve_endpoint_binding` for a fresh
read-only endpoint snapshot. It returns an endpoint only when the persisted
opaque ID and expected direction both match; a missing ID or direction change
is returned explicitly, with no friendly-name or enumeration-order fallback.
The regression passes in the 18-test Windows-audio suite with strict Clippy
and doc-tests. This is a recovery decision boundary only; native stream
re-opening, hardware removal/reconnect, and graph rebind remain open.

## Route output ownership cleanup (2026-09-07)

The adapter-route probe now separates render serialization/submission errors
from scheduler-output recycling, so every received processed block is returned
to the bounded output pool even when routing fails. The compile check and a
guarded 500 ms VB-Audio route acceptance passed again with 24,480 captured,
24,448 scheduled, and 24,448 routed frames; the media snapshot and temporary
artifact cleanup remained unchanged.

## Current-head one-second adapter-route requalification (2026-09-07)

The authorized Rust adapter-route wrapper was rerun for 1,000 ms using the
explicitly selected VB-Audio render and capture endpoint IDs. Capture delivered
48,960 frames; the fixed-quantum scheduler processed 48,896 frames; and
48,384 frames were routed through the generation-1 graph. The endpoint/media
snapshot remained unchanged and temporary native outputs were removed. This
extends the adapter data-path evidence under a longer bounded window; it does
not qualify cross-device clock synchronization, physical latency, driver
integration, or production graph lifecycle.

## Format-aware endpoint binding resolution (2026-09-07)

The fail-closed endpoint resolver now also compares the persisted mix format
(sample rate, channels, bits per sample, and format tag). A changed format
returns `FormatChanged` with the expected and observed metadata instead of
silently accepting a stale graph binding. The regression passes in the
19-test Windows-audio suite with strict Clippy and doc-tests. This remains a
read-only decision boundary; deliberate native renegotiation and stream
recovery are still open.

The resolver is now exposed on `EndpointMonitor` as a snapshot-only method,
making the fail-closed decision available immediately after notification-driven
refresh without coupling recovery code to enumeration or stream activation.

Persisted endpoint format identity also includes the extensible channel mask and
subformat GUID. A changed mask or subformat now returns `FormatChanged`, just
like a changed rate, channel count, bit depth, or format tag; recovery must
explicitly renegotiate rather than reopening a materially different stream.

Recovery can call `EndpointMonitor::refresh_changes` to force a fresh endpoint
enumeration even when no dirty notification is visible. The method clears the
coalesced notification flag, returns the same deterministic diff, and remains
metadata-only; stream reopen, renegotiation, and replacement selection remain
deliberate caller operations.

The stream clients now also expose `open_refreshed_bound`, which refreshes the
monitor snapshot immediately before applying the exact ID/direction/format
guard. The adapter route uses this entry point for both streams, reducing the
window in which a pending coalesced notification could leave validation stale;
an endpoint change after validation is still reported by WASAPI.

The stream clients now expose `open_bound` entry points that enforce this
decision immediately before activation. Missing, direction-changed, or format-
changed bindings return a structured `AudioError::EndpointBinding` without
activating WASAPI; a topology race after validation remains surfaced by the
underlying WASAPI HRESULT. The new error payload is boxed so ordinary audio
errors remain small, and native stream recovery still requires the caller to
refresh and deliberately retry or renegotiate.

The adapter-route probe now creates an `EndpointMonitor` from the selected
inventory and opens both streams through `SharedCapture::open_bound` and
`SharedRender::open_bound`. Packet strides come from the validated endpoint
metadata helper, so the live route cannot bypass the binding or frame-shape
checks. The monitor validation remains a control-plane precondition; topology
races after validation are still surfaced by WASAPI errors.

The one-second authorized route also asserted scheduler telemetry before
stopping: processed quanta matched graph blocks, the active generation remained
current, and input/output overruns plus XRuns were zero. This is bounded route
health evidence, not proof of long-run native callback timing.

The adapter route now requires `EndpointInfo::is_ieee_float32` for both selected
streams. This accepts legacy IEEE-float tags and extensible float subformats,
but rejects 32-bit integer PCM before any packet bytes are decoded as floats,
closing a format-confusion data-path risk.

`AudioError::binding_resolution` exposes the structured fail-closed decision to
recovery callers without requiring string parsing. This preserves explicit
handling for endpoint disappearance, direction changes, and deliberate format
renegotiation while retaining the stable HRESULT/error-kind classification.

Each stream client now exposes `replace_with_refreshed_bound`. This explicit
recovery operation stops/resets and drops the old client before forcing a fresh
metadata snapshot and reopening the exact binding. If refresh or validation
fails, no replacement stream is opened; callers must deliberately renegotiate
or choose another endpoint. This closes the portable recovery orchestration
boundary while native device-invalidation fault injection remains open.

The adapter now exposes a ratio-controlled resampler boundary and feeds the
render-side bounded pending-frame occupancy into `DriftController` whenever
capture and render rates differ. This makes the correction loop explicit and
bounded at the route boundary, while cross-block fractional phase, sufficient
FIFO depth for arbitrary rate ratios, and hardware clock qualification remain
native scheduler work.

The route now uses the preallocated `StreamingResampler` FIFO for those
different-rate streams. Fractional phase and source samples survive packet and
quantum boundaries, while a short production result remains an explicit
bounded underflow rather than repeating the last sample. Capacity exhaustion
is surfaced as an error; arbitrary-rate stress, native callback timing, and
hardware clock qualification remain open.

The streaming boundary now treats an incomplete destination quantum
transactionally: it silences the caller-owned output and leaves source FIFO
ownership plus fractional phase untouched. This prevents an adapter route from
dropping the consumed prefix of a partial block; the focused engine and probe
checks pass, while arbitrary-rate stress and native timing remain open.

The authorized one-second adapter-route acceptance was rerun after this fix.
The selected VB-Audio pair delivered 48,960 capture frames, the scheduler
processed 48,896 frames, and 47,968 frames were routed through the generation-
1 graph. Endpoint/media identity and state remained unchanged and temporary
outputs were removed. This requalifies the existing-rate live path; differing
hardware-rate stress and native callback timing remain open.

The route now waits on the production render client's event before draining
the bounded carry queue. This connects the native event-driven render boundary
to route submission and avoids treating an unavailable render period as a
successful write. Compile/tests and strict Clippy pass; the follow-up live run
was blocked before execution by the host's Application Control policy (OS
error 4551), so no new runtime route claim is made.

The differing-rate route now targets 512 queued source frames, the midpoint of
its fixed 1,024-frame resampler FIFO. This avoids the previous unreachable
8,192-frame target, which forced the integral/proportional controller to remain
at a correction bound. A centered-target regression and focused engine/probe
checks pass; hardware clock behavior remains unqualified.

Adapter-route output now reports the resampler queue depth and applied drift
correction, and rejects a run if queue occupancy exceeds 1,024 frames or the
correction exceeds the configured ±100 ppm bound. This makes the bounded
feedback state observable to acceptance tooling; probe tests and strict Clippy
pass, while live differing-rate execution remains blocked by host policy.

## Differing-rate candidate and host launch boundary (2026-09-07)

The read-only native format inventory found an available 48 kHz CABLE capture
and 96 kHz SteelSeries Sonar Aux render pair, both using float32 mix formats.
The guarded differing-rate route was then attempted with those exact friendly
names. Its temporary native build completed, but Windows Application Control
blocked launching the freshly generated executable before endpoint inventory;
therefore no stream opened and no differing-rate runtime result is claimed.
The route wrapper now uses a per-run temporary object path and cleans it with
the executable, so this host-policy failure cannot strand `main.obj` in the
repository.

## Repeatable differing-rate route (2026-09-07)

The route acceptance now accepts both exact endpoint IDs as an alternative to
friendly-name resolution. This preserves fail-closed opaque binding while
allowing a run to bypass the host's blocked temporary native inventory launch.
Using the 96 kHz mono Digital Audio Interface capture ID and 48 kHz stereo
CABLE render ID, the authorized 500 ms wrapper run passed with 48,000 capture
frames, 23,936 scheduler frames, and 23,936 routed frames. The output reported
zero XRuns and input/output overruns, 133 queued resampler frames, and
correction within the configured ±100 ppm bound. Media identity/state and
temporary-artifact cleanup checks passed; no persistent audio configuration
changed.

## 2026-09-07 — Current-head Rust adapter route

The authorized guarded route was replayed after the event-gated render and
bounded streaming-resampler changes using the existing VB-Audio endpoints.
The route reported 24,000 capture frames, 23,936 scheduler frames, and 23,456
routed frames through the generation-1 graph. Endpoint/media identity and state
were unchanged, and the temporary executable was removed. This is real
existing-rate route evidence; it does not qualify differing-rate hardware
clock behavior, native invalidation recovery, or physical acoustic latency.

The adapter now also exposes bounded `open_refreshed_bound_with_retry` helpers
for capture and render. They run only on the recovery/control thread, force a
fresh snapshot before every exact-ID/direction/format validation, retry only
`DeviceInUse`, `DeviceInvalidated`, or `ServiceUnavailable`, and cap attempts
at five with a one-second maximum delay. They never select a substitute or
sleep on the realtime path. The focused Windows-audio suite passes 22 tests
with formatting and strict Clippy; native fault-injection evidence remains a
separate host/device gate.

The recovery boundary now also provides
`replace_with_refreshed_bound_with_retry` for both stream directions. It stops
and releases the failed client before invoking the bounded exact-binding retry
policy, so no old client overlaps a replacement and no substitute endpoint can
be selected. The focused Windows-audio suite remains green at 22 tests with
doc-tests, formatting, and strict Clippy; actual device-invalidation fault
injection is still unqualified.

The shared bounded retry policy is now directly regression-tested: transient
device invalidation retries until success, invalid argument fails immediately
without retry, and an always-transient failure stops at five attempts. This
tests policy behavior without pretending to be native fault injection; the
focused Windows-audio suite passes 25 tests with doc-tests, formatting, and
strict Clippy.

The full locked workspace was then requalified at this checkpoint: 393
unit/integration tests, all doc-tests, and strict all-target/all-feature
Clippy with `-D warnings` passed. This validates dependent control and
transport compilation against the recovery API; it does not replace native
device-invalidation fault injection or production supervisor evidence.

## Current-head guarded live requalification (2026-09-07)

The authorized guarded wrappers passed at the current head. The native bounded
capture/silent-render lifecycle discovered 13 capture and 21 render endpoints
and completed its 200 ms run with one occupied render endpoint classified. The
VB-Audio digital loopback passed with nonzero capture payload, and the explicit
Rust adapter route passed with 48,000 capture frames and 23,936 scheduler and
routed frames. Temporary binaries were removed and media identity/state checks
were unchanged. This is digital/adapter evidence only; calibrated physical
latency, device-invalidation fault injection, managed driver, signing, and
production shell gates remain open.

## Current-tip adapter route requalification (2026-09-07)

The guarded Rust adapter-route acceptance was rerun against the existing
friendly-name VB-Audio endpoints. The bounded run captured 24,000 frames and
processed 23,936 scheduler frames into 23,936 routed frames. It passed exact
endpoint binding, cleanup, and before/after media-state checks. A separate
read-only format inventory passed for all 34 active endpoints. This is
adapter-path evidence only; native scheduler ownership, hardware clock drift,
and physical acoustic latency remain open. No defaults, volume, mute, privacy,
driver, signing, startup, or other machine audio configuration changed.
## Rust adapter route requalification (2026-09-07)

The guarded `m02-rust-adapter-route-live.ps1 -AllowLiveAudio -DurationMilliseconds
500` wrapper passed over the existing VB-Audio Virtual Cable endpoints. The
route processed 24,480 capture frames and 24,448 scheduler/routed frames, then
completed lifecycle cleanup and verified unchanged media-device identity/state.
Defaults, volume, mute, privacy, drivers, signing, startup, and other machine
audio configuration were not changed. This is digital adapter/engine evidence;
physical acoustic latency, production-driver integration, and hardware clock
qualification remain open.

An authorized differing-rate attempt used the read-only inventory's 96 kHz
render endpoint and 48 kHz capture endpoint. The render endpoint is 8-channel,
so the adapter correctly failed closed with `InvalidFrameSize` before stream
activation; exact cleanup and media-state checks passed. No valid differing-rate
mono/stereo pair is currently available, leaving hardware clock/resampler
qualification open.

The legacy array response of `devices.list` now advertises the 500-endpoint
bound and returns a pagination-required error if more endpoints are present,
instead of silently truncating a read-only native inventory. This check does
not open an audio stream or change endpoint state.
## Production Rust adapter live run (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-rust-adapter-live.ps1 -AllowLiveAudio
-DurationMilliseconds 250`.

The selected existing endpoints completed a bounded production Rust adapter run
with 26 capture packets, 12,480 capture frames, 97 generation-1 graph blocks,
13,536 render frames, zero scheduler XRuns, and clean stream stop/reset. The
run used zero-valued caller-owned render buffers, so `routed_frames=0` and
`route=false` are expected; this proves adapter lifecycle and processing only.
It does not close endpoint-specific initialization failures, routed signal, or
calibrated physical-latency gates. Media-device state and persistent audio
configuration were unchanged.
## Production Rust adapter route run (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-rust-adapter-route-live.ps1 -AllowLiveAudio
-DurationMilliseconds 100`.

The selected existing VB-Audio virtual cable completed the bounded route run
with 4,800 capture frames, 4,736 scheduler frames, and 4,736 routed frames.
The adapter stopped and reset its streams, and the acceptance harness reported
defaults, volume, mute, privacy, drivers, signing, and startup configuration
unchanged. This proves selected-endpoint route activation only; calibrated
physical latency and production-driver gates remain open.

## Rust process-loopback adapter (2026-09-07)

The Windows adapter now provides bounded asynchronous process-loopback capture
with explicit include and exclude target-tree selection. Focused Windows tests
(27), strict workspace Clippy, and the guarded live acceptance passed. Both
modes completed 250 ms runs with 24 packets and 10,584 frames, and the harness
verified stream stop/reset plus unchanged media-device identity/state. The
implementation uses caller-owned packet storage and the supported 44.1 kHz
stereo PCM/event-callback initialization shape; physical latency and full
cross-process isolation thresholds remain separate gates.

## PCM16 planar boundary (2026-09-07)

`AudioBlock` now exposes allocation-free bridges for one complete interleaved
PCM16 block. Decode maps signed 16-bit samples to planar float32 using
`-32768 -> -1.0` and `32767 -> 32767/32768`; encode silences non-finite values,
clamps finite values to `[-1, 1]`, and emits the bounded PCM16 range. Both
methods reject any source or destination whose shape is not exactly the
preallocated block shape.

The focused engine suite passed 66 tests, including round-trip boundary,
non-finite, clamp, and shape regressions. Strict engine Clippy, formatting, and
diff checks passed. This is a portable adapter-boundary result; packet
accumulation/splitting, native realtime scheduling, physical latency, and
production-driver integration remain open.

## Live PCM16 packet-boundary adapter (2026-09-07)

The guarded `m00-rust-process-live.ps1 -AllowLiveAudio` acceptance fed actual
Rust process-loopback packets through the fixed PCM16 quantum adapter. Include
and exclude modes each delivered 10,584 PCM16 frames in 24 packets and emitted
82 complete 128-frame engine quanta. Both modes stopped/reset successfully and
the media-device identity/state snapshot was unchanged. No persistent audio
configuration changed.

This is packet-boundary and format-conversion evidence only. The adapter is not
yet connected to the realtime scheduler rings; physical latency,
generation-aware live routing, and production-driver integration remain open.

## Live scheduler-ring integration (2026-09-07)

The guarded `m00-rust-process-live.ps1 -AllowLiveAudio` acceptance now feeds
the fixed PCM16 quantum adapter into a bounded `RealtimeScheduler`. Include and
exclude modes each converted 10,584 frames across 24 packets into 82 exact
128-frame blocks, processed every block as generation 1, and recycled the
matching output. Both modes stopped/reset successfully and the media-device
identity/state snapshot was unchanged. No persistent audio configuration
changed.

This proves process-loopback packet conversion and scheduler-ring ownership for
the tested path. Deterministic overflow/underrun stress, native output routing,
physical latency, and production-driver integration remain open.

## Scheduler pressure regression (2026-09-07)

The engine regression suite now exercises a one-block `RealtimeScheduler` under
both input and output pressure. A second prepared input is rejected immediately
and increments `input_overruns`; when the only output block is deliberately
held, the next processing step fails closed, recycles the input, and increments
the scheduler xrun counter without waiting.

The focused engine suite passed 68 tests, with strict engine Clippy, formatting,
and diff checks passing. This proves portable bounded ownership behavior;
native scheduler timing, physical latency, and production-driver integration
remain open.

## Fixed PCM16 quantum adapter (2026-09-07)

`Pcm16QuantumAdapter` stages split interleaved PCM16 packets in a fixed
one-quantum buffer. It accepts complete frames only, returns the number of
frames consumed, stops accepting input once 128 frames are ready, and exposes
`pop_into` to decode the complete block into caller-owned planar engine
storage. It rejects malformed channel shapes and destination shapes without
discarding pending audio. No allocation occurs after construction and the
buffer cannot grow beyond one mono/stereo quantum.

The focused engine suite passed 67 tests, including split-packet accumulation,
backpressure, exact 128-frame output, and shape regressions. Strict engine
Clippy, formatting, and diff checks passed. Native packet-boundary integration,
realtime scheduling, physical latency, and production-driver integration remain
open.

## Cross-crate verification after scheduler changes (2026-09-07)

Command: `cargo test --workspace --locked` at pushed head `2ec7701`.

All workspace unit tests and doc-tests passed, including engine (68),
Windows-audio (27), control (86), CLI (25), plugin-host and worker, recording,
storage, transport, domain, DSP, protocol, and their documented test targets.
This is portable and Windows host test evidence; it does not close native
realtime timing, physical latency, driver, signing, installer, or hardware
acceptance gates.

## Generation replacement and live recheck (2026-09-07)

`RealtimeScheduler::publish` now recycles queued output blocks at the control
publication boundary before exposing the replacement generation. A concurrent
old callback may still submit an old block, so generation-filtered receive
remains the final protection; the new regression verifies queued ownership is
restored and the next block is processed under the replacement generation.

The engine suite passed 69 tests, strict Clippy, formatting, and diff checks
passed, and the guarded process-loopback acceptance re-ran successfully in both
include and exclude modes (10,584 frames, 82 generation-1 quanta each). No
persistent audio configuration changed. Native callback timing and physical
latency remain unqualified.

## Event-driven process-loopback wakeup (2026-09-07)

`ProcessLoopbackCapture::wait_for_data` now waits on the WASAPI event handle
with a caller-bounded timeout and structured failure result. The guarded live
probe uses this wait before draining packets rather than a polling sleep.
Include and exclude modes each delivered 11,025 frames across 25 packets and
processed 86 generation-1 quanta; stop/reset and media-device snapshot checks
passed, with no persistent audio configuration changed.

This qualifies event delivery and bounded wakeup behavior on the tested host,
not callback deadline, clock-drift, physical-latency, or production-driver
compliance.

## Process-loopback delivery telemetry (2026-09-07)

The Windows adapter now exposes a nonblocking `ProcessLoopbackTelemetry`
snapshot backed by saturating atomics. It records wait calls, wait timeouts,
successful packets and frames, minimum/maximum packet-frame counts, and silent
packets. Counters update only after successful buffer release or bounded event
wait outcomes; no logging or allocation is introduced into packet delivery.

The guarded live acceptance measured, for both include and exclude modes, 25
packets and 11,025 frames, with 441 minimum and maximum packet frames. Include
recorded 39 waits/14 timeouts; exclude recorded 40 waits/15 timeouts; both had
zero silent packets. Media state and persistent audio configuration were
unchanged. These are host observations, not callback deadline or physical
latency evidence.

## Routed adapter 2-second qualification (2026-09-08)

The guarded routed adapter wrapper passed for 2,000 ms on the explicitly
selected VB-Audio pair: 96,480 captured frames, 96,384 scheduler frames,
95,904 routed frames, and 753 graph blocks. Processing p99.9 was 32,768 ns
(maximum 28,900 ns); deadline misses and lateness were zero, and all 32
histogram buckets were present. Streams stopped/reset cleanly and defaults,
volume, mute, privacy, driver, signing, and startup snapshots were unchanged.
This is shared-mode user-space adapter evidence, not managed-driver callback or
physical-latency qualification.

## Endpoint worker lifecycle composition (2026-09-10)

`audiorouter-windows-audio` now exposes `WasapiEndpointWorker`, an explicit
owner for the selected `SharedCapture`, `SharedRender`, and preallocated
`WasapiSchedulerBridge`. It activates capture before render, stops capture if
render activation fails, attempts both endpoint stops during shutdown, clears
queued/partial graph audio, and refuses pump calls while stopped. The worker
also forwards the allocation-free tap/deadline path. The focused Windows-audio
suite passed 53 tests, strict package Clippy passed, and formatting/diff checks
passed. This is lifecycle composition evidence only: no endpoint was opened by
the focused checks, and automatic endpoint replacement, managed-driver
ownership, production callback timing, signing, and physical latency remain
open.

## Shared adapter 2-second qualification (2026-09-08)

The guarded shared capture/render adapter wrapper passed for 2,000 ms on the
selected endpoints: 96,480 capture frames, 96,384 scheduler frames, 97,056
silent render frames, and 753 graph blocks. Processing p99.9 was 65,536 ns
(maximum 46,800 ns); scheduler xruns, input/output overruns, deadline misses,
and deadline lateness were zero. Streams stopped/reset cleanly and the media
identity/state snapshot was unchanged. This is shared-mode user-space evidence,
not managed-driver callback or physical-latency qualification.

## Windows-audio identity and adapter regression suite (2026-09-08)

The complete `audiorouter-windows-audio` test target passed 29 tests plus
doc-tests. Coverage includes exact endpoint binding and format validation,
snapshot diffs, bounded transient recovery, process-loopback input bounds and
telemetry, read-only application/session inventory, creation-time identity
matching, and the restart helper regression. This is read-only adapter and
portable identity evidence; managed-driver callback timing, physical latency,
and production endpoint ownership remain open.

## Bounded process-loopback packet-period policy (2026-09-07)

The process-loopback adapter now enforces the explicit
`MAX_PROCESS_LOOPBACK_PACKET_FRAMES = 4,096` bound after `GetBuffer` and before
copying. A zero or oversized packet is released and rejected, so an endpoint
period cannot expand staging beyond the fixed quantum contract. The policy
regression covers the lower and upper accepted bounds and both rejection cases.

Windows-audio tests (28), strict Clippy, formatting, and tool compilation
passed. Guarded live include/exclude runs observed 25/24 packets, 11,025/10,584
frames, and 441-frame minimum/maximum packets; both completed 86/82 scheduler
quanta with unchanged media state and no persistent audio configuration
changes. These observations do not establish realtime deadline or physical
latency compliance.

## Rejected-period telemetry (2026-09-07)

`ProcessLoopbackTelemetry` now includes a saturating `rejected_packets` counter.
The process-loopback adapter increments it only after releasing a packet that
violates the zero/4,096-frame admission policy, so rejected ownership cannot
remain held while diagnostics are recorded.

The guarded live acceptance reported zero rejected packets in both modes:
include delivered 10,584 frames in 24 packets and 82 quanta; exclude delivered
11,025 frames in 25 packets and 86 quanta. Both observed 441-frame packets and
unchanged media state/configuration. This remains host telemetry, not deadline
or physical-latency evidence.

## Telemetry invariants (2026-09-07)

Windows-audio regression coverage verifies that `ProcessLoopbackTelemetry` has
an all-zero initial snapshot and that its saturating atomic increment helper
does not wrap a counter at `u64::MAX`. The Windows-audio suite passed 29 tests,
with strict Clippy, formatting, and diff checks passing. No audio stream or
machine configuration was accessed by these tests.

## Synthetic packet-period boundary matrix (2026-09-07)

The engine regression suite now covers a 127-frame packet followed by one
frame, an exact 128-frame packet, and a maximum 4,096-frame packet drained in
128-frame steps. The adapter emits complete quanta while retaining at most one
quantum of staging; no source growth or blocking is possible. The engine suite
passed 70 tests with strict Clippy, formatting, and diff checks.

This is deterministic portable period-policy evidence only. No hardware,
native stream, or machine audio configuration was accessed or changed.

## Shared packet-admission policy (2026-09-07)

The engine-side `Pcm16QuantumAdapter::push_packet` now enforces the same
non-empty, 4,096-frame maximum period bound as the Windows process-loopback
adapter before copying into fixed staging. The existing chunk method remains
available only for already validated packet pieces. Regression coverage checks
empty, malformed, and over-bound inputs while preserving pending ownership.

Engine tests (69), Windows-audio tests (28), strict workspace Clippy, formatting,
and tool compilation passed. Guarded live include/exclude runs each observed
441-frame packets and emitted 82 generation-1 scheduler quanta; stop/reset and
media-state checks passed with no persistent audio configuration change.

## Process-loopback rate-domain bridge (2026-09-08)

The live process-loopback probe now makes the source/engine clock conversion
explicit: caller-owned 44,100 Hz PCM16 stereo packets are accumulated into
128-frame source blocks, passed through the bounded phase-preserving
`StreamingResampler` at `44,100/48,000`, and drained into zero-or-more fixed
128-frame 48 kHz scheduler quanta. The FIFO is allowed to emit an occasional
second engine quantum after enough source data arrives; this prevents a false
one-quantum-per-source-block assumption from filling the bounded queue.

Focused engine tests (70), probe compilation, and formatting passed. The
guarded 300 ms include/exclude acceptance passed: each mode captured 12,789
source frames and emitted 13,696 engine frames across 107 generation-1 quanta;
packets were 441 frames, rejected packets were zero, and the final resampler
queue was 89 frames. Media-device identity/state snapshots were unchanged.
This proves the adapter's explicit rate-domain bridge and lifecycle only; it
does not claim drift correction against an independent render clock or
physical/native production-driver latency.

## Differing-rate native route qualification (2026-09-08)

The guarded, endpoint-ID-selected route wrapper passed against the current
96 kHz mono capture endpoint and 48 kHz stereo render endpoint. The Rust
adapter processed 28,800 capture frames into 14,336 scheduler/routed frames,
with generation 1 active and zero scheduler xruns or overruns. The bounded
streaming resampler and drift controller were exercised; correction reached
the declared -100 ppm bound during this short run. The wrapper verified
media-device identity/state equality and the documented unchanged defaults,
volume, mute, privacy, driver/signing, and startup state.

This is endpoint-specific shared-mode adapter evidence only. The correction
bound and short duration do not establish long-term independent-clock lock,
physical latency, managed virtual-driver lifecycle, or Discord/OBS behavior.

## Maximum-duration differing-rate stability (2026-09-08)

The guarded endpoint-ID-selected route wrapper passed its maximum 2,000 ms
duration using the same 96 kHz mono capture and 48 kHz stereo render pair:
191,040 capture frames, 95,488 scheduler frames, and 95,488 routed frames.
The detailed run observed 192,000 capture frames and 96,000 scheduler quanta,
20 queued resampler frames, -100 ppm bounded correction, and zero scheduler
xruns/overruns. The wrapper confirmed stream cleanup and unchanged endpoint
identity/default/volume/mute/privacy/driver/signing/startup state.

This strengthens bounded stability evidence for the adapter's differing-rate
path, but remains short-duration shared-mode testing and does not establish
independent-clock lock, physical latency, or production-driver behavior.

## Scheduler telemetry drain accounting (2026-09-08)

The scheduler output-ring control-boundary drain now uses a non-counting raw
pop. Intentional draining to an empty queue no longer increments the consumer
underrun metric; actual empty consumer reads retain the underrun counter. A
regression test covers generation replacement and asserts zero false output
underruns.

Engine tests (70), probe compilation, formatting, and the guarded 300 ms
include/exclude live acceptance passed. Include emitted 107 and exclude 112
generation-1 quanta; both modes reported zero scheduler xruns, input/output
overruns, and input/output underruns. Media-device snapshots were unchanged.

## Workspace regression after scheduler accounting (2026-09-08)

The locked full workspace suite passed after the queue-drain correction:
control 86, domain 53, DSP 27, engine 70, plugin-host 39 plus 8 worker-process
tests, protocol 6, recording 30, storage 40, transport 17, and Windows audio
29, with all doc tests passing. This is portable/native API regression
coverage; it does not close managed-driver, signing, installer, physical
latency, or manual UI gates.

## Native scheduler adapter and scoped route qualification (2026-09-08)

The existing native `adapter_smoke` acceptance passed for 300 ms using the
selected shared capture/render endpoints: 32 capture packets, 15,360 capture
and scheduler frames, 120 generation-1 graph quanta, zero scheduler xruns or
overruns, and zero queued resampler frames. Streams stopped/reset and the
media-device identity/state snapshot was unchanged.

The explicitly selected existing VB-Audio virtual-cable route acceptance also
passed for 300 ms: 13,920 capture frames, 13,824 scheduler frames, and 13,824
routed frames. Defaults, volume, mute, privacy, drivers, signing, startup
configuration, and media-device identity were unchanged. This is bounded
shared-mode adapter/route evidence only; it does not prove managed AudioRouter
driver lifecycle, Discord/OBS compatibility, physical latency, or callback
deadline compliance.

## Histogram-content validation (2026-09-08)

Both guarded adapter acceptance wrappers now validate the raw processing-time
histogram structurally: exactly 32 entries, sequential labels from bucket 0
through 31, numeric nonnegative counts, and a count sum equal to the reported
sample total. The adapter and routed 300 ms checks passed; the routed run
reported 108 complete samples and unchanged media state. This remains
adapter/event-loop evidence, not native callback deadline compliance.

## Runtime processing-time instrumentation (2026-09-08)

`CallbackMetrics` now records saturating total and maximum monotonic processing
duration in nanoseconds at the `RuntimeProcessor` boundary. `SchedulerTelemetry`
exposes both values for off-thread diagnostics. The update uses atomics only:
it allocates no memory, takes no locks, logs nothing, and performs no I/O.
Processing without an active graph is timed as well, so a future native callback
can account for the complete runtime boundary rather than only successful graph
blocks.

The engine suite (70 tests), locked workspace tests/doc-tests, strict Clippy,
formatting, and diff checks passed. This is instrumentation readiness and
portable processing evidence; it is not native callback p99.9/deadline evidence
until a production-style native scheduler owns the endpoint callback.

## Bounded processing-time histogram (2026-09-08)

The callback metrics now retain a fixed 32-bucket logarithmic nanosecond
histogram alongside saturating total and maximum durations. Each processing
observation updates one atomic bucket; no per-callback allocation or sample
list is retained. This is sufficient for a later control-boundary percentile
calculation while preserving the realtime boundary constraints.

The engine suite (70 tests), workspace compilation, strict Clippy, formatting,
diff checks, and documentation validation passed. The histogram is portable
instrumentation readiness only; it does not establish native callback p99.9 or
deadline compliance until a production-style native scheduler owns the endpoint
callback.

## Adapter timing telemetry propagation (2026-09-08)

The Rust `adapter_smoke` probe now reports and validates scheduler processing
time total, maximum, and histogram sample count. Its validation requires the
histogram sample count to equal processed quanta, preventing a silently stale
or partial timing report. The guarded route acceptance requires the same timing
fields and bounded total/max relationship.

The probe check and guarded 300 ms live adapter/route checks passed. The
adapter run reported 116 histogram samples for 116 processed quanta,
1,090,800 ns total, 21,600 ns maximum, zero scheduler xruns/overruns, and
unchanged media state. The routed run passed with 14,400 capture frames,
14,336 scheduler frames, and 13,856 routed frames. This remains adapter/event
loop evidence and does not establish production native callback deadline
compliance.

## Inactive-runtime timing regression (2026-09-08)

The engine regression suite now covers the pre-activation silence path: it
verifies that an inactive `RuntimeProcessor` records exactly one nonzero timing
observation while leaving processed-quanta telemetry at zero and clearing the
block. Engine tests (71), strict Clippy, formatting, diff checks, and workspace
compilation passed. This remains portable instrumentation evidence and does not
establish native callback deadline compliance.

## Routed telemetry completeness regression (2026-09-08)

The guarded routed-adapter acceptance now parses `graph_blocks` and requires
the processing-time histogram sample count to match it exactly. This prevents
a route from passing with only a partial timing report. The 300 ms acceptance
passed with 13,920 capture frames, 13,824 scheduler frames, and 13,824 routed
frames; endpoint/media state remained unchanged.

## Raw timing-distribution propagation (2026-09-08)

The adapter probe now emits all 32 fixed processing-time histogram entries as
`bucket:count` pairs. Both guarded adapter acceptance wrappers require exactly
32 entries, and the routed wrapper preserves the complete distribution in its
final summary. The guarded 300 ms adapter run reported 116 samples, with 22 in
bucket 13 and 94 in bucket 14; the routed run reported 116 samples, with 9 in
bucket 13, 102 in bucket 14, and 5 in bucket 15. The routed aggregate was
1,134,000 ns total and 28,400 ns maximum. Both runs completed with unchanged
media state. This remains adapter/event-loop evidence, not native callback
deadline compliance.

## Portable scheduler deadline telemetry (2026-09-08)

`RealtimeScheduler::process_once_with_deadline` now provides an explicit
caller-owned deadline boundary. Completed processing that finishes after the
deadline records a saturating miss count and total/maximum lateness using
atomics only; the scheduler does not wait, allocate, log, or touch an
endpoint. A regression verifies a late deadline is recorded while the active
generation and processed output remain correct.

Engine tests (72), doc-tests, strict Clippy, formatting, and diff checks pass.
This closes portable callback-integration readiness only. Native endpoint-owned
callback period/deadline measurements, managed-driver lifecycle, signing,
physical latency, and manual application acceptance remain open.

## Shared-mode scheduler deadline qualification (2026-09-08)

The authorized guarded adapter and explicitly selected VB-Audio route now call
`RealtimeScheduler::process_once_with_deadline` for each processed 128-frame
engine quantum. The acceptance wrappers require deadline-miss and lateness
fields and validate their bounds alongside the existing processing histogram.

The 300 ms adapter run processed 120 quanta with zero xruns, zero deadline
misses, and zero deadline lateness. The routed run processed 112 quanta with
zero xruns, zero deadline misses, and zero deadline lateness. Both runs stopped
and reset their streams and verified unchanged media-device state. This is
shared-mode endpoint-adapter evidence only; it does not establish managed
virtual-driver ownership, production callback compliance, physical latency, or
long-term soak behavior.

## Deadline-lateness distribution (2026-09-08)

Scheduler telemetry now retains a fixed 32-bucket logarithmic histogram for
positive deadline lateness. The adapter output preserves all buckets, and both
guarded acceptance wrappers require sequential bucket labels, numeric counts,
and a histogram sum equal to the reported deadline misses.

The follow-up 300 ms adapter run processed 120 quanta with zero deadline
misses/lateness; the routed run processed 116 quanta with zero deadline
misses/lateness. Both stopped/reset their streams and verified unchanged media
state. This remains shared-mode adapter evidence, not managed-driver callback,
physical-latency, or long-term soak evidence.

## Strict deadline-boundary correction (2026-09-08)

Deadline accounting now treats completion exactly at the supplied deadline as
on time. Only strictly positive `Instant` lateness increments the miss count,
lateness totals/maximum, and positive-only histogram. Engine tests (72), strict
Clippy, formatting, diff checks, and documentation validation pass.

## Deterministic exact-deadline regression (2026-09-08)

A direct zero-duration lateness regression now proves that an exactly on-time
completion does not increment deadline misses or the positive lateness
histogram. Engine tests (73), strict Clippy, formatting, diff checks, and
documentation validation pass. Native callback and hardware timing gates are
unchanged.

## Histogram counter-overflow hardening (2026-09-08)

Percentile extraction now saturates the diagnostic histogram total before rank
calculation, preventing malformed or adversarial counter snapshots from
panicking in a diagnostics path. A regression covers a saturated zero bucket
plus an additional sample. Engine tests (75), strict Clippy, formatting, and
diff checks pass. This remains bounded portable telemetry and does not change
native callback or hardware timing evidence.

## Bounded percentile extraction (2026-09-08)

The engine now exposes conservative upper-bound extraction from fixed
logarithmic histograms. The 99.9th-percentile rank uses integer ceiling
arithmetic, rejects invalid percentile inputs, returns no value for an empty
histogram, and never reports below the bucket containing the selected sample.
The adapter emits processing and deadline-lateness p99.9 upper-bound fields;
both guarded live acceptance paths require those bounds to be at least the
reported maxima. Engine tests (74), strict Clippy, formatting, tool checking,
PowerShell parsing, live adapter/route acceptance, diff checks, and
documentation validation pass. The guarded 300 ms runs reported processing
p99.9 upper bounds of 32,768 ns and zero deadline-lateness p99.9 bounds, with
unchanged media-device state. Native callback and hardware timing gates remain
open.

The guarded route acceptance summary now includes both validated p99.9 upper
bounds instead of leaving them visible only in the underlying adapter line.
The 300 ms route requalification reported 14,400 captured frames, 13,856
routed frames, processing maximum 26,200 ns, processing p99.9 upper bound
32,768 ns, and zero deadline misses/lateness. The endpoint snapshot and
explicit configuration-safety checks passed; this remains shared-mode adapter
evidence rather than production callback evidence.

## Rust adapter live requalification (2026-09-08)

The guarded 300 ms production Rust adapter smoke completed with 32 capture
packets, 15,360 captured/scheduled frames, 15,456 silent render frames, 120
graph blocks, zero xruns, zero deadline misses, and a 32,768 ns processing
p99.9 upper bound. Streams stopped/reset cleanly and the media-device snapshot
was unchanged. This is shared-mode endpoint evidence; managed-driver callback,
physical-latency, signing, and installer gates remain open.

The guarded 300 ms route acceptance also passed on the explicitly selected
VB-Audio render/capture pair: 14,400 captured frames, 13,824 routed frames,
112 graph blocks, 32,768 ns processing p99.9 upper bound, and zero xruns or
deadline misses/lateness. Streams stopped/reset and defaults, volume, mute,
privacy, drivers, signing, and startup configuration were unchanged.
## Live shared-mode adapter requalification (`acf1454`, 2026-09-08)

The authorized bounded adapter smoke passed on the selected native endpoints:
24,480 capture frames produced 191 generation-1 graph blocks and 24,448
scheduler frames, with zero scheduler xruns, input/output overruns, or
deadline misses. Processing p99.9 was 65,536 ns. The wrapper used zero-valued
caller-owned render buffers, stopped/reset both streams, and verified an
unchanged media-device snapshot. This is shared-mode user-space adapter
evidence only; it does not satisfy managed-driver callback-deadline or
physical-latency gates.

## Routed adapter requalification (`a4b72ec`, 2026-09-08)

The authorized bounded route acceptance passed on the explicitly selected
existing VB-Audio endpoints: 24,000 capture frames became 23,936 routed
frames across 187 graph blocks. Processing p99.9 was 32,768 ns, with zero
xruns, deadline misses, or deadline lateness. Streams stopped/reset and
endpoint/default/volume/mute/privacy/driver/signing/startup snapshots were
unchanged. This remains shared-mode diagnostic evidence, not managed-driver
callback or physical-latency qualification.

## Rust process-loopback requalification (2026-09-08)

`tests/acceptance/m00-rust-process-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500` passed for both include and exclude modes. Include
processed 21,609 source frames into 23,296 engine frames and 182 scheduler
quanta; exclude processed 22,050 source frames into 23,936 engine frames and
187 quanta. Both used the explicit 44.1 kHz-to-48 kHz bridge, reported zero
rejected packets, scheduler xruns, and buffer overruns/underruns, then stopped
cleanly with persistent media configuration unchanged. This is asynchronous
user-mode process-loopback evidence, not managed-driver callback or physical
latency evidence.

## Adapter quantile validation correction (2026-09-08)

The shared and routed PowerShell wrappers incorrectly required the p99.9 upper
bound to be greater than or equal to the absolute maximum. That rejects valid
histogram results when a rare tail sample exceeds the p99.9 bucket. Both
wrappers now validate totals, sample counts, and histogram accounting without
imposing that invalid ordering. Corrected validation passed PowerShell parsing,
the five-second shared soak (1,878 graph blocks, zero xruns/overruns/deadlines),
and the two-second routed run (96,384 routed frames, zero deadline misses and
lateness). No persistent audio configuration changed.

## Five-second soak regression and harness repair (2026-09-08)

The initial five-second shared-adapter run exposed an acceptance-wrapper defect:
the probe completed with 240,480 capture frames, 1,878 graph blocks, zero
xruns/overruns/deadlines, but the wrapper rejected it because the absolute
processing maximum (111,500 ns) exceeded the p99.9 upper bound (65,536 ns).
That ordering is valid for a quantile with a rare tail. The shared and routed
wrappers now validate histogram/sample accounting and totals without requiring
the p99.9 bound to cover the maximum. PowerShell parsing, the corrected
five-second shared run, and a corrected two-second routed run passed; no
persistent audio configuration changed.

## Scheduler lifecycle queue invalidation (2026-09-08)

`RealtimeScheduler::publish`, successful `activate_session`, and `deactivate`
now recycle both bounded input and output rings. This prevents pending audio
captured for an old runtime generation from being processed after replacement
or stop. The regression queues an input block, replaces the graph, and checks
that the input pool is restored; it also verifies deactivation drains a queued
block. Engine tests (77), strict Clippy, and formatting pass. No audio endpoint
or machine configuration was accessed.

## Live Rust adapter and routed requalification (2026-09-08)

The guarded `m02-rust-adapter-live.ps1 -AllowLiveAudio -DurationMilliseconds
300` run passed with 14,880 capture frames, 14,848 scheduler frames, 116 graph
blocks, zero xruns/overruns/deadline misses, and a 32,768 ns processing p99.9
upper bound. The routed wrapper passed with explicitly selected VB-Audio
endpoints, 14,400 capture frames, and 14,336 scheduler/routed frames, also with
zero deadline misses/lateness. Both wrappers stopped/reset streams, removed
temporary outputs, and verified unchanged media/configuration state. This is
shared-mode adapter evidence, not managed-driver callback or physical-latency
evidence.

## WASAPI teardown hardening (2026-09-08)

All three WASAPI client types now attempt `Reset` after `Stop`, even if the
stop call fails, and clear their local `started` flag before invoking COM.
This closes a teardown/rollback hole without changing the endpoint selection
or stream configuration. Windows-audio tests (29) and strict Clippy pass.
The guarded shared adapter and explicitly selected VB-Audio routed smoke tests
also passed after the change; each stopped/reset its streams and verified
unchanged media/configuration state. This is lifecycle evidence, not managed
driver or physical-latency evidence.
## 2026-09-08 - Live adapter requalification

The guarded live adapter wrappers passed at `3179ad9`. The normal adapter
processed 14,880 capture frames and 14,848 scheduler frames across 116 graph
blocks. The explicitly selected VB-Audio route processed 14,400 capture and
14,336 routed frames across 112 blocks. Both runs had zero xruns, overruns,
deadline misses, and deadline lateness; streams stopped/reset and endpoint,
media, and persistent configuration snapshots were unchanged. This qualifies
the existing shared-mode adapter path only; it does not qualify a managed
driver callback, production endpoint ownership, physical latency, or signing.

## 2026-09-08 - RMS-window allocation bound

The engine's public `RmsWindow::new` now caps caller-requested storage at
480,000 samples, the ten-second upper bound used by the internal 48 kHz meter,
and rejects zero or oversized capacities before allocation. The regression
covers both invalid boundaries; engine coverage is 77 tests with strict
Clippy. This is a portable allocation-safety boundary and does not access
audio devices or machine configuration.

## 2026-09-08 - Native built-in gain transformation

The guarded `m02-rust-adapter-route-live.ps1` wrapper requalified the
repository's built-in gain graph on explicitly selected VB-Audio endpoints for
300 ms. The route processed 14,880 captured frames into 116 graph blocks and
14,304 routed frames, with zero deadline misses and a 32,768 ns p99.9
processing-time bound. Streams stopped/reset successfully and endpoint,
media, and persistent configuration snapshots were unchanged. This proves
native built-in DSP execution through the shared-mode adapter; managed-driver
callback timing, physical latency, and production signing remain open.
## Negotiated-rate deadline requalification (2026-09-08)

After the adapter deadline-rate correction, the guarded endpoint-ID-selected
route was rerun for 500 ms from the documented 96 kHz mono capture endpoint
`{0.0.1.00000000}.{2b694137-729a-4e08-b290-e891a6bc2487}` to the documented
48 kHz stereo render endpoint
`{0.0.0.00000000}.{1869e2ef-82c1-4602-a35a-be804a32112a}`. The route passed
with 48,000 capture frames, 187 graph blocks, 23,936 scheduler frames, and
23,936 routed frames. Capture rate was 96,000 Hz, render rate was 48,000 Hz,
the graph quantum was 128 frames, and the computed graph deadline was
1,333,334 ns. Processing p99.9 was 16,384 ns; deadline misses and lateness
were zero, and the wrapper reported zero scheduler overruns/XRuns.
The wrapper verified unchanged endpoint/media state and restored/cleaned all
temporary resources. This is shared-mode adapter evidence, not managed-driver
callback, independent-clock, or physical-latency qualification.

Command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m02-rust-adapter-route-live.ps1 -AllowLiveAudio -DurationMilliseconds 500 -CaptureEndpointId '{0.0.1.00000000}.{2b694137-729a-4e08-b290-e891a6bc2487}' -RenderEndpointId '{0.0.0.00000000}.{1869e2ef-82c1-4602-a35a-be804a32112a}'
```
## Guarded digital impulse correlation (2026-09-08)

The repository's guarded `m00-native-impulse.ps1` acceptance was run with
`-AllowLiveAudio -ImpulseCount 100` against the explicitly named existing
VB-Audio virtual-cable render/capture pair. It detected 97 of 100 impulse
groups, with p95 spacing error of 0 frames and an estimated onset of 72.21 ms.
The streams were stopped/reset and temporary artifacts were removed; the
wrapper verifies media-state preservation and does not change defaults or
persistent audio configuration. This is bounded digital signal correlation,
not calibrated acoustic p95 latency, independent-clock evidence, managed-driver
callback timing, or production-driver qualification.

## Guarded native lifecycle and attribution requalification (2026-09-08)

The guarded native lifecycle wrapper passed all 13 capture and 18 render
endpoints at a 100 ms duration; one render endpoint reported the expected
ownership conflict and satisfied the wrapper's explicit conflict criteria. The
event-driven wrapper then passed the selected VB-Audio pair at 200 ms with
10,080 capture frames and 14,400 submitted render frames. Finally, the
controlled process-attribution wrapper passed at 500 ms with 21,609 capture
frames and 77,823 nonzero bytes from the selected disposable child process.
Each wrapper stops/resets its streams, removes temporary binaries, and verifies
media-device state preservation. This is native shared/event/process-loopback
evidence only; it does not establish managed-driver callback timing, calibrated
physical latency, or an actual PID-reuse occurrence.

## Guarded loopback and exclusion requalification (2026-09-08)

The guarded endpoint-loopback signal-path acceptance passed on the explicitly
named VB-Audio pair with a 500 ms capture and 800 ms tone, producing 72,054
nonzero payload bytes. The guarded process-exclusion acceptance also passed at
500 ms with 22,050 capture frames and the disposable controlled child excluded
from the selected process tree. Both wrappers stop/reset streams, remove
temporary binaries, and verify unchanged media state. This evidence covers
selected Windows API/data paths only; it does not establish full cross-process
isolation, calibrated physical latency, PID reuse, or managed-driver behavior.
## Read-only native format inventory requalification (2026-09-08)

The guarded native format inventory passed for 31 endpoints. The observed
formats included 48 kHz 32-bit extensible mono and stereo, 96 kHz 32-bit mono,
and 96 kHz 32-bit eight-channel endpoint mix formats. The wrapper only performs
endpoint activation/metadata inspection, removes its temporary executable, and
verifies media-device state preservation. This is endpoint-format evidence, not
evidence of arbitrary format negotiation, managed-driver routing, or physical
latency.

## Rate-aware adapter smoke assertion (2026-09-08)

The live adapter wrapper was hardened to require the probe's capture/render
rates, 128-frame quantum, and computed graph deadline, then independently
recalculate the expected deadline with ceiling nanosecond arithmetic. The
requalified 250 ms smoke passed at 48 kHz capture/render with a 2,666,667 ns
deadline, 12,480 capture frames, 97 graph blocks, and zero deadline misses,
XRuns, or queue overruns. Stream teardown and media-state preservation passed.
This remains shared-mode adapter evidence, not managed-driver callback or
physical-latency qualification.

## Differing-rate route requalification (2026-09-08)

The hardened route wrapper passed for 250 ms using the documented 96 kHz mono
capture endpoint and 48 kHz stereo render endpoint. It processed 23,040
capture frames into 90 graph blocks, 11,520 scheduler frames, and 11,424
routed frames. The computed 128-frame deadline was 1,333,334 ns; processing
p99.9 was 65,536 ns, with zero deadline misses/lateness and no telemetry
accounting faults. Endpoint/media state was unchanged after teardown. This is
shared-mode adapter evidence only, not managed-driver callback, independent
clock, or physical-latency qualification.

## Exact-binding endpoint rebind transaction (2026-09-10)

`WasapiEndpointWorker::rebind_with_refreshed_bound_with_retry` now provides the
control-thread lifecycle boundary around the two endpoint clients and the
preallocated scheduler bridge. It stops and discards the old capture/render
pair before opening only the exact persisted bindings through the existing
monitor and bounded retry helpers. A successful rebind is deliberately left
stopped for an explicit subsequent `start`; an open failure cannot leave the
old pair processing or silently select a replacement. The focused
Windows-audio suite passed 53 tests, strict package Clippy passed, and
formatting/diff checks passed. No endpoint was opened by these checks; managed
driver ownership, production callback timing, signing, and physical latency
remain open.

## 2026-09-11 — Control-thread notification refresh

`ControlPlane` now lazily owns one `EndpointMonitor` after the first
`devices.list` request. The monitor consumes the atomic notification dirty bit
on the control thread and refreshes its active endpoint snapshot only when a
notification is pending; destruction unregisters the callback. Discovery
errors remain structured and no route is automatically rebound. The callback
still performs only an atomic store, preserving the realtime and callback
invariants. Focused control (106) and Windows-audio (56) tests, strict Clippy,
formatting, and diff checks passed. Inactive format-optional records and
public snapshot-change events remain open.

## 2026-09-11 — Full guarded acceptance requalification

The complete `tests/acceptance/safe-all.ps1` chain passed at pushed head
`744c2194`. It rechecked the Windows toolchain, non-installing AudioRouter
driver build, read-only endpoint inventory, disposable SysVAD, portable DSP and
UI, pinned VST3 SDK/native workers, VST2 legacy/state fixtures, headless
control, unsigned M08 artifacts, traceability, and documentation. The runner
removed 13 run-owned temporary children. This is compile/portable/disposable
evidence only; no driver was installed or loaded, no stream was opened, and no
persistent audio configuration was changed.

## 2026-09-11 — Bounded endpoint-change event signal

When `devices.list` consumes a notification and the active endpoint snapshot
diff is non-empty, the control plane now appends one bounded `devices.changed`
state event. The event intentionally carries no endpoint payload; clients
refetch the authoritative snapshot, preventing stale or oversized event data
and preserving exact-ID binding behavior. Focused control (106) and
Windows-audio (56) tests, strict Clippy, formatting, and diff checks passed.
An injected transition seam is still needed for deterministic inactive-to-active
event tests without modifying live endpoint state.

## 2026-09-11 — Pure endpoint-state transition seam

`diff_endpoint_state_snapshots` now provides a deterministic identity/state
diff without format activation. Tests cover unplugged-to-active recovery,
removal, addition, and stable ordering. This supports future control event
replay tests without mutating the user's live device state. Windows-audio
coverage passed 57 tests with strict Clippy, formatting, and diff checks.

## 2026-09-11 â€” Control event replay coverage

The control-plane endpoint-change signal now has a dedicated replay regression:
an injected non-empty transition produces one `devices.changed` state event,
an empty transition produces none, and `events.subscribe` returns the event
through its category filter and cursor response. Control coverage passed 107
tests with strict Clippy, formatting, and diff checks. This is deterministic
control evidence; it does not claim a live endpoint transition or driver
activation.

## 2026-09-11 â€” Opt-in format-optional inactive inventory

`devices.list({includeInactive:true})` now merges the active monitor snapshot
with the all-state identity inventory. Active items retain negotiated format
and period metadata; disabled, unplugged, not-present, and unknown-state items
omit those fields rather than inventing values or attempting activation. The
response schema, TypeScript union, and UI presentation preserve this
distinction. Focused control (106), Windows-audio (56), contracts/UI
typechecks, UI (121), strict Clippy, formatting, and diff checks passed. No
stream or persistent audio configuration changed.

## 2026-09-11 — Authorized existing-endpoint route probe

`tests/acceptance/m02-rust-adapter-route-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500` ran against the existing VB-Audio virtual-cable
render/capture pair. It captured 24,480 frames, processed 191 graph blocks,
and routed 24,448 frames at 48 kHz with a 128-frame graph quantum. Processing
time totaled 4,196,600 ns with a 55,700 ns maximum; deadline misses and
deadline lateness were zero. The harness compared the before/after media
identity/state snapshot and removed its temporary executable/object. Defaults,
volume, mute, privacy, drivers, signing, startup configuration, and endpoint
registration were unchanged. This is live adapter evidence, not production
driver or physical-latency qualification.

## 2026-09-11 — Control-owned endpoint route probe

The new `m02-control-route-live.ps1` harness requires explicit endpoint IDs
and `-AllowLiveAudio`, snapshots present media identity/state, and invokes the
control-owned route probe. Against the existing VB-Audio cable it ran for 500
ms at generation 1, processing 50 packets/24,000 captured frames, 187 graph
quanta, and 23,936 rendered frames at 48 kHz. Its recorder tap finalized a
4,140-byte WAV before the worker was explicitly stopped and detached; the
before/after media snapshot was unchanged. This qualifies control-owned graph
publication, prebuilt tap delivery, bounded pumping, and file finalization on
existing endpoints, not driver installation, production signing, or physical
latency.
# 2026-09-11 - Stereo-linked built-in dynamics

The portable graph compiler now constructs stereo compressor and gate stages
with one two-channel detector instead of two independent mono detectors. The
DSP layer exposes allocation-free planar linked processing, applying one
envelope/gain decision to both channels so a single-channel peak cannot move
the stereo image. Mono processing and malformed manually assembled stages
retain their existing behavior, including fail-closed silence when a required
stereo state is absent.

Validation: `cargo test -p audiorouter-dsp -p audiorouter-engine --locked`
passed 32 DSP tests and 102 engine tests. This is portable built-in processing
evidence for DSP-03; native driver callback timing and endpoint activation
remain separate gates.

## 2026-09-12 — Topology correction for virtual-bus taps

A provisional control integration that attached every enabled virtual bus to
every native graph was removed after review because it created an invisible
pass-through, contrary to the graph-authority and explicit-topology rules.
The reusable allocation-free `VirtualBusBridge` `AudioTap` remains; selecting
it must wait for an explicit `VirtualCaptureSink` route in the compiled graph.
No implicit bus activation is shipped.

Validation after removal: the full locked workspace suite passed, including
121 control and 105 engine tests, with strict relevant Clippy, formatting,
and diff checks. No endpoint was opened and no machine audio configuration
changed.

The compiler-level follow-up `cargo test -p audiorouter-engine
compiler_uses_one_stereo_detector_for_dynamics_nodes --locked` passed on
2026-09-12 for both compressor and gate nodes. It verifies that the graph
compiler selects the shared two-channel detector for stereo ports and preserves
the asymmetric channel ratio. Native callback and endpoint activation remain
separate gates.

The sequential workspace requalification
`cargo test --workspace --locked -- --test-threads=1` passed on 2026-09-12;
the updated engine suite reported 103 passing tests and the updated DSP suite
32 passing tests, with all other workspace suites also green. Documentation
validation passed separately with 52 Markdown files and 167 local links.

The native render-source worker teardown path was hardened on 2026-09-12:
endpoint stop is attempted even when staged scheduler reset fails, and the
first cleanup error remains visible. The focused
`cargo test -p audiorouter-windows-audio --locked` suite passed 63 tests;
strict package Clippy and `git diff --check` passed. No endpoint was opened.

## 2026-09-12 — Processed graph handoff to virtual-bus bridge

`VirtualBusBridge` now implements the engine's allocation-free `AudioTap`
boundary. Each processed block is copied into a bounded render handoff only
when the active generation matches; inactive, shape-invalid, or full handoffs
are dropped without waiting and remain fail-closed. The bridge's existing
control-side `process_once` then transfers accepted blocks to the bounded
capture side for the future virtual capture endpoint.

Validation: `cargo fmt --all -- --check` and
`cargo test -p audiorouter-engine --locked` passed; the engine suite reported
105 passing tests, including processed-audio handoff and inactive/full-drop
regressions. This proves the portable graph-to-bridge boundary only; loaded
driver, PortCls ownership, endpoint activation, signing, and physical latency
remain separate gates.

The follow-up `cargo test --workspace --locked -- --test-threads=1` passed on
2026-09-12 after control integration: all workspace unit/integration suites
and doc-tests passed, including 105 engine, 121 control, 63 Windows-audio,
and 67 plugin-host tests. No endpoint was opened and no machine audio or
driver configuration changed.

## 2026-09-12 — Guarded live adapter bridge

With explicit live-audio authorization, `tests/acceptance/m02-rust-adapter-bridge-live.ps1`
completed one 500 ms cycle against the existing VB-Audio render/capture
endpoints. Telemetry reported 24,480 captured frames, 191 processed quanta
and tap calls, 24,448 rendered frames, zero non-finite samples, dropped frames,
scheduler xruns, or deadline misses, and a 25,072-byte temporary recording.
The harness verified unchanged media identity/state before and after and
removed its temporary executable/object/recording outputs. This is native
user-mode adapter evidence only; no managed driver was installed or loaded and
no default device, volume, mute, privacy, or persistent audio setting changed.
## 2026-09-12 - Explicit virtual capture-sink ownership marker

Compiled runtime graphs now retain an immutable `has_virtual_capture_sink`
marker when an enabled `VirtualCaptureSink` node is present. Native graph
activation uses that marker together with the explicit route registry to add
only matching enabled bus bridge taps; graphs without the node cannot publish
virtual-bus output implicitly. Engine (106) and control (124) tests pass,
including formatting and strict Clippy. This remains user-mode ownership
evidence; the managed driver and physical endpoint gates are still open.

The control regression `virtual_route_taps_require_an_explicit_sink_and_select_only_matching_enabled_buses`
passed on 2026-09-12. It proves that no virtual tap is built without an
explicit capture-sink marker and that a sink-owned producer receives only its
matching enabled route bridge; unrelated producer routes are excluded.

The control-owned live route harness was re-run with explicit authorization on
2026-09-12 for one 500 ms cycle. It reached generation 1 with 50 packets,
24,000 captured frames, 187 processed quanta, 23,936 rendered frames, one
successful start/stop/reset, and one deliberate stale-generation rejection.
The worker detached cleanly and the media identity/state snapshot was unchanged.
This is existing-endpoint user-mode evidence; production driver installation,
signing, PortCls ownership, and physical latency remain open.
# Native adapter requalification after reboot (2026-09-12)

The authorized bounded system-selected adapter smoke passed for 500 ms at
48 kHz. It captured 24,480 frames in 51 packets, processed 191 graph blocks,
submitted 24,448 scheduler frames, and reported zero xruns, input/output
overruns, or deadline misses. The selected capture and render endpoint
identities were explicit; the probe stopped and reset both streams and found
unchanged media-device identity/state.

The authorized one-cycle explicit CABLE bridge also passed for 500 ms at
48 kHz. It captured 24,480 frames, processed 191 quanta/taps, rendered 24,448
frames, and reported zero non-finite tap samples, drops, xruns, or deadline
misses. Its temporary stream and recording were stopped/removed and media
state remained unchanged.

These are shared-mode user-space adapter and digital-bridge checks. They do
not qualify physical acoustic latency, PortCls callback ownership, a loaded
managed driver, or production signing. No persistent machine-audio setting
was changed.
## 2026-09-12 - default bridge constructor ABI alignment

The default `NativeBridgeController::create` path now delegates to the
section-backed constructor. The secured driver broker requires a mapped
section on `OPEN`; the previous unmapped request would have been rejected by
the intentionally fail-closed driver boundary. This preserves the public
compatibility entry point while giving it the same section lifetime and lease
ownership as the explicit path.

The focused Windows-audio suite passed 67 tests with formatting and diff
checks. No driver was installed or loaded and no audio endpoint or persistent
machine configuration was changed.

## 2026-09-12 - exactly-once controller close

`NativeBridgeController::close` now records successful broker closure before
flushing the user-mode session. Its destructor skips the broker request after
that point, while failed broker requests still use the existing best-effort
drop retry. This prevents duplicate close IOCTLs during normal ownership
teardown.

The focused Windows-audio suite passed 67 tests with formatting and diff
checks. No driver was installed or loaded and no audio endpoint or persistent
machine configuration was changed.

## 2026-09-12 - bridge leaf-path validation

`NativeBridgeRegion::open` now inspects the leaf with `symlink_metadata` and
rejects anything that is not a regular file before opening or mapping it.
Parent reparse checks remain in place, and a focused regression proves a
directory leaf is rejected without creating a mapping.

The focused Windows-audio suite passed 68 tests with formatting and diff
checks. No driver was installed or loaded and no audio endpoint or persistent
machine configuration was changed.

## 2026-09-12 - Windows reparse-point coverage

Bridge path validation now checks the Windows `FILE_ATTRIBUTE_REPARSE_POINT`
metadata bit for both existing parents and the mapping leaf, in addition to
Rust symlink metadata. This rejects junction-style redirection before a
shared-memory file is opened or mapped.

The focused Windows-audio suite passed 68 tests with formatting and diff
checks. No driver was installed or loaded and no audio endpoint or persistent
machine configuration was changed.
# 2026-09-15 - authorized VB-Cable bridge requalification

The elevated `tests/acceptance/m02-rust-adapter-bridge-live.ps1
-AllowLiveAudio -DurationMilliseconds 500 -Cycles 1` run passed against the
exact existing VB-Cable endpoints: capture
`{0.0.1.00000000}.{06268191-5f8c-42ed-827e-d3c7a19637ed}` and render
`{0.0.0.00000000}.{71f96f14-94c1-4189-b9b9-df8c960db8f2}`. At 48 kHz stereo,
the adapter processed 24,480 captured frames, 191 graph quanta/tap calls, and
24,448 rendered frames with zero non-finite tap samples, dropped render
frames, scheduler XRuns, or deadline misses. Temporary streams and the
recording were removed, and media identity/state were unchanged afterward.
This qualifies the existing VB-Cable user-mode route only; it does not qualify
the AudioRouter driver or physical latency.
## 2026-09-15 - control-owned VB-Cable lifecycle re-run

The authorized guarded acceptance used the exact active VB-Audio pair and
completed one 500 ms control-owned capture → graph → render cycle. It
processed 23,520 captured frames across 183 graph quanta and rendered 23,424
frames; the lifecycle test passed and the temporary process environment was
restored. No persistent audio configuration, endpoint default, volume, mute,
driver, or signing state was changed. This remains user-mode existing-device
evidence; loaded AudioRouter driver/PortCls timing and physical-latency gates
remain open.
