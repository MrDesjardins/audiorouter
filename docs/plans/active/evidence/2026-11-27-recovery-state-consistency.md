# A5-A9 backend recovery repairs

Owner approved all five recommended repairs on 2026-11-27. Requirements:
RES-3, STATE-03/09/10/11, CAP-11, API-07, AUTO-18, ENG-01/02/04.
Windows, pinned Rust 1.96.0. No driver changes or eight-hour soak.

## Behavior and regressions

- A5: an edited revision, newly active recorder or failed plugin cancels
  recovery through bounded session Stop. Audio stops before recorder file
  finalization; `runtime.recoveryFailed` and the existing `status.get.reason`
  field explain the
  cancellation. Tests verify stopped state, unchanged start generation,
  preserved saved edits, finalized legacy recorder and failed-plugin latch.
- A6: device loss latches the interrupted runtime until Stop retires it.
  Direct Start, dispatcher Start and task-level Play refuse `nodeNotReady`;
  repeated requests retain eligible recovery. Stop clears the latch. Automatic
  recovery still stops the interrupted runtime before preparing exact saved
  endpoints; its budgets, backoff and endpoint choices are unchanged.
- A7: repeated Sleep merges and deduplicates portable/native restart lists.
  Tests cover two different stopped routes, explicit Stop cancelling an offer,
  delayed duplicate notifications and paused native recovery. Portable routes
  remain stopped until deliberately restarted.
- A8: crash and resume offers exclude failed-plugin routes. Resume rechecks
  faults that occurred after Sleep and records a diagnostic for exclusions.
- A9: the existing active-recorder predicate now governs session recovery
  safety, including Armed. Tests cover Armed, Recording, Paused and Stopping
  through the witness, crash decision and refused Sleep paths.

## Verification

Raw logs are local under `target/recovery-state-consistency/`.

1. Baseline fault suite: 29 passed (`baseline.log`). Initial changed assertions
   failed in five cases (`regressions-before.log`). With the expanded tests and
   production fixes temporarily stashed, 10 cases failed, including A8 and A9
   (`expanded-before.log`). Production changes were then restored.
2. `cargo test -p audiorouter-control -p audiorouter-cli --lib --locked`:
   527 control and 72 CLI tests passed; 31 control and one CLI tests ignored.
   Includes all 32 fault-interleaving tests, seeded scheduler, dispatcher,
   default MCP reachability, and `mcp_tool_call` repeated Sleep plus default
   `control_session` forwarding of the existing `nodeNotReady` error.
   The MCP error test uses an unprepared fixture, not a physical device loss.
3. `cargo test -p audiorouter-transport --lib --locked`: 43 passed, one ignored;
   native pipe lifecycle, authorization and concurrent clients included.
4. Workspace and `src-tauri` strict Clippy, all targets and features: passed.
   Both Rust formatting checks and `git diff --check`: passed.
5. Contract drift: passed, 156 methods, 38 node kinds, 20 processors,
   24 event categories. No public schema, API method, default MCP catalog,
   NodeKind setting or UI control was added. Documentation validation passed.

## Limits and next gate

Fault injection uses the same backend markers read in production, with fake
runtime/device workers. It exercises real dispatcher and MCP adapters but
does not establish audible recovery or recorder file integrity on physical
unplug/suspend. No fresh desktop executable, UI interaction, live-device
unplug, actual OS sleep, installer run or long soak was performed for these
repairs. Existing ignored hardware gates remain ignored, not waived.

Next: short attended exact-endpoint loss/resume qualification when a quiet
device window is available. Keep the independent frontend qualification branch
intact. PR creation still requires explicit human approval.
