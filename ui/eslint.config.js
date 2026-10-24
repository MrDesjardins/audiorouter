// ESLint flat config for the UI (code review P2-2). CI fails on any error.
// `react-hooks/exhaustive-deps` is an error. Never satisfy it by blindly
// adding dependencies: effects fed by 20 Hz telemetry can then loop. A
// callback that must be current but must not restart the effect goes
// through `useEffectEvent`; a value is depended on as a primitive or memo.
import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist/", "dist-review-*/", "coverage/", "node_modules/"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: { ecmaVersion: 2022, globals: { ...globals.browser, ...globals.node } },
    rules: {
      // A leading underscore marks a deliberately unused parameter (fixture
      // stubs with the real signature) or binding.
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
          destructuredArrayIgnorePattern: "^_",
        },
      ],
    },
  },
  {
    // React code only: Playwright fixtures in e2e/ call a `use` callback that
    // is not React's `use` hook.
    files: ["src/**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: {
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "error",
    },
  },
);
