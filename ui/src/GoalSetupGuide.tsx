import { useState } from "react";

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
  const [goal, setGoal] = useState("voice");
  const recording = goal === "recording";
  return (
    <details>
      <summary>Choose a setup goal</summary>
      <label className="discord-setup-field">
        <span>What do you want to do?</span>
        <select value={goal} onChange={(event) => setGoal(event.target.value)}>
          <option value="voice">Processed microphone</option>
          <option value="gaming">Gaming + Discord</option>
          <option value="recording">Record one application</option>
        </select>
      </label>
      <ol>
        <li>
          {recording
            ? "In Tools, add Application Capture and choose the exact running process in Properties. Capture includes its process tree, not a browser tab."
            : "Use Set up for Discord to choose your exact microphone and paired voice cable and add optional voice cleanup to the draft."}
        </li>
        <li>
          {recording
            ? "Add a Recorder and connect only that application to it. Leave it unarmed until you deliberately choose to record."
            : "Send only your processed microphone to the voice cable. Keep microphone monitoring muted; do not connect a headphone monitoring branch until you deliberately want it."}
        </li>
        <li>
          <strong>Device availability:</strong> check selected endpoints in Properties. An available device or running
          process does not prove audio is arriving.
        </li>
        <li>
          <strong>Local signal, not yet verified:</strong> review the draft, then deliberately press Play. Make sound in
          the selected source and check fresh route meters. Silence needs investigation before continuing.
        </li>
        <li>
          <strong>{recording ? "Recording verification" : "Receiving-app verification"}, not yet verified:</strong>{" "}
          {recording
            ? "deliberately arm and start the Recorder, stop it, then review the file for the selected app and absence of unrelated apps."
            : "open Discord or your receiving app, select the cable's recording end shown by setup, and use its input test. A closed app or correct device selection alone is not a successful audio test."}
        </li>
      </ol>
      {goal === "gaming" && (
        <p>
          Keep game and browser audio out of the voice cable. Select headphones for Discord playback and check for
          duplicate playback separately.
        </p>
      )}
      <button type="button" disabled={!connected} onClick={recording || !onSetUpDiscord ? onOpenTools : onSetUpDiscord}>
        {recording || !onSetUpDiscord ? "Open Tools for this goal" : "Choose microphone and voice cable"}
      </button>
      <p className="muted">
        This guide does not start audio, enable monitoring, arm recording, or change another app. The canvas remains
        available for advanced routes.
      </p>
    </details>
  );
}
