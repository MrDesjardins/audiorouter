import { tx } from "./i18n";

const STATES = {
  idle: "recordingUi.idle",
  armed: "recordingUi.armed",
  recording: "recordingUi.recordingState",
  paused: "recordingUi.paused",
  stopping: "recordingUi.stopping",
  completed: "recordingUi.completed",
  failed: "recordingUi.failed",
  unavailable: "recordingUi.unavailable",
  partial: "recordingUi.partial",
  recovered: "recordingUi.recovered",
} as const;

export function recordingStateLabel(state: string): string {
  return Object.hasOwn(STATES, state) ? tx(STATES[state as keyof typeof STATES]) : state;
}
