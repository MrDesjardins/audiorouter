import type { DeviceListItem } from "@audiorouter/contracts";
import { useMessages } from "./i18n/useMessages";

export type InputChannelMode = "stereo" | "mono" | "left" | "right";

const MODES = ["stereo", "mono", "left", "right"] as const;

/**
 * Physical Input channel handling (CAP-03): a stereo audio interface often
 * delivers a single microphone on one channel, so it is heard in one ear.
 * The mono choices send one signal to both channels.
 */
export function InputChannelsField({
  devices,
  endpointId,
  mode,
  surround,
  disabled,
  onChange,
}: {
  devices: DeviceListItem[];
  endpointId: string;
  mode: unknown;
  surround: boolean;
  disabled: boolean;
  onChange: (name: "channelMode", value: InputChannelMode) => void;
}) {
  const tr = useMessages();
  const selected = MODES.find((item) => item === mode) ?? "stereo";
  const device = devices.find(
    (item): item is Extract<DeviceListItem, { state: "active" }> => item.state === "active" && item.id === endpointId,
  );
  const guidance = surround
    ? "workflowUi.spatial"
    : device?.format.channels === 1
      ? "workflowUi.monoDevice"
      : selected === "stereo"
        ? "workflowUi.stereoHelp"
        : selected === "mono"
          ? "workflowUi.monoHelp"
          : selected === "left"
            ? "workflowUi.leftHelp"
            : "workflowUi.rightHelp";
  return (
    <div className="node-binding-editor" aria-label={tr("workflowUi.inputSettings")}>
      <label>
        {tr("workflowUi.channels")}
        <select
          aria-label={tr("workflowUi.inputChannels")}
          value={selected}
          disabled={disabled || surround}
          onChange={(event) => onChange("channelMode", event.target.value as InputChannelMode)}
        >
          {MODES.map((item) => (
            <option key={item} value={item}>
              {tr(`workflowUi.${item}`)}
            </option>
          ))}
        </select>
      </label>
      <small className="input-channels-guidance">{tr(guidance)}</small>
    </div>
  );
}
