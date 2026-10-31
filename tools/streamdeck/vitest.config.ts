import { defineConfig } from "vitest/config";
import ts from "typescript";

export default defineConfig({
  plugins: [{
    name: "streamdeck-action-decorators",
    enforce: "pre",
    transform(source, id) {
      if (!id.replaceAll("\\", "/").includes("/src/actions/") || !id.endsWith(".ts")) return;
      return ts.transpileModule(source, {
        compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
        fileName: id,
      }).outputText;
    },
  }],
});
