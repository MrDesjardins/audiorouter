# Nightly packaged smoke repair

Objective: restore the packaged qualification's first-launch navigation.
Requirements: ENG-02/04/05, NFR-14; M08 tooling maintenance.

Reproduction: nightly run 42362489121, main 04c8a6dd399af6cce6eeb7ded2defacdd017fda1.
Downloaded packaged-smoke-qualification to target/nightly-42362489121.
Baseline, page-error, cleanup-fallback and cancel-after-restart all time out
clicking Session at packaged-shell-smoke.mjs:807. Retained HTML identifies the
blocking goal dialog, "What do you want to do?", with its "Not now" button.
The three seeded cases therefore never reach their intended assertion.

Prerequisites: Windows, installed Edge and WebView2, release shell with embedded
UI, standard-user token and a free production control pipe. No audio hardware,
driver changes or external runtime credentials are needed. Preserve the owner's
running app and unrelated UI/guide changes.

Ordered tasks: add scoped first-launch dismissal through the real UI; exercise
a delayed modal regression in Edge; run the same binary through all eleven
qualification cases where prerequisites permit; inspect the scoped diff.
Do not relax qualification expectations or grant device access.

Validation matrix: Edge regression covers delayed dialog and usable Session
navigation. Native qualification must cover baseline, intentional failures,
cleanup and all cancellation checkpoints. A fixture is not packaged evidence.
Rollback: revert only the onboarding helper, import/call and focused test.
Risk: a changed first-launch dialog should fail explicitly instead of allowing
navigation behind it. Next action: run regression and native qualification.

Windows results: `node --test tools/quality/packaged-shell-onboarding.test.mjs`
passes in real Edge (one test); `node --check tools/quality/packaged-shell-smoke.mjs`
passes. Full qualifier command with the local release shell was blocked by its
preflight: "Close the existing AudioRouter instance before qualification".
The owner's process was preserved. No full native pass is claimed. Next task:
rerun all eleven cases when the owner has closed AudioRouter, then exercise the
ordinary smoke; obtain explicit approval before opening any PR.

Final local qualification (2026-11-07): after the owner closed the app,
`node tools/quality/qualify-packaged-shell-smoke.mjs --shell src-tauri/target/release/audiorouter-shell.exe --output target/ui-quality-review/round-26/pr-nightly-qualification --timeout-ms 15000`
passed all eleven cases: baseline, missing-binary, occupied-pipe, shell-exit,
bridge, readiness, page-error, cleanup-fallback and cancellation before attach,
after readiness and after restart. Intentional failures reached their owning
checks; no qualification expectation was relaxed. Evidence:
`target/ui-quality-review/round-26/pr-nightly-qualification/qualification.json`.
The ordinary release smoke also passed with full opening-view geometry and
save/restart checks in `target/ui-quality-review/round-26/native-release/`.
Owner now explicitly authorized inclusion in one combined commit/push/PR with
all pending UI and guide work. No merge or release is authorized.
