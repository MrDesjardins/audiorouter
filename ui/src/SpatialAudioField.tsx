import type { DeviceListItem } from "@audiorouter/contracts";
import { SliderField } from "./Slider";
import { useMessages } from "./i18n/useMessages";

type SpatialMode = "off" | "headphones" | "speakers";

/**
 * Physical Input surround rendering (5.1/7.1 capture → two ears): for
 * headphones, or for two speakers with crosstalk cancellation, plus an
 * optional room around the virtual speakers.
 */
export function SpatialAudioField({
  devices,
  endpointId,
  mode,
  roomPercent,
  disabled,
  running,
  onChange,
}: {
  devices: DeviceListItem[];
  endpointId: string;
  mode: string;
  roomPercent: number;
  disabled: boolean;
  running: boolean;
  onChange: (name: "spatialMode" | "spatialRoomPercent", value: SpatialMode | number) => void;
}) {
  const tr = useMessages();
  const device = devices.find(
    (item): item is Extract<DeviceListItem, { state: "active" }> => item.state === "active" && item.id === endpointId,
  );
  const channels = device?.format.channels ?? null;
  const selected: SpatialMode = mode === "headphones" || mode === "speakers" ? mode : "off";
  const room = Math.round(Math.min(100, Math.max(0, Number.isFinite(roomPercent) ? roomPercent : 0)));
  const layout = channels === 8 ? "7.1" : "5.1";
  const guidance =
    selected === "off"
      ? tr("g2.spGuideOff")
      : channels === null
        ? tr("g2.spGuideNoDevice")
        : channels === 6 || channels === 8
          ? selected === "headphones"
            ? tr("g2.spGuideHeadphones", { layout })
            : tr("g2.spGuideSpeakers", { layout })
          : tr("g2.spGuideChannels", { n: channels });
  return (
    <div className="node-binding-editor" aria-label={tr("g2.spSettingsAria")}>
      <label>
        {tr("g2.spLabel")}
        <select
          aria-label={tr("g2.spModeAria")}
          value={selected}
          disabled={disabled}
          onChange={(event) => onChange("spatialMode", event.target.value as SpatialMode)}
        >
          <option value="off">{tr("g2.spOff")}</option>
          <option value="headphones">{tr("g2.spHeadphones")}</option>
          <option value="speakers">{tr("g2.spSpeakers")}</option>
        </select>
      </label>
      {selected !== "off" && (
        <SliderField
          className="spatial-room"
          label={tr("g2.spRoom")}
          name={tr("g2.spRoom")}
          valueText={room === 0 ? tr("g2.spRoomOff") : tr("g2.spRoomPercent", { n: room })}
          value={room}
          min={0}
          max={100}
          step={1}
          unit="%"
          defaultValue={0}
          disabled={disabled}
          help={<small>{tr("g2.spRoomHelp")}</small>}
          onChange={(value) => onChange("spatialRoomPercent", value)}
        />
      )}
      <small>
        {guidance}
        {running ? ` ${tr("g2.spTakesEffect").trim()}` : ""}
      </small>
    </div>
  );
}
