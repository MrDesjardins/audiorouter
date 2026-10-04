import { useEffect, useState } from "react";
import type { NetworkNodeTelemetry, Node } from "@audiorouter/contracts";
import { DEFAULT_NETWORK_AUDIO_PORT } from "./draft";

const IPV4 = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/;

/** An IPv4 or IPv6 literal, as the backend accepts it. Host names are not
 * resolved, so a route never depends on DNS. */
export function isNetworkAddress(value: string): boolean {
  if (value.length === 0 || value.length > 64 || value.trim() !== value) return false;
  if (IPV4.test(value)) return true;
  // IPv6: hex groups with at most one "::"; the backend performs the exact check.
  return value.includes(":") && /^[0-9a-fA-F:.]+$/.test(value) && (value.match(/::/g) ?? []).length <= 1;
}

/** A one-line status of a running network node, or null when idle. */
export function networkTelemetryText(telemetry: NetworkNodeTelemetry | null | undefined): string | null {
  if (!telemetry) return null;
  if (telemetry.direction === "send") {
    const problems = [
      telemetry.droppedPackets ? `${telemetry.droppedPackets} dropped` : null,
      telemetry.sendErrors ? `${telemetry.sendErrors} send errors${telemetry.lastErrorCode === 10054 ? " (the receiving computer is not listening on this port)" : ""}` : null,
    ].filter(Boolean);
    return `Sending · ${telemetry.sentPackets ?? 0} packets${problems.length ? ` · ${problems.join(" · ")}` : ""}`;
  }
  const received = telemetry.receivedPackets ?? 0;
  if (received === 0) {
    if (telemetry.rejectedFrom) return `Waiting for audio · audio from ${telemetry.rejectedFrom} was ignored because it is not the address entered`;
    return telemetry.rejectedDatagrams
      ? `Waiting for audio · ${telemetry.rejectedDatagrams} packets from another address were ignored`
      : "Waiting for audio from the sending computer";
  }
  const problems = [
    telemetry.lostPackets ? `${telemetry.lostPackets} lost` : null,
    telemetry.latePackets ? `${telemetry.latePackets} late` : null,
    telemetry.underruns ? `${telemetry.underruns} gaps` : null,
  ].filter(Boolean);
  return `Receiving · ${received} packets · ${Math.round(telemetry.bufferedMs ?? 0)} ms buffered${problems.length ? ` · ${problems.join(" · ")}` : ""}`;
}

/** Properties of a Network Send or Network Receive node. Only valid values
 * reach the draft; the backend validates them again before audio starts. */
export function NetworkNodeEditor({ node, disabled, telemetry, onChange }: {
  node: Node;
  disabled: boolean;
  telemetry: NetworkNodeTelemetry | null | undefined;
  onChange: (name: string, value: number | string) => void;
}) {
  const sending = node.kind === "networkSend";
  const addressKey = sending ? "host" : "sender";
  const savedAddress = typeof node.parameters[addressKey] === "string" ? String(node.parameters[addressKey]) : "";
  const savedPort = Number(node.parameters.port ?? DEFAULT_NETWORK_AUDIO_PORT);
  const savedBuffer = Number(node.parameters.bufferMs ?? 40);
  const [address, setAddress] = useState(savedAddress);
  const [port, setPort] = useState(String(savedPort));
  const [buffer, setBuffer] = useState(String(savedBuffer));
  useEffect(() => setAddress(savedAddress), [node.id, savedAddress]);
  useEffect(() => setPort(String(savedPort)), [node.id, savedPort]);
  useEffect(() => setBuffer(String(savedBuffer)), [node.id, savedBuffer]);

  const addressValid = isNetworkAddress(address);
  const portNumber = Number(port);
  const portValid = Number.isInteger(portNumber) && portNumber >= 1 && portNumber <= 65535;
  const bufferNumber = Number(buffer);
  const bufferValid = Number.isFinite(bufferNumber) && bufferNumber >= 10 && bufferNumber <= 500;
  const status = networkTelemetryText(telemetry);
  const thisPc = sending ? telemetry?.localAddress : telemetry?.thisAddress;
  const addressLabel = sending ? "Receiving computer's IP address" : "Sending computer's IP address";

  return <div className="node-binding-editor network-node-editor" aria-label={sending ? "Network send settings" : "Network receive settings"}>
    <div><p className="eyebrow">{sending ? "Stream to another computer" : "Play audio from another computer"}</p><strong>{sending ? "Where should this audio go?" : "Which computer sends the audio?"}</strong></div>
    <figure className="network-route-diagram" aria-label="Network audio direction">
      <div className="network-route-computer"><svg viewBox="0 0 48 36" aria-hidden="true"><rect x="6" y="2" width="36" height="24" rx="3" /><path d="M18 34h12M24 26v8M12 16h4l3-7 6 13 4-9h7" /></svg><strong>{sending ? "This PC" : "Sending PC"}</strong><span>Audio in</span><b>Network Send</b><code>{sending ? "Your input / mix" : addressValid ? address : "Sender IP"}</code></div>
      <div className="network-route-link"><span aria-hidden="true">→</span><small>UDP</small><code>{portValid ? portNumber : "Port"}</code></div>
      <div className="network-route-computer"><svg viewBox="0 0 48 36" aria-hidden="true"><rect x="6" y="2" width="36" height="24" rx="3" /><path d="M18 34h12M24 26v8M12 16h4l3-7 6 13 4-9h7" /></svg><strong>{sending ? "Receiving PC" : "This PC"}</strong><span>Audio out</span><b>Network Receive</b><code>{sending ? addressValid ? address : "Destination IP" : "Your output / OBS"}</code></div>
      <figcaption>{sending ? `Set Receive's sender to ${thisPc ?? "this PC's IP"}.` : `Set Send's destination to ${thisPc ?? "this PC's IP"}.`} Use port {portValid ? portNumber : "the same port"} on both PCs.</figcaption>
    </figure>
    <label>{addressLabel}<input type="text" inputMode="decimal" autoComplete="off" spellCheck={false} placeholder="192.168.1.20" value={address} disabled={disabled} aria-invalid={address.length > 0 && !addressValid}
      onChange={(event) => { const value = event.target.value.trim(); setAddress(value); if (isNetworkAddress(value)) onChange(addressKey, value); }} /></label>
    {address.length > 0 && !addressValid && <small role="alert">Enter a numeric IP address such as 192.168.1.20. Computer names are not used.</small>}
    <label>Port<input type="number" min={1} max={65535} step={1} value={port} disabled={disabled} aria-invalid={!portValid}
      onChange={(event) => { setPort(event.target.value); const value = Number(event.target.value); if (Number.isInteger(value) && value >= 1 && value <= 65535) onChange("port", value); }} /></label>
    {!sending && <label>Buffer (ms)<input type="number" min={10} max={500} step={5} value={buffer} disabled={disabled} aria-invalid={!bufferValid}
      onChange={(event) => { setBuffer(event.target.value); const value = Number(event.target.value); if (Number.isFinite(value) && value >= 10 && value <= 500) onChange("bufferMs", value); }} /></label>}
    {status && <p className="network-node-status" role="status">{status}</p>}
    {!sending && telemetry?.rejectedFrom && telemetry.rejectedFrom !== address && <div className="network-node-fix" role="alert">
      <span>AudioRouter audio is arriving from <code>{telemetry.rejectedFrom}</code>. If that is the sending computer, use its address.</span>
      <button type="button" className="secondary" disabled={disabled} onClick={() => { setAddress(telemetry.rejectedFrom!); onChange("sender", telemetry.rejectedFrom!); }}>Use {telemetry.rejectedFrom}</button>
    </div>}
    {/* While playing, name this computer's real address: with several network
        adapters, the one ipconfig lists first is often not the one used. */}
    <small>{sending
      ? <>On the other computer, add a Network Receive node with {thisPc ? <>this computer's address <code>{thisPc}</code></> : "this computer's IP address"} as the sender and the same port. Both computers must be on the same local network. Audio is sent unencrypted, so use it only on a network you trust.</>
      : <>On the other computer, add a Network Send node with {thisPc ? <>this computer's address <code>{thisPc}</code></> : "this computer's IP address"} and port {portValid ? portNumber : "the same port"}. Audio from any other address is ignored. The first time, Windows Firewall may ask you to allow AudioRouter on private networks. A larger buffer rides out Wi-Fi hiccups but adds delay.</>}
      {!thisPc && <>{" "}To find a computer's IP address, run <code>ipconfig</code> and read its IPv4 Address.</>}</small>
  </div>;
}
