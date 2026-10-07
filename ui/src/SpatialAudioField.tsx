import type { DeviceListItem } from "@audiorouter/contracts";

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
  const device = devices.find(
    (item): item is Extract<DeviceListItem, { state: "active" }> => item.state === "active" && item.id === endpointId,
  );
  const channels = device?.format.channels ?? null;
  const selected: SpatialMode = mode === "headphones" || mode === "speakers" ? mode : "off";
  const room = Math.round(Math.min(100, Math.max(0, Number.isFinite(roomPercent) ? roomPercent : 0)));
  const layout = channels === 8 ? "7.1" : "5.1";
  const guidance =
    selected === "off"
      ? "Plays the input as it is. Choose a surround mode to place a 5.1/7.1 game mix around you."
      : channels === null
        ? "Choose the input device above. It must carry 5.1 or 7.1 audio; a 7.1 playback device appears as a Loopback choice."
        : channels === 6 || channels === 8
          ? selected === "headphones"
            ? `Renders this ${layout} input with a measured head (MIT KEMAR) to stereo for headphones.`
            : `Renders this ${layout} input with a measured head (MIT KEMAR) for two speakers about ±30° in front of you, cancelling the sound each ear hears from the far speaker. Sit centred between them.`
          : `This device has ${channels} channel(s); Play will refuse it. In Windows Sound settings, set the game's playback device (for example CABLE-B Input) to 7.1 with Configure speakers, then choose it above as a Loopback device.`;
  return (
    <div className="node-binding-editor" aria-label="Spatial audio settings">
      <label>
        Spatial audio
        <select
          aria-label="Spatial audio mode"
          value={selected}
          disabled={disabled}
          onChange={(event) => onChange("spatialMode", event.target.value as SpatialMode)}
        >
          <option value="off">Off (stereo as captured)</option>
          <option value="headphones">Surround to headphones (5.1/7.1)</option>
          <option value="speakers">Surround to speakers (5.1/7.1)</option>
        </select>
      </label>
      {selected !== "off" && (
        <label className="param-row spatial-room">
          <span className="param-caption">
            <span>Room</span>
            <b aria-hidden="true">{room === 0 ? "Off" : `${room} %`}</b>
          </span>
          <input
            type="range"
            aria-label="Room"
            aria-valuetext={room === 0 ? "Off" : `${room} percent`}
            min={0}
            max={100}
            step={1}
            value={room}
            disabled={disabled}
            onChange={(event) => onChange("spatialRoomPercent", Number(event.target.value))}
          />
          <small>Adds a small room around the virtual speakers; higher sounds further away.</small>
        </label>
      )}
      <small>
        {guidance}
        {running ? " Changing these takes effect after Stop and Play." : ""}
      </small>
    </div>
  );
}
