// Require native read-back evidence from this launch, not an earlier process.
export function requireDisabledBrowserKeys(log, processId, launchedAt) {
  const record = log
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line))
    .findLast(
      (item) =>
        item.method === "shell.browserAccelerators" &&
        item.processId === processId &&
        Number.isSafeInteger(item.timeUnixMs) &&
        item.timeUnixMs >= launchedAt,
    );
  if (record?.outcome !== "ok" || record.detail?.state !== "disabled")
    throw new Error(
      `Native browser accelerators were not verified disabled for process ${processId}`,
    );
  return { processId, state: record.detail.state };
}
