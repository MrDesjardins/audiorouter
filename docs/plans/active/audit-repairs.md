# High and medium audit repairs — 2026-11-12

## Objective and authority

Owner requested all three high and twelve medium findings in
[the audit](bug-audit-2026-11-11.md), regression tests, a new branch and a PR.
Branch: `fix/audit-high-medium`, isolated at `target/audit-repair` from
`a2dd275e`. The original checkout's unrelated changes are preserved.
The owner's request explicitly authorizes opening this PR after validation.

## Requirements and tasks

STATE-02/04/05/07/09/12, REC-01/07/08/09, graph atomicity and DSP dynamics
requirements govern these repairs. No release, driver installation or owner
audio changes are authorized or needed.

1. Reproduce SES-1 and fix deletion ordering (first independent repair).
2. Repair MEDIA-1, STO-5 and SES-2 with dispatcher/storage/failure coverage.
3. Repair DSP-1/2, ENG-1, REC-1, STO-1/2/3, SES-3 and SIM-1.
4. Reproduce STO-4 before changing path security; retain name-surrogate
   escape protections. Record a rejected finding if evidence disproves it.
5. Run focused tests before and after each fix, then formatting, relevant
   Clippy, workspace tests, shell checks, contracts drift and docs acceptance.
6. Inspect the complete diff, commit focused repairs, push and create PR.

## Prerequisites and validation

Windows is available. Builds use one job to limit interference with live
audio; no owner process or session is stopped. Tests use disposable data.
Real OneDrive placeholders, physical endpoints and attended shell evidence
must be distinguished from unit tests. Product compilation and dispatcher
tests do not prove live hardware behavior. Record exact results below.
API/schema changes require contracts and default MCP parity checks. Any
changed user controls require guide and three-theme visual qualification.

## Risks and rollback

Startup isolation must retain unreadable data and keep privacy/device latches
fail-closed. Stop must silence audio despite file failures. Route edits must
preserve ports, matrices, flags and Mixer volume. Backups must not overwrite
existing destinations. Rollback is reverting this branch's focused commits;
do not delete or alter the owner's original checkout or live database.

## Evidence and next action

Initial inspection: original checkout has unrelated tracked and untracked
plugin/UI work; isolated worktree created successfully at the recorded base.
SES-1: `cargo test -p audiorouter-control --lib
refused_delete_preserves_saved_version_notes -j 1` failed before the fix:
the saved timestamp became null. After moving cleanup past all guards and
durable deletion, `cargo test -p audiorouter-control --lib refused_delete -j 1`
passes 2/2, including persisted notes after reopen and comparison expiry.
An attempted native-free `nodes.compare` setup correctly refused an unattached
route; the comparison test seeds only the hold and uses real delete dispatch.
Both workspace/shell formatting commands and checks pass. Control all-target,
all-feature Clippy with `-D warnings` passes (Windows, Rust 1.96.0).

MEDIA-1: dispatcher deletion regression fails because used Soundboard media
was deleted; storage regression fails because pad expiry is retained.
Next: centralize exhaustive node media references and use them in all four
owners, then rerun these failures and broader media/bundle tests.
