import { useEffect, useState } from "react";
import type { NetworkNodeTelemetry, Node } from "@audiorouter/contracts";
import { tx } from "./i18n";
import { DEFAULT_NETWORK_AUDIO_PORT } from "./draft";
import { boldMarkup } from "./boldMarkup";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

const IPV4 = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/;

/** An IPv4 or IPv6 literal, as the backend accepts it. Host names are not
 * resolved, so a route never depends on DNS. */
export function isNetworkAddress(value: string): boolean {
  if (value.length === 0 || value.length > 64 || value.trim() !== value) return false;
  if (IPV4.test(value)) return true;
  // IPv6: hex groups with at most one "::"; the backend performs the exact check.
  return value.includes(":") && /^[0-9a-fA-F:.]+$/.test(value) && (value.match(/::/g) ?? []).length <= 1;
}

/** Pairing keys the backend accepts: blank (not paired) or 16-128
 * printable ASCII characters without leading or trailing spaces. */
export function isPairingKey(value: string): boolean {
  return (
    value.length === 0 ||
    (value.length >= 16 && value.length <= 128 && /^[\x20-\x7e]+$/.test(value) && value.trim() === value)
  );
}

/** 32 symbols without the look-alikes 0/O and 1/I: a random byte masked to
 * 5 bits picks one without bias, so 24 characters carry 120 random bits. */
const PAIRING_SYMBOLS = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
export const GENERATED_PAIRING_KEY_LENGTH = 24;

/** A new random pairing key from the platform's cryptographic generator. */
export function generatePairingKey(
  fill: (bytes: Uint8Array<ArrayBuffer>) => Uint8Array = (bytes) => {
    crypto.getRandomValues(bytes);
    return bytes;
  },
): string {
  const bytes = fill(new Uint8Array(GENERATED_PAIRING_KEY_LENGTH));
  return Array.from(bytes, (byte) => PAIRING_SYMBOLS[byte & 31]).join("");
}

type Translate = (key: MessageKey, options?: Record<string, unknown>) => string;

/** Translator for callers outside React components (the canvas); components pass their own. */
const currentLanguage: Translate = tx;

/** Short enough for the canvas node's one-line status. */
function pairingProblemText(telemetry: NetworkNodeTelemetry, tr: Translate): string {
  const n = telemetry.authFailures ?? 0;
  switch (telemetry.authProblem) {
    case "senderNotPaired":
      return tr("g2.nwNoKey", { n });
    case "receiverNotPaired":
      return tr("g2.nwHadKey", { n });
    case "senderOutdated":
      return tr("g2.nwOlder", { n });
    default:
      return tr("g2.nwOtherKey", { n });
  }
}

/** What to change when audio arrives with the wrong pairing, or null. */
export function pairingAdvice(
  telemetry: NetworkNodeTelemetry | null | undefined,
  tr: Translate = currentLanguage,
): string | null {
  if (
    !telemetry ||
    telemetry.direction !== "receive" ||
    !telemetry.authFailures ||
    (telemetry.receivedPackets ?? 0) > 0
  )
    return null;
  switch (telemetry.authProblem) {
    case "senderNotPaired":
      return tr("g2.nwAdviceNoKey");
    case "receiverNotPaired":
      return tr("g2.nwAdviceHasKey");
    case "senderOutdated":
      return tr("g2.nwAdviceOutdated");
    default:
      return tr("g2.nwAdviceWrongKey");
  }
}

/** A one-line status of a running network node, or null when idle. */
export function networkTelemetryText(
  telemetry: NetworkNodeTelemetry | null | undefined,
  tr: Translate = currentLanguage,
): string | null {
  if (!telemetry) return null;
  if (telemetry.direction === "send") {
    const problems = [
      telemetry.droppedPackets ? tr("g2.nwDropped", { n: telemetry.droppedPackets }) : null,
      telemetry.sendErrors
        ? tr(telemetry.lastErrorCode === 10054 ? "g2.nwSendErrorsClosed" : "g2.nwSendErrors", {
            n: telemetry.sendErrors,
          })
        : null,
    ].filter(Boolean);
    return `${tr("g2.nwSending", { n: telemetry.sentPackets ?? 0 })}${problems.length ? ` · ${problems.join(" · ")}` : ""}`;
  }
  const received = telemetry.receivedPackets ?? 0;
  if (received === 0) {
    if (telemetry.authFailures) return tr("g2.nwWaitingPairing", { problem: pairingProblemText(telemetry, tr) });
    if (telemetry.rejectedFrom) return tr("g2.nwWaitingRejectedFrom", { from: telemetry.rejectedFrom });
    return telemetry.rejectedDatagrams
      ? tr("g2.nwWaitingOtherAddress", { n: telemetry.rejectedDatagrams })
      : tr("g2.nwWaiting");
  }
  const problems = [
    telemetry.lostPackets ? tr("g2.nwLost", { n: telemetry.lostPackets }) : null,
    telemetry.latePackets ? tr("g2.nwLate", { n: telemetry.latePackets }) : null,
    telemetry.underruns ? tr("g2.nwGaps", { n: telemetry.underruns }) : null,
    telemetry.authFailures ? tr("g2.nwWithOtherKey", { n: telemetry.authFailures }) : null,
    telemetry.replayedPackets ? tr("g2.nwReplayed", { n: telemetry.replayedPackets }) : null,
  ].filter(Boolean);
  return `${tr("g2.nwReceiving", { n: received, ms: Math.round(telemetry.bufferedMs ?? 0) })}${problems.length ? ` · ${problems.join(" · ")}` : ""}`;
}

/** Properties of a Network Send or Network Receive node. Only valid values
 * reach the draft; the backend validates them again before audio starts. */
export function NetworkNodeEditor({
  node,
  disabled,
  telemetry,
  onChange,
}: {
  node: Node;
  disabled: boolean;
  telemetry: NetworkNodeTelemetry | null | undefined;
  onChange: (name: string, value: number | string) => void;
}) {
  const tr = useMessages();
  const sending = node.kind === "networkSend";
  const addressKey = sending ? "host" : "sender";
  const savedAddress = typeof node.parameters[addressKey] === "string" ? String(node.parameters[addressKey]) : "";
  const savedPort = Number(node.parameters.port ?? DEFAULT_NETWORK_AUDIO_PORT);
  const savedBuffer = Number(node.parameters.bufferMs ?? 40);
  const savedPairingKey = typeof node.parameters.pairingKey === "string" ? node.parameters.pairingKey : "";
  const [address, setAddress] = useState(savedAddress);
  const [port, setPort] = useState(String(savedPort));
  const [buffer, setBuffer] = useState(String(savedBuffer));
  useEffect(() => setAddress(savedAddress), [node.id, savedAddress]);
  useEffect(() => setPort(String(savedPort)), [node.id, savedPort]);
  useEffect(() => setBuffer(String(savedBuffer)), [node.id, savedBuffer]);
  const [pairingKey, setPairingKey] = useState(savedPairingKey);
  useEffect(() => setPairingKey(savedPairingKey), [node.id, savedPairingKey]);
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1500);
    return () => window.clearTimeout(timer);
  }, [copied]);

  const addressValid = isNetworkAddress(address);
  const portNumber = Number(port);
  const portValid = Number.isInteger(portNumber) && portNumber >= 1 && portNumber <= 65535;
  const bufferNumber = Number(buffer);
  const bufferValid = Number.isFinite(bufferNumber) && bufferNumber >= 10 && bufferNumber <= 500;
  const pairingValid = isPairingKey(pairingKey);
  const changePairingKey = (value: string) => {
    setPairingKey(value);
    if (isPairingKey(value)) onChange("pairingKey", value);
  };
  const copyPairingKey = () => {
    void navigator.clipboard?.writeText(pairingKey).then(
      () => setCopied(true),
      () => setCopied(false),
    );
  };
  // One reserved slot (UI-17): the warning, the paired note and the
  // validation message replace each other without moving anything below.
  // Advice from live telemetry applies to the saved key only: once the key
  // is edited here, the counters describe the previous one.
  const advice =
    pairingKey === savedPairingKey && Boolean(telemetry?.paired) === savedPairingKey.length > 0
      ? pairingAdvice(telemetry, tr)
      : null;
  const pairingNote = !pairingValid
    ? tr("g2.nwPairingInvalid")
    : (advice ??
      (pairingKey.length === 0
        ? sending
          ? tr("g2.nwSendUnpaired")
          : tr("g2.nwRecvUnpaired")
        : sending
          ? tr("g2.nwSendPaired")
          : tr("g2.nwRecvPaired")));
  const status = networkTelemetryText(telemetry, tr);
  const thisPc = sending ? telemetry?.localAddress : telemetry?.thisAddress;
  const addressLabel = sending ? tr("g2.nwAddrSend") : tr("g2.nwAddrRecv");
  const portPhrase = portValid ? tr("g2.nwPortPhrase", { n: portNumber }) : tr("g2.nwSamePortPhrase");
  const thisAddress = thisPc ? tr("g2.nwThisAddrKnown", { addr: thisPc }) : tr("g2.nwThisAddrUnknown");

  return (
    <div
      className="node-binding-editor network-node-editor"
      aria-label={sending ? tr("g2.nwSendAria") : tr("g2.nwRecvAria")}
    >
      <div>
        <p className="eyebrow">{sending ? tr("g2.nwEyebrowSend") : tr("g2.nwEyebrowRecv")}</p>
        <strong>{sending ? tr("g2.nwHeadSend") : tr("g2.nwHeadRecv")}</strong>
      </div>
      <figure className="network-route-diagram" aria-label={tr("g2.nwDiagramAria")}>
        <div className="network-route-computer">
          <svg viewBox="0 0 48 36" aria-hidden="true">
            <rect x="6" y="2" width="36" height="24" rx="3" />
            <path d="M18 34h12M24 26v8M12 16h4l3-7 6 13 4-9h7" />
          </svg>
          <strong>{sending ? tr("g2.nwThisPc") : tr("g2.nwSendingPc")}</strong>
          <span>{tr("g2.nwAudioIn")}</span>
          <b>{tr("g2.nwToolSend")}</b>
          <code>{sending ? tr("g2.nwYourInputMix") : addressValid ? address : tr("g2.nwSenderIp")}</code>
        </div>
        <div className="network-route-link">
          <span aria-hidden="true">→</span>
          <small>UDP</small>
          <code>{portValid ? portNumber : tr("g2.nwPortWord")}</code>
        </div>
        <div className="network-route-computer">
          <svg viewBox="0 0 48 36" aria-hidden="true">
            <rect x="6" y="2" width="36" height="24" rx="3" />
            <path d="M18 34h12M24 26v8M12 16h4l3-7 6 13 4-9h7" />
          </svg>
          <strong>{sending ? tr("g2.nwReceivingPc") : tr("g2.nwThisPc")}</strong>
          <span>{tr("g2.nwAudioOut")}</span>
          <b>{tr("g2.nwToolRecv")}</b>
          <code>{sending ? (addressValid ? address : tr("g2.nwDestIp")) : tr("g2.nwYourOutputObs")}</code>
        </div>
        <figcaption>
          {tr(sending ? "g2.nwSetSender" : "g2.nwSetDest", { addr: thisPc ?? tr("g2.nwThisPcIp") })}{" "}
          {tr("g2.nwUsePortBoth", { port: portPhrase })}
        </figcaption>
      </figure>
      <label>
        {addressLabel}
        <input
          type="text"
          inputMode="decimal"
          autoComplete="off"
          spellCheck={false}
          placeholder="192.168.1.20"
          value={address}
          disabled={disabled}
          aria-invalid={address.length > 0 && !addressValid}
          aria-describedby={`${node.id}-address-note`}
          onChange={(event) => {
            const value = event.target.value.trim();
            setAddress(value);
            if (isNetworkAddress(value)) onChange(addressKey, value);
          }}
        />
      </label>
      {/* The slot is always there and keeps its height, so the reason appearing moves nothing (UI-17). */}
      <small
        id={`${node.id}-address-note`}
        className="network-address-note"
        role={address.length > 0 && !addressValid ? "alert" : undefined}
      >
        {address.length > 0 && !addressValid ? tr("g2.nwAddrInvalid") : ""}
      </small>
      <label>
        {tr("g2.nwPortWord")}
        <input
          type="number"
          min={1}
          max={65535}
          step={1}
          value={port}
          disabled={disabled}
          aria-invalid={!portValid}
          onChange={(event) => {
            setPort(event.target.value);
            const value = Number(event.target.value);
            if (Number.isInteger(value) && value >= 1 && value <= 65535) onChange("port", value);
          }}
        />
      </label>
      <div className="network-pairing">
        <label>
          {tr("g2.nwPairingKey")}
          <input
            type="text"
            autoComplete="off"
            spellCheck={false}
            maxLength={128}
            placeholder={tr("g2.nwPairingPlaceholder")}
            value={pairingKey}
            disabled={disabled}
            aria-invalid={!pairingValid}
            aria-describedby={`${node.id}-pairing-note`}
            onChange={(event) => changePairingKey(event.target.value)}
          />
        </label>
        <div className="network-pairing-actions">
          <button
            type="button"
            className="secondary"
            disabled={disabled}
            onClick={() => changePairingKey(generatePairingKey())}
          >
            {tr("g2.nwGenerate")}
          </button>
          <button
            type="button"
            className="secondary"
            disabled={pairingKey.length === 0 || !pairingValid}
            onClick={copyPairingKey}
            aria-live="polite"
          >
            {copied ? tr("g2.nwCopied") : tr("g2.nwCopy")}
          </button>
        </div>
        <p
          id={`${node.id}-pairing-note`}
          className={`network-pairing-note${pairingValid && pairingKey.length > 0 && !advice ? " paired" : ""}`}
          data-testid="network-pairing-note"
        >
          {pairingNote}
        </p>
      </div>
      {!sending && (
        <label>
          {tr("g2.nwBuffer")}
          <input
            type="number"
            min={10}
            max={500}
            step={5}
            value={buffer}
            disabled={disabled}
            aria-invalid={!bufferValid}
            onChange={(event) => {
              setBuffer(event.target.value);
              const value = Number(event.target.value);
              if (Number.isFinite(value) && value >= 10 && value <= 500) onChange("bufferMs", value);
            }}
          />
        </label>
      )}
      {status && (
        <p className="network-node-status" role="status">
          {status}
        </p>
      )}
      {!sending && telemetry?.rejectedFrom && telemetry.rejectedFrom !== address && (
        <div className="network-node-fix" role="alert">
          <span>{boldMarkup(tr("g2.nwArrivingFrom", { from: telemetry.rejectedFrom }))}</span>
          <button
            type="button"
            className="secondary"
            disabled={disabled}
            onClick={() => {
              setAddress(telemetry.rejectedFrom!);
              onChange("sender", telemetry.rejectedFrom!);
            }}
          >
            {tr("g2.nwUseAddr", { from: telemetry.rejectedFrom })}
          </button>
        </div>
      )}
      {/* While playing, name this computer's real address: with several network
        adapters, the one ipconfig lists first is often not the one used. */}
      <small>
        {boldMarkup(
          sending
            ? tr("g2.nwFooterSend", { addr: thisAddress })
            : tr("g2.nwFooterRecv", { addr: thisAddress, port: portPhrase }),
        )}
        {!thisPc && <> {boldMarkup(tr("g2.nwFindIp"))}</>}
      </small>
    </div>
  );
}
