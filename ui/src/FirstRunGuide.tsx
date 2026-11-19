import type { DeviceListItem } from "@audiorouter/contracts";
import { GoalSetupGuide } from "./GoalSetupGuide";
import { useMessages } from "./i18n/useMessages";

export function FirstRunGuide({
  devices,
  connected,
  onRefresh,
  onOpenTools,
  onSetUpDiscord,
}: {
  devices: DeviceListItem[];
  connected: boolean;
  onRefresh: () => void;
  onOpenTools: () => void;
  /** Opens the guided Discord route ("Set up for Discord"). */
  onSetUpDiscord?: () => void;
}) {
  const tr = useMessages();
  const active = devices.filter((device) => device.state === "active");
  const captures = active.filter((device) => device.direction === "capture").length;
  const renders = active.filter((device) => device.direction === "render").length;
  const hasRouteEndpoints = captures > 0 && renders > 0;

  return (
    <section className="panel setup-panel first-run-guide" aria-labelledby="first-run-guide-heading">
      <p className="eyebrow">{tr("firstRun.eyebrow")}</p>
      <h3 id="first-run-guide-heading">{tr("firstRun.heading")}</h3>
      <GoalSetupGuide connected={connected} onOpenTools={onOpenTools} onSetUpDiscord={onSetUpDiscord} />
      {!connected ? (
        <p className="muted" role="status">
          {tr("firstRun.connect")}
        </p>
      ) : hasRouteEndpoints ? (
        <p className="muted" role="status">
          {tr("firstRun.found", {
            count: captures,
            renders,
            outputs: tr(renders === 1 ? "firstRun.outputOne" : "firstRun.outputOther"),
          })}
        </p>
      ) : (
        <p className="muted" role="status">
          {tr("firstRun.noPair")}
        </p>
      )}
      <ol>
        <li>{tr("firstRun.step1")}</li>
        <li>{tr("firstRun.step2")}</li>
        <li>{tr("firstRun.step3")}</li>
      </ol>
      <div className="setup-guide-actions actions">
        <button type="button" onClick={onOpenTools} disabled={!connected}>
          {tr("firstRun.openTools")}
        </button>
        {onSetUpDiscord && (
          <button type="button" className="secondary" onClick={onSetUpDiscord} disabled={!connected}>
            {tr("firstRun.setUpDiscord")}
          </button>
        )}
        <button type="button" className="secondary" onClick={onRefresh} disabled={!connected}>
          {tr("firstRun.refresh")}
        </button>
      </div>
      <small>{tr("firstRun.virtualCable")}</small>
    </section>
  );
}
