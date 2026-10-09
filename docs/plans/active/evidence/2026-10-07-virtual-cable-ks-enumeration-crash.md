# Virtual cable KS enumeration crash and repair — 2026-10-07

## Reproduction and diagnosis

The first attended `smoke` run on VM `AR-DriverTest` installed the test-signed
package, but found zero AudioRouter endpoints. Windows then restarted after
bugcheck `0x3B` (`SYSTEM_SERVICE_EXCEPTION`), access violation at
`ks.sys+0x5c81a`. The user copied
`C:\Windows\Minidump\100726-6484-01.dmp` to the host shared folder as
`C:\VMs\ar-share\100726-6484-01.dmp`.

WinDbg 10.0.28000.2526 loaded the private AudioRouter PDB for the driver image
(timestamp 2026-10-07 19:01:15). The stack passed the global
`audioroutervirtual!CableStreamDataRanges` (11 pointers in the crashed image)
through KS range-processing code; `ks.sys` dereferenced a null pointer. The
mini dump did not contain the static table's data pages, so it cannot identify
the precise bad entry by memory inspection.

Source inspection found ten `KSDATARANGE_AUDIO` entries with
`KSDATARANGE_ATTRIBUTES` set, but the pointer table had only one
`PinDataRangeAttributeList` entry at its end. Microsoft's Sysvad sample places
an attribute-list pointer immediately after **each** attributed format range
([Microsoft sample range table](https://github.com/microsoft/Windows-driver-samples/blob/main/audio/sysvad/TabletAudioSample/micarray2wavtable.h#L3220-L3260)).
The mismatch was a real descriptor-table defect and was corrected. However,
the second VM replay still bugchecked during device start with the corrected
table present, so this defect alone does not explain the crash and no fix is
verified yet.

## Repair

- Interleave `PinDataRangeAttributeList` after every PCM and float range in
  `CableStreamDataRanges`.
- Add `C_ASSERT(SIZEOF_ARRAY(CableStreamDataRanges) == 20)` and an acceptance
  guard that rejects the former one-attribute-at-end layout.
- Add a one-time VM retry procedure to
  `docs/operations/virtual-cable-vm-guide.md`, using a fresh package under the
  existing VirtualBox shared folder.
- Fix `vm-checks.ps1 -Step collect` to close its transcript before archiving;
  the earlier collector tried to zip the open transcript and failed with a
  sharing violation. Add a host-safe VM guard regression for that ordering.

Requirements/scenarios: VCAB-02, VCAB-10/11, VDEV-09; Stage A A2/A3.

## Host checks and package

Environment: Windows host, Windows SDK/WDK 10.0.28000.0, Visual Studio 2026.
The default 32-bit-host linker failed with `LNK1101` because the available
PDB helper directories had inconsistent versions. Selecting the native x64
tool architecture and putting the matching 14.51.36256 tools first in the
process-local `PATH` fixed the build without modifying the machine environment.

- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\drivers\audiorouter-virtual\tests\build-tests.ps1` — passed, 215 checks.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m03-driver-build.ps1 -Platform x64` with process-local `PreferredToolArchitecture=x64` — passed; WDK build, signability/catalog checks, no errors or warnings.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m03-driver-vm-guards.ps1` — passed, 30 checks, including PowerShell syntax and transcript-close-before-archive regression.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\docs.ps1` — passed, 132 Markdown files and 707 local links; `git diff --check` passed.
- Required Jev review ran; no `jev/` rules apply to the changed files.
- `tools/vm/prepare-vm-share.ps1 -Share C:\VMs\ar-share\repair-20261007-r2 -Version 0.1.0` with the same process-local tool selection — completed; test-signed package integrity passed 33 checks; helper, tone/inventory tools, fuzzer and corrected VM scripts staged; manifest written.

No driver was installed or loaded on the host. No host boot, Secure Boot,
audio, or certificate-store settings changed. The test-signed package was
staged at `C:\VMs\ar-share\repair-20261007-r2`.

## Remaining gate

### Second VM replay (2026-10-07)

The user restored a clean test-signing state, copied the repaired package to
`C:\ar`, and preflight passed all 15 checks (`20261007-203258-preflight`).
The smoke attempt bugchecked again. Collection initially hit a transcript
sharing violation; the user replaced `vm-checks.ps1` with the staged
collector fix, reran collection, and copied
`C:\ar\evidence-20261007-204252.zip` to the shared folder. The collector
summary confirms both evidence archiving checks passed. Archive:
`C:\VMs\ar-share\evidence-20261007-204252.zip` (137,040 bytes).

The archived `setupapi.dev.log` shows the repaired package from
`C:\ar\repo\drivers\audiorouter-virtual\x64\Release\vm-smoke-f9f948c08d0c42548720dfcd3ffe1809`
was imported as `oem5.inf`; Windows configured `ROOT\MEDIA\0000` and began
`Starting device 'ROOT\MEDIA\0000'` at 20:39:39.439. It has no subsequent
device-start completion record. The smoke transcript ends with “The pipeline
has been stopped,” and its runner summary has no completed checks. The new
dump is `100726-9796-01.dmp` (463,926 bytes). WinDbg reports another
`SYSTEM_SERVICE_EXCEPTION` 0x3B / access violation in `ks.sys+0x5c81a` while
the stack references `audioroutervirtual!CableStreamDataRanges`. The *host
shared package* has the 20-entry interleaved table (SYS SHA-256
`F0051DCAF02FF8C5BB068697DE88B52DD79B0D75C7E8BD8867761DD2197AA57C`) and
package metadata `builtAt=2026-10-08T03:18:08Z`. The second dump's x64 call
arguments include the table address at driver RVA `0xCCD0` and a following
count of `0xB` (11), which matches the old table shape. The archive's earlier
runner summary also reports a package built at `02:01:16Z`, before the repair.
Because the second runner crashed before writing its own package summary and
the mini dump lacks the table data pages, we cannot yet prove which package
the second attempt loaded. A stale VM package is now the leading explanation;
the prior statement that the repaired table was loaded in the VM was too
strong. The host-shared artifact is repaired, but the VM deployment must be
verified before another smoke run.

Do not rerun smoke, tone, or Verifier yet. Next, boot the VM and perform a
read-only identity check of `C:\ar\driver\package.json` and
`audioroutervirtual.sys`; require the metadata and SHA-256 above before
proceeding. If they match, check the install runner's exact package source and
Windows Driver Store selection before testing again. If they do not match,
restore the clean checkpoint and recopy the repaired share. The two clean
checkpoint smoke runs remain required after the repaired binary is confirmed
to load and start without a bugcheck.

### Repaired package VM replay (2026-10-07)

The user confirmed the repaired package identity in `C:\ar\driver`:
`builtAt=2026-10-08T03:18:08.0549566Z`, SYS SHA-256
`F0051DCAF02FF8C5BB068697DE88B52DD79B0D75C7E8BD8867761DD2197AA57C`,
`RepairedPackageMatches=True`. Preflight passed all 15 checks at
`20261007-211034-preflight`. Smoke no longer bugchecked, but the helper still
timed out after 30 seconds with zero of four expected endpoints. Its runner
summary confirms the repaired package metadata, A1 baseline passed, and A14
removed the package and restored the baseline. The archive is
`C:\VMs\ar-share\evidence-20261007-211354.zip` (64,276 bytes).

In `setupapi.dev.log`, Windows imported this run's package as `oem5.inf`,
configured `ROOT\MEDIA\0000`, and recorded `Starting device completed` at
21:11:44.514. The helper began cleanup at 21:12:14.965 and successfully
removed the device and package. Collected PnP inventory after cleanup contains
only the VM's High Definition Audio devices. This proves the repaired driver
starts without the prior crash in this replay; it does not explain why Windows
did not enumerate its four audio endpoints. No AudioEndpointBuilder,
Kernel-PnP, or Code Integrity event logs were included in this archive.

Next: do not rerun smoke. From an elevated PowerShell in the VM, export the
enabled Audio, Kernel-PnP, and Code Integrity events around 21:11–21:12 to
`C:\VMs\ar-share\m03-endpoint-events.txt`, then send that file's contents.
Use the exact interval and logs from this run; if the VM clock has rolled over
to another date, preserve the original event timestamps when querying.

The user supplied those events from `m03-endpoint-events.txt`. Kernel-PnP
records show four `SWD\MMDEVAPI` endpoint devices (two capture and two render)
configured with parent `ROOT\MEDIA\0000` and started at 21:11:45. Audio
Operational has MMDevAPI state-change events at 21:11:45 and 21:12:14. The
only Kernel-PnP events shown are these successful configure/start records and
the expected device deletions at 21:12:15; Code Integrity has no events in
the interval. This confirms Windows created the four PnP endpoint children,
although the helper's MMDevice enumeration/classification returned zero.
The event output itself omits each endpoint's FriendlyName; the direct PnP
inventory below later captured those names.

The user then performed that direct diagnostic install. `Get-PnpDevice`
showed four healthy AudioRouter endpoints, but their friendly names were two
`Speakers (AudioRouter Virtual Audio Device)` render endpoints and two
`Line (AudioRouter Virtual Audio Device)` capture endpoints. The VM's two
built-in endpoints were also healthy. The helper expects unique names
`AudioRouter Cable A/B Input/Output`, so `classify_endpoint_name` rejected the
four AudioRouter devices and reported zero. The helper's remove command then
returned exit code 0 and removed `oem5.inf`; the VM is back to its clean
baseline.

The INF defines per-cable `szPname` and interface `FriendlyName` strings, but
those strings did not become the MMDevice friendly names. Windows exposed the
generic driver `DeviceDesc` plus the topology pin labels instead. This
confirms the current failure is the endpoint naming/registration path (and
its name-based helper detection), not missing PnP endpoint children or a
driver-start failure. VCAB-02 requires the per-cable names. Microsoft
documents that endpoint names come from KS bridge-pin
names/categories and the attached audio adapter's interface friendly name;
speaker endpoints use the fixed `Speakers` label ([friendly
names](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/friendly-names-for-audio-endpoint-devices),
[endpoint builder](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-endpoint-builder-algorithm)).
The source currently has `NULL` for both cable topology bridge-pin Name GUIDs
and supplies only a generic `DeviceDesc`; the next host investigation was to
correct the per-cable naming path. That change and a new test-signed package
are now recorded below; VM validation remains pending.

### Endpoint-name implementation and package (2026-10-08)

Attempted to provide the per-cable identity at the audio interface property layer in
`drivers/audiorouter-virtual/Source/Filters/minipairs.h`. Each cable's render
and capture adapter now has its own friendly-name string, and that value is
registered for both the topology and wave `KSCATEGORY_AUDIO` interfaces.
The INF root description is now `AudioRouter Virtual Cable`. The helper
still requires the exact Cable A/B Input/Output identity. VM validation is
required to show whether Windows uses these properties. Updated spec §5.5 and the VM
retry instructions accordingly.

Host checks on 2026-10-08, Windows x64, Visual Studio 18 / WDK 10.0.28000.0:

- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m03-driver-build.ps1 -Platform x64` — passed; no driver load/install.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m03-driver-vm-guards.ps1` — passed, 30 checks.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\docs.ps1` — passed, 133 Markdown files and 717 local links.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\drivers\audiorouter-virtual\tests\build-tests.ps1` — passed, 215 checks.
- Jev working-tree review — passed; no `jev/` rules apply to changed files.
- `git diff --check` — passed (Git emitted only a CRLF-to-LF normalization warning for the INF).
- VM package script passed all six steps, including package integrity checks (33 checks). No driver was loaded on the host.

Package staged at `C:\VMs\ar-share\repair-20261008-endpoint-names`.
`package.json`: version `0.1.0`, driver `0.1.0.0`, built
`2026-10-08T04:32:59.5971206Z`, x64 Release, test-signed, dirty working tree.
`audioroutervirtual.sys` SHA-256:
`0706772A3A26A2827534E2C0C6455061EA27FB022D7D89B5844A995E07596954`.
The VM run is pending; do not report endpoint naming or smoke as runtime
passed until the user provides VM evidence. Run the copy/preflight/smoke/
collect block in the updated [VM guide](../../../operations/virtual-cable-vm-guide.md#retry-the-endpoint-naming-check-2026-10-08).

### Endpoint-name replay and diagnostic capture (2026-10-08)

The user ran the `repair-20261008-endpoint-names` package. Preflight passed
all 15 checks. Smoke again timed out after 30 seconds with zero of four
matching endpoints; collection passed and produced
`C:\VMs\ar-share\evidence-20261007-214724.zip` (64,261 bytes). The runner
summary confirms the new driver package (`builtAt=2026-10-08T04:32:59.5971206Z`,
commit `0683ed694fa70abdd1689c5daac794802c1dc42b`), and SetupAPI shows that
package was staged from the per-run folder as `oem5.inf`. The archive contains
no endpoint name inventory because the helper failed before the runner reached
its post-install inventory, then cleanup removed the PnP devices. This run
therefore does not reveal whether Windows used the new interface friendly
names or whether endpoint children were absent.

The runner now snapshots present `AudioEndpoint` PnP device names to
`endpoint-names-on-install-failure.json` immediately when helper install
fails, before cleanup. The updated diagnostic package is staged at
`C:\VMs\ar-share\repair-20261008-endpoint-diagnostics`. Package integrity
passed (33 checks); VM guards passed (31 checks); Jev found no applicable
rules. The next VM replay should use the updated block in the VM guide and
return the resulting ZIP. Use its endpoint names and instance IDs to select
the naming layer for the driver correction; do not repeat the interface-only
change without that evidence.

The user ran that diagnostic package. Its ZIP,
`C:\VMs\ar-share\evidence-20261007-220030.zip` (64,261 bytes), contains the
pre-cleanup inventory. Four AudioRouter endpoints were present and healthy:
two `Speakers (AudioRouter Virtual Cable)` render devices and two
`Line (AudioRouter Virtual Cable)` capture devices. The only other listed
AudioEndpoint was the VM microphone. Thus the previous interface-property
change reached the driver and the root description changed, but neither cable
identity nor direction appeared in endpoint names. The failure is specifically
the bridge-pin naming path, not absence or health of PnP endpoint children.

The corrected host implementation now assigns a unique custom pin-category
GUID to each of the 16 bridge pins (A–H, render/capture), registers each
category name in the root device software key through the INF generator, and
uses a per-cable topology descriptor. This follows Microsoft's documented
custom pin-category name lookup. It removes the ineffective per-interface
friendly-name override while retaining the WaveRT packet-size constraint.
Custom categories may change Windows' generic endpoint icon/form-factor; the
VM run must verify render/capture direction and app visibility as well as
names.

Host verification for this revision on Windows x64:

- `tests/acceptance/m03-driver-build.ps1 -Platform x64` — passed WDK compile, INF verification, and catalog signability; no install/load.
- `tests/acceptance/m03-inf-gen.ps1` — passed 80 interfaces, 16 unique pin categories, deterministic output, and invalid/duplicate manifest rejection.
- `drivers/audiorouter-virtual/tests/build-tests.ps1` — passed 215 checks.
- `tests/acceptance/m03-driver-vm-guards.ps1` — passed 31 checks.
- `tests/acceptance/docs.ps1` — passed 133 Markdown files and 719 local links.
- Jev working-tree review — no applicable rules; `git diff --check` passed with only Git's CRLF normalization warnings.

The x64 test-signed package is staged at
`C:\VMs\ar-share\repair-20261008-pin-categories`. Package integrity passed
33 checks; no host driver install/load occurred. Metadata:
`builtAt=2026-10-08T05:07:54.0862144Z`, driver `0.1.0.0`, x64 Release,
test-signed, dirty working tree. SYS SHA-256:
`ABFF3407D940308136B9598AEFFFCE46067B1AD0D38287F0EC5B02D1196B495E`.
The next action is a clean-checkpoint VM smoke run using the updated
[VM guide](../../../operations/virtual-cable-vm-guide.md#retry-the-endpoint-naming-check-2026-10-08).
Endpoint naming, endpoint direction/app visibility, and smoke remain unverified
until the VM result arrives.

### Review requested before proceeding (2026-10-08)

Scope: pending bridge-pin categories, descriptor/INF wiring, native helper,
runner cleanup/evidence and copy/paste retry. Requirements VCAB-02, VDEV-03,
A2/A3/A14. Manual source review found and repaired:

1. **Name classifier mismatch:** Rust rejected
   `AudioRouter Cable A Input (AudioRouter Virtual Cable)` while the VM
   PowerShell check accepted it. A new shared fixture failed against the old
   helper with `None` instead of `(0, Input)`, then passed after the correction.
   Rust and PowerShell now consume the same 15 accepted/rejected fixtures.
2. **Rename property:** the helper attempted to write
   `PKEY_Device_DeviceDesc`, which supplies the category identity. It now
   writes `PKEY_Device_FriendlyName` and reads the unchanged category
   description to recognize renamed cables. This follows the separation of
   [endpoint properties](https://learn.microsoft.com/en-us/windows/win32/coreaudio/device-properties)
   and [category descriptions](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/friendly-names-for-audio-endpoint-devices).
   Actual Windows rename/persistence remains unverified.
3. **Stale endpoint count:** despite its name, the display inventory enumerates
   `DEVICE_STATEMASK_ALL`. The native helper now filters by active MMDevice
   state and verifies actual render/capture flow. A pure Windows metadata
   regression excludes stale, wrong-flow and missing-state records.
4. **Failure evidence:** collecting endpoint names could replace the original
   install exception if writing the diagnostic failed. The runner preserves
   the install ErrorRecord, reports secondary collection errors separately,
   and writes a valid empty JSON array when there are no endpoints.
5. **Interactive retry:** separate pasted commands could continue after a
   terminating error in an earlier command. New `tools/vm/retry-smoke.ps1`
   runs the sequence as one script, refuses an unidentified host, copies to
   `C:\ar`, verifies all manifest hashes, stops before smoke if preflight
   fails, and collects/copies evidence even when smoke returns failure.

The category implementation follows the documented custom GUID registration
path, and the constructor returns each miniport's own topology descriptor.
The review did not establish endpoint creation, Windows form factor, app
compatibility or runtime stability; those still require the VM.

Verification on the Windows x64 host:

- `cargo test --locked -p audiorouter-driver-helper --lib` — **28 passed**.
  First full run failed filesystem rename operations in sandbox temp storage;
  rerun with process-only `TEMP`/`TMP` set to
  `C:\code\audiorouter\target\review-temp` passed. No system environment change.
- `cargo clippy --locked -p audiorouter-driver-helper --all-targets --all-features -- -D warnings` — passed; compiler reported only incremental-cache hard-link fallback warnings.
- Both repository Rust formatting check commands — passed.
- `tests/acceptance/m03-driver-vm-guards.ps1` — **274 passed** in source and staged share (pure checks and read-only default-role enumeration).
- `tests/acceptance/m03-driver-build.ps1 -Platform x64` — passed WDK build/INF/catalog checks; no host install/load.
- `drivers/audiorouter-virtual/tests/build-tests.ps1` — **215 passed**; host copy timing is not kernel DPC evidence.
- `tests/acceptance/m03-inf-gen.ps1` — passed.
- `tests/acceptance/docs.ps1` — passed, 133 Markdown files / 719 links.
- `tools/vm/prepare-vm-share.ps1 -Share C:\VMs\ar-share\repair-20261008-reviewed -Version 0.1.0` — passed all six stages, including **33 package integrity checks**.
- Independently reread and verified **29 staged manifest hashes**; staged helper matches the newly built static-CRT helper.
- `git diff --check` — passed, with only CRLF normalization warnings.
- Jev: **not completed**. The local attempt selected 57 applicable rules but
  failed to reach its API. Automatic approval review rejected a network retry
  because it would transmit the source diff externally. Explicit user approval
  for that transmission has been requested; no workaround was attempted.

Reviewed share: `C:\VMs\ar-share\repair-20261008-reviewed`, x64 Release,
test-signed, dirty source tree, driver version `0.1.0.0`, built
`2026-10-08T05:19:21.1527719Z` at HEAD
`0683ed694fa70abdd1689c5daac794802c1dc42b`.

- SYS SHA-256: `2455BBDAFC7D7E90860F3266AEE82F58EE3DA83D3902C6577FFA8A3CB2DD4F4E`.
- Helper SHA-256: `11886DF1C15D58D4CE59B3B4D981BE6893E3B9D67D4344910FB00B86F70FD735`.

Next: restore the clean test-signing snapshot and run the single command in
the [retry guide](../../../operations/virtual-cable-vm-guide.md#retry-the-endpoint-naming-check-2026-10-08).
Remain at Session 1 until two clean smoke runs pass. Rename/persistence and
app compatibility must subsequently pass their own VM gates.

## 2026-10-08 18:44: speaker pin Names remain overridden

Evidence archive: `C:\VMs\ar-share\evidence-20261008-184523.zip`, SHA-256
`2F92448CDA2555FDE495647D0CADD04FB1CB4F9D081ECB34078207253723A515`.
Package: clean commit `06e594575aee63a399d68b36212677450c710901`, built
`2026-10-09T01:41:09.3969489Z`. Preflight passed 15/15. Smoke's A1 passed,
install timed out at two recognized endpoints, and A14 restored the baseline.
Collection passed and copied the ZIP automatically.

The inventory captured before cleanup contains all four healthy project
endpoints: two render devices named `Speakers (AudioRouter Virtual Cable)`
and two captures named `AudioRouter Cable A/B Output (AudioRouter Virtual
Cable)`. The helper correctly rejects the anonymous render names. Thus this
is an endpoint identity failure, not proof that the driver created only two
devices. It disproves the previous candidate's assumption that a custom pin
Name overrides the speaker endpoint name. Microsoft explicitly documents
the [speaker naming exception](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-endpoint-builder-algorithm).

Next candidate uses an analog connector plus the unique render pin Name and
the [documented default-enable property](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/pkey-audiodevice-enableendpointbydefault)
with render mask `0x00000101`. Its EP association matches the analog category.
Capture policy is unchanged. The earlier analog candidate lacked this enable
property. A new read-only COM snapshot records every MMDevice's flow/state
before cleanup, so visibility failures can be diagnosed directly. Build and
static checks do not prove names, active state or runtime safety; two clean
A1-A3/A14 runs are still required. Use the updated
[retry guide](../../../operations/virtual-cable-vm-guide.md#retry-the-endpoint-naming-check-2026-10-08).

Candidate `repair-20261008-render-enabled` was built from clean commit
`5f61df3c367a6774fb829393b5458444889cad45` at
`2026-10-09T01:51:01.3638446Z`, using the already-trusted VM test certificate.
SYS SHA-256: `54B7ABB3730550A8F2698C00F9850ACC5A3612F09D9B25318D82D3C6A1297362`.
Verification: INF generator and x64 WDK acceptance passed; source and staged
VM guards passed 275 checks; package integrity passed 33; all 29 manifest
file hashes matched; docs passed 133 Markdown files/721 links; diff check
passed. Build/staging performed no host driver load/install or trust/boot
changes. Jev remains blocked on its previous source-export approval rejection.

## 2026-10-08 18:57: first clean smoke pass

Archive `C:\VMs\ar-share\evidence-20261008-185852.zip`, SHA-256
`857FAC1C7EDA67744CC8D4F764D8043E00B396B553A7FFA0AC40CFF0D0AA29ED`.
The package matches clean source commit `5f61df3c`, built
`2026-10-09T01:51:01.3638446Z`. Preflight passed all 15 checks; runner A1,
A2, A3 and A14 passed. Collection produced the archive successfully.

The pre-cleanup endpoint inventory contains exactly Cable A/B Input/Output
with suffix `(AudioRouter Virtual Cable)`, all status OK. Input InstanceIds
use the render flow prefix `{0.0.0.00000000}`; Output uses capture prefix
`{0.0.1.00000000}`. Default roles stayed unchanged. Uninstall restored driver
store, present devices and default roles to the baseline. This confirms the
combined naming/default-enable change for one clean install/remove cycle.
Required two-run gate is **1/2**. Restore the clean snapshot and repeat the
same candidate; audio quality, formats and Verifier remain unqualified.

## 2026-10-08 19:20: two clean smoke cycles passed

Second archive `C:\VMs\ar-share\evidence-20261008-192120.zip`, SHA-256
`B04D885E35FC2A4C4EE0AA0696DEBC1D2637DAA6321316B338B98F2941FF766E`.
Package identity and build time match the first successful run. Preflight
passed 15/15; A1/A2/A3/A14 passed. Before removal, all four project endpoints
again had the required Cable A/B Input/Output names with adapter suffix and
status OK. InstanceId flow prefixes confirm Input is render and Output is
capture. Default roles stayed unchanged; removal restored the full baseline.

The required clean-snapshot install/name/remove gate is **2/2 passed**.
The naming repair is now supported by two actual Windows VM cycles. The
successful package remains the candidate for Session 2: install and retain
it, inspect the 60-format/engine-period inventory, then run tone qualification.
Rename, IDs across configuration/restart, Verifier, security, audio quality
and release signing remain separate gates.

## 2026-10-08 19:25: bridge open crashed Session 2

Evidence archive `C:\VMs\ar-share\evidence-20261008-192858.zip`, SHA-256
`A910CA7F93ED584A949EACFDFC8BD86B05CACF870B84529F251B7D85066D0C34`;
guest minidump `100826-7703-01.dmp`, SHA-256
`DB6735B6720AE13BC96DB2CBB2225C0E8202F694E08245FC8B4FF3C42808A0C2`.
These are distinct from the earlier host crash. The install transcript reports
four named endpoints and a successful idempotent second install. No status
transcript survived. Guest System event 1001 confirms bugcheck 0x3B/c0000005.

Local CDB analysis with matching Microsoft symbols (`target/vm-1925-crash-symbols.txt`)
places the fault at `portcls!AcquireRemoveLock+4`, dereferencing null + 0x178,
through `portcls!DispatchCreate`, `nt!NtCreateFile`; process `audiorouter-dr`.
The minidump lacks device-object/IRP memory, so those cannot be inspected.
Source confirms bridge handlers were assigned before `PcInitializeAdapterDriver`,
which replaces dispatch entries as documented by
[Microsoft](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/portcls/nf-portcls-pcinitializeadapterdriver).
The standalone bridge object has no PortCls device extension. This explains
the observed null-extension device-open failure; formats were not qualified.

Repair: register CREATE/CLEANUP/CLOSE/DEVICE_CONTROL wrappers after PortCls
initialization and publish the bridge device afterward. Each wrapper handles
only `g_BridgeControlDevice`; other device objects are forwarded to
`PcDispatchIrp`. Regression checks pin initialization/publication ordering
and both audio forwarding paths. SEC-08 and VCAB-11 remain blocked on a new
guest install/status run. The old package's two smoke passes qualify naming
and uninstall only; smoke never opened the bridge. Rollback is the clean
guest snapshot, after preserving the evidence above.

Host validation: Windows x64 WDK `tests/acceptance/m03-driver-build.ps1`
passed with the dispatch regression guards (`target/bridge-dispatch-build-check.txt`);
`tests/acceptance/docs.ps1` passed 133 Markdown files/723 local links;
`git diff --check` passed. Jev was not run because automatic approval review
previously rejected uploading the source diff to that external service.
Guest runtime verification of the repair is pending.

Replacement staged at `C:\VMs\ar-share\repair-20261008-bridge-dispatch`,
built `2026-10-09T02:32:39.0185028Z` from clean commit
`151a3b6959dadbad784153ab8017969ffc367b03`. Test-signed Release x64;
package integrity passed 33 checks and all 29 manifest hashes matched.
SYS SHA-256: `5A641E0CA21171BDF4318DBF1972BE92D1BC7D18F3BC8600F952D9B39135E8B6`.
The staged package retains the existing VM certificate. Next: clean snapshot,
copy/hash verification and install; review output before the separate status
command. No tone or Verifier until the bridge-open crash gate passes.

## 2026-10-08 19:36: bridge open passed; 8-channel layout mismatch

Archive `C:\VMs\ar-share\evidence-20261008-193649.zip`, SHA-256
`D871004B16A04AAF6B19AAE25C8A1269EF6F24E63C4784132FCA8137BD82E341`.
Preflight passed 15 checks; install passed three including its idempotent
repeat. Helper status opened the bridge and returned installed, protocol 1.1,
capabilities 63 and all four correctly named endpoints without a crash.
The original bridge-open failure is repaired for this guest run.

Inventory JSON contains all four endpoints: each accepts 48/60 formats, and
all 12 rejected combinations are 8 channels at every rate/encoding
(AUDCLNT_E_UNSUPPORTED_FORMAT, 0x88890008). Every minimum shared period is
128 frames at 48 kHz, default/max 480, fundamental 1. VCAB-11 remains failed.
The WaveRT table used `KSAUDIO_SPEAKER_7POINT1` (0xFF), while inventory asks
for `KSAUDIO_SPEAKER_7POINT1_SURROUND` (0x63F). The format validator compares
channel masks exactly. Microsoft documents the former as obsolete in
[header changes](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/header-file-changes).

Repair: change the driver's 8-channel table to the surround mask, assert the
SDK value 0x63F and guard agreement with the user-mode probe. Clarify VCAB-11
speaker positions without changing the 60 required combinations. Stereo and
the other channel counts retain their formats. Existing test installs should
be replaced from the clean snapshot; old wide 7.1 requests no longer match.

Also repair inventory invocation under Windows PowerShell 5.1: the native
tool writes both PASS and FAIL summaries to stderr; redirected stderr under
`Stop` aborted the wrapper before saving inventory.txt or checking exit code.
Temporarily use Continue only around that native call, restore the preference
in finally, and use its exit code. `m03-vm-inventory-output.ps1` passes native
exit 0/1 fixtures with stdout/stderr retained and preference restored; it
executes only the wrapper and never a driver or audio stream. Evidence:
`target/inventory-output-d84f83a883c34fe49a1fe84d5e2bad01`.
WDK x64 acceptance, including layout and dispatch guards, passed
(`target/surround-layout-build-check.txt`); docs passed 133 files/723 links;
diff whitespace passed. Jev remains unrun due to the earlier automatic
approval rejection of external source-diff upload. New VM format validation
and tone qualification remain pending.
