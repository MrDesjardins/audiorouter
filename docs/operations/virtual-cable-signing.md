# Signing the AudioRouter virtual cable (step-by-step guide)

For the maintainer. These steps cost money and use your identity documents,
so only you can do them. The agent prepares the driver files, checks every
result and keeps the records. Facts, prices and Microsoft rules below were
re-read on **2026-10-14**; check prices again on the day you buy.

The driver being signed is the frozen **RC3, version 0.2.4** (source
`d2560715`), the exact binary Stage A tested. No driver change is planned
before signing.

## Overview

| Part | Who | What | Time | Cost (USD, before tax) |
| --- | --- | --- | --- | --- |
| 0 | You | Read the scope question; choose the name | 30 min | $0 |
| 1 | You | Buy the EV code-signing certificate | 3–5 business days of checks | $359 (1 year) |
| 2 | You | Set up signing on this PC and test it | 30 min | $0 or cloud fee |
| 3 | You | Register for the Hardware Developer Program | 1 h, then days of approval | $0 |
| 4 | You | Ask Microsoft the distribution question in writing | 15 min, then wait | $0 |
| 5 | Agent | Build the submission CAB from RC3 | 1 h | $0 |
| 6 | You | Sign the CAB and submit it | 20 min, ~1 h of checks | $0 |
| 7 | Agent, then you | Verify the signed files; Stage B | 1 session | $0 |

Total spend for the first release: about **$360–$740** (certificate plus key
storage, see 1.3). Microsoft charges nothing.

## Part 0 — Before you spend anything

### 0.1 Microsoft's stated scope for attestation signing

Microsoft signs drivers two ways:

- **WHCP/HLK certification:** you run the Hardware Lab Kit tests on a test
  lab and submit the logs. Required for Windows Update and Windows Server.
- **Attestation signing:** no HLK tests; you attest that you tested the
  driver. Works on Windows 10 and 11 desktop. This is our plan.

Microsoft's documentation calls attestation signing "for testing purposes
only" and says attestation-signed drivers "can't be published to Windows
Update for retail audiences"
([driver signing options](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/driver-signing-offerings),
dated 2026-03-23; same wording since 2023 in the
[IHV attested signing page](https://learn.microsoft.com/en-us/windows-hardware/design/compatibility/ihv-attested-signing-service)).
The [Code Signing Agreement](https://learn.microsoft.com/en-us/legal/windows/hardware/code-signing-agreement-2208)
you accept at registration does not forbid distributing the signed driver
yourself: it requires submitted code to be "ready for release for general
public distribution" and licenses you to "reproduce, use, and distribute"
Microsoft's signature on your signed code. Windows itself loads an
attestation-signed driver with Secure Boot and Memory Integrity on, and many
small vendors ship drivers this way in their own installers.

So shipping RC3 in the AudioRouter installer is technically sound, and the
agreement permits it. But Microsoft's documentation describes the service
more narrowly. **Part 4 asks Microsoft in writing before the first public
release.** If the answer is no, the fallback is WHCP/HLK (Part 9): the same
account and certificate, more testing time, no extra money.

### 0.2 What the agreement asks of you

Read the agreement before you accept it (Part 3, step 9). Points that apply
to a one-person project (not legal advice):

- You support your users. Microsoft forwards support questions only for
  users with paid support agreements (§5).
- Microsoft can block or revoke a signed driver if you stop supporting it,
  breach the agreement or submit code with malware or vulnerabilities
  (§3). Users who already installed it keep their licences, but the driver
  may stop loading.
- Microsoft may audit a signed driver on 30 days' notice (§6).
- §15(g): you "will maintain commercially reasonable liability insurance
  customary in the industry". Decide for yourself what that means for you.
- You may not submit code on behalf of anyone else (§2(a)(iv)).

### 0.3 Choose the name

The certificate, Partner Center and the agreement all carry one legal
identity. It is what Microsoft sees as the publisher.

- **As an individual (simplest):** SSL.com's
  [Sole Proprietor EV](https://www.ssl.com/products/software-integrity/code-signing/ev-sole-proprietor/)
  validates you as a person, with no business registration, and SSL.com
  states it supports kernel-mode driver signing and Partner Center access.
  The name is your legal name.
- **As a business:** if you want a company name instead, register the
  business first (sole proprietorship or company, local fee). Then any EV
  provider works. The CA verifies the registration and a public phone
  listing.

Pick one now: changing it later means a new certificate and a new Partner
Center account.

## Part 1 — Buy the EV certificate

### 1.1 Order

1. Go to the SSL.com Sole Proprietor EV page above. Price on 2026-10-14:
   **$359 for 1 year**. Multi-year prepay is cheaper per year ($201/yr over
   5 years), but since March 2026 a code-signing certificate lasts at most
   458 days, so a multi-year order means a reissue (new identity check) each
   year. Buy **1 year** unless you plan several driver releases.
2. Choose key storage (1.3). Ignore add-ons you don't need (expedited
   validation is +$599; standard is 3–5 business days).
3. Pay. Use the email address you will also use for Partner Center notices.

### 1.2 Validation

SSL.com verifies your identity: expect a government photo ID, proof of
address, and a phone or video call. Answer quickly; each delay restarts the
clock. When approved, follow their email to issue the certificate onto the
storage you chose.

### 1.3 Key storage (required by the rules: the private key lives in hardware)

| Option | Cost | Notes |
| --- | --- | --- |
| **eSigner cloud signing** | $100/month (Tier 1, 10 signatures) or $900/year; the first 30 days have unlimited signatures at no charge ([eSigner pricing](https://www.ssl.com/guide/esigner-pricing-for-code-signing/)) | No hardware. Works with `signtool` through SSL.com's eSigner CKA (Cloud Key Adapter). You need only a few signatures per driver release (test file, `Signablefile.bin`, the CAB, maybe a retry). |
| **YubiKey from SSL.com** | +$379 one-time | Physical token; sign with `signtool` on this PC. SSL.com notes a YubiKey "may not meet EV requirements for Microsoft HLK driver signing" and asks you to contact them; ask before buying. |

**Recommendation:** eSigner. Do everything inside the free first 30 days if
Partner Center approves in time (Part 3 usually takes days, sometimes
longer). Otherwise pay one month ($100) when you sign the CAB. Cancel the
subscription afterwards and resubscribe for the next driver release. Ask
SSL.com to confirm two things before ordering: that cancelling eSigner keeps
the certificate valid, and that resubscribing works. Credits are lost when
a subscription lapses.

### 1.4 Keep the secrets out of everything

Store the eSigner credentials, TOTP secret, token PIN and recovery codes in
your password manager. **Never** put them in the repository, a script, an
agent prompt, a terminal the agent can read or a log. The agent never needs
them: you run every signing command yourself.

## Part 2 — Set up signing on this PC and test it

1. `signtool.exe` is already installed with the WDK:
   `C:\Program Files (x86)\Windows Kits\10\bin\<version>\x64\signtool.exe`.
2. **eSigner:** install eSigner CKA from SSL.com, sign in, and confirm the
   certificate appears in the current user's Personal store (`certmgr.msc`
   → Personal → Certificates). **YubiKey:** install the driver SSL.com
   tells you to; plug it in; confirm the certificate appears the same way.
3. Test on a copy of any small `.exe` (not a project file):

   ```powershell
   $signtool = (Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' | Sort-Object FullName | Select-Object -Last 1).FullName
   Copy-Item C:\Windows\System32\whoami.exe $env:TEMP\sign-test.exe
   & $signtool sign /fd sha256 /tr http://ts.ssl.com /td sha256 /n "<name exactly as on the certificate>" $env:TEMP\sign-test.exe
   & $signtool verify /pa /v $env:TEMP\sign-test.exe
   ```

   `verify` must end with "Successfully verified" and show your name and a
   timestamp. If `/n` finds several certificates, use `/sha1 <thumbprint>`.
   Use the timestamp URL SSL.com gives you if it differs.

## Part 3 — Register for the Windows Hardware Developer Program

Microsoft's page:
[Register for the Hardware Developer Program](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/hardware-program-register)
(dated 2026-08-11). Before you start, make sure you can receive external
email at the address you will name as legal contact.

1. Open the
   [registration link](https://partner.microsoft.com/dashboard/account/exp/enrollment/welcome?cloudInstance=Global&accountProgram=hardware).
2. You have no organization directory, so choose **Create work account**.
   This creates a free Microsoft Entra ID tenant with you as global
   administrator (an address like `you@<name>.onmicrosoft.com`). Save that
   sign-in in your password manager and turn on its multi-factor sign-in.
3. Confirm you signed in as global administrator → **Next**.
4. The **Hardware** program is preselected → **Go to Dashboard**.
5. Company information: search for yourself or your business. With no
   D-U-N-S number, choose **I don't have D-U-N-S number** and enter the
   details manually: the same legal name and address as on the certificate.
6. Legal contact: you, with your real email. **Answer Microsoft's
   verification email** when it arrives.
7. Read the agreements (Code Signing, Hardware Compatibility, Marks Licence,
   Analytics); see Part 0.2. Select **Accept and Continue**.
8. Add the certificate: gear icon → **Account settings** → **Manage
   certificates** → **Add a new certificate** → **Next**. Download
   `Signablefile.bin`, sign it, and upload the signed file:

   ```powershell
   & $signtool sign /fd sha256 /tr http://ts.ssl.com /td sha256 /n "<name on the certificate>" "$env:USERPROFILE\Downloads\Signablefile.bin"
   ```

   ([manage certificates](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-cert-manage))
9. Watch the global administrator mailbox for a **business questionnaire**.
   A new `onmicrosoft.com` mailbox may not reach you, so set a preferred
   email under **My Preferences** right away. Registration stalls until you
   answer the questionnaire.
10. Wait for the approval email. Tell the agent when it arrives.

## Part 4 — Ask Microsoft the distribution question

After approval, open a support request in Partner Center (the **?** /
Support icon → hardware dashboard → driver signing). Suggested text:

> We are a single-developer publisher (account: <your account>). We plan to
> attestation-sign a software-only (root-enumerated) virtual audio driver
> for Windows 10/11 desktop, x64, and distribute it to the public inside
> our own signed installer, not through Windows Update. Is distributing an
> attestation-signed driver this way permitted under the Code Signing
> Agreement and current Hardware Dev Center policy, or is WHCP/HLK
> certification required?

Paste the answer into a text file and give it to the agent; it goes into
the release record. You can still do Part 6 while you wait: signing and
Stage B testing are allowed either way. Only the **public release** waits
for a yes. If the answer is no, see Part 9.

## Part 5 — The agent prepares the submission (nothing for you to do)

The agent builds `C:\ar\signing\audiorouter-cable-0.2.4.cab` from the frozen
RC3 package (`C:\VMs\ar-share\rc-0.2.4-base\driver`), not a new build:

- One folder `audioroutervirtual` (short, plain name: Microsoft requires
  under 40 characters and no special characters) containing
  `audioroutervirtual.inf`, `audioroutervirtual.sys` and
  `audioroutervirtual.cat`. Microsoft replaces the test signature on the
  `.sys` and regenerates the catalog
  ([attestation signing](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-attestation)).
  The test certificate, logs and licence file stay out.
- SHA-256 of the `.sys` and `.inf` must match the RC3 freeze record
  (`AF3B4E5E…`, `E29D89DC…`). `infverif` must report zero errors.
- **Debug symbols (open item, agent):** Microsoft lists the `.pdb` as a CAB
  component for its crash analysis. RC3's own `.pdb` was not kept: the
  0.2.5 build overwrote it in the build folder (checked with `symchk`
  2026-10-14). The agent rebuilds 0.2.4 from `d2560715`, and includes that
  `.pdb` only if the rebuilt `.sys` differs from RC3 in nothing but
  timestamps and debug identifiers. Otherwise the CAB goes without symbols
  (Microsoft's checks report whether that is accepted). Future packages keep
  their own `.pdb`.
- The agent writes the CAB with `makecab` and a `.ddf` (as in Microsoft's
  example), lists its contents and records its SHA-256.

## Part 6 — Sign the CAB and submit it (you)

1. Sign and verify:

   ```powershell
   & $signtool sign /fd sha256 /tr http://ts.ssl.com /td sha256 /n "<name on the certificate>" C:\ar\signing\audiorouter-cable-0.2.4.cab
   & $signtool verify /pa /v C:\ar\signing\audiorouter-cable-0.2.4.cab
   ```

2. Open the
   [hardware dashboard](https://partner.microsoft.com/dashboard/hardware/Search)
   → **Submit new hardware**.
3. Product name: `AudioRouter Virtual Cable 0.2.4`. Other companies see it
   only if you share the driver.
4. Upload the signed CAB.
5. Leave **both test-signing options unchecked**.
6. **Requested signatures:** select every **Windows 11 client x64**
   version offered, plus Windows 10 client x64 if offered. A driver
   signed for only one Windows build fails on others with Code 52.
7. **Submit**. The automated checks usually finish within an hour. If they
   fail, copy the error text for the agent; don't change the files
   yourself.
8. When the submission shows **Completed**, download the signed package
   (a ZIP) into `C:\ar\signing\signed\` and tell the agent. Note the
   submission ID shown on the page.

## Part 7 — Verification and Stage B

1. **Agent:** checks that `audioroutervirtual.sys` and the new
   `audioroutervirtual.cat` verify with `signtool verify /kp /v` and chain to
   *Microsoft Windows Hardware Compatibility Publisher*. Checks that the
   INF differs from the submitted one only in signing-related lines. Records
   the hashes, submission ID, requested OS list and verification output in
   the release evidence, and stages the signed files for the installer
   (WP-11).
2. **You and the agent:** Stage B in a clean VM with Secure Boot and Memory
   Integrity on, then the installer check and Stage C on this PC, as in the
   [testing guide](virtual-cable-testing.md).

## Part 8 — Records and hygiene

- For each submission the release evidence holds: driver version, CAB
  SHA-256, submission ID, requested OS list, signed file hashes, the
  verification output and Microsoft's Part 4 answer. No identity documents,
  no credentials, no screenshots of personal data.
- Calendar reminders: the certificate expiry date, and 30 days before it.

## Part 9 — Later driver releases, renewal and the fallback

- **Certificate still valid:** the agent repeats Part 5 for the new
  version; you repeat Part 6. Always raise the driver version (the build
  does it).
- **Certificate expired:** buy a new one (Part 1), add it in Partner Center
  (Part 3, step 8, which also retires the old one), then Parts 5–6. Drivers
  Microsoft already signed keep working after your certificate expires;
  the certificate must be valid "at the time of submission". So you need
  one only in years with a driver change. Group driver fixes accordingly.
- **If Microsoft says attestation is not for public distribution (Part 4),
  or ends it:** move to WHCP certification. Same account and certificate,
  plus a Hardware Lab Kit setup (the free VHLK virtual machine image can
  serve as controller) and passing the audio test playlist on a test
  machine. It costs time, not money. The agent records the decision in the
  driver plan before starting.
