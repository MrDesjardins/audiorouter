# 2026-09-30 Spectral Gate naming and two-tool composition

Status: requested display rename and deterministic built-in pair checks complete.
Requirements: DSP-01/02/03/04/05/06/08/11/12/13/15/16/17, GRAPH-02/10/14,
QUAL-01/02/03, UI-04 (partial offline evidence, not complete milestone gates).
Environment: Windows 10.0.26200.0 x64, pinned Rust 1.96.0, locked Cargo inputs.

## Names and compatibility

The frequency-dependent learned gate is now **Spectral Gate**; **FIR Filter**
retains its name for impulse-response convolution. Library, new-node defaults,
canvas type caption, connection picker and editor accessibility labels agree.
`spectralGate` and `spectral-gate@1` are unchanged, as are parameters and all
saved user-defined names. Existing nodes named "FIR Filter Hz" remain valid.
No database migration or desktop launch occurred.

## Pair matrix and references

All 289 ordered pairs of 17 built-ins, including same-kind pairs, run at
48/96 kHz in mono/stereo with two paired parameter sets: 2,312 active cases.
Tools: Gain, Volume, Mute, Delay, Advanced EQ, Graphic EQ, Bass & Treble,
Dehum, Compressor, Gate, Limiter, Declick, Denoise, Speech Denoise,
Spectral Gate, FIR Filter and Pitch. FIR uses a deterministic sparse impulse
response crossing the partition boundary; noise processors have stored profiles.

Each input has initial silence, distinct stereo tones, mains hum, fixed-seed
noise, a click, a level step and trailing silence. Each connected graph's node
array is reversed to ensure edges determine processing order. Compare every
sample against fresh single-tool A execution followed by fresh B execution at
identical 128-frame boundaries. No trimming or latency realignment hides delays.
Finite output and absolute error <=1e-6 are required. This isolates composition
and complements existing independent DSP transfer-law tests; it is not an
independent implementation of each processor.

Neighbor pairs exercise each tool on both sides with first, second or both
bypassed: 408 cases. Independent analytical references add 24 EQ/Dehum
product-response cases (three frequencies, two rates, both orders/repeated
kinds) and five gain/delay/compressor cases. Double Delay must add exactly
288 frames at 48 kHz; Gain/Delay must scale at the expected offset. Gain before
and after Compressor must produce the distinct hard-knee static levels, over
4 dB apart. Combined magnitude tolerances remain 0.5 dB.

Total new pair cases: **2,749**. Total retained graph cases: **3,813**, plus
48 native gain configurations. Inputs/outputs/metrics are generated ignored
artifacts under `target/two-tool-evidence`; no personal audio is used.
Mixer/Input Switch and Time Shift transport stay in their dedicated matrix;
arbitrary vendor plugin pairs and every possible parameter cross-product are
not claimed. Live WASAPI, hardware timing and listening remain unrun.

## Commands and outcomes

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/deterministic-audio.ps1 -TargetDirectory target/groups-http-20260929 -ArtifactDirectory target/two-tool-evidence -NativeFixtures -Vst2Fixture third_party/local-test-fixtures/Vst2State/audiorouter-vst2-state-fixture.dll -Vst3Fixture third_party/vst3sdk-build/VST3/Release/again.vst3/Contents/x86_64-win/again.vst3 -Vst3Worker target/deterministic-plugin/m06-vst3-worker.exe
```

Exit 0; 33 signal tests, 52 DSP +145 engine unit tests, the separately rerun
Dehum preservation gate, and two actual native plugin tests pass. Logs and
`qualification.json` are in the artifact directory. A first matrix fixture
used incorrect Volume/Graphic EQ parameter spellings; backend validation
rejected it. Corrected to `percent` and `band<N>Db` before the final run.

From `ui`:

```powershell
npm.cmd run test -- --run src/SpectralGateEditor.test.tsx src/draft.test.ts src/library.test.ts
npx.cmd playwright test e2e/unfed-processors.pw.ts --config=playwright.config.ts
npm.cmd run build -- --outDir ../target/two-tool-ui-build
```

All exit 0: 60 focused UI tests, three headless Edge tests (dark/light/high
contrast), TypeScript checking and production UI build. Screenshots inspected:
`%TEMP%/audiorouter-unfed-{dark,light,high-contrast}.png`; Spectral Gate label
and warning are readable and contained. Windows Playwright teardown initially
waited on its Vite child; only that identity-checked disposable server was
terminated, after which the runner exited 0. Sandbox write restrictions blocked
the initial build; the authorized build succeeded in a separate output directory.
No user's app process or audio endpoint was touched. Desktop exe not rebuilt.
Documentation acceptance passes: 79 Markdown files and 422 local links.
Test-file rustfmt and `git diff --check` also pass.

Rollback: revert display labels and pair tests/docs; identifiers stay stable.
Next: guarded native continuity/performance and real-speech listening evidence.
