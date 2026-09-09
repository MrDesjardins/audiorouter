import type { ApplicationRow } from "./backend";

export function ApplicationIdentityPanel({ applications }: { applications: ApplicationRow[] }) {
  const identified = applications.filter((application) => application.executablePath !== null);

  return (
    <section className="panel application-identity-panel" aria-labelledby="application-identity-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Binding identity</p>
          <h2 id="application-identity-heading">Verified executable paths</h2>
        </div>
        <span className="badge">{identified.length}</span>
      </div>
      {identified.length === 0 ? (
        <p className="muted">No verified full executable paths are available in the current backend snapshot.</p>
      ) : (
        <ul aria-label="Verified application executable paths">
          {identified.map((application) => (
            <li key={`${application.processId}-${application.creationTime100ns ?? "unknown"}`}>
              <strong>{application.executable}</strong>
              <br />
              <small>PID {application.processId} · {application.executablePath}</small>
            </li>
          ))}
        </ul>
      )}
      <p className="muted">Identity is read-only discovery data; binding still requires verified process identity and backend authorization.</p>
    </section>
  );
}
