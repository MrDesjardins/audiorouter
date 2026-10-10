# Active plan — AudioRouter virtual cable (DEC-18)

Updated 2026-10-09. The driver is installed only in AR-DriverTest. Smoke,
active format inventory and short tone checks pass; reported Cable B hiss and
sustained audio loss still block qualification. Direct recordings have been
reviewed, including the guest speaker-loopback capture. The current next
action is read-only playback/backend review at the end of this plan;
do not repeat the completed diagnostics unchanged.
All implementation and testing happens on
the user's Windows 11 development PC and its VirtualBox test VM, with the
host's Hyper-V/WSL capability preserved.

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
Root cause: `enumerate_active_endpoint_display_info()` was documented to return
active endpoints but called `EnumAudioEndpoints(..., DEVICE_STATE_ALL)`. The
inventory therefore counted records from every Windows endpoint state and
attempted to activate clients for them. The JSON lists only four because the
tool skips a record before serializing it when client activation fails; the
three-match/12-total diagnostics are computed before that filter. The guest
output did not include each record's state, so the eight extra records are not
individually classified.
Requirements: VCAB-11, VCAB-25. Fix committed and pushed to `main` as
`d9e72560`: enumerate active endpoints only and pin the state flag in the M03
build acceptance guard. Driver behavior and acceptance thresholds are unchanged;
no VM devices were removed. Focused evidence: inventory example tests 2/2,
release executable build with static CRT, focused Clippy, Rust formatting,
docs validation (136 Markdown files/738 links), and manifest verification for
all 29 staged files. The driver's SYS hash is unchanged. Jev review could not
reach its API and remains unrun. New bundle:
`C:\VMs\ar-share\repair-20261009-active-inventory`; inventory tool SHA-256
`D08495D741E99C1AC4FA4F0BE234C5C5CC2D920E30C1AACF28684DC6B94CDEB2`.
The active-state inventory repair passed in the VM. The status step found
exactly four active AudioRouter endpoints, each with 60 supported formats and
a 128-frame minimum period. The 30-second tone run passed all four checks
using bundle `repair-20261009-active-inventory`; archive
`C:\ar\repair-20261009-active-inventory\evidence-20261009-171045.zip`,
SHA-256 `F6D16DDFA8AC1B0A02AFBC349F074D249B74BEDD5324F6F59E8E24AF69F94BA3`.
The trace records 3,003 capture blocks written, 3,000 render blocks read
(1,440,000 stereo frames / 30 seconds), zero driver error counters, zero
harness sequence gaps, and maximum capture/render worker poll gaps of 6.200 /
6.192 ms. WAV format/header was verified as stereo 48 kHz float32 for exactly
30 seconds. This qualifies the status repair and short-tone gate only; it does
not qualify sustained continuity or latency.

The 600-second attempt failed; it is not continuity evidence. Archive:
`C:\ar\repair-20261009-active-inventory\evidence-20261009-173249.zip`,
SHA-256 `A6BA1BE7E0AD2DABE1C4523D61386C3D6898E41E0437E51AE34160E38CA61E1B`.
The trace began with zero counters, then at 178.536 seconds showed 1,776
capture underrun frames (37 ms) and 2,256 render overrun frames (47 ms). At
269.828 seconds those totals rose to 3,744 (78 ms) and 5,184 (108 ms), then
remained there through 316.956 seconds. After that, both audio worker pollers
had a 895,145 ms gap (about 14 minutes 55 seconds). Final counter totals were
42,969,360 capture underrun frames and 42,970,944 render overrun frames, which
correspond to roughly 895 seconds. The tool recorded 31,722 capture blocks
and 31,716 render blocks (15,223,680 WAV frames, about 317 seconds), zero
harness sequence gaps, and then failed with HRESULT `0x80070016` (“The device
does not recognize the command”). This strongly indicates execution stopped
for a long interval, but the trace alone cannot identify whether Windows, the
VM, or its host paused it. The earlier 37/47 ms continuity errors occurred
before that pause, so the result also fails even when the long gap is set
aside.

The host's available `VBox.log.1` is an older session (opened
`2026-10-09T03:51:36Z`); the current `VBox.log` is empty. The guest query
produced no matching System or Power-Troubleshooter events. Reading the live
log for registered VM `AR-DriverTest` with VBoxManage supplied the relevant
host-side record: VirtualBox is using NEM's “Snail execution mode” because the
Windows Hyper-V hypervisor is active. The log records a separate 44-second
guest heartbeat gap early in the VM session. At relative 00:55:15 it reports
a 895,139,986,951 ns catch-up lag and a guest heartbeat absent for 896
seconds, then reports the guest alive again. The VM session start and these
relative timestamps align with the test's 14m55s worker gap. The host System
log had no matching Kernel-Power, boot, shutdown or power-troubleshooter
event. This confirms a VirtualBox guest execution stall, not a 14m55s
driver-side audio loop. It does not establish what paused or starved the VM.
The earlier 37 ms capture underrun and 47 ms render overrun at 178.536
seconds, followed by additional errors at 269.828 seconds, remain separate
continuity failures.

Next: do not rerun tone yet. Review the host's Hyper-V/NEM configuration and
VM scheduling/power conditions; do not disable Hyper-V, VBS, WSL or other host
features without an explicit user decision. Then decide whether to qualify
this VM under its current compatibility backend or use a host configuration
where VirtualBox can use native hardware virtualization. Do not reinstall the
unchanged driver. Acceptance remains zero driver error counters and zero
harness sequence gaps; rollback is the existing clean VM snapshot and
previously installed driver.

User decision (2026-10-09): preserve WSL; do not disable the Windows
hypervisor or its dependent features. Host inspection found the documented
VM settings already in use (4 vCPUs, 8192 MB, 100% execution cap, nested
paging on), with VirtualBox NEM active. The host Balanced power plan has
sleep-after set to Never on AC; no matching host power/boot/shutdown events
were present for the 14m55s stall. The earlier suggestion that the guest may
have idled while the user was away is not supported: the user confirms the VM
remained on and neither sleep nor a screen saver occurred.

Supervised 300-second diagnostic (2026-10-09): completed the full duration,
with no long pause or harness sequence gap. Four progress snapshots showed
new counter bursts at 58.160 s (816 capture underrun / 1,824 render overrun
frames), 83.234 s (7,008 / 8,496 cumulative), 210.589 s (13,728 / 15,696),
and 273.788 s (16,704 / 19,152). Final totals correspond to 348 ms of
capture underrun and 399 ms of render overrun at 48 kHz; maximum measured
capture/render pump gaps were 163,040 / 163,082 us. Thus keeping the VM
awake/foreground avoided the previous multi-minute execution pause but did not
produce clean continuity. The user's confirmation rules out sleep and screen
saver as causes. NEM scheduling remains a plausible contributor, not a proven
root cause. Evidence archive: `C:\VMs\ar-share\evidence-20261009-175425.zip`,
SHA256 `477BD491D3C8BD84B364A4337F581EEA5A5D23739B61E5557E09867317033CAB`;
guest tone evidence is `20261009-174917-tone/tone.txt` in the archive.

Next: do not repeat the unchanged long test. Review VM/host scheduling and
diagnostic instrumentation, preserving WSL and leaving Hyper-V settings
untouched. The initial review of this trace did not identify a driver defect
(superseded by the packet-clock findings below):
`WriteBytes`' capture-side queue holds at most two 480-frame blocks (20 ms at
48 kHz), while `ReadBytes`' render-side queue is capped at four blocks (40 ms
at 48 kHz); the shared bridge slot adds at most one block. A worker scheduling
gap of 163 ms is therefore far outside available buffering. Absorbing it
would require buffer headroom on the order of 160 ms or more and introduce
comparable transient latency, conflicting with VCAB-25's 20 ms low-latency
target, while masking a stalled VM rather than repairing a demonstrated
product bug. The tone recorder's 64 preallocated
10-ms packets provide 640 ms of disk backpressure, and its maximum WAV append
time was 56.2 ms; the writer did not run out of packets. It read 29,960 blocks
(14,380,800 frames) versus 30,000 expected for 300 seconds, a 40-block / 400-ms
shortfall. That closely matches the 19,152-frame / 399-ms render overrun
counter, confirming actual audio loss rather than a counter-only failure.
The initial conclusion that no code change was justified is superseded by
the deeper packet-clock review below. Scheduling remains unproven. Obtain ETW CPU,
context-switch, DPC/ISR and VirtualBox scheduling traces during a bounded run,
or qualify on a host where VirtualBox uses native hardware virtualization.
Keep the continuity gate failed; do not claim sustained runtime qualification
or install the test driver on the host. Preserve the clean VM snapshot and
unchanged driver package.

### Packet-clock repair (2026-10-09, requested code review)

Objective: repair demonstrated WaveRT accounting defects before another VM
run; preserve WSL and host settings. Requirements VCAB-24/25/27/28 and VDEV-12.
Prerequisites: clean working tree; previous guest evidence preserved in the
archives above; WDK/MSVC build tools available. Host builds do not load a driver.

Confirmed findings: DMA advances by full elapsed time while the notification
callback increments the completed-packet count only once; position queries
can consequently return an inconsistent count. Capture catch-up traverses
unbounded DMA laps under a spinlock. GetReadPacket extrapolates the end rather
than the first sample of the latest completed packet. STOP retains fractional
time/byte carry from the previous stream.

Ordered tasks: (1) derive packet count from DMA position, separately retain
the last notified count, correct capture timestamp and STOP reset; (2) bound
capture catch-up to one surviving DMA lap and count skipped frames;
(3) regress production arithmetic for delayed callbacks, wrap and restart;
(4) add per-progress-interval worker gap diagnostics; (5) run host-safe C++
units, WDK x64/ARM64 acceptance, Rust example tests/Clippy/format and docs;
(6) prepare a checksummed VM bundle and one bounded test procedure.

Validation: arithmetic and source wiring are checked on the host; only the
guest can establish install, endpoints and continuity. Do not increase queues,
ignore losses, relax zero-counter acceptance, or claim a delayed callback can
recover overwritten audio. Rollback: restore the known clean test-signing VM
snapshot and retain the previous bundle. Next action: implement these repairs,
then document measured host results and the precise next guest command.

Implementation/checks completed: packet clock and separate notification state,
first-sample timestamp, STOP fractional reset, bounded capture traversal and
interval worker diagnostics. C++ host units: 569 checks (578 after the QPC
conversion follow-up below); Rust tone example:
15 tests; WDK x64 and ARM64 acceptance: pass; workspace/shell Clippy and fmt:
pass. Documentation validation: 138 Markdown files / 746 local links; retest
script parses in Windows PowerShell and refuses host execution. Details and limitations:
[review evidence](evidence/2026-10-09-m03-packet-clock-review.md).
Next: build the identified candidate, then follow the
[bounded VM procedure](../../operations/virtual-cable-packet-clock-retest.md).
Original runtime failure remains unverified until the guest run; do not mark
continuity or measured kernel latency as passed from host-safe checks.

Final arithmetic review found another independent defect: converting an
absolute capture timestamp back to QPC multiplied before dividing, overflowing
after about 51 hours of uptime at 10 MHz. Split seconds/remainder before
scaling; regress 100-day timestamps and representable/overflow boundaries.
Use the `repair-20261009-packet-clock-r2` bundle including this follow-up,
not the earlier packet-clock candidate. Both WDK targets and all 578 C++
checks pass again after this follow-up. Next: verify the rebuilt test-signed
bundle and hand off the bounded procedure. Completed: r2 built from clean
source `97b393d5`, package integrity 33/33 and all 30 manifest hashes pass.
Driver SHA256 is recorded in the review evidence. Next action is the guest
procedure; no VM run was performed during this review.

Guest follow-up: the user ran r2 Prepare successfully. Archive
`evidence-20261009-182328.zip` is independently hash-verified on the host;
preflight, smoke A1/A2/A3/A14, install/idempotency, four-endpoint format status
and collection all pass. Details are in the packet-clock review evidence.
Next: 30-second tone in the same installed guest session, with Cable A Input
loop playback and Cable B Output listening already active; 300 seconds only
after the short run passes. No new continuity result yet.

The r2 30-second tone now passes, independently reviewed in archive
`evidence-20261009-183059.zip`: all driver error counters and harness sequence
gaps zero; exactly 30 seconds recorded with a valid WAV header/length.
Maximum worker gaps were 7.523 ms capture / 7.467 ms render. Next action:
300-second Tone phase in the same guest session with loop playback/listening
kept active. This short result does not close the sustained, eight-cable,
waveform, DPC-duration or hardware latency gates.

### 2026-10-09: five-minute harness exceeded its deadline

Archive `evidence-20261009-191454.zip`, independently verified SHA256
`B576C55519311EF2E7DC00D89AF5B2304C0F6077B0E90801D500C4D2AC1C30C5`,
shows synchronous progress output blocked 2,310,481,790 us. The control
thread could not service heartbeats or stop leases, while audio workers
continued beyond the requested 300 seconds (2,530.98 seconds recorded).
Output blocking is measured; its console/environment trigger is unknown.
There was also earlier real loss: around 56 seconds, both worker gaps reached
53 ms and counters rose to 1,584 capture underrun / 2,016 render overrun frames.
Thus repairing the report hang alone cannot establish sustained continuity.

Objective (VCAB-24, VDEV-12; VCAB-27/28 remain open): make duration independent
of output and preserve failures. Ordered implementation: (1) buffer bounded
progress snapshots in memory while leases are active; (2) enforce the control
deadline independently of worker completion; (3) stop/join/close before
writing reports; (4) redirect native tone output to evidence files and add a
process timeout; (5) host-only regressions, formatting/Clippy and docs review.
No driver or host hypervisor change is planned from this evidence. Preserve
zero-counter acceptance and all prior archives. Rollback: retain the previous
bundle and revert the harness changes. Next action: implement and verify these
repairs before requesting another guest run; sustained audio loss stays open.

Reporting repair implemented: deferred bounded snapshots, independent control
deadline, lease shutdown/WAV worker join before reporting, control-loop timing,
native file redirection and owned-child watchdog. Host verification: 17 Rust
tone tests, 11 fake-process regressions, workspace/shell Clippy/format,
PowerShell syntax, and docs (138 Markdown / 747 local links) pass. The process
regression caught and corrected Windows PowerShell 5.1 losing the native exit
code unless its handle is retained. No VM/driver/hypervisor operation was run.
Next: build and independently hash-verify the separate bounded-tone update,
then hand off its 30-second command. Earlier real audio loss is not repaired
or waived by these host results; preserve the failed sustained gate.

Candidate prepared from clean source `658be117`: 30 manifest hashes pass,
all 16 driver files byte-identical to r2, copied runner refuses host execution.
Only the Release static-runtime tone executable and two supporting scripts
changed. Artifact hashes and exact next guest task are in the packet-clock
review/retest procedure. Next: updated 30-second tone in the existing guest
session; no reinstall/restore required. Review before any sustained run.

Guest bounded-tone follow-up: archive `evidence-20261009-192823.zip` hash
independently verified. Status/tone/collect pass; zero error counters and
harness sequence gaps. Leases stop at 30,001 ms; process completes in 30.604 s
without timeout. All 29 deferred snapshots and the valid 29.99-second WAV
are present. Full measurements and limitations are in the review record.
Next: bounded 300-second Tone with the same installed driver and audio setup;
inspect control/worker timing alongside any loss. Sustained acceptance stays
open until that and the remaining required gates have evidence.

### 2026-10-09 19:35: bounded five-minute run still loses audio

Archive `evidence-20261009-193519.zip`, SHA256
`F77E1359EC22E368420A58BDC04ABD8A26ABEB2A1E35E141A49554DCBB4E6D2B`,
independently verified. Tool exits 0/no timeout; leases stop at 300,003 ms,
process 300.145662 s. Capture underrun 16,704 / render overrun 17,520 frames;
all other error counters/harness sequence gaps zero. First loss snapshot at
122.349 s correlates capture/render/control gaps 310.442/310.289/317.342 ms;
second at 184.541 s correlates 23.117/23.222/28.291 ms. The output hang is
repaired, but the sustained gate remains failed.

All three independent loops were delayed in the same observation intervals.
Mapped audio loops have no file/console/IOCTL call or shared control lock;
maximum heartbeat operation was only 372 us. These facts support a broader
execution delay, without identifying a guest DPC, guest scheduler, host or
hypervisor cause. Host System log has no events in 19:31:50–19:33:25 local;
VBox.log supplies no scheduler trace that distinguishes these possibilities.
Do not blame sleep, relax counters, or change WSL/Hyper-V.

Next objective (VCAB-24/27 diagnostics, VDEV-12 lifecycle): prepare an opt-in,
guest-only scheduler trace around one bounded run, using built-in WPR
GeneralProfile.Light (context switches/ready threads/DPC/ISR). Verify no
existing recorder session, start only after status passes, stop/save the owned
trace before collection even after tone failure, retain partial output and
report startup/stop failure. Host-only fake-recorder tests must cover ownership,
startup refusal and failed-tone cleanup; no host trace or VM run is authorized
by those tests. Trace overhead makes this diagnostic, not release evidence.
Rollback: omit the tracing option/use the retained bounded-tone bundle. Next:
implement and verify the collector before handing off a trace command.

Collector implemented inside the tone step: its traced callback runs only the
redirected child under the existing watchdog; no console output occurs while
trace/audio is active. Save/stop before counter reporting and archive creation.
Use a unique named WPR instance and require guest free space. Host checks:
77 fake-recorder/lifecycle/tone-integration assertions and 11 existing process
regressions pass; scripts parse and host identity guard refuses tracing.
The exported GeneralProfile.Light definition contains the intended scheduling
keywords. No real host recording, VM operation or driver change was performed.
Next: verify the separate diagnostics bundle (same driver/audio executable),
then collect one five-minute guest trace. Root cause/sustained gate stay open.

Scheduling diagnostic artifact prepared from clean `bf03cf26`: 31 manifest
hashes pass; driver files and audio executable are unchanged; copied wrapper
refuses host tracing. Exact identity/hashes are in the review record. Next
action: one 300-second guest trace using the updated retest procedure, with
existing audio settings. No guest trace/continuity result is claimed yet.

### 2026-10-09 19:53: recorder startup unsupported

The copied archive `evidence-20261009-195302.zip` independently matches SHA256
`7B481B854285D656364C44D7E8F03CD669042B4A52936E9FD3DBB1A5B3A0EABE`.
Status passes, but WPR rejects GeneralProfile.Light.File with 0x80070032 in
0.169 seconds. No tone started. Cancellation reports no trace profiles running.
The error does not identify which profile feature or recorder option is
unsupported; listing/exporting a profile and fake lifecycle checks did not
prove runtime compatibility. Sustained audio loss remains unresolved.

Objective: repair diagnostic compatibility without changing driver/audio bytes
or host settings (VCAB-24/27 diagnostics, VDEV-12 ownership). Ordered work:
1. Use a versioned minimal kernel scheduling WPRP (process/thread, loader,
   context switch, ready thread, priority, DPC/ISR), preserving named ownership.
2. Validate its syntax with read-only WPR queries and regress actual profile
   forwarding/startup errors; keep real recorder startup explicitly unverified.
3. Add a guest-only two-second TraceProbe phase that saves the trace and copies
   evidence, without launching audio. Require its reviewed result before a
   new five-minute diagnostic. No global cancel or automatic broad fallback.
4. Prepare a separate checksummed script update; preserve old bundles.
Rollback: use the retained bounded-tone bundle or omit tracing. Risk: narrowed
profile/named instance may still be unsupported; probe fails visibly and
collects exact command output. Next action: implement and check the collector.

Collector/probe implemented. Read-only WPR profile/profiledetails queries parse
and enumerate the minimal events; 128 fake-recorder/probe/integration checks,
11 process checks, script parsing, docs and host guard pass. Actual guest
start/save/event content is still unverified. Failed startup cancellation now
reports any error other than the observed no-profiles code. Probe collection
archives only its own folder, avoiding earlier large audio files. Full checks
and evidence paths are in the packet-clock review. Next: prepare and verify
the separate r2 script update, then review the two-second guest probe.

Separate r2 diagnostic bundle prepared from clean `ee2048a6`: 32 hashes pass,
all 16 driver files and tone executable unchanged; copied host guard and
read-only profile query pass. Artifact identity is in the review record.
Next action: guest TraceProbe only, then inspect its trace and exact errors.

### 2026-10-09 20:03: guest minimal recorder probe passes

Copied archive SHA256
`A3E50C9558066CE4F928341768E8338DA0C0A8A248328F8430949AA588E7019B`
independently verified. Start/save exit 0, no timeout/cleanup failure. Saved
13.6 MB trace decodes: 96,771 events, zero reported lost; ready-thread,
context-switch event type, DPC/ISR, priority and process/image records present.
Newer context-switch payload version is not fully interpreted by host tracerpt;
timing attribution remains pending. This was two seconds without audio leases.
Minimal profile compatibility is now guest evidence; original unsupported
GeneralProfile feature and audio-loss cause remain unidentified. Full command,
counts and limitations are in the review record. Next: one 300-second traced
Tone in the same session/bundle with loop/listener active. Preserve WSL/Hyper-V,
the zero-error gate and all failed-run evidence; no longer or stall test yet.

### 2026-10-09 20:11: five-minute scheduling trace collected

Archive `evidence-20261009-201122.zip` independently matches SHA256
`16A48B2590608BEBC512213FCE092784BF9DCB231E280F28A487C162E01763C2`.
Native process finishes in 300.1581863 s; capture underrun 7,056 frames,
render overrun 14,832. Trace save succeeds and decodes 3,420,594 events with
zero lost. The gate still fails. Review of version-5 context-switch common
fields using Microsoft's parser layout shows long before-ready waits, not
long runnable-queue delays at the maximum worker gap. Largest capture/render
off-CPU intervals 35.6225/35.7156 ms; ready-to-run 30.2/161 us, priority 24.
Two event-silent intervals across recorded live scheduler/DPC/ISR events
(28.443 and 35.2318 ms) coincide with the first and largest loss windows.
Full guest-versus-host attribution is not proven. Do not modify buffers,
counters, driver timer behavior or WSL based on this alone.

Next ordered tasks (VCAB-24/27 diagnostics): retain an offline-only version-5
context-switch reader in source so analysis is reproducible, check its length
guards and compare both raw/converted event counts to tracerpt; record exact
thread/timer and WAV findings; decide what paired host/guest timing evidence
is needed without repeating the unchanged guest-only test. No audio/driver
implementation change is justified yet. Rollback: remove the diagnostic reader;
it never creates/controls tracing sessions or accesses a driver.

Offline reader and review complete: MSVC x64 `/W4 /WX` build, eight prefix/
length checks, actual ETL decoding in converted and raw modes, and existing
output refusal pass. Both decodes match tracerpt's 1,717,691 context switches
and 1,232,133 ready events, with zero rejected/lost. WAV parses as 48-kHz stereo
float, 14,384,640 frames (299.68 s). Largest individual decoded DPC is 2.754 ms;
ISR 2.317 ms. These do not establish the cause of the much longer waits.
Pinned Rust sleep uses a high-resolution waitable timer when available;
fallback use and timer occlusion are not proven. No driver/audio bytes changed.

Next task: design paired host/guest timing collection to distinguish delayed
guest timer delivery from host/VirtualBox execution delay. Prerequisites:
read-only recorder availability/ownership checks, explicit clock alignment,
bounded duration and cleanup, host trace privacy/size limits, short start/save
probe before attended audio. Preserve WSL/Hyper-V and existing evidence.
Do not start another unchanged guest-only run, install host drivers/tools,
change host features, waive counters, or claim VCAB-24/27 closed. User can stop
loop/listener between measurements. Full timings and verification commands are
in the packet-clock review; diagnostic reader rollback removes only its source.

### Paired timing preparation (2026-10-10, user asks for next step)

Objective: correlate host VirtualBox thread execution with the guest's late
worker readiness (VCAB-24/27 diagnostic; VDEV-12 recorder ownership). Ordered
work: (1) prepare an owned, bounded host recorder using the already narrowed
profile and local evidence; (2) verify idle recorder/free space and a two-second
start/save probe before any paired audio; (3) add shared-folder request/reply
clock brackets before and after measurements, preserving clock uncertainty;
(4) coordinate paired recorder lifetimes with explicit timeout/failure markers;
(5) hand off one small paired probe before a reviewed 300-second audio run.
No changes to driver/audio binaries, host security, power, WSL or VM settings.
No new dependency installation. Host trace contains system process/image and
scheduling metadata, stays in the private evidence share and is not committed.
Bound recording duration/disk use; preserve temporary logs on failed save and
stop only the uniquely owned instance. Fake checks validate protocol logic;
real start/save and clock brackets require both machines, not mocks. Rollback:
omit paired collection and retain all prior bundles; do not repeat unpaired
audio. Next action: implement and verify the host recorder probe.

Prepared copy-only probes: HostProbe (two seconds) and PairProbe (no audio),
with sixteen before/after clock brackets, atomic unique messages, peer timeout/
failure propagation, storage stop thresholds and the existing named-recorder
cleanup. Local Windows PowerShell 5.1 checks: 52 protocol/production-callback
checks and 128 fake-recorder lifecycle checks pass. Initial background-job
check needed unsandboxed process IPC; approved fake checks run successfully.
Review corrected a dynamic-scope collision before handoff. HostProbe candidate
invocation refused because the Windows token is not Administrator, before any
real recorder call. No real host or paired trace success is claimed.
Next: commit/push, prepare the source-identified copy-only share update and
independently verify it, then user runs HostProbe in Administrator PowerShell.
Inspect its saved event coverage/loss before any paired probe. No audio test
is requested yet. Exact procedure: virtual-cable-paired-trace runbook.

Copy-only share bundle published from clean `88f6e808`, prepared 03:44:25Z:
`C:\VMs\ar-share\diagnostics-20261010-paired-probe`. Independent verification
passes all 35 hashes; all 32 base files unchanged. Manifest identity is in the
review record. Next user action: HostProbe only in Administrator PowerShell
on the main PC; review the local archive's coverage/loss before PairProbe.

User HostProbe reviewed: archive `paired-6ac9b6ced68f41dc89c99d1861642218-host.zip`
SHA256 `89ACCA58C665D234621A8A20955F2FDC0FB0BE471E993767B38D29B4AEC9316A`
matches. Real host start/save exit 0, profile matches, decoder processes 717,343
events with zero lost. Offline reader matches 232,406 switches / 141,391 ready
events, zero rejected. WPR read-only status is idle. Next: paired two-second
probe with Media Player/listener off; copy guest bundle before starting host
PairProbe, then join from guest within two minutes. Review both traces and
clock uncertainty before any audio phase. Driver/continuity gates stay open.

Paired probe `02d977908cf14e5cbfd31f33eb5103ae` reviewed: both archive hashes
match, host/guest traces decode with zero lost/rejected. Sixteen QPC clock
brackets have a compatible 22.45631-ms offset interval. Guest UTC progresses
3.8900716 s while guest QPC progresses 5.3119743 s between the two tightest
round trips; UTC-only alignment is invalid. This is an observed clock anomaly,
not proof of the audio-loss cause. Do not synchronize/change guest or host
clocks/settings from it. ETW raw QPC is required for paired timing attribution.

Next ordered work (VCAB-24/27 diagnostic): export normalized QPC bounds in
future clock CSVs; extend the paired coordinator with explicit Tone phase,
matching 30/300-second offers and read-only guest status before joining;
retain VM identity guards and the existing native watchdog/zero-counter gate;
save current guest run only, keep large host ETL separate from metadata ZIP;
scale host free-space/stop thresholds for 300 seconds; check actual production
callbacks and argument paths, prepare a separate copy-only update. Hand off
30 seconds first with both recorders, then review clock quality/loss before
300 seconds. No driver/audio binary or timer behavior changes. Rollback: use
retained probe-only bundle; paired traces add overhead and are not qualification.

Paired Tone implemented and reviewed: phase/duration matching, read-only
status before join, unchanged zero-counter/native watchdog gate, normalized
QPC bounds, failure-preserving after clocks/trace saves, current guest evidence
only and separate host ETL. Host budgets: 16 GB initial/8 GB reserve/6 GB
sampled stop. Windows PowerShell 5.1: 88 paired production-boundary checks
and 128 fake recorder checks pass; evidence in packet-clock review. No real
paired audio, build or setting change occurred. Next: commit/push, verify
separate copy-only paired-tone bundle and hand off 30 seconds. Review before
any longer run; remaining gates stay open.

Copy-only paired-tone bundle published from clean `565c9e6d`, prepared
`2026-10-10 04:34:19Z`, under `C:\VMs\ar-share\diagnostics-20261010-paired-tone`.
All 35 hashes independently verify; all 32 base files unchanged. Source pushed
to main. Artifact identity in packet-clock review. Next user action: copy
inside VM, start loop/listener, host Tone 30 then guest Tone 30 when ready.
No paired audio has run yet; review before any longer test.

Paired 30-second run `2efe2c4cb0ae466a8536fbe49ed2ac74` reviewed: both ZIP
hashes match, native exit 0/no timeout, leases stop at 30,004 ms, all counters
and harness sequence gaps zero. WAV parses as 30 s/1,440,000 frames. Host
and guest traces independently decode with zero loss/rejections; QPC bounds
intersect across all sixteen samples within 23.0877 ms. Host recorder idle
afterward. This short diagnostic does not close sustained/audio-quality gates.

New user observation: continuous hiss plus audible beep through Cable B
listener, stopping with the script. Guest has no Audacity. Hold longer/stall
tests. Existing WAV captures Cable A only; there is no direct Cable B sample
to attribute hiss to driver, shared audio engine, listener or VirtualBox/host
playback. Source review of generator, capture queue and sample conversion
does not demonstrate an owning defect from this evidence. Next task: design
and prepare a bounded exact-endpoint Cable B recorder using existing Windows
capture code; refuse default/microphone substitution, preserve packet/format
metadata and compare captured signal to generated 997/47-Hz reference. No
new test or implementation is claimed. Preserve current artifacts and WSL.
## Convergence task — automatic direct audio diagnosis (2026-10-09)

User requests faster convergence after repeated attended failures. The paired
30-second run passes counters, but the user hears continuous hiss through
Cable B. Media Player routed directly to VM speakers plays without hiss or
crackles (user observation, not a digital-path measurement).

- Requirements: VCAB-12/20/24/29, VDEV-12; sustained, latency and signing
  gates remain open. Preserve WSL, host settings and the installed driver.
- Prerequisites: installed `97b393d5` driver, passing active inventory,
  AR-DriverTest guest, local checked bundle and shared Z: evidence destination.
- Decision: measure before another driver change. Add a VM-only user-mode
  source on the exact Cable A Input and recorder on exact Cable B Output.
  Generate 440/660 Hz and retain 997/47 Hz bridge tones. No default endpoint,
  microphone, external player or Audacity dependency. Record packet flags,
  positions and pump gaps. Analyze both WAVs after streaming ends.
- Ordered tasks: implement bounded preallocated probe and offline signal
  analysis; regress clean/noise/drop/repeat/channel failures and VM guards;
  compile and lint on Windows without opening host streams; publish a
  checksummed copy-only update; run one automatic 30-second guest diagnostic.
- Validation: synthetic checks prove diagnostic behavior only. Guest WAVs,
  native counters and timing establish the actual short-run outcome. Hold
  long/stall tests until those samples explain the audible defect. Then use
  the same build for 30 s, 5 min, 10 min and remaining specified gates, with
  a fixed duration/watchdog and current-run-only evidence each time.
- Rollback: stop only owned user-mode processes and use the previous bundle;
  no driver install, settings migration or change to acceptance thresholds.
- Risk: additional WASAPI clients add scheduling work. A diagnostic failure
  is evidence, not automatically a driver defect. First run remains pending.
- Preparation: implemented the safe WASAPI helper, phase/noise analyzer,
  owned-process wrapper and copy-only preparer. Windows host-safe checks pass:
  9 Rust regressions, 75 PowerShell checks, release build, workspace/shell
  Clippy and both format checks. No host audio stream or driver was opened.
  See [preparation evidence](evidence/2026-10-09-m03-direct-audio-preparation.md).
- Published: clean source `c0bc9595`, 37 verified bundle files, all 35 base
  files unchanged, including every driver file. Share folder:
  `C:\VMs\ar-share\diagnostics-20261010-direct-audio`.
- Next action: run the
  [automatic 30-second guest diagnostic](../../operations/virtual-cable-direct-audio.md).
  First real direct recording and waveform review remain pending.

### Direct recorder startup repair — 2026-10-09

Guest archive `direct-4a281d278cde43ada173fb07d2506aea.zip` matches SHA-256
`CA129D76FCA71BD8A42125FA3B73DA5E7ED9A27E162641990CFF61673052E0AD`.
Status passes; recorder exits before readiness with empty stdout/stderr and
no recording. No tone process starts. The new helper imports VCRUNTIME140.dll
and dynamic CRT API sets, unlike the working statically linked VM tools.
This is a confirmed packaging defect; the archive omitted the native exit
code, so the precise guest loader failure cannot yet be confirmed.

Repair before retry: build the helper with `+crt-static` in a separate target
folder; gate packaging on actual PE imports; add an offline startup command
and preserve numeric/hex startup and recorder exits even before readiness.
Regress a missing-runtime exit with empty stderr and verify the shipped
helper's dependency table, host refusal and offline command. Do not install a
redistributable or rebuild/install the driver. Publish a new immutable bundle;
keep this failure and prior artifacts. Requirements/sustained gates unchanged.

Repair complete and pushed as `0e854d7e`. Static build and independent PE
inspection confirm no Visual C++ runtime DLL import. Nine Rust regressions,
103 PowerShell checks (including loader/early-exit failures), formatting,
workspace/shell Clippy and documentation checks pass. R2 is published under
`C:\VMs\ar-share\diagnostics-20261010-direct-audio-r2`; all 37 file hashes
verify, all 35 paired-tone base files unchanged. Next: the automatic guest
procedure with r2; actual direct recording and hiss cause remain pending.

### Direct r2 waveform review — 2026-10-09

Received `direct-4586cab5c56d429c8835b3c0eaa3008c.zip`; its SHA-256 matches
`7F47E3C08F46DF6A3945C60F5731E79CC9FDEE94E530A3F2240BF30A2B554AA4`.
Recorder startup/exit and installed status pass. The native zero-error gate
fails: 12,912 capture underrun and 12,912 render overrun frames, with a
288-ms gap shared by both native workers and the independent direct recorder.
Both active recordings span 29.73 seconds. Offline inspection of 100-ms
windows finds Cable B clean between the pause (residual near float32 rounding,
amplitude 0.25), with one in-signal discontinuity flag and a phase break at
the pause. Continuous hiss is not present in those direct Cable B samples.
Cable A also has separate phase breaks at recorded seconds 12.037–12.050. Cause of
the shared scheduling gap and the downstream audible hiss remains open.

Requirements: VCAB-12/20/24/29, VDEV-12. Before another guest run:
1. Fix the offline analyzer to retain signal metrics when duration fails;
   keep the 29.8–30.2-second gate and all noise/phase thresholds unchanged.
2. Regress shortened/noisy recordings and rerun analysis on both saved WAVs
   on the host without opening audio endpoints. Retain the raw originals.
3. Record duration, packet/phase failures and the distinction between captured
   Cable B samples and Windows Listen/speaker playback. Do not attribute the
   hiss to sample-rate conversion or a specific driver function without data.
4. Review the listening path separately before requesting another audio run;
   do not repeat the unchanged test, rebuild the driver or start a long run.

Validation: focused synthetic regressions, offline reproduction using this
archive, formatting/Clippy/docs/diff checks. No host audio or system settings
changes. Rollback: revert only diagnostic reporting; the current installed
driver and all retained bundles remain unchanged. Next action: reporting
repair and saved-waveform review, not an attended retry.

Reporting repair complete: unchanged duration/noise/phase bounds now retain
per-channel window metrics on short or long active intervals. Ten Rust tests
and 117 orchestration checks pass, including short duration plus packet loss.
The repaired analyzer reproduces failures on both original WAVs offline and
retains their 29.71-second fitted intervals, phase breaks and packet bounds.
See [direct r2 waveform evidence](evidence/2026-10-09-m03-direct-audio-preparation.md#direct-r2-recordings-and-reporting-repair-2026-10-09).
Next: locate the downstream Listen/speaker hiss separately from the measured
shared pause. No new VM audio run is requested by this reporting repair.

### Speaker-format investigation — 2026-10-09

User confirms continuous hiss when replaying the saved Cable B WAV through
Media Player → Speakers, independently of the live bridge tone. Speakers
offers only 16-bit 16/22.05/44.1 kHz; its current setting is 44.1 kHz. Cable B
is stereo 32-bit/48 kHz. Preserve those supported settings; do not force
48 kHz or alter the virtual cable. A five-second clean recorded section
(file seconds 2–7) differs from a generated 997/47-Hz float32 reference by at
most one float32 step (1.49e-8); all 480,000 samples quantize identically to
PCM16. This excludes added continuous digital noise in that section, but
does not identify the playback defect or excuse the separate lost frames.

Next ordered task (VCAB-12/20/24/29): generate offline PCM16 reference files
at 44.1 and 48 kHz, each with both tones, high-only, then low-only sections.
Use a fixed -12-dBFS peak with brief boundary fades; no AudioRouter device,
recording, live host playback or Windows setting change. Regress WAV header,
duration, amplitude, phase/frequency and fade boundaries; publish checksummed
files in a new share folder. User first plays the 44.1-kHz file through the
same VM Speakers. Review which section has hiss before requesting the
48-kHz comparison. Rollback: stop file playback; all driver artifacts and
settings remain unchanged. This is playback triage, not driver qualification.

Reference preparation complete: standard-library generator and three offline
regressions pass for both supported reference rates; initial sandbox temporary
path failure corrected to workspace-owned test storage. New share folder
`C:\VMs\ar-share\playback-reference-20261009` contains both files and verified
checksums. Next: play only reference-44100.wav in the VM and identify which
of the three sections has hiss; no driver test or new setting is requested.
Exact copy/paste procedure is in the direct audio runbook above.

### Independent reference also has audible static — 2026-10-09

User reports continuous static in all three 44.1-kHz PCM16 reference sections.
The file was synthesized independently of AudioRouter. Reinspection of the
published bytes confirms the original SHA-256, correct stereo PCM16/44.1-kHz
header, 749,700 frames, peak 8,192, and zero nonzero separator samples.
793,800 interior samples match the sine equations within 0.5 PCM16 step.
The tone content and supported reference format do not explain continuous
digital noise in the file. Playback-chain cause remains open; do not clear
the separate 288-ms stall/lost-frame/phase-break defects.

Read-only review: current AR-DriverTest configuration is HDA/Windows Audio,
four CPUs, 8 GB. VBox.log opened 2026-10-10T01:21:10Z identifies VirtualBox
7.2.20 r175154, HostAudioWas and default output Speakers (Focusrite USB Audio).
It contains scheduling-hint warnings with implausible printed durations;
those numbers are not measured pauses and do not establish the hiss cause.
No machine, audio endpoint or host setting was changed.

Next ordered comparison (VCAB-12/20/24/29, VDEV-12): stop scripts/playback,
clear Cable B Output's guest Listen setting, confirm Media Player routes to
Speakers and replay the same 17-second reference. Ask for static during tone
sections and silent gaps. This removes the concurrent listener from the
comparison; it does not prove the driver or VirtualBox at fault. Preserve
44.1-kHz speakers and 48-kHz cable settings. Rollback is rechecking Listen
if previously enabled. No live bridge, long/stall run or 48-kHz comparison.

Listener-off result: user reports "a lot less" static, still audible especially
in the first and final parts. The setting affects the audible reproduction,
but does not establish that all remaining noise originates in the same layer.
First/final sections contain 47 Hz, middle only 997 Hz. Next is clarification
of continuous sections versus brief start/stop sounds, plus silent-gap and
middle-section quality. Leave Listen off; do not change formats or request
another driver run based on this subjective reduction.

### Next diagnostic: guest speaker loopback (2026-10-09)

User clarifies several distinct crackles in the early/final portions; the
middle is not perfectly clean either. Timing is approximate, not a measured
number of clicks. Silence quality remains unconfirmed. Do not infer a 47-Hz
or sample-rate defect from this subjective description.

Objective (VCAB-12/20/24/29, VDEV-12): obtain a digital recording of this
independent reference at the guest speaker boundary before changing drivers.
Prerequisites: current reference checksum, Media Player already routed to
Speakers, Cable B Listen off, all bridge tools/playback stopped. Only the
user runs native audio inside AR-DriverTest; host validation is offline.

Ordered tasks:
1. Add a separate 30-second, VM-only speaker loopback mode to the static
   user-mode diagnostic. Select exactly one active Speakers (High Definition
   Audio Device), stereo native 44.1-kHz PCM16 or float32; no default/mic/cable
   fallback, no resampling or endpoint/volume setting change.
2. Preallocate samples/packet metadata before Start; keep file/log work off
   the service thread. Preserve raw native WAV format, packet flags/positions,
   monotonic gaps and endpoint identity. Independent process watchdog applies.
3. Add a wrapper that waits for verified recorder readiness before opening
   the checksummed reference in the existing Media Player, retains startup/
   exit/error evidence, archives only this run and stops only its own child.
4. Verify endpoint/format rejection, WAV serialization, startup/timeout/failure
   paths, static imports, formatting, Clippy and documentation offline. Package
   from a clean identified commit into a new immutable share folder.
5. Give one exact guest command; review the recorded samples before a new
   bridge/long test. Capture completion is not a signal or driver pass.

Boundary: WASAPI loopback observes the guest rendering endpoint mix; it is
not a recording of host/hardware/acoustic output. Clean loopback would narrow
further playback investigation but cannot prove which downstream component
is defective. Source: [Microsoft loopback recording](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording).
Risks: another guest app's audio mixes into the recording; stop unrelated
playback. Rollback: stop this owned helper/file playback and return to the
previous bundle. No driver install/rebuild, host audio, VM configuration,
WSL/Hyper-V, security or qualification threshold change.

Preparation complete: 12 Rust tests, 82 speaker-wrapper checks, existing
117 direct-audio checks, static import gate, formatting, workspace/shell Clippy
and documentation checks pass. Latest speaker fixture evidence:
`target\speaker-tests-70cabb42e15349639510dafc7815eb09`.
Published and independently reverified five-entry manifest in
`C:\VMs\ar-share\diagnostics-20261009-speaker-loopback`, from clean source
`72766a66dab28830644aca6f330270b6b802a375`.
Manifest SHA-256:
`7F899E6AC49C211E32F7EB61E0F2E01738A692ED3D1BFAEA1481860D18B769FC`.
Next: the exact speaker-loopback guest command in the runbook. Real guest
capture and signal attribution remain pending; no driver qualification pass.

### Speaker recording reviewed: noisy sections match exactly (2026-10-09)

The user heard crackles at reference seconds 0–5 and 12–16. Returned archive
`speaker-loopback-0e7000c055ba41d392abb3a902097fca.zip` matches the reported
SHA-256. Native stereo float32/44.1-kHz recording: 1,320,256 frames,
29.937778 seconds, elapsed 30.005337 s, maximum pump gap 15.324 ms.
After aligning by 64 frames, 749,260 consecutive reference frames match
both channels exactly (16.990023 s), including both reported noisy sections
and both silent separators. Last 440 frames of the first reference differ;
subsequent tone sections continue. No full-recording pass is claimed.

Retain 35 discontinuity flags (including startup) and 34 device-position
gaps totaling 15,232 frames. These metadata anomalies occur despite exact
sample agreement in the compared prefix; do not convert them into asserted
sample loss or treat the virtual device position as an independently verified
physical clock. Full checksums, reproduction and limits are in the
[speaker evidence](evidence/2026-10-09-m03-direct-audio-preparation.md#guest-speaker-recording-review-2026-10-09).

Decision (VCAB-12/20/24/29, VDEV-12): investigate the audible reproduction
after this capture boundary separately from the measured bridge loss. Do
not make speculative driver/buffer/rate changes based on the hiss alone.
No acceptance criterion is waived. Installed driver remains `97b393d5`.

Next ordered work: review the installed VirtualBox version's playback/backend
configuration and logs read-only; prepare at most one reversible comparison
with original settings and rollback recorded before a user run. The initial
`VBoxManage showvminfo` query failed with COM E_ACCESSDENIED; only existing XML
and logs have been read successfully. No configuration change was attempted.
Leave Listen off; do not request another driver/long/stall run now. Preserve
WSL/Hyper-V and host settings. Rollback for this review is documentation only.

### Prepared playback backend comparison (2026-10-09)

Read-only review is complete. Official VirtualBox 7.2.20 source archive
matches Oracle's SHA-256 `5c2138213b72f36c129b92c2c267f2a40e9c98513f4c86a584327f09f9be706d`.
`ConsoleImplConfigCommon.cpp:3755–3787` selects HostAudioWas even for
DirectSound on modern Windows unless the `VBoxInternal2/Audio/WindowsDrv`
override prevents that substitution. `VBoxManageModifyVM.cpp:2783–2806`
confirms `--audio-driver dsound` and `default` syntax. Installed binary reports
7.2.20r175154. Current machine XML uses HDA, default driver/WAS, input/output
enabled; machine/global XML have no audio override. Current log confirms
HostAudioWas. COM-dependent agent queries still fail; version query succeeds.
Source files were read only, not built or executed.

Objective (VCAB-12/20/24/29, VDEV-12): compare the same independent reference
through VirtualBox's DirectSound implementation, keeping the guest HDA,
formats, driver and host output unchanged. This tests a playback hypothesis,
not driver qualification. DirectSound still uses Windows audio services;
success would implicate differences between VirtualBox backend paths, not
prove a Windows/Focusrite/driver cause or cure the independent bridge loss.

Ordered tasks: user shuts down guest Windows normally (no saved state or
snapshot restore); guarded host block checks powered-off state and expected
current XML, saves its configuration and sets only this VM's backend plus
override. On failure after the override, remove it; retain saved settings.
User boots normally; agent verifies current log selects DSoundAudio before
requesting a bounded replay/capture. Keep Listen off and no cable tests.
Record subjective crackles and raw waveform metadata; preserve failed gates.
Rollback, with VM off: restore driver `default` and remove the machine
override. Original default/WAS state is checked before and after. No global
override, host endpoint, WSL/Hyper-V, security, power or controller change.
Exact commands and source links are in the
[backend comparison runbook](../../operations/virtual-cable-direct-audio.md#prepared-next-step-virtualbox-playback-backend-comparison).

User applied the guarded host block successfully. Original configuration:
`C:\VMs\ar-share\vbox-audio-before-336a68538bfa437aa8167432a0de8729.xml`.
Read-only verification confirms current HDA/DirectSound and machine override
`VBoxInternal2/Audio/WindowsDrv=dsound`. No VirtualBoxVM process is running;
the existing log ends with powered-off shutdown and belongs to the earlier
HostAudioWas session. Next: user starts VM normally, then verify the fresh
startup log selects DSoundAudio before requesting reference playback.
