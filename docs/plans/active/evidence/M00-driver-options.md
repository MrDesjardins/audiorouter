# M00 virtual-driver feasibility — 2026-09-05

## Decision

Use a project-owned SysVAD-derived prototype as the technical evaluation path, while keeping production distribution blocked until the project has a real WDK build environment, target-machine validation, and an approved Microsoft signing route. Do not treat an installed third-party virtual cable as the product driver.

This is a prototype-path decision, not a release approval. It keeps the driver boundary aligned with VDEV-01/VDEV-02/VDEV-09: AudioRouter must own persistent virtual endpoints, lifecycle, data bridging, and recovery rather than depending on a user's unrelated cable installation.

## Evidence

- Microsoft describes [SYSVAD](https://github.com/Microsoft/Windows-driver-samples/tree/main/audio/sysvad) as a WDM virtual-audio sample exposing multiple devices and demonstrating WaveRT/audio-offload architecture. It is an appropriate technical starting point, but its sample endpoints and topology are not the AudioRouter product contract.
- The [SYSVAD build instructions](https://github.com/Microsoft/Windows-driver-samples/blob/main/audio/sysvad/README.md) require Visual Studio, Windows SDK, WDK, and WIL. Running and testing requires a separate target computer. The current host now has Visual Studio Community 2026 and Windows SDK/WDK 10.0.28000.0; the later build evaluation below records the remaining validation/tooling blockers.
- The Windows driver-samples repository is under the [Microsoft Public License](https://github.com/Microsoft/Windows-driver-samples/blob/main/LICENSE). Any reused source must retain notices and comply with the license; this does not provide Microsoft branding rights, product support, or a signing identity.
- Microsoft’s [driver-signing requirements](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs) state that attestation submissions require a Hardware Dev Center account with an EV certificate associated with it. A production package therefore needs organizational credentials and a signing/submission workflow that are not present in this workspace.

## Consequences and gates

The prototype must first demonstrate separate render/capture buses, stable endpoint identities, a bounded user-mode bridge, restart/rebind behavior, and clean uninstall on an isolated target machine. It must not be installed on the current development machine without a separate explicit authorization. Production remains blocked until x64 package signing, Secure Boot/HVCI behavior, upgrade/uninstall, and Windows 11 compatibility are evidenced.

No driver source was downloaded, built, installed, or changed during the initial
investigation recorded above; the disposable build evaluation below is a later
separate checkout and leaves no source or output in AudioRouter.

## Toolchain update (2026-09-05)

The earlier inventory statement is superseded: Visual Studio 2026 and matching
Windows SDK/WDK 28000 are now installed. Microsoft driver samples were used
only from a temporary checkout for compile evaluation and then removed. A
direct SysVAD kernel-source compile succeeded; the full reference solution
still requires a clean MSBuild environment and WIL dependency resolution.

## 2026-09-06 — SysVAD build evaluation

The official Microsoft `Windows-driver-samples` repository was cloned into a
disposable temporary checkout and its `audio/sysvad/sysvad.sln` was attempted
with Visual Studio Community 2026/MSBuild and WDK 10.0.28000.0. The complete
solution did not finish: APO projects require the repository's WIL dependency,
and the installed WDK's x86 `InfVerif.dll`/API-validator path reported missing
modules and exit-code failures.

A narrower, explicitly non-installing evaluation then built
`EndpointsCommon.vcxproj` followed by `TabletAudioSample.vcxproj` for x64
Release. Both succeeded and produced `TabletAudioSample.sys` (243,928 bytes).
The command used the sample/WDK properties `SkipPackageVerification=true` and
`ApiValidator_Enable=false` only to isolate compilation; it is not package or
API validation evidence. MSBuild also applied the sample's local automatic test
signature, which is not a production signature. The checkout and all outputs
were removed afterward. No driver was installed, no test-signing mode was
enabled, and no machine audio configuration changed.

Conclusion: the installed WDK is sufficient to compile the sample's core x64
driver target, but the full sample dependency/validation path and any
AudioRouter-owned SysVAD adaptation remain open. This does not satisfy the
managed-driver or production-signing gates.

## 2026-09-06 — WIL dependency follow-up

The SysVAD README identifies WIL as a Git submodule at the sample repository
root. A disposable sparse checkout was populated with WIL revision
`81abc8e44c075eba65fe4b68ea2f08525cc701e8` and rebuilt with normal validation.
The earlier `wil/com.h` errors disappeared: the APO projects, EndpointsCommon,
and TabletAudioSample all compiled and the driver was locally auto-signed by
the sample build.

The solution still failed at the same host-side WDK validation boundary:
`InfVerif.dll` could not be loaded from the WDK x86 path, and the subsequent
ApiValidator invocation returned exit code 193/-1. This is now isolated from
the WIL dependency. The generated driver/APO outputs and both temporary
checkouts were removed. No driver was installed, no test-signing mode was
enabled, and no machine audio configuration changed. Production signing,
package validation, target-machine testing, and AudioRouter-specific driver
adaptation remain open.
No driver was copied into AudioRouter, installed, registered, loaded, or
changed, and production signing remains unresolved.

On 2026-09-06 the checked-in native probe build was rerun successfully with
Visual Studio Community 2026, MSVC 14.51, Windows SDK/WDK 10.0.28000.0, and
the C++20 toolchain. The generated executable/object were removed immediately
after compilation. This is toolchain evidence only; no driver was installed,
loaded, signed, or changed, and no machine audio configuration was touched.

## 2026-09-07 — Full SysVAD validation with 64-bit MSBuild

The disposable Microsoft driver-samples checkout was rebuilt with the
repository WIL dependency and the installed VS2026/WDK 10.0.28000.0 toolchain.
The normal x64 Release `sysvad.sln` build passed with package and API
validation when invoked through
`MSBuild\\Current\\Bin\\amd64\\MSBuild.exe`; the x64 WDK validator components
were selected successfully. The generated package contained
`TabletAudioSample.sys` (243,928 bytes), four APO DLLs, INF files, and
`sysvad.cat`; signability reported no errors or warnings. The build's local
automatic test signature is not production signing evidence. The checkout and
all outputs were removed after verification. No driver was installed, loaded,
test-signing mode was enabled, or machine audio configuration changed.

This resolves the earlier host-build limitation: 32-bit MSBuild selected the
absent x86 validator path. It does not close AudioRouter-specific driver
adaptation, isolated target-machine behavior, uninstall/recovery, or
production-signing gates.

The repeatable wrapper is checked in at
`tests/acceptance/m00-sysvad-build.ps1`. It requires a disposable SysVAD/WIL
checkout under the system temporary directory, selects the 64-bit MSBuild
host, requires normal package/API validation, checks the driver and catalog
outputs, and removes generated `x64` directories by default. It performs no
driver installation or machine configuration operation.

The checked-in wrapper was then executed against fresh disposable SysVAD and
WIL checkouts. It passed the full x64 Release solution build, package/API
validation, and output assertions, and removed generated output directories;
the source checkout was removed separately afterward.

## 2026-09-07 — Repeatable wrapper requalification

The repository-local SDK provenance acceptance passed first, confirming the
pinned checkout and all seven recursive submodules. A fresh disposable
Microsoft Windows-driver-samples checkout was then created and passed to the
checked-in wrapper. The x64 Release solution again passed normal package/API
validation and the `TabletAudioSample.sys`/`sysvad.cat` assertions under the
64-bit VS2026 MSBuild host. The checkout and generated outputs were removed
afterward. This remains reference-sample build evidence only: no driver was
installed or loaded, test-signing mode was changed, or machine audio
configuration was touched.

## 2026-09-07 — Current-head SysVAD requalification

A fresh disposable `Windows-driver-samples` checkout with its WIL submodule
was passed to `tests/acceptance/m00-sysvad-build.ps1`. The 64-bit VS2026 MSBuild
host and WDK 10.0.28000.0 completed the full x64 Release solution with normal
package/API validation and signability checks. `TabletAudioSample.sys`, the
APO/INF outputs, and `sysvad.cat` were generated successfully; the checkout
and generated `x64` outputs were removed afterward.

The build's local automatic test signature is not production signing evidence.
This remains reference-sample qualification only: no driver was installed or
loaded, test-signing mode was changed, and no machine audio configuration was
changed.

The pinned-helper rerun verified the source revisions directly: Microsoft
`Windows-driver-samples` at
`197ba2156a60e2b76fcd4820bae594223e91a1e9` and its WIL gitlink at
`3c00e7f1d8cf9930bbb8e5be3ef0df65c84e8928`. The exact-commit checkout,
full x64 package/API validation, signability checks, and cleanup all passed.

The new top-level helper was then executed end-to-end. It cloned the Microsoft
reference repository, initialized WIL, passed the existing x64 package/API and
signability checks, and removed the exact temporary checkout with exit code 0.
This validates the documented reproduction path; it remains reference-sample
evidence only and performed no driver installation/loading, test-signing mode
change, or machine audio configuration action.

## 2026-09-07 — Current-head helper requalification

The repository-local SDK installer provenance acceptance passed, followed by
the pinned disposable SysVAD helper. Microsoft `Windows-driver-samples` at
commit `197ba2156a60e2b76fcd4820bae594223e91a1e9` and WIL at
`3c00e7f1d8cf9930bbb8e5be3ef0df65c84e8928` were checked out. The installed
64-bit VS2026 MSBuild/WDK 10.0.28000.0 toolchain completed the full x64 Release
solution, normal package/API validation, signability checks, and generated
the driver, APO/INF, and `sysvad.cat` outputs. The exact temporary checkout
and generated outputs were removed. The build's local automatic test signature
is not production signing evidence; no driver was installed or loaded, test
signing was not enabled, and no machine audio configuration changed.

## 2026-09-08 â€” Installed-toolchain qualification

The read-only M00 toolchain acceptance verified Visual Studio Community
18.9.2, MSVC 14.51.36231, and Windows SDK/WDK `10.0.28000.0`. The repository
native probe compile passed with the same installed headers, libraries, and
WDK properties.

The authorized `tools/m00-sysvad/qualify.ps1` run used a disposable checkout
of Microsoft driver-samples at revision
`197ba2156a60e2b76fcd4820bae594223e91a1e9` and WIL at
`3c00e7f1d8cf9930bbb8e5be3ef0df65c84e8928`. The normal x64 build compiled
the sample driver and APO targets, generated the x64 package/catalog, and
reported successful package/API validation with no signability errors or
warnings. The sample's automatic test signature is not production signing
evidence. The exact temporary checkout and outputs were removed by the
qualification helper.

This closes the local compile/package prerequisite but not AudioRouter-specific
driver adaptation, isolated target-machine testing, installation/loading,
restart/rebind, uninstall, Secure Boot/HVCI, or production signing. No driver
was installed or loaded, and no test-signing mode, plugin/startup registration,
or machine audio configuration changed.
