# M07 OS-transition policy evidence

Date: 2026-09-14

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

Validation:

```text
cargo test -p audiorouter-control --locked --lib -- --test-threads=1
164 passed; 2 ignored; 0 failed
```

This is portable policy evidence, not Windows power-notification evidence.
The native shell still needs a guarded adapter for lock, sign-out, sleep, and
resume notifications, plus endpoint re-enumeration and before/after identity
proof. No machine power state, audio endpoint, driver, or user configuration
was changed by this work.
