import commonjs from "@rollup/plugin-commonjs";
import nodeResolve from "@rollup/plugin-node-resolve";
import typescript from "@rollup/plugin-typescript";

const sdPlugin = "com.mrdesjardins.audiorouter.sdPlugin";

/** Bundle the plugin (and the Stream Deck SDK) into one ES module. */
export default {
  input: "src/plugin.ts",
  output: {
    file: `${sdPlugin}/bin/plugin.js`,
    format: "es",
    sourcemap: true,
    sourcemapPathTransform: (relativeSourcePath, sourcemapPath) => new URL(relativeSourcePath, `file:///${sourcemapPath}`).href,
  },
  plugins: [
    typescript({ noEmit: false, outDir: `${sdPlugin}/bin`, include: ["src/**/*.ts"] }),
    nodeResolve({ browser: false, exportConditions: ["node"], preferBuiltins: true }),
    commonjs(),
    {
      // The bundle is an ES module; Node needs this marker next to it.
      name: "emit-module-package-file",
      generateBundle() {
        this.emitFile({ fileName: "package.json", source: `{ "type": "module" }`, type: "asset" });
      },
    },
  ],
};
