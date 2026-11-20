import { useEffect, useState, type ReactNode } from "react";
import { useMessages } from "./i18n/useMessages";
import { NumberField } from "./NumberField";
import { StreamDeckPluginPanel } from "./StreamDeckPluginPanel";

type ApiStatus = {
  running: boolean;
  port: number;
  url: string | null;
  network?: string | null;
  networkUrl?: string | null;
  token: string | null;
};
type LanAddress = { address: string; adapter: string };
/** `builder` renders the request builder for the API base URL. */
export function ApiPanel({ builder }: { builder?: (baseUrl: string) => ReactNode } = {}) {
  const tr = useMessages();
  const [status, setStatus] = useState<ApiStatus>({ running: false, port: 17891, url: null, token: null });
  const [port, setPort] = useState(17891);
  // "" = this PC only; otherwise one of this PC's private addresses (HTTP-09).
  const [network, setNetwork] = useState("");
  const [addresses, setAddresses] = useState<LanAddress[]>([]);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [regenerateConfirm, setRegenerateConfirm] = useState(false);
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const loadAddresses = () =>
    invoke?.("http_api_addresses")
      .then((value) => setAddresses(Array.isArray(value) ? (value as LanAddress[]) : []))
      .catch(() => setAddresses([]));
  useEffect(() => {
    let active = true;
    void invoke?.("http_api_control", { action: "reveal", port: null })
      .then((value) => {
        if (active) {
          const next = value as ApiStatus;
          setStatus(next);
          setPort(next.port);
          setNetwork(next.network ?? "");
        }
      })
      .catch(() => {
        if (active) setMessage(tr("g6.apiPanelReadError"));
      });
    void loadAddresses();
    return () => {
      active = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [invoke]);
  const control = async (action: string) => {
    if (!invoke || busy) return;
    setBusy(true);
    setMessage("");
    try {
      const next = (await invoke("http_api_control", { action, port, network: network || null })) as ApiStatus;
      setStatus((previous) => ({
        ...next,
        token: action === "reveal" || action === "regenerate" ? next.token : previous.token,
      }));
      if (action === "regenerate") {
        setRegenerateConfirm(false);
        setMessage(tr("g6.apiPanelRegenerated"));
      }
    } catch (error) {
      setMessage(typeof error === "string" ? error : tr("g6.apiPanelActionFailed"));
      // Replacement can save successfully but fail to rebind the listener.
      try {
        setStatus(
          (await invoke("http_api_control", { action: "status", port, network: network || null })) as ApiStatus,
        );
      } catch {
        setStatus((previous) => ({ ...previous, token: null }));
      }
    } finally {
      setBusy(false);
    }
  };
  const copyToken = async () => {
    if (!status.token) return;
    try {
      await navigator.clipboard.writeText(status.token);
      setMessage(tr("settingsCopy.tokenCopied"));
    } catch {
      setMessage(tr("settingsCopy.tokenManual"));
    }
  };
  return (
    <section className="workbench-page" aria-label={tr("g6.apiPanelHeading")}>
      <p className="eyebrow">{tr("settingsCopy.apiHeading")}</p>
      <h2>{tr("g6.apiPanelHeading")}</h2>
      <p className="muted">{tr("settingsCopy.apiIntro")}</p>
      {!invoke && <p role="status">{tr("settingsCopy.apiDesktop")}</p>}
      <label>
        {tr("settingsCopy.port")}
        <NumberField
          aria-label={tr("settingsCopy.apiPort")}
          value={port}
          min={1024}
          max={65535}
          step={1}
          disabled={status.running || busy}
          onValue={(value) => {
            if (Number.isInteger(value)) setPort(value);
          }}
        />
      </label>
      <label>
        {tr("settingsCopy.who")}
        <select
          aria-label={tr("settingsCopy.who")}
          value={network}
          disabled={status.running || busy}
          onFocus={() => void loadAddresses()}
          onChange={(event) => setNetwork(event.currentTarget.value)}
        >
          <option value="">{tr("settingsCopy.pcOnly")}</option>
          {network && !addresses.some((entry) => entry.address === network) && (
            <option value={network}>{tr("settingsCopy.networkOption", { name: network })}</option>
          )}
          {addresses.map((entry) => (
            <option key={entry.address} value={entry.address}>
              {tr("settingsCopy.networkOption", {
                name: `${entry.adapter || tr("settingsCopy.adapter")} (${entry.address})`,
              })}
            </option>
          ))}
        </select>
      </label>
      {network ? (
        <p className="muted">{tr("settingsCopy.networkAccess")}</p>
      ) : (
        <p className="muted">
          {tr("settingsCopy.localAccess")}
          {addresses.length === 0 && invoke ? ` ${tr("settingsCopy.noAddress")}` : ""}
        </p>
      )}
      <p role="status">
        {status.running
          ? status.network
            ? tr("settingsCopy.apiNetwork", { address: status.network })
            : tr("settingsCopy.apiLocal")
          : tr("settingsCopy.apiStopped")}
      </p>
      <button type="button" disabled={!invoke || busy} onClick={() => void control(status.running ? "stop" : "start")}>
        {tr(busy ? "settingsCopy.working" : status.running ? "settingsCopy.apiStop" : "settingsCopy.apiStart")}
      </button>
      <p className="muted">{tr("settingsCopy.apiAutostart")}</p>
      {status.url && (
        <>
          <label>
            {tr("g6.apiPanelUrl")}
            <input aria-label={tr("g6.apiPanelUrlAria")} readOnly value={status.url} />
          </label>
          {status.networkUrl && (
            <label>
              {tr("settingsCopy.networkUrl")}
              <input aria-label={tr("settingsCopy.apiNetworkUrl")} readOnly value={status.networkUrl} />
            </label>
          )}
          <button type="button" className="secondary" disabled={busy} onClick={() => void control("openDocs")}>
            {tr("settingsCopy.swagger")}
          </button>
          <p className="muted">{tr("settingsCopy.swaggerHelp")}</p>
        </>
      )}
      <p className="muted">{tr("settingsCopy.tokenHelp")}</p>
      {!status.token ? (
        <button type="button" className="secondary" disabled={!invoke || busy} onClick={() => void control("reveal")}>
          {tr("settingsCopy.revealToken")}
        </button>
      ) : (
        <>
          <label>
            {tr("settingsCopy.bearer")}
            <textarea
              aria-label={tr("settingsCopy.apiBearer")}
              rows={2}
              readOnly
              value={status.token}
              onFocus={(event) => event.currentTarget.select()}
            />
          </label>
          <div className="actions">
            <button type="button" className="secondary" disabled={busy} onClick={() => void copyToken()}>
              {tr("g6.apiPanelCopyToken")}
            </button>
            <button
              type="button"
              className="secondary"
              disabled={busy}
              onClick={() => setStatus({ ...status, token: null })}
            >
              {tr("g6.apiPanelHideToken")}
            </button>
          </div>
        </>
      )}
      <p className="muted">{tr("g6.apiPanelRegenerateHelp")}</p>
      {!regenerateConfirm ? (
        <button
          type="button"
          className="secondary"
          disabled={!invoke || busy}
          onClick={() => setRegenerateConfirm(true)}
        >
          {tr("g6.apiPanelGenerate")}
        </button>
      ) : (
        <div className="api-token-confirm">
          <p>{tr("g6.apiPanelConfirm")}</p>
          <div className="actions">
            <button type="button" disabled={busy} onClick={() => void control("regenerate")}>
              {tr("g6.apiPanelReplace")}
            </button>
            <button type="button" className="secondary" disabled={busy} onClick={() => setRegenerateConfirm(false)}>
              {tr("common.cancel")}
            </button>
          </div>
        </div>
      )}
      {message && <p role="status">{message}</p>}
      <StreamDeckPluginPanel />
      {builder?.(status.url ?? `http://127.0.0.1:${port}`)}
    </section>
  );
}
