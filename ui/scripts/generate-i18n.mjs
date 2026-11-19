/** Regenerate the typed message-key union from en.json. */
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { GENERATED_FILE, LOCALE_DIR, flattenMessages, generatedKeysSource } from "./i18n-catalog.mjs";

const english = JSON.parse(readFileSync(path.join(LOCALE_DIR, "en.json"), "utf8"));
const problems = [];
const flat = flattenMessages(english.messages, [], problems);
if (problems.length > 0) {
  for (const problem of problems) console.error(problem);
  process.exitCode = 1;
} else {
  mkdirSync(path.dirname(GENERATED_FILE), { recursive: true });
  writeFileSync(GENERATED_FILE, generatedKeysSource(flat), "utf8");
  console.log(`Generated ${Object.keys(flat).length} message keys.`);
}
