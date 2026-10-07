// Keyboard connection form of the Advanced tab (moved from App.tsx).
import type { Dispatch, SetStateAction } from "react";
import type { UiBackend } from "./backend";

export function ConnectionForm({
  backend,
  connectionSource,
  setConnectionSource,
  outputPorts,
  encodePort,
  connectionDestination,
  setConnectionDestination,
  inputPorts,
  addConnection,
}: {
  backend: UiBackend;
  connectionSource: string;
  setConnectionSource: Dispatch<SetStateAction<string>>;
  outputPorts: { nodeId: string; nodeName: string; portName: string; channels: number }[];
  encodePort: (nodeId: string, portName: string) => string;
  connectionDestination: string;
  setConnectionDestination: Dispatch<SetStateAction<string>>;
  inputPorts: { nodeId: string; nodeName: string; portName: string; channels: number }[];
  addConnection: () => boolean;
}) {
  return (
    <fieldset className="connection-editor workbench-connection-editor" disabled={!backend.connected}>
      <legend>Add connections to the draft</legend>
      <p className="muted">
        Choose an output and input for each link. These connections are reviewed when you plan the graph.
      </p>
      <label>
        Output
        <select
          aria-label="Source output port"
          value={connectionSource}
          onChange={(event) => setConnectionSource(event.target.value)}
        >
          <option value="">Choose source</option>
          {outputPorts.map((port) => (
            <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
              {port.nodeName} · {port.portName} · {port.channels}ch
            </option>
          ))}
        </select>
      </label>
      <label>
        Input
        <select
          aria-label="Destination input port"
          value={connectionDestination}
          onChange={(event) => setConnectionDestination(event.target.value)}
        >
          <option value="">Choose destination</option>
          {inputPorts.map((port) => (
            <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
              {port.nodeName} · {port.portName} · {port.channels}ch
            </option>
          ))}
        </select>
      </label>
      <button type="button" className="primary" onClick={addConnection}>
        Add connection
      </button>
    </fieldset>
  );
}
