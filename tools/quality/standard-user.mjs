import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

// Elevated WebView2 ignores both the debugger and disposable-profile overrides.
// Refuse before launching a shell, even when invoked outside the CI wrapper.
export function assertStandardUser() {
  const probe = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      fileURLToPath(new URL("./standard-user-process.ps1", import.meta.url)),
      "-InspectOnly",
    ],
    { encoding: "utf8", timeout: 10_000, windowsHide: true },
  );
  if (probe.status !== 0)
    throw new Error(
      `Cannot inspect verification token: status=${probe.status}, signal=${probe.signal}, error=${probe.error?.message ?? "none"}; stdout=${(probe.stdout ?? "").slice(-4096)}; stderr=${(probe.stderr ?? "").slice(-4096)}`,
    );
  const identity = JSON.parse(probe.stdout);
  if (identity.parentIntegrityRid !== 8192 || identity.parentAdmin !== false)
    throw new Error(
      "Packaged smoke requires Medium integrity without administrator membership; use standard-user-process.ps1",
    );
  return {
    integrityRid: identity.parentIntegrityRid,
    isAdministrator: identity.parentAdmin,
  };
}
