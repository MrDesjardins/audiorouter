/** Validate the shipped language catalogs and the generated key union. */
import { existsSync, readFileSync } from "node:fs";
import {
  GENERATED_FILE,
  LOCALE_DIR,
  flattenMessages,
  findDuplicateKeys,
  generatedKeysSource,
  invalidMarkup,
  localeFiles,
  normalizeLanguageTag,
  placeholders,
  pluralBases,
  pluralCategories,
} from "./i18n-catalog.mjs";

const errors = [];
const files = localeFiles();

if (files.length === 0) errors.push(`No language files found in ${LOCALE_DIR}`);
if (!files.includes("en.json")) errors.push("en.json is required as the reference catalog");

const catalogs = new Map();
for (const file of files) {
  const raw = readFileSync(new URL(`../src/i18n/locales/${file}`, import.meta.url), "utf8");
  for (const duplicate of findDuplicateKeys(raw)) {
    errors.push(`${file}:${duplicate.line}: duplicate key "${duplicate.path}"`);
  }
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (cause) {
    errors.push(`${file}: invalid JSON (${cause instanceof Error ? cause.message : String(cause)})`);
    continue;
  }
  const tag = file.replace(/\.json$/i, "");
  if (normalizeLanguageTag(tag) !== tag) errors.push(`${file}: "${tag}" is not a canonical BCP 47 tag`);
  const meta = parsed && typeof parsed === "object" ? parsed.meta : null;
  if (!meta || typeof meta !== "object" || Array.isArray(meta)) {
    errors.push(`${file}: missing meta object`);
  } else {
    if (typeof meta.nativeName !== "string" || !meta.nativeName.trim())
      errors.push(`${file}: meta.nativeName must be a non-empty string`);
    if (meta.direction !== "ltr" && meta.direction !== "rtl")
      errors.push(`${file}: meta.direction must be "ltr" or "rtl"`);
  }
  const problems = [];
  const flat = flattenMessages(parsed?.messages, [], problems);
  for (const problem of problems) errors.push(`${file}: ${problem}`);
  catalogs.set(tag, flat);
}

const reference = catalogs.get("en");
if (reference) {
  const referenceKeys = new Set(Object.keys(reference));
  for (const [tag, flat] of catalogs) {
    const keys = new Set(Object.keys(flat));
    for (const key of referenceKeys) if (!keys.has(key)) errors.push(`${tag}: missing key "${key}"`);
    for (const key of keys) if (!referenceKeys.has(key)) errors.push(`${tag}: unknown key "${key}"`);
    const required = pluralCategories(tag);
    for (const [base, present] of pluralBases(flat)) {
      for (const category of required) {
        if (!present.includes(category)) errors.push(`${tag}: "${base}" needs plural form "_${category}"`);
      }
    }
    for (const key of keys) {
      const bad = invalidMarkup(flat[key]);
      if (bad.length > 0) errors.push(`${tag}: "${key}" uses unsupported markup <${bad.join(">, <")}>`);
    }
    if (tag === "en") continue;
    for (const key of keys) {
      if (!referenceKeys.has(key)) continue;
      const expected = [...placeholders(reference[key])].sort();
      const actual = [...placeholders(flat[key])].sort();
      if (expected.join("\u0000") !== actual.join("\u0000"))
        errors.push(`${tag}: "${key}" placeholders differ (expected ${expected.join(", ") || "none"})`);
    }
  }
}

if (existsSync(GENERATED_FILE) === false) {
  errors.push("src/i18n/generated/messageKeys.ts is missing; run i18n:generate");
} else if (reference) {
  const current = readFileSync(GENERATED_FILE, "utf8").replace(/\r\n/g, "\n");
  if (current !== generatedKeysSource(reference))
    errors.push("src/i18n/generated/messageKeys.ts is stale; run i18n:generate");
}

if (errors.length > 0) {
  for (const error of errors) console.error(error);
  process.exitCode = 1;
} else {
  console.log(`i18n catalogs valid: ${files.length} language file(s), ${Object.keys(reference ?? {}).length} messages.`);
}
