import { readFileSync, writeFileSync } from "node:fs";

if (process.argv.length !== 4) {
  console.error("usage: node generate-npm-sbom.mjs <package-lock.json> <output.json>");
  process.exit(2);
}

const [, , lockPath, outputPath] = process.argv;
const lock = JSON.parse(readFileSync(lockPath, "utf8"));
if (lock.lockfileVersion !== 3 || typeof lock.packages !== "object" || lock.packages === null) {
  throw new Error("unsupported npm lockfile schema");
}

const components = [];
for (const [location, entry] of Object.entries(lock.packages)) {
  if (!location || typeof entry !== "object" || entry === null || typeof entry.version !== "string") continue;
  const name = typeof entry.name === "string"
    ? entry.name
    : location.slice(location.lastIndexOf("node_modules/") + "node_modules/".length);
  if (!name || name.includes("/node_modules/")) continue;
  const component = {
    type: "library",
    "bom-ref": `npm:${name}@${entry.version}`,
    name,
    version: entry.version,
    purl: `pkg:npm/${encodeURIComponent(name)}@${entry.version}`,
    scope: entry.dev ? "optional" : "required",
  };
  if (entry.dev) component.properties = [{ name: "audiorouter:npm-development-dependency", value: "true" }];
  components.push(component);
}
components.sort((left, right) => left.name.localeCompare(right.name) || left.version.localeCompare(right.version));

const bom = {
  bomFormat: "CycloneDX",
  specVersion: "1.5",
  version: 1,
  metadata: {
    component: {
      type: "application",
      name: lock.name ?? "@audiorouter/ui",
      version: lock.version ?? "0.0.0",
    },
  },
  components,
};
writeFileSync(outputPath, `${JSON.stringify(bom, null, 2)}\n`, "utf8");
