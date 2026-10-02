import { useEffect, useRef, useState } from "react";

export function NodeIdentity({ nodeId }: { nodeId: string }) {
  return <IdentityFooter key={nodeId} id={nodeId} kind="node" />;
}

export function SessionIdentity({ sessionId }: { sessionId: string }) {
  return <IdentityFooter key={sessionId} id={sessionId} kind="session" />;
}

function IdentityFooter({ id, kind }: { id: string; kind: "node" | "session" }) {
  const caption = kind === "node" ? "Node ID" : "Session ID";
  const [message, setMessage] = useState("");
  const [copying, setCopying] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; if (timer.current) clearTimeout(timer.current); }; }, []);
  const copy = async () => {
    if (copying) return;
    if (timer.current) clearTimeout(timer.current);
    setCopying(true);
    try {
      await navigator.clipboard.writeText(id);
      if (!mounted.current) return;
      setMessage("Copied.");
      timer.current = setTimeout(() => setMessage(""), 2000);
    } catch {
      if (!mounted.current) return;
      setMessage("Could not copy. Select the ID and copy it manually.");
    } finally { if (mounted.current) setCopying(false); }
  };
  return <footer className="node-identity">
    <span className="node-identity-caption">{caption}</span>
    <div className="node-identity-value">
      <code aria-label={caption} tabIndex={0}>{id}</code>
      <button type="button" className="secondary node-identity-copy" aria-label={`Copy ${kind} ID`} title={`Copy ${kind} ID for API integrations`} disabled={copying} onClick={() => void copy()}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true" focusable="false"><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3" /></svg>
      </button>
    </div>
    <p className="node-identity-message" role="status">{message}</p>
  </footer>;
}
