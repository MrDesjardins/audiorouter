# M03 driver and signing track (AudioRouter virtual cable)

**Update (2026-11-21, user): the one investment goes to the driver.** SignPath
Foundation declined on 2026-11-16 (reputation requirement; its program does not
cover driver signing at all). The owner is in the USA and chose that any spend
should target the audio driver. The EV certificate bought for driver attestation
also signs the app and installer, so it replaces the separate app-signing
purchase (see [release signing](../active/release-signing-and-publication.md)).
Microsoft Artifact Signing cannot sign the driver. The purchase gate is
unchanged: no certificate until the pre-purchase assurance plan and Stage A on the
current driver package pass in the isolated VM (see
[virtual-cable.md](../active/virtual-cable.md)); the app-only release stays the
supported path meanwhile. Open before buying: whether a sole-proprietor EV
certificate gives Hardware Dev Center access in the owner's name, and which cloud
signer lets CI sign the app without exporting the key. Recheck all prices.

## Purchase plan: cheapest route to a signed driver (research 2026-11-21)

Planning only. Nothing was bought, enrolled or submitted. Prices were read on
2026-11-21 from the pages linked in the sources below, in USD before tax unless
marked; vendor pages are marketing, so every "it works for X" claim below is a
question for the vendor in writing before paying. Where a page could not be read
or was silent, that is said.

### Answers in short

| Question | Answer (confidence) |
| --- | --- |
| Which CAs does Hardware Dev Center accept? | Microsoft's list: Certum, DigiCert, GlobalSign, IdenTrust, Sectigo, SSL.com ([code-signing-reqs](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs), updated 2026-08-26). High. |
| Who sells an EV certificate to an individual? | Only **SSL.com Sole Proprietor EV** says so plainly ("no registered business entity required"). Certum's EV pages say "organization data" but price a 2-year warranty for "individual customers" (unclear). Sectigo/DigiCert resellers target organizations; SignMyCode's FAQ says EV is for organizations active three years. SSL.com's own 2026-10 guide says EV "may only be issued to businesses and other registered organizations, not to individuals", which conflicts with its sole-proprietor product. **Medium-low until SSL.com answers in writing.** |
| Identity validation? | SSL.com: ID check, callback to a listed phone number, agreements; standard 3-5 days, expedited +$599. D-U-N-S is not mentioned on its pages. Exact documents are on a page not read. |
| Does Partner Center need an organization? | Microsoft's registration page ([updated 2026-08-13](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/hardware-program-register)) never mentions individuals; every step says "company", "organization" and a legal contact. D-U-N-S is optional (a manual entry exists), a free Entra ID tenant is created if needed, and an EV certificate is required for approval. Whether a sole proprietor passes review is **not documented**. Forum reports of blocked enrollments and slow manual review exist (2026). |
| Token or cloud? | Both are offered. SSL.com warns a YubiKey "may not meet" EV requirements for Microsoft HLK driver signing and says to confirm before buying (its 2026-10 guide lists YubiKey 5.7.4 or Thales SafeNet for 3072-bit keys). Microsoft's pages say nothing about key storage. eSigner cloud is the path SSL.com names for kernel drivers. |
| Can CI sign the app? | eSigner: yes in principle (CKA with signtool, CodeSignTool, a GitHub Actions guide exists; not read). A hardware token cannot run on a hosted runner. The release tooling signs locally first anyway (see [release signing](../active/release-signing-and-publication.md)), so a token on the owner's PC is workable. Certum cloud: 5,000 signatures per month, CI use not verified. |
| WHQL or attestation? | Attestation is the plan (below). WHCP is the fallback and costs time, not money. |

### Prices (2026-11-21)

| Option | Certificate | Key storage | Notes |
| --- | --- | --- | --- |
| **SSL.com Sole Proprietor EV** | $359 (1 yr); $308.74/yr (2); $254.89/yr (3); $226.17/yr (4); $201.04/yr (5) | eSigner cloud: Tier 1 $100/month (10 signings) or $900/yr (120 signings, $75/month), first 30 days unlimited at no cost, credits lost if the subscription lapses. YubiKey +$379, HLK caveat | Validation included. Up-front billing for multi-year is not stated. Each certificate lasts at most 458 days, so a multi-year term means yearly reissues (re-validation unclear). |
| Certum EV, card set | from EUR 359 gross | card and reader included (CC EAL6+) | Page says it is trusted by Microsoft for the Hardware Program. Individual eligibility not stated. 459-day cap. |
| Certum EV, cloud (SimplySign) | from EUR 379 gross | cloud, FIPS 140-2 level 3 | Same wording. 2- and 3-year products include free reissues. USD store (certum.store) not read. |
| Sectigo EV via SignMyCode | $389.99 (1 yr); $335.99/yr (2); $279.99/yr (3, $839.97 total) | token from the CA +$80 US; multi-year only on your own HSM | FAQ points to organizations only. Not suitable without a registered business. |
| DigiCert EV via SignMyCode | $559.99 (1 yr only since 2026-02-15) | KeyLocker or token | Highest price; organization-oriented. Not read: DigiCert's own page figures were unlabeled. |
| GlobalSign, IdenTrust | not researched | | Accepted by Microsoft; both target organizations. |

### Cheapest multi-year option

Multi-year prices fall (SSL.com 5 years $201/yr, about $1,005 if billed up front),
but they only pay off if the driver changes in several years. The 458-day cap
forces a reissue each year anyway, and the existing lowest-cost strategy (a
certificate is needed only in a year with a driver submission) is cheaper for a
driver that rarely changes. **Buy 1 year.** Revisit a 3-year term ($254.89/yr)
only if releases are planned each year and SSL.com confirms reissues need no new
fee or token.

### Attestation versus WHCP (WHQL), and the unsigned path

- **Attestation** ([driver signing offerings](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings), updated 2026-04-14): "works only on Windows 10 Desktop and later"; the account needs at least one EV certificate and the returned driver is Microsoft-signed, so it loads with Secure Boot and Memory Integrity on. Limits: "for testing purposes only" wording, not publishable to Windows Update for retail, not "Windows Certified", not accepted for Windows Server 2016+ device drivers. We distribute it in our own installer, as small vendors do (AudioRelay's pnputil output shows "Universal Attested"; see above).
- **Policy risk is real but unresolved.** Microsoft's [Windows Driver Policy](https://support.microsoft.com/en-US/Windows/Hardware/Drivers/the-windows-driver-policy) says new kernel drivers must be signed through WHCP, blocks cross-signed drivers (April 2026 update; audit first, enforcement after 250 hours and 3 boots) and says nothing about attestation. A secondary report of tougher 2027 rules (December 2026 WDK) could not be read (HTTP 403) and is **unconfirmed**. Microsoft engineers said in March 2026 that attestation is not ending immediately (earlier in this file). Treat "attestation stops being enough for general distribution" as the main risk, and ask Partner Center support in writing before buying.
- **WHCP/HLK fallback:** same account and certificate; adds an HLK controller and test client (free VHLK image) and passing audio tests on each target Windows version. Weeks of work, no Microsoft fee. It is the only route Microsoft documents for retail Windows Update.
- **Unsigned or test-signed drivers** load only with test signing on, which needs Secure Boot off (and Memory Integrity off for most test drivers). That path stays VM-only (SEC-08, VDEV-09) and is never an install route for users; it also fails anti-cheat games. Users without the signed driver keep VB-Cable (Route D); nothing changes for them.

### Total cost

| Scenario | First year | Ongoing |
| --- | --- | --- |
| **A. SSL.com Sole Proprietor EV + eSigner, driver only.** Do registration and the first submissions inside the free 30 days | **$359** (add $100 if Partner Center approval overruns the trial) | $359 per driver-release year; $0 in years with no driver change |
| **B. A, plus signing the app and installer from the same certificate** (about 5 files per release; eSigner Tier 1 = 10 signings a month) | $359 + $900/yr annual plan = **$1,259** | $1,259/yr |
| **C. SSL.com certificate + YubiKey, signing locally, if SSL.com confirms the token is accepted for HDC** | $359 + $379 = **$738** | $359/yr (token reused after reissue) |
| **D. Certum card set (card and reader included), if Certum confirms individual EV and HDC use** | about EUR 359 (about $415 at current rates; confirm in USD at certum.store) | about EUR 359/yr |
| **E. Register a business first, then any CA** | Option cost plus state fee (not researched) | |

Within DEC-18's estimate ($300-450 per driver window) for driver-only use (A or D);
the app-signing request pushes A to B unless a token is acceptable (C or D).
Expedited validation (+$599) is never worth it. Legal contact email and a free Entra tenant cost $0. No
liability insurance cost is budgeted (agreement §15(g); see the signing guide).

### Risks

1. **Identity.** A sole proprietor EV may issue yet Partner Center may refuse an individual, or SSL.com may require a registered sole proprietorship or DBA. Mitigation: written answers first; fallback is registering a business (cost varies by state, not researched) and a possibly new certificate.
2. **Key storage mismatch.** The token SSL.com ships may be rejected for HDC signing. Mitigation: ask before buying; use eSigner for the driver window.
3. **eSigner cost creep.** App signing makes eSigner a $900/yr line; credits are lost when it lapses. Mitigation: keep it driver-only (A), or get a token.
4. **Attestation policy drift** (2027 claim above). Mitigation: written Microsoft answer; HLK fallback; keep the driver small so a re-submission is cheap.
5. **Revocation or an urgent security fix** needs a valid certificate at submission time; an expired one means days of re-validation. Not documented whether expiry affects signed drivers (it should not; confirm).
6. **SmartScreen.** EV no longer gives instant reputation, so the app may still warn at first; signing helps Defender false positives but does not guarantee them.
7. **A kernel defect found in Stage B/C** costs a new RC and resubmission, free within the certificate's ~458 days.

### Recommended order of steps (each gated)

0. **Now, $0, nothing bought.** Send two written questions (see below) to SSL.com and Partner Center support; ask Certum the same two. Do not buy before the answers.
1. **Pre-purchase gate (existing, unchanged):** the [assurance plan](../active/virtual-cable.md) tasks 1-4 and Stage A on the frozen RC pass in the VM; owner decision (task 5).
2. **Owner go-ahead to spend**, then buy the 1-year Sole Proprietor EV and finish identity validation (3-5 days). Prepare the Entra tenant and the Partner Center form beforehand.
3. **Start the eSigner trial only when the certificate is issued**, so the 30 free days cover registration, the Signablefile.bin signing, the CAB signature and any resubmission ([signing guide](../../operations/virtual-cable-signing.md) Parts 2-6).
4. **Stage B/C** with the Microsoft-signed package. Only then decide the app-signing key storage: eSigner annual, token, or keep releases unsigned for another cycle.
5. **App signing:** add `-Sign` per [release signing](../active/release-signing-and-publication.md) after the key location is settled.

### Recommendation (revised 2026-11-21 after the Partner Center follow-up: see the follow-up below; ask OSSign first, buy nothing before written answers)

Original recommendation, still valid as route 2 if OSSign declines: **Buy SSL.com's Sole Proprietor EV, 1 year, with eSigner used only inside its free first 30 days for the driver (scenario A, about $359-$459 all in), but only after SSL.com and Partner Center confirm in writing that an individual can register and that the key storage is accepted, and after the existing pre-purchase gate passes.** Defer the app-signing key decision (token about $379 once versus eSigner $900/yr) until the driver is signed. If SSL.com or Microsoft says an individual cannot register, register a sole proprietorship or LLC before buying anything.

### Follow-up (2026-11-21, owner): Partner Center and individuals

Owner inputs: the owner's legal name is acceptable as the public publisher, and
the plan is **one driver signing** (the kernel driver is not expected to change).

**Finding: no source shows an individual, or even a sole proprietor, getting
through Partner Center, and the failure mode has no appeal path.**

- Microsoft's registration page (updated 2026-08-13) and attestation page
  (2026-04-14) assume an organization: "your organization's Microsoft Entra ID
  global administrator", a legal contact "on behalf of your organization", a
  questionnaire, and an EV certificate "required for your request to be
  considered". They never mention individuals, rejection, appeal or a support
  path. A Microsoft Q&A moderator wrote on 2026-06-04 that registering "requires
  a company account (Entra ID) and a valid EV (Extended Validation) Code Signing
  Certificate" ([thread](https://learn.microsoft.com/en-us/answers/questions/5903879/process-to-submit-files-for-signing)).
- Reported cases (all unresolved in the threads):
  - [2025-07, sole proprietor](https://learn.microsoft.com/en-us/answers/questions/4371800/registration-blocked-for-hardware-developer-progra):
    registered business, GlobalSign EV, company domain and Entra tenant, no D-U-N-S
    (manual entry). An automated "Microsoft runs on trust" check blocked
    registration **before the EV upload step**. Vetting support asked for traces,
    then answered "unable to reactivate your access" with no reason; a second user
    got the same reply in 2025-10.
  - [2026-09, QFC company](https://techcommunity.microsoft.com/t5/partner-compliance-verification/hardware-enrollment-blocked-requesting-manual-review/m-p/4554412):
    D-U-N-S matched, EV held, still blocked; a Community Manager said "there is no
    pathway to route a case, you have to go thru the support ticket portal".
  - [2026-10, security vendor](https://techcommunity.microsoft.com/discussions/partnercomplianceforum/to-enroll-in-microsofts-windows-hardware-developer-program-for-kernel-driver-att/4564879):
    status "Rejected", no "Fix now", no Hardware workspace to open a ticket, no
    reply after about two months.
  So rejection is not specific to individuals: even companies are blocked, and
  the three-case sample is of complaints, not a measured rate.
- The trust check can fire before the certificate is uploaded, so **the EV
  certificate cannot be used to test enrollment, and the $359 is spent first.**
  SSL.com's refund terms were not read; ask.
- Counter-evidence: a small publisher's driver (AudioRelay, shown earlier in this
  file) is Microsoft-attested under an individual's name, so individuals have
  succeeded at some point. The date, certificate and account type are unknown.
- The signature users see on an attested driver is Microsoft's, so the owner's
  legal name appears only as the INF provider and inside Partner Center.

**A cheaper route for a single signing: OSSign (unconfirmed).** OSSign is a
Swedish open-source signing initiative (GitHub org
[OSSign](https://github.com/ossign), contact@ossign.org, backed by
Scheibling Consulting AB, Cloudyne Systems and Clysec; small, 44 followers). Its
listing says "Signed binaries (EV) and attestation signed drivers via
Microsoft" for one driver project, usbip-win2. Another project's issues
([#5](https://github.com/ciroiriarte/wnbd-dist/issues/5),
[#8](https://github.com/ciroiriarte/wnbd-dist/issues/8)) say OSSign submits on
the project's behalf with its own Hardware Program certificate (so no own EV or
Partner Center account) and cite "more than 6 months of public history" as the
eligibility bar. That is a third party's account: ossign.org's own terms, cost,
publisher name and turnaround could not be read, and OSSign has not been
contacted before 2026-11-21. **The owner emailed contact@ossign.org on 2026-11-21**
(eligibility, driver attestation, app signing, process, publisher name). An
automatic reply the same day gave ticket **991393** (Cloudyne support system) and
said responses take 3-5 business days but a large backlog is slowing them; no
human answer yet. The owner also sent the pre-sales questions to SSL.com on
2026-11-21 (eligibility, Hardware Dev Center acceptance, key storage, refund if
enrollment is refused, eSigner after the trial, reissue, timing); no reply yet. If it accepts the driver, the driver costs $0 and avoids the
enrollment risk. Risks: eligibility mirrors SignPath's reputation test; a small
volunteer-style service is a continuity and trust dependency (it would handle
the CAB and the RC); the first release would be signed by OSSign's account, not
the owner's.

**What one signing changes for cost.** Certificate $359 plus at most one
eSigner month ($100) if approval overruns the trial: **$359–$459**, after
which the certificate can lapse (a lapsed certificate only blocks new
submissions; confirm already-signed drivers stay valid). A multi-year term and
the token have no value for the driver alone. The EV purchase then buys the
app/installer signing only if it is kept alive (token $379 once, or eSigner
$900/yr; unchanged from scenarios B and C), so decide that separately and
consider Artifact Signing ($9.99/month) for the app instead.

**Routes, best first for a single signing:**

| Route | Cost | Main risk | Next action |
| --- | --- | --- | --- |
| 1. OSSign sponsorship | $0 | Eligibility (history, community), terms unknown, third-party dependency | Owner or agent emails contact@ossign.org with the project, license, repo and driver scope; no purchase |
| 2. Own EV (SSL.com Sole Proprietor) + Partner Center as an individual | $359–$459 | Automated trust block with no appeal; certificate cost sunk | Written questions to SSL.com (acceptance and refund) and a Partner Center support ticket before buying |
| 3. Register a business, then EV and Partner Center | state fee plus $359+ (not researched) | A business was blocked too in one report; company name replaces the personal one | Only if route 2 is refused |
| 4. Stay on VB-Cable (Route D) | $0 | Install step stays for users | Always available |

### Questions only the owner can answer

1. Will the owner register a business (state fee, a business address and phone listing become public in the certificate) if Partner Center refuses individuals? Or is "no business" firm?
2. ~~Is the legal name acceptable as the public publisher name?~~ Answered 2026-11-21: yes.
2b. Is the owner willing to let OSSign (a third-party, small, Sweden-based initiative) handle the signed submission, with the signature coming from its Hardware Program account? And willing to wait for its eligibility window (a third-party report says six months of public history)? Is a $359 spend acceptable with a real chance of enrollment refusal and no refund?
3. Does signing the app and installer justify $379 (token) to $900/yr (eSigner) beyond the driver, given that SmartScreen reputation is not instant?
4. How many driver releases are expected in the first two years? That decides 1-year versus a multi-year term.
5. Is a signed driver still wanted if Microsoft answers that attestation is for testing only and WHCP/HLK is needed (weeks of lab work, no extra money)?
6. Is the owner willing to do the identity validation call and ID upload now-ish (3-5 days) and keep a government photo ID, proof of address and a listed phone number to hand?
7. Insurance (agreement §15(g)): any intent to buy cover?

### Not verified (read before acting)

- SSL.com's accepted ID list and whether it needs a sole-proprietor registration; its GitHub Actions eSigner guide; whether multi-year is billed up front.
- Certum in USD, its individual eligibility and CI use.
- Windows Report's 2027 claim (HTTP 403), Microsoft's own announcement link, and whether an expired certificate affects signed drivers.
- No Partner Center registration was attempted; whether a sole-proprietor EV passes manual review is unknown.
- OSSign's own pages (ossign.org returned only a title; `ossign-docs/pages/` not read): eligibility, cost, publisher name, application and turnaround. SSL.com's refund policy. Microsoft's `hardware-submission-support` page (support path for blocked registrations) was not fetched.

### Sources (read 2026-11-21)

[SSL.com Sole Proprietor EV](https://www.ssl.com/products/software-integrity/code-signing/ev-sole-proprietor/) and [buy page](https://secure.ssl.com/certificates/iv-ev-codesigning/buy); [eSigner pricing](https://www.ssl.com/guide/esigner-pricing-for-code-signing/) (updated 2026-03-25); [SSL.com cloud signing guide](https://www.ssl.com/guide/code-signing-certificates-cloud-signing-options-and-signing-operations-integration/) (2026-10-07); [SSL.com HDP registration how-to](https://www.ssl.com/how-to/register-with-windows-hardware-developer-program/); [Certum EV cloud](https://shop.certum.eu/ev-code-signing-in-the-cloud.html) and [card set](https://shop.certum.eu/certum-ev-code-sigining.html); [SignMyCode Sectigo EV](https://signmycode.com/sectigo-ev-code-signing); [DigiCert code signing](https://www.digicert.com/signing/code-signing-certificates); Microsoft: [register](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/hardware-program-register), [code-signing requirements](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs), [signing offerings](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings), [Windows Driver Policy](https://support.microsoft.com/en-US/Windows/Hardware/Drivers/the-windows-driver-policy), [Trusted Signing is not an EV substitute (Q&A)](https://learn.microsoft.com/en-us/answers/questions/5866910/hardware-program-verification-using-azures-trusted).

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
