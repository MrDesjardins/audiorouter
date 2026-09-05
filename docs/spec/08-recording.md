# 08 — Recording and file management

Milestone ownership: M04 core recorder/library API; M05 UI; M07 recovery; M08 endurance and disk-failure evidence.

## Requirements

- **REC-01 — Independent sinks.** A Recorder node subscribes to a graph branch and never gates other audio. Allow eight concurrent recorders globally within resource limits. Recording processed voice, raw voice, desktop, and a mixed feed simultaneously uses explicit branches. Each file is mono or stereo in v1.
- **REC-02 — Formats.** v1 shall write WAV PCM16, PCM24, or float32, and FLAC 16/24-bit. Default is WAV PCM24, 48 kHz, matching the connected channels. Support 44.1 or 48 kHz export with explicit conversion. Recorders may run different formats simultaneously. MP3/AAC/ALAC/AIFF and custom external encoders are future.
- **REC-03 — Dither.** Integer quantization uses configurable TPDF dithering, enabled by default when reducing precision. Floating output is not dithered. Record format, conversion, and dither settings in recording metadata. Limiting/normalization is an explicit upstream effect, not silently applied during export.
- **REC-04 — Controls.** Arm, start, pause, resume, split, and stop are API operations. An unarmed recorder does not start with a session. Pause omits frames from that file while maintaining a sidecar timeline of pause intervals. Stop drains queued frames and finalizes, or returns a failure with recoverable partial-file details. Closing the UI has no recording effect.
- **REC-05 — Time and synchronization.** Recorders scheduled in one start operation share a graph-frame boundary. Use monotonic sample counters for alignment; UTC is descriptive. Files on paths with different DSP delays store start frame and path delay; offer a synchronized group mode that compensates within the graph budget. Do not claim that simultaneous commands alone make independently delayed paths sample-aligned.
- **REC-06 — Splitting.** Support manual split and automatic duration/size thresholds. Default WAV split occurs before 2 GiB or at 24 hours, whichever comes first; account for headers and complete sample frames. Other formats default to the same operational limits. Split boundaries preserve all frames exactly once. Advanced silence trimming/splitting is future.
- **REC-07 — Naming and paths.** Choose a user-approved recording root and filename template using allowlisted tokens: session, recorder, UTC date/time, and monotonic sequence. Sanitize Windows reserved characters/names and collisions; use atomic exclusive file creation. Never overwrite an existing recording. Resolve canonical paths and enforce grants against traversal, UNC/network roots, and reparse-point escapes.
- **REC-08 — Bounded writing.** Audio pushes to a fixed-capacity queue, default two seconds per recorder. File allocation, encoding, hashing, and metadata are off-thread. On overflow, fail the affected recorder, preserve the written prefix, and mark the gap/cause; never block live audio or silently omit samples in a file labeled complete.
- **REC-09 — Failures and recovery.** Detect disk full, revoked permission, removed disk, encoder crash, and invalid destination. Maintain enough off-thread checkpoints to recover a WAV prefix after abrupt termination. M04 must document FLAC partial-file recoverability and mark irrecoverable tails honestly. Temporary files remain identifiable and appear in a recovery list on next start.
- **REC-10 — Library.** List recordings per session with format, channels, sample rate, duration, size, start time, state, path, and missing-file status. Support rename, basic title/artist/comment metadata, reveal in Explorer, preview, remove library entry, and separately recycle file. Missing files do not crash the library. Playback preview selects an output explicitly and warns if that output is being captured.
- **REC-11 — Destructive actions.** Removing a library entry leaves the file intact. Recycling a file is a separately authorized action identifying exact paths and affected entries; use the Windows Recycle Bin where supported. If recycling is unavailable, return `recycleUnavailable`; do not silently permanently delete. Bulk actions preview every target.
- **REC-12 — Privacy.** Recording is visible in global status, node state, and API events. Import, preset application, API discovery, and ordinary session open shall not start recording. Recording permissions apply equally to UI, CLI, and MCP. Logs/support bundles contain recording metadata only after redaction, never audio by default.

## State machine

`idle → armed → recording ↔ paused → stopping → completed`, with `failed` from any active stage. A new start creates a new recording ID/file set. Node configuration and historical recording objects are separate. Stop is idempotent; retrying the same start idempotency key cannot create another file. A timed-out caller queries operation status before retrying with a new key.

Split requests return the old and new recording-part IDs and exact boundary frame. Paused recorders split by closing the old part and preparing the next; the next receives samples only on resume. Stopping a session finalizes all active recorders with individual results. A failed recorder does not stop other recorders or outputs.

## Crash durability target

Checkpoint at most every second of encoded data, without adding realtime work. Following an ordinary process crash, a WAV recovery test shall lose at most the final two seconds and identify exact recoverable duration. Sudden power loss/storage hardware failure cannot have the same guarantee unless measured with the selected filesystem/durability policy; document that limit. Support both graceful shutdown and forced-termination test evidence.

## Acceptance

Use synchronized impulses in separate mic/desktop fixtures, pause/resume, and force split at a small test threshold. Decode every file to count frames and check no split duplication/loss. Simulate slow/full storage and crash the encoder while a continuous voice path is monitored for gaps. Verify missing-file library behavior and that remove-entry leaves the bytes untouched. Verify duplicate filenames cannot overwrite previous recordings. Test Unicode and long Windows paths within the declared support limit.
