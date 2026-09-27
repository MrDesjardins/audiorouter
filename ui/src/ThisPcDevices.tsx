import type { DeviceListItem } from "@audiorouter/contracts";

const ROLE_LABELS: Record<string, string> = { console: "Windows default", multimedia: "Windows default", communications: "Default for calls" };

/**
 * Read-only list of the audio devices Windows exposes on this PC. Choosing a
 * device happens in an Input or Output node's Properties; this list only shows
 * what is available and each device's format, and refreshes on demand.
 */
export function ThisPcDevices({ devices, connected, onRefresh }: { devices: DeviceListItem[]; connected: boolean; onRefresh: () => void }) {
  const byName = (left: DeviceListItem, right: DeviceListItem) => left.name.localeCompare(right.name, undefined, { sensitivity: "base" });
  const active = devices.filter((device) => device.state === "active").sort(byName);
  const inactive = devices.filter((device) => device.state !== "active").length;
  const group = (direction: "capture" | "render", heading: string) => {
    const items = active.filter((device) => device.direction === direction);
    return <div className="this-pc-device-group">
      <h4>{heading} <span className="badge">{items.length}</span></h4>
      {items.length === 0 ? <p className="muted">None found.</p> : <ul aria-label={heading}>{items.map((device) => {
        const roles = [...new Set(device.defaultRoles.map((role) => ROLE_LABELS[role]).filter(Boolean))];
        return <li key={device.id}>
          <strong>{device.name}</strong>
          <small>{"format" in device ? `${device.format.sampleRateHz / 1000} kHz · ${device.format.channels === 1 ? "mono" : device.format.channels === 2 ? "stereo" : `${device.format.channels} channels`}` : ""}{roles.length > 0 ? ` · ${roles.join(", ")}` : ""}</small>
        </li>;
      })}</ul>}
    </div>;
  };
  return <section className="panel this-pc-devices" aria-labelledby="this-pc-devices-heading">
    <div className="section-heading"><h3 id="this-pc-devices-heading">Audio devices on this PC</h3><button type="button" className="secondary" onClick={onRefresh} disabled={!connected}>Refresh</button></div>
    <p className="muted">What Windows offers right now. To use one, add an Input Device or Output Device tool and choose it in that node's Properties. AudioRouter never changes Windows defaults or volumes.</p>
    {group("capture", "Inputs (microphones, line-in, virtual cables)")}
    {group("render", "Outputs (speakers, headphones, virtual cables)")}
    {inactive > 0 && <p className="muted">{inactive} more {inactive === 1 ? "device is" : "devices are"} unplugged or disabled in Windows.</p>}
  </section>;
}
