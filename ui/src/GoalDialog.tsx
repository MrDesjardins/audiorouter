import { useEffect, useRef, useState } from "react";
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

/** Running apps, each executable once, in name order. */
function appChoices(rows: readonly ApplicationRow[]) {
  const seen = new Map<string, ApplicationRow>();
  for (const row of rows) if (row.executable && !seen.has(row.executable)) seen.set(row.executable, row);
  return [...seen.values()].sort((a, b) => a.executable.localeCompare(b.executable));
}

function GoalCard({ goal, onChoose, busy }: { goal: SessionGoal; onChoose: () => void; busy: boolean }) {
  return (
    <section className="goal-card" aria-labelledby={`goal-${goal.id}`}>
      <h3 id={`goal-${goal.id}`}>
        <Icon name={GOAL_ICONS[goal.id] ?? "circle-dot"} size={20} />
        {goal.title}
      </h3>
      <p>{goal.summary}</p>
      <p className="goal-card-adds">
        <strong>Adds:</strong> {goal.adds.length > 0 ? goal.adds.join(", ") : "Nothing"}
      </p>
      <button type="button" disabled={busy} onClick={onChoose} aria-label={`Use this: ${goal.title}`}>
        Use this
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
  const [goals, setGoals] = useState<SessionGoal[] | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [asking, setAsking] = useState<SessionGoal | null>(null);
  const [apps, setApps] = useState<ApplicationRow[]>([]);
  const [app, setApp] = useState("");
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
        if (live) setMessage(formatUiError(error, "The starting routes are unavailable."));
      });
    return () => {
      live = false;
    };
  }, [backend]);

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
      setMessage(formatUiError(error, "Unable to make this route."));
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
          <h2 id="goal-dialog-heading">{asking ? asking.title : "What do you want to do?"}</h2>
          <button type="button" className="secondary" onClick={onClose}>
            Not now
          </button>
        </div>
        {asking ? (
          <div className="goal-app-step">
            <label className="goal-app-field">
              <span>{asking.applicationPrompt}</span>
              <select value={app} onChange={(event) => setApp(event.target.value)}>
                <option value="">Choose a running app</option>
                {apps.map((row) => (
                  <option key={row.executable} value={row.executable}>
                    {row.audioDisplayNames[0] || row.executable}
                  </option>
                ))}
              </select>
            </label>
            <p className="muted">
              Not running yet? Choose later: the route is made without it, and you add it from Tools.
            </p>
            <div className="actions">
              <button type="button" className="secondary" disabled={busy} onClick={() => setAsking(null)}>
                Back
              </button>
              <button type="button" className="secondary" disabled={busy} onClick={() => void create(asking, null)}>
                Choose later
              </button>
              <button
                type="button"
                className="primary"
                disabled={busy || !app}
                onClick={() => void create(asking, apps.find((row) => row.executable === app) ?? null)}
              >
                Make this route
              </button>
            </div>
          </div>
        ) : (
          <>
            <p id="goal-dialog-description" className="muted">
              Pick one to get a starting route you can change. Nothing plays until you press Play. Your mic stays out of
              your headphones, and recording stays off.
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
                <span className="muted">Also:</span>
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
