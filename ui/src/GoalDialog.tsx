import { useEffect, useEffectEvent, useRef, useState } from "react";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";
import type { SessionFromGoalResult, SessionGoal } from "@audiorouter/contracts";
import { formatUiError, type ApplicationRow, type UiBackend } from "./backend";

import { Icon, type IconName } from "./icons";
import { uiIdempotencyKey } from "./idempotency";

const GOAL_ICONS: Record<string, IconName> = {
  "clean-mic": "mic",
  "game-discord": "gamepad-2",
  podcast: "podcast",
  empty: "square-dashed",
  "record-app": "circle-dot",
  "mix-minus": "users",
};

/** The backend sends goal text in English; known texts map to the catalog and
 * anything else (a newer backend) is shown as received. */
const GOAL_TEXT_KEYS: Record<string, MessageKey> = {
  "Clean up my mic": "h2.goalTitleCleanMic",
  "Game and voice for Discord": "h2.goalTitleGameDiscord",
  "Record a podcast": "h2.goalTitlePodcast",
  "Start empty": "h2.goalTitleEmpty",
  "Record one app": "h2.goalTitleRecordApp",
  "Talk to a guest without echo": "h2.goalTitleMixMinus",
  "A clearer voice for calls and games.": "h2.goalSumCleanMic",
  "Your voice goes to Discord, and the game stays out of it.": "h2.goalSumGameDiscord",
  "Your mic and a guest's app, saved to a file.": "h2.goalSumPodcast",
  "An empty canvas. Add tools yourself.": "h2.goalSumEmpty",
  "One app's sound saved to a file, and nothing else.": "h2.goalSumRecordApp",
  "Your guest hears you but not themselves. You hear your guest.": "h2.goalSumMixMinus",
  "Which game should play in your headphones?": "h2.goalPromptGame",
  "Which app is your guest's voice in?": "h2.goalPromptGuest",
  "Which app do you want to record?": "h2.goalPromptRecord",
  "Input device": "h2.goalAddInput",
  "Output device": "h2.goalAddOutput",
  "Application source": "h2.goalAddApp",
  "voice cleanup": "h2.goalAddCleanup",
  Mixer: "h2.goalAddMixer",
  Recorder: "h2.goalAddRecorder",
  "Speech Denoise": "h2.goalAddDenoise",
  Gate: "h2.goalAddGate",
  Compressor: "h2.goalAddCompressor",
};
type Translate = (key: MessageKey, options?: Record<string, unknown>) => string;
function goalText(tr: Translate, english: string): string {
  const key = GOAL_TEXT_KEYS[english];
  return key ? tr(key) : english;
}

/** Running apps, each executable once, in name order. */
function appChoices(rows: readonly ApplicationRow[]) {
  const seen = new Map<string, ApplicationRow>();
  for (const row of rows) if (row.executable && !seen.has(row.executable)) seen.set(row.executable, row);
  return [...seen.values()].sort((a, b) => a.executable.localeCompare(b.executable));
}

function GoalCard({ goal, onChoose, busy }: { goal: SessionGoal; onChoose: () => void; busy: boolean }) {
  const tr = useMessages();
  return (
    <section className="goal-card" aria-labelledby={`goal-${goal.id}`}>
      <h3 id={`goal-${goal.id}`}>
        <Icon name={GOAL_ICONS[goal.id] ?? "circle-dot"} size={20} />
        {goalText(tr, goal.title)}
      </h3>
      <p>{goalText(tr, goal.summary)}</p>
      <p className="goal-card-adds">
        <strong>{tr("g4Panel.goalAdds")}</strong>{" "}
        {goal.adds.length > 0 ? goal.adds.map((item) => goalText(tr, item)).join(", ") : tr("g4Panel.goalNothing")}
      </p>
      <button
        type="button"
        disabled={busy}
        onClick={onChoose}
        aria-label={tr("g4Panel.goalUseThisAria", { title: goalText(tr, goal.title) })}
      >
        {tr("g4Panel.goalUseThis")}
      </button>
    </section>
  );
}

/**
 * "What do you want to do?": makes a stopped session from a goal the backend
 * defines. Nothing plays, no device is chosen and recording stays off.
 */
export function GoalDialog({
  backend,
  onCreated,
  onClose,
}: {
  backend: UiBackend;
  onCreated: (result: SessionFromGoalResult) => void;
  onClose: () => void;
}) {
  const tr = useMessages();
  const goalsUnavailableText = useEffectEvent(() => tr("g4Panel.goalsUnavailable"));
  const [goals, setGoals] = useState<SessionGoal[] | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [asking, setAsking] = useState<SessionGoal | null>(null);
  const [apps, setApps] = useState<ApplicationRow[]>([]);
  const [app, setApp] = useState("");
  const [appFilter, setAppFilter] = useState("");
  const dialog = useRef<HTMLElement>(null);

  useEffect(() => {
    const previous = document.activeElement;
    dialog.current?.focus();
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);
  useEffect(() => {
    let live = true;
    void backend
      .listSessionGoals?.()
      .then((result) => {
        if (live) setGoals(result.goals);
      })
      .catch((error) => {
        if (live) setMessage(formatUiError(error, goalsUnavailableText()));
      });
    return () => {
      live = false;
    };
  }, [backend]);

  // Typing narrows the list by shown name or executable; the chosen app stays listed.
  const filterText = appFilter.trim().toLocaleLowerCase();
  const filteredApps = filterText
    ? apps.filter(
        (row) =>
          row.executable === app ||
          `${row.audioDisplayNames[0] ?? ""} ${row.executable}`.toLocaleLowerCase().includes(filterText),
      )
    : apps;
  const create = async (goal: SessionGoal, chosen: ApplicationRow | null) => {
    if (busy || !backend.createSessionFromGoal) return;
    setBusy(true);
    setMessage(null);
    try {
      const result = await backend.createSessionFromGoal({
        goal: goal.id,
        ...(chosen
          ? {
              application: {
                executable: chosen.executable,
                ...(chosen.executablePath ? { executablePath: chosen.executablePath } : {}),
              },
            }
          : {}),
        idempotencyKey: uiIdempotencyKey("session-goal"),
      });
      onCreated(result);
    } catch (error) {
      setMessage(formatUiError(error, tr("g4Panel.goalMakeFailed")));
      setBusy(false);
    }
  };

  const choose = (goal: SessionGoal) => {
    if (!goal.applicationPrompt) {
      void create(goal, null);
      return;
    }
    setAsking(goal);
    setApp("");
    void backend
      .listApplications()
      .then((rows) => setApps(appChoices(rows)))
      .catch(() => setApps([]));
  };

  const main = goals?.filter((goal) => goal.main) ?? [];
  const more = goals?.filter((goal) => !goal.main) ?? [];
  return (
    <div className="dialog-backdrop" role="presentation">
      <section
        ref={dialog}
        className="connection-dialog goal-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="goal-dialog-heading"
        aria-describedby="goal-dialog-description"
        tabIndex={-1}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          }
        }}
      >
        <div className="goal-dialog-head">
          <h2 id="goal-dialog-heading">{asking ? goalText(tr, asking.title) : tr("g4Panel.goalHeading")}</h2>
          <button type="button" className="secondary" onClick={onClose}>
            {tr("g4Panel.goalNotNow")}
          </button>
        </div>
        {asking ? (
          <div className="goal-app-step">
            <label className="goal-app-field">
              <span>{tr("g4Panel.goalFindApp")}</span>
              <input
                type="search"
                value={appFilter}
                placeholder={tr("g4Panel.goalTypePart")}
                autoComplete="off"
                spellCheck={false}
                onChange={(event) => setAppFilter(event.target.value)}
              />
            </label>
            <label className="goal-app-field">
              <span>{goalText(tr, asking.applicationPrompt ?? "")}</span>
              <select value={app} onChange={(event) => setApp(event.target.value)}>
                <option value="">
                  {filteredApps.length === 0 && appFilter.trim()
                    ? tr("g4Panel.goalNoMatch")
                    : tr("g4Panel.goalChooseRunning")}
                </option>
                {filteredApps.map((row) => (
                  <option key={row.executable} value={row.executable}>
                    {row.audioDisplayNames[0] || row.executable}
                  </option>
                ))}
              </select>
            </label>
            <p className="muted goal-app-note">{tr("g4Panel.goalNotRunning")}</p>
            <div className="actions">
              <button type="button" className="secondary" disabled={busy} onClick={() => setAsking(null)}>
                {tr("g4Panel.goalBack")}
              </button>
              <button type="button" className="secondary" disabled={busy} onClick={() => void create(asking, null)}>
                {tr("g4Panel.goalChooseLater")}
              </button>
              <button
                type="button"
                className="primary"
                disabled={busy || !app}
                onClick={() => void create(asking, apps.find((row) => row.executable === app) ?? null)}
              >
                {tr("g4Panel.goalMakeRoute")}
              </button>
            </div>
          </div>
        ) : (
          <>
            <p id="goal-dialog-description" className="muted">
              {tr("g4Panel.goalPick")}
            </p>
            <div className="goal-grid" aria-busy={goals === null && message === null}>
              {goals === null
                ? (message === null ? [0, 1, 2, 3] : []).map((slot) => (
                    <div key={slot} className="goal-card goal-card-placeholder" />
                  ))
                : main.map((goal) => <GoalCard key={goal.id} goal={goal} busy={busy} onChoose={() => choose(goal)} />)}
            </div>
            {more.length > 0 && (
              <div className="goal-more">
                <span className="muted">{tr("g4Panel.goalAlso")}</span>
                {more.map((goal) => (
                  <button
                    key={goal.id}
                    type="button"
                    className="secondary"
                    disabled={busy}
                    title={goal.summary}
                    onClick={() => choose(goal)}
                  >
                    {goal.title}
                  </button>
                ))}
              </div>
            )}
          </>
        )}
        <p className="goal-dialog-message" role="status">
          {message ?? ""}
        </p>
      </section>
    </div>
  );
}
