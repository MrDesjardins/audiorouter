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
import { useMessages } from "./i18n/useMessages";

type Format = "json" | "curl" | "powershell";

function savedValueText(target: BuilderTarget, node: Session["nodes"][number]): string {
  const current = target.kind === "flag" ? node[target.key] : (node.parameters[target.key] ?? target.spec.default);
  if (current !== undefined && current !== null) return String(current);
  return target.kind === "parameter" && target.choices ? (target.choices[0]?.value ?? "") : "";
}

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
  const tr = useMessages();
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
  // A backend refresh brings new objects with the same content, and can land after a send has
  // returned. Compare plain values, so it cannot erase the Sent message or a value being typed.
  const choiceIdentity = target ? JSON.stringify([session?.id, nodeId, target.kind, target.key]) : null;
  const savedText = target && node ? savedValueText(target, node) : null;
  useEffect(() => {
    if (savedText !== null) setValueText(savedText);
  }, [choiceIdentity, savedText]);
  useEffect(() => {
    setStatus(null);
  }, [choiceIdentity]);

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
      setStatus(tr("g6.apiBuilderCopied"));
    } catch {
      setStatus(tr("g6.apiBuilderCopyManual"));
    }
  };
  const send = async () => {
    if (!request || !onSend || sending) return;
    setSending(true);
    setStatus(null);
    try {
      const result = await onSend(request.body as MethodParams["nodes.set"]);
      setStatus(tr("g6.apiBuilderSent", { revision: result.revision ?? "—" }));
      setKey(exampleIdempotencyKey());
    } catch (error) {
      setStatus(formatUiError(error, tr("g6.apiBuilderRefused")));
    } finally {
      setSending(false);
    }
  };

  if (!session)
    return (
      <section className="request-builder" aria-label={tr("g6.apiBuilderTitle")}>
        <h3>{tr("g6.apiBuilderBuild")}</h3>
        <p className="muted">{tr("g6.apiBuilderSaveFirst")}</p>
      </section>
    );
  return (
    <section className="request-builder" aria-label={tr("g6.apiBuilderTitle")}>
      <h3>{tr("g6.apiBuilderBuild")}</h3>
      <p className="muted">{tr("g6.apiBuilderIntro")}</p>
      <fieldset className="request-builder-session">
        <legend>{tr("g6.apiBuilderSession")}</legend>
        <label className="request-builder-radio">
          <input type="radio" name="request-session" checked={follow} onChange={() => setFollow(true)} />
          {tr("g6.apiBuilderFollow")}
        </label>
        <label className="request-builder-radio">
          <input type="radio" name="request-session" checked={!follow} onChange={() => setFollow(false)} />
          {tr("g6.apiBuilderAlways")}
        </label>
        {!follow && (
          <label>
            {tr("g6.apiBuilderPinned")}
            <select
              aria-label={tr("g6.apiBuilderPinned")}
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
        {tr("g6.apiBuilderTool")}
        <select
          aria-label={tr("g6.apiBuilderToolAria")}
          value={nodeId}
          onChange={(event) => setNodeId(event.target.value)}
        >
          {nodes.map((item) => (
            <option key={item.id} value={item.id}>
              {item.label}
            </option>
          ))}
        </select>
      </label>
      <label>
        {tr("g6.apiBuilderSetting")}
        <select
          aria-label={tr("g6.apiBuilderSettingAria")}
          value={targetKey}
          onChange={(event) => setTargetKey(event.target.value)}
        >
          {targets.map((item) => (
            <option key={item.key} value={item.key}>
              {item.message ? tr(item.message.key, item.message.vars) : item.label}
            </option>
          ))}
        </select>
      </label>
      {target && (
        <label>
          {tr("g6.apiBuilderValue")}
          {booleanTarget ? (
            <select
              aria-label={tr("g6.apiBuilderValueAria")}
              value={valueText}
              onChange={(event) => setValueText(event.target.value)}
            >
              <option value="true">true</option>
              <option value="false">false</option>
            </select>
          ) : choices ? (
            <select
              aria-label={tr("g6.apiBuilderValueAria")}
              value={valueText}
              onChange={(event) => setValueText(event.target.value)}
            >
              {choices.map((choice) => (
                <option key={choice.value} value={choice.value}>
                  {choice.label}
                </option>
              ))}
            </select>
          ) : (
            <input
              aria-label={tr("g6.apiBuilderValueAria")}
              inputMode="decimal"
              value={valueText}
              onChange={(event) => setValueText(event.target.value)}
            />
          )}
        </label>
      )}
      {target?.kind === "parameter" && target.spec.type === "number" && (
        <small className="muted">
          {tr("g6.apiBuilderRange", {
            min: target.spec.minimum ?? "−∞",
            max: target.spec.maximum ?? "∞",
            unit: target.spec.unit ? ` ${target.spec.unit}` : "",
          })}
        </small>
      )}
      {parsed && "error" in parsed && (
        <p className="request-builder-error" role="alert">
          {parsed.message ? tr(parsed.message.key, parsed.message.vars) : parsed.error}
        </p>
      )}
      {request && (
        <>
          <label>
            {tr("g6.apiBuilderRequest")}
            <input aria-label={tr("g6.apiBuilderUrlAria")} readOnly value={`POST ${request.url}`} />
          </label>
          <div className="request-builder-formats" role="group" aria-label={tr("g6.apiBuilderFormatAria")}>
            {(["json", "curl", "powershell"] as const).map((item) => (
              <button
                key={item}
                type="button"
                className="secondary"
                aria-pressed={format === item}
                onClick={() => setFormat(item)}
              >
                {item === "json" ? tr("g6.apiBuilderJsonBody") : item === "curl" ? "curl" : "PowerShell"}
              </button>
            ))}
          </div>
          <label>
            {format === "json" ? tr("g6.apiBuilderBodyLabel") : tr("g6.apiBuilderCommand")}
            <textarea
              aria-label={tr("g6.apiBuilderGenerated")}
              readOnly
              rows={format === "json" ? 7 : 4}
              value={text}
              onFocus={(event) => event.currentTarget.select()}
            />
          </label>
          <small className="muted">{tr("g6.apiBuilderKeyHelp")}</small>
          <div className="actions">
            <button type="button" className="secondary" onClick={() => void copy()}>
              {tr("g6.apiBuilderCopy")}
            </button>
            <button type="button" onClick={() => void send()} disabled={!connected || !onSend || sending}>
              {sending ? tr("g6.apiBuilderSending") : tr("g6.apiBuilderSend")}
            </button>
          </div>
        </>
      )}
      {status && <p role="status">{status}</p>}
    </section>
  );
}
