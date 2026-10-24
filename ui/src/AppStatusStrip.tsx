import type { AppCheck } from "./appUsage";
import { KNOWN_APPS } from "./appUsage";

const WORD: Record<AppCheck["state"], string> = {
  ready: "ready",
  check: "needs a check",
  notRunning: "not running",
  unknown: "waiting",
};
const ICON: Record<AppCheck["state"], string> = { ready: "✓", check: "!", notRunning: "–", unknown: "…" };

/**
 * Glanceable app status beside "Audio running". Each chip has a fixed width
 * and keeps its place as the state changes (UI-17); the state is in the
 * accessible name and the tooltip, never only in colour.
 */
export function AppStatusStrip({ checks, onOpen }: { checks: AppCheck[]; onOpen: (check: AppCheck) => void }) {
  if (checks.length === 0) return null;
  return (
    <span className="app-status-strip" role="group" aria-label="Apps">
      {checks.map((check) => {
        const label = KNOWN_APPS[check.appId].label;
        return (
          <button
            key={check.appId}
            type="button"
            className={`app-status-chip is-${check.state}`}
            aria-label={`${label}: ${WORD[check.state]}. Open Apps.`}
            title={`${label}: ${check.summary}`}
            onClick={() => onOpen(check)}
          >
            <span className="app-status-name">{label}</span>
            <span className="app-status-icon" aria-hidden="true">
              {ICON[check.state]}
            </span>
          </button>
        );
      })}
    </span>
  );
}
