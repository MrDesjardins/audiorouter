// Keep the full Windows environment needed by WebView2, replacing only the
// smoke-owned state and removing switches that change startup authorization or
// pause/perturb the frontend. Windows environment names are case insensitive.
export function packagedShellEnvironment(inherited, paths) {
  const replaced = new Set([
    "AUDIOROUTER_ALLOW_DEVICE_ADMIN",
    "AUDIOROUTER_CONTROL_PIPE",
    "AUDIOROUTER_STARTUP_METRICS",
    "AUDIOROUTER_SHELL_PROBE_FILE",
    "WEBVIEW2_WAIT_FOR_SCRIPT_DEBUGGER",
    "WEBVIEW2_PIPE_FOR_SCRIPT_DEBUGGER",
    "AUDIOROUTER_DATABASE",
    "LOCALAPPDATA",
    "APPDATA",
    "WEBVIEW2_USER_DATA_FOLDER",
    "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
  ]);
  const env = Object.fromEntries(
    Object.entries(inherited).filter(
      ([key]) => !replaced.has(key.toUpperCase()),
    ),
  );
  return {
    ...env,
    AUDIOROUTER_DATABASE: paths.database,
    LOCALAPPDATA: paths.localAppData,
    APPDATA: paths.appData,
    WEBVIEW2_USER_DATA_FOLDER: paths.webview,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
      "--remote-debugging-port=0 --remote-debugging-address=127.0.0.1 --remote-allow-origins=http://localhost",
  };
}
