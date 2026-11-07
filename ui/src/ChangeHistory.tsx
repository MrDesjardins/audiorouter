import { useEffect, useEffectEvent, useRef, useState } from "react";
import type { SavedVersion, Session } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { describeChange, savedAtLabel, savedByLabel } from "./historyLabels";

import { Icon } from "./icons";
import { uiIdempotencyKey } from "./idempotency";
import { sameSessionDraft } from "./sessionInventory";

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
        if (mine === generation.current) setMessage(formatUiError(error, "Saved versions are unavailable."));
      });
  });
  useEffect(() => {
    refresh();
  }, [backend.connected, session.id, session.revision]);

  const goBack = async (version: SavedVersion) => {
    if (busy || !backend.restoreVersionPlan) return;
    if (routeChanged) {
      setMessage("Save or discard your unsaved changes first.");
      return;
    }
    setBusy(true);
    setMessage(null);
    try {
      const plan = await backend.restoreVersionPlan(session.id, session.revision, version.revision);
      await backend.commitGraph(plan.planId, plan.baseRevision, uiIdempotencyKey("graph-restore"));
      setMessage(
        `Went back to the version from ${savedAtLabel(version.savedAtMs)}. Later versions are still in this list.${
          running ? " The sound changed too." : ""
        }`,
      );
      onReverted();
    } catch (error) {
      setMessage(formatUiError(error, "Unable to go back to this version."));
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
      <h2 id="change-history-heading">Change history</h2>
      <p className="muted">
        Saved versions are kept by AudioRouter. Going back saves a new version, so nothing later is lost. Moving a card
        is not listed here; use Undo layout above the canvas.
      </p>
      <h3 className="eyebrow">Since last save</h3>
      {steps.length === 0 ? (
        <p className="muted">
          {undonePastSave ? "You undid past your last save. Save to keep this, or use Redo." : "No unsaved changes."}
        </p>
      ) : (
        <ol className="change-history-list" aria-label="Changes since last save">
          {steps.map((step, index) => (
            <li key={step.steps}>
              <Icon name="sliders-horizontal" size={15} />
              <span>{step.text}</span>
              <button
                type="button"
                className="secondary"
                aria-label={`Undo back to before: ${step.text}`}
                onClick={() => onUndoSteps(step.steps)}
              >
                {index === 0 ? "Undo this" : "Undo to here"}
              </button>
            </li>
          ))}
        </ol>
      )}
      <h3 className="eyebrow">Saved versions</h3>
      {versions.length === 0 ? (
        <p className="muted">Nothing saved yet.</p>
      ) : (
        <ol className="change-history-list" aria-label="Saved versions">
          {versions.map((version) => {
            const isCurrent = version.revision === session.revision;
            return (
              <li key={version.revision}>
                <Icon name="save" size={15} />
                <span>
                  <strong>
                    {version.name}, {version.toolCount} {version.toolCount === 1 ? "tool" : "tools"}
                  </strong>
                  <small>
                    {savedAtLabel(version.savedAtMs)} · {savedByLabel(version.savedBy)}
                  </small>
                </span>
                {isCurrent ? (
                  <small className="change-history-here">You are here</small>
                ) : (
                  <button
                    type="button"
                    className="secondary"
                    disabled={busy}
                    aria-label={`Go back to the version from ${savedAtLabel(version.savedAtMs)}`}
                    onClick={() => void goBack(version)}
                  >
                    Go back here
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
