import type { ReactNode } from "react";

/** One help affordance across workbench tabs. Opens only on user action. */
export function HelpDisclosure({ title, children }: { title: string; children: ReactNode }) {
  return (
    <details className="help-disclosure">
      <summary>
        <span className="help-icon" aria-hidden="true">
          ?
        </span>
        <span>{title}</span>
      </summary>
      <div className="help-content">{children}</div>
    </details>
  );
}
