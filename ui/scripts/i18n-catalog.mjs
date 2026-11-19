/**
 * Shared catalog tooling for `npm run i18n:check` and `i18n:generate`.
 * Plain Node ESM, no application imports, so it runs before TypeScript.
 */
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const LOCALE_DIR = fileURLToPath(new URL("../src/i18n/locales", import.meta.url));
export const GENERATED_FILE = fileURLToPath(new URL("../src/i18n/generated/messageKeys.ts", import.meta.url));
export const PLURAL_SUFFIXES = ["zero", "one", "two", "few", "many", "other"];
const ALLOWED_TAGS = new Set(["br", "strong", "em", "b", "i"]);

export function readCatalog(file) {
  return JSON.parse(readFileSync(path.join(LOCALE_DIR, file), "utf8"));
}

export function localeFiles() {
  return readdirSync(LOCALE_DIR)
    .filter((file) => file.endsWith(".json"))
    .sort();
}

export function normalizeLanguageTag(tag) {
  const parts = String(tag).trim().replace(/_/g, "-").split("-").filter(Boolean);
  if (parts.length === 0) return "";
  const [language, ...rest] = parts;
  if (!/^[a-zA-Z]{2,3}$/.test(language)) return "";
  const normalized = [language.toLowerCase()];
  for (const part of rest) {
    if (/^[a-zA-Z]{4}$/.test(part)) normalized.push(part[0].toUpperCase() + part.slice(1).toLowerCase());
    else if (/^([a-zA-Z]{2}|\d{3})$/.test(part)) normalized.push(part.toUpperCase());
    else if (/^[a-zA-Z0-9]{1,8}$/.test(part)) normalized.push(part.toLowerCase());
    else return "";
  }
  return normalized.join("-");
}

/** Tokens with offsets, enough for duplicate-key detection and line numbers. */
export function tokenizeJson(text) {
  const tokens = [];
  let index = 0;
  while (index < text.length) {
    const char = text[index];
    if (char === '"') {
      const start = index;
      index += 1;
      while (index < text.length) {
        const current = text[index];
        if (current === "\\") {
          index += 2;
          continue;
        }
        if (current === '"') {
          index += 1;
          break;
        }
        index += 1;
      }
      const raw = text.slice(start, index);
      let value;
      try {
        value = JSON.parse(raw);
      } catch {
        value = raw.slice(1, -1);
      }
      tokens.push({ type: "string", value, index: start });
      continue;
    }
    if ("{}[]:,".includes(char)) {
      tokens.push({ type: "punct", value: char, index });
      index += 1;
      continue;
    }
    if (/\s/.test(char)) {
      index += 1;
      continue;
    }
    const match = /^(-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?|true|false|null)/.exec(text.slice(index));
    if (match) {
      tokens.push({ type: "scalar", value: match[0], index });
      index += match[0].length;
      continue;
    }
    tokens.push({ type: "invalid", value: char, index });
    index += 1;
  }
  return tokens;
}

export function lineOf(text, index) {
  let line = 1;
  for (let cursor = 0; cursor < index && cursor < text.length; cursor += 1) {
    if (text[cursor] === "\n") line += 1;
  }
  return line;
}

/** Duplicate keys silently lose data in JSON.parse; find them before parsing. */
export function findDuplicateKeys(text) {
  const duplicates = [];
  const stack = [{ kind: "root", expect: "value", path: [] }];
  for (const token of tokenizeJson(text)) {
    const top = stack[stack.length - 1];
    if (token.type === "string") {
      if (top.kind === "object" && (top.expect === "key" || top.expect === "keyOrEnd")) {
        if (top.seen.has(token.value)) {
          duplicates.push({ key: token.value, path: [...top.path, token.value].join("."), line: lineOf(text, token.index) });
        }
        top.seen.add(token.value);
        top.pendingKey = token.value;
        top.expect = "colon";
      } else {
        completeValue(stack);
      }
      continue;
    }
    if (token.type === "punct") {
      switch (token.value) {
        case "{": {
          const parent = stack[stack.length - 1];
          stack.push({
            kind: "object",
            expect: "keyOrEnd",
            seen: new Set(),
            path: parent.pendingKey ? [...parent.path, parent.pendingKey] : [...parent.path],
          });
          break;
        }
        case "}":
          stack.pop();
          completeValue(stack);
          break;
        case "[":
          stack.push({ kind: "array", expect: "valueOrEnd", path: [...top.path] });
          break;
        case "]":
          stack.pop();
          completeValue(stack);
          break;
        case ",":
          if (top.kind === "object") top.expect = "key";
          else top.expect = "value";
          break;
        case ":":
          top.expect = "value";
          break;
        default:
          break;
      }
      continue;
    }
    completeValue(stack);
  }
  return duplicates;
}

function completeValue(stack) {
  const parent = stack[stack.length - 1];
  if (!parent || parent.kind === "root") return;
  parent.expect = "commaOrEnd";
  delete parent.pendingKey;
}

export function flattenMessages(value, prefix = [], problems = []) {
  const flat = {};
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    problems.push(`${prefix.join(".") || "messages"} must be a JSON object`);
    return flat;
  }
  for (const [key, child] of Object.entries(value)) {
    const next = [...prefix, key];
    if (!/^[A-Za-z][A-Za-z0-9]*(_(zero|one|two|few|many|other))?$/.test(key))
      problems.push(`Unsupported key "${next.join(".")}"`);
    if (typeof child === "string") {
      flat[next.join(".")] = child;
    } else if (typeof child === "object" && child !== null && !Array.isArray(child)) {
      Object.assign(flat, flattenMessages(child, next, problems));
    } else {
      problems.push(`${next.join(".")} must be a string`);
    }
  }
  return flat;
}

export function placeholders(message) {
  return new Set([...message.matchAll(/\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g)].map((match) => match[1]));
}

export function invalidMarkup(message) {
  const tags = [...message.matchAll(/<\/?([a-zA-Z][a-zA-Z0-9]*)[^>]*>/g)].map((match) => match[1].toLowerCase());
  return tags.filter((tag) => !ALLOWED_TAGS.has(tag));
}

export function pluralCategories(tag) {
  return new Intl.PluralRules(tag).resolvedOptions().pluralCategories;
}

export function pluralBases(flat) {
  const bases = new Map();
  for (const key of Object.keys(flat)) {
    const match = /^(.*)_(zero|one|two|few|many|other)$/.exec(key);
    if (!match) continue;
    if (!bases.has(match[1])) bases.set(match[1], []);
    bases.get(match[1]).push(match[2]);
  }
  return bases;
}

export function generatedKeysSource(flat) {
  const keys = new Set(Object.keys(flat));
  // i18next is called with the base key plus `count`; the suffixed forms are
  // selected at runtime.
  for (const base of pluralBases(flat).keys()) keys.add(base);
  const sorted = [...keys].sort();
  return `// Generated by ui/scripts/generate-i18n.mjs. Do not edit by hand.\n// Run \`npm --prefix ui run i18n:generate\` after changing en.json.\nexport const MESSAGE_KEYS = [\n${sorted
    .map((key) => `  "${key}",`)
    .join("\n")}\n] as const;\n\nexport type MessageKey = (typeof MESSAGE_KEYS)[number];\n`;
}
