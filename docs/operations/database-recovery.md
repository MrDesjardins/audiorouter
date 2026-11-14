# Recovering saved configuration

AudioRouter keeps one consistent database backup per UTC day in
`state.backups` beside `state.sqlite`, retaining the ten newest daily copies.
Backups include imported Audio File and Soundboard audio. Before upgrading an
older schema, AudioRouter also keeps a pre-migration copy in that folder.
Recordings stored outside the database are not copied or pruned by this process.

If the desktop cannot open a damaged database, the startup screen lists backups
that pass SQLite integrity and application-schema checks. Choose a backup,
press **Restore selected backup**, read the explanation, then confirm. No
restoration happens automatically. A database from a newer AudioRouter instead
asks you to update the app; it is not treated as corruption.

Restoration first prepares a separate validated copy. It preserves the damaged
database and its WAL/SHM files in a new `damaged-<process>-<time>` folder beside
the database before publishing the replacement. Keep that folder if you need
help diagnosing data loss. Failed validation leaves the original files in place.
If publication fails, the shell attempts to put the originals back and retains
the preservation folder for diagnosis.

Restored routes start stopped and privacy mute is on. Client enrollments and
capture-device consent are reset so an older backup cannot revive revoked
access. Review your devices and route, allow device access again, and reconnect
external clients deliberately. Recordings do not resume. Startup registration
is not changed by restoration; the restored backend startup preference is off.

If no validated backup is available, the damaged data remains in place. Keep a
copy for diagnosis; do not replace it with an empty database. The CLI's explicit
backup and restore-to-new-file commands remain available as described in the
[headless runbook](headless-runbook.md).
