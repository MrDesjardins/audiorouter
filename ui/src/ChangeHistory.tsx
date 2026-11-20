import { useEffect, useEffectEvent, useRef, useState } from "react";
import type { SavedVersion, Session } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { describeChange, savedAtLabel, savedByLabel } from "./historyLabels";

import { Icon } from "./icons";
import { uiIdempotencyKey } from "./idempotency";
import { sameSessionDraft } from "./sessionInventory";
import { useMessages } from "./i18n/useMessages";

/**
 * Session › Change history. Unsaved changes come from the route's undo list.
 * Saved versions come from the backend; going back is a new save, so later
 * versions stay in the list. Card positions are not route changes, so they
 * are undone with the canvas's own Undo layout.
 */
export function ChangeHistory({
  backend,
  session,
  past,
  current,
  routeChanged,
  running,
  onUndoSteps,
  onReverted,
}: {
  backend: UiBackend;
  session: Session;
  past: readonly Session[];
  current: Session;
  routeChanged: boolean;
  running: boolean;
  onUndoSteps: (steps: number) => void;
  onReverted: () => void;
}) {
  const tr = useMessages();
  const [versions, setVersions] = useState<SavedVersion[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const refresh = useEffectEvent(() => {
    const mine = ++generation.current;
    void backend
      .listSavedVersions?.(session.id)
      .then((result) => {
        if (mine === generation.current) setVersions(result.items);
      })
      .catch((error) => {
        if (mine === generation.current) setMessage(formatUiError(error, tr("historyUi.versionsError")));
      });
  });
  useEffect(() => {
    refresh();
  }, [backend.connected, session.id, session.revision]);

  const goBack = async (version: SavedVersion) => {
    if (busy || !backend.restoreVersionPlan) return;
    if (routeChanged) {
      setMessage(tr("historyUi.saveFirst"));
      return;
    }
    setBusy(true);
    setMessage(null);
    try {
      const plan = await backend.restoreVersionPlan(session.id, session.revision, version.revision);
      await backend.commitGraph(plan.planId, plan.baseRevision, uiIdempotencyKey("graph-restore"));
      setMessage(
        tr(running ? "historyUi.restoredLive" : "historyUi.restored", { date: savedAtLabel(version.savedAtMs) }),
      );
      onReverted();
    } catch (error) {
      setMessage(formatUiError(error, tr("historyUi.restoreError")));
    } finally {
      setBusy(false);
    }
  };

  // Newest first: each entry describes the step from the state before it.
  // The undo list outlives a save, so only what came after the saved state counts.
  const savedAt = routeChanged
    ? past.map((before) => sameSessionDraft(before, session)).lastIndexOf(true)
    : past.length;
  const undonePastSave = routeChanged && savedAt < 0;
  const steps = past
    .map((before, index) => ({
      steps: past.length - index,
      text: describeChange(before, past[index + 1] ?? current),
    }))
    .slice(Math.max(savedAt, 0));
  if (undonePastSave) steps.length = 0;
  steps.reverse();
  return (
    <section className="panel change-history" aria-labelledby="change-history-heading">
      <h2 id="change-history-heading">{tr("historyUi.history")}</h2>
      <p className="muted">{tr("historyUi.historyIntro")}</p>
      <h3 className="eyebrow">{tr("historyUi.since")}</h3>
      {steps.length === 0 ? (
        <p className="muted">{tr(undonePastSave ? "historyUi.pastSave" : "historyUi.noChanges")}</p>
      ) : (
        <ol className="change-history-list" aria-label={tr("historyUi.sinceLabel")}>
          {steps.map((step, index) => (
            <li key={step.steps}>
              <Icon name="sliders-horizontal" size={15} />
              <span>{step.text}</span>
              <button
                type="button"
                className="secondary"
                aria-label={tr("historyUi.undoBefore", { step: step.text })}
                onClick={() => onUndoSteps(step.steps)}
              >
                {tr(index === 0 ? "historyUi.undoThis" : "historyUi.undoHere")}
              </button>
            </li>
          ))}
        </ol>
      )}
      <h3 className="eyebrow">{tr("historyUi.savedVersions")}</h3>
      {versions.length === 0 ? (
        <p className="muted">{tr("historyUi.noneSaved")}</p>
      ) : (
        <ol className="change-history-list" aria-label={tr("historyUi.savedVersions")}>
          {versions.map((version) => {
            const isCurrent = version.revision === session.revision;
            return (
              <li key={version.revision}>
                <Icon name="save" size={15} />
                <span>
                  <strong>{tr("historyUi.toolCount", { name: version.name, count: version.toolCount })}</strong>
                  <small>
                    {savedAtLabel(version.savedAtMs)} · {savedByLabel(version.savedBy)}
                  </small>
                </span>
                {isCurrent ? (
                  <small className="change-history-here">{tr("historyUi.here")}</small>
                ) : (
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    aria-label={tr("historyUi.goBackTo", { date: savedAtLabel(version.savedAtMs) })}
                    onClick={() => void goBack(version)}
                  >
                    {tr("historyUi.goBack")}
                  </button>
                )}
              </li>
            );
          })}
        </ol>
      )}
      <p className="change-history-message" role="status">
        {message ?? ""}
      </p>
    </section>
  );
}
