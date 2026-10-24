import { useState } from "react";
import type { DeviceListItem } from "@audiorouter/contracts";
import { CopyName } from "./AppsPanel";
import type { DiscordSetupOptions } from "./discordRoute";

/** "CABLE-A Input (VB-Audio Virtual Cable A)" as "CABLE-A Input", for the one-line review. */
const cableShort = (name: string) => name.replace(/\s*\(VB-Audio[^)]*\)\s*$/i, "");

export interface DiscordSetupChoice {
  microphone: string;
  cable: string;
  cleanup: boolean;
}

/**
 * The guided "Set up for Discord" flow (D2), shown in the Apps tab above the
 * Discord card. Step 1 picks the devices and adds one route to the draft;
 * step 2 says what to choose in Discord, which the card below then checks.
 */
export function DiscordSetup({
  options,
  connected,
  added,
  onAdd,
  onClose,
  onOpenVirtualCables,
}: {
  options: DiscordSetupOptions;
  connected: boolean;
  /** Discord's microphone once the route is added; null before. */
  added: DeviceListItem | null;
  /** Adds the route to the draft; returns an error message, or null. */
  onAdd: (choice: DiscordSetupChoice) => string | null;
  onClose: () => void;
  onOpenVirtualCables: () => void;
}) {
  const [microphone, setMicrophone] = useState(options.microphone ?? "");
  const [cable, setCable] = useState(options.cable ?? "");
  const [cleanup, setCleanup] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const chosenCable = options.cables.find((choice) => choice.input.id === cable) ?? null;
  const chosenMicrophone = options.microphones.find((device) => device.id === microphone) ?? null;

  return (
    <section className="panel app-card discord-setup" aria-labelledby="discord-setup-heading">
      <header className="app-card-heading">
        <span className="app-monogram" aria-hidden="true">
          D
        </span>
        <h3 id="discord-setup-heading">Set up for Discord</h3>
        <span className="discord-setup-step">Step {added ? 2 : 1} of 2</span>
      </header>
      {added ? (
        <>
          <p className="muted">The route is in your draft. Undo removes it in one step.</p>
          <ol className="discord-setup-list">
            <li>
              <strong>Save</strong>, then press <strong>Play</strong>.
            </li>
            <li>
              In Discord, open User Settings › Voice &amp; Video and set Input Device to <strong>{added.name}</strong>.
              <CopyName name={added.name} />
            </li>
            <li>Join a voice channel. The Discord card below turns Ready when Discord hears your processed voice.</li>
          </ol>
          <div className="obs-actions">
            <button type="button" className="secondary" onClick={onClose}>
              Done
            </button>
          </div>
        </>
      ) : options.cables.length === 0 ? (
        <>
          <p className="muted">
            Discord needs a virtual cable: AudioRouter plays your voice into one end, and Discord records the other. No
            VB-Cable is installed yet.
          </p>
          <div className="obs-actions">
            <button type="button" className="primary" onClick={onOpenVirtualCables}>
              Set up a virtual cable
            </button>
            <button type="button" className="secondary" onClick={onClose}>
              Cancel
            </button>
          </div>
        </>
      ) : (
        <>
          <p className="muted">
            AudioRouter sends your microphone through a virtual cable, and Discord records the cable instead of the
            microphone.
          </p>
          <label className="discord-setup-field">
            <span>Microphone</span>
            <select
              value={microphone}
              onChange={(event) => {
                setMicrophone(event.target.value);
                setError(null);
              }}
            >
              {microphone === "" && <option value="">Choose a microphone</option>}
              {options.microphones.map((device) => (
                <option key={device.id} value={device.id}>
                  {device.name}
                </option>
              ))}
            </select>
          </label>
          <label className="discord-setup-field">
            <span>Voice cable</span>
            <select
              value={cable}
              onChange={(event) => {
                setCable(event.target.value);
                setError(null);
              }}
            >
              {options.cables.map((choice) => (
                <option key={choice.input.id} value={choice.input.id}>
                  {choice.input.name}
                  {choice.busy ? ` (${choice.busy})` : ""}
                </option>
              ))}
            </select>
          </label>
          <label className="discord-setup-check">
            <input type="checkbox" checked={cleanup} onChange={(event) => setCleanup(event.target.checked)} />
            <span>Clean up my voice: Gate, Compressor and Limiter, adjustable later</span>
          </label>
          {/* Reserved: the summary and an error share fixed lines (UI-17). */}
          <p className={error ? "discord-setup-review is-error" : "discord-setup-review"} role="status">
            {error ??
              (chosenMicrophone && chosenCable
                ? `Adds ${chosenMicrophone.name} → ${cleanup ? "Gate → Compressor → Limiter → " : ""}${cableShort(chosenCable.input.name)}. Discord's microphone: ${cableShort(chosenCable.output.name)}.`
                : "Choose a microphone and a cable.")}
          </p>
          <div className="obs-actions">
            <button
              type="button"
              className="primary"
              disabled={!connected || !chosenMicrophone || !chosenCable}
              onClick={() => setError(onAdd({ microphone, cable, cleanup }))}
            >
              Add route
            </button>
            <button type="button" className="secondary" onClick={onClose}>
              Cancel
            </button>
          </div>
        </>
      )}
    </section>
  );
}
