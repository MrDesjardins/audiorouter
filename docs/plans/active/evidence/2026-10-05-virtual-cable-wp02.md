# Virtual cable WP-02 — versioned test package

Date: 2026-10-05. Requirements: VDEV-09 (development side), SEC-08,
ENG-05. Scope: host build, package integrity and test signing only. WP-02
passed; WP-01 VM readiness and all loaded-driver/audio gates remain open.

## Environment and inputs

- Windows x64, reported OS version 10.0.26300.0; PowerShell 5.1; VS 2022
  MSBuild; WDK 10.0.28000.0, WDK assembly 10.0.28000.2526.
- CIM OS inventory returned access denied. Hyper-V `Get-VM` was unavailable
  in the agent session; no VM, checkpoints or host Secure Boot baseline claimed.
- Baseline source: `2a32310b71666368dc802a83d50619a972ead066`.
- Final clean-tree one-command build: `9a59215abadccb6c5277854ea4b71717975581f7`,
  metadata `dirty: false`, version `0.1.0`, DriverVer `10/05/2026,0.1.0.0`,
  Release/x64, `builtAt: 2026-10-05T23:43:47.2987352Z`.
- Existing WDK test certificate was selected in the clean-tree build;
  thumbprint `FF6876FBE50A74DC0B69077C11DC28A1A9FAAD40`. The earlier
  explicit-certificate run also exercised creation/use of a non-exportable
  CurrentUser/My AudioRouter test key. Only public CERs were exported. No
  Root/TrustedPublisher import, driver installation/loading, boot-policy,
  endpoint or audio-default change was performed.

## Implementation and decisions

`build.ps1` validates X.Y.Z (each component 0–65534), rebuilds the requested
configuration, stages its exact artifacts into a fresh folder, stamps the
generated INF with WDK StampInf, generates the catalog and records provenance.
MSBuild receives an argument array; paths with spaces and `&` were exercised.
Caller output is not overwritten; existing artifacts remain intact after a
refused rebuild. Generated SYS/CAT/CER and private PFX files are git-ignored.

`sign-test.ps1` validates the package, runs InfVerif before key access, signs
SYS, generates the catalog from those signed bytes, then signs CAT. It checks
catalog membership of INF/SYS, both standalone signatures and the selected
signer's identity; it exports only a public CER. Metadata is marked unsigned
before signing mutations and test-signed only after all verification completes.
The shared helpers have no side effects when imported and can be exercised
without kernel execution.

Local test signing has no timestamp by default, so it works offline; optional
RFC 3161 timestamping is exposed but was not exercised. This is a procedural
refinement of WP-02, with no audio architecture or quality target change.
The package remains a development prototype, not Microsoft-signed.

The installed SignTool reports the expected untrusted root in English text
without an HRESULT. Exit code 1 alone is never accepted: the verifier requires
the exact single root error, expected certificate and no other failure. Other
or localized failures stop signing. Native trust remains **untrusted** on the
host; this is explicitly recorded rather than called a trust pass. Deliberate
INF/SYS corruption returned `File not found in the specified catalog` and
failed verification, proving the exception does not hide membership failures.

## Checks performed

Commands below ran on the Windows host, without live audio. Raw logs and
generated packages are disposable files under `target/`, not source artifacts.

| Command | Result | Local evidence |
| --- | --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1` (baseline) | Pass | `target/driver-baseline-build.log` |
| `cargo test -p audiorouter-windows-audio` | 110 passed, 0 failed; 2 live tests ignored | `target/driver-baseline-rust.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1` (new workflow) | x64 pass | `target/driver-wp02-acceptance.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform ARM64` | ARM64 compile/catalog pass; no ARM64 load/signing claim | `target/driver-wp02-arm64.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File drivers/audiorouter-virtual/build.ps1 -Version 0.1.0 -KeepOutput -TestSign -Output 'target/driver wp02 & clean'` | One-command clean-tree package produced | `target/driver-wp02-clean-build.log` |
| `InfVerif /v /h <package>/audioroutervirtual.inf` (signing preflight) | INF valid; 0 errors/warnings | package `infverif.log` |
| `Inf2Cat /driver:<package> /os:10_X64 /uselocaltime` (after SYS signing) | Pass; installed `Inf2Cat /?` help confirms `10_X64` | package `inf2cat.log` |
| `signtool verify /pa /v /c <cat> <sys-or-inf>`; `signtool verify /pa /v <cat>` | Expected untrusted-root code 1, membership intact; standalone signatures identify the selected test certificate | package `verify-*.log`, `signature-verification.json` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-package.ps1 -Package 'target/driver wp02 & clean'` | 33 checks passed | `target/driver-wp02-clean-tests.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/docs.ps1` | Pass: 121 Markdown files / 647 local links | terminal output |
| `git diff --check`; review of all changed scripts and staged diff | Pass | code commit above |

The 33 checks cover version boundaries/hostile values, expected root diagnostics
versus wrong signer/multiple errors/hash failure, real catalog membership,
public-only certificate export, current artifact hashes, refusal to overwrite
output with original hashes preserved, tampered INF/SYS and metadata/INF version
mismatch. Tamper logs: `target/driver-package-test-940241e996834ac58c4a39555c04bf24/`.

Initial failures were corrected: sandbox certificate creation could not reach
the certificate store (the authorized signing step ran outside the sandbox);
the PowerShell EKU ObjectId is a string, not an object with `.Value`; standalone
catalog verification emits one diagnostic whereas membership verification
emits two. The root-warning parser has regression cases for both forms.

## Clean-tree package SHA-256

Package directory: `target/driver wp02 & clean/`.

| File | SHA-256 |
| --- | --- |
| `audioroutervirtual.inf` | `2AFB43418939DD998C9D20754F1529D41295AEA9D8F128073FC59CF8F06B2197` |
| `audioroutervirtual.sys` | `46035D99749B64EF565DEA79A76C0F33D457D27D19407E1C3BD0B4FA2419552A` |
| `audioroutervirtual.cat` | `F19F00B57EB3E6AAEF1C1558260FEF62CC1B9D3797352956F58CC2A234926BC8` |

## Limitations, rollback and next task

No VM install/load/uninstall result, endpoint enumeration, sound-quality,
latency, CPU/DPC, security fuzzing or hardware evidence exists for this work.
VCAB-20–30 remain required at their assigned WPs; no audio-path code changed.
The clean-tree build used existing dependencies, not a newly provisioned PC.
Timestamped signing, localized diagnostics and production trust were not tested.
WP-00 D2–D4/D6 remain proposed defaults, not silently accepted decisions.

Rollback: revert the package tooling commit and this documentation commit;
generated packages are disposable and no installed driver needs removal.
Keep or deliberately remove only the newly-created AudioRouter test certificate
from CurrentUser/My; never remove an existing WDK certificate. The exported CER
contains no private key.

Exact next task: WP-03, prepare the guarded A1/A2/A3/A14 VM smoke script, then
have the user run it twice from `AR-DriverTest/02-test-signing-ready`. WP-01
still needs the user's VM/checkpoint and host-policy evidence. Never execute
the install or smoke script on the host.

Microsoft references used for tool syntax:
[StampInf options](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/stampinf-command-options),
[Inf2Cat](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/inf2cat),
[test-signing a catalog](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/test-signing-a-catalog-file).
