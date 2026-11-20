import { useState } from "react";
import { AppLogo } from "./appLogos";
import type { DeviceListItem } from "@audiorouter/contracts";
import { CopyName } from "./AppsPanel";
import type { DiscordSetupOptions } from "./discordRoute";
import { boldMarkup } from "./boldMarkup";
import { useMessages } from "./i18n/useMessages";

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
  const tr = useMessages();
  const [microphone, setMicrophone] = useState(options.microphone ?? "");
  const [cable, setCable] = useState(options.cable ?? "");
  const [cleanup, setCleanup] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const chosenCable = options.cables.find((choice) => choice.input.id === cable) ?? null;
  const chosenMicrophone = options.microphones.find((device) => device.id === microphone) ?? null;

  return (
    <section className="panel app-card discord-setup" aria-labelledby="discord-setup-heading">
      <header className="app-card-heading">
        <AppLogo app="discord" />
        <h3 id="discord-setup-heading">{tr("g2.dsTitle")}</h3>
        <span className="discord-setup-step">{tr("g2.dsStep", { n: added ? 2 : 1 })}</span>
      </header>
      {added ? (
        <>
          <p className="muted">{tr("g2.dsAdded")}</p>
          <ol className="discord-setup-list">
            <li>{boldMarkup(tr("g2.dsSavePlay"))}</li>
            <li>
              {boldMarkup(tr("g2.dsInDiscord", { name: added.name }))}
              <CopyName name={added.name} />
            </li>
            <li>{tr("g2.dsLocalUnverified")}</li>
            <li>{tr("g2.dsRemoteUnverified")}</li>
          </ol>
          <div className="obs-actions">
            <button type="button" className="secondary" onClick={onClose}>
              {tr("g2.dsDone")}
            </button>
          </div>
        </>
      ) : options.cables.length === 0 ? (
        <>
          <p className="muted">{tr("g2.dsNeedsCable")}</p>
          <div className="obs-actions">
            <button type="button" className="primary" onClick={onOpenVirtualCables}>
              {tr("g2.dsSetupCable")}
            </button>
            <button type="button" className="secondary" onClick={onClose}>
              {tr("g2.dsCancel")}
            </button>
          </div>
        </>
      ) : (
        <>
          <p className="muted">{tr("g2.dsIntro")}</p>
          <label className="discord-setup-field">
            <span>{tr("g2.dsMicrophone")}</span>
            <select
              value={microphone}
              onChange={(event) => {
                setMicrophone(event.target.value);
                setError(null);
              }}
            >
              {microphone === "" && <option value="">{tr("g2.dsChooseMic")}</option>}
              {options.microphones.map((device) => (
                <option key={device.id} value={device.id}>
                  {device.name}
                </option>
              ))}
            </select>
          </label>
          <p className="muted">
            {chosenCable
              ? tr("g2.dsCableHint", { input: chosenCable.input.name, output: chosenCable.output.name })
              : tr("g2.dsChooseCableFirst")}
          </p>
          <label className="discord-setup-field">
            <span>{tr("g2.dsVoiceCable")}</span>
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
            <span>{tr("g2.dsCleanup")}</span>
          </label>
          {/* Reserved: the summary and an error share fixed lines (UI-17). */}
          <p className={error ? "discord-setup-review is-error" : "discord-setup-review"} role="status">
            {error ??
              (chosenMicrophone && chosenCable
                ? tr("g2.dsReview", {
                    mic: chosenMicrophone.name,
                    chain: cleanup ? tr("g2.dsChain") : "",
                    cable: cableShort(chosenCable.input.name),
                    output: cableShort(chosenCable.output.name),
                  })
                : tr("g2.dsChooseBoth"))}
          </p>
          <div className="obs-actions">
            <button
              type="button"
              className="primary"
              disabled={!connected || !chosenMicrophone || !chosenCable}
              onClick={() => setError(onAdd({ microphone, cable, cleanup }))}
            >
              {tr("g2.dsAddRoute")}
            </button>
            <button type="button" className="secondary" onClick={onClose}>
              {tr("g2.dsCancel")}
            </button>
          </div>
        </>
      )}
    </section>
  );
}
