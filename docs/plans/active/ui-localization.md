# UI language support plan

Date: 2026-11-19. Status: foundation, everyday UI surfaces and the native tray
text path implemented 2026-11-19 after the owner asked to execute the plan.
Remaining slices and gates are listed under “Execution record” below.

## Objective and recommendation

Make AudioRouter's interface available in multiple languages, with one editable
JSON file per language in a central folder. Adding a translation should require
copying the English file, translating it, and running validation; it should not
require editing React components or maintaining another language list.

Use i18next and react-i18next for translation, language switching, interpolation
and language-specific plurals. Keep English as the reference and fallback.
Bundle translations with the app so language selection works offline. Do not
build a custom translation engine or require a translation service.

This is an M05 extension, with M08 accessibility and packaged-app evidence.
Existing requirements: UI-01/04/05/08/10 (guidance, states, errors and tray),
UI-11 (accessibility), UI-17 (stable layout), UI-19/29 (preferences/Settings),
UI-33 (search). Proposed acceptance IDs L10N-01 through L10N-08 below are local
to this plan until approval; implementation will synchronize the interface
specification and delivery traceability without renumbering existing IDs.

## Proposed language-file layout

```text
ui/src/i18n/
  locales/
    en.json
    fr.json                  example, not a selected launch language
    de.json                  example, not a selected launch language
  index.ts                   initialization and locale discovery
  format.ts                  locale-aware display formatting
  generated/                 generated types/native resource output
```

Each file contains metadata and nested messages. For example, a small excerpt
of en.json would be:

```json
{
  "meta": { "nativeName": "English", "direction": "ltr" },
  "messages": {
    "common": { "save": "Save", "cancel": "Cancel" },
    "settings": { "language": "Language", "systemLanguage": "System default" },
    "session": {
      "deleteConfirm": "Delete session \u201c{{name}}\u201d?",
      "selected_one": "{{count}} session selected",
      "selected_other": "{{count}} sessions selected"
    }
  }
}
```

The filename is a canonical BCP 47 language tag, such as en, fr, pt-BR or
zh-Hant. Metadata supplies the selector label and writing direction. Discover
files with Vite's build-time glob; generate the native-shell resources from
the same files. Generated files are never translator inputs. There is no
second hand-maintained locale registry or Rust translation catalog.

Use stable semantic keys such as settings.language, not English sentences.
Organize sections by feature inside each language file. Translate complete
sentences, with named placeholders; never concatenate translated fragments.
Share genuinely identical actions, but keep separate keys when meaning differs.
Use i18next plural forms appropriate to each language, not an English singular/
plural assumption. Keep translator context and an audio terminology glossary
in a companion contributor guide; JSON does not support comments.

## Behavior and boundaries

- **L10N-01 — Central resources.** All AudioRouter-owned interface text uses
  the catalog: tabs, controls, tool descriptions, parameter captions/options,
  help, onboarding, search, empty/loading states, dialogs, notifications,
  tooltips, accessibility names, charts and native tray/app-owned dialog text.
  An inventory must account for backend-seeded descriptions and schema text,
  not only JSX. Windows-owned picker/chrome text follows Windows language;
  third-party plugin editors follow the plugin's own capabilities.
- **L10N-02 — Selection.** Settings → Appearance offers Language, with System
  default first and language names in their own language, without flag icons.
  Resolve explicit saved choice, then the ordered system language preferences,
  then English. Match exact supported tags before supported parents and use
  explicit script-aware fallback rules; never arbitrarily choose a regional
  or script variant. Unsupported/corrupt preferences recover to the default.
  Persist per-user alongside existing presentation preferences; denied storage
  leaves the in-memory choice usable. Language is not part of an audio session.
- **L10N-03 — Safe switching.** Switch without restarting audio, reconnecting,
  saving a graph, losing a draft, resetting selection or moving canvas nodes.
  Refresh open UI, search, visible notices and the native tray. Hold message
  keys/arguments plus explicit severity/progress state rather than translated
  strings. Update document lang/dir and formatter caches. Initial resources
  must be ready before the first usable frame, avoiding an English flash.
- **L10N-04 — Reliable fallback.** Missing translations fall back to English;
  normal UI never displays raw keys or empty captions. English must cover every
  referenced key. Release language files must be complete and validated; a
  missing-key runtime fallback remains defensive protection. Keep unreviewed
  work-in-progress translations outside the shipped locale directory.
- **L10N-05 — Presentation only.** Never translate IDs, API method/parameter
  names, enum wire values, file paths, endpoint identities, plugin/vendor names,
  or user-entered session/node/preset names. Do not rename saved entities on
  language changes. Localize factory-template explanations and display labels
  without mutating their persisted identity. Keep API/log diagnostics stable.
  Format displayed numbers/dates/durations with the chosen locale and Intl;
  retain existing canonical serialization and precise numeric input semantics.
  Chart ticks and search collation follow the selected locale. Protect paths,
  IDs and numeric units from bidirectional text reordering.
- **L10N-06 — Errors and safety.** Render known backend error/status codes and
  structured arguments through a UI mapping. Never match English error prose
  to decide severity, visibility, permissions or recovery actions. Unknown or
  vendor errors show a translated safe summary with original technical detail
  available for support. Do not silently hide diagnostic detail. Preserve
  privacy, permission and destructive-action meaning in every translation.
- **L10N-07 — Layout and accessibility.** Allow longer labels, wrapping and
  glyph coverage at 200% zoom in dark/light/high contrast. Reserve live-state
  slots for the longest relevant translated content (UI-17). A deliberate
  language change may reflow text; telemetry must not. Support logical CSS
  directions for text/panels; keep audio graph source-to-destination geometry,
  waveforms and technical values intentional rather than blindly mirrored.
  RTL is not declared supported until tested with a real reviewed RTL catalog.
- **L10N-08 — Adding a language.** Copy en.json to a new supported-tag filename,
  change metadata, translate messages/placeholders and supply its required
  plural categories. Run one documented catalog-check command and the focused
  UI smoke suite. Discovery includes it automatically on the next build.
  A fluent reviewer approves terminology and safety text before it ships.

## Findings and prerequisites

The UI is React/TypeScript/Vite with no existing i18n dependency in
ui/package.json. ui/src/actionMessage.ts classifies severity and expiry using
English regular expressions; replace that dependency before translating its
callers. ui/src/UpdateNotice.tsx creates default-locale Intl formatters at module
scope; formatters must track the app locale. Several views render caught
error.message directly. Native surfaces live in src-tauri/src, including tray
playback, startup and recovery, and need the same central resources.

Before execution, finish the surface inventory, read the linked M05
specifications for each changed boundary, inspect the working tree again, and
check dependency versions/licenses against the pinned Node/TypeScript tools.
Planning observed unrelated edits to current.md, delegation briefs, preview
directories and tsbuildinfo; preserve them. No driver, external credentials,
cloud service or microphone access is needed to develop the resource system.
Packaged Windows, live audio continuity and Narrator need separate evidence.

## Ordered execution

1. **Inventory and contract.** Account for UI/native/backend-owned displayed
   strings and all English-text-dependent behavior/tests. Record migrated
   surfaces in a coverage checklist. Add approved L10N requirements to the
   interface spec and delivery map. Select the first real additional language
   with the owner; no language is selected by this plan.
2. **Foundation.** Pin dependencies; add English catalog, discovery, typed
   keys and placeholder types generated from JSON, validators and local-only
   language resolution. Include duplicate JSON key detection before parsing,
   valid filenames/metadata, string leaf types, unknown/missing keys, required
   plural forms, placeholder equality and restricted rich-text markup. Provide
   a development-only expanded/accented pseudolocale generated from English.
   Typecheck immediately after each implementation slice.
3. **Remove prose dependencies.** Replace severity/progress regex decisions
   with structured UI messages and regression tests for expiry/recovery.
   Inventory backend error codes; use existing structured data where available.
   If a missing structured field requires a public contract change, update the
   backend, contracts, REST/default MCP reachability, dispatcher tests and
   generated API/site catalogs together under AUTO-18. UI language preference
   alone does not require a public backend API.
4. **Migrate in bounded slices.** Start with Settings and common actions, then
   header/workbench/canvas, Properties and every tool, sessions/recording,
   onboarding/search, API/MCP guidance and diagnostics. Keep English behavior
   usable between slices. Translate on render, including module-level tool
   catalogs and memoized search data; language changes must invalidate these.
   Add a maintained lint/check guard against new unlocalized visible strings,
   with narrow documented exceptions for technical literals.
5. **Native integration.** Generate a small native lookup resource from the
   same catalogs. Resolve saved/system locale before startup/recovery dialogs
   and refresh tray captions after selection, without holding audio locks.
   Use a presentation-only shell bridge with bounded locale validation; never
   accept paths or arbitrary translated markup from the renderer.
6. **First translation and contributor workflow.** Translate one complete
   additional language, obtain fluent review, and document the copy/translate/
   validate/build workflow. Pseudolocalization catches layout problems but is
   not translation-quality evidence. Include a diagnostic coverage report so
   later English additions identify every language needing updates.
7. **Qualification and documentation.** Run the checks below, inspect the
   full diff, retain evidence, update screenshots/help as needed, and record
   limitations. Changed tool controls require the repository's portrait real
   Properties guide workflow, coverage manifest and Firefox playback checks;
   catalog extraction alone should preserve the existing English appearance.

## Verification and acceptance evidence

- Catalog tests: English key/type coverage; valid tag discovery; duplicate,
  malformed and incomplete file rejection; plural categories for representative
  languages; placeholders/markup parity; generated native-resource parity.
- Runtime tests: exact/parent/script fallback, unsupported system language,
  missing key, denied storage, reload persistence, live switching of open
  notifications and search; no graph dispatch or draft loss on switching.
- Formatting/safety tests: locale-specific number/date display, canonical
  request values, unknown errors, immutable user/device names, explicit severity
  and unchanged error lifetime. Every repaired defect gets a failing-before
  regression through its real entry point.
- Browser workflows: first-run, select/persist/switch language, search/add a
  tool, edit Properties, cancel destructive action, trigger a known refusal,
  inspect privacy/recording states. Cover English, the reviewed second language
  and expanded pseudolocale; inspect all themes, narrow layouts and 200% zoom.
  Sample bounds during live status changes to verify UI-17.
- Local checks: UI typecheck:all, format, lint, unit suite and production build;
  Rust formatting/tests/strict Clippy for changed native code; contract drift,
  dispatcher/default MCP checks if contracts change; docs acceptance. Rebuild
  e2e_backend before real-backend browser checks.
- Windows evidence: freshly built isolated-profile desktop app, selector and
  tray, restart/startup/recovery text, no stale embedded resources; attended
  Narrator pronunciation/focus and Windows 150%/200% scaling. An explicitly
  authorized controlled live route must keep playing while switching, with
  no unintended start/stop/mute/rebind and no backend errors. Do not alter the
  owner's saved route for this test.

Record actual commands, environment, result and evidence paths here during
execution. None of these implementation checks has run for this planning task.
Do not declare a language supported until its catalog, review and applicable
UI/native acceptance evidence exist.

## Risks, rollback and next action

Main risks are missed native/backend text, translated prose changing behavior,
stale cached labels, long text resizing live slots, incomplete safety wording
and numeric display affecting editable values. The ordered slices and tests
above address these directly. Large one-file catalogs are a deliberate tradeoff
for the requested simple translator workflow; nested sections keep them usable.

Rollback: revert the localization change and generated resources. Keep the
locale preference optional and safely ignored by older versions. No audio
session/database schema migration is planned; language switching never writes
graph names or device selections.

Next action: choose the first additional language and approve the remaining
bounded migration slices in the execution record. No commit, push, PR or
publication was performed by the implementation session.

## Execution record (2026-11-19)

Implemented the foundation, a broad set of everyday UI surfaces, and the
native shell text path:

- Pinned `i18next` 27.2.2 and `react-i18next` 18.0.3 in `ui/package.json`.
  No backend, contract, driver or API change was needed; the language choice
  is a per-PC presentation preference (`audiorouter.ui.language`).
- `ui/src/i18n/`: `locales/en.json` (302 messages with `meta`), build-time
  discovery through `import.meta.glob`, synchronous initialization (no
  English flash), English fallback, a missing-key handler that never renders
  a raw key, exact/parent/script tag resolution, document `lang`/`dir`
  updates, locale-aware `Intl` formatters, a typed key union generated from
  English, and a development-only `en-XA` pseudolocale.
- Catalog tooling: duplicate JSON keys with line numbers, invalid
  filenames/metadata, non-string leaves, missing/unknown keys, placeholder
  differences, plural categories, supported markup, generated key-union
  freshness, and generated native-resource freshness (`i18n:check`).
- Settings → Appearance offers Language (System default first, each language
  in its own name), saved immediately and applied live. Localized surfaces now
  include the top bar (skip links, title, Reconnect, Undo/Redo, Save, Play/Stop,
  notifications), workbench tabs and page headings, Settings navigation and
  search, Appearance, startup/autoplay/API auto-start, live autosave,
  connection animation, device-access dialog and setting, troubleshoot/recover,
  canvas keyboard panel, first-run and goal guides, device-binding help, quit
  and session-switch dialogs, Settings page headings, the This PC device list,
  app status chips, and New versions. New-version text now formats times and
  dates through the locale-aware formatter instead of module-level `Intl`
  instances, fixing the formatter finding in this plan.
- Native shell: `ui/scripts/generate-native-i18n.mjs` emits
  `src-tauri/src/ui_language.rs` from the same catalogs. The shell stores the
  window's choice in `shell-settings.json`, resolves it before the tray is
  built (saved choice, then the Windows locale, then English), and the
  presentation-only `set_ui_language` command refreshes static tray labels,
  status/recording lines, playback labels and tray failure text. The UI invokes
  it through a bounded optional bridge; it never sends paths or markup.
  `GetUserDefaultLocaleName` supplies the system language.

Checks run on Windows 11, 2026-11-19:

- `npm --prefix ui run i18n:generate` and `i18n:check`: 1 catalog, 302
  messages; generated TypeScript and Rust resources are current.
- `npm --prefix ui run typecheck:all`, `lint` (0 warnings),
  `format:check`: pass.
- `npm --prefix ui test`: 131 files, 1,143 tests pass.
- `npm --prefix ui run build`: pass; retained the existing main-chunk size
  advisory (about 720 kB).
- `cargo test --manifest-path src-tauri/Cargo.toml`: 129 passed, 4 ignored.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
  --all-features -- -D warnings` and both `cargo fmt --check` runs: pass.
- New tests: language-tag resolution, catalog discovery/fallback/document
  attributes, pseudolocale, language preference round-trip and denied
  storage, Appearance Language System-default handling, native tag
  resolution/formatting and shell language persistence.
- Documentation acceptance passes (281 files, 1,576 links). Unrelated
  delegation briefs, preview directories and `ui/tsconfig.tsbuildinfo` were
  left untouched.

Still open (next bounded tasks):

1. Migrate the remaining UI text: App action messages, canvas menus and
   toolbar, Properties and every tool editor, sessions/recording, search
   results, API/MCP guidance and diagnostics. Replace the `actionMessage`
   English-regex classifier with explicit severity/expiry at those call sites
   and a generated backend error-code mapping (L10N-06).
2. Localize backend-seeded tool descriptions and schema text after an
   inventory. The native startup/recovery dialogs remain English; only the
   tray path is generated today.
3. Choose and translate one real additional language with a fluent reviewer,
   then add the contributor workflow and coverage report (L10N-08). The
   generated native lookup already accepts any added catalog file.
4. Browser/Windows evidence: all-theme and 200% zoom layout in English and
   en-XA, live switching during audio, packaged shell/tray, Narrator.
5. Add the guarded check against new unlocalized visible strings.

## French and Simplified Chinese test build

The owner selected French and Simplified Chinese for the next desktop test
build. The two catalogs cover all 302 English reference messages and generate
their tray text from the same files. Catalog validation uses each language's
plural categories and checks placeholders against the matching English form
or its general plural form. The window and native tray resolve Windows
zh-CN and zh-SG to zh-Hans when an exact catalog is unavailable.

Windows checks passed for catalog generation and validation, typecheck,
lint, formatting, all 1,145 UI unit tests and the Edge Settings workflow
that switches English → French → Simplified Chinese and checks the saved
language after reload. Docs acceptance passed. Vite reports a main chunk
size advisory (778 kB before compression).

The normal output shell was locked by the running app. The release build
completed in `target/ui-language-build` with `custom-protocol`, and the shell,
CLI and four plug-in workers were staged in `target/ui-language-test`.
The desktop WebView2 smoke used an isolated external backend and verified
both Settings language changes and their native settings writes.
Evidence is in `target/ui-language-test-smoke.json`. The Windows x64 ZIP is
`target/AudioRouter-0.0.30-language-test-x64.zip` (SHA-256
`A4D3CE2EC13197B706DC65DA69BF3D7267187328911BD8FF043390999481CAFF`).
The broader string migration, fluent review, native tray visual review,
attended Narrator and audio continuity qualification are still open.

Settings search missed `lang` because Appearance's keywords did not include
the language control. Index the English alias and the active language's
caption in Settings and Ctrl+K. Check that Enter opens Appearance and
exposes the Language selector in all three themes. Revert the search keyword
change to roll it back. No driver or audio hardware is needed for this check.
Windows Edge passed the three theme cases and localized search in French
and Simplified Chinese. UI typecheck, lint, formatting and the release build
passed. The isolated WebView2 smoke in `target/ui-language-test-smoke.json`
confirmed the `lang` search opens Appearance in the rebuilt shell, staged at
`target/ui-language-search-test`. Next, continue the remaining string migration.

### Tools and shared Properties coverage

Moved the route help, tool search, shelf headings, all built-in shelf labels
and descriptions, guide links, and shared Properties help and actions into
the three catalogs. The catalog test covers every built-in shelf entry.
Localized search uses labels and descriptions alongside the existing English
search terms. Saved section keys, tool IDs, graph operations, and existing
node names retain their existing values. Rollback is a revert of this UI
slice and regeneration of the catalog outputs.

Windows verification: all 1,146 unit tests passed. Edge covered both shipped
translations through tool search, selection, description, and the shared
Enabled control. Still open under L10N-01/06: individual parameter captions
and tool editors, canvas and session actions, plug-in and setup errors, and
remaining recording, diagnostics and native text.

The Windows release shell and companions are staged in
`target/ui-localized-tools-test`. The isolated WebView2 smoke verified
language switching, Settings search, and the Simplified Chinese tool shelf
and tool description. Results: `target/ui-language-test-smoke.json`.

### Canvas copy and source inventory

Moved the canvas heading, layout toolbar, selection count, connection form
labels and legend into the catalogs (UI-02/03/11, UI-37, L10N-01/07).
The audit command `npm --prefix ui run i18n:audit` lists direct JSX text
and display attributes. Add `-- --all` to inspect literals in TypeScript
helpers as well. The output is a review queue because it includes technical
values and preview fixtures. Revert the canvas text changes and regenerate
the catalog outputs to roll back.

Next, work through the inventory starting with device setup, plug-ins and
individual tool settings. Confirm backend states and action messages are
localized at the point of display.

The shared built-in parameter editor now gets its known captions, help
text, and format and switch choices from the three catalogs (UI-05/11/37).
The editor and catalog still use stable parameter and option IDs. The next
pass covers dedicated tool controls and status messages that bypass the
shared editor, including dynamics, EQ, recording, and device and app
properties. Rollback: revert the parameter text changes and regenerate the
catalog outputs.

The dedicated Gain and Volume editor also uses the catalogs for its title,
instructions, chart text and preset group names. Windows verification passed
catalog validation (656 keys), lint, formatting, the 1,146-test suite for the
shared change, 11 focused tests after the dedicated editor change, and five
Edge scenarios including the translated Gain editor in both languages. The
release shell built and passed the isolated WebView2 smoke for the language
selector, canvas and shelf. Test build: `target/ui-localized-parameters-test`.
The existing Vite chunk-size advisory and browser runner notices remain.

The French Properties screenshot exposed the shared flag state column's
English-sized `3ch` width. Size the column from both translated states and
let long text wrap. The regression checks horizontal overflow, accessible
state descriptions and switch positions in all three themes (UI-11/17/37).
Rollback is limited to the flag layout and markup. No live audio is needed.
All six Edge cases passed, including French at 960 px. Screenshots in all
three themes show the complete state label. The release build and isolated
WebView2 smoke passed. Test folder: `target/ui-localized-layout-test`.

### Session, Recording and Properties workflow text

Migrated canvas undo/redo feedback, input channel modes and spatial help,
live-reading states, session management and history, session file summaries,
recording folder and manual controls, library rows, file actions and
confirmation messages (UI-05/08/09/11/37). User names, paths and backend state
values keep their existing representation. Roll back the presentation edits
and regenerate the catalogs to revert. Checks use a disposable route and
require no audio hardware or driver access. Backend-supplied error detail,
unmigrated telemetry values and other tool-specific panels remain separate
localization work.

Validation: the three catalogs pass with 908 keys. All seven Edge language
scenarios passed, including Session and Recording in both languages. The
full UI suite passed 1,146 tests before the final Recorder controls pass;
109 focused tests, lint, format, typecheck, and docs passed after it.
The rebuilt Windows shell opened both translated workflows during isolated
WebView2 smoke. Test folder: `target/ui-localized-workflows-test`.

PR #166's previous four Edge shards failed during Playwright startup.
Playwright imports shared UI modules in Node, where Vite's
`import.meta.glob` is unavailable. The Node loader now uses the generated
English catalog, while Vite still discovers the bundled language files.
Catalog validation checks that both generated outputs are current.
Local Windows validation includes the CI browser fixture build and discovery
of all 793 browser tests, all 1,146 UI unit tests, Rust format and shell
Clippy, UI lint and format, website checks, and documentation validation.

### Settings copy and per-file JSX review

Moved the reported text from Keyboard shortcuts, Virtual Cables, Plug-ins,
Startup, Help, AI assistants and API into the three catalogs (UI-11/29/37).
The API pass also covers listener and network warnings. Revert the copy
bindings and regenerate the catalogs to roll back. No live audio, driver
installation, or API listener is needed to verify these labels.

`npm --prefix ui run i18n:audit -- --tsx --by-file` gives each production
TSX file its own section, including files with no candidates. It checks
conditional JSX text and display attributes as well as literal children.
Use `--all` with those flags to inspect data and action literals too. The
output is an inventory, with technical values and preview text mixed in.

Windows evidence: all 103 production TSX files were scanned. The refreshed
display-focused report has 1,677 candidates in
`target/ui-tsx-localization-review.txt`. Catalog validation passed at 952
keys. Nine Edge scenarios passed, including the French and Simplified
Chinese Settings checks. The staged Windows build in
`target/ui-localized-settings-test` passed the isolated WebView2 smoke.
Next, triage the remaining per-file candidates and migrate the app-owned
copy, starting with the rest of the shortcut editor, plug-in dialogs, and
MCP connection steps.

### Simplified Chinese rendered audit

Use the opt-in Edge fixture to visit every node kind, the daily workbench
tabs, the two additional Status views, and all Settings pages. It selects
Simplified Chinese through the app and records panel screenshots plus
Latin-script text and accessible attributes. The fixture covers the plug-in
and instrument inspectors with placeholders, without opening vendor code.
It doesn't qualify live devices, third-party windows or hardware.

The workspace screenshots identified English runtime status, route actions
and warnings. The tool screenshots identified English tool status pills and
device binding copy. Move those strings to the catalogs while preserving
the stable route and device values (UI-04/05/08/11/37). Rollback is a revert
of the text and audit changes. Specialized editors and Settings branches
still need translation where their captured text shows English.

Windows Edge captured 104 panels and scroll positions across 38 node kinds
and 21 workbench and Settings views, plus the workspace screenshot. The
evidence folder is `target/chinese-ui-audit`; `inventory.json` lists the
files and detected display text by view. The capture and nine language
scenarios passed, as did 104 focused unit tests, catalog validation,
typecheck, lint, format and docs. This is capture coverage, not a clean
translation verdict. Compressor and other specialized editors, Status,
and MCP still show English. Migrated since then (2026-11-19): the Mic test
card (`micTest.*`) and the Compressor, Gate and Limiter editor
(`dynamicsUi.*`), with typecheck, catalog check, lint and the dynamics unit
tests passing. A parallel pass then migrated the rest (namespaces `g1`-`g6`,
`h1`-`h3`; the catalogs now hold about 3,700 messages). The rerun Chinese
capture (104 panels) leaves only fixture data, units, product names, the
English/Francais endonyms, API-page literals and diagnostics log lines in
English. Backend-originated text stays English where the backend sends it
(Fun Voice Changer presets, some node-issue messages). Known gap:
`plainWords.test.ts` scans source literals, so the glossary is not yet
enforced on the English catalog (about 30 existing messages still say "node").
Previously: use the inventory to guide the next migration,
then rerun the same opt-in capture.

Rollback: remove the two locale files and regenerate native resources. Older
builds resolve a saved unsupported locale through the usual fallback.

### 2026-11-24 — Status messages carry their own severity (L10N-06)

Requirements: L10N-06 (never match English prose to decide severity),
L10N-03 (hold keys plus explicit severity), UI-04/05/17. Branch
`feat/l10n-structured-action-messages`, local commits only, not pushed.

What changed:

- `ui/src/actionMessage.ts` now defines `UiMessage` (`text`, `tone`,
  `progress`, optional `actions`, `key`). `say(key, args)` builds one from a
  catalog key; `errorMessage(text)` and `infoMessage(text)` cover text that is
  not a catalog message (a caught backend error, a backend `reason`, the four
  literal recovery notices in `useWorkspaceEvents.ts`). `BLANK_MESSAGE` keeps
  the Virtual Cable status line's reserved space (UI-17).
- `ui/src/actionMessageTones.ts` is the typed table (`MESSAGE_TONES`, 260
  keys, `satisfies Partial<Record<MessageKey, …>>`). `say` accepts only keys in
  it, so a new status message without a declared severity does not compile.
  The table was derived by running the old English rules over each English
  message once, so English severity, expiry and buttons are unchanged:
  `progress` is the old trailing-ellipsis rule, and `actions` replaces the
  `englishOf(...).includes("Troubleshoot")` checks that chose which buttons
  App shows (a message can offer more than one: two keys offer both "go to the
  tool" and "open Status").
- The English regexes, `markErrorMessage` registry, and `englishOf` memory in
  `actionText.ts` are deleted. `formatUiError` stays a plain string function;
  `uiError(error, fallback)` is the status-message form. `actionMessage` state
  in `App.tsx`, `PanelMessage`, the panel `message` states, `onActionMessage`
  (cable panel), `announce`/`onMessage` (Soundboard), the canvas layout and
  connection callbacks, `QuitButton` and `audioUploadProblem` all take
  `UiMessage`; TypeScript listed every site. The lifetime effect is keyed on the
  message text, so showing the same message again does not restart its timer,
  as before.
- Backend codes: `AudioRouterRpcError.data.code` exists, but every caught
  error already shows as an error whatever its code, and no code changes
  severity, so no code→tone mapping was added (it would be dead code). Backend
  `reason` strings are classified by the structured field next to them where
  one exists (`availability.status`), otherwise as information.
- `plainWords.test.ts` also scans `en.json` now (placeholders such as
  `{{node}}` ignored; keys used by the developer panels, and the API's "node
  ID" terms, are listed exceptions). It found 29 English messages with
  "node", "draft" or "physical output"; their English wording was changed to
  tool, unsaved edits and output device. `fr.json` and `zh-Hans.json` were not
  touched, so their wording for these keys is the old meaning and needs
  reviewer follow-up: `workspaceAudit.notChosen`, `recordingUi.attached`,
  `workflowUi.{startNode,working,readyHelp,fileMissing_one,fileMissing_other,
  fileCheck}`, `parameterUi.help.mute.muted`, `canvasUi.disabled`,
  `toolCatalog.audio-file.help`, `settings.liveAutosave.offHelp`,
  `g2.{nwFooterSend,nwFooterRecv}`, `g1.toneNotRouted`, `g5.{meterPressPlay,
  appChangeHelp}`, `g4Canvas.recNone`, `h1.{detailHears,detailPlaysInto,
  spkYourNode}`.

Deliberate differences from the old English behavior (all in the direction of
the structured fact; none hides a message):

- A caught error whose own text matched no pattern ("Invalid JSON", the
  plugin and application-capture failures in `App.tsx`, the cable and Native
  Binding panels) is now an error: red, announced assertively, kept until
  dismissed. Before it depended on its wording. The virtual-device plan reason
  is an error because the contract only ever returns `unavailable`.
- A name or reason placed inside a message can no longer change its severity
  (a node called "failed" used to turn a success red). The old rules judged the
  rendered sentence, so a failure wrapped in a message was often red. The table
  was built from templates, so I checked every `say` call that interpolates an
  error, reason, message, details or state: `g3Adv.signInFailed` and
  `quit.refusedMessage` (both in a `catch`) are now errors, a recorder that
  reports `failed` shows as an error (`withTone`), and an audio file that
  reports `stopped` keeps its green check. The vd* reasons ("requires M03 managed
  virtual driver") and `g1.appSavedNotApplied` stay information and success, as
  their real reasons never matched an old pattern.
- `g1.appGoalMade` keeps its key's severity when the list of tools still to
  choose is appended.

Messages whose English severity looks wrong but was kept identical, for the
owner to decide (change the entry in `MESSAGE_TONES`):
`h1.importUnavailable`, `g2.neNoVbCapture`, `g2.neNoVbPair`,
`g1.appFindingSessionGone`, `g1.appFindingToolGone`, and
`g1.appOlderEdits` (success tone, green check, because the old rule matched
"saved").

Still English-prose based (not severity; recovery decisions on English
backend text, so they only work while the backend speaks English):
`backend.ts` error friendliness patterns, `DeviceAccess.tsx`,
`App.tsx` "session to be stopped", `PluginPanels.tsx` scan-error filters, and
`backendText`'s fixed-sentence map. A structured error code from the backend
(AUTO-18 change) is the real fix. The four recovery notices in
`useWorkspaceEvents.ts` and the plugin-scan/plug-in recovery notices are
still literal English, not catalog messages.

Checks (Windows 11, from `ui/`, 2026-11-24):

- `npm run i18n:generate`, `npm run i18n:check`: 3 language files, 3,666
  messages, generated TypeScript and Rust resources current.
- `npm run typecheck:all`: pass. `npm run lint`: pass, 0 warnings.
  `npm run format:check`: pass.
- `npm test`: 133 files, 1,175 tests pass.
  New tests in `actionMessage.test.ts`: severity per key, buttons, lifetimes,
  a warning still in progress, the blank message, French and Simplified
  Chinese messages keeping severity/expiry/buttons, names inside text not
  changing severity, and table consistency with the English catalog. Changed
  tests that asserted bare strings now assert `UiMessage` objects.
- Playwright (Edge, `--workers=1`, own `--output`): `copy-error`,
  `banner-layout`, `device-access-consent`, `application-sources`,
  `audio-tools`, `canvas-layout`, `compact-workspace`,
  `header-canvas-usability`, `live-controls`, `live-bypass-draft` and
  `device-used-by` pass (three themes where the specs cover them). Two
  runs with two workers or a shared output folder failed with trace-file
  ENOENT and stale-generated-catalog errors, and passed when rerun alone.
- `node tools/docs/validate.mjs`: 295 Markdown files, 1,641 links, pass.

Not run: specs that import `real-backend` (need a rebuilt `e2e_backend`; no
Rust build was done), including `goals-and-tour`, `canvas-copy-paste`,
`media-and-routing`, `canvas-layout-review`, `inspector-stability`; the
packaged shell or tray; Narrator; a fluent-reviewer pass of French and
Chinese; the website guide videos (no tool or control changed). The English
wording edits above change only catalog text, so no website guide copy was
regenerated: the site guides are written from the Rust parameter catalog and
hand-written content (`tools/site/export-tool-guide-catalog.mjs` reads Rust
sources), none of the reworded strings appears under `site/`, and
`node tools/site/check-tool-guide-catalog.mjs` passes. Rollback: revert the
commits on the branch.

Deviation from "never a patch script": about 250 call sites were renamed with perl
(`setActionMessage(tm(…))`, `setMessage(tr(…))` → `say(…)`, `formatUiError` →
`uiError`), state-type and import lines were changed with perl or sed, three
files were written with heredocs, and `actionMessageTones.ts` was generated
once from the old rules by a script. Everything was verified by TypeScript, a
read of the diff and the full unit run; the table is reviewed by test. The
remaining changes used the Edit and Write tools.

## References

- [M05 visual editor](../../milestones/M05-visual-editor.md),
  [interface specification](../../spec/09-interface.md),
  [delivery map](../../spec/15-delivery.md).
- Official [i18next fallback](https://www.i18next.com/principles/fallback),
  [plurals](https://www.i18next.com/translation-function/plurals) and
  [TypeScript](https://www.i18next.com/overview/typescript) documentation,
  consulted 2026-11-19. JSON interpolation types require generation rather
  than assuming JSON imports provide literal-value inference.
- Official [react-i18next resource organization](https://react.i18next.com/guides/multiple-translation-files).
  Namespaces are available if needed later; this plan deliberately keeps one
  author-edited file per language.
