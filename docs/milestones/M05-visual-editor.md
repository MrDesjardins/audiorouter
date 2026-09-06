# M05 — React visual editor and guided setup

Status: React/UI foundation implemented; native shell injection and manual acceptance remain open. Prerequisite: M04. Outcome: the primary workflow is understandable and configurable entirely through the desktop UI and guided external-app steps.

## Read first

[Product](../spec/01-product.md), [workflows](../spec/02-workflows.md), [architecture](../spec/03-architecture.md), [interface](../spec/09-interface.md), [API](../spec/10-api.md), and [security](../spec/13-security.md).

## Ordered implementation

1. Create the React/TypeScript/Vite app and agreed Windows desktop shell. Connect the generated API client via a transport-only bridge. Apply CSP and minimal shell permissions before exposing native actions.
2. Build the session list, source/effect/output library, canvas, inspector, status strip, meters, and recording library. Render backend snapshots and events; preserve stale/disconnected state honestly.
3. Implement drag and keyboard connection creation, explicit mixers, channel inspector, node enable/bypass/mute/remove, names/presets, undo/redo, and presentation-only layout persistence.
4. Implement template onboarding for mic/headphones/virtual buses and guided Discord/OBS/Windows selections. Verify levels and duplicate playback; keep monitoring muted and recorders unarmed initially.
5. Add route explanations, conflict/reconnect handling, visible privacy/recording state, recoverable device/plugin placeholders, and clear stop-versus-close semantics.
6. Add tray controls, optional compact/pinned meters, shortcuts with conflict detection, theme/high contrast, reduced motion, scaling, and keyboard/list alternative to the canvas.
7. Test with external CLI edits, Windows Narrator, keyboard-only routes, and first-time users. Record findings and fix workflow failures before completing the gate.

## Acceptance gate

UI-01–14 applicable baseline; PROD-02/04; SEC-05; NFR-06/08/14/15 UI portions. UC-01/02/03/08 can be completed from the UI, with external app selections clearly distinguished from AudioRouter settings. Corresponding API/CLI graphs are equivalent. A node dragged to a different position never changes its route.

Keyboard-only users can build and inspect the same routes; high-contrast/reduced-motion/200% scaling work. A missing device, backend disconnect, rejected value, stale revision, and recording failure each has an actionable visible state. Capture and processing continue with the editor closed.

## Verification

Run typecheck/lint/build and meaningful component/UI integration tests using a fake backend, then Windows packaged-app tests against real audio. Record screen-reader/manual accessibility results and the product's five-participant usability target where available; final release may not waive it. Measure meter freshness, UI responsiveness, memory, and cold/warm launch.

## Boundaries and rollback

Do not implement audio DSP in browser APIs or duplicate graph validation in React. The shell cannot bypass client permissions. A browser dev-server demo is not packaged Windows evidence. Keep new UI options out of the product if their backend capabilities are absent.

## Handoff

Document view/component ownership, generated contract workflow, keyboard commands, onboarding instructions, test fixtures, UX findings, and known accessibility limitations. M06 uses the existing generic inspector for new node types.

Suggested request: “Implement M05's React/TypeScript/Vite visual editor over the existing API, including guided gaming setup, accessible connections, live status, and parity with CLI edits.”
