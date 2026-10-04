// Stream Deck property inspector protocol, without third-party code. The
// Stream Deck app calls connectElgatoStreamDeckSocket when the panel opens.
// Settings save on change; lists come live from the plugin (AudioRouter).
const PI = {
  uuid: null,
  action: null,
  settings: {},
  global: {},
  options: null,
  socket: null,
  /** Called with the plugin's options: { state, error, session, nodes }. */
  onOptions: () => {},
  /** Called once the action's saved settings are known. */
  onSettings: () => {},
  send(message) {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) this.socket.send(JSON.stringify(message));
  },
  saveSettings(patch) {
    this.settings = { ...this.settings, ...patch };
    this.send({ event: "setSettings", context: this.uuid, payload: this.settings });
  },
  saveGlobal(patch) {
    this.global = { ...this.global, ...patch };
    this.send({ event: "setGlobalSettings", context: this.uuid, payload: this.global });
    this.sendToPlugin({ event: "getOptions" });
  },
  sendToPlugin(payload) {
    this.send({ event: "sendToPlugin", action: this.action, context: this.uuid, payload });
  },
};

window.connectElgatoStreamDeckSocket = (port, uuid, registerEvent, _info, actionInfo) => {
  const info = JSON.parse(actionInfo);
  PI.uuid = uuid;
  PI.action = info.action;
  PI.settings = info.payload?.settings ?? {};
  PI.socket = new WebSocket(`ws://127.0.0.1:${port}`);
  PI.socket.onopen = () => {
    PI.send({ event: registerEvent, uuid });
    PI.send({ event: "getGlobalSettings", context: uuid });
    PI.sendToPlugin({ event: "getOptions" });
    PI.onSettings(PI.settings);
  };
  PI.socket.onmessage = (message) => {
    const data = JSON.parse(message.data);
    if (data.event === "didReceiveGlobalSettings") {
      PI.global = data.payload?.settings ?? {};
      fillConnection();
    } else if (data.event === "sendToPropertyInspector" && data.payload?.event === "options") {
      PI.options = data.payload;
      showConnectionState();
      PI.onOptions(PI.options);
    }
  };
};

/** The Connection section shared by every AudioRouter key. */
function connectionSection() {
  return `<details class="connection" id="connection">
    <summary>AudioRouter connection <span id="connection-state" class="state">…</span></summary>
    <label>API address<input id="base-url" type="url" placeholder="http://127.0.0.1:PORT" spellcheck="false"></label>
    <label>API token<input id="token" type="password" placeholder="Copy from AudioRouter → API tab" spellcheck="false" autocomplete="off"></label>
    <p class="hint">In AudioRouter, open the API tab, start the API, and copy its address and token. Shared by all AudioRouter keys.</p>
  </details>`;
}

function fillConnection() {
  const url = document.getElementById("base-url");
  const token = document.getElementById("token");
  if (!url || !token) return;
  if (document.activeElement !== url) url.value = PI.global.baseUrl ?? "";
  if (document.activeElement !== token) token.value = PI.global.token ?? "";
}

function showConnectionState() {
  const element = document.getElementById("connection-state");
  const details = document.getElementById("connection");
  if (!element || !PI.options) return;
  const { state, error, session } = PI.options;
  element.textContent = state === "online" ? `connected · ${session ?? ""}` : state === "unconfigured" ? "not set up" : state === "connecting" ? "connecting…" : "offline";
  element.className = `state is-${state}`;
  element.title = error ?? "";
  if (state !== "online" && details) details.open = true;
}

function wireConnection() {
  document.getElementById("connection-slot").innerHTML = connectionSection();
  document.getElementById("base-url").addEventListener("change", (event) => PI.saveGlobal({ baseUrl: event.target.value.trim() }));
  document.getElementById("token").addEventListener("change", (event) => PI.saveGlobal({ token: event.target.value.trim() }));
}

/** Fill a select with options, keeping a saved value that is not in the list. */
function fillSelect(select, items, saved, missingLabel) {
  const current = saved ?? "";
  select.innerHTML = "";
  const placeholder = new Option("Choose…", "");
  select.add(placeholder);
  for (const item of items) select.add(new Option(item.label, item.value));
  if (current && !items.some((item) => item.value === current)) select.add(new Option(missingLabel ?? `${current} (not found)`, current));
  select.value = current;
}

window.addEventListener("DOMContentLoaded", wireConnection);
