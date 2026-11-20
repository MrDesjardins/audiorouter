import { Component, useState, type ErrorInfo, type ReactNode } from "react";
import { browserDiagnosticStorage } from "./clientDiagnostics";
import { tx } from "./i18n";
import { readTheme } from "./preferences";

/** Keeps an error's name safe to show and to store as a diagnostic. */
export function safeErrorName(error: unknown): string {
  const raw = error instanceof Error ? error.name : typeof error;
  return /^[A-Za-z][A-Za-z0-9_.-]{0,47}$/.test(raw) ? raw : "Error";
}

/** The diagnostic row an area's render failure leaves in the Logs tab. */
export function renderErrorDiagnostic(area: string, error: unknown): string {
  return `UI render error in ${area} (${safeErrorName(error)})`;
}

type BoundaryProps = {
  /** Short user-facing name of the guarded part, for example "Signal flow". */
  area: string;
  children: ReactNode;
  onError?: (diagnostic: string) => void;
  /** Replaces the default recovery panel (the root uses its own). */
  fallback?: (reset: () => void) => ReactNode;
};

type BoundaryState = { failed: boolean };

/**
 * Contains a render failure to one part of the window. Audio keeps running in
 * the backend; the rest of the window, including the toolbar's Stop and
 * privacy mute, stays usable.
 */
export class ErrorBoundary extends Component<BoundaryProps, BoundaryState> {
  state: BoundaryState = { failed: false };

  static getDerivedStateFromError(): BoundaryState {
    return { failed: true };
  }

  componentDidCatch(error: unknown, _info: ErrorInfo): void {
    // React does not rethrow a caught render error. Report it like an
    // uncaught one so browser tests and the window error log still see it.
    if (typeof globalThis.reportError === "function") globalThis.reportError(error);
    try {
      this.props.onError?.(renderErrorDiagnostic(this.props.area, error));
    } catch {
      // A failing diagnostic sink must not hide the recovery panel.
    }
  }

  private reset = () => this.setState({ failed: false });

  render(): ReactNode {
    if (!this.state.failed) return this.props.children;
    if (this.props.fallback) return this.props.fallback(this.reset);
    return (
      <section
        className="panel error-boundary-panel"
        role="alert"
        aria-label={tx("g1.boundaryStoppedLabel", { area: this.props.area })}
      >
        <p className="panel-message is-error">{tx("g1.boundaryStopped", { area: this.props.area })}</p>
        <p className="muted">{tx("g1.boundaryCause")}</p>
        <div className="error-boundary-actions">
          <button type="button" className="secondary" onClick={this.reset}>
            {tx("g1.boundaryTryAgain")}
          </button>
          <button type="button" className="secondary" onClick={reloadWindow}>
            {tx("g1.boundaryReload")}
          </button>
        </div>
      </section>
    );
  }
}

function reloadWindow() {
  window.location.reload();
}

/**
 * Shown when the whole window failed to render. It still offers privacy mute,
 * since the normal toolbar is gone, and a reload.
 */
export function RootRecoveryPanel({ onPrivacyMute }: { onPrivacyMute?: () => Promise<unknown> }) {
  const [muteState, setMuteState] = useState<"idle" | "busy" | "done" | "failed">("idle");
  const mute = async () => {
    if (!onPrivacyMute) return;
    setMuteState("busy");
    try {
      await onPrivacyMute();
      setMuteState("done");
    } catch {
      setMuteState("failed");
    }
  };
  const status =
    muteState === "done" ? tx("g1.rootMuteOn") : muteState === "failed" ? tx("g1.rootMuteFailed") : "\u00a0";
  return (
    <main
      className={`app-shell theme-${readTheme(browserDiagnosticStorage())} root-recovery`}
      role="alert"
      aria-label={tx("g1.rootStoppedLabel")}
    >
      <section className="panel error-boundary-panel">
        <h1>{tx("g1.rootHeading")}</h1>
        <p>{tx("g1.rootBody")}</p>
        <div className="error-boundary-actions">
          <button type="button" onClick={reloadWindow}>
            {tx("g1.boundaryReload")}
          </button>
          {onPrivacyMute && (
            <button
              type="button"
              className="secondary"
              disabled={muteState === "busy" || muteState === "done"}
              onClick={() => void mute()}
            >
              {tx("g1.rootMuteButton")}
            </button>
          )}
        </div>
        <p className="muted error-boundary-status" role="status">
          {status}
        </p>
      </section>
    </main>
  );
}
