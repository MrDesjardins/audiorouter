# M07 OS-transition policy evidence

Date: 2026-09-14

The control crate now exposes a side-effect-free `plan_os_transition` contract
for STATE-11. Lock keeps the explicitly running session set. Sign-out and
sleep request stop/release for every running session. Resume only returns
non-native, non-recording sessions for endpoint revalidation before restart;
protected sessions remain stopped. IDs are sorted and deduplicated before a
decision is returned.

Validation:

```text
cargo test -p audiorouter-control --locked --lib -- --test-threads=1
162 passed; 2 ignored; 0 failed
```

This is portable policy evidence, not Windows power-notification evidence.
The native shell still needs a guarded adapter for lock, sign-out, sleep, and
resume notifications, plus endpoint re-enumeration and before/after identity
proof. No machine power state, audio endpoint, driver, or user configuration
was changed by this work.
