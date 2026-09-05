# 14 — Non-functional requirements and verification

Milestone ownership: M00 establishes measurement methods/reference machines; M01–M07 attach evidence incrementally; M08 enforces release gates. All numbers below are proposed product acceptance targets, not measured results.

## Reference environment and workloads

M00 records a concrete reference PC: Windows 11 x64, at least a contemporary four-core/eight-thread CPU, 16 GiB RAM, SSD, wired USB audio interface with physical loopback capability, and wired headphones. Also test an ordinary laptop with integrated audio and a separate USB microphone. Record exact CPU, Windows build, power mode, device/driver versions, negotiated periods, sample rates, and test harness revision. Do not substitute a high-end workstation result for the laptop compatibility result.

Reference workload W1: 48 kHz graph, 128-frame quantum if M00 confirms it, mono mic → EQ → gate → compressor → 5 ms lookahead limiter → virtual voice sink; mic monitor branch without extra high-latency effects; stereo desktop virtual source → headphone mixer and stereo game-recording sink; one PCM24 WAV recorder. W2 adds a VST3 worker, pitch branch, FLAC recorder, and a second asynchronous physical output. W3 exercises maximum graph/resource limits with cheap processors and a recorded, fixed topology. Resource limits are structural caps, not a promise that arbitrary heavy plugins fit the CPU budget.

## Quantitative requirements

| ID | Target and measurement | Gate |
| --- | --- | --- |
| NFR-01 | W1 wired mic-to-headphones physical loopback latency p95 ≤30 ms over 1,000 impulses; publish min/p50/p95/max and negotiated buffers | M02 baseline, M04 W1, M08 |
| NFR-02 | W1 mic-to-virtual-capture latency p95 ≤40 ms measured by a timestamped local capture client; excludes Discord network/codec delay | M03/M04/M08 |
| NFR-03 | AudioRouter-added scheduling/buffering delay ≤10 ms p95 excluding device periods and declared DSP lookahead; instrument each term rather than subtracting guessed hardware latency | M02/M08 |
| NFR-04 | Realtime callback execution p99.9 <50% of its quantum deadline for W1; zero missed callbacks attributable to the engine during 8-hour W1 soak on reference PC | M02/M04/M08 |
| NFR-05 | W1 backend plus recorder CPU average ≤10% of total machine capacity, p95 ≤20%; engine working set ≤250 MiB; idle stopped backend ≤60 MiB | M04/M08 |
| NFR-06 | UI working set ≤400 MiB; aggregate plugin-free W1 process working set ≤750 MiB; report plugin workers separately; retained memory growth <10 MiB after warmup over 8 hours | M05/M08 |
| NFR-07 | Warm local query p95 ≤100 ms; ordinary graph plan/commit acknowledgment p95 ≤200 ms for 32 nodes, excluding device open/plugin preparation; completion/status always distinguishes these | M01/M07 |
| NFR-08 | Meter freshness p95 ≤150 ms; default 20 Hz and maximum 30 Hz; UI input-to-paint p95 ≤50 ms at 64 visible nodes on reference PC | M05/M08 |
| NFR-09 | Emergency privacy mute applied within two audio blocks after backend receipt; input-to-effective mute p95 ≤100 ms locally; persistent latch survives restart | M02/M07 |
| NFR-10 | Backend heartbeat loss silences virtual capture within 500 ms; ordinary device-return recovery ≤5 s after Windows announces readiness; no stale-frame replay | M03/M07 |
| NFR-11 | Process-corruption-free 24-hour W1 operation, 8-hour W2 soak with injected worker/device failures, and 100 sleep/resume or reconnect cycles; no backend crash or unrecovered resource leak | M07/M08 |
| NFR-12 | Two active sessions; ≤64 nodes/128 edges per session; ≤128 nodes/256 edges globally; eight virtual buses, eight simultaneous recorders, eight plugin instances, eight inputs per mixer | M01 enforcement, M08 load |
| NFR-13 | Eight asynchronous hardware streams globally, maximum four physical captures and four physical renders; capability reporting lowers unavailable hardware limits explicitly | M02/M08 |
| NFR-14 | App UI opens ready within 3 s warm / 8 s cold; approved W1 startup ready within 10 s after backend launch when devices are ready; no startup audio before permissions resolve | M05/M07/M08 |
| NFR-15 | Functional core operates with network disabled; no mandatory account, remote assets, or external model connection | M05/M07/M08 |
| NFR-16 | All normal configuration operations run as standard user; only driver/package administration elevates | M03/M08 |

Windows scheduling, hardware drivers, Bluetooth, other software, and plugin algorithms affect results. A failure to meet the reference target requires optimization or an explicit revised acceptance decision before release. Do not erase failed samples as “external” without causal evidence. W2 must publish its measured added latency; pitch and plugins cannot inherit W1's low-latency guarantee automatically.

## Signal correctness requirements

- **QUAL-01 — Routing isolation.** In digital fixtures, absent routes shall be exactly zero before optional dithering. In loopback hardware/app tests, identify forbidden source tones using correlation/spectral analysis and require at least 80 dB rejection relative to included tones where the physical noise floor permits; otherwise record measured floor and use the digital test as decisive topology proof. Do not confuse ambient acoustic pickup with graph leakage.
- **QUAL-02 — Numeric fidelity.** Unity/flat internal float paths shall differ from the reference by ≤1e-6 absolute sample error. Test fixed channel maps, polarity, mono duplication, stereo downmix, and mixer summation. Declared limiters/dither/converters have separate expected references. No NaN/Inf may reach a sink.
- **QUAL-03 — DSP.** Validate EQ response within ±0.5 dB away from transition/numerical extremes, notch attenuation ≥30 dB at configured center for the tested tone, compressor static transfer within ±0.5 dB, gate attenuation within ±1 dB of configured range after settling, and limiter sample peaks within 0.1 dB of ceiling. Publish frequencies/levels/window lengths and bypass behavior.
- **QUAL-04 — Timing and drift.** With simulated ±100 ppm clock mismatch and a real dual-device 8-hour test, queues remain bounded with no monotonic latency growth. Aligned graph mixer impulses agree within one graph sample after declared compensation. Physical outputs may differ by unknown hardware delay; report rather than claiming sample synchronization across devices.
- **QUAL-05 — Glitches.** Record underrun/overrun, stale worker frame, discontinuity, and clipping counters independently. During a controlled steady-signal gain/bypass/topology test, changes use the declared ramp and produce no unexplained discontinuity; compare against a reference transition waveform. A single “sounds fine” listen is insufficient.
- **QUAL-06 — Recording integrity.** Decode produced files and verify exact expected frame counts, formats, split boundaries, and metadata. Pauses/failures have explicit timeline intervals. Test crash recovery target in [08](08-recording.md) and verify remaining live routes do not lose frames during disk errors.

## Maintainability and engineering quality

- **ENG-01 — Contracts.** Schema validation and generated-client drift checks run in CI. Every public method has a success fixture and representative error fixture; high-risk operations include authorization/conflict/retry cases. Avoid UI tests that merely assert a button exists.
- **ENG-02 — Code health.** Pin toolchains/dependencies, run Rust formatting/lint/unit tests and TypeScript typechecking/lint/build on affected code, and document every unsafe/FFI boundary. Fuzz untrusted parsers/compiler/import and use bounded input corpus regression. CI must distinguish portable tests from Windows-only jobs.
- **ENG-03 — Observability.** Provide per-node health, active graph generation, negotiated device format/period, estimated/measured latency labels, CPU/queue load, drift, xrun counters, recorder throughput, and plugin failures. Diagnostic collection is bounded and off-thread; logs use operation IDs and redact sensitive fields.
- **ENG-04 — Documentation.** Each milestone supplies API examples, user instructions for changed behavior, architecture decisions, validation evidence, known limitations, and a next-step handoff. Update active/archived/future plans and validated lessons as prescribed by [AGENTS.md](../../AGENTS.md).
- **ENG-05 — Release reproducibility.** A clean pinned Windows build produces traceable app/CLI/MCP/driver artifacts, dependency notices/SBOM, version metadata, checksums, and test reports. Signing credentials remain external to source and logs. Record build inputs even where signatures/timestamps prevent byte-for-byte reproducibility.

## Required test layers

| Layer | Examples | What it cannot prove |
| --- | --- | --- |
| Domain/unit/property | Cycles, channel matrices, limits, permissions, revisions | Actual sound or Windows compatibility |
| Deterministic DSP | Impulses, tones, silence, dynamic controls | All subjective speech quality |
| Fake-device integration | Transactions, event loss, clock drift, worker/disk faults | Driver/OS timing |
| Windows integration | WASAPI, PID rebinding, endpoint lifecycle, app compatibility | Broad hardware population |
| UI automation/accessibility | Keyboard flow, external edits, layout, Narrator audit | Audio isolation by screenshots |
| Hardware and endurance | Analog loopback, USB drift, load, sleep/resume | Universal latency on every PC |
| Installer/security | Standard user, Secure Boot/HVCI, update/rollback, hostile requests | Immunity to compromised OS |

M08 tests at least two Windows 11 builds supported at release, the reference PC and laptop, USB/integrated audio, and informational Bluetooth cases. Discord/OBS/browser/game fixtures record actual versions. Exact OS maintenance status is reverified at release; the specification does not freeze an obsolete build forever.

## Evidence format

Each report records requirement IDs/scenario, commit/build ID if available, date, machine/OS/drivers, workload and fixture seed, commands, measured distributions, failures, logs/artifact paths, and conclusion. Attach raw data for latency/glitch claims and sanitized graphs for routing results. State `not run`, `blocked`, or `failed` explicitly. No Linux execution, mocked backend, or typecheck alone can pass a Windows hardware gate.
