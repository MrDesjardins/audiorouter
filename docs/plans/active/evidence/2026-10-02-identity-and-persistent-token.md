# Node/session identity and persistent API token

Windows 11 x64, 2026-10-02. User explicitly approved implementation.
Requirements: UI-05/11/12, API-09, HTTP-03, SEC-01/10.

## Changes and decision

Node Properties and Session tab display a muted, selectable full identifier,
keyboard-accessible copy icon, brief copied status and manual-copy fallback.
Shared theme variables and existing button styles apply.

User supersedes HTTP-03's activation-only credential lifetime: generate once,
store with current-user Windows DPAPI, reuse after Stop/Start or app restart.
API remains disabled at launch. Generate new token requires explicit replacement
confirmation and revokes the previous credential. Corrupt storage fails closed;
explicit regeneration repairs it. No plaintext token is stored in configuration,
tracked files or logs. Storage is `%LOCALAPPDATA%\AudioRouter\api-token.dpapi`.

The user's supplied current token was saved there privately. Ignored example
configuration pins `patrick-main-native`, Mixer `mixer-1`, game `siege-eq`,
Discord `application-capture-1`. The standalone example reads DPAPI privately.
`npm.cmd run inspect` authenticated to the real running HTTP API and resolved
those targets without changes to audio. Stats.cc installation remains unchanged.

## Verification

| Command / environment | Result |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked`, Windows | 44 passed, 1 attended fixture ignored; encrypted reload/rotation/corruption and real loopback HTTP restart/old-token rejection |
| `npm.cmd run typecheck`, `ui` | Passed |
| `npm.cmd test -- NodeIdentity.test.tsx ApiPanel.test.tsx`, `ui` | 7 passed; copy failure/long ID/selection/unmount, explicit token replacement, reveal/hide |
| Production Vite build and `npx.cmd playwright test --config playwright.usability.config.ts --grep "node and session identity"`, Edge | 3 passed, 1280×720 dark/light/high-contrast; keyboard node copy and session copy |
| `npm.cmd test`, example | 11 passed; phase policy, targeting, live fixture service, retries and shutdown |
| `npm.cmd run build`, `ui` | Passed; current embedded bundle prepared |
| `cargo build --manifest-path src-tauri/Cargo.toml --release --features custom-protocol --locked` | Passed; review executable, no installer |

Initial sandboxed production-fixture write returned EPERM; approved rebuild
succeeded. Browser screenshots are retained under `identity-token/` beside this
file. Browser API controls do not establish native Tauri clipboard or attended
replacement acceptance. Tests use disposable tokens, not the user's.

Reviewed all nine screenshots: full identifiers/copy controls fit within the
sidebar, with visible focus and readable feedback in all themes. Documentation
and diff checks pass. Unpublished review executable and matching v0.0.3 companion
executables are in `target/reviews/identity-token-20261002/`. Shell SHA-256:
`e8532ac6b7598b79f9f7ce4c219b007abb10ed1cc38b70ad2295f8207cd06792`.
Shell timestamp follows current `ui/dist/index.html`; bundled JavaScript contains
the new token and identity strings. No installer was generated.

## Limitations and rollback

### Token visibility correction

User reported replacement token unavailable (same date). Cause: token controls
were nested under the running listener URL; stopped regeneration also returned
no token. Opening API now explicitly reads the saved token, stopped reveal and
regeneration return it, and the UI renders a selectable two-line field with Copy
and optional Hide independently of listener state. Start/Stop retain the shown
credential. No user's saved credential was read into tool output or replaced.

Windows checks: `npm.cmd test -- ApiPanel.test.tsx` 3 passed (stopped copy,
start/stop, hide/reveal, replacement/cancel, missing clipboard); typecheck and UI
build passed. Shell suite 44 passed, 1 ignored; custom-protocol release build
passed. Production Edge `--grep "saved token visibility"` 3 passed; all three
synthetic-token screenshots in `identity-token/` reviewed for fit/readability.
Updated review folder: `target/reviews/token-visibility-20261002/`, with matching
v0.0.3 companions. Shell SHA-256:
`1919cfabb5e974b2a5c955b7f667e1513c0d51fe4333c8913465140447dcb07c`.
Embedded asset strings/timestamps verified. The user runs the prior review build;
no app was stopped, no native attended UI or fresh-install check claimed.

The running older executable still has activation-only tokens. Use the updated
review build for persistence; no user-owned process was stopped or second shell
launched. Real Stats.cc feed delivery/gameplay and heard sample levels remain
pending. No installer/release or fresh-install qualification is claimed here.

Rollback to v0.0.3 regenerates on activation, requiring integration credential
updates. Session/audio formats are unchanged. Stop the example and restore both
Mixer input levels to 100% before removing it. Next task is attended Stats.cc
feed setup and prep/action validation.
