import type { DeviceListItem } from "@audiorouter/contracts";

export type InputChannelMode = "stereo" | "mono" | "left" | "right";

const MODES: ReadonlyArray<{ value: InputChannelMode; label: string }> = [
  { value: "stereo", label: "Stereo (as captured)" },
  { value: "mono", label: "Mono: mix both channels" },
  { value: "left", label: "Mono: left channel only (input 1)" },
  { value: "right", label: "Mono: right channel only (input 2)" },
];

/**
 * Physical Input channel handling (CAP-03): a stereo audio interface often
 * delivers a single microphone on one channel, so it is heard in one ear.
 * The mono choices send one signal to both channels.
 */
export function InputChannelsField({ devices, endpointId, mode, surround, disabled, onChange }: {
  devices: DeviceListItem[]; endpointId: string; mode: unknown; surround: boolean; disabled: boolean;
  onChange: (name: "channelMode", value: InputChannelMode) => void;
}) {
  const selected: InputChannelMode = MODES.some((item) => item.value === mode) ? mode as InputChannelMode : "stereo";
  const device = devices.find((item): item is Extract<DeviceListItem, { state: "active" }> => item.state === "active" && item.id === endpointId);
  const guidance = surround
    ? "Not used while spatial audio renders surround to two ears."
    : device?.format.channels === 1
      ? "This device is mono; its sound already reaches both ears."
      : selected === "stereo"
        ? "Keeps left and right apart. If your voice is only in one ear, choose a mono option."
        : selected === "mono"
          ? "Mixes left and right into one signal sent to both ears."
          : `Sends only the ${selected} channel to both ears; the other channel is ignored. Use it for one microphone on a stereo interface.`;
  return <div className="node-binding-editor" aria-label="Input channel settings">
    <label>Channels<select aria-label="Input channels" value={selected} disabled={disabled || surround} onChange={(event) => onChange("channelMode", event.target.value as InputChannelMode)}>
      {MODES.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
    </select></label>
    <small className="input-channels-guidance">{guidance}</small>
  </div>;
}
