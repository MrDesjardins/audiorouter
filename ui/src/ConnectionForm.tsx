// Keyboard connection form of the Advanced tab (moved from App.tsx).
import type { Dispatch, SetStateAction } from "react";
import type { UiBackend } from "./backend";
import { useMessages } from "./i18n/useMessages";

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
  const tr = useMessages();
  return (
    <fieldset className="connection-editor workbench-connection-editor" disabled={!backend.connected}>
      <legend>{tr("g2.connAdd")}</legend>
      <p className="muted">{tr("g2.connHelp")}</p>
      <label>
        {tr("g2.connOutput")}
        <select
          aria-label={tr("g2.connSourceAria")}
          value={connectionSource}
          onChange={(event) => setConnectionSource(event.target.value)}
        >
          <option value="">{tr("g2.connChooseSource")}</option>
          {outputPorts.map((port) => (
            <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
              {port.nodeName} · {port.portName} · {tr("g2.connChannels", { n: port.channels })}
            </option>
          ))}
        </select>
      </label>
      <label>
        {tr("g2.connInput")}
        <select
          aria-label={tr("g2.connDestAria")}
          value={connectionDestination}
          onChange={(event) => setConnectionDestination(event.target.value)}
        >
          <option value="">{tr("g2.connChooseDest")}</option>
          {inputPorts.map((port) => (
            <option key={encodePort(port.nodeId, port.portName)} value={encodePort(port.nodeId, port.portName)}>
              {port.nodeName} · {port.portName} · {tr("g2.connChannels", { n: port.channels })}
            </option>
          ))}
        </select>
      </label>
      <button type="button" className="primary" onClick={addConnection}>
        {tr("g2.connAddButton")}
      </button>
    </fieldset>
  );
}
