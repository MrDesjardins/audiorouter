# M07 automation and recovery evidence

## 2026-09-06 — CLI graph plan/apply files

Implemented the portable AUTO-04 CLI slice in `crates/cli`:

- `graph plan <session-id> --base-revision <n> --file <candidate.json> --output <plan.json> --database <path>` validates the candidate through the shared `graph.plan` dispatcher and writes a `audiorouter.graph-plan` schema-versioned JSON envelope.
- `graph inspect <plan.json>` validates the envelope and returns it without opening storage or mutating state.
- `graph apply <plan.json> --idempotency-key <key> --database <path>` reads the current session revision, rejects stale plans before planning, then replans and commits through the same control-plane methods.
- Plan files use exclusive creation to avoid accidental overwrite. Candidate/session identity and absolute file/database paths are checked.

Regression coverage verifies an independent CLI-process-equivalent plan, inspect, and apply sequence. `cargo test -p audiorouter-cli` passes 6 tests. The implementation is configuration-only: it does not open audio devices, install drivers, or change machine audio settings.

Remaining AUTO-04 work includes durable server-side plan retention/expiry across backend restarts, warning acknowledgments, and parity for all convenience commands.
