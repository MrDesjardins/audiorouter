import { useCallback, useEffect, useRef, useState } from "react";
import type { SetupCheckResult } from "@audiorouter/contracts";

export type SetupCheckState = {
  connected: boolean;
  available: boolean;
  result: SetupCheckResult | null;
  checking: boolean;
  failure: string | null;
  run: () => Promise<void>;
};

/** One shared check for the Status indicator and panel, independent of tab. */
export function useSetupCheck({
  connected,
  checkSetup,
  refreshKey,
  ready = true,
}: {
  connected: boolean;
  checkSetup?: () => Promise<SetupCheckResult>;
  refreshKey: string;
  ready?: boolean;
}): SetupCheckState {
  const [result, setResult] = useState<SetupCheckResult | null>(null);
  const [checking, setChecking] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const latest = useRef(0);
  const run = useCallback(async () => {
    if (!connected || !ready || !checkSetup) return;
    const request = ++latest.current;
    setChecking(true);
    try {
      const next = await checkSetup();
      if (request !== latest.current) return;
      setResult(next);
      setFailure(null);
    } catch (error) {
      if (request !== latest.current) return;
      setFailure(error instanceof Error ? error.message : "The check could not run.");
    } finally {
      if (request === latest.current) setChecking(false);
    }
  }, [connected, ready, checkSetup]);
  useEffect(() => {
    if (connected && ready) void run();
    else setChecking(false);
    return () => {
      latest.current += 1;
    };
  }, [connected, ready, run, refreshKey]);
  return { connected, available: checkSetup !== undefined, result, checking, failure, run };
}
