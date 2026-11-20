import type { ApplicationRow } from "./backend";
import { useMessages } from "./i18n/useMessages";

export function ApplicationIdentityPanel({ applications }: { applications: ApplicationRow[] }) {
  const tr = useMessages();
  const identified = applications.filter((application) => application.executablePath !== null);

  return (
    <section className="panel application-identity-panel" aria-labelledby="application-identity-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("g1.identEyebrow")}</p>
          <h2 id="application-identity-heading">{tr("g1.identHeading")}</h2>
        </div>
        <span className="badge">{identified.length}</span>
      </div>
      {identified.length === 0 ? (
        <p className="muted">{tr("g1.identNone")}</p>
      ) : (
        <ul aria-label={tr("g1.identListLabel")}>
          {identified.map((application) => (
            <li key={`${application.processId}-${application.creationTime100ns ?? "unknown"}`}>
              <strong>{application.executable}</strong>
              <br />
              <small>
                PID {application.processId} · {application.executablePath}
              </small>
            </li>
          ))}
        </ul>
      )}
      <p className="muted">{tr("g1.identNote")}</p>
    </section>
  );
}
