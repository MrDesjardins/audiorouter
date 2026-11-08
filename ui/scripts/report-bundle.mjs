// Reproduce production chunk ownership without overwriting the embedded UI.
import { build } from "vite";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, URL } from "node:url";
import process from "node:process";
import { Buffer } from "node:buffer";
import console from "node:console";

const root = fileURLToPath(new URL("..", import.meta.url));
const output = path.resolve(process.argv[2] ?? path.join(root, "../target/ui-bundle-report"));
if (output === root || !output.startsWith(path.resolve(root, "../target") + path.sep))
  throw new Error("Bundle report output must be a directory under the repository target directory");

await build({
  root,
  configFile: path.join(root, "vite.config.ts"),
  configLoader: "runner",
  // The report uses only this build's bundle graph; retaining unrelated old
  // files avoids recursive cleanup of caller-selected directories/junctions.
  build: { outDir: path.join(output, "assets"), emptyOutDir: false },
  plugins: [
    {
      name: "audiorouter-bundle-report",
      async writeBundle(_options, bundle) {
        const chunks = Object.values(bundle)
          .filter((item) => item.type === "chunk")
          .map((chunk) => ({
            file: chunk.fileName,
            entry: chunk.isEntry,
            bytes: Buffer.byteLength(chunk.code),
            imports: chunk.imports,
            dynamicImports: chunk.dynamicImports,
            modules: Object.entries(chunk.modules)
              .map(([id, module]) => ({
                module: path.relative(root, id).replaceAll("\\", "/"),
                renderedLength: module.renderedLength,
              }))
              .sort((a, b) => b.renderedLength - a.renderedLength),
          }))
          .sort((a, b) => b.bytes - a.bytes);
        await mkdir(output, { recursive: true });
        await writeFile(path.join(output, "report.json"), JSON.stringify({ chunks }, null, 2) + "\n");
        console.log(`Bundle ownership report: ${path.join(output, "report.json")}`);
      },
    },
  ],
});
