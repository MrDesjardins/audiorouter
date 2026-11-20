import type { DeviceListItem } from "@audiorouter/contracts";
import { setDeviceHidden, useHiddenDevices } from "./hiddenDevices";
import { BooleanField } from "./BooleanField";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";
import { formatNumber } from "./i18n/format";

const ROLE_LABELS: Record<string, MessageKey> = {
  console: "devices.windowsDefault",
  multimedia: "devices.windowsDefault",
  communications: "devices.defaultForCalls",
};

/**
 * List of the audio devices Windows exposes on this PC. Choosing a
 * device happens in an Input or Output node's Properties; this list only shows
 * what is available and each device's format, and refreshes on demand. Its
 * "Show in pickers" switches hide unused devices from those pickers (UI-22).
 */
export function ThisPcDevices({
  devices,
  connected,
  onRefresh,
}: {
  devices: DeviceListItem[];
  connected: boolean;
  onRefresh: () => void;
}) {
  const tr = useMessages();
  const hidden = useHiddenDevices();
  const byName = (left: DeviceListItem, right: DeviceListItem) =>
    left.name.localeCompare(right.name, undefined, { sensitivity: "base" });
  const active = devices.filter((device) => device.state === "active").sort(byName);
  const inactive = devices.filter((device) => device.state !== "active").length;
  const group = (direction: "capture" | "render", heading: string) => {
    const items = active.filter((device) => device.direction === direction);
    return (
      <div className="this-pc-device-group">
        <h4>
          {heading} <span className="badge">{items.length}</span>
        </h4>
        {items.length === 0 ? (
          <p className="muted">{tr("devices.noneFound")}</p>
        ) : (
          <ul aria-label={heading}>
            {items.map((device) => {
              const roles = [
                ...new Set(
                  device.defaultRoles
                    .map((role) => ROLE_LABELS[role])
                    .filter((role): role is MessageKey => Boolean(role))
                    .map((role) => tr(role)),
                ),
              ];
              const shown = !hidden.has(device.id);
              return (
                <li key={device.id} className={shown ? undefined : "is-hidden-device"}>
                  <strong>{device.name}</strong>
                  <small>
                    {"format" in device
                      ? tr("devices.formatSummary", {
                          rate: formatNumber(device.format.sampleRateHz / 1000, { maximumFractionDigits: 1 }),
                          layout:
                            device.format.channels === 1
                              ? tr("devices.mono")
                              : device.format.channels === 2
                                ? tr("devices.stereo")
                                : tr("devices.channels", { count: device.format.channels }),
                        })
                      : ""}
                    {roles.length > 0 ? ` · ${roles.join(", ")}` : ""}
                  </small>
                  <BooleanField
                    label={tr("devices.showInPickers")}
                    name={tr("devices.showDevice", { name: device.name })}
                    checked={shown}
                    onChange={(checked) => setDeviceHidden(device.id, !checked)}
                  />
                </li>
              );
            })}
          </ul>
        )}
      </div>
    );
  };
  return (
    <section className="panel this-pc-devices" aria-labelledby="this-pc-devices-heading">
      <div className="section-heading">
        <h3 id="this-pc-devices-heading">{tr("devices.heading")}</h3>
        <button type="button" className="secondary" onClick={onRefresh} disabled={!connected}>
          {tr("devices.refresh")}
        </button>
      </div>
      <p className="muted">{tr("devices.help")}</p>
      {group("capture", tr("devices.inputs"))}
      {group("render", tr("devices.outputs"))}
      {inactive > 0 && <p className="muted">{tr("devices.inactive", { count: inactive })}</p>}
    </section>
  );
}
