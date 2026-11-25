# Interface language workflow

AudioRouter ships its interface text in one JSON file per language. English is
the reference; every other file must cover the same keys. This page is the
contributor workflow for adding or changing a language (requirement L10N-08).

## Files

```text
ui/src/i18n/
  locales/en.json              English reference; edit this first
  locales/fr.json              French
  locales/zh-Hans.json         Simplified Chinese
  locales/<tag>.json           any added language (canonical BCP 47 tag)
  generated/messageKeys.ts     generated key union; never edit
  generated/englishMessages.ts generated English catalog; never edit
src-tauri/src/ui_language.rs   generated native tray/status text; never edit
```

`meta.nativeName` is the name shown in Settings → Appearance → Language.
`meta.direction` is `ltr` or `rtl`; RTL is not supported until a reviewed RTL
catalog exists. Messages are nested under `messages`; a message is always a
string (never an array or object).

## Add a language

1. Copy `ui/src/i18n/locales/en.json` to `ui/src/i18n/locales/<tag>.json`, where
   `<tag>` is a canonical BCP 47 tag such as `de`, `pt-BR` or `zh-Hant`.
2. Change `meta.nativeName` and `meta.direction`. Do not change key names.
3. Translate every value. Keep `{{name}}`, `{{count}}`, `{{device}}` and every
   other placeholder exactly as written, including the surrounding spaces and
   punctuation the sentence needs.
4. Supply the plural categories the language requires. English uses `_one`
   and `_other`; French uses `_one`, `_other` and `_many`; Chinese uses
   `_other`. Base keys stay in `en.json`.
5. Run the checks below. Language discovery is automatic: the build glob and
   the generated native lookup include the new file on the next build.

An unreviewed draft belongs outside `ui/src/i18n/locales/` until a fluent
reviewer approves it (L10N-04). Delete it or keep it in a separate folder; the
app only bundles files in `locales/`.

## Change English text

1. Edit `en.json` first. Update the same keys in every other language file.
2. If the change adds a key, translate it in every file; `i18n:check` fails
   until each language covers it.
3. Use the plain-word glossary: "route" for the canvas graph, "tool" for a card
   on it, "unsaved edits" for changes not saved, "input device"/"output device"
   for real devices. `plainWords.test.ts` enforces the English catalog and
   rejects new inline JSX prose.
4. Run the checks. Never edit `generated/` or `ui_language.rs`; they are
   written by `npm --prefix ui run i18n:generate`.

## What is not translated

IDs, API method and parameter names, enum wire values, file paths, endpoint
identities, plugin/vendor names, and user-entered session, tool or preset names
are technical values and stay as they are (L10N-05). Windows-owned pickers and
third-party plugin windows follow Windows or the plugin's own language. Windows
Sound-settings tab names used inside guidance are catalog keys because the
sentence needs them (`backendErrors.windowsPlaybackTab`/`windowsRecordingTab`).

## Commands

Run from `ui/`:

```powershell
npm run i18n:generate      # regenerate typed keys and native resources
npm run i18n:check         # validate every catalog and both generated outputs
npm run i18n:audit -- --tsx --by-file   # list unlocalized visible candidates
npm run typecheck:all
npm run lint
npm run format:check
npm test
```

The validation `i18n:check` performs is the coverage report: it rejects an
invalid filename or metadata, a duplicate JSON key (with its line number), a
non-string leaf, an unknown key, a missing key in any language, a placeholder
difference against the matching English form, a missing plural category, or
stale generated output. A new English key therefore names every language that
still needs it.

`i18n:audit` is a review queue, not a failure: it lists units, product names,
paths, preview fixtures and values the backend returns beside any real
leftover. Its output must be triaged by hand.

Browser checks (Edge, one worker, a free port):

```powershell
npx playwright test --config=playwright.config.ts --workers=1 ui-language.pw.ts ui-language-zoom.pw.ts
```

`ui-language.pw.ts` switches languages, checks Settings search, the tool shelf,
the canvas, Session/Recording and the application picker. `ui-language-zoom.pw.ts`
checks the 200% (720 × 500) layout in dark, light and high contrast, in English
and the development-only `en-XA` pseudolocale. The pseudolocale is a layout
probe, never translation evidence.

## Review

The French and Simplified Chinese text was written by the agent from existing
catalog terms. A fluent speaker must approve terminology and safety wording
before a language is called supported. The current queue, with screen context
and open wording decisions, is
[2026-11-24-fluent-review-queue.md](../plans/active/evidence/2026-11-24-fluent-review-queue.md).

## Troubleshooting

- A missing key renders an empty string in production (a development console
  warning names it) by design; it never shows the raw key. Fix the catalog.
- An English flash at startup means the catalogs were not bundled; the
  application falls back to English when initialization fails.
- The tray keeps English text until `set_ui_language` succeeds; the tray uses
  the saved choice, then the Windows locale, then English.