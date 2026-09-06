# M00 virtual-driver feasibility — 2026-09-05

## Decision

Use a project-owned SysVAD-derived prototype as the technical evaluation path, while keeping production distribution blocked until the project has a real WDK build environment, target-machine validation, and an approved Microsoft signing route. Do not treat an installed third-party virtual cable as the product driver.

This is a prototype-path decision, not a release approval. It keeps the driver boundary aligned with VDEV-01/VDEV-02/VDEV-09: AudioRouter must own persistent virtual endpoints, lifecycle, data bridging, and recovery rather than depending on a user's unrelated cable installation.

## Evidence

- Microsoft describes [SYSVAD](https://github.com/Microsoft/Windows-driver-samples/tree/main/audio/sysvad) as a WDM virtual-audio sample exposing multiple devices and demonstrating WaveRT/audio-offload architecture. It is an appropriate technical starting point, but its sample endpoints and topology are not the AudioRouter product contract.
- The [SYSVAD build instructions](https://github.com/Microsoft/Windows-driver-samples/blob/main/audio/sysvad/README.md) require Visual Studio, Windows SDK, WDK, and WIL. Running and testing requires a separate target computer. The current host has SDK tools but no discovered Visual Studio/WDK installation.
- The Windows driver-samples repository is under the [Microsoft Public License](https://github.com/Microsoft/Windows-driver-samples/blob/main/LICENSE). Any reused source must retain notices and comply with the license; this does not provide Microsoft branding rights, product support, or a signing identity.
- Microsoft’s [driver-signing requirements](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs) state that attestation submissions require a Hardware Dev Center account with an EV certificate associated with it. A production package therefore needs organizational credentials and a signing/submission workflow that are not present in this workspace.

## Consequences and gates

The prototype must first demonstrate separate render/capture buses, stable endpoint identities, a bounded user-mode bridge, restart/rebind behavior, and clean uninstall on an isolated target machine. It must not be installed on the current development machine without a separate explicit authorization. Production remains blocked until x64 package signing, Secure Boot/HVCI behavior, upgrade/uninstall, and Windows 11 compatibility are evidenced.

No driver source was downloaded, built, installed, or changed during this investigation.

## Toolchain update (2026-09-05)

The earlier inventory statement is superseded: Visual Studio 2026 and matching
Windows SDK/WDK 28000 are now installed. Microsoft driver samples were used
only from a temporary checkout for compile evaluation and then removed. A
direct SysVAD kernel-source compile succeeded; the full reference solution
still requires a clean MSBuild environment and WIL dependency resolution.
No driver was copied into AudioRouter, installed, registered, loaded, or
changed, and production signing remains unresolved.

On 2026-09-06 the checked-in native probe build was rerun successfully with
Visual Studio Community 2026, MSVC 14.51, Windows SDK/WDK 10.0.28000.0, and
the C++20 toolchain. The generated executable/object were removed immediately
after compilation. This is toolchain evidence only; no driver was installed,
loaded, signed, or changed, and no machine audio configuration was touched.
