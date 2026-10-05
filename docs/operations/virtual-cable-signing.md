# Signing the AudioRouter virtual cable (human runbook)

For the maintainer. These steps cost money and use identity documents, so
an agent cannot do them; an agent prepares the files and checks the results.
Do them only after **stage A passes** in the VM
([testing](virtual-cable-testing.md), plan WP-12) and you have decided to
spend the money. Prices and Microsoft rules were read on 2026-10-05; check
them again before buying.

## What you pay and when

| When | What | Approximate cost (USD, before tax) |
| --- | --- | --- |
| Once per driver-release window | EV code-signing certificate, 1 year (validity up to ~15 months) | $280–$450 (Sectigo via resellers ~$280–$410; SSL.com $349; Certum €329 token / €379 cloud) |
| Same time, if not cloud | Hardware token + shipping | $0–$120 |
| If you have no business registration | SSL.com "Sole Proprietor EV" (for individuals), or register a sole proprietorship | Included / local fee |
| Never | Microsoft Hardware Developer Program, submissions | $0 |

You need a **valid** EV certificate only on the day you submit a driver.
Drivers that Microsoft already signed keep working after your certificate
expires (Microsoft states the certificate "must be valid at the time of
submission"; confirm with Partner Center support before relying on it). App
updates never need it. A driver change after the certificate expired means
buying a new one first, which takes several days of identity checks — so
group driver fixes, and keep the driver small.

## 1. Choose the identity

- **Individual:** SSL.com Sole Proprietor EV states that it validates an
  individual without a registered organization and supports kernel-mode
  driver signing and Windows Hardware Dev Center access.
- **Business:** any EV provider; you need the business registration and a
  public phone listing (the CA calls to verify).
- The name on the certificate is what Partner Center and the driver's
  publisher information show. Pick the name you want users to see.

## 2. Buy the EV certificate

1. Choose **cloud signing** (no USB token to lose; signing from any PC) or a
   **token** (FIPS 140-2 Level 2). Cloud is simpler for one person; check
   whether the provider charges per signature.
2. Complete validation (documents, phone call). Expect 1–10 business days.
3. Test it: sign any small `.exe` with
   `signtool sign /fd sha256 /tr <provider timestamp URL> /td sha256 ...`
   and check `signtool verify /pa /v`.
4. Store recovery codes and token PIN in your password manager. **Never**
   put the certificate, key, token PIN or cloud credentials in the
   repository, a script, an agent prompt or a log.

## 3. Register for the Hardware Developer Program

Follow Microsoft's page
[Register for the Hardware Developer Program](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/hardware-program-register):

1. Sign in to Partner Center with a work account. If you have none, create
   a free Microsoft Entra ID tenant during registration ("Create work
   account"); you become its global administrator.
2. Enter your company (or sole-proprietor) details; D-U-N-S is optional.
3. Provide a legal-contact email that you check; answer Microsoft's
   verification email.
4. Upload/prove the EV certificate as the page asks.
5. Fill in the business questionnaire sent to the administrator mailbox.
6. Wait for the approval email.

## 4. Prepare the submission (agent can do this)

From a clean checkout on the Windows PC, at the release version:

```powershell
.\drivers\audiorouter-virtual\build.ps1 -Configuration Release -Platform x64 -Version X.Y.Z -KeepOutput -Output C:\ar\release-driver
# Must show zero errors:
infverif /v /h /rulever 25h2 C:\ar\release-driver\audioroutervirtual.inf
.\tools\release\make-driver-cab.ps1 -Package C:\ar\release-driver -Output C:\ar\audiorouter-cable-X.Y.Z.cab
```

The CAB contains one folder with `audioroutervirtual.inf`,
`audioroutervirtual.sys` and `audioroutervirtual.pdb` (symbols help
Microsoft's crash analysis; they are not shipped to users). It is built with
`makecab /f <ddf>` (`make-driver-cab.ps1` writes the `.ddf`).

## 5. Sign the CAB and submit (you)

```powershell
signtool sign /fd sha256 /tr <provider timestamp URL> /td sha256 /n "<name on your EV certificate>" C:\ar\audiorouter-cable-X.Y.Z.cab
signtool verify /pa /v C:\ar\audiorouter-cable-X.Y.Z.cab
```

Partner Center → Hardware → **Submit new hardware**:

1. Product name: `AudioRouter Virtual Cable X.Y.Z`.
2. Upload the signed CAB.
3. Requested signatures: select **every Windows 11 client x64 version**
   offered (not only one build — a driver signed for a single build fails on
   others with Code 52).
4. Leave the test-signing options unchecked.
5. Submit; wait for the automated checks (usually under an hour).
6. Download the signed package.

## 6. Verify the returned package (agent can do this)

```powershell
signtool verify /kp /v <signed>\audioroutervirtual.sys
signtool verify /kp /v <signed>\audioroutervirtual.cat
```

Both must show `Microsoft Windows Hardware Compatibility Publisher`. Compare
the INF with the one you submitted (only signing-related changes). Copy the
signed `inf/sys/cat` into the release staging folder (plan WP-11) and record
SHA-256 hashes in the release evidence. Then run **stage B** of the testing
procedure.

## 7. Later driver releases

- Certificate still valid: repeat steps 4–6.
- Certificate expired: buy a new EV certificate (step 2), add it in Partner
  Center (Settings → certificates; follow
  [manage certificates](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-cert-manage)),
  then steps 4–6.
- Always increase `DriverVer` (the build does it from the version).

## 8. If Microsoft ends attestation for public drivers

The fallback is WHCP certification: same account and certificate, plus an
HLK test environment (free VHLK image) and passing the audio test playlist.
It costs time, not money. Record the decision in the driver track before
starting.

## Record keeping

For each submission, add to the release evidence: driver version, CAB
SHA-256, Partner Center submission ID, requested OS list, signed file
hashes, verification output. No personal identity documents, no
credentials.
