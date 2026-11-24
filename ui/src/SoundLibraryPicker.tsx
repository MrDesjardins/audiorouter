// Soundboard sound picker: search and category filter over the bundled library, a
// preview in the browser (never sent to a route), and a way to use the owner's own file.
import { useEffect, useMemo, useRef, useState } from "react";
import type { MethodResult } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { audioUploadProblem } from "./audioUpload";
import { tx } from "./i18n";
import { useMessages } from "./i18n/useMessages";
import { MAX_USER_SOUND_SECONDS } from "./soundImport";

type Library = MethodResult["soundLibrary.list"];
type LibrarySound = Library["sounds"][number];

/**
 * The sounds the picker lists (SND-12): in `category` when one is chosen, matching the
 * search in the title, a tag or the category name, sorted by category and then title.
 */
export function pickerSounds(library: Library | null, query: string, category: string): LibrarySound[] {
  const needle = query.trim().toLowerCase();
  const titles = new Map((library?.categories ?? []).map((item) => [item.id, item.title]));
  const categoryTitle = (id: string) => titles.get(id) ?? id;
  return (library?.sounds ?? [])
    .filter(
      (sound) =>
        (category === "" || sound.category === category) &&
        (needle === "" ||
          sound.title.toLowerCase().includes(needle) ||
          sound.tags.some((tag) => tag.toLowerCase().includes(needle)) ||
          categoryTitle(sound.category).toLowerCase().includes(needle)),
    )
    .sort(
      (left, right) =>
        categoryTitle(left.category).localeCompare(categoryTitle(right.category)) ||
        left.title.localeCompare(right.title),
    );
}

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
  if (!Context || !backend.soundLibraryRead) throw new Error(tx("h1.pickPreviewNA"));
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
  const tr = useMessages();
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
      setProblem(tx("h1.pickLibraryNA"));
      return;
    }
    backend
      .soundLibraryList()
      .then((result) => {
        if (!cancelled) setLibrary(result);
      })
      .catch((error) => {
        if (!cancelled) setProblem(formatUiError(error, tx("h1.pickLibraryFail")));
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
  const shown = useMemo(() => pickerSounds(library, query, category), [library, query, category]);
  const categoryTitle = (id: string) => library?.categories.find((item) => item.id === id)?.title ?? id;
  const preview = (sound: LibrarySound) => {
    setPreviewing(sound.id);
    playPreview(backend, sound.id, audioContext, previewSource)
      .then(() => setProblem(null))
      .catch((error) => setProblem(formatUiError(error, tx("h1.pickPreviewFail"))))
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
          {tr("h1.pickSearch")}
          <input
            ref={search}
            type="search"
            value={query}
            placeholder={tr("h1.pickPlaceholder")}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <label>
          {tr("h1.pickCategory")}
          <select value={category} onChange={(event) => setCategory(event.target.value)}>
            <option value="">{tr("h1.pickAllCategories")}</option>
            {(library?.categories ?? []).map((item) => (
              <option key={item.id} value={item.id}>
                {item.title}
              </option>
            ))}
          </select>
        </label>
        <ul className="sound-picker-list" aria-label={tr("h1.pickBuiltIn")}>
          {shown.map((sound) => (
            <li key={sound.id} className="sound-picker-row">
              <span className="sound-picker-title" title={`${sound.title} · ${categoryTitle(sound.category)}`}>
                {sound.title}
              </span>
              <span className="sound-picker-length">{seconds(sound.durationMs)}</span>
              <button
                type="button"
                className="secondary"
                aria-label={tr("h1.pickPreviewTitle", { title: sound.title })}
                disabled={previewing === sound.id}
                onClick={() => preview(sound)}
              >
                {tr("h1.pickPreview")}
              </button>
              <button
                type="button"
                className="primary"
                aria-label={tr("h1.pickUseTitle", { title: sound.title })}
                disabled={busy}
                onClick={() => onChoose({ id: sound.id, title: sound.title })}
              >
                {tr("h1.pickUse")}
              </button>
            </li>
          ))}
        </ul>
        <p className="sound-picker-status" role="status">
          {problem ??
            (library === null
              ? tr("h1.pickLoading")
              : !library.installed
                ? tr("h1.pickNotInstalled")
                : shown.length === 0
                  ? tr("h1.pickNoMatch")
                  : tr("h1.pickCount", { shown: shown.length, total: library.sounds.length }))}
        </p>
        <label>
          {tr("h1.pickOwnFile", { seconds: MAX_USER_SOUND_SECONDS })}
          <input
            type="file"
            accept="audio/wav,audio/mpeg,.wav,.mp3"
            disabled={busy}
            onChange={(event) => {
              const file = event.target.files?.[0];
              event.currentTarget.value = "";
              if (!file) return;
              const reason = audioUploadProblem(file);
              if (reason) setProblem(reason.text);
              else onChooseFile(file);
            }}
          />
        </label>
        <div className="actions">
          <button type="button" className="secondary" onClick={onClose}>
            {tr("h1.pickClose")}
          </button>
        </div>
      </section>
    </div>
  );
}
