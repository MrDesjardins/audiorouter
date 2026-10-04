import { SingletonAction, type DidReceiveSettingsEvent, type KeyAction, type TitleParametersDidChangeEvent, type WillAppearEvent, type WillDisappearEvent } from "@elgato/streamdeck";
import type { JsonObject } from "@elgato/utils";
import { dataUrl, messageFace } from "../faces.js";
import type { AudioRouterStore } from "../store.js";

/** What the user set in Stream Deck's title field for this key. */
export type KeyTitle = { text: string; shown: boolean };

/** A face, and for two-state keys which state (0 off, 1 on) it shows. */
export type Face = string | { svg: string; state: 0 | 1 };

type Visible<T extends JsonObject> = { action: KeyAction<T>; settings: T; title: KeyTitle; image: string; state: 0 | 1 | null };

/**
 * Keys that draw AudioRouter's state. Tracks visible instances, redraws them
 * when the store changes, and sends an image only when it changed.
 *
 * Titles: with no title the key draws its own label (the tool's name). A
 * title replaces that label; when Stream Deck shows the title itself (its
 * "Show title" option, with the user's font and position) the key leaves
 * the label out so the two never overlap.
 *
 * Two-state keys switch Stream Deck's state from AudioRouter's value, so a
 * user's own image for each state (Stream Deck's image picker) is used.
 */
export abstract class LiveKeyAction<T extends JsonObject> extends SingletonAction<T> {
  private readonly visible = new Map<string, Visible<T>>();
  protected readonly pending = new Set<string>();

  constructor(protected readonly store: AudioRouterStore, private readonly watch: { levels?: boolean; sessions?: boolean; recorders?: boolean } = {}) {
    super();
    store.subscribe(() => this.renderAll());
  }

  /** The face for one key, given its settings, title and the store. */
  protected abstract face(settings: T, actionId: string, title: KeyTitle): Face;

  /** The label to draw: none when Stream Deck draws the title, else the title or `fallback`. */
  protected label(title: KeyTitle, fallback: string): string | null {
    if (title.shown && title.text.trim()) return null;
    return title.text.trim() || fallback;
  }

  override onWillAppear(ev: WillAppearEvent<T>) {
    if (!ev.action.isKey()) return;
    this.visible.set(ev.action.id, { action: ev.action, settings: ev.payload.settings, title: { text: "", shown: false }, image: "", state: null });
    if (this.watch.levels) this.store.watchLevels(1);
    if (this.watch.sessions) this.store.watchSessions(1);
    if (this.watch.recorders) { this.store.watchRecorders(1); this.store.refreshSoon(); }
    this.render(ev.action.id);
  }

  override onWillDisappear(ev: WillDisappearEvent<T>) {
    if (!this.visible.delete(ev.action.id)) return;
    if (this.watch.levels) this.store.watchLevels(-1);
    if (this.watch.sessions) this.store.watchSessions(-1);
    if (this.watch.recorders) this.store.watchRecorders(-1);
  }

  override onDidReceiveSettings(ev: DidReceiveSettingsEvent<T>) {
    const entry = this.visible.get(ev.action.id);
    if (!entry) return;
    entry.settings = ev.payload.settings;
    this.render(ev.action.id);
  }

  override onTitleParametersDidChange(ev: TitleParametersDidChangeEvent<T>) {
    const entry = this.visible.get(ev.action.id);
    if (!entry) return;
    entry.title = { text: ev.payload.title ?? "", shown: ev.payload.titleParameters?.showTitle ?? false };
    this.render(ev.action.id);
  }

  protected settingsOf(actionId: string): T | undefined {
    return this.visible.get(actionId)?.settings;
  }

  /** The face when AudioRouter cannot be used, else null. */
  protected unavailableFace(): string | null {
    if (this.store.state === "unconfigured") return messageFace("Set up", "in key settings");
    if (this.store.state === "connecting") return messageFace("AudioRouter", "connecting…");
    if (this.store.state === "offline" || !this.store.summary) return messageFace("AudioRouter", "offline");
    return null;
  }

  protected render(actionId: string) {
    const entry = this.visible.get(actionId);
    if (!entry) return;
    const face = this.unavailableFace() ?? this.face(entry.settings, actionId, entry.title);
    const { svg, state } = typeof face === "string" ? { svg: face, state: null } : face;
    if (state !== null && state !== entry.state) {
      entry.state = state;
      entry.image = "";
      void entry.action.setState(state).catch(() => {});
    }
    if (svg === entry.image) return;
    entry.image = svg;
    // Per state, so a user's own image for that state is kept by Stream Deck.
    void entry.action.setImage(dataUrl(svg), entry.state === null ? undefined : { state: entry.state }).catch(() => {});
  }

  protected renderAll() {
    for (const actionId of this.visible.keys()) this.render(actionId);
  }

  /** Run a command, showing the pending look until the store confirms. */
  protected async run(action: KeyAction<T>, command: () => Promise<unknown>) {
    this.pending.add(action.id);
    this.render(action.id);
    try {
      await command();
    } catch {
      await action.showAlert().catch(() => {});
    } finally {
      this.pending.delete(action.id);
      this.store.refreshSoon();
      this.render(action.id);
    }
  }
}
