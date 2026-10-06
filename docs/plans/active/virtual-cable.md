# Active plan — AudioRouter virtual cable (DEC-18)

Updated 2026-10-05. WP-02 package tooling complete; WP-01 host baseline
passed, VM evidence pending. No driver has been loaded.
All implementation and testing happens on
the user's Windows 11 development PC and in its Hyper-V test VM.

- Decision: DEC-18 in [15-delivery](../../spec/15-delivery.md).
- Design (what to build): [17 Virtual cable](../../spec/17-virtual-cable.md).
- Testing (how to prove it): [virtual cable testing](../../operations/virtual-cable-testing.md).
- Signing (human steps, costs): [virtual cable signing](../../operations/virtual-cable-signing.md).
- Background, costs and gap analysis: [driver track](../future/M03-driver-signing.md).

## Objective

Ship AudioRouter-owned virtual cables (up to 8, Cable A–H; 2 enabled by
default, the user picks 1–8) with the app, with studio-grade sound
(bit-exact float path, no hidden processing) and low latency, and built so
that the signed driver rarely or never needs to change ("sign once", 17 §1),
installed with one click and one UAC prompt, so users no longer need
VB-Cable. Requirement IDs: VCAB-01–12, VCAB-20–30 (performance and sound quality), VDEV-01–12 (with the deviations in
17 §8), SEC-08, NFR-02, NFR-16, DIST-01/02/03/06/07, UI-17.

## Rules for any agent working on this plan

1. Read `AGENTS.md`, this file, [17](../../spec/17-virtual-cable.md) and the
   [testing procedure](../../operations/virtual-cable-testing.md) completely
   first.
2. **Never install, load or test the driver on the host PC** until stage C,
   and then only the Microsoft-signed package. Never run `bcdedit`, change
   Secure Boot, HVCI or test signing on the host. Driver loading happens only
   in the VM `AR-DriverTest`, which the user operates; the agent prepares
   scripts and packages and reads the evidence the user copies back.
3. Do one work package (WP) at a time, in order, unless the dependency table
   says two can run in parallel. Before coding a WP, write its "Status" line
   here as `in progress` with the date.
4. Each WP ends with: tests listed in the WP run and passing (exact command,
   environment and result recorded under "Evidence"), diff reviewed, docs
   updated, files committed **by name** (validated lesson 2026-10-04).
5. Never report a check as passed that was not run. A VM result is not a
   stage C result; a build is not a load test.
6. Keep the kernel change small and reviewed; every `unsafe` Rust block and
   every kernel function touching user memory gets a comment with its
   invariants (AGENTS.md architecture rules).
7. **Performance and sound quality come first** (user, 2026-10-05). A WP
   that touches the audio path runs the relevant VCAB-20–30 checks, not only
   functional tests, and records the numbers. Never trade a quality target
   for convenience without the user's decision.
8. **Sign once.** Anything that may need to change after release goes into
   the registry configuration (17 §5.5), the protocol extension/capabilities
   (17 §5.2) or AudioRouter itself — not into a future driver build.
9. If a WP finds the design wrong, stop, record the finding here, propose the
   change to 17, and ask the user before continuing.

## Open decisions (WP-00) — proposed defaults, user to confirm

| ID | Question | Proposed default |
| --- | --- | --- |
| D1 | How many cables in the first release? | **Decided 2026-10-05:** up to 8 (A–H, 16 endpoints), user enables the first N (1–8, default 2); develop and test with 2, qualify all 8 before signing (A15) |
| D2 | Endpoint names | `AudioRouter Cable A Input` / `AudioRouter Cable A Output` (and B), VB-Cable style |
| D3 | Where install is offered | First-run guide and Setup → Virtual cables; never automatic |
| D4 | On app uninstall | Ask; default "keep the cables" |
| D5 | Formats | **Decided 2026-10-05:** 44.1/48/96 kHz, 1–8 channels, float32 (default) and PCM 16/24/32; default 48 kHz float stereo |
| D6 | Accept the deviations in 17 §8 ("first N of 8" instead of freely created/deleted buses; Windows names not renameable) | Yes for first release |

Status: waiting for the user. WP-02 onward may start with the defaults; a
changed answer is applied before WP-05.

## Dependencies

Execution authorization (2026-10-05): user requested all steps possible on this
computer. Finish each WP's host implementation/checks in order, record its VM
gate as pending, and proceed with independent downstream host preparation.
This does not authorize loading a test driver on the host or waive VM,
hardware, signing, purchase or publication gates. Kernel security review still
uses the fresh agent context required by WP-04.

```text
WP-01 (VM) ─┬─► WP-02 (test-signed build) ─► WP-03 (VM smoke script)
            │                                   │
            │   WP-04 (security fixes) ─────────┤
            │   WP-05 (4 endpoints/formats) ────┤
            │   WP-06 (protocol 1.1 + Rust) ────┤
            │                                   ▼
            │   WP-07 (helper) ─► WP-08 (status/API) ─► WP-09 (engine) ─► WP-09b (quality harness) ─► WP-10 (UI) ─► WP-11 (installer)
            │                                   │
            └──────────────────────────────────►WP-12 (stage A full) ─► WP-13 (signing, $) ─► WP-14 (main PC) ─► WP-15 (beta + release)
```

WP-04, WP-05 and WP-06 all edit the driver; do them one after another in
that order (one writer per tree). WP-07 can start after WP-03 in parallel
with the driver work if a second worktree is used.

---

## WP-01 — Windows development PC and test VM

- **Who:** user (agent guides). **Where:** Windows host + VM. **Cost:** $0.
- **Prerequisites:** Windows 11 Pro/Enterprise; Visual Studio 2022 with
  "Desktop development with C++"; the Windows Driver Kit (WDK) matching the
  installed Windows SDK; Rust, Node.js, Git (as in the README); a Windows 11
  ISO; 16 GB+ RAM and 100 GB free disk.
- **Steps:** follow [testing → Stage A one-time host setup](../../operations/virtual-cable-testing.md#one-time-host-setup-main-pc-administrator-powershell)
  and "Prepare the VM for test signing". Create checkpoints
  `01-clean-windows` and `02-test-signing-ready`.
- **Also on the host:** run `tests/acceptance/m03-driver-build.ps1` to prove
  the existing driver still builds, and `cargo test -p audiorouter-windows-audio`
  to get a baseline.
- **Acceptance:** VM boots with "Test Mode" shown on the desktop; the host
  shows Secure Boot unchanged (`Confirm-SecureBootUEFI` returns the same as
  before); both checkpoints exist; driver build passes.
- **Evidence:** a short note in this file (Windows build numbers, WDK
  version, VM name, checkpoint names, build result).
- **Rollback:** delete the VM; disable the Hyper-V feature.
- **Status:** in progress 2026-10-05 (host baseline); VM evidence pending.

## WP-02 — Test-signed driver package

- **Who:** agent on the Windows host. **Requirements:** VDEV-09 (dev side),
  SEC-08.
- **Files:** `drivers/audiorouter-virtual/build.ps1`, new
  `drivers/audiorouter-virtual/sign-test.ps1`, `.gitignore`.
- **Steps:**
  1. Add `-Version <X.Y.Z>` to `build.ps1`; stamp `DriverVer` in the
     generated INF (today hard-coded `02/22/2016, 1.0.0.1` in
     `Source/Main/AudioRouterVirtual.inx`) using the WDK `StampInf` step or
     MSBuild property `/p:StampInfDriverVer=...`. Today's date, version
     `X.Y.Z.0`.
  2. Keep `/p:SignMode=Off` for the plain build. New `sign-test.ps1 -Package
     <dir>`:
     - create, if missing, a self-signed code-signing certificate
       `CN=AudioRouter Test Driver` in `Cert:\CurrentUser\My` (use the
       existing `WDKTestCert` if present);
     - sign the `.sys` first (embedding its signature changes its bytes),
       then run `Inf2Cat /driver:<dir> /os:10_X64` (Windows 11 uses the 10 OS
       code family in Inf2Cat; verify the accepted values with
       `Inf2Cat /?`);
     - sign the `.cat` with the selected test certificate's exact thumbprint;
       SHA-256, optional RFC 3161 timestamp URL (offline by default);
     - export only the public certificate to `<dir>\AudioRouterTest.cer`.
  3. Output layout (both for tests and later for the release):
     `audioroutervirtual.inf`, `audioroutervirtual.sys`,
     `audioroutervirtual.cat`, `LICENSE-MS-PL.txt`, `package.json`
     (`{ version, builtAt, gitCommit, signed: "unsigned" | "test" | "microsoft" }`,
     plus driverVersion, platform, configuration and dirty-tree flag).
  4. `build.ps1 -TestSign` performs build plus signing in one command.
     Never commit `.pfx`, `.cer`, `.sys` or `.cat` files; add to
     `.gitignore`.
- **Tests:** `signtool verify /pa /v` on the `.sys` and `.cat` (expected:
  chain to the test root, which is untrusted on the host — that is
  correct); `InfVerif /v /h <inf>` (record warnings; errors must be zero
  before WP-13).
- **Acceptance:** package folder produced in one command from a clean
  checkout; signature verification output recorded.
- **Rollback:** revert the two scripts.
- **Status:** complete 2026-10-05. [WP-02 evidence](evidence/2026-10-05-virtual-cable-wp02.md):
  x64/ARM64 build acceptance, clean-tree one-command test package, valid INF,
  33 packaging/integrity checks. No loaded-driver or production-signing claim.

### Execution record (2026-10-05)

User authorized starting this plan with testability, performance and sound
quality as priorities. D2–D4/D6 remain proposed defaults, as WP-00 permits;
this tooling does not settle product decisions. Work on the host is build-only.
The Hyper-V cmdlets are unavailable in this session; VM/checkpoint and host
Secure Boot evidence have not been supplied. WP-01 is not complete. Independent
package preparation proceeds; no loaded-driver or audio gate is waived.

Requirements/scenarios: VDEV-09 development packaging, SEC-08 isolation,
ENG-05 provenance; WP-02 stamped INF, unsigned build, test-signed SYS/catalog,
public certificate export, signature/integrity checks and invalid-input tests.

Ordered tasks: (1) baseline existing x64 acceptance and windows-audio tests;
(2) validate version/output and stage exact configuration artifacts with WDK
StampInf and metadata; (3) sign SYS before generating its catalog, sign catalog,
export only CER, record verification without trusting a root on the host;
(4) test pure packaging rules and real WDK tools, inspect diff, document and
commit exact paths. No audio-path changes in WP-02; VCAB-20–30 measurements
start with the WPs that touch audio. Rollback: revert package tooling commits;
delete only the exact generated package; leave existing host trust/boot/audio
configuration intact. A newly created test certificate stays in CurrentUser/My,
never Root/TrustedPublisher, and its key is non-exportable.

Validation/evidence: baseline x64 `m03-driver-build.ps1` passed; Windows Rust
`cargo test -p audiorouter-windows-audio`: 110 passed, 2 live tests ignored.
Logs: `target/driver-baseline-build.log`, `target/driver-baseline-rust.log`
(local disposable evidence). Windows reported 10.0.26300.0; WDK tools
10.0.28000.0 installed. CIM OS inventory denied access; no build number from
CIM claimed. Final commands, artifact hashes, initial failures and limitations
are recorded in [WP-02 evidence](evidence/2026-10-05-virtual-cable-wp02.md).

## WP-03 — VM smoke script (A1, A2, A3, A14)

- **Who:** agent writes; user runs in the VM.
- **Files:** new `tests/acceptance/m03-driver-vm.ps1`.
- **Behavior:**
  - refuses to run unless `bcdedit /enum {current}` shows `testsigning Yes`
    **and** the computer name or a marker file `C:\ar\IS_TEST_VM` exists
    (so it cannot run on the host by mistake);
  - parameters: `-Package <dir>`, `-Evidence <dir>` (default
    `C:\ar\evidence\<timestamp>`), `-KeepInstalled`;
  - A1: saves `pnputil /enum-drivers`, `pnputil /enum-devices /class MEDIA`,
    `Get-PnpDevice -Class AudioEndpoint -PresentOnly | Select FriendlyName,
    InstanceId, Status`, default devices (via `Get-CimInstance
    Win32_SoundDevice` and the AudioRouter CLI `devices list --json` when
    available);
  - A2: `drivers/audiorouter-virtual/manage.ps1 -Install -Preview`, then
    `-Install -AllowDriverInstall` (later replaced by the helper, WP-07);
  - A3: waits up to 30 s for endpoints whose names start with `AudioRouter`
    (prototype names until WP-05; then the four VCAB-02 names) and fails if
    not all expected endpoints are present and `Status = OK`;
  - A14 (unless `-KeepInstalled`): uninstalls and compares with A1; fails on
    any leftover `oem*.inf` with provider `AudioRouter Project` or any
    `ROOT\AudioRouterVirtual` device;
  - writes `summary.json` (`{ checks: [{ id, passed, detail }], package }`).
- **Acceptance:** user runs it twice from checkpoint `02-test-signing-ready`
  with the WP-02 package; both pass; evidence copied to
  `docs/plans/active/evidence/<date>-virtual-cable-wp03.md` (text summary;
  no audio).
- **Expected first result:** the prototype creates its device only through
  the Software Device probe or `devcon`-style creation. If A3 fails because
  no device node exists, add a VM-only `-CreateRootDevice` step using
  `pnputil /add-driver <inf> /install` plus a root device creation (WDK
  `devcon install <inf> ROOT\AudioRouterVirtual`; `devcon.exe` is in the
  WDK tools folder). The product helper (WP-07) replaces this.
- **Rollback:** restore the VM checkpoint.
- **Status:** host implementation/checks done 2026-10-05; VM A1/A2/A3/A14
  acceptance pending (two checkpoint runs still required).

Implementation steps: separate pure guard and baseline comparison helpers;
reject host execution before package/files/inventory mutations; capture PnP,
driver store and actual default endpoint IDs; stage a verified test package
under the developer wrapper's allowed driver root; install with exact ownership
state, optional explicit root creation; bounded endpoint check; finally remove
only recorded package/device and compare baseline, retaining JSON failure
evidence. Host checks: guard matrix, comparison regressions, parser check and
real host refusal. VM checks: A1/A2/A3/A14 twice from the checkpoint (pending).
Rollback: revert scripts; in VM restore the checkpoint if compensation fails.

Evidence: Windows PowerShell `m03-driver-vm-guards.ps1` passed 16 checks
(identity/signing matrix, baseline deltas, parser, read-only MMDevice defaults).
Real `m03-driver-vm.ps1 -Package C:/does-not-exist` refused at VM identity before
boot query, inventory, output or package access; log
`target/driver-vm-host-refusal.log`. No install or VM result is claimed.

### Higher-precision decision (2026-10-05, user)

Host experiment: PCM32 `1073741889 / 2^31` cast to float32 and converted
back gives `1073741952` (63 LSB error). Float32 cannot satisfy VCAB-12's
exact PCM32 conversion and VCAB-21's ≤1 LSB round trip. User selected:
**keep PCM32 ≤1 LSB; redesign bridge and engine around higher precision**.
VCAB-21 is preserved; do not relax it. Before WP-05/06/09 implement revised
17 requirements: float64 transport/internal cable path, explicit precision
negotiation and bounded 256 KiB maximum payload; exact conversions and
no implicit narrowing into legacy float32 stages. Record remaining design
decisions/tests as each WP reaches that seam. Existing physical/VB-Cable
paths remain outside this driver change until explicitly migrated.

## WP-04 — Driver security fixes (17 §5.3)

- **Who:** agent on the host; user runs VM checks.
- **Files:** `drivers/audiorouter-virtual/Source/Inc/bridgeio.h`,
  `Source/Main/adapter.cpp`, `Source/Main/minwavertstream.{h,cpp}`;
  `tests/acceptance/m03-driver-build.ps1`; new
  `drivers/audiorouter-virtual/tests/` (user-mode unit tests for the pure
  helpers) and `tools/m03-bridge-fuzz/` (user-mode IOCTL fuzzer).
- **Steps:**
  1. Fix the double-fetch in `AudioRouterCopyBridgeBlock`: one volatile copy
     of the header into a local; validate and use only the local; single
     pass per sample (read, check finite, store). Add a comment explaining
     why.
  2. Object type check: pass `*MmSectionObjectType` to
     `ObReferenceObjectByHandle`; require mapped size == required size.
  3. Exact IOCTL buffer lengths.
  4. SDDL change to `D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)` plus
     ownership: store session ID and `FILE_OBJECT` at OPEN; check on
     HEARTBEAT/CLOSE; `STATUS_SHARING_VIOLATION` for a held lease;
     `STATUS_ACCESS_DENIED` for another session; zero the shared buffers and
     scratch state on release.
  5. Unit tests: make the pure functions in `bridgeio.h` compile in user mode
     behind a small shim header (`tests/km_shim.h` defining `NTSTATUS`,
     `STATUS_*`, `RtlCopyMemory`, `C_ASSERT`, SAL macros) so they can be
     tested with a normal C++ test executable on the host (built by
     `tests/build-tests.ps1` with MSVC). Cases: valid block; frames/channels
     at limits; payload mismatch; view too small; NaN/Inf; header changed
     between validation and copy (simulate by a callback that mutates a copy
     — the fixed code must not read the shared header twice, so assert the
     local copy is used); bus ID with embedded NUL; reserved fields.
  6. Fuzzer `tools/m03-bridge-fuzz`: opens `\\.\AudioRouterVirtualBridge`,
     sends random sizes/values for every IOCTL, races heartbeat/close from two
     threads, maps undersized sections, swaps section handles for other
     object types (event, file); runs for a set duration.
- **Tests:** host: unit tests pass, driver builds. VM (user): fuzzer 30 min
  with Driver Verifier on (`verifier /standard /driver audioroutervirtual.sys`)
  → no bug check; a second Windows user in the VM cannot take an active
  lease (A11).
- **Acceptance:** all above pass; a written review of every kernel function
  that touches user memory (list in the evidence file) by a fresh agent
  context (independent review, AGENTS.md).
- **Rollback:** revert the commits; the prototype is unshipped.
- **Status:** host implementation and build checks complete 2026-10-05;
  independent review has no remaining source finding; VM qualification pending.

WP-04 host follow-up: keep lease control serialization at PASSIVE_LEVEL; align
copy/publish direction guards with the capture-sink consumer and render-source
producer; return active lease generation with shape; separately track the
capture-sink reader's generation and clear its scratch/reset its sequence on
turnover. Retain and enforce a strictly increasing per-direction generation
high-water mark across close/expiry. The fuzzer's undersized-section case uses
a valid 2-channel request whose 1056-byte extent exceeds its 64-byte section.
Host checks: `build-tests.ps1` passed 49 checks and measured 0.122 us/block;
`m03-driver-build.ps1` passed for x64 and ARM64; docs validation passed 122
Markdown files/647 links; the host VM guard refused as intended. Fresh review
confirmed fixes to all source findings. ASan linking is unavailable because
`clang_rt.asan_static_runtime_thunk-x86_64.lib` is absent from the installed
MSVC toolchain. The microbenchmark is not kernel DPC evidence. VM Driver
Verifier/fuzz and A11 second-user checks remain pending and block WP completion.

Steps/validation: single-read header/payload copy with fail-closed finite checks;
user-mode MSVC shim tests including hostile mutation; typed section handles,
exact logical quantum size with bounded OS page-rounded mapping, serialized
control operations at PASSIVE_LEVEL for cross-direction section exclusivity,
file/session-owned leases and post-rundown scrubbing; callback direction
guards match endpoint roles; lease-generation changes scrub scratch and reset
sequence state even for same-shape replacement. VM-only fuzzer compiled,
including a real undersized section-object case. Host checks passed 45 unit
checks, x64 and ARM64 WDK acceptance. Initial fresh-review findings prompted
these fixes; follow-up review pending. Host microbenchmark is not kernel DPC
evidence. Rollback: revert WP-04 changes; no driver is loaded on the host.
VM Verifier/fuzz/second-user checks remain pending and block WP completion.

## WP-05 — Eight cables, registry configuration, formats, timing, sample cleanup (17 §5.1, §5.4, §5.5, VCAB-01/02/10–12)

WP-05 execution record (2026-10-05): objective is 16 stable Cable A–H
render/capture endpoints with a registry-selected first N enabled, exact
format conversion, bounded timing and sample-state cleanup. Requirements:
VCAB-01/02/10/11/12, VDEV-01/03/04/05, NFR-16. Use WP-00 proposed D2/D3/D4/D6
defaults unless the user changes them; D5 format family is recorded, with the
user's PCM32 ≤1 LSB decision requiring a float64 bridge/internal representation
and no silent narrowing. Prerequisites/evidence: Windows x64 and ARM64 WDK
build acceptance available; WP-03/04 VM load, endpoint, Verifier and audio
evidence remain pending and are not inferred from builds. Steps: (1) add pure
host-tested format/conversion and cable-list/registry-bound helpers; (2) define
the stable endpoint/bus/direction map and generate INF names/interfaces from
one canonical list; (3) add CableCount registry read with default/range clamp
and install only first N pairs; (4) replace sample topologies/formats and
remove sample-only paths; (5) integrate double-precision bridge conversion,
QPC position and period constraints where supported; (6) run unit tests, INF
validation and x64/ARM64 builds; fresh-review diff and preserve VM/audio gates.
Validation matrix: host helper tests cover all format edges and counts 1/2/8,
INF generator determinism and InfVerif, WDK builds both architectures; VM A3/
A9, 2→8→2 stable IDs, all rate/channel/format inventory, 1-hour QPC drift,
and sound-quality harness remain pending. Rollback: revert only WP-05 paths;
never install/load on the host. Evidence path:
`docs/plans/active/evidence/2026-10-05-virtual-cable-wp05.md`.

Host progress update (2026-10-05): canonical cable list/INF generation,
CableCount default/range read, first-N installation loops, 8-pair enum and
double bridge conversion helpers are implemented. The cable pairs now use
direct render/capture WaveRT tables for all 60 rate/channel/encoding
combinations and minimal speaker/line-in topologies without microphone-array
properties or inserted volume/mute processing. INF generator, 132 host
bridge/conversion/format checks, x64 and ARM64 WDK acceptance, and VM-only
fuzzer build pass. Host implementation remains non-releasable: VM gates are
still outstanding. The intermediate package remains prohibited from loading.
See the evidence record for exact commands and limitations.

Timing follow-up (2026-10-05): WaveRT notification cadence now stores and
compares packet intervals in 100 ns units derived from DMA bytes per second,
avoiding millisecond truncation for short periods while retaining timer
overshoot across ticks. The static acceptance guard now checks the high
resolution interval path. x64/ARM64 WDK acceptance and 132 portable bridge
checks passed; this is compile/host evidence only. The 1-hour VM QPC drift and
tone-continuity measurements remain pending.

Sample-path cleanup follow-up (2026-10-05): removed the synthetic capture tone,
its tone-generation utility and registry controls, and all per-stream SaveData
initialization, allocation, stop-wait, and DRM hooks. Shared SaveData worker
pool setup/teardown remains in common-device lifecycle code, which carries the
adapter's existing shared-state and Bluetooth HFP constraints; WaveRT streams
no longer reference or call it. x64/ARM64 WDK acceptance, 132 portable bridge
checks, source-reference scan, and docs link validation pass. VM period/drift
and audio-quality gates remain pending.

Scratch-state follow-up (2026-10-05): generation and shape changes invalidate
scratch through frame/offset/sequence bounds only. Removed the full 256 KiB
scratch zero from audio callbacks; each successful block read overwrites every
sample consumed, and each published block is completely rewritten before send.
Acceptance rejects callback-wide scratch clears. x64/ARM64 WDK acceptance,
132 portable bridge checks, and docs validation pass. Host WP-05 implementation
is ready for downstream host preparation; endpoint, one-hour drift and audio
quality VM gates remain explicitly pending under WP-03/04.

- **Who:** agent on the host; user runs VM checks.
- **Files:** `Source/Filters/minipairs.h`, new
  `Source/Filters/cablewavtable.h` and `cabletopo*.{h,cpp}` (replacing
  `speakerwavtable.h`, `micarraywavtable.h`, `micarray*` and
  `speakertop*` files), `Source/Inc/definitions.h`,
  `Source/Main/AudioRouterVirtual.inx`, `Source/Main/minwavertstream.cpp`
  (format conversion), project files (`*.vcxproj`), removal of
  `Source/Utilities/ToneGenerator.*` and `savedata.*`.
- **Steps:**
  1. Define the 16 `ENDPOINT_MINIPAIR`s (8 cables × render/capture) with a
     macro per cable index, and the endpoint → (bus, direction) table.
  1a. Read `CableCount` (HKR, default 2, range 1–8) at `StartDevice` and
     register only the first N cables. New `tools/m03-inf-gen` generates
     the 16 INF interface sections and strings from one list.
  1b. Read all 17 §5.5 values (names, period limits, lease and silence
     limits) with range checks and compiled defaults; set endpoint names
     from `Cables\<n>\Name` (verify the renaming mechanism; record which one
     works and keep endpoint IDs stable).
  2. Data ranges per 17 VCAB-11: 44.1/48/96 kHz; 1, 2, 4, 6, 8 channels with
     standard masks; float32, PCM 16, PCM 24-in-32, PCM 32. Default format
     48 kHz float32 stereo.
  3. Conversion per VCAB-12 (float copy bit-exact; integer exact; float →
     integer round-to-nearest + clamp, no dither); `__forceinline` per
     format; unit-test every conversion, including ±full scale, −0.0, NaN,
     Inf and denormals, in the WP-04 test project.
  3a. Timing per 17 §5.4: position from `KeQueryPerformanceCounter` with
     exact 64-bit arithmetic, high-resolution notification timer per stream,
     no timer when stopped, packet-size constraints for 128-frame periods.
  4. INF: generated interface sections, friendly names per VCAB-02, `HKR,,CableCount,0x00010001,2` in `.HW`, generic
     topology node types, remove `SWD\AudioRouterVirtual`, remove the WDF
     section if KMDF is not linked.
  5. Remove sample-only code and registry parameters.
- **Tests:** host build + `InfVerif /h`; VM: WP-03 script with the default
  count (2 cables → 4 endpoints with the exact names, A3); set `CableCount`
  to 8 in the VM registry and restart the device (Device Manager → disable/
  enable) → 16 endpoints; back to 2 → the Cable A/B endpoint IDs are
  unchanged; rename Cable B in the registry → new name shown, same
  endpoint ID; `m00-native-format-inventory.ps1` lists every rate ×
  channel × format; `IAudioClient3::GetSharedModeEnginePeriod` reports a
  minimum of 128 frames; a 1-hour stream shows no position drift against
  QPC (|error| < 1 frame).
- **Acceptance:** endpoint count follows `CableCount`; names come from the
  registry; IDs stable across count changes and renames; all formats open in
  shared mode; low-latency period available; uninstall clean.
- **Rollback:** revert.
- **Status:** host implementation and checks complete 2026-10-05; VM qualification pending under WP-03/04.

## WP-06 — Bridge protocol 1.1, counters, Rust client (17 §5.2)

- **Files:** `bridgeio.h`, `adapter.cpp`, `minwavertstream.cpp`;
  `crates/windows-audio/src/lib.rs` (`NativeBridgeOpenRequest`,
  `NativeBridgeControlClient`, constants), its tests.
- **Steps:**
  1. 16 lease slots indexed by `(bus 0–7, direction)`; streams use their
     endpoint's bus index (WP-05 table) when publishing/reading.
  2. Bus ID parsing (`cable-a` … `cable-h`; not-enabled cable →
     `STATUS_DEVICE_NOT_CONNECTED`), version negotiation, QUERY IOCTL with
     `AR_BRIDGE_DRIVER_INFO` (capabilities, supported rates, period limits).
  2a. Optional 64-byte open extension (flags + reserved, unknown bits →
     `STATUS_NOT_SUPPORTED`); `AR_BRIDGE_MAX_CHANNELS` = 8; 128-byte shared
     header with driver-written counters (17 §5.2); `SampleRateHz` taken
     from the stream, mismatch with the lease → `FormatMismatches` counter
     and silence, never a wrong-speed stream.
  3. Rust: `NativeBridgeControlClient::query()`, capability checks with
     graceful fallback, counter reader on the mapped header, bus ID in open requests,
     `STATUS_REVISION_MISMATCH` / `SHARING_VIOLATION` / `ACCESS_DENIED`
     mapped to distinct error variants; offset test against the C header
     (parse the `C_ASSERT` values or hard-code with a comment pointing to
     the header).
  4. A small CLI tool `tools/m03-bridge-tone` (Rust, Windows-only): writes a
     997 Hz and a 47 Hz tone into `(cable-b, CAPTURE_SINK)` and reads
     `(cable-a, RENDER_SOURCE)` into a WAV; used by VM tests before the
     engine integration exists.
- **Tests:** `cargo test -p audiorouter-windows-audio` on the host (portable
  parts); VM: `m03-bridge-tone` → a recording app (e.g. `ffmpeg -f dshow`
  or the AudioRouter WASAPI probe) on `Cable B Output` captures the tone
  without glitches; tone played into `Cable A Input` with the probe arrives
  in the WAV (A4 precursor). Two cables at once, no crosstalk (tone A ≠ tone
  B).
- **Acceptance:** both directions on both cables; float32 tone in →
  identical samples out (bit-exact, VCAB-20 precursor); crosstalk below
  −140 dBFS; counters stay zero in a clean 10-minute run and increase when
  the tool deliberately stalls; 8-channel and 96 kHz leases work; version
  mismatch and unknown-flag paths tested.
- **Rollback:** revert; protocol 1.0 users do not exist outside the repo.
- **Status:** in progress 2026-10-05 (host ABI/Rust work; VM integration remains gated by WP-03/04).

WP-06 execution record (2026-10-05): objective is one safe, versioned 16-lease
bridge contract with float64 payload support and a Rust client that preserves
VCAB-21 PCM32 precision. Requirements: VCAB-10/12/20/21/26, SEC-08, NFR-16.
Prerequisites: WP-05 host work complete; WP-03/04 VM evidence remains pending.
This authorizes portable and WDK host preparation only; no driver load or
quality gate is inferred. Tasks: (1) inspect the current C ABI, slot selection,
per-lease sample-rate and generation rules; (2) extend protocol to 16 stable
slots with cable identity, precision negotiation, counters and exact QUERY/
open IOCTL validation; (3) mirror layout/capabilities/errors in Rust, with
portable offset, negotiation, bounds and conversion tests; (4) run Rust tests,
x64/ARM64 WDK acceptance and security/static checks, update evidence, inspect
the exact diff and commit named paths. Validation matrix: header/layout parity,
unknown/old protocol rejection, all 16 slot identities, PCM32 precision,
malformed/racing bridge payload regression, Cargo package tests and both WDK
builds; VM multi-cable/format/crosstalk/audio tests remain pending. Risks:
protocol header-size drift, sample-rate mismatch, and callback contention.
Rollback: revert only WP-06 paths; keep the intermediate package unloaded.

## WP-07 — Elevated driver helper (17 §6)

- **Files:** new `crates/driver-helper/` (`Cargo.toml`, `src/main.rs`,
  `build.rs` embedding the manifest with `requireAdministrator`), workspace
  `Cargo.toml` member list.
- **Steps:** implement `status`, `install`, `update`, `repair`, `set-cables`, `configure`, `remove`
  exactly as 17 §6, including `WinVerifyTrust` (9.1), state and log files,
  exit codes, rollback, default-device snapshot (`IMMDeviceEnumerator::
  GetDefaultAudioEndpoint` for eRender/eCapture × eConsole/eCommunications;
  read only).
- **Tests:** portable unit tests for argument validation, state file
  read/write (atomic replace), result JSON schema; VM (user, debug build
  with `AUDIOROUTER_ALLOW_TEST_DRIVER=1`): install → status → update (bump
  version, same device instance ID and same four endpoint IDs) → repair
  (after deleting the device in Device Manager) → `set-cables --count 8`
  (16 endpoints) → `--count 3` (A–C keep their IDs, D–H gone) →
  `configure --name 2="Discord" --param MinPeriodFrames=256` (name shown,
  IDs unchanged, period limit applied; out-of-range values refused) →
  remove → baseline match.
  Release build refuses the test-signed package (exit 2).
- **Acceptance:** all VM cases pass twice from the checkpoint; WP-03 script
  switched to use the helper; `manage.ps1` and the helper produce the same
  end state.
- **Rollback:** remove the crate.
- **Status:** not started.

## WP-08 — Status detection, API, CLI, shell commands (17 §7.1–7.2)

- **Files:** `crates/windows-audio/src/` (new `virtual_cable.rs`),
  `crates/control/src/lib.rs` (method registry `API_METHODS`, schema,
  `virtualDevices.*` deprecation), `crates/domain/src/lib.rs` (buses
  `cable-a` … `cable-h`, enabled set from status), `crates/cli/src/lib.rs`
  (`virtual-cable status|install|set-count|remove`), `src-tauri/src/main.rs` (shell
  commands that start the helper with `ShellExecuteExW` `runas` and wait),
  `src-tauri/src/http_api.rs` (status route only), `contracts/src/index.ts`,
  `docs/operations/api-reference.md`, `docs/spec/10-api.md`.
- **Tests:** control unit tests for status mapping (fake inventory),
  deprecation responses, permission scopes (MCP/HTTP cannot install);
  shell test that the helper path resolves next to the shell executable and
  never from `PATH`; `fresh_install_shell` still passes (validated lesson
  2026-10-01).
- **Acceptance:** `audiorouter-cli virtual-cable status --json` in the VM
  reports `installed` with the enabled cables' endpoint IDs after WP-07 install and
  `notInstalled` after remove.
- **Status:** not started.

## WP-09 — Engine: Virtual Render Source / Virtual Capture Sink (17 §7.3)

- **Files:** `crates/engine/src/lib.rs`, `crates/control/src/lib.rs`,
  `crates/engine/tests/tool_combinations.rs`, live tests in
  `crates/control` (guarded like the existing `live_*` tests).
- **Tests:**
  - portable: compile/plan tests with fake bridges (both node kinds, with
    Mixer, Duck, bypass, connected Mixers);
  - VM: `live_backend_service_keeps_a_routed_tone_continuous` adapted for
    `Cable A Input → graph → Cable B Output` with reference capture, zero
    glitches at 997 Hz and 47 Hz for 10 minutes (lesson 2026-09-26);
  - VM: kill the backend during a tone → `Cable B Output` silent within
    500 ms (A7); restart → no old audio.
  - portable: a cable-only route (all ends AudioRouter cables, same rate)
    compiles **without** drift correction or resampler stages; a route with
    a physical device compiles with them (17 §7.3);
  - portable: resampler 44.1/96 ↔ 48 kHz against VCAB-22 (sweep, 997 Hz
    THD+N, aliasing) in the deterministic-audio harness;
  - VM: bit-exact pass-through Cable A Input → Cable A Output, float32 noise,
    48 kHz stereo and 8 channels (VCAB-20);
  - VM: counters visible in `system.diagnostics`.
- **Acceptance:** above pass; no new allocation or lock in the audio path
  (review); same-rate cable routes never touch the resampler.
- **Status:** not started.

## WP-09b — Sound-quality and performance harness (17 §11)

- **Who:** agent on the host; user runs it in the VM now and on the main PC
  in stage C.
- **Files:** new `tools/m03-cable-quality/` (Rust, Windows-only, WASAPI
  shared mode), new `tests/acceptance/m03-cable-quality.ps1`, docs in
  `docs/operations/virtual-cable-testing.md` (A16 and stage C).
- **Steps:**
  1. Signals and analysis exactly as 17 §11.2 (seeded noise for bit-exact
     compare with cross-correlation alignment; silence; 997 Hz THD+N; log
     sweep; 47/997 Hz continuity; impulse train; per-cable tones for
     crosstalk).
  2. Cases: cable-only pass-through at 44.1/48/96 kHz × 1/2/8 channels ×
     float/PCM16/PCM24/PCM32; cable → physical output → physical input
     loopback (drift path, needs a loopback cable on the main PC); 8 cables
     at once; low-latency and default periods; VB-Cable comparison on the
     same machine.
  3. Output `metrics.json` per case with the VCAB target, measured value
     and pass/fail; a summary table; WAVs under `target/cable-quality/`
     (git-ignored). Never record a microphone or private audio.
  4. CPU/DPC/memory capture script using `wpr` and `Get-Process`/
     Performance Monitor counters (17 §11.2), on request only.
- **Tests:** the harness's analysis code is unit-tested with synthetic
  signals of known THD+N, delay and crosstalk (portable, runs on any OS).
- **Acceptance:** VM run of all cable-only cases recorded (indicative);
  every metric has a value; failures are listed with numbers, not hidden.
- **Status:** not started.

## WP-10 — UI (17 §7.4)

- **Files:** `ui/src/FirstRunGuide.tsx`, new `ui/src/VirtualCablePanel.tsx`
  (+ test), `ui/src/library.ts`, templates, `ui/src/backend.ts`,
  `ui/src/host.ts` (shell commands), `ui/e2e/virtual-cable.pw.ts`.
- **Also:** per-cable rename, endpoint format display with a hint when not
  48 kHz float, live underrun/overrun counters (reserved slots, UI-17), and
  the quality guidance from VCAB-29 (Windows communications ducking, app
  noise suppression).
- **Tests:** unit tests for every status state, button enablement, rename
  validation and the Number of cables select (lowering lists affected
  sessions first); Edge
  test in three themes with layout-shift sampling (lesson 2026-10-03), host
  bridge test for the shell command wiring (lesson 2026-10-04); screenshots
  written to `testInfo.outputPath` (lesson 2026-10-03).
- **Acceptance:** UI conventions in AGENTS.md; no layout shift while status
  changes.
- **Status:** not started.

## WP-11 — Installer, update and uninstall (17 §7.5)

- **Files:** `src-tauri/tauri.release.conf.json`, new
  `src-tauri/installer-hooks.nsh`, `tools/release/prepare-artifacts.ps1`,
  `tools/release/verify-artifacts.ps1`, `tests/acceptance/m08-installer-smoke.ps1`,
  `docs/operations/distribution.md`.
- **Tests:** installer smoke in the VM: install app (no UAC), install cable
  from Setup (one UAC), upgrade app with newer driver → Update shown → one
  UAC → endpoint IDs unchanged; uninstall app with "keep" and with
  "remove"; verify-artifacts checks driver hashes.
- **Status:** not started.

## WP-12 — Stage A full qualification

- Run A1–A16 of the [testing procedure](../../operations/virtual-cable-testing.md)
  on one build, in the VM (A1–A14 with 2 cables, A15 with all 8, A16 the
  WP-09b quality harness). VM latency/CPU numbers are indicative; bit-exact,
  crosstalk, conversion and glitch results must already pass. Record evidence in
  `docs/plans/active/evidence/<date>-virtual-cable-stage-a.md`.
- **Gate:** all pass → ask the user for the go-ahead to buy the EV
  certificate (WP-13).
- **Status:** not started.

## WP-13 — Microsoft signing and stage B (user, costs money)

- Follow [virtual cable signing](../../operations/virtual-cable-signing.md).
  Agent prepares the CAB (`tools/release/make-driver-cab.ps1`, new) and
  verifies the returned package; user buys, registers and submits.
- Stage B in a clean VM with Secure Boot and Memory Integrity on.
- **Status:** not started; needs user go-ahead.

## WP-14 — Stage C on the main PC

- Testing procedure stage C, one week of daily use, Siege/BattlEye check.
- **Binding performance and sound-quality measurements** (VCAB-20–30) with
  the WP-09b harness on the main PC, including the physical loopback drift
  case, 1-hour 8-cable glitch run under CPU load, WPR DPC capture and the
  VB-Cable latency comparison. A missed target needs a fix or a recorded
  user decision with the numbers (DEC-14 method) before WP-15.
- **Status:** not started.

## WP-15 — Beta, release, documentation

- Stage D with 2–3 testers; then release with M08 gates.
- Update: `README.md` (install section, "Why AudioRouter"),
  `docs/operations/quickstart.md`, `privacy-permissions.md`,
  `release-notes.md`, `distribution.md`, `examples/setups/`.
- Archive this plan under `docs/plans/archived/` with the date.
- **Status:** not started.

## Validation matrix

| Requirement | WP | Evidence |
| --- | --- | --- |
| VCAB-01/02, VDEV-01 (deviation) | WP-05, WP-07, WP-12 | VM endpoint list at 2 and 8 cables; A15 |
| VCAB-03/04, VDEV-04/12 | WP-06, WP-09 | Tone, silence, crash tests |
| VCAB-05, VDEV-03 | WP-07, WP-12 | Reboot and update keep endpoint IDs |
| VCAB-07 | WP-07, WP-14 | Default-device snapshot before/after |
| VCAB-10/11 | WP-05 | Format inventory |
| SEC-08, VDEV-07 | WP-04 | Unit tests, fuzzer, verifier, second user |
| VDEV-08, NFR-16 | WP-07, WP-08 | Only the helper elevates; backend standard user |
| VDEV-09 | WP-13 | Microsoft signer, stage B |
| VDEV-09 development, SEC-08 packaging, ENG-05 | WP-02 | [Versioned test package](evidence/2026-10-05-virtual-cable-wp02.md); production gate remains WP-13 |
| VDEV-11 | WP-12, WP-14 | Discord, OBS |
| NFR-02 | WP-12 | Impulse loopback p95 ≤ 160 ms, compared with VB-Cable |
| VCAB-20/21/26 (bit-exact, conversions, isolation) | WP-05, WP-06, WP-09, WP-09b | Harness: noise compare, LSB error, crosstalk |
| VCAB-22/23 (resampling, drift) | WP-09, WP-09b, WP-14 | Sweep, THD+N, aliasing; physical loopback 1 h |
| VCAB-24/30 (no glitches, under load) | WP-09b, WP-12, WP-14 | 8 cables, 1 h, continuity harness, counters zero |
| VCAB-25 (latency) | WP-05, WP-09b, WP-14 | Impulse p95/jitter, low-latency and default periods, vs VB-Cable |
| VCAB-27/28 (CPU, memory) | WP-14 | WPR DPC, backend CPU, non-paged pool 24 h |
| VCAB-29 (no hidden processing) | WP-05, WP-10 | No APO, volume 100 %, UI guidance |
| DIST-01/02/06 | WP-11 | Installer smoke, artifact verification |

## Risks

See [17 §10](../../spec/17-virtual-cable.md#10-known-risks). Plan-specific:
the WDK/SDK version mismatch on the host is the most common build blocker —
install the WDK that matches the SDK version shown in Visual Studio
Installer.

## Rollback

Every WP is a separate commit set. Until WP-15, nothing reaches users: the
published app keeps the VB-Cable workflow. Reverting DEC-18 restores DEC-16.

## Next action

WP-06 host work: implement the 16-directional-lease protocol and Rust float64
client; WP-05 and WP-03/04 VM endpoint, verifier and audio-quality evidence
remain pending. Do not load the intermediate package or call a host build a
runtime or quality pass.
