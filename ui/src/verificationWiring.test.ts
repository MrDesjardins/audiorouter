import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { expect, it } from "vitest";
import production from "../playwright.production.config";
import acceptanceBuild from "../vite.canvas-production.config";
import browserBuild from "../vite.browser-acceptance.config";
import development from "../playwright.config";
import ci from "../playwright.ci.config";

it("ordinary browser verification preserves development and optimized suites", () => {
  const { scripts } = JSON.parse(readFileSync(path.resolve("package.json"), "utf8"));
  expect(scripts.e2e).toBe("npm run e2e:guard && npm run e2e:development && npm run e2e:production-canvas");
  expect(scripts["e2e:guard"]).toBe("node scripts/qualify-browser-guard.mjs");
  expect(scripts["e2e:development"]).toContain("playwright test --config=playwright.config.ts");
  expect(scripts["e2e:production-canvas"]).toContain("--example e2e_backend --locked");
  expect(scripts["e2e:production-canvas"]).toContain("playwright test --config=playwright.production.config.ts");
});
it("CI compiles every full-suite adapter without narrowing discovery or weakening failure policy", () => {
  const { scripts } = JSON.parse(readFileSync(path.resolve("package.json"), "utf8"));
  expect(scripts["e2e:ci"]).toContain("npm run e2e:guard");
  expect(scripts["e2e:ci"]).toContain("--example e2e_backend --locked");
  expect(scripts["e2e:ci"]).toContain("vite build --config vite.browser-acceptance.config.ts");
  expect(scripts["e2e:ci"]).toContain("playwright test --config=playwright.ci.config.ts");
  expect(browserBuild.build?.outDir).toBe("../target/browser-acceptance-ui");
  expect(browserBuild.build?.emptyOutDir).toBe(true);
  expect(browserBuild.build?.rollupOptions?.input).toEqual([
    "index.html",
    "harness.html",
    "route-harness.html",
    "backend-harness.html",
  ]);
  expect(ci.testMatch).toEqual(development.testMatch);
  expect(ci.testIgnore).toEqual(development.testIgnore);
  expect(ci.retries).toBe(development.retries);
  expect(ci.failOnFlakyTests).toBe(development.failOnFlakyTests);
  expect(ci.expect).toEqual(development.expect);
  expect(ci.webServer).toMatchObject({ command: expect.stringContaining("vite preview"), reuseExistingServer: false });
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
