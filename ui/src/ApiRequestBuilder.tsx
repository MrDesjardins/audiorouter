import { useEffect, useMemo, useState } from "react";
import type { DiscoveryDocument, MethodParams, MethodResult, Session } from "@audiorouter/contracts";
import {
  buildRequest,
  builderNodes,
  builderTargets,
  exampleIdempotencyKey,
  parseValue,
  type BuilderTarget,
} from "./requestBuilder";
import { formatUiError } from "./backend";

type Format = "json" | "curl" | "powershell";

/**
 * API tab request builder: pick a session, a tool and one setting, then copy
 * the exact REST request or send it once. Choosing never changes anything;
 * only Send does, through the same backend method the request calls.
 */
export function RequestBuilder({
  sessions,
  activeSessionId,
  nodeTypes,
  baseUrl,
  connected,
  onSend,
}: {
  sessions: Session[];
  activeSessionId: string;
  nodeTypes: DiscoveryDocument["nodeTypes"] | null;
  baseUrl: string;
  connected: boolean;
  onSend?: (params: MethodParams["nodes.set"]) => Promise<MethodResult["nodes.set"]>;
}) {
  const [follow, setFollow] = useState(true);
  const [pinnedId, setPinnedId] = useState(activeSessionId);
  const session = sessions.find((item) => item.id === (follow ? activeSessionId : pinnedId)) ?? sessions[0] ?? null;
  const nodes = useMemo(() => (session ? builderNodes(session) : []), [session]);
  const [nodeId, setNodeId] = useState("");
  const node = session?.nodes.find((item) => item.id === nodeId) ?? null;
  const targets = useMemo(
    () => (session && node ? builderTargets(session, node, nodeTypes) : []),
    [session, node, nodeTypes],
  );
  const [targetKey, setTargetKey] = useState("");
  const target: BuilderTarget | null = targets.find((item) => item.key === targetKey) ?? null;
  const [valueText, setValueText] = useState("");
  const [format, setFormat] = useState<Format>("json");
  const [key, setKey] = useState(() => exampleIdempotencyKey());
  const [status, setStatus] = useState<string | null>(null);
  const [sending, setSending] = useState(false);

  // Keep selections valid when the session or tool changes.
  useEffect(() => {
    if (!nodes.some((item) => item.id === nodeId)) setNodeId(nodes[0]?.id ?? "");
  }, [nodes, nodeId]);
  useEffect(() => {
    if (!targets.some((item) => item.key === targetKey)) setTargetKey(targets[0]?.key ?? "");
  }, [targets, targetKey]);
  useEffect(() => {
    if (!target) return;
    const current = target.kind === "flag" ? node?.[target.key] : (node?.parameters[target.key] ?? target.spec.default);
    setValueText(
      current === undefined || current === null
        ? target.kind === "parameter" && target.choices
          ? (target.choices[0]?.value ?? "")
          : ""
        : String(current),
    );
    setStatus(null);
  }, [target, node]);

  const parsed = target ? parseValue(target, valueText) : null;
  const request =
    session && node && target && parsed && "value" in parsed
      ? buildRequest({
          baseUrl,
          sessionId: follow ? null : session.id,
          nodeId: node.id,
          target,
          value: parsed.value,
          idempotencyKey: key,
        })
      : null;
  const text = request
    ? format === "json"
      ? request.json
      : format === "curl"
        ? request.curl
        : request.powershell
    : "";
  const booleanTarget = target && (target.kind === "flag" || target.spec.type === "boolean");
  const choices =
    target?.kind === "parameter"
      ? (target.choices ?? target.spec.enum?.map((value) => ({ value, label: value })))
      : undefined;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setStatus("Copied. Replace the token placeholder with your API token.");
    } catch {
      setStatus("Select the text and copy it manually.");
    }
  };
  const send = async () => {
    if (!request || !onSend || sending) return;
    setSending(true);
    setStatus(null);
    try {
      const result = await onSend(request.body as MethodParams["nodes.set"]);
      setStatus(`Sent. Saved as revision ${result.revision ?? "—"}; the app updates from the backend.`);
      setKey(exampleIdempotencyKey());
    } catch (error) {
      setStatus(formatUiError(error, "The backend refused the request."));
    } finally {
      setSending(false);
    }
  };

  if (!session)
    return (
      <section className="request-builder" aria-label="Request builder">
        <h3>Build a request</h3>
        <p className="muted">Save a session first; requests change saved sessions.</p>
      </section>
    );
  return (
    <section className="request-builder" aria-label="Request builder">
      <h3>Build a request</h3>
      <p className="muted">
        Pick a tool and a setting to get the exact request another app (a game integration, StreamDeck, a script) can
        send. Choosing does not change anything; only Send does.
      </p>
      <fieldset className="request-builder-session">
        <legend>Session</legend>
        <label className="request-builder-radio">
          <input type="radio" name="request-session" checked={follow} onChange={() => setFollow(true)} />
          Follow the session selected in AudioRouter
        </label>
        <label className="request-builder-radio">
          <input type="radio" name="request-session" checked={!follow} onChange={() => setFollow(false)} />
          Always this session
        </label>
        {!follow && (
          <label>
            Pinned session
            <select
              aria-label="Pinned session"
              value={session.id}
              onChange={(event) => setPinnedId(event.target.value)}
            >
              {sessions.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </label>
        )}
      </fieldset>
      <label>
        Tool
        <select aria-label="Request tool" value={nodeId} onChange={(event) => setNodeId(event.target.value)}>
          {nodes.map((item) => (
            <option key={item.id} value={item.id}>
              {item.label}
            </option>
          ))}
        </select>
      </label>
      <label>
        Setting
        <select aria-label="Request setting" value={targetKey} onChange={(event) => setTargetKey(event.target.value)}>
          {targets.map((item) => (
            <option key={item.key} value={item.key}>
              {item.label}
            </option>
          ))}
        </select>
      </label>
      {target && (
        <label>
          Value
          {booleanTarget ? (
            <select aria-label="Request value" value={valueText} onChange={(event) => setValueText(event.target.value)}>
              <option value="true">true</option>
              <option value="false">false</option>
            </select>
          ) : choices ? (
            <select aria-label="Request value" value={valueText} onChange={(event) => setValueText(event.target.value)}>
              {choices.map((choice) => (
                <option key={choice.value} value={choice.value}>
                  {choice.label}
                </option>
              ))}
            </select>
          ) : (
            <input
              aria-label="Request value"
              inputMode="decimal"
              value={valueText}
              onChange={(event) => setValueText(event.target.value)}
            />
          )}
        </label>
      )}
      {target?.kind === "parameter" && target.spec.type === "number" && (
        <small className="muted">
          From {target.spec.minimum ?? "−∞"} to {target.spec.maximum ?? "∞"}
          {target.spec.unit ? ` ${target.spec.unit}` : ""}.
        </small>
      )}
      {parsed && "error" in parsed && (
        <p className="request-builder-error" role="alert">
          {parsed.error}
        </p>
      )}
      {request && (
        <>
          <label>
            Request
            <input aria-label="Request URL" readOnly value={`POST ${request.url}`} />
          </label>
          <div className="request-builder-formats" role="group" aria-label="Request format">
            {(["json", "curl", "powershell"] as const).map((item) => (
              <button
                key={item}
                type="button"
                className="secondary"
                aria-pressed={format === item}
                onClick={() => setFormat(item)}
              >
                {item === "json" ? "JSON body" : item === "curl" ? "curl" : "PowerShell"}
              </button>
            ))}
          </div>
          <label>
            {format === "json" ? "Body (send with an Authorization: Bearer header)" : "Command"}
            <textarea
              aria-label="Generated request"
              readOnly
              rows={format === "json" ? 7 : 4}
              value={text}
              onFocus={(event) => event.currentTarget.select()}
            />
          </label>
          <small className="muted">
            Use a new idempotencyKey for each event; a retry of the same event reuses its key so it applies once. The
            token is a placeholder; copy yours from above.
          </small>
          <div className="actions">
            <button type="button" className="secondary" onClick={() => void copy()}>
              Copy
            </button>
            <button type="button" onClick={() => void send()} disabled={!connected || !onSend || sending}>
              {sending ? "Sending…" : "Send now"}
            </button>
          </div>
        </>
      )}
      {status && <p role="status">{status}</p>}
    </section>
  );
}
