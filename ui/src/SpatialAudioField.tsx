import type { DeviceListItem } from "@audiorouter/contracts";

/** Physical Input "surround to headphones" mode (5.1/7.1 capture → binaural stereo). */
export function SpatialAudioField({ devices, endpointId, mode, disabled, running, onChange }: { devices: DeviceListItem[]; endpointId: string; mode: string; disabled: boolean; running: boolean; onChange: (mode: "off" | "headphones") => void }) {
  const device = devices.find((item): item is Extract<DeviceListItem, { state: "active" }> => item.state === "active" && item.id === endpointId);
  const channels = device?.format.channels ?? null;
  const headphones = mode === "headphones";
  const guidance = !headphones
    ? "Plays the input as it is. Choose Surround to headphones to place a 5.1/7.1 game mix around you on headphones."
    : channels === null
      ? "Choose the input device above. It must carry 5.1 or 7.1 audio; a 7.1 playback device appears as a Loopback choice."
      : channels === 6 || channels === 8
        ? `Renders this ${channels === 8 ? "7.1" : "5.1"} input with a measured head (MIT KEMAR) to stereo for headphones.`
        : `This device has ${channels} channel(s); Play will refuse it. In Windows Sound settings, set the game's playback device (for example CABLE-B Input) to 7.1 with Configure speakers, then choose it above as a Loopback device.`;
  return <div className="node-binding-editor" aria-label="Spatial audio settings"><label>Spatial audio<select aria-label="Spatial audio mode" value={headphones ? "headphones" : "off"} disabled={disabled} onChange={(event) => onChange(event.target.value === "headphones" ? "headphones" : "off")}><option value="off">Off (stereo as captured)</option><option value="headphones">Surround to headphones (5.1/7.1)</option></select></label><small>{guidance}{running ? " Changing this takes effect after Stop and Play." : ""}</small></div>;
}
