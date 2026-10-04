import { SingletonAction, type DidReceiveSettingsEvent, type KeyAction, type WillAppearEvent, type WillDisappearEvent } from "@elgato/streamdeck";
import type { JsonObject } from "@elgato/utils";
import { dataUrl, messageFace } from "../faces.js";
import type { AudioRouterStore } from "../store.js";

/**
 * Keys that draw AudioRouter's state. Tracks visible instances, redraws them
 * when the store changes, and sends an image only when it changed.
 */
export abstract class LiveKeyAction<T extends JsonObject> extends SingletonAction<T> {
  private readonly visible = new Map<string, { action: KeyAction<T>; settings: T; image: string }>();
  protected readonly pending = new Set<string>();

  constructor(protected readonly store: AudioRouterStore, private readonly showsLevels = false) {
    super();
    store.subscribe(() => this.renderAll());
  }

  /** The SVG for one key, given its settings and the store. */
  protected abstract face(settings: T, actionId: string): string;

  override onWillAppear(ev: WillAppearEvent<T>) {
    if (!ev.action.isKey()) return;
    this.visible.set(ev.action.id, { action: ev.action, settings: ev.payload.settings, image: "" });
    if (this.showsLevels) this.store.watchLevels(1);
    this.render(ev.action.id);
  }

  override onWillDisappear(ev: WillDisappearEvent<T>) {
    if (this.visible.delete(ev.action.id) && this.showsLevels) this.store.watchLevels(-1);
  }

  override onDidReceiveSettings(ev: DidReceiveSettingsEvent<T>) {
    const entry = this.visible.get(ev.action.id);
    if (!entry) return;
    entry.settings = ev.payload.settings;
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
    const svg = this.unavailableFace() ?? this.face(entry.settings, actionId);
    if (svg === entry.image) return;
    entry.image = svg;
    void entry.action.setImage(dataUrl(svg)).catch(() => {});
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
