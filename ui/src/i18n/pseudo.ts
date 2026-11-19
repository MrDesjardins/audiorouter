/**
 * Development-only pseudolocale. It accents letters and adds width so layout
 * problems from German-length or longer strings appear before a real
 * translation is written. Placeholders and markup stay intact. Pseudotext is
 * a layout probe, never translation-quality evidence.
 */

const ACCENTS: Record<string, string> = {
  a: "á",
  b: "ƀ",
  c: "ç",
  d: "ð",
  e: "é",
  f: "ƒ",
  g: "ĝ",
  h: "ĥ",
  i: "í",
  j: "ĵ",
  k: "ķ",
  l: "ļ",
  m: "ɱ",
  n: "ñ",
  o: "ó",
  p: "þ",
  q: "ǫ",
  r: "ŕ",
  s: "š",
  t: "ţ",
  u: "ú",
  v: "ṽ",
  w: "ŵ",
  x: "ẋ",
  y: "ý",
  z: "ž",
};

const PROTECTED = /(\{\{[^}]*\}\}|<[^>]+>)/g;

export function pseudolocalize(value: string): string {
  return value
    .split(PROTECTED)
    .map((part) => {
      if (part.startsWith("{{") || part.startsWith("<")) return part;
      const accented = part.replace(/[a-zA-Z]/g, (letter) => {
        const lower = ACCENTS[letter.toLowerCase()] ?? letter;
        return letter === letter.toUpperCase() ? lower.toUpperCase() : lower;
      });
      return accented + "·".repeat(Math.max(1, Math.round(part.length * 0.3)));
    })
    .join("");
}

export function pseudolocalizeMessages(value: unknown): unknown {
  if (typeof value === "string") return pseudolocalize(value);
  if (typeof value !== "object" || value === null || Array.isArray(value)) return value;
  return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, pseudolocalizeMessages(child)]));
}
