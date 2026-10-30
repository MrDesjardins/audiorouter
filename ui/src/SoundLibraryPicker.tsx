// Soundboard sound picker: search and category filter over the bundled library, a
// preview in the browser (never sent to a route), and a way to use the owner's own file.
import { useEffect, useMemo, useRef, useState } from "react";
import type { MethodResult } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { audioUploadProblem } from "./audioUpload";

type Library = MethodResult["soundLibrary.list"];
type LibrarySound = Library["sounds"][number];

export const MAX_USER_SOUND_SECONDS = 120;

function seconds(durationMs: number): string {
  return `${(durationMs / 1000).toFixed(1)} s`;
}

/** Decodes and plays library bytes through the page's own audio output. */
async function playPreview(
  backend: UiBackend,
  id: string,
  context: { current: AudioContext | null },
  source: { current: AudioBufferSourceNode | null },
): Promise<void> {
  const Context = globalThis.AudioContext;
  if (!Context || !backend.soundLibraryRead) throw new Error("Preview is not available here.");
  const read = await backend.soundLibraryRead(id);
  const binary = atob(read.dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
  context.current ??= new Context();
  const audio = context.current;
  const buffer = await audio.decodeAudioData(bytes.buffer);
  source.current?.stop();
  const next = audio.createBufferSource();
  next.buffer = buffer;
  next.connect(audio.destination);
  next.start();
  source.current = next;
}

export function SoundLibraryPicker({
  backend,
  title,
  busy,
  onChoose,
  onChooseFile,
  onClose,
}: {
  backend: UiBackend;
  title: string;
  busy: boolean;
  onChoose: (sound: { id: string; title: string }) => void;
  onChooseFile: (file: File) => void;
  onClose: () => void;
}) {
  const [library, setLibrary] = useState<Library | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("");
  const [previewing, setPreviewing] = useState<string | null>(null);
  const audioContext = useRef<AudioContext | null>(null);
  const previewSource = useRef<AudioBufferSourceNode | null>(null);
  const search = useRef<HTMLInputElement>(null);
  useEffect(() => search.current?.focus(), []);
  useEffect(() => {
    let cancelled = false;
    if (!backend.soundLibraryList) {
      setProblem("The sound library is not available.");
      return;
    }
    backend
      .soundLibraryList()
      .then((result) => {
        if (!cancelled) setLibrary(result);
      })
      .catch((error) => {
        if (!cancelled) setProblem(formatUiError(error, "The sound library could not be read."));
      });
    return () => {
      cancelled = true;
    };
  }, [backend]);
  useEffect(
    () => () => {
      previewSource.current?.stop();
      void audioContext.current?.close();
    },
    [],
  );
  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return (library?.sounds ?? []).filter(
      (sound) =>
        (category === "" || sound.category === category) &&
        (needle === "" ||
          sound.title.toLowerCase().includes(needle) ||
          sound.tags.some((tag) => tag.toLowerCase().includes(needle))),
    );
  }, [library, query, category]);
  const categoryTitle = (id: string) => library?.categories.find((item) => item.id === id)?.title ?? id;
  const preview = (sound: LibrarySound) => {
    setPreviewing(sound.id);
    playPreview(backend, sound.id, audioContext, previewSource)
      .then(() => setProblem(null))
      .catch((error) => setProblem(formatUiError(error, "The sound could not be previewed.")))
      .finally(() => setPreviewing(null));
  };
  return (
    <div
      className="dialog-backdrop"
      role="presentation"
      onKeyDown={(event) => {
        if (event.key === "Escape") onClose();
      }}
    >
      <section
        className="connection-dialog sound-picker"
        role="dialog"
        aria-modal="true"
        aria-labelledby="sound-picker-heading"
      >
        <h2 id="sound-picker-heading">{title}</h2>
        <label>
          Search sounds
          <input
            ref={search}
            type="search"
            value={query}
            placeholder="fart, applause, ta-da…"
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <label>
          Category
          <select value={category} onChange={(event) => setCategory(event.target.value)}>
            <option value="">All categories</option>
            {(library?.categories ?? []).map((item) => (
              <option key={item.id} value={item.id}>
                {item.title}
              </option>
            ))}
          </select>
        </label>
        <ul className="sound-picker-list" aria-label="Built-in sounds">
          {shown.map((sound) => (
            <li key={sound.id} className="sound-picker-row">
              <span className="sound-picker-title" title={`${sound.title} · ${categoryTitle(sound.category)}`}>
                {sound.title}
              </span>
              <span className="sound-picker-length">{seconds(sound.durationMs)}</span>
              <button
                type="button"
                className="secondary"
                aria-label={`Preview ${sound.title}`}
                disabled={previewing === sound.id}
                onClick={() => preview(sound)}
              >
                Preview
              </button>
              <button
                type="button"
                className="primary"
                aria-label={`Use ${sound.title}`}
                disabled={busy}
                onClick={() => onChoose({ id: sound.id, title: sound.title })}
              >
                Use
              </button>
            </li>
          ))}
        </ul>
        <p className="sound-picker-status" role="status">
          {problem ??
            (library === null
              ? "Loading the sound library…"
              : !library.installed
                ? "The built-in sounds are not installed with this copy of AudioRouter."
                : shown.length === 0
                  ? "No sound matches."
                  : `${shown.length} of ${library.sounds.length} sounds. Previews play on this computer only, not in your route.`)}
        </p>
        <label>
          Or use your own file (WAV or MP3, up to {MAX_USER_SOUND_SECONDS} seconds)
          <input
            type="file"
            accept="audio/wav,audio/mpeg,.wav,.mp3"
            disabled={busy}
            onChange={(event) => {
              const file = event.target.files?.[0];
              event.currentTarget.value = "";
              if (!file) return;
              const reason = audioUploadProblem(file);
              if (reason) setProblem(reason);
              else onChooseFile(file);
            }}
          />
        </label>
        <div className="actions">
          <button type="button" className="secondary" onClick={onClose}>
            Close
          </button>
        </div>
      </section>
    </div>
  );
}
