/**
 * Dragging a tool from a library (canvas library or the Tools tab) onto the
 * canvas. The kind travels in the drag data and, readable during dragover,
 * as a marker type, so the canvas can draw a live preview node under the
 * pointer before the drop. A small label follows the pointer elsewhere.
 */
import { useEffect, useSyncExternalStore } from "react";

export const LIBRARY_DROP_MIME = "application/x-audiorouter-library-kind";
export const LIBRARY_DROP_TEXT_MIME = "text/plain";
const KIND_MARKER = "application/x-audiorouter-kind-";
const TRANSPARENT_PIXEL = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

type DragState = { kind: string; label: string; x: number; y: number; overCanvas: boolean } | null;
let state: DragState = null;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((listener) => listener());
const store = {
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  get: () => state,
  set(next: DragState) {
    state = next;
    emit();
  },
  patch(update: Partial<NonNullable<DragState>>) {
    if (state) {
      state = { ...state, ...update };
      emit();
    }
  },
};

let blankImage: HTMLImageElement | null = null;

/** Begin a library drag: drag data for the drop, a readable kind marker, and
 * a hidden browser drag image (the app draws its own preview). */
export function startLibraryDrag(
  event: { dataTransfer: DataTransfer; clientX: number; clientY: number },
  kind: string,
  label: string,
) {
  const { dataTransfer } = event;
  dataTransfer.effectAllowed = "copy";
  dataTransfer.setData(LIBRARY_DROP_MIME, kind);
  dataTransfer.setData(LIBRARY_DROP_TEXT_MIME, kind);
  try {
    dataTransfer.setData(`${KIND_MARKER}${kind.toLowerCase()}`, "1");
  } catch {
    /* optional marker */
  }
  if (typeof Image !== "undefined" && typeof dataTransfer.setDragImage === "function") {
    blankImage ??= Object.assign(new Image(), { src: TRANSPARENT_PIXEL });
    dataTransfer.setDragImage(blankImage, 0, 0);
  }
  store.set({ kind, label, x: event.clientX, y: event.clientY, overCanvas: false });
}

export function endLibraryDrag() {
  store.set(null);
}

export function setLibraryDragOverCanvas(overCanvas: boolean) {
  if (state && state.overCanvas !== overCanvas) store.patch({ overCanvas });
}

/** The dragged kind during dragover (drop data is unreadable until drop). */
export function draggedLibraryKind(dataTransfer: DataTransfer | null, knownKinds: readonly string[]): string | null {
  if (state?.kind) return state.kind;
  const marker = Array.from(dataTransfer?.types ?? []).find((type) => type.startsWith(KIND_MARKER));
  if (!marker) return null;
  const lower = marker.slice(KIND_MARKER.length);
  return knownKinds.find((kind) => kind.toLowerCase() === lower) ?? null;
}

export function readLibraryDropKind(dataTransfer: DataTransfer): string {
  return dataTransfer.getData(LIBRARY_DROP_MIME) || dataTransfer.getData(LIBRARY_DROP_TEXT_MIME);
}

/** Floating label that follows the pointer while a tool is dragged outside the canvas. */
export function LibraryDragOverlay() {
  const drag = useSyncExternalStore(store.subscribe, store.get, store.get);
  useEffect(() => {
    if (!drag) return;
    const move = (event: DragEvent) => {
      if (event.clientX || event.clientY) store.patch({ x: event.clientX, y: event.clientY });
    };
    const finish = () => store.set(null);
    document.addEventListener("dragover", move);
    document.addEventListener("dragend", finish);
    document.addEventListener("drop", finish);
    return () => {
      document.removeEventListener("dragover", move);
      document.removeEventListener("dragend", finish);
      document.removeEventListener("drop", finish);
    };
  }, [drag !== null]); // eslint-disable-line react-hooks/exhaustive-deps
  if (!drag || drag.overCanvas) return null;
  return (
    <div
      className="library-drag-chip"
      aria-hidden="true"
      style={{ transform: `translate(${drag.x + 14}px, ${drag.y + 12}px)` }}
    >
      <span className="library-drag-chip-plus">+</span>
      {drag.label}
      <small>Drop on the canvas</small>
    </div>
  );
}
