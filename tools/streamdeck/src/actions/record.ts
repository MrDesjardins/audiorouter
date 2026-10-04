import { action, type KeyDownEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { formatElapsed, messageFace, recordFace } from "../faces.js";
import { recorderFor, type RecordSettings } from "../recorders.js";
import type { AudioRouterStore } from "../store.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

/**
 * Record / Stop on a Recorder node, like its Record button in AudioRouter:
 * each press starts a new file or stops and saves it. Recording needs the
 * session to be playing.
 */
@action({ UUID: "com.mrdesjardins.audiorouter.record" })
export class RecordAction extends LiveKeyAction<RecordSettings> {
  constructor(store: AudioRouterStore) {
    super(store, { recorders: true });
  }

  protected face(settings: RecordSettings, actionId: string, title: KeyTitle): Face {
    const node = recorderFor(this.store, settings);
    if (!node) return settings.nodeId ? messageFace(settings.nodeName || "Recorder", "not found") : messageFace("Choose", "a Recorder");
    const recording = this.store.isRecording(node.id);
    const since = this.store.recordingSince.get(node.id);
    const elapsed = formatElapsed(since === undefined ? 0 : Math.max(0, Math.floor((Date.now() - since) / 1000)));
    const canStart = (this.store.summary?.playing ?? false) && node.enabled;
    // State 0 = not recording, 1 = recording: users can pick their own image for each.
    return { svg: recordFace({ label: this.label(title, node.name), recording, elapsed, canStart, pending: this.pending.has(actionId) }), state: recording ? 1 : 0 };
  }

  override async onKeyDown(ev: KeyDownEvent<RecordSettings>) {
    const settings = ev.payload.settings;
    const sessionId = this.store.summary?.sessionId;
    const node = recorderFor(this.store, settings);
    if (!sessionId || !node || this.store.state !== "online") {
      await ev.action.showAlert();
      return;
    }
    const press = settings.press ?? "toggle";
    const recording = this.store.isRecording(node.id);
    if ((press === "start" && recording) || (press === "stop" && !recording)) return;
    const stop = press === "stop" || (press === "toggle" && recording);
    if (!stop && !(this.store.summary?.playing ?? false)) {
      // Like the Record button: the Recorder saves what plays through it.
      await ev.action.showAlert();
      return;
    }
    await this.run(ev.action, () => this.store.call(stop ? "recorders.stopRecording" : "recorders.startRecording", { sessionId, nodeId: node.id, idempotencyKey: idempotencyKey(stop ? "record-stop" : "record") }));
  }
}
