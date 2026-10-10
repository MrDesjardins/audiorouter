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

## Active repair — 2026-10-09 render publication review

User confirmed playback was already running and requested source analysis and
an owning-layer repair before any further VM retry. This supersedes the final
"pause Media Player" workaround below. Already-running playback is supported.
Scope: VCAB-20/24/25/27/28, VDEV-12, SEC-08; Stage A tone continuity.

Source finding: `ReadBytes` can finish several bridge quanta in one locked
callback; the publisher replaces the sole shared slot each time. An armed
reader cannot acknowledge between those writes. The 18 ms first-read gap
is consistent with this defect; existing archives do not identify the exact
callback schedule. A test that waits for each reader acknowledgement cannot
exercise it. Also review time-unit guards and timer/resource lifetime.

Ordered repair: retain completed render quanta in bounded private storage,
publish only after an exact acknowledgement, fence publication by expected
lease generation, and service transport independently of notification cadence.
Reuse existing stream storage, preserve protocol 1.1 and honest loss counters;
no acceptance threshold change. Handle queue capacity, lease replacement,
pause/EOS/STOP and DMA catch-up explicitly. Document the internal buffering
decision in spec 17. Fresh-context kernel review is required by WP-04.

Verification after the code repair: deterministic production-helper cases for
back-to-back completion without intervening reader execution, exact samples,
bounded overflow, hostile acknowledgements and generation turnover; host-only
WDK build and source guards. No driver load or new VM run in this task.
Earlier capture loss during a 65 ms guest scheduling gap remains a separate
open issue; this repair cannot claim arbitrary scheduler stalls are lossless.
Rollback: revert this repair and retain the existing VM snapshot and evidence;
never replace the installed package automatically.

Implemented and reviewed: private FIFO in existing storage, exact ack flow
control, expected-generation publication/counters, initial OPEN pointer-last,
per-tick render service, bounded DMA catch-up, consistent clock units and safe
timer teardown. The second review's EoS ordering finding is also repaired.
Host FIFO suite passes 447 checks; x64/ARM64 WDK/source acceptance and formatting/
Clippy pass. ASan is unavailable (missing runtime library); Jev returned a
network error. See the [full review and validation record](evidence/2026-10-09-m03-render-publication-review.md).
Next action: commit the reviewed source and prepare a single candidate.
Runtime/latency/long-run gates remain pending; no further user test requested.

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
  2026-10-06 finding: the development PC runs **Windows 11 Home** (EditionID
  `Core`, build 26300, HVCI/VBS on), which has no Hyper-V. The VM is set up
  with **VirtualBox** through Windows Hypervisor Platform instead, following
  the [VM guide](../../operations/virtual-cable-vm-guide.md); snapshots
  `01-clean-windows` and `02-test-signing-ready` replace the Hyper-V
  checkpoints. `tools/vm/prepare-vm-share.ps1` builds every VM file (ran end
  to end on this host into a scratch folder: package, integrity checks,
  static-CRT tools, fuzzer, scripts, manifest) and `tools/vm/vm-checks.ps1`
  runs each session step in the VM.

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

### VCAB-02 endpoint naming repair (2026-10-08, user authorized)

- **Objective:** make Windows expose the four enabled endpoints with cable-
  and direction-specific names so the helper can recognize Cable A/B and
  users can select the devices as specified.
- **Requirements:** VCAB-01/02, VDEV-01/03; Stage A A2/A3/A14.
- **Prerequisites/evidence:** clean VM after successful helper removal;
  repaired x64 package starts without a bugcheck. Direct VM inventory showed
  two render endpoints named `Speakers (AudioRouter Virtual Audio Device)`
  and two capture endpoints named `Line (AudioRouter Virtual Audio Device)`.
  The INF's per-interface names were not reflected. See the [VM failure and
  naming evidence](evidence/2026-10-07-virtual-cable-ks-enumeration-crash.md).
- **Decision:** preserve the VCAB-02 endpoint names and strict helper
  classification. The interface friendly-name attempt still produced
  `Speakers (AudioRouter Virtual Cable)` and `Line (AudioRouter Virtual
  Cable)`, two each. Per the Windows audio naming documentation, endpoint
  names come from bridge-pin categories/names; speaker form-factor names are
  fixed. Use unique custom pin categories, registered in the device software
  key, for each cable and direction. This changes endpoint form-factor labels
  from generic Speakers/Line to the specified virtual cable names while
  preserving render/capture data flow and strict helper classification.
- **Ordered work:** (1) add 16 deterministic pin-category GUIDs to the cable
  manifest and generate matching INF registrations and C declarations; (2)
  assign a per-cable topology descriptor/category to each bridge pin; (3) add
  build guards for generated GUID/string parity and descriptor wiring; (4)
  build, sign and stage an x64 test package; (5) have the user run a focused
  inventory, then two clean-checkpoint smoke runs only after the four names
  match.
- **Validation matrix:** host driver build and table checks; acceptance INF/
  property guards; VM `Get-PnpDevice` names and helper install/remove; then
  A1/A2/A3/A14 twice. No host driver load. VM tone/Verifier remain blocked
  until endpoint naming and both smoke runs pass.
- **Risk:** custom pin categories may use a generic Windows icon/form factor.
  Confirm endpoint creation and direction in the VM, and preserve unique
  exact cable/direction names and endpoint IDs across package updates.
- **Rollback:** restore the previous test package in the VM checkpoint and
  revert only the naming property and its focused acceptance checks.
- **Status:** initial naming and install/remove gate passed twice on
  2026-10-08 with clean package `5f61df3c`. See the latest two-run evidence
  below. Rename, persistence, formats, audio and Verifier remain pending.

### Render-name follow-up (2026-10-08, VM evidence)

- **Finding:** the retried package created four healthy endpoint children.
  Both capture endpoints expose their cable-specific names, while both render
  endpoints are `Speakers (AudioRouter Virtual Cable)`. The helper therefore
  sees only two identifiable endpoints. Its remove command succeeded and the
  before/after snapshots match. Evidence:
  `C:\VMs\ar-share\evidence-20261008-172141.zip`, especially
  `20261008-172014-smoke\runner\endpoint-names-on-install-failure.json`,
  `helper-remove.json`, and the baseline snapshots.
- **Correction:** the prior assumption that a custom category would replace
  the render speaker label was disproved. Microsoft documents the speaker
  endpoint label as fixed. The render bridge currently uses a custom category
  yet Windows still exposes the fixed `Speakers` name.
- **Ordered work:** use `KSNODETYPE_ANALOG_CONNECTOR` for the render bridge
  and a per-cable bridge-pin `Name` GUID registered in the device software
  key. Keep capture's observed per-cable category naming and the helper's
  strict flow/name checks. Add source/INF generation guards, rebuild and
  stage a fresh package, then use the VM to verify all four names and clean
  removal before repeating A1/A2/A3/A14.
- **Validation:** driver build, INF generation, package signature and hash
  checks, plus focused driver and VM-guard checks. The render-name behavior
  remains unqualified until observed in a new VM installation.
- **Rollback:** restore `02-test-signing-ready` and retry the prior reviewed
  package if this test package fails. Do not load it on the host.

#### Review before VM retry (2026-10-08)

Review scope: the pending category package, native helper classification,
failure evidence, and copy/paste retry. Requirements VCAB-02, VDEV-03,
Stage A A2/A3/A14. Found a reproducible mismatch: the Rust helper rejects
`AudioRouter Cable A Input (AudioRouter Virtual Cable)` while the VM guard
accepts it. Also, failure evidence can mask the original install exception,
and interactive retry commands do not form one terminating script block.
Ordered repair: align strict Rust/PowerShell name classification with shared
fixtures; preserve the original exception when collecting failure evidence;
wrap the retry and check package hashes before smoke; rebuild a fresh share.
Validate the reproduced rejection before repair, focused helper regressions,
VM guards, driver/INF checks, package manifest hashes, docs and Jev.
Rollback: restore the clean VM snapshot and keep the earlier share untouched.
Runtime naming, rename persistence and custom-category app compatibility
remain VM gates; host checks do not establish them.
Additional finding: native rename writes `PKEY_Device_DeviceDesc`, the very
category description supplying the new cable identity. Preserve that property
and update only the display friendly name; classify from the unchanged device
description, check actual render/capture direction, and regress renamed
metadata separately from display parsing. This stays within VCAB-02/VDEV-03.
The Windows display inventory also includes historical/disabled endpoints
despite its function name. Filter helper results by active MMDevice state and
actual data flow; regress stale, missing-state and wrong-flow records.
Review repairs and host checks completed: helper 28 tests, native Clippy,
formatting, VM guards 274, driver checks 215, WDK/INF/catalog build, generator,
docs, package integrity 33 and all 29 staged manifest hashes. The required
Jev source review is incomplete: network access failed, and automatic review
rejected exporting the source diff; user approval was requested. Fresh share:
`C:\VMs\ar-share\repair-20261008-reviewed`. See the linked failure evidence
for exact package hashes, limitations and failed attempts. Next VM task is
Session 1 via `retry-smoke.ps1`; two clean runs remain required.

### Crash follow-up (2026-10-07, user authorized repair and continuation)

The user's Stage A `smoke` run installed the test-signed driver, found zero
AudioRouter endpoints, then Windows bugchecked with `SYSTEM_SERVICE_EXCEPTION`
0x3B / access violation. The copied mini dump
`C:\VMs\ar-share\100726-6484-01.dmp` places the fault in `ks.sys` while it
walks the driver's `CableStreamDataRanges` pointer list. AudioRouter's ten
`KSDATARANGE_AUDIO` values all set `KSDATARANGE_ATTRIBUTES`, but the list only
contained one `PinDataRangeAttributeList` pointer after all ten ranges. The
Microsoft Sysvad format table interleaves that attribute-list pointer after
each flagged range. This was a real descriptor-table defect and was corrected,
but the second replay bugchecked with the corrected 20-entry table present;
the defect alone does not explain the crash.

- Requirements/scenarios: VCAB-02 (endpoint enumeration), VCAB-10/11 (format
  enumeration), VDEV-09 (test package); Stage A A2/A3 and crash-free install.
- Prerequisites: the dump and test-signed driver from the smoke run; current
  Windows WDK; VM snapshot `02-test-signing-ready`. No host driver load.
- Ordered work: (1) match dump symbols to the local build and compare the
  KS range-list convention with Microsoft's Sysvad sample; (2) interleave
  one attribute-list pointer after each attributed range and assert the
  expected pointer-list shape in host acceptance; (3) build and package a new
  test-signed x64 driver; (4) prepare a fresh VM share and rerun preflight,
  then smoke from the clean test-signing snapshot; (5) only after A2/A3 pass,
  continue the next authorized VM session.
- Validation: `drivers/audiorouter-virtual/tests/build-tests.ps1`,
  `tests/acceptance/m03-driver-build.ps1 -Platform x64`,
  `tools/vm/prepare-vm-share.ps1`, and in-VM `vm-checks.ps1 -Step smoke`.
  The final VM install must enumerate all four endpoints and produce no new
  bugcheck. Existing failed-run evidence is retained.
- Rollback: restore `02-test-signing-ready` and use the previous package only
  for reproducing the recorded failure; revert the pointer-table/test changes
  if the corrected table fails static checks. Never load the test driver on
  the host.
- Status: blocked on host diagnosis after the 2026-10-07 VM replay; see
  the outcome below and linked evidence.

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
- **Status:** host implementation/checks done 2026-10-05; VM run 1 on
  2026-10-07 installed the package, found no endpoints and bugchecked during
  KS format enumeration. The first table repair added the expected
  per-format attribute pointers and passed host build/guard checks, but the
  2026-10-07 replay from the test-signing checkpoint bugchecked again as
  Windows started `ROOT\MEDIA\0000`. The host shared package contains the
  20-entry table, but the second dump's stack passes a count of 11 at that
  table and an earlier runner summary reports the pre-repair build time. The
  second runner crashed before writing its package summary, so the VM-loaded
  package is unconfirmed and may be stale. First verify the VM package's
  manifest and SYS hash; do not rerun smoke until the exact repaired image is
  confirmed. Two clean checkpoint runs are still required.

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
`target/driver-vm-host-refusal.log`. The first attended smoke result and the
2026-10-07 crash repair are recorded in
[crash follow-up evidence](evidence/2026-10-07-virtual-cable-ks-enumeration-crash.md).

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
132 portable bridge checks, and docs validation pass. Registry-controlled
friendly names and Min/DefaultPeriodFrames remain unimplemented; WP-05 is not
closed. Endpoint, one-hour drift and audio-quality VM gates remain pending.

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
- **Status:** host work complete 2026-10-06 (endpoints, CableCount, formats, conversion, timing, cleanup; registry `MinPeriodFrames`/`DefaultPeriodFrames`/`MaxLeaseMs`; low-latency packet constraints on every cable wave interface). Decisions recorded in 17 §5.5: names are applied by the helper (endpoint description), the default period is reported only, stale-silence holds by construction. VM qualification (endpoint names, 60-format inventory, ≤128-frame period, 2→8→2 IDs) pending; `m03_cable_inventory` measures it.

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
- **Status:** host work complete 2026-10-05 (stable bus slots, float64 transport, OPEN extension, QUERY, 128-byte header with stream counters, Rust client/query/error mapping, 8-channel driver path, VM tone tool). Open: all VM acceptance below; VM integration remains gated by WP-03/04. The tone tool lives at `crates/windows-audio/examples/m03_bridge_tone.rs` instead of `tools/m03-bridge-tone` (an example of the client crate needs no new workspace crate or lockfile entry).

WP-06 execution record (2026-10-05): objective is one safe, versioned 16-lease
bridge contract with float64 payload support and a Rust client that preserves
VCAB-21 PCM32 precision. Requirements: VCAB-10/12/20/21/26, SEC-08, NFR-16.
Prerequisites: WP-05 core tables, conversion, timing and scratch-state host
work are implemented; registry name/period configuration and WP-03/04 VM
evidence remain pending. This downstream protocol slice is being prepared
while those explicit WP-05 gaps stay open; no driver load or
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

WP-06 host progress update (2026-10-05): introduced testable exact `cable-a`
through `cable-h` parsing and pure `(bus, direction) -> slot` mapping across
16 unique slots; OPEN rejects unknown and disabled IDs. WaveRT reads/publishes
using the endpoint's stable cable index and clears stale capture state on a
lease sample-rate mismatch. The Windows Rust control encoder now emits driver
protocol 1.1 independently of the internal AudioBridge protocol 1.0 and
rejects non-cable IDs. C host tests cover all buses/slots; Rust ABI test checks
version, bus bytes and direction. x64/ARM64 WDK builds pass. Remaining WP-06:
float64 mapped-region/session support now carries double payloads and exposes
`write_f64`/`read_into_f64`; the legacy f32 adapter widens values in the wire
mapping. The Rust regression round-trips PCM32 value 1,073,741,889 exactly.
Remaining WP-06: negotiated open extension and QUERY/capability/counter
contracts. No claim of end-to-end PCM32 precision until the WP-09 float64
engine path is complete.

WP-06 negotiation/counters update (2026-10-05, host-only): OPEN now requires
the 64-byte extension with `FLOAT64` (prefix-only OPEN, unknown flags and
reserved words → `STATUS_NOT_SUPPORTED`); QUERY returns `AR_BRIDGE_DRIVER_INFO`
with only implemented capabilities and the build-stamped version; the shared
header is 128 bytes with driver-written counters, `SampleBytes` and the
consumer's `ReaderSequence`. Decision recorded in 17 §5.2 "As implemented":
the first driver has float64 transport only, so no float32 path exists in the
kernel. Two defects fixed: Rust sent `METHOD_NEITHER` IOCTL codes the driver
never dispatches, and a rate-mismatched capture-sink lease would still have
played at the wrong speed. Checks: 206 host bridge checks, x64/ARM64 WDK
acceptance, `cargo test -p audiorouter-windows-audio` 116 passed. Details:
[WP-06 evidence](evidence/2026-10-05-virtual-cable-wp06.md). Not VM, endpoint
or audio evidence.

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
- **Status:** host implementation complete 2026-10-06 (`crates/driver-helper`,
  25 unit tests with a fake platform; release binary embeds
  `requireAdministrator`; never run on the host). VM acceptance pending:
  `vm-checks.ps1` sessions 1–3 exercise install/idempotent install/status/
  set-cables/configure/remove, and the WP-03 runner gained `-Helper`. Not yet
  exercised: `update` with a bumped version and `repair` after deleting the
  device (add to session 3 when a second package version is built).

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

**VM Session 2 (user action, two smoke runs verified):** continue in the
current clean VM using package `repair-20261008-render-enabled`. Run the
Session 2 preflight/install/status block in the
[VM guide](../../operations/virtual-cable-vm-guide.md), then inspect the
format/period inventory before tone or Verifier. The second smoke removed
the driver and restored the baseline; Session 2 installs and keeps it.

After two clean A1–A3/A14 runs show the four Cable A/B Input/Output names,
continue sessions 2–5: WP-04 (Verifier + fuzz, second user), WP-05 (60
formats, ≤128-frame period, 2→8→2 IDs), WP-06 (tone both ways, counters,
8 ch/96 kHz) and WP-07 (helper install/status/set-cables/configure/remove).

The 2026-10-07 failure and repaired-package replay are recorded in
[the crash follow-up evidence](evidence/2026-10-07-virtual-cable-ks-enumeration-crash.md).
The repaired package identity matched in the VM and Windows completed the
device start without a bugcheck, but the helper still found zero of four
endpoints. A direct diagnostic inventory showed the four healthy devices were
named `Speakers (AudioRouter Virtual Audio Device)` (render) and `Line
(AudioRouter Virtual Audio Device)` (capture), twice each. Those names do not
match the required `AudioRouter Cable A/B Input/Output` names, so the helper
correctly failed its strict name classification. The helper's remove command
returned 0 and removed `oem5.inf`; the VM is clean. The INF's per-cable
interface names are not reflected in the MMDevice friendly names. Microsoft
documents endpoint naming through KS bridge-pin names/categories and the
audio adapter's interface friendly name; speaker endpoints use the fixed
`Speakers` label ([friendly names](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/friendly-names-for-audio-endpoint-devices),
[endpoint builder](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-endpoint-builder-algorithm)).
Two interface-name retries failed. The diagnostic retry captured four healthy
endpoint children, named `Speakers (AudioRouter Virtual Cable)` twice and
`Line (AudioRouter Virtual Cable)` twice. This confirms the missing cable
identity is in the bridge-pin naming path. Full evidence is in the
[crash and endpoint-naming record](evidence/2026-10-07-virtual-cable-ks-enumeration-crash.md).
Do not proceed to tone or Verifier until the required endpoint names appear.
Two clean smoke runs are still required before tone or Verifier sessions.

The first revised package (`repair-20261008-reviewed`) was copied and verified
in the VM. Preflight passed 15/15 checks; smoke found two named endpoints,
both captures, while render endpoints remained `Speakers (AudioRouter Virtual
Cable)`. Cleanup and collection passed; see the 2026-10-08 evidence above.
The code now uses `KSNODETYPE_ANALOG_CONNECTOR` plus per-cable bridge-pin
`Name` GUIDs on render and keeps custom categories on capture. INF generation
and the 274-check VM guard suite pass.

On 2026-10-08, Visual Studio repair completed with exit code 0, but an
ordinary build still failed with `LNK1101: incorrect MSPDB140.DLL version`.
Repeating the documented process-local tool selection (`PreferredToolArchitecture=x64`
and the matching 14.51.36256 Hostx64 linker directory first in `PATH`) made
the x64 WDK build acceptance pass. The first share-preparation attempt then
stopped at test-certificate creation with access denied. An elevated
sign-only step reused certificate thumbprint
`FF6876FBE50A74DC0B69077C11DC28A1A9FAAD40`; no host trust change was made.
The resulting driver package passed 33 integrity checks. Static-CRT helper,
tone/inventory tools and fuzzer were staged; the complete share passed all 29
manifest hash checks and is at
`C:\VMs\ar-share\repair-20261008-render-pin-name`. The SYS SHA-256 is
`C6931B62DDFB6317A75D17D7DC7213838024EAC24A12CC801A8B8A7154DBE6BD`.

The host bugcheck dump `C:\Windows\Minidump\100826-29328-01.dmp` was copied
to the shared folder (SHA-256
`ADE420AF6A2D0E275E5E4A98CDE28C11E8C4D89FC6302DBA18F28D10AE799A65`) and
analyzed with matching Microsoft symbols. It records bugcheck `0x3B`
(`0xC0000005`) at 2026-10-08 17:48:10, in `explorer.exe` waiting through
`nt!KeWaitForSingleObject+0x369` on Windows 11 kernel 10.0.26100.9549. The
dump contains no AudioRouter module, so this candidate driver was not loaded
on the host. The fault occurred about ten seconds after the candidate SYS
was built; the dump and call stack do not establish a cause. Blackbox PnP's
last record is `STORAGE\VolumeSnapshot\HarddiskVolumeSnapshot2`, problem
code 24, with no PnP event in progress; this is not evidence of causation.
The host crash remains unexplained. No host driver was installed or loaded.
Jev still awaits approval for its source export.

The 2026-10-08 `repair-20261008-render-pin-name` retry passed all 15 preflight
checks, installed and started the driver, then exposed two endpoints (Cable A/B
Output) and no render endpoints. The captured endpoint inventory confirms that
the unique pin `Name` registrations work for capture; the driver start record
has no PnP problem. The missing render endpoints were caused by categorizing
their bridge pins as `KSNODETYPE_ANALOG_CONNECTOR`: Windows classifies those
endpoints as `UnknownFormFactor` and hides/disables them by default. The render
bridge now uses `KSNODETYPE_SPEAKER` so endpoints are enabled by default while
keeping the unique per-cable pin `Name` GUID, which Windows checks before the
category's default friendly name. This is supported by Microsoft's endpoint
naming and default-visibility documentation. The static regression now rejects
the analog-connector category on render bridge pins. VM evidence is at
`C:\VMs\ar-share\evidence-20261008-183636.zip`; extracted locally under ignored
`target/vm-evidence-20261008-183636/`. The VM helper removed `oem5.inf`
successfully, and the final collected device inventory is clean.

Fix implemented: render bridge pins now retain the speaker pin category and
use unique per-cable `Name` GUIDs. `m03-inf-gen.ps1` passed; VM guards passed
274 checks; `m03-driver-build.ps1 -Platform x64` passed and explicitly did
not install/load the driver or change host trust/boot/audio settings. Package
integrity passed 33 checks. Commit `06e59457` contains the source fix,
regression check, and diagnostic notes. A clean test-signed x64 package was
staged at `C:\VMs\ar-share\repair-20261008-speaker-endpoints-clean`: version
0.1.0, built 2026-10-09 01:41 UTC from commit
`06e594575aee63a399d68b36212677450c710901`, `dirty: false`, 29 manifest file
hashes verified. Driver SYS SHA-256:
`05F74CC94ECCA7158BBD303BCA8C79B6EBA7860578E2E8D1EE9999B20A6900DD`.
The earlier `repair-20261008-speaker-endpoints` folder was built while the
worktree was dirty; use the `-clean` folder only.

Next action: in the VM, restore `02-test-signing-ready` and run the one-shot
command in the VM guide. The VM must pass A1-A3/A14 twice, with all four Cable
A/B Input/Output endpoints, before tone or Verifier. Host driver installation
remains paused while the unrelated host bugcheck is unexplained.

### VM follow-up: speaker naming still fails (2026-10-08 18:44)

The clean `06e59457` package passed preflight but failed A3 again. Evidence:
`C:\VMs\ar-share\evidence-20261008-184523.zip`, inventory
`20261008-184403-smoke/runner/endpoint-names-on-install-failure.json`.
All four project endpoints are healthy: two renders named
`Speakers (AudioRouter Virtual Cable)` and captures named Cable A/B Output.
The helper counts only two because the render identities lack cable letters;
A14 cleanup restores the complete baseline. This supersedes the claim that
the speaker category plus a custom pin Name would fix the names. The earlier
capture success uses custom category GUIDs, not render pin Name GUIDs.

Microsoft's [endpoint builder algorithm](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-endpoint-builder-algorithm)
explicitly fixes speaker names to Speakers. Its
[default visibility setting](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/pkey-audiodevice-enableendpointbydefault)
provides the missing counterpart to a non-speaker category: enable render
endpoints with `FLAG_ENABLE | FLOW_MASK_RENDER = 0x00000101` under the
associated interface's `EP\\0` key.

Plan for VCAB-02 and WP-03 A2/A3/A14: use the analog-connector render category
and unique pin Names, associate generated render EP properties with that
category, and set EnableEndpointByDefault to `0x00000101`. Keep capture
unchanged. Add generator checks for each render Wave/Topology interface and
capture isolation; run INF generation, VM guards, WDK build, package integrity,
docs and diff checks; build a fresh clean signed share and update the guide.
The combined naming/visibility change remains a candidate until Windows VM
evidence passes twice. Rollback is reverting this change and restoring the
clean VM snapshot. No host driver installation or loading is authorized.

Implemented the category/visibility combination and generated the INF.
Render Wave and Topology interfaces associate EP properties with the analog
bridge category and enable only render flow; all capture policy stays as
before. Generator regression checks each of the 32 Wave/Topology sections for
the correct role policy and pins the SDK property/category GUIDs. Driver build
acceptance passed; INF generation passed. The VM guard suite passed 275 checks
including a new COM all-state enumeration, verified against the SDK's
IMMDeviceCollection ABI. A helper-install failure now saves MMDevice IDs,
flow and state before cleanup without activating streams, as well as PnP
friendly names. This closes the evidence gap in the earlier analog-connector
retry: its hidden/disabled state was inferred from documentation, not captured.
Jev remains unrun because the previous automatic approval review rejected
transmitting the source diff to its external service.

Candidate staged and verified: `C:\VMs\ar-share\repair-20261008-render-enabled`,
version 0.1.0, built `2026-10-09T01:51:01.3638446Z` from clean commit
`5f61df3c367a6774fb829393b5458444889cad45`. Package integrity passed 33 checks;
29 staged manifest hashes were independently reread and verified; the staged
guard suite also passed 275 checks. Existing certificate thumbprint remains
`FF6876FBE50A74DC0B69077C11DC28A1A9FAAD40`. SYS SHA-256:
`54B7ABB3730550A8F2698C00F9850ACC5A3612F09D9B25318D82D3C6A1297362`.
Documentation passed 133 files/721 local links; diff whitespace check passed.
Runtime names, active state, stream safety and two-run gate remain unverified.
Next: restore `02-test-signing-ready`, run the guide's one-shot retry command,
and inspect its evidence before continuing Session 1 or later sessions.

First clean runtime pass verified (2026-10-08 18:57): archive
`C:\VMs\ar-share\evidence-20261008-185852.zip`, SHA-256
`857FAC1C7EDA67744CC8D4F764D8043E00B396B553A7FFA0AC40CFF0D0AA29ED`.
The clean `5f61df3c` package passed preflight, A1/A2/A3/A14 and collection.
All four names are `AudioRouter Cable A/B Input/Output (AudioRouter Virtual
Cable)`, status OK, with render flow on Input and capture flow on Output.
Windows default roles did not change; uninstall restored the full baseline.
The combined naming/default-enable fix is confirmed for this single install.
Next: restore `02-test-signing-ready` and run the same candidate again. Two-run
gate is 1/2; tone, formats, Verifier and later sessions remain pending.

Second clean runtime pass verified (2026-10-08 19:20): archive
`C:\VMs\ar-share\evidence-20261008-192120.zip`, SHA-256
`B04D885E35FC2A4C4EE0AA0696DEBC1D2637DAA6321316B338B98F2941FF766E`.
Same clean package; preflight and A1/A2/A3/A14 passed again. All four names
and render/capture flows are correct, default roles remained unchanged, and
uninstall restored the baseline. Session 1 naming/install/remove gate is
**2/2 passed**. This qualifies the initial VCAB-02 names, not rename,
ID persistence or audio quality. Next action: Session 2 install/status with
the existing package, inspect 60-format and minimum-period inventory, then
set up listening and run the tone tests only after that inventory passes.

Unexpected VM restart reported after the Session 2 install/status instructions
(2026-10-08). The exact command and restart cause are not yet established.
Session 2 is paused pending the guest System events, minidump and collected
evidence. Preserve these before restoring a snapshot or retrying installation.
The two completed smoke passes remain valid; stream safety is still unqualified.
Next action: collect read-only diagnostics in the restarted guest and copy the
archive and latest minidump through `Z:\` to the host shared folder for analysis.

Crash evidence received: `evidence-20261008-192858.zip` and
`100826-7703-01.dmp`. Matching Microsoft symbols identify bugcheck 0x3B,
access violation in `portcls!AcquireRemoveLock+4` during helper device open.
Installation and its idempotent repeat had succeeded; Session 2 remains blocked.
The bridge handlers were registered before `PcInitializeAdapterDriver`, which
overwrites those dispatch entries. Repair plan (SEC-08, VCAB-11): initialize
PortCls first, register bridge handlers afterward, publish the control device
last, and forward audio-device requests to PortCls by device identity. Add
regression guards for ordering and both dispatch paths; build/sign a fresh
candidate and verify the package. Runtime verification requires the user VM:
restore the clean snapshot, install and run status separately, then inspect
evidence before tone. Rollback: restore the clean guest snapshot; never load
this package on the host. A build alone cannot close this crash gate.

Replacement is ready in `C:\VMs\ar-share\repair-20261008-bridge-dispatch`,
clean source `151a3b69`, built `2026-10-09T02:32:39.0185028Z`. Windows x64
WDK acceptance and dispatch regression guards passed; package integrity 33
checks passed, 29 manifest hashes matched. The guide now separates install
from status and pins the replacement SYS hash. Next: user restores the clean
guest snapshot and runs the copy/preflight/install block, then sends output.
Status/format inventory follows only after installation is reviewed.

2026-10-08 19:36 guest evidence: clean preflight 15/15, install 3/3 and bridge
QUERY/status succeeded with protocol 1.1; the bridge-open crash reproduction
now passes on `151a3b69`. Archive `evidence-20261008-193649.zip` shows all four
endpoints at 48/60 accepted formats and a minimum period of 128 frames. All
12 rejected combinations per endpoint are 8-channel formats. The driver uses
obsolete wide 7.1 mask 0xFF, while inventory correctly requests surround
0x63F. VCAB-11 repair: use `KSAUDIO_SPEAKER_7POINT1_SURROUND`, assert its SDK
value, pin inventory/table agreement in the build regression, then build/sign
and stage a fresh clean package. No acceptance target is reduced. Guest
verification: clean snapshot, copy/hash verification, install, status and
collection; tone stays pending. Rollback: clean guest snapshot. Existing
stereo package configurations are unchanged; this corrects the advertised
eight-channel speaker positions for the test candidate.

Candidate ready: `C:\VMs\ar-share\repair-20261008-surround-layout`, clean
source `a8138984`, built `2026-10-09T02:40:41.6587231Z`. Host WDK acceptance,
native stdout/stderr regression, package integrity (33), manifest hashes (29),
and staged VM guards (275) passed. Guide pins the replacement SYS hash.
Next: user restores `02-test-signing-ready`, copies this candidate, runs
preflight/install and status/collection. Guest 60-format gate is still open.

2026-10-08 19:43 guest evidence verified: archive
`C:\VMs\ar-share\evidence-20261008-194356.zip`, SHA-256
`89740A881754EDAE031CEEF369C252BF330CEFEEC30C5195581F40F239C645A3`.
Preflight 15/15 and install 3/3 passed; bridge status returned installed,
protocol 1.1 and all four names. All four endpoints now accept **60/60**
formats, no rejected entries, mix 48 kHz float32 stereo. Minimum shared
period 128 frames, default/max 480, fundamental 1. VCAB-11 advertised format
gate and the original bridge-open crash reproduction passed. These checks
do not prove actual streaming, latency, fidelity or Verifier safety. Next:
continue in this installed VM, set Cable B Output Listen to the VM's physical
speakers, run Session 2 tone for 600 seconds and collect/copy evidence. Review
the WAV and counters before stall/8-channel tests; do not restore yet.

2026-10-08 19:54 tone interrupted: archive `evidence-20261008-195458.zip`,
SHA-256 `7CCFA6934BF5CA7D07D9CA123CE01E1EBCD36A3BC1C9AAE59FE66A97C33E1071`.
Transcript started 19:46:23 and was stopped at 19:54:34 (491 seconds), before
the configured 600-second duration. The wrapper buffers native output through
Out-String until process exit, so silence in the console is expected. No
counter report or completed summary exists; the 14,607,404-byte WAV retains
placeholder RIFF/data sizes (36/0) after interruption and is not a qualified
recording. This does not establish a hang or an audio-quality pass. Next:
same installed guest and listening setup, run `tone -ToneSeconds 30`, collect
and inspect completion/counters before repeating the full 600-second check.

2026-10-08 19:56 short tone completed but failed VCAB-24: 3000 capture blocks
written, capture-sink underrun_frames 1104 (23 ms), all other counters zero;
render-source 1530 blocks/734400 frames, all error counters zero. Final totals
do not locate the underruns in time. Do not waive them or assume VM scheduling.
Diagnostic plan: add user-mode counter snapshots at lease open, first write
and one-second intervals; print progress live through the VM wrapper while
retaining the log. Keep kernel behavior and zero-counter gate unchanged.
Run focused host tool tests/format checks, stage the diagnostic build, then
run only a 30-second guest trace before selecting a repair. Rollback: previous
test tool/package or clean guest snapshot. No host driver loading.

Additional review requested by user: completed OPEN/setup, single-slot ack,
WaveRT scratch/catch-up, close/counter timing and script gate review. Fixed
missing/duplicate report handling and `tone-8ch` zero-counter enforcement;
added live progress and raw counter snapshots. No kernel pacing change or
counter waiver. Host tool tests 4/4, focused Clippy, Rust fmt checks and
PowerShell output/counter regressions passed. Startup exposure and delayed
callback starvation remain hypotheses requiring the diagnostic guest trace;
see the [review record](evidence/2026-10-07-virtual-cable-ks-enumeration-crash.md#2026-10-08-1956-short-run-underrun-and-additional-review).

Diagnostic bundle staged in `C:\VMs\ar-share\diagnostics-20261008-tone-timing`
from clean `5cb14067`: only the static-CRT tone tool and vm-checks.ps1, with
SHA-256 manifest `diagnostics.json`. Existing `a8138984` guest driver stays
installed; no snapshot restore or driver update for this trace. Next: copy
and verify those two files, run 30 seconds with existing Listen setup, then
collect/copy and locate when underrun counts first rise. The producer bug
is not marked repaired. Jev remains unrun (prior external-upload rejection).

2026-10-08 20:16 diagnostic trace verified: archive
`evidence-20261008-201644.zip`, SHA-256
`CA1C2447E49362C1EFC5391C407D2B9FBF2B01C31C333F181B942A8F1200370A`.
Counters are zero at lease-open/first-write, 96 underrun frames at 1 second,
144 at 2 seconds, and 192 at 22 seconds through exit. The late 48-frame
increment disproves a startup-only diagnosis. Other error counters remain
zero. 3000 capture blocks written, 765 render blocks/367200 frames recorded.
Do not retry the long test or change acceptance thresholds.

### Capture buffering decision — approved 2026-10-08

The review found a structural limitation: the capture callback requests a
new bridge block only after its scratch is exhausted; a producer busy publishing
or a callback catching up cannot wait for user mode and currently outputs
counted silence. The trace proves runtime starvation, but not which timing
race caused each increment. Proposal for VCAB-24/25/28 and SEC-08:

1. Add one fixed preallocated capture prefetch block per stream. Opportunistically
   copy/acknowledge the next valid block while the current block still has
   samples; retain both blocks and consume strictly by sequence. No shared
   protocol/layout, helper command, endpoint identity or format change.
2. Preserve generation/format boundaries: reset both valid lengths on
   generation change, unusable format, stop and teardown; never replay stale
   audio. An empty queue still yields silence and increments the true underrun
   counter. No allocation, wait, lock or logging in the audio callback.
3. Add offline regressions for publication-in-progress, partial block reads,
   callback batches across two blocks, missing-block silence, no duplicates,
   generation/format changes and bounded memory. Test the owning buffering
   logic rather than merely checking source strings.
4. Record fixed memory increase and added buffered duration; measure the
   existing latency gate rather than assuming prefetch is free. Build/sign
   only after regressions, diff review and host checks pass; user VM checks
   30 seconds, then 600 seconds, deliberate stall and 8-channel operation.
5. Rollback: restore the clean guest snapshot and retain prior package/tool
   plus archived evidence; no host driver loading.

The user approved this proposal on 2026-10-08, satisfying rule 9. Implement
the bounded queue, verify FIFO/partial reads/busy publication and reset paths,
then build a fresh signed candidate. The zero-counter acceptance gate remains
unchanged. Runtime repair and latency remain unverified until guest evidence.

Implemented: the capture path uses a two-slot private FIFO and bounded
prefetch; the existing render scratch stays independent. Generation races
discard the queue and disable further reads for that callback. STOP and
unusable format invalidate buffered audio; acknowledged sequence is retained
across STOP. Memory increase is 256 KiB per stream plus 32-byte queue state,
less the removed 8-byte sequence field. Added buffering is bounded to one
quantum beyond the old scratch (10 ms at the diagnostic setup); VCAB-25 is
still open. Shared bridge layout and zero-error counters are unchanged.

Host regression: `drivers/audiorouter-virtual/tests/build-tests.ps1` passed
252 checks, including actual queue sample ordering, busy-publication attempts,
partial/batched consumption, duplicate/invalid-length refusal, and reset
boundaries. Log: `target/driver-unit-release/tests.log`. x64 WDK acceptance
passed (`target/capture-prefetch-build.txt`). Neither is kernel runtime
evidence. AddressSanitizer was attempted but could not link because the installed
MSVC toolchain lacks `clang_rt.asan_static_runtime_thunk-x86_64.lib`
(`target/driver-unit-asan/compile.log`); no ASan pass is claimed.
Jev remains unrun due the prior automatic-review rejection of
external source upload. Next: finish host checks and stage a signed clean
candidate, then restore the clean guest snapshot and run install/status and
only a 30-second trace before the long continuity and latency gates.

Candidate staged: `C:\VMs\ar-share\repair-20261008-capture-prefetch`, clean
source `abb303ee8834c64af6ea273a7bdd960b128926e0`, built
`2026-10-09T03:30:54.0452936Z`. SYS SHA-256:
`B45C9EF5B4B466D729BBD6579962F1540BB853377A14C14A647AA641BD4605BA`.
Package checks 33/33 and independently reread manifest hashes 29/29 passed;
VM guard checks 275 passed. Final x64 WDK acceptance used
`PreferredToolArchitecture=x64` and the installed MSVC Hostx64/x64 compiler
directory on the process PATH. Logs: `target/capture-prefetch-build.txt`,
`target/capture-prefetch-share.txt`, `target/capture-prefetch-vm-guards.txt`.
Documentation validation passed 133 files/724 links; whitespace check passed.
No driver was installed or loaded on the host. Next: restore
`02-test-signing-ready`, use Session 2's copy/hash/preflight/install block,
review installation, then status and a configured 30-second trace. Successful
copy acknowledgement means private buffering, not completed playback; the
short diagnostic and its counters do not substitute for external capture,
fidelity or impulse latency qualification. VCAB-24/25/28 remain open.

2026-10-08 20:36 candidate failed VCAB-24: archive
`evidence-20261008-203731.zip`, SHA-256
`8EDD4624AA55FE2619F7C866516BB2B5085118E7B05BE5B11C8460088FE54833`.
Install and all four 60-format inventories passed. Capture underruns were
already 480 at lease-open/first-write, 576 at 1 second, 624 at 4 seconds,
and 672 at exit. Render was never stimulated: zero blocks and a header-only
WAV; its zero counters are not render streaming evidence. Prefetch alone did
not resolve the defect and is not qualified.

Source review found the missing integration: TimerNotifyRT runs every 1 ms
but skips UpdatePosition until a notification is due (10 ms here). Capture
prefetch therefore cannot build headroom between notifications. The tone tool
also opens the capture lease before opening render, creating its WAV and
allocating/filling samples, exposing an active empty lease during setup.
Within the approved bounded-prefetch repair, next tasks are: service capture
DMA on each RUN timer tick while keeping notification/packet cadence unchanged;
prime the first validated capture block before broker OPEN, with all other
setup completed first; test timer integration plus actual primed mapped-session
contents/sequence/error cleanup; rerun WDK, focused Rust tests/Clippy/format,
VM guards/package checks and stage a clean candidate. No counter subtraction
or startup exemption. Rollback remains the clean snapshot/prior package.

Implemented the missing timer integration and primed capture startup.
Offline queue/timing suite passed 254 checks, including a model that fails
with notification-only service and passes with per-tick service under a
one-tick producer delay and notification overshoot. This is simulation, not
kernel timing evidence. The actual mapped-session priming test passed:
sequence 1 and all samples are readable before broker OPEN, the next write
is sequence 2, and wrong direction/invalid samples remove the owned file.
Seven existing session tests and four tone-tool tests passed; crate/all-target
Clippy and both Rust formatting checks passed (compiler reported a cache
hard-link fallback, not a code lint). Logs: `target/primed-session-tests.txt`,
`target/capture-tick-session-tests.txt`, `target/capture-tick-tone-tests.txt`,
`target/capture-tick-clippy.txt`, `target/driver-unit-release/tests.log`.
The failed prefetch-only candidate remains preserved; there is no accepted
runtime continuity result yet. Next: complete WDK/package/VM guard checks,
stage a clean candidate and request only install/status followed by a
30-second trace with both capture listening and render Test stimulus.

Review also caught a compatibility risk from faster DMA updates: the old
byte-rate arithmetic can advance by partial frames at 44.1 kHz. UpdatePosition
now rounds displacement down to complete frames and carries the fractional
byte numerator modulo `1000 * nBlockAlign`. The shared helper is tested for
all three supported rates and all seven possible frame byte sizes across
1000 one-millisecond updates: exact one-second sample totals, zero remaining
carry, and no partial frame. Invalid zero alignment fails closed. Final native
suite: 276 checks. This preserves the existing rate/format contract (VCAB-11/12)
while servicing capture between notifications. WDK acceptance, 275 VM guards,
PowerShell output/counter regression, docs and diff checks passed; ASan/Jev
limitations remain as recorded above. No host driver loading occurred.

Corrected candidate staged: `C:\VMs\ar-share\repair-20261008-capture-tick-primed`,
clean source `dbf19e178c737e9aa4cc93bc1048bd66ec591c3d`, built
`2026-10-09T03:46:14.0611758Z`. SYS SHA-256:
`692C013CF9727985FE804F4020388108D1A568ECF8FC0D99B9B525771E39D646`.
Package checks 33 passed; all 29 manifest hashes independently reread and
matched. Log: `target/capture-tick-share.txt`. Guide now runs install/status
checks together before any listening/tone; an earlier failure stops the block.
Next: restore `02-test-signing-ready`, copy/hash/preflight/install/status,
review output, then configure Listen and run 30 seconds with Cable A Input
Test stimulus. The previous trace's empty render WAV cannot qualify A4.
No long test until this candidate's trace is reviewed. The zero-error gate
and measured latency requirement remain unchanged.

### 2026-10-08 20:54: per-tick candidate still fails; review before another retry

Archive `C:\VMs\ar-share\evidence-20261008-205521.zip`, SHA-256
`54FD1A4D3A2DDC4C4BEF1EB28A80D2EAC680ECCE4A7C6F953CAA99EA73A5EA6E`.
Preflight/install/status passed. Capture counters are zero at primed OPEN and
through 19.030 seconds, then 4272 underrun frames at 20.030 seconds, unchanged
through exit. That is 89 ms of missing capture audio in one interval. Render
has 480 overrun frames by 29.044 seconds (one 10 ms block). 2993 capture
blocks written, 508 render blocks read, 243840 WAV frames. This is a failed
VCAB-24 run; neither direction's continuity gate passes. Primed startup and
per-tick prefetch behaved cleanly during the initial interval, which does not
qualify sustained playback or establish the later failure's cause.

Review of the actual test loop found an unisolated audio path: the same thread
publishes capture, polls render, allocates/serializes/writes WAV data, performs
two synchronous heartbeat IOCTLs, prints synchronous console progress, and
sleeps. The log does not time those operations, so it cannot distinguish a
blocking tool operation from scheduler delay or driver timing. The ~20 ms
private capture reserve cannot cover an arbitrarily blocked producer. The
earlier host timing model tested a one-tick producer delay; it did not cover
this failure class. Do not increase buffers or relax counters on this evidence.

Harness design approved by the user (2026-10-08) under plan rule 9:

1. Keep driver/ABI and zero-error acceptance unchanged. Prepare resources and
   prime capture before activation; move mapped capture production and render
   consumption to dedicated audio workers with explicit readiness and shutdown.
2. Keep heartbeat IOCTLs, progress output and WAV I/O outside those workers.
   Transfer float64 render blocks through fixed, preallocated bounded storage;
   report an exhausted recording queue as a harness failure, never block or
   silently discard. Preserve exactly one producer/consumer per direction.
3. Record each worker's maximum pump gap and control/recording operation
   durations through bounded diagnostics. Treat thread scheduling support
   failures explicitly. Do not claim MMCSS or scheduling guarantees unmeasured.
4. Before any guest retry, exercise the actual worker/mapped-slot path on the
   host with deliberately blocked heartbeat, console and recording operations,
   plus recording backpressure, producer stalls, sample order, sequence counts,
   startup failures and shutdown. Verify that peripheral stalls do not halt
   audio service and that a real producer stall is still reported.
5. Inspect the complete implementation and focused Windows checks, then stage
   only the harness update for the existing guest driver. Review a 30-second
   trace before any long run. New transport buffering, if still needed, is a
   separate decision supported by that evidence. Latency/fidelity gates remain
   open. Rollback is the previous tool bundle or clean guest snapshot.

The isolated harness is implemented: prepared activation, two mapped audio
workers, control-only main thread and bounded recording worker. Both leases
deactivate before cleanup; final render data is drained. Host regressions:
11 tone/worker tests and 30 bridge tests passed, including a 160 ms blocked
recording/control interval, explicit backpressure and a visible 120 ms producer
stall. Details and limits are in the
[harness record](evidence/2026-10-08-isolated-tone-harness.md).

The first 30-second isolated VM run exercised capture and render, but exited
with `SampleSizeMismatch` during teardown. Its final counters were zero because
the driver resets the mapping on CLOSE. Fixed the order: snapshot raw counters
before CLOSE; deactivate leases, stop/join workers and drain WAV buffers,
then clean up their mappings. Details are in the evidence record. Twelve tone
tests, 30 bridge tests, focused Clippy and formatting pass after this fix. No
driver files or thresholds changed.

The rebuilt, hash-verified tool-only bundle is ready at
`C:\VMs\ar-share\diagnostics-20261008-isolated-tone` (source `966a83b6`).
The driver file was not included. Corrected guest run passed all four tone
checks; archive and metrics are in the harness evidence record. Next action:
run the 10-minute tone check on the already installed VM using
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step tone`.
Keep Cable B Output Listen enabled and stimulate Cable A Input during the run.
Review the 10-minute trace before stall/8-channel checks. Archives are retained;
no kernel patch or threshold change. VM sustained continuity, fidelity and
latency gates remain open; the 30-second success does not qualify them.

The next guest attempt stopped after about 29 seconds with one 480-frame
render overrun and one harness sequence gap. Its maximum render poll gap was
11.186 ms; heartbeat and WAV operations were each below 1 ms. It requested a
600-second run but did not complete the 10-minute gate. Full archived evidence,
limits, and the scheduling follow-up are in the
[render-overrun record](evidence/2026-10-08-m03-render-overrun.md). Before
retrying, the isolated workers were updated to request MMCSS `Pro Audio`,
report fallback capability, and include sequence and timing details in any
gap error. Focused host checks and workspace Clippy pass; this is not VM timing
evidence. The updated tool-only bundle is staged at
`C:\VMs\ar-share\diagnostics-20261008-mmcss-render`; its executable and driver
hashes and the exact VM command are in the
[render-overrun record](evidence/2026-10-08-m03-render-overrun.md). Run the
30-second guest validation before another 10-minute attempt. Preserve the
zero-counter acceptance criterion and installed VM driver.

The MMCSS 30-second retry confirmed both workers acquired `Pro Audio`, with
zero counters and no sequence gap, but exposed a second shutdown race:
deactivation clears the mapping header just before the worker stop flag is
set. The worker can report the expected clear as `SampleSizeMismatch`. Fixed
with a separate retirement signal set before deactivation; only that exact
error is tolerated after retirement begins. Keep runtime mapping errors and
the zero-error gate unchanged. Host formatting, tests and Clippy pass. The new
tool-only short retry is staged at
`C:\VMs\ar-share\diagnostics-20261008-retirement-fix`; its exact command and
hashes are in the [render-overrun record](evidence/2026-10-08-m03-render-overrun.md).
Do not run the 10-minute test until this retry passes.

The next short retry had zero render blocks because Cable A Input was not
stimulated; counters and sequence gaps stayed at zero, and MMCSS was active.
The tool correctly reported missing playback. The retry script now waits for
readiness and gives a five-second countdown before its 30-second tone run;
click Cable A Input Test while the run is active. The exact guest archive and
result are recorded in the
[render-overrun evidence](evidence/2026-10-08-m03-render-overrun.md).
The updated tool-only bundle is staged at
`C:\VMs\ar-share\diagnostics-20261008-tone-ready`; in the VM, run
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File Z:\diagnostics-20261008-tone-ready\retry-isolated-tone.ps1`.
The 30-second retry passed with all error counters and harness sequence gaps
at zero. It recorded 506 render blocks (about 5.06 seconds of audio), so for
the 10-minute gate, route a known file into Cable A Input and keep playback
continuous for the full run. Use
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step tone -ToneSeconds 600`,
then run the collect step. Archive and detailed metrics are in the
[render-overrun evidence](evidence/2026-10-08-m03-render-overrun.md).

The first 600-second run completed but failed the clean-run gate: capture
underrun was 2,208 frames (46 ms), first observed at 67.193 seconds; maximum
capture/render worker poll gaps were 64.984/65.410 ms. Both workers had MMCSS
Pro Audio and a 1 ms timer, and no fallback. The progress trace shows Cable A
Input data for only 76.52 seconds total, not continuous for the whole test.
The max-gap timestamps are unavailable, so a scheduling pause is plausible but
unconfirmed. Do not run stall or 8-channel checks. Repeat the clean 600-second
run only with a known file looped continuously into Cable A Input, the VM in
the foreground and no heavy host work. If the underrun recurs, investigate VM
scheduling and capture buffering before changing the driver or its thresholds.
Archive and details are in the
[render-overrun evidence](evidence/2026-10-08-m03-render-overrun.md).

The next 600-second attempt failed during startup: at 17 ms the render reader
first observed sequence 2 instead of 1, recorded one 480-frame overrun, and
exited before recording WAV frames. Its own poll gap was only 1.466 ms
(maximum 2.165 ms). Code review found that the harness activated the render
and capture leases before waking the render reader, so the first block could
be overwritten before polling began. The harness now starts the render poller
first, waits for its first empty poll, then activates both leases. A focused
regression verifies that an immediately published first block is consumed
without a gap. Host checks: 14/14 focused tests passed; workspace and
`src-tauri` Clippy passed with `-D warnings`; formatting and `git diff --check`
passed. A static-CRT tone executable was built and its non-device `--help`
path exited as expected. The tool-only VM bundle is staged at
`C:\VMs\ar-share\diagnostics-20261008-render-startup`; no driver package was
rebuilt or included. The guest short test remains pending. Keep the zero-error
criterion and do not retry the 600-second gate until the short guest test
passes. Full archive and run details are in the
[render-overrun evidence](evidence/2026-10-08-m03-render-overrun.md).

That short retry confirmed startup polling was fixed but still observed
sequence 2 followed by sequence 4 at 996 ms, with one 480-frame render
overrun. The run used only the brief Windows Test sound, and the current trace
cannot distinguish a callback burst from delayed reader scheduling. The
render worker now repolls immediately after a successful read rather than
sleeping 1 ms before the next poll. The retry script now requires a sound file
looped to Cable A Input for the full 30 seconds. The updated short run passed:
3,000 render blocks / 1,440,000 frames, with all driver counters and harness
sequence gaps at zero. This clears the short-run gate only; the 600-second
run remains unqualified. Next, run the 600-second test with the known file
looped continuously into Cable A Input.

The first 600-second retry after the short pass failed at 18 ms, before its
first progress report: render sequence 2 was observed before sequence 1 was
acknowledged. The startup-poller message was present, and the poll gap was
2,564 μs. This is not a 564-second failure; the guest archive's tone trace
shows immediate startup failure. If Media Player was already playing before
the render lease opened, that could explain the initial burst, but playback
state is not recorded. For the next retry, prepare Media Player routed to
Cable A Input but paused, start the 600-second command, then start playback
when the first progress line appears. The clean 10-minute gate remains open.
Archive and measurements are in the
[render-overrun evidence](evidence/2026-10-08-m03-render-overrun.md).

**Agent (host, can start now):** WP-08 status detection and the
`virtual-cable` CLI/API (17 §7.1–7.2), then WP-09 engine nodes. WP-09 must
use the capture-sink acknowledgement (`consumer_sequence`) for producer
pacing, as `m03_bridge_tone` does, and poll render-source leases faster than
one period; a wall-clock producer against the single-block slot cannot be
glitch-free (VCAB-24). Evidence from the VM sessions takes priority over new
host work: fix what they find first.

Do not load the intermediate package on the host or call a host build a
runtime or quality pass.

2026-10-09 VM status failure (candidate `repair-20261009-render-retention`):
install and both smoke cycles passed. The status inventory JSON contains four
valid endpoints (all 60 formats, minimum period 128 frames), but the inventory
tool reports 12 name matches and eight `IAudioClient3` activation failures.
Root-cause review found `enumerate_active_endpoint_display_info()` is documented
to return active endpoints but calls `EnumAudioEndpoints(..., DEVICE_STATE_ALL)`;
the tool therefore counts inactive/stale Windows endpoint records as duplicates
and tries to activate clients for them. Requirements: VCAB-11, VCAB-25. Scope:
make this inventory snapshot enumerate active endpoints only; do not remove VM
devices, alter acceptance thresholds, or change driver behavior. Add a source
acceptance guard that pins the active-state flag, then run the focused acceptance
and build/package integrity checks, stage a replacement package, and ask for a
single status rerun before tone. Validation does not claim guest behavior until
that rerun passes. Rollback: retain the current guest/package and replace only
the staged inventory/tool bundle. Next action: fix the enumeration flag and
source regression guard.
