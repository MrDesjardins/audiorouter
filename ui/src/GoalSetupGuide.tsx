import { useState } from "react";
import { useMessages } from "./i18n/useMessages";

/** Guidance only: all graph edits and playback remain explicit existing actions. */
export function GoalSetupGuide({
  connected,
  onOpenTools,
  onSetUpDiscord,
}: {
  connected: boolean;
  onOpenTools: () => void;
  onSetUpDiscord?: () => void;
}) {
  const tr = useMessages();
  const [goal, setGoal] = useState("voice");
  const recording = goal === "recording";
  return (
    <details>
      <summary>{tr("goalGuide.summary")}</summary>
      <label className="discord-setup-field">
        <span>{tr("goalGuide.question")}</span>
        <select value={goal} onChange={(event) => setGoal(event.target.value)}>
          <option value="voice">{tr("goalGuide.voice")}</option>
          <option value="gaming">{tr("goalGuide.gaming")}</option>
          <option value="recording">{tr("goalGuide.recording")}</option>
        </select>
      </label>
      <ol>
        <li>{recording ? tr("goalGuide.step1Recording") : tr("goalGuide.step1Voice")}</li>
        <li>{recording ? tr("goalGuide.step2Recording") : tr("goalGuide.step2Voice")}</li>
        <li>
          <strong>{tr("goalGuide.deviceLabel")}</strong> {tr("goalGuide.deviceHelp")}
        </li>
        <li>
          <strong>{tr("goalGuide.signalLabel")}</strong> {tr("goalGuide.signalHelp")}
        </li>
        <li>
          <strong>{recording ? tr("goalGuide.recordingVerifyLabel") : tr("goalGuide.receivingVerifyLabel")}</strong>{" "}
          {recording ? tr("goalGuide.recordingVerifyHelp") : tr("goalGuide.receivingVerifyHelp")}
        </li>
      </ol>
      {goal === "gaming" && <p>{tr("goalGuide.gamingNote")}</p>}
      <button type="button" disabled={!connected} onClick={recording || !onSetUpDiscord ? onOpenTools : onSetUpDiscord}>
        {recording || !onSetUpDiscord ? tr("goalGuide.openTools") : tr("goalGuide.chooseMicrophone")}
      </button>
      <p className="muted">{tr("goalGuide.footnote")}</p>
    </details>
  );
}
