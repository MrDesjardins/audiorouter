# M01 contracts and control-plane evidence

## Scope and boundary

This report covers the portable M01 foundation implemented while M00 Windows capture, process-loopback, physical-latency, and managed-driver gates remain blocked. It does not claim a Windows named pipe, real audio activation, driver provisioning, or realtime behavior.

## Implemented components

- `crates/domain`: opaque IDs; session/node/port/edge contracts; graph limits; direction/channel/matrix/dangling/duplicate/cycle validation; node registry; fake runtime generation lifecycle; revision-checked plans; idempotent commit replay; plan expiry.
- `crates/control`: discovery metadata; session reads; graph plan/commit; storage-backed construction; fake session start/stop; JSON-RPC dispatch; batch handling; scoped grants; notification semantics.
- `crates/protocol`: 4-byte little-endian framing, 4 MiB maximum frame, JSON-RPC request/response types, 32-request batch limit, malformed/version/method validation.
- `crates/storage`: SQLite schema migration, transactional session JSON/history writes, validated bounded import/export, online SQLite backups, client enrollment/revocation records, and idempotent operation journal.
- `crates/cli`: offline `help`, `status`, `schema`, `devices list`, `apps list`, `nodes types`, and `api methods` commands with human and `--json` output.
- `tests/fixtures/valid-session.json`: checked-in camelCase contract fixture.
- `tests/acceptance/m01-cli.ps1`: offline CLI acceptance script.

## Reproducible checks

From the repository root:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo check --manifest-path tools/m00-wasapi-probe/Cargo.toml
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m01-cli.ps1
```

On 2026-09-05, the workspace suite passed 3 CLI, 14 control, 12 domain, 5 protocol, 6 storage, and 9 transport tests. The standalone WASAPI probe checked successfully. The CLI acceptance script reported `M01 CLI acceptance passed`. The script-policy bypass was process-scoped; Windows execution policy was not changed.

## Requirement evidence

Portable evidence supports the domain/control portions of ARCH-01/03/06/10/12, GRAPH-01/02/03/05/06/07/08/09/12/13, API-01/02/03/04/05/06/09/10/11, AUTO-02/03/04/09/10/11/12, and ENG-01/02/04. Storage and fake lifecycle are foundations for STATE and persistence requirements, not final crash-recovery proof.

Still not evidenced: durable journal crash injection; control-plane integration of persistent enrollment/revocation; backup restore policy and path restrictions; named-pipe singleton/concurrency stress; real endpoint discovery/activation; process-tree capture; driver lifecycle; realtime callback safety; physical latency; and M02 hardware acceptance. M00 remains open and M01 is not a releasable product gate.

## Next action

Implement durable client enrollment/revocation, singleton/concurrency stress, journal crash injection, and SQLite backup/import limits over the now-tested local transport. Keep a portable fake transport for deterministic tests and do not add an HTTP listener.
