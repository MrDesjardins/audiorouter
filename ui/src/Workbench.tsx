import type { LibraryEntry } from "./library";
import type { Session } from "@audiorouter/contracts";
import { useState, type ReactNode } from "react";

const TOOL_ICONS: Record<string, string> = { volume: "◖", "bass-treble": "♮", dehum: "≁", declick: "⌇", denoise: "░", "speech-denoise": "☊", "fir-filter": "⧉", "input-switch": "⇄", "time-shift": "↺", "physical-input": "◉", "test-signal": "∿", "audio-file": "♫", "endpoint-loopback": "↶", "virtual-render-source": "⊞", "physical-output": "◎", "virtual-capture-sink": "⊟", gain: "◢", mixer: "⋈", recorder: "●", mute: "⊘", meter: "▥", "parametric-eq": "⌁", compressor: "⤓", gate: "⊐", limiter: "⊤", delay: "◷", "graphic-eq": "▤", pitch: "↟" };
const TOOL_HELP: Record<string, string> = { "physical-input": "Bring sound from a microphone, line input, or installed virtual capture bus into the route.", "test-signal": "Generate a steady test tone to check the route.", "physical-output": "Send the route to speakers, headphones, or an installed virtual playback device.", gain: "Raise or lower the level of sound passing through.", mixer: "Combine several sound sources into one route.", recorder: "Record sound passing through this point.", mute: "Silence this part of the route without removing it.", meter: "See the level of sound at this point.", "parametric-eq": "Shape chosen frequency ranges.", compressor: "Reduce the difference between loud and quiet sound.", gate: "Reduce sound below a chosen level.", limiter: "Keep peaks below a chosen level.", delay: "Shift sound later in time.", "graphic-eq": "Adjust fixed frequency bands.", pitch: "Change the pitch of sound." };

function ApplicationSourceAction({ onClick, disabled }: { onClick: () => void; disabled: boolean }) {
  return <button type="button" className="secondary application-source-action" onClick={onClick} disabled={disabled}>
    <svg className="application-source-action-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
      <rect x="3" y="4" width="14" height="15" rx="2" />
      <path d="M3 8h14M7 6h.01M10 6h.01M19 10v4m-2-2h4" />
    </svg>
    <span><strong>Choose an application source</strong><small>Capture audio from a running app into this route</small></span>
  </button>;
}

export type WorkbenchTab = "tools" | "properties" | "session" | "setup" | "devices" | "recording" | "advanced" | "mcp" | "diagnostics";
export type McpActivity = { timeUnixMs: number; clientId: string; tool: string; argumentFields: string[]; outcome: string; errorKind?: string | null };
export type McpSetupInfo = { cliPath: string; cliAvailableBesideShell: boolean; databasePath: string; pipeName: string; transport: string };

export function Workbench({ tab, onTab, tools, connected, onAdd, onApplicationPicker, librarySearch, onLibrarySearch, onNewSession, onDuplicate, onDelete, onUndo, onRedo, onDiscard, actionMessage, onReplaceInputConnection, sessions, selectedSessionId, onSelectSession, sessionName, onNameChange, diagnostics, backendActivity, mcpActivity, mcpSetupInfo, clientsPanel, setupContent, devicesContent, recordingContent, advancedContent, pluginsContent }: {
  tab: WorkbenchTab; onTab: (tab: WorkbenchTab) => void; tools: LibraryEntry[]; connected: boolean;
  onAdd: (kind: NonNullable<LibraryEntry["kind"]>) => void; onNewSession: () => void; onDuplicate: () => void;
  onApplicationPicker: () => void; librarySearch: string; onLibrarySearch: (value: string) => void;
  onDelete: () => void; onUndo: () => void; onRedo: () => void; onDiscard: () => void;
  onPlan?: () => void; onCommit?: () => void; canCommit?: boolean; pendingPlan?: boolean;
  actionMessage?: string | null; onReplaceInputConnection?: () => void;
  sessions: Session[]; selectedSessionId: string; onSelectSession: (id: string) => void;
  sessionName: string; onNameChange: (name: string) => void;
  revision?: number; warnings?: string[]; acknowledgedWarnings?: string[]; onAcknowledgeWarning?: (warning: string, checked: boolean) => void;
  diagnostics: string[]; backendActivity: Record<string, unknown>[]; mcpActivity: McpActivity[]; mcpSetupInfo: McpSetupInfo | null; clientsPanel: ReactNode;
  setupContent: ReactNode; devicesContent: ReactNode; recordingContent: ReactNode; advancedContent: ReactNode; pluginsContent?: ReactNode;
}) {
  const [mcpClientId, setMcpClientId] = useState("audiorouter-local");
  const [copyMessage, setCopyMessage] = useState("");
  const [renaming, setRenaming] = useState(false);
  const cli = mcpSetupInfo?.cliPath ?? "PATH_TO_AUDIOROUTER_CLI";
  const db = mcpSetupInfo?.databasePath ?? "PATH_TO_AUDIOROUTER_DATABASE";
  const pipe = mcpSetupInfo?.pipeName ?? "\\\\.\\pipe\\audiorouter-control";
  const quoted = (value: string) => JSON.stringify(value);
  const psQuoted = (value: string) => `'${value.replaceAll("'", "''")}'`;
  const codexConfig = `[mcp_servers.audiorouter]\ncommand = ${quoted(cli)}\nargs = ["mcp", "serve", "--client-id", ${quoted(mcpClientId)}, "--database", ${quoted(db)}, "--pipe", ${quoted(pipe)}]`;
  const claudeCommand = `claude mcp add --scope user audiorouter -- ${psQuoted(cli)} mcp serve --client-id ${psQuoted(mcpClientId)} --database ${psQuoted(db)} --pipe ${psQuoted(pipe)}`;
  const copy = (value: string) => { if (!navigator.clipboard) { setCopyMessage("Clipboard unavailable. Select the text and copy it."); return; } void navigator.clipboard.writeText(value).then(() => setCopyMessage("Copied to clipboard.")).catch(() => setCopyMessage("Clipboard unavailable. Select the text and copy it.")); };
  const tabs: [WorkbenchTab, string][] = [["tools", "Tools"], ["properties", "Properties"], ["session", "Session"], ["setup", "Setup"], ["devices", "Devices"], ["recording", "Recording"], ["advanced", "Advanced"], ["mcp", "MCP"], ["diagnostics", "Logs"]];
  return <aside className="right-workbench" aria-label="Tools and settings">
    <div className="workbench-tabs" role="tablist" aria-label="Right sidebar">{tabs.map(([id, label]) => <button key={id} type="button" role="tab" aria-selected={tab === id} className={tab === id ? "active" : ""} onClick={() => onTab(id)}>{label}</button>)}</div>
    {tab === "tools" && <div className="workbench-tool-search"><label>Find a tool<input type="search" aria-label="Find a tool" value={librarySearch} onChange={(event) => onLibrarySearch(event.target.value)} placeholder="Input, gain, recorder…" /></label></div>}
    {tab === "tools" && <section className="workbench-page" role="tabpanel" aria-label="Tools">
      <p className="eyebrow">Build a route</p>
      <h2>Add tools</h2>
      <p className="muted">Add and connect tools, press Play to hear the route, then use Save at the top to keep it.</p>
      {(["input", "tool", "output"] as const).map((flow) => <section className="tool-group" key={flow}>
        <h3>{flow === "input" ? "Inputs" : flow === "tool" ? "Processing" : "Outputs"}</h3>
        {flow === "input" && <ApplicationSourceAction onClick={onApplicationPicker} disabled={!connected} />}
        {tools.filter((entry) => entry.flow === flow).map((entry) => {
          const help = entry.unavailableReason ?? entry.note ?? TOOL_HELP[entry.id] ?? entry.category;
          return <button className="tool-card" key={entry.id} type="button" onClick={() => entry.kind && onAdd(entry.kind)} disabled={!connected || !entry.kind} title={help} aria-description={help}>
            <span className="tool-card-icon" aria-hidden="true">{TOOL_ICONS[entry.id]}</span>
            <span><strong>{entry.label}</strong><small>{help}</small></span>
          </button>;
        })}
        {flow === "tool" && pluginsContent}
      </section>)}
    </section>}
    {tab === "properties" && <section className="workbench-page" role="tabpanel"><h2>Node properties</h2><p className="muted">Select a node on the canvas to edit it here.</p></section>}
    {tab === "setup" && <section className="workbench-page" role="tabpanel"><h2>First route</h2><p className="muted">Follow these steps to hear a route.</p><ol><li>Add a Test Signal, audio file, or microphone input.</li><li>Add Gain or another processing tool if needed.</li><li>Add a Physical Output and choose your speaker or headphone device in its Properties.</li><li>Connect the nodes and press Play. If an extra device selection is needed, the message will tell you where to make it.</li><li>Press Save at the top when you want to keep this setup.</li></ol>{setupContent}</section>}
    {tab === "devices" && <section className="workbench-page" role="tabpanel"><h2>Audio devices</h2><p className="muted">Choose the speaker or headphone device in the Physical Output node’s Properties. Choose a microphone in Physical Input Properties if the route uses one. Play prepares these exact devices when permission allows. This tab shows device details and recovery controls.</p>{devicesContent}</section>}
    {tab === "recording" && <section className="workbench-page" role="tabpanel"><h2>Recording</h2><p className="muted">Recording starts only after you explicitly arm and start a recorder. Confirm the destination and format before recording.</p>{recordingContent}</section>}
    {tab === "advanced" && <section className="workbench-page" role="tabpanel"><h2>Advanced controls</h2><p className="muted">These controls affect plugins, startup, connected assistant clients, and recovery. Review each action before applying it.</p>{advancedContent}</section>}
    {tab === "session" && <section className="workbench-page session-page" role="tabpanel"><h2>Sessions</h2><p className="muted">Switch between saved audio setups, or create a copy to try another arrangement. Use Save at the top when you want to keep your edits.</p><label>Current session<select aria-label="Choose session" value={selectedSessionId} onChange={(event) => { setRenaming(false); onSelectSession(event.target.value); }}>{sessions.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>{renaming ? <label>New session name<input aria-label="Session name" value={sessionName} maxLength={120} disabled={!connected} autoFocus onChange={(event) => onNameChange(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" || event.key === "Escape") setRenaming(false); }} /></label> : <button type="button" className="secondary" onClick={() => setRenaming(true)} disabled={!connected}>Rename</button>}<div className="session-action-grid"><button type="button" onClick={onNewSession} disabled={!connected}>New</button><button type="button" className="secondary" onClick={onDuplicate} disabled={!connected}>Duplicate</button><button type="button" className="secondary" onClick={onUndo} disabled={!connected}>Undo</button><button type="button" className="secondary" onClick={onRedo} disabled={!connected}>Redo</button><button type="button" className="secondary" onClick={onDiscard} disabled={!connected}>Revert edits</button><button type="button" className="danger" onClick={onDelete} disabled={!connected}>Delete session</button></div></section>}
    {tab === "mcp" && <section className="workbench-page" role="tabpanel"><p className="eyebrow">Local assistant connection</p><h2>Incoming tool calls</h2><p className="muted">Only the tool name, authorized client ID, safe field names, and outcome are shown. Audio and argument values are excluded.</p>{mcpActivity.length === 0 ? <p className="muted">No MCP tool calls have been recorded on this computer yet.</p> : <ol className="diagnostic-list" aria-label="MCP tool activity">{mcpActivity.map((entry, index) => <li key={`${entry.timeUnixMs}-${index}`}><strong>{new Date(entry.timeUnixMs).toLocaleTimeString()} · {entry.tool}</strong><small> {entry.outcome}{entry.errorKind ? ` (${entry.errorKind})` : ""} · client {entry.clientId} · fields: {entry.argumentFields.join(", ") || "none"}</small></li>)}</ol>}</section>}
    {tab === "mcp" && <section className="workbench-page mcp-setup-card"><h3>Use this installation</h3><p className="muted">Authorize this client ID below, then copy a setup snippet into your assistant. In the desktop shell, paths are filled automatically; browser previews show placeholders.</p><label>Client ID<input aria-label="Assistant client ID" value={mcpClientId} maxLength={128} onChange={(event) => setMcpClientId(event.target.value.trim())} /></label>{mcpSetupInfo && <details><summary>Connection paths</summary><p><code>{mcpSetupInfo.cliPath}</code></p><p><code>{mcpSetupInfo.databasePath}</code></p><p><code>{mcpSetupInfo.pipeName}</code></p>{!mcpSetupInfo.cliAvailableBesideShell && <p role="status">CLI executable is missing beside this shell.</p>}</details>}<details><summary>Codex config.toml</summary><pre className="mcp-command">{codexConfig}</pre><button className="secondary" onClick={() => copy(codexConfig)}>Copy Codex config</button></details><details><summary>Claude Code in PowerShell</summary><pre className="mcp-command">{claudeCommand}</pre><button className="secondary" onClick={() => copy(claudeCommand)}>Copy Claude command</button></details>{copyMessage && <p role="status">{copyMessage}</p>}<details><summary>Authorize or revoke a client</summary>{clientsPanel}</details></section>}
    {tab === "mcp" && <p className="muted mcp-howto">Authorize the same client ID above. Start with observer access. Copy the Codex TOML to <code>%USERPROFILE%\.codex\config.toml</code>, or run the Claude Code PowerShell command. Restart the assistant and ask it to list AudioRouter capabilities.</p>}
      {tab === "diagnostics" && <section className="workbench-page" role="tabpanel"><h2>Client diagnostics</h2><p className="muted">Recent UI error categories and graph checkpoints. Up to 80 short entries are saved in this local app profile so they survive a reload. Raw error text, file paths, and audio are excluded.</p><ol className="diagnostic-list">{diagnostics.map((row, index) => <li key={`${index}-${row}`}>{row}</li>)}</ol><h3>Backend RPC</h3>{backendActivity.length === 0 ? <p className="muted">No backend diagnostics have been recorded yet. Log files are stored under LocalAppData/AudioRouter/logs.</p> : <ol className="diagnostic-list" aria-label="Backend RPC activity">{backendActivity.map((entry, index) => <li key={`${String(entry.timeUnixMs)}-${index}`}><strong>{String(entry.method ?? "RPC")}</strong> · {String(entry.outcome ?? "unknown")}{entry.errorCode == null ? "" : ` · code ${String(entry.errorCode)}`}{entry.errorKind == null ? "" : ` · ${String(entry.errorKind)}`}<small> {JSON.stringify(entry.summary ?? entry.detail ?? {})}</small></li>)}</ol>}</section>}
  </aside>;
}
