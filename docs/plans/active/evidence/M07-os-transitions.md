# M07 OS-transition policy evidence

Date: 2026-09-15

The control crate now exposes a side-effect-free `plan_os_transition` contract
for STATE-11. Lock keeps the explicitly running session set. Sign-out and
sleep request stop/release for every running session. Resume only returns
non-native, non-recording sessions for endpoint revalidation before restart;
protected sessions remain stopped. IDs are sorted and deduplicated before a
decision is returned.

The control plane now applies the stop/release boundary through
`ControlPlane::handle_os_transition`. It refuses a sleep/sign-out transition
when an active recorder still needs explicit finalization, preserving the
running session rather than silently losing recording data. A successful sleep
stores only eligible portable sessions; resume consumes that bounded set and
returns `revalidateBeforeRestart` without starting audio itself.
On resume, an existing endpoint monitor is force-refreshed through its
read-only snapshot path before the response is returned; the response reports
`endpointInventory: refreshed` or `notStarted`.

Sleep also preserves the exact IDs of native-owned sessions separately from
portable restart candidates. Resume returns those IDs as `nativeSessionIds`
and requires `revalidateBeforeRestart`; it does not restart or reopen a native
endpoint. This keeps hardware and driver recovery explicit while preserving
the identity needed by the future exact-rebind operation.

The same boundary is exposed as the authenticated, idempotent
`system.osTransition` JSON-RPC method so a native notification adapter can use
the shared backend authority. Its input and output schemas are included in
method discovery and the readable API reference. The TypeScript contract and
UI backend adapter now expose the same typed method to connected hosts; the
disconnected adapter fails closed.

The MCP adapter now exposes a focused `os_transition` tool with the same
transition enum and required idempotency key. It is non-read-only and
idempotent, and dispatches through the authenticated control plane; the MCP
adapter owns no transition state.

The Tauri shell now owns a Windows message-only listener in
`src-tauri/src/os_transition_windows.rs`. WTS session notifications map the
current user's lock and logoff events; `WM_POWERBROADCAST` maps suspend and
automatic resume. Delivery uses a bounded `SyncSender::try_send`, while a
separate shell thread forwards authenticated RPC requests. The listener uses
the actual listener thread ID for shutdown and releases its window callback
state during `WM_NCDESTROY`.

Validation:

```text
cargo test -p audiorouter-control --locked --lib -- --test-threads=1
165 passed; 2 ignored; 0 failed
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
clean
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --test-threads=1
22 passed; 0 failed
npm.cmd run typecheck --prefix contracts
clean
npm.cmd run typecheck --prefix ui
clean
npm.cmd test --prefix ui -- --run
217 passed; 0 failed
cargo test -p audiorouter-cli --locked
36 passed; 0 failed
```

This is portable policy evidence, not Windows power-notification evidence.
The native listener startup/teardown smoke test now passes, but attended
delivery still needs a guarded lock/sign-out/sleep/resume acceptance and
endpoint re-enumeration with
before/after identity proof. No machine power state, audio endpoint, driver,
or user configuration was changed by this work.

The shell transition forwarder uses a process-and-timestamp-qualified
idempotency key, preventing a newly launched shell from colliding with a
durable transition operation from an earlier shell instance. The uniqueness
regression is covered by the native shell test suite.
