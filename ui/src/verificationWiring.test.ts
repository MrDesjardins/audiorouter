import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { expect, it } from "vitest";
import production from "../playwright.production.config";
import acceptanceBuild from "../vite.canvas-production.config";

it("ordinary browser verification preserves development and optimized suites", () => {
  const { scripts } = JSON.parse(readFileSync(path.resolve("package.json"), "utf8"));
  expect(scripts.e2e).toBe("npm run e2e:guard && npm run e2e:development && npm run e2e:production-canvas");
  expect(scripts["e2e:guard"]).toBe("node scripts/qualify-browser-guard.mjs");
  expect(scripts["e2e:development"]).toContain("playwright test --config=playwright.config.ts");
  expect(scripts["e2e:production-canvas"]).toContain("--example e2e_backend --locked");
  expect(scripts["e2e:production-canvas"]).toContain("playwright test --config=playwright.production.config.ts");
});
it("every active scenario uses an uncaught-error-guarded fixture", () => {
  for (const file of readdirSync("e2e").filter((name) => name.endsWith(".pw.ts"))) {
    const source = readFileSync(path.resolve("e2e", file), "utf8");
    expect(source, file).toMatch(/import\s*\{[^}]*\btest\b[^}]*\}\s*from\s*"\.\/(browser-test|real-backend)"/);
  }
});
it("optimized acceptance builds both adapters in a clean dedicated output", () => {
  expect(acceptanceBuild.build?.outDir).toBe("../target/canvas-production-ui");
  expect(acceptanceBuild.build?.emptyOutDir).toBe(true);
  expect(acceptanceBuild.build?.rollupOptions?.input).toEqual(["route-harness.html", "backend-harness.html"]);
});
it("production patterns refer to real scenarios and retain critical workflows", () => {
  const patterns = production.testMatch as string[];
  expect(Array.isArray(patterns)).toBe(true);
  for (const pattern of patterns) expect(existsSync(path.resolve("e2e", path.basename(pattern))), pattern).toBe(true);
  for (const file of [
    "canvas-layout.pw.ts",
    "canvas-layout-review.pw.ts",
    "canvas-copy-paste.pw.ts",
    "session-workflows.pw.ts",
    "recording-workflows.pw.ts",
    "canvas-readiness.pw.ts",
  ])
    expect(patterns).toContain(`**/${file}`);
  expect(production.outputDir).toContain("audiorouter-production-playwright-results");
});
