# M08 — Windows release qualification and delivery

Status: unsigned release-preparation foundation implemented; clean-checkout and remaining in-scope qualification remain open. The approved v1 excludes an AudioRouter-owned driver and its production signing, plus installer and clean-machine gates under DEC-16. DEC-16 does not decide whether the desktop app/installer receives Authenticode signing. Prerequisite: M07 and a Windows qualification environment. Outcome: an evidence-backed unsigned Windows 11 x64 artifact set for the VB-Cable-first profile, with clear operating and recovery instructions; this is not a releasable installer and publication is not implied.

## Read first

All specification files, [delivery traceability](../spec/15-delivery.md), prior milestone evidence, and the active risk register. This is a release gate, not permission to publish externally without the user's requested release scope.

## Ordered implementation

1. Reconcile every in-scope requirement ID with actual implementation and evidence. Resolve missing requirements, stale docs, misleading capability claims, and deviations. Reverify supported Windows 11 builds and dependency requirements.
2. Build app/backend/CLI/MCP/worker artifacts from pinned clean inputs. Document per-user runtime, WebView2 prerequisites, dependency notices/SBOM, versions, and checksums. Do not build or package an AudioRouter-owned driver under this scope.
3. **Future distribution track only (DEC-16):** installer, repair, upgrade,
   rollback, uninstall, and clean-machine qualification are not v1 gates.
   Preserve user configuration and third-party endpoint selections in any
   separately authorized distribution plan; never install or remove audio
   drivers as part of this project scope.
4. Run the complete UC-01–10 suite, hardware/app matrix, DSP/file correctness, accessibility/usability, performance/endurance, and security regressions. Retain raw evidence and failures; fix release blockers.
5. Write quickstart, Windows/app device-selection walkthroughs, CLI/MCP reference, effects explanation, privacy/permissions guide, plugin compatibility list, troubleshooting, diagnostics export, backup/migration, third-party endpoint guidance, and uninstall guide. Put operational docs at stable paths when implementation creates them.
6. Prepare versioned release notes stating supported OS/architecture, measured reference latency, hardware/plugin caveats, omitted future features, fixed issues, known issues, install elevation/restart needs, and recovery options.
7. Prepare the concrete release artifacts and summary for the authorized publication workflow. If publishing/signing requires unavailable credentials or new authority, finish all unaffected preparation and identify the exact remaining action and dependency.

## Mandatory release gate

All approved v1 PROD, ARCH, GRAPH, CAP, DSP, PLUG, REC, UI, API, AUTO, STATE, SEC, NFR, QUAL, and ENG requirements have evidence. VDEV-01/03/09 and SEC-08 are excluded from v1 by DEC-16 and remain normative only for a separately authorized future driver track. The primary workflow works with the documented existing-device boundary. UI closure, backend crash, reboot/sign-in, disk failure, plugin failure, and user switching exhibit the specified behavior.

Performance targets are met on the declared reference hardware, with distributions and workload details published. No universal Bluetooth or arbitrary-plugin latency claim is made. The primary workflow documents the separately installed, supported external endpoints it requires. At least four of five first-time users complete the setup within ten minutes and all identify Discord's source set. Keyboard/Narrator and 200% scaling checks pass. External AI control is optional, local, discoverable, and permission-constrained.

## Future installer and uninstall acceptance — excluded from v1

If a distribution track is separately authorized, run as a standard user and verify the package does not require driver-administration elevation. Update while sessions are stopped or after a user-approved stop plan; preserve configuration/recordings and validate app compatibility before restart. Rollback restores a compatible package/configuration pair. Uninstall preserves existing third-party audio drivers/endpoints and offers configuration retention; recordings remain unless individually targeted through a separate explicit action. Verify no stale startup task, privileged broker, or exposed control pipe remains.

## Evidence and handoff

Archive the release execution plan, full requirement/evidence matrix, artifact hashes and unsigned status, test-machine manifests, measured reports, security findings/resolutions, and known issues. Keep private audio out of the repository. Update README from “specification only” only when it reflects actual implemented/tested state. Start a new active maintenance plan for defects and future requests; move deferred ideas only after scope authorization.

## Stop conditions

Do not label the release complete with missing real Windows tests, unresolved private-audio leakage, reproducible backend crashes, corrupted recordings, or unfulfilled core API parity. If hardware or environment access is missing, report blocked release evidence and retain prepared artifacts. A documented blocker is more useful than an invented pass.

Suggested request: “Execute M08 release qualification for the approved non-driver scope, prepare the unsigned Windows artifacts and complete evidence/docs, and report any exact publication action still requiring unavailable authority.”
