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
collect block in the updated [VM guide](../../../operations/virtual-cable-vm-guide.md#retry-after-the-2026-10-07-endpoint-enumeration-crash).

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
[VM guide](../../../operations/virtual-cable-vm-guide.md#retry-after-the-2026-10-07-endpoint-enumeration-crash).
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
the [retry guide](../../../operations/virtual-cable-vm-guide.md#retry-after-the-2026-10-07-endpoint-enumeration-crash).
Remain at Session 1 until two clean smoke runs pass. Rename/persistence and
app compatibility must subsequently pass their own VM gates.
