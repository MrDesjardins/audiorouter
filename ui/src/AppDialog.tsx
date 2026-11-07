import { useEffect, useId, useRef, useState, useSyncExternalStore } from "react";

/**
 * The app's own question dialogs, in place of the browser's confirm and prompt,
 * which ignore the theme. Callers ask from anywhere with `askConfirm` or
 * `askText` and await the answer; `AppDialogHost` shows one question at a time.
 */

interface ConfirmQuestion {
  kind: "confirm";
  title: string;
  message: string;
  confirmLabel: string;
  /** The confirm button removes or stops something. */
  destructive?: boolean;
}

interface TextQuestion {
  kind: "text";
  title: string;
  label: string;
  initialValue: string;
  confirmLabel: string;
  maxLength?: number;
}

type Pending = { id: number } & (
  | (ConfirmQuestion & { resolve: (answer: boolean) => void })
  | (TextQuestion & { resolve: (answer: string | null) => void })
);

let nextId = 1;
let queue: Pending[] = [];
const listeners = new Set<() => void>();
const publish = (next: Pending[]) => {
  queue = next;
  for (const listener of listeners) listener();
};
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};
const currentQuestion = () => queue[0] ?? null;

/** Resolves true when the user confirms, false when they cancel. */
export function askConfirm(question: Omit<ConfirmQuestion, "kind">): Promise<boolean> {
  return new Promise((resolve) => publish([...queue, { ...question, id: nextId++, kind: "confirm", resolve }]));
}

/** Resolves the trimmed text, or null when the user cancels. Empty text cannot be submitted. */
export function askText(question: Omit<TextQuestion, "kind">): Promise<string | null> {
  return new Promise((resolve) => publish([...queue, { ...question, id: nextId++, kind: "text", resolve }]));
}

const FOCUSABLE = "input:not([disabled]), button:not([disabled])";

function QuestionDialog({ question }: { question: Pending }) {
  const headingId = useId();
  const descriptionId = useId();
  const [text, setText] = useState(question.kind === "text" ? question.initialValue : "");
  const [refused, setRefused] = useState(false);
  const panel = useRef<HTMLDivElement>(null);
  const firstFocus = useRef<HTMLButtonElement>(null);
  const field = useRef<HTMLInputElement>(null);

  const finish = (confirmed: boolean) => {
    if (question.kind === "confirm") question.resolve(confirmed);
    else question.resolve(confirmed ? text.trim() : null);
    publish(queue.slice(1));
  };

  useEffect(() => {
    const previous = document.activeElement;
    if (field.current) {
      field.current.focus();
      field.current.select();
    } else {
      firstFocus.current?.focus();
    }
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);

  return (
    <div className="dialog-backdrop app-dialog-backdrop" role="presentation">
      <div
        ref={panel}
        className="connection-dialog app-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={headingId}
        aria-describedby={question.kind === "confirm" ? descriptionId : undefined}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            finish(false);
          }
          if (event.key !== "Tab") return;
          const items = [...(panel.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? [])];
          const first = items[0];
          const last = items[items.length - 1];
          if (!first || !last) return;
          if (event.shiftKey && document.activeElement === first) {
            event.preventDefault();
            last.focus();
          } else if (!event.shiftKey && document.activeElement === last) {
            event.preventDefault();
            first.focus();
          }
        }}
      >
        <h2 id={headingId}>{question.title}</h2>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (question.kind === "text" && !text.trim()) {
              setRefused(true);
              field.current?.focus();
              return;
            }
            finish(true);
          }}
        >
          {question.kind === "confirm" ? (
            <p id={descriptionId}>{question.message}</p>
          ) : (
            <div>
              <label>
                {question.label}
                <input
                  ref={field}
                  type="text"
                  value={text}
                  maxLength={question.maxLength}
                  aria-invalid={refused && !text.trim()}
                  onChange={(event) => {
                    setText(event.target.value);
                    setRefused(false);
                  }}
                />
              </label>
              {/* Reserved so the buttons do not move when the hint appears (UI-17). */}
              <small className="app-dialog-hint" role="status">
                {refused && !text.trim() ? "Enter a name first." : ""}
              </small>
            </div>
          )}
          <div className="actions">
            <button ref={firstFocus} type="button" className="secondary" onClick={() => finish(false)}>
              Cancel
            </button>
            <button
              type="submit"
              className={question.kind === "confirm" && question.destructive ? "danger" : "primary"}
            >
              {question.confirmLabel}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

/** Mount once inside the themed app shell. */
export function AppDialogHost() {
  const question = useSyncExternalStore(subscribe, currentQuestion);
  // The key gives each question its own field and focus restore.
  return question ? <QuestionDialog key={question.id} question={question} /> : null;
}
