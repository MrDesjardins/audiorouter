# M08 — Windows release qualification and delivery

Status: not started. Prerequisite: M07 and available production driver-signing/distribution prerequisites. Outcome: a verified, signed, installable Windows 11 x64 v1 with clear operating and recovery instructions.

## Read first

All specification files, [delivery traceability](../spec/15-delivery.md), prior milestone evidence, and the active risk register. This is a release gate, not permission to publish externally without the user's requested release scope.

## Ordered implementation

1. Reconcile every requirement ID with actual implementation and evidence. Resolve missing requirements, stale docs, misleading capability claims, and deviations. Reverify supported Windows 11 builds and dependency/signing requirements.
2. Build app/backend/CLI/MCP/worker/driver artifacts from pinned clean inputs. Produce installer with clear per-user versus machine components, WebView2 prerequisite handling, standard-user runtime, signed binaries/packages, dependency notices/SBOM, versions, and checksums.
3. Test clean install, repair, upgrade from previous development package, incompatible app/driver versions, rollback, and uninstall on Windows with Secure Boot and Memory Integrity enabled. Preserve endpoint identity where promised and guide any required reselection. Never remove unrelated audio drivers/devices.
4. Run the complete UC-01–10 suite, hardware/app matrix, DSP/file correctness, accessibility/usability, performance/endurance, and security regressions. Retain raw evidence and failures; fix release blockers.
5. Write quickstart, Windows/app device-selection walkthroughs, CLI/MCP reference, effects explanation, privacy/permissions guide, plugin compatibility list, troubleshooting, diagnostics export, backup/migration, driver rollback, and uninstall guide. Put operational docs at stable paths when implementation creates them.
6. Prepare versioned release notes stating supported OS/architecture, measured reference latency, hardware/plugin caveats, omitted future features, fixed issues, known issues, install elevation/restart needs, and recovery options.
7. Prepare the concrete release artifacts and summary for the authorized publication workflow. If publishing/signing requires unavailable credentials or new authority, finish all unaffected preparation and identify the exact remaining action and dependency.

## Mandatory release gate

All PROD, ARCH, GRAPH, CAP, VDEV, DSP, PLUG, REC, UI, API, AUTO, STATE, SEC, NFR, QUAL, and ENG requirements assigned to v1 have evidence. VDEV-09 requires a production-signed driver on normal Windows security settings. The primary workflow works without a separately installed virtual mixer/plugin host. UI closure, backend crash, reboot/sign-in, disk failure, plugin failure, and user switching exhibit the specified behavior.

Performance targets are met on the declared reference hardware, with distributions and workload details published. No universal Bluetooth or arbitrary-plugin latency claim is made. At least four of five first-time users complete the setup within ten minutes and all identify Discord's source set. Keyboard/Narrator and 200% scaling checks pass. External AI control is optional, local, discoverable, and permission-constrained.

## Installer and uninstall acceptance

Install with only necessary elevation; decline elevation and return an actionable partial-install state without claiming virtual routing is ready. Update while sessions are stopped or after a user-approved stop plan; preserve configuration/recordings and validate app/driver compatibility before restart. Rollback restores a compatible package/configuration pair. Uninstall previews affected endpoints and offers configuration retention; recordings remain unless individually targeted through a separate explicit action. Verify no orphaned owned endpoint, stale startup task, privileged broker, or exposed control pipe remains.

## Evidence and handoff

Archive the release execution plan, full requirement/evidence matrix, signed artifact hashes, test-machine manifests, measured reports, security findings/resolutions, and known issues. Keep private signing material and audio out of the repository. Update README from “specification only” only when it reflects actual implemented/tested state. Start a new active maintenance plan for defects and future requests; move deferred ideas only after scope authorization.

## Stop conditions

Do not label the release complete with an unsigned/test-mode driver, missing real Windows tests, unresolved private-audio leakage, reproducible backend/driver crashes, corrupted recordings, or unfulfilled core API parity. If credentials/hardware are missing, report blocked release evidence and retain the prepared artifacts. A documented blocker is more useful than an invented pass.

Suggested request: “Execute M08 release qualification, prepare the signed Windows artifacts and complete evidence/docs, and report any exact publication/signing action still requiring unavailable authority.”
