import type { DeviceListItem } from "@audiorouter/contracts";

export function FirstRunGuide({
  devices,
  connected,
  onRefresh,
  onOpenTools,
}: {
  devices: DeviceListItem[];
  connected: boolean;
  onRefresh: () => void;
  onOpenTools: () => void;
}) {
  const active = devices.filter((device) => device.state === "active");
  const captures = active.filter((device) => device.direction === "capture").length;
  const renders = active.filter((device) => device.direction === "render").length;
  const hasRouteEndpoints = captures > 0 && renders > 0;

  return (
    <section className="panel setup-panel first-run-guide" aria-labelledby="first-run-guide-heading">
      <p className="eyebrow">Getting started</p>
      <h3 id="first-run-guide-heading">Set up a route on this PC</h3>
      {!connected ? (
        <p className="muted" role="status">
          Connect to AudioRouter to check available devices.
        </p>
      ) : hasRouteEndpoints ? (
        <p className="muted" role="status">
          Found {captures} input{captures === 1 ? "" : "s"} and {renders} output{renders === 1 ? "" : "s"}. Choose the
          exact devices in your route's node Properties.
        </p>
      ) : (
        <p className="muted" role="status">
          No complete input-and-output pair is available yet. Connect or enable your audio devices, then refresh this
          list. AudioRouter does not install virtual audio drivers.
        </p>
      )}
      <ol>
        <li>Open Tools and add an Input Device and Output Device, or start with Test Signal.</li>
        <li>Choose an available device in each node's Properties, then connect the route.</li>
        <li>Review the route and press Play when you are ready. AudioRouter never changes Windows default devices.</li>
      </ol>
      <div className="setup-guide-actions">
        <button type="button" onClick={onOpenTools} disabled={!connected}>
          Open Tools
        </button>
        <button type="button" className="secondary" onClick={onRefresh} disabled={!connected}>
          Refresh devices
        </button>
      </div>
      <small>
        Virtual-microphone workflows require an existing supported endpoint such as VB-Cable or Voicemeeter. Install
        that endpoint separately from its vendor.
      </small>
    </section>
  );
}
