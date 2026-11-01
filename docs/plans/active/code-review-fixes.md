# Active plan — code-review fixes (2026-11-01, owner request)

Owner request, 2026-11-01: "do the two code review markdown in the future
folder. Set them in current and tackle all of them." Sources:
[code review 2026-11-01](../future/code-review-2026-11-01.md) and the items
still open in [code review 2026-10-07](../future/code-review-2026-10-07.md).

- Branch `feature/code-review-fixes-2026-11-01`, created from
  `review/code-review-2026-11-01` (`9a0c4067`). No pull request without the
  owner's approval (AGENTS.md).
- Builds and test suites: the owner allowed them on 2026-11-01 while their
  app runs ("Build now").
- One commit per item or small group, each with its regression test.
  Mechanical file splits are separate pure-move commits.

## Owner decisions (2026-11-01)

1. **P2-7 backups:** raise the backup budget so it covers the audio media
   limit, and say so in STATE-12 (option "raise the budget"). Not a separate
   media export.
2. **2026-10-07 P2-5 remainder:** encrypt network audio and protect a
   restarted receiver from replay now, not later.
3. Work on a new branch from the review branch.

## Requirement IDs

PLUG-05 and PLUG-07 (plugin latency and failure policy), SEC-13 and
GRAPH-16 (network audio), STATE-12 (backup budget), ARCH-04 and the
audio-callback rules (work on the audio service thread), AUTO-18 (API and
MCP output), SND-* (Soundboard memory), UI-17 (unsaved-state report).

## Triage and tasks

Status key: **todo**, **done** (code and regression test committed),
**blocked** (needs Windows hardware, a measurement or the owner).

### From the 2026-11-01 review

| Item | Task | Status |
| --- | --- | --- |
| P2-2 | VST3 worker answers Latency from its cache on the pipe thread; a plugin-requested restart is posted to the UI thread and runs under a processing lock; the worker's UI timeout drops below the host's 5 s response timeout. Fixture mode on the Rust worker for a slow latency answer; host test that a slow poll does not fail the worker | todo |
| P2-1 | One unchanged-audio watch for a whole VST2 chain (chain input vs. final output); the chain is guarded when any member is; tests on bridge behavior | todo |
| P2-4 | One exhaustive `node_latency_samples` in the domain, used by the catalog and `inspect_routes`; engine sample rate instead of 48; exhaustive cost class; remove unused `routeLatencyText` or use it | todo |
| P2-3 | Plugin shared memory on `MmapRaw`, raw-pointer access, volatile payload copies and acquire/release fences | todo |
| P2-7 | Backup budget covers the media budget; source opened read-only without migration; STATE-12 and a decision record | todo |
| P2-8 | Soundboard Play: manifest parsed once and indexed; file read and hash inside `while_servicing_audio`; blobs loaded in bounded steps | todo |
| P2-9 | Full integrity check once per app start; light open for the shell's small reads; corrupt-database result logged | todo |
| P2-5 | `App.tsx` and `SessionFlowCanvas.tsx` split by pure moves; engine and windows-audio splits | todo |
| P3-1 | Doc comment; HTTP header `BufReader`; sample-rate-derived poll and watch windows; `unsavedReport` diagnostic; MCP stdio bounded byte reads with -32700 for bad UTF-8; SID passed from the I/O thread and a pipe read timeout; session-wide Soundboard memory budget; denormal bench | todo |
| P2-6 | Carried items from 2026-10-07 (below) | see below |
| Branch | `pluginSkipReason` exhaustiveness is on the unmerged `feature/plugin-library-ux`; fix it on that branch, not here | todo |

### From the 2026-10-07 review

| Item | Task | Status |
| --- | --- | --- |
| P0-1 remainder | SQLite reads/writes at Play, export/import and backups moved off the audio service thread or into bounded steps | todo |
| P2-1 remainder | Typed method table for `dispatch` | todo (behavior-sensitive; own commit) |
| P2-2 remainder | `AppContent` handlers into hooks (with P2-5 above) | todo |
| P2-5 remainder | Network audio encryption (ChaCha20 with HMAC-SHA256, encrypt-then-MAC, packet version 3) and replay protection for a restarted receiver (authenticated sender time, bounded skew) | todo |
| P2-7 | Windows sine-continuity run for the optimized DSP loops | blocked: needs VB-Cable route and a quiet PC |
| P3 UI bundle | Lazy-load rarely used panels if a fresh build still shows a large main chunk | todo |
| P3 silent catches | Diagnostic categories for the remaining swallowed UI errors | todo |
| P3 unsafe audit | Bridge shared memory `MmapRaw` (with P2-3) | todo |

## Validation matrix

| Check | When |
| --- | --- |
| `cargo fmt --all -- --check`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | every Rust commit |
| Clippy workspace and `src-tauri`, all targets and features, `-D warnings` | every Rust commit |
| `cargo test` for the touched crates (`plugin-host` with `test-fixtures`, `domain`, `storage`, `control`, `protocol`, `windows-audio`, `cli`, `transport`) | per item |
| VST3 worker build (`tools/m06-vst3-worker/build.ps1`) and its acceptance script | P2-2 |
| UI format, lint, typecheck, Vitest; Playwright for moved UI | UI items |
| `npm --prefix contracts run check:drift`, API reference regeneration | API output changes |
| `tests/acceptance/docs.ps1` | every docs change |
| Sine-continuity harness, `AudioServicePassTiming` measurements | blocked items; record when run |

## Risks

- P2-2 changes the VST3 worker's threading; a restart racing a process call
  must be impossible (processing lock) and is covered by review plus the
  acceptance fixture.
- Network audio version 3 does not pair with AudioRouter 0.0.26 or older;
  both computers must be updated. The receiver says so ("senderOutdated").
- Clock skew larger than the allowed window between two paired computers
  would stop audio; the receiver reports `clockSkew` so the user can fix the
  time setting.
- Raising the backup budget makes a backup of a full library take longer.

## Rollback

Revert the item's commit. No storage migration is added; the network packet
version change is reverted together with its UI note.

## Progress

- 2026-11-01: plans tidied (Soundboard, Output Switch, Wave Link first
  round, release 0.0.24/0.0.25 and the old `current.md` archived); this plan
  created.

## Next action

P2-2, then the items in the order of the tables.
