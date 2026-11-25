import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

// Shared by the smoke runner and its qualifier so a call site cannot reference
// a helper that one copy dropped. The smoke startup-failure diagnostic used to
// call `windows` with no definition, so it failed with "windows is not defined".
export const windowsHelperPath = fileURLToPath(
  new URL("./packaged-shell-windows.ps1", import.meta.url),
);

export function windows(operation, extra = []) {
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      windowsHelperPath,
      "-Operation",
      operation,
      ...extra,
    ],
    { encoding: "utf8", windowsHide: true, timeout: 15_000 },
  );
  if (result.error || result.status !== 0)
    throw new Error(
      `${operation} helper failed: ${result.error?.message ?? result.stderr}`,
    );
  return JSON.parse(result.stdout.trim());
}