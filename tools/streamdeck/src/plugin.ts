import streamDeck from "@elgato/streamdeck";
import { normalizeBaseUrl } from "./api.js";
import { MeterAction } from "./actions/meter.js";
import { PlayAction } from "./actions/play.js";
import { PrivacyAction } from "./actions/privacy.js";
import { RecordAction } from "./actions/record.js";
import { SessionAction } from "./actions/session.js";
import { StepAction } from "./actions/step.js";
import { ToggleAction } from "./actions/toggle.js";
import { AudioRouterStore, toggleTargets } from "./store.js";

/** Saved once for the whole plugin: where AudioRouter's API listens, and its token. */
type GlobalSettings = { baseUrl?: string; token?: string };

const store = new AudioRouterStore();

function applyConnection(settings: GlobalSettings) {
  const baseUrl = normalizeBaseUrl(settings.baseUrl ?? "");
  const token = (settings.token ?? "").trim();
  store.configure(baseUrl && token ? { baseUrl, token } : null);
}

/** What a settings panel needs: connection state, tools and their toggleable settings. */
function panelOptions() {
  return {
    event: "options",
    state: store.state,
    error: store.error,
    session: store.summary?.name ?? null,
    sessions: store.sessions,
    nodes: (store.summary?.nodes ?? []).map((node) => ({
      id: node.id,
      name: node.name,
      kind: store.kind(node.kind)?.name ?? node.kind,
      type: node.kind,
      targets: toggleTargets(store.kind(node.kind)),
      numbers: (store.kind(node.kind)?.parameters ?? [])
        .filter((spec) => spec.type === "number")
        .map((spec) => ({ name: spec.name, unit: spec.unit ?? "", minimum: spec.minimum ?? null, maximum: spec.maximum ?? null })),
    })),
  };
}

streamDeck.settings.onDidReceiveGlobalSettings<GlobalSettings>((ev) => applyConnection(ev.settings));

// Settings panels ask for options when they open; answer again whenever the
// store changes while a panel is open, so lists fill as soon as it connects.
let panelOpen = false;
streamDeck.ui.onSendToPlugin<{ event?: string }>((ev) => {
  if (ev.payload && typeof ev.payload === "object" && ev.payload.event === "getOptions") {
    panelOpen = true;
    void streamDeck.ui.sendToPropertyInspector(panelOptions());
  }
});
store.subscribe(() => {
  if (panelOpen) void streamDeck.ui.sendToPropertyInspector(panelOptions()).catch(() => { panelOpen = false; });
});

streamDeck.actions.registerAction(new ToggleAction(store));
streamDeck.actions.registerAction(new MeterAction(store));
streamDeck.actions.registerAction(new PrivacyAction(store));
streamDeck.actions.registerAction(new PlayAction(store));
streamDeck.actions.registerAction(new SessionAction(store));
streamDeck.actions.registerAction(new StepAction(store));
streamDeck.actions.registerAction(new RecordAction(store));

await streamDeck.connect();
applyConnection(await streamDeck.settings.getGlobalSettings<GlobalSettings>());
