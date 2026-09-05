# 12 — Persistence, sessions, and operational lifecycle

Milestone ownership: M01 storage/revisions/import schema; M04 recording state; M07 recovery/startup; M08 migration/update validation.

## Requirements

- **STATE-01 — Storage.** Use a transactional local SQLite database for backend-owned configuration, revisions, operations, client grants, and recording index, with versioned files for large plugin states. Store per-user data under a documented LocalAppData directory; machine driver inventory belongs to a separate protected location. UI/CLI never write the database directly.
- **STATE-02 — Revision durability.** Persist committed graph changes atomically with operation outcomes and a recoverable journal. Keep a last-known-good snapshot and bounded history. On corruption, preserve the original database for diagnosis, offer validated backup restoration, and start stopped. Never replace a corrupted configuration with an empty “successful” setup without notice.
- **STATE-03 — Desired/observed separation.** Saved bindings, startup preference, and node parameters describe desired state. Device availability, active revision, metering, and failure state are observed runtime data. Temporary device loss must not rewrite a session or clear the user's graph.
- **STATE-04 — Session management.** Create, name, duplicate, export/import, delete, and run multiple sessions subject to global resource limits. Duplication generates new entity IDs and keeps endpoint references unresolved if ownership would conflict. Deletion requires stopped state and never deletes recordings automatically.
- **STATE-05 — Import/export.** Define a versioned `.audiorouter` ZIP bundle containing a JSON manifest, graph, optional UI layout, and referenced plugin/preset assets. Exclude credentials, grants, raw recordings, plugin executables, machine-specific absolute paths, and automatic driver actions. Present rebindings for missing devices/apps/plugins. Imported sessions are stopped, recorders unarmed, and startup disabled.
- **STATE-06 — Import validation.** Validate schema, type versions, node/edge budgets, unique IDs, hashes, sizes, and archive paths before extraction. Maximum bundle is 100 MiB compressed, 250 MiB expanded, 1,000 entries; a plugin state asset is at most 16 MiB. Reject traversal, absolute paths, duplicate paths, symlinks, decompression bombs, and unknown executable content. Extract to a bounded staging directory and commit only after successful validation.
- **STATE-07 — Migration.** Each schema migration has a tested before/after fixture, a backup, and a compatibility policy. Never silently open a newer unsupported schema for write. Downgrade either restores the pre-upgrade backup or refuses clearly; do not assume every migration has a reversible SQL inverse.
- **STATE-08 — Startup.** Start-at-sign-in is opt-in per user. Only designated sessions start; recorders require separate explicit recording-autostart consent, which is off in v1 templates. Persistent privacy mute survives restart. Show active microphone/recording state from the tray even when the editor is closed.
- **STATE-09 — Shutdown.** Closing the window disconnects that client only. Stopping a session fades outputs, releases ownership, and finalizes recorders. Quitting the backend stops all sessions with per-recorder outcomes, bounded to ten seconds; timed-out finalizations are marked recoverable/failed. Endpoint drivers remain installed until an explicit uninstall.
- **STATE-10 — Recovery.** On backend crash, endpoints go silent, then optional supervisor restart uses the last committed graph and prior startup/recovery policy. Default is to restore previously running non-recording routes after one recoverable crash, preserving privacy mute; after three crashes in ten minutes start in safe mode stopped. Recording never silently resumes into an old file.
- **STATE-11 — OS transitions.** On lock, keep explicitly running sessions according to saved policy; default permits a live call to continue and retains visible tray status when unlocked. On sign-out stop and release ownership; on sleep suspend streams; on resume re-enumerate and revalidate before continuing. No cross-user automatic microphone substitution.
- **STATE-12 — Backups and retention.** Keep at least the ten most recent daily configuration backups and pre-migration backup, subject to a documented 100 MiB default configuration backup budget; large referenced plugin assets are deduplicated and counted. Never prune recordings as part of configuration retention. Provide explicit export and restore preview via API.

## Bundle manifest example

```json
{
  "format": "audiorouter.session",
  "schemaVersion": 1,
  "createdWith": "0.1.0",
  "graphPath": "session.json",
  "assets": [],
  "requiredNodeTypes": ["physical-input@1", "gain@1", "physical-output@1"],
  "bindingHints": [{"bindingId": "mic-binding", "direction": "capture", "label": "USB microphone"}]
}
```

This illustrates the manifest shape; M01 freezes exact schemas. Binding hints aid user selection and do not authorize binding to a different device. Export preserves graph meaning and parameter values; import allocates new IDs when conflicts would otherwise overwrite an existing session.

## Recovery evidence

Terminate the backend at every journal stage and verify one of two outcomes: the previous committed revision or the new committed revision, never a torn graph. Reissue a previously committed idempotency key and verify no duplicate recording/device/node. Corrupt a copy of the database and demonstrate preservation/restoration. Test old/new schema fixtures, missing plugin assets, settings restored on a new machine, and bundle traversal rejection. Driver state and per-user configuration mismatch must be repairable through an explicit reconciliation plan.
