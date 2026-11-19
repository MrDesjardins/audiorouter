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

No additional language is declared supported; only English ships. Rollback:
revert the localization change and the optional preference; no audio session
or database schema changed.

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
