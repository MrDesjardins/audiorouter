import type { AppCheck } from "./appUsage";
import { KNOWN_APPS } from "./appUsage";
import { Icon, type IconName } from "./icons";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

const WORD: Record<AppCheck["state"], MessageKey> = {
  ready: "appStatus.ready",
  check: "appStatus.needsCheck",
  notRunning: "appStatus.notRunning",
  unknown: "appStatus.waiting",
};
const ICON: Record<AppCheck["state"], IconName> = {
  ready: "check",
  check: "triangle-alert",
  notRunning: "minus",
  unknown: "ellipsis",
};

/**
 * Glanceable app status beside "Audio running". Each chip has a fixed width
 * and keeps its place as the state changes (UI-17); the state is in the
 * accessible name and the tooltip, never only in colour.
 */
export function AppStatusStrip({ checks, onOpen }: { checks: AppCheck[]; onOpen: (check: AppCheck) => void }) {
  const tr = useMessages();
  if (checks.length === 0) return null;
  return (
    <span className="app-status-strip" role="group" aria-label={tr("workbench.apps")}>
      {checks.map((check) => {
        const label = KNOWN_APPS[check.appId].label;
        return (
          <button
            key={check.appId}
            type="button"
            className={`app-status-chip is-${check.state}`}
            aria-label={tr("appStatus.aria", { app: label, state: tr(WORD[check.state]) })}
            title={`${label}: ${check.summary}`}
            onClick={() => onOpen(check)}
          >
            <span className="app-status-name">{label}</span>
            <span className="app-status-icon" aria-hidden="true">
              <Icon name={ICON[check.state]} size={14} />
            </span>
          </button>
        );
      })}
    </span>
  );
}
