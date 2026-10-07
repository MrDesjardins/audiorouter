import { Component, useState, type ErrorInfo, type ReactNode } from "react";
import { browserDiagnosticStorage } from "./clientDiagnostics";
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
      <section className="panel error-boundary-panel" role="alert" aria-label={`${this.props.area} stopped`}>
        <p className="panel-message is-error">
          {this.props.area} stopped working. Audio keeps running, and the rest of the window still works.
        </p>
        <p className="muted">
          The cause is listed in the Logs tab. Try again, or reload the window if it keeps failing.
        </p>
        <div className="error-boundary-actions">
          <button type="button" className="secondary" onClick={this.reset}>
            Try again
          </button>
          <button type="button" className="secondary" onClick={reloadWindow}>
            Reload window
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
    muteState === "done"
      ? "Privacy mute is on."
      : muteState === "failed"
        ? "Could not turn on privacy mute. Use the tray icon instead."
        : "\u00a0";
  return (
    <main
      className={`app-shell theme-${readTheme(browserDiagnosticStorage())} root-recovery`}
      role="alert"
      aria-label="AudioRouter stopped"
    >
      <section className="panel error-boundary-panel">
        <h1>AudioRouter's window stopped working</h1>
        <p>Audio keeps running in the background. Reload the window to continue; your saved sessions are kept.</p>
        <div className="error-boundary-actions">
          <button type="button" onClick={reloadWindow}>
            Reload window
          </button>
          {onPrivacyMute && (
            <button
              type="button"
              className="secondary"
              disabled={muteState === "busy" || muteState === "done"}
              onClick={() => void mute()}
            >
              Turn on privacy mute
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
