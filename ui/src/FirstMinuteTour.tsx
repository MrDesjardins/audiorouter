import { useEffect, useEffectEvent, useLayoutEffect, useRef, useState } from "react";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

const STEPS = [
  {
    target: ".right-workbench",
    title: "g1.tourStep1Title",
    text: "g1.tourStep1Text",
  },
  {
    target: ".session-flow-canvas",
    title: "g1.tourStep2Title",
    text: "g1.tourStep2Text",
  },
  {
    target: "[data-tour='play']",
    title: "g1.tourStep3Title",
    text: "g1.tourStep3Text",
  },
] as const satisfies readonly { target: string; title: MessageKey; text: MessageKey }[];

export const TOUR_STEP_COUNT = STEPS.length;
const COACH_WIDTH = 310;
const GAP = 14;

/** Three short steps that point at the real controls. It presses nothing and never starts audio. */
export function FirstMinuteTour({ onOpenTools, onClose }: { onOpenTools: () => void; onClose: () => void }) {
  const tr = useMessages();
  const [step, setStep] = useState(0);
  const [box, setBox] = useState<DOMRect | null>(null);
  const next = useRef<HTMLButtonElement>(null);
  const openTools = useEffectEvent(onOpenTools);
  const current = STEPS[step];

  useEffect(() => {
    if (step === 0) openTools();
  }, [step]);
  useLayoutEffect(() => {
    const measure = () => {
      const target = document.querySelector(current.target);
      setBox(target ? target.getBoundingClientRect() : null);
    };
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [current.target]);
  useEffect(() => {
    next.current?.focus();
  }, [step]);

  const coach = (() => {
    if (!box) return { left: 16, top: 16 };
    const left = box.left >= COACH_WIDTH + GAP + 16 ? box.left - COACH_WIDTH - GAP : box.right + GAP;
    return {
      left: Math.max(16, Math.min(left, window.innerWidth - COACH_WIDTH - 16)),
      top: Math.max(16, Math.min(box.top + 24, window.innerHeight - 220)),
    };
  })();
  const overTarget =
    box && coach.left < box.right && coach.left + COACH_WIDTH > box.left
      ? { top: Math.min(box.bottom + GAP, window.innerHeight - 220) }
      : {};
  const last = step === STEPS.length - 1;
  return (
    <div className="tour-layer">
      {box && (
        <div
          className="tour-ring"
          style={{ left: box.left - 3, top: box.top - 3, width: box.width + 6, height: box.height + 6 }}
        />
      )}
      <section
        className="tour-coach"
        style={{ ...coach, ...overTarget, width: COACH_WIDTH }}
        role="dialog"
        aria-modal="true"
        aria-labelledby="tour-heading"
        aria-describedby="tour-text"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          }
        }}
      >
        <small>{tr("g1.tourCount", { step: step + 1, total: STEPS.length })}</small>
        <h2 id="tour-heading">{tr(current.title)}</h2>
        <p id="tour-text">{tr(current.text)}</p>
        <div className="tour-actions">
          <button type="button" className="secondary" onClick={onClose}>
            {tr("g1.tourSkip")}
          </button>
          <button ref={next} type="button" onClick={() => (last ? onClose() : setStep(step + 1))}>
            {last ? tr("g1.tourDone") : tr("g1.tourNext")}
          </button>
        </div>
      </section>
    </div>
  );
}
