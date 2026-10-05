# M08 — Windows release qualification and delivery

Status: installer and manual draft-release path implemented; bundle and script regressions pass. Clean-checkout release acceptance, standard-user install, upgrade/uninstall, missing-WebView2, clean-machine, and remaining product qualification are open. The AudioRouter-owned driver, PortCls integration, and driver signing remain excluded by DEC-16. The first desktop-app/installer release may be unsigned and must disclose that state. Prerequisite: M07 and Windows qualification. Outcome: a qualified per-user Windows 11 x64 installer and verified manual GitHub draft-release path for the existing-endpoint profile, plus evidence-backed operating, recovery, and known-issue documentation. Creating a draft does not publish a release.

## Read first

All specification files, [delivery traceability](../spec/15-delivery.md), prior milestone evidence, and the active risk register. This is a release gate, not permission to publish externally without the user's requested release scope.

## Ordered implementation

1. Reconcile every in-scope requirement ID with actual implementation and evidence. Resolve missing requirements, stale docs, misleading capability claims, and deviations. Reverify supported Windows 11 builds and dependency requirements.
2. Build shell/backend/CLI/plugin-worker artifacts from pinned clean inputs. Bundle the CLI and plugin worker with the shell and prove packaged path resolution. Record versions, checksums, SBOMs, and third-party notices. Never build/package the excluded AudioRouter-owned driver.
3. Produce a per-user NSIS setup package. Verify standard-user install, shortcut, WebView2 present/missing path, endpoint discovery guidance, upgrade/repair, and uninstall retention. Never install/remove audio drivers or change Windows defaults.
4. Implement a manual PowerShell/GitHub Actions release trigger from an existing version tag. Run checks, prepare assets, verify provenance and hashes, and create a draft only. Require a separate human publish action. The first app release may be unsigned under DIST-07; no credential purchase is implied.
5. Run the complete UC-01–10 suite, hardware/app matrix, DSP/file correctness, accessibility/usability, performance/endurance, and security regressions. Retain raw evidence and failures; fix blockers or document an explicit deviation.
6. Write install/upgrade/uninstall instructions, WebView2 and endpoint prerequisites, CLI/MCP setup, privacy/permissions, plugin compatibility, troubleshooting, backup/migration, unsigned-package trust notice, recovery, and uninstall guide.
7. Prepare versioned release notes with OS/architecture, measured latency, endpoint/plugin caveats, unsigned status, known issues, installation/runtime needs, and rollback/recovery steps. Upload all reviewed assets to a draft release; publish only through a separate explicit human action.

## Mandatory release gate

All approved v1 PROD, ARCH, GRAPH, CAP, DSP, PLUG, REC, UI, API, AUTO, STATE, SEC, NFR, QUAL, and ENG requirements have evidence. VDEV-01/03/09 and SEC-08 were excluded by DEC-16 and reopened by DEC-18 (2026-10-05); they gate only a release that ships the AudioRouter cable, through the [driver track](../plans/future/M03-driver-signing.md). The primary workflow works with the documented existing-device boundary. UI closure, backend crash, reboot/sign-in, disk failure, plugin failure, and user switching exhibit the specified behavior.

Performance targets are met on the declared reference hardware, with distributions and workload details published. No universal Bluetooth or arbitrary-plugin latency claim is made. The primary workflow documents the separately installed, supported external endpoints it requires. At least four of five first-time users complete the setup within ten minutes and all identify Discord's source set. Keyboard/Narrator and 200% scaling checks pass. External AI control is optional, local, discoverable, and permission-constrained.

## Installer, upgrade, and uninstall acceptance — required by DEC-17

Run as a standard user and verify the package does not require elevation. Upgrade while sessions are stopped or after a user-approved stop plan; preserve configuration/recordings and validate app compatibility before restart. Rollback restores a compatible package/configuration pair. Uninstall removes app-owned files and shortcuts, leaves third-party audio drivers/endpoints untouched, and retains user data by default. Verify no stale startup task, privileged broker, or exposed control pipe remains. Test both present and absent WebView2 and supported endpoint states on Windows.

## Evidence and handoff

Archive the release execution plan, full requirement/evidence matrix, artifact hashes and unsigned status, test-machine manifests, measured reports, security findings/resolutions, and known issues. Keep private audio out of the repository. Update README from “specification only” only when it reflects actual implemented/tested state. Start a new active maintenance plan for defects and future requests; move deferred ideas only after scope authorization.

## Stop conditions

Do not label the release complete with missing real Windows tests, unresolved private-audio leakage, reproducible backend crashes, corrupted recordings, or unfulfilled core API parity. If hardware or environment access is missing, report blocked release evidence and retain prepared artifacts. A documented blocker is more useful than an invented pass.

Suggested request: “Execute M08 release qualification for the approved non-driver scope, prepare the unsigned Windows artifacts and complete evidence/docs, and report any exact publication action still requiring unavailable authority.”
