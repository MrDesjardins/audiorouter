# M03 driver and signing track (AudioRouter virtual cable)

**Status (2026-10-05): reopened by the user (DEC-18).** Goal: ship
AudioRouter's own signed virtual cable with the app so users do not need to
install VB-Cable. Lowest-cost signing (see "Lowest-cost strategy" below).
Development and loading tests happen only in an isolated test-signed VM.
The EV certificate purchase and the first public driver release each need a
separate user go-ahead. VB-Cable/Voicemeeter stay supported. Execution
status is in the [active plan](../active/current.md); work packages in the
[virtual cable plan](../active/virtual-cable.md); design in
[17 Virtual cable](../../spec/17-virtual-cable.md).

## Previous status (2026-09-19, superseded by DEC-18)

Status: set aside indefinitely by explicit user cost decision on 2026-09-19
(originally deferred by scope decision on 2026-09-17). This is a business
decision, not a temporary technical blocker: the user does not have budget
for a production code-signing credential, and this project will not pursue
one. Do not describe this track as "in progress," "next," or "pending" in any
other document; it is set aside until the user explicitly reopens it with a
funded signing plan.

AudioRouter-owned virtual endpoints require a production-signed, Secure-Boot/
Memory-Integrity-compatible Windows driver. That signing and trusted
installation path costs money (an EV/driver-signing credential and,
typically, a Microsoft hardware dashboard submission) that is not available.
The project's supported and complete virtual-routing strategy is instead
binding to already-installed third-party virtual endpoints — VoiceMeeter
Banana and VB-Cable — as ordinary `physicalInput`/`physicalOutput` graph
nodes, exactly like a physical device. This is not a workaround or a partial
substitute for a future managed bus; it is the product's permanent virtual-
routing design for this project. See
[the "Existing virtual input/output" library entries](../../spec/09-interface.md)
and [06-virtual-devices.md](../../spec/06-virtual-devices.md) for the
supported path.

Resume only if the user later supplies budget and explicit instruction for a
production signing route and credentials, a signed x64 package, an isolated
qualification machine or equivalent approved test environment, and an agreed
rollback plan. Then resume M03 loaded-driver/PortCls bridge, endpoint
lifecycle, identity, ownership, recovery, and clean-machine evidence before
claiming M08 release readiness. Setting this track aside does not waive
VDEV-01 through VDEV-12 or SEC-08 as normative requirements of a future
signed track; it does mean they are explicitly **not** part of this
project's v1 completion target, per the user's 2026-09-19 decision.

## Completion gap analysis (2026-10-05, user request)

Written at the user's request to list what replacing VB-Cable with
AudioRouter-owned virtual devices would take. The user then reopened the
track the same day (DEC-18 below). Facts about Microsoft programs were read on
2026-10-05 and must be rechecked before any purchase or submission.

### What already exists

| Area | State | Evidence |
| --- | --- | --- |
| Driver source | SysVAD "Simple Audio Sample" derivative, pinned upstream commit, MS-PL; endpoints `AudioRouter - Desktop In` (render) and `AudioRouter - Voice Chat` (capture); own GUIDs, INF with `SWD\AudioRouterVirtual` match | [driver README](../../../drivers/audiorouter-virtual/README.md) |
| Build | x64 and ARM64 WDK build, package/catalog signability pass | `tests/acceptance/m03-driver-build.ps1`; [rebaseline log](../archived/2026-09-17-vb-cable-rebaseline.md) |
| Kernel bridge | `bridgeio.h` protocol 1.0: open/close/heartbeat IOCTLs, direction-aware leases, bounded mapped block (≤2 ch, ≤4096 frames); both WaveRT directions call the bridge helpers | `Source/Inc/bridgeio.h`, `Source/Main/minwavertstream.cpp` |
| User-mode bridge | `NativeBridgeInputWorker` / `OutputWorker` / `DuplexWorker`, realtime writer, stale-data → silence; 88 bridge tests | `crates/windows-audio/src/lib.rs` |
| Provisioning seam | Software Device API create/close (max 8), probed once on 2026-09-17, cleanup verified | `crates/windows-audio/src/software_device.rs`, `m03-swdevice-probe.ps1` |
| Control plane | `virtualDevices.list/plan/provision/remove`, bus registry and routes persisted; provisioning returns `unavailable` | `crates/control/src/lib.rs`, `m03-virtual-buses.ps1` |
| Install wrapper | `manage.ps1` preview/install/uninstall with recorded `oem*.inf` and rollback | driver README |
| UI | Virtual render source / capture sink library entries, shown as unavailable | `ui/src/library.ts` |

### Why it stopped

1. **Signature (hard blocker).** On 2026-09-17 `pnputil` rejected the
   package: `No signature was present in the subject`. Windows 11 with
   Secure Boot loads a kernel driver only with a Microsoft signature. The only
   local certificate is a developer `WDKTestCert`.
2. **No isolated test machine.** Test-signed loading requires test-signing
   mode, which must not be enabled on the daily workstation (SEC-08,
   VDEV-09). HVCI evidence could not be read on that host
   (`Win32_DeviceGuard` unavailable).
3. **Cost decision.** The user set the track aside on 2026-09-19 because the
   signing route costs money.

Blockers 1 and 3 affect **distribution only**. Blocker 2 also blocks
development, but it can be removed without spending money (see phase 1).

### Signing routes to decide between

| Route | What it needs | Notes / risks |
| --- | --- | --- |
| A. Attestation signing (Partner Center) | Hardware Developer Program account; an **EV** code-signing certificate to register and to sign the submission CAB | Windows 11 client loads attestation-signed drivers; they cannot go to retail Windows Update, so the app installer ships them. Microsoft's page now says "for testing purposes only"; a Microsoft engineer wrote in March 2026 that nothing is immediate for attestation but that requiring HLK for everything is being studied. Main policy risk of this route. |
| B. WHCP / HLK certification | Route A's account and EV certificate, plus an HLK test lab (controller + test client) and passing audio HLK tests | The only route Microsoft documents for retail Windows Update publication. Much more test effort. |
| C. Use an already-signed third-party driver | License and redistribution rights; its endpoints used as ordinary devices | No kernel bridge, so this is the current VB-Cable design with a different cable. Example lead: `VirtualDrivers/Virtual-Audio-Driver` ("Signed" release via SignPath); its issue #23 reports Code 52 on most Windows builds, so its signature must be verified before relying on it. VB-Audio redistribution licensing is another option to price. |
| D. Stay on VB-Cable, improve guidance | Nothing new | Keeps today's permanent design; see "cheaper improvements" below. |

SignPath Foundation (the app-signing lead in
[release signing](../active/release-signing-and-publication.md)) produces
Authenticode signatures; on its own that is not the Microsoft kernel
signature Windows requires. Azure Artifact Signing is not accepted for
Hardware Program registration (Microsoft Q&A, 2026).

### Cost detail (prices read 2026-10-05, USD unless noted, before tax)

**One-time and yearly costs, route A (attestation):**

| Item | Cost | Notes |
| --- | --- | --- |
| EV code-signing certificate | ~$280–$450 per year | Sectigo via resellers $280–$410/yr (multi-year prepay is cheaper per year); SSL.com $349 for 1 year, down to $149/yr on a 5-year prepay; Certum €329 (token) or €379 (cloud). Since late February 2026 a code-signing certificate is valid at most ~459 days, so multi-year products need yearly reissues. |
| Key storage | $0–$120 | EV keys must be in a FIPS 140-2 Level 2 token or cloud HSM. Shipped tokens: Sectigo resellers $80–$120. Cloud signing (Certum cloud, SSL.com eSigner) avoids hardware but may cost more per year. |
| Legal identity | $0 to a small registration fee | EV normally requires a registered organization. SSL.com sells a **Sole Proprietor EV** that it says validates an individual and gives Hardware Dev Center access. Otherwise register a sole proprietorship/business (fee depends on country/province). |
| Hardware Developer Program | $0 | No fee stated; a free Microsoft Entra ID tenant is created during registration. Needs a legal contact email, company lookup (D-U-N-S optional), the EV certificate and a business questionnaire. |
| Attestation submissions | $0 | Each driver release: CAB signed with the EV certificate, uploaded, Microsoft-signed result returned. |
| Test VM | $0 | Hyper-V on Windows 11 Pro; test signing inside the VM only. |
| Clean test PC (optional) | $0 if a spare PC exists | Needed for the Secure Boot + Memory Integrity qualification; a Hyper-V Gen2 VM with Secure Boot can cover part of it. |

**Totals.** First year about **$300–$600**; then about **$280–$450 per
year** to keep a valid certificate for driver updates. The same EV
certificate is an Authenticode certificate, so it can also sign the app
and installer: this would replace the separate app-signing decision
(Azure Artifact Signing, about $9.99/month, or SignPath Foundation) in
[release signing](../active/release-signing-and-publication.md).

**Route B (WHCP/HLK) adds:** no Microsoft fee, but an HLK environment
(Microsoft provides a free VHLK virtual-machine image for the controller;
a test client is also needed), a passing audio playlist on each targeted
Windows version, and resubmission for each release. Main cost is time
(weeks for a first pass), not money. Required only for retail Windows
Update or if Microsoft ends attestation for retail use.

**Lowest-cost strategy (user goal 2026-10-05: minimum spend).** Microsoft's
[code-signing requirements](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs)
say the account's EV certificate "must be valid at the time of
submission"; the returned driver carries Microsoft's signature, not the EV
certificate. So the EV certificate is needed only in years when a driver
release is submitted, not every year:

1. Keep the kernel driver small and stable (all processing already lives in
   the user-mode engine), so driver releases are rare.
2. Buy a 1-year EV certificate, register, submit every driver build needed
   in that window, then let the certificate lapse. Renew only when a new
   driver release is required (a new EV purchase also means re-validating
   identity and associating the new certificate).
3. Sign the app with the free SignPath Foundation route, not with the EV
   certificate, so app releases never depend on it.
4. Fund the purchase through GitHub Sponsors or similar before buying.

Timing: buy the certificate only after stage A of the
[test procedure](../../operations/virtual-cable-testing.md) passes in the
VM. One certificate (valid up to ~15 months) then covers the signed
package, stage B/C/D tests, the release and the first driver fixes. App
updates never need it; later driver updates need a new certificate only if
the old one has expired.

Result: about $300–$450 per driver release window instead of every year;
$0 in years without driver changes. Risks: Microsoft could revoke a
signature (for example, after a vulnerability) or end attestation for
retail use; an urgent driver security fix would need a new certificate
first, which takes days of validation. Confirm with Microsoft support
that an expired EV certificate does not affect already signed drivers
before relying on this.

**Route C** costs depend on the vendor: VB-Audio does not publish a
redistribution price (ask them); a free open-source signed driver costs
nothing but must be verified to load.

**Route D** costs nothing.

### How VoiceMeeter and SteelSeries Sonar do it

Windows Update is not how these drivers reach users, and is not what makes
them load. Two separate things are involved:

1. **Trust: a Microsoft signature.** The vendor submits the driver to the
   Partner Center hardware dashboard (attestation or WHCP). Microsoft
   returns it signed by "Microsoft Windows Hardware Compatibility
   Publisher". That signature, not the delivery channel, is what Windows 11
   with Secure Boot checks. Since the April 2026 update, Windows 11
   24H2/25H2/26H1 also stops trusting the old self-signed "cross-signed"
   drivers, so a Microsoft signature is the only general path.
2. **Delivery: the vendor's own installer.** The setup program (run as
   administrator) adds the signed package to the driver store
   (`pnputil`/SetupAPI) and creates the device. VB-CABLE's setup must be
   "run in administrator mode" and asks for a reboot. Sonar's virtual
   device is root-enumerated (`ROOT\VEN_SSGG&DEV_0001`,
   `steelseries-sonar-vad.inf`) and installed by SteelSeries GG, with a
   separate `steelseries-sonar-vad-extension.inf` extension package.

They are companies (VB-Audio Software; SteelSeries ApS) with Partner Center
accounts and EV certificates. Small publishers do the same: the AudioRelay
virtual microphone, published under an individual's name, shows
`Signer Name: Microsoft Windows Hardware Compatibility Publisher` and
`Attributes: Universal Attested` in `pnputil /enum-drivers`, i.e. an
attestation-signed driver shipped by its own installer.

Whether VB-Audio and SteelSeries use attestation or full WHCP was not
confirmed from public pages. To check on the user's PC (read-only):
`pnputil /enum-drivers /class MEDIA` and read `Signer Name` and
`Attributes` for the VB-Audio and Sonar entries.

Lesson for SEC-08: in 2026 Voicemeeter and VB-Audio Matrix drivers received
CVEs (CVE-2026-23762 to -23764) for mapping non-paged pool into user space
and for handle-attribute flaws. AudioRouter's bridge also maps a block to
user mode, so its security review (phase 2, step 6) must cover exactly
these cases; a vulnerable signed driver can be added to Microsoft's
vulnerable-driver blocklist.

### Work remaining, by phase

**Phase 0 — Decisions (user).** Done 2026-10-05: DEC-18, route A with the
lowest-cost strategy. Still open: first-release cable count and install
placement (defaults under DEC-18 below); the test machine (phase 1).
Recheck Microsoft requirements before buying (VDEV-09).

**Phase 1 — Isolated test environment (no cost).** A Hyper-V VM or spare PC
with test signing enabled **inside it only**, test certificate trusted
there, snapshot before each install. Script it in `tests/acceptance/` and
record the rollback (revert snapshot). This unblocks all of phase 2.

**Phase 2 — Driver completion (in the VM).**
1. Load the existing package; prove both endpoints enumerate and the bridge
   carries a tone each way (loaded-driver/PortCls qualification).
2. Remove sample leftovers from the realtime path: tone generator, diagnostic
   file writer; update `DriverVer` (still `02/22/2016`).
3. One bus per software device: per-bus friendly names, rename, up to eight
   buses (VDEV-01, VDEV-03). Today the INF exposes one fixed pair.
4. Persistence: endpoints stay enumerated when AudioRouter is closed
   (`SwDeviceSetLifetime` parent-present or an INF-installed root device);
   capture yields initialized silence with no owner (VDEV-04).
5. Ownership and recovery: one writer lease per capture endpoint, heartbeat
   timeout → silence within 500 ms, buffers cleared on owner change and user
   switch, version negotiation (VDEV-07, VDEV-12).
6. Security (SEC-08): bridge device security descriptor, IOCTL length and
   handle validation, fuzz tests, Driver Verifier, Code Analysis/CodeQL
   (also required for WHCP).

**Phase 3 — Product integration.**
1. Control: make `virtualDevices.provision/remove` real behind a scoped
   elevated helper; normal routing stays standard-user (VDEV-08, NFR-16).
2. Engine: wire the bridge workers into the multi-path engine as Virtual
   Render Source / Virtual Capture Sink nodes; enable them in the UI library.
3. Installer: the app installer is per-user (DIST-01); the driver needs a
   separate elevated install, upgrade and uninstall that removes only its own
   package and keeps sessions (VDEV-10, DIST rules).
4. First-run: offer "Create AudioRouter devices" or keep VB-Cable; never
   change Windows defaults.

**Phase 4 — Qualification.** The test cases in
[06-virtual-devices.md](../../spec/06-virtual-devices.md#test-cases): three
buses, rename, reboot identity, silence before startup, kill → silence
≤500 ms, one hour of render with no reader and no memory growth, two
readers, user switch, cycles, upgrade/rollback. VDEV-11 in Discord and OBS
(app noise suppression on and off). NFR-02 latency compared with VB-Cable.
Secure Boot + Memory Integrity on a clean machine.

**Phase 5 — Signing and release.** Chosen route; signed package in the
release artifacts with checksums; release notes and known issues.

### Documents to update when the track is reopened

- [15-delivery.md](../../spec/15-delivery.md): DEC-18 superseding DEC-16 for
  the driver; traceability rows for VDEV-01/03/09 and SEC-08.
- [06-virtual-devices.md](../../spec/06-virtual-devices.md) "Current scope
  decision"; [M03 milestone](../../milestones/M03-virtual-routing.md)
  ordered steps 1–2; [M08](../../milestones/M08-release.md) driver gates.
- The status paragraph of this file; [future README](README.md); the active
  plan (new section with steps, validation and rollback).
- User docs once shipped: README install section, quickstart, privacy guide,
  release notes, `examples/setups`.

### Cheaper improvements that need no driver

If the user keeps route D: detect missing VB-Cable at first run and link to
its installer; one-click Discord/OBS templates; name the cable roles in the
UI ("CABLE Input = Discord microphone"); support multiple VB-Audio cables
(A/B) in templates. These fit the current scope.

### DEC-18 (accepted by the user 2026-10-05)

> Reopen the AudioRouter virtual cable and ship it with the app to remove
> the VB-Cable install step. Route A (attestation) with the lowest-cost
> strategy. Phases 1–4 in an isolated test VM; buy the EV certificate only
> after phase 4 passes there (user go-ahead); ship only after the signed
> package passes on a clean Secure Boot + Memory Integrity machine.

Open choices, with proposed defaults:

- **First release scope (decided 2026-10-05):** up to eight cables,
  "AudioRouter Cable A" to "Cable H" (each a playback + recording pair, 16
  Windows endpoints); the user enables the first 1–8, default 2. Develop
  and test with 2; qualify all 8 before paying for signing.
- **Where the install is offered:** first-run guide and Setup, as an
  optional step with a Windows elevation prompt; the per-user app installer
  stays standard-user. Uninstalling the app does not remove the driver
  (DIST-02); a separate "Remove AudioRouter cable" action does.
- **License:** the driver derives from Microsoft's MS-PL sample. Shipping
  its binaries is allowed; keep `LICENSE-MS-PL.txt` with the package and
  its source available under MS-PL, and list it in third-party notices.

### Sources (read 2026-10-05)

- [Driver signing offerings](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings)
- [Attestation signing](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-attestation)
- [Driver code-signing requirements](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs)
- [Hardware Program and Azure Trusted Signing (Q&A)](https://learn.microsoft.com/en-us/answers/questions/5866910/hardware-program-verification-using-azures-trusted)
- [VirtualDrivers/Virtual-Audio-Driver issue #23](https://github.com/VirtualDrivers/Virtual-Audio-Driver/issues/23)
- [Register for the Hardware Developer Program](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/hardware-program-register)
- [The Windows Driver policy (April 2026)](https://support.microsoft.com/en-us/windows/hardware/drivers/the-windows-driver-policy)
- [Removing trust for the cross-signed driver program](https://techcommunity.microsoft.com/blog/windows-itpro-blog/advancing-windows-driver-security-removing-trust-for-the-cross-signed-driver-pro/4504818)
- [OSR thread on attestation after the policy change](https://community.osr.com/t/advancing-windows-driver-security-the-windows-driver-policy/60086)
- [SSL.com Sole Proprietor EV](https://www.ssl.com/products/software-integrity/code-signing/ev-sole-proprietor/), [SSL.com EV prices](https://secure.ssl.com/certificates/ev-code-signing/buy), [Sectigo EV reseller](https://signmycode.com/sectigo-ev-code-signing), [Certum EV](https://shop.certum.eu/certum-ev-code-sigining-code.html), [Certum validity change](https://www.certum.eu/en/news/shortening-code-signing-certificate-validity/)
- [VB-CABLE install notes](https://vb-audio.com/Cable/index.htm); [Sonar VAD driver listing](https://treexy.com/products/driver-fusion/database/sound-video-and-game-controllers/steelseries/sonar-virtual-audio/); [AudioRelay driver signer output](https://community.audiorelay.net/t/virtual-mic-driver-does-not-install/1482)
- [CVE-2026-23762 (Voicemeeter drivers)](https://nvd.nist.gov/vuln/detail/cve-2026-23762)
