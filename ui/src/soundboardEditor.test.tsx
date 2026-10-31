/** @vitest-environment jsdom */
// Editor, picker and drag-and-drop paths of the Soundboard through the real card, Properties
// editor and controller, with the draft held the way App holds it.
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useRef, useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { MethodResult, Node, Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { removeNodeDraftParameters, setNodeDraftParameter } from "./draft";
import { demoSession } from "./fixtures";
import { assignSoundboardSlotDraft } from "./soundboard";
import { SoundboardNodeControls } from "./SoundboardNodeControls";
import { SoundboardNodeEditor } from "./SoundboardNodeEditor";
import { pickerSounds } from "./SoundLibraryPicker";
import { SoundboardContext, useSoundboardController } from "./soundboardLive";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

type Library = MethodResult["soundLibrary.list"];
const library: Library = {
  installed: true,
  categories: [
    { id: "horns-alerts", title: "Horns and alerts" },
    { id: "bodily-humor", title: "Bodily humor" },
  ],
  sounds: [
    {
      id: "lib:air-horn-01",
      title: "Air horn 01",
      category: "horns-alerts",
      tags: ["horn"],
      durationMs: 1200,
      license: "CC0",
      author: "a",
    },
    {
      id: "lib:fart-02",
      title: "Fart 02",
      category: "bodily-humor",
      tags: ["gross"],
      durationMs: 900,
      license: "CC0",
      author: "b",
    },
    {
      id: "lib:burp-01",
      title: "Burp 01",
      category: "bodily-humor",
      tags: ["gross", "belch"],
      durationMs: 800,
      license: "CC0",
      author: "c",
    },
  ] as Library["sounds"],
};

function boardNode(parameters: Node["parameters"]): Node {
  return {
    id: "board",
    kind: "soundboard",
    typeVersion: 1,
    name: "Soundboard 1",
    enabled: true,
    bypass: false,
    parameters,
    ports: [{ name: "out", direction: "output", channels: 2 }],
  };
}

/**
 * A backend that records every method the UI calls. Any method the test did not provide
 * fails the test, so "preview never reaches a route" is checked, not assumed.
 */
function recordingBackend(overrides: Partial<UiBackend> = {}) {
  const calls: string[] = [];
  const base: Partial<UiBackend> = {
    connected: true,
    soundLibraryList: async () => library,
    soundLibraryRead: async (id: string) => ({ id, format: "flac", dataBase64: "AAAA" }),
    beginAudioUpload: async () => ({ uploadId: "upload-1", chunkBytes: 96 * 1024 }),
    uploadAudioChunk: async () => ({ receivedBytes: 4, nextChunkIndex: 1 }),
    finishAudioUpload: async () => ({
      mediaId: "media-dropped",
      fileName: "horn.wav",
      format: "wav",
      durationMs: 3_000,
      channels: 2,
      sampleRateHz: 48_000,
    }),
    ...overrides,
  };
  const backend = new Proxy(base, {
    get(target, name: string) {
      if (name === "then") return undefined;
      const value = target[name as keyof UiBackend];
      if (typeof value !== "function") {
        if (value === undefined && name !== "connected") throw new Error(`Unexpected backend member: ${name}`);
        return value;
      }
      return (...args: unknown[]) => {
        calls.push(name);
        return (value as (...args: unknown[]) => unknown).apply(target, args);
      };
    },
  }) as UiBackend;
  return { backend, calls };
}

/** Card and Properties of one board, with the draft held as App holds it (latest-draft ref). */
function renderBoard(initial: Node["parameters"], backend: UiBackend) {
  const announced: string[] = [];
  const drafts: Session[] = [];
  function Harness() {
    const [draft, setDraft] = useState<Session>({ ...demoSession, nodes: [boardNode(initial)] });
    const draftRef = useRef(draft);
    draftRef.current = draft;
    const record = (next: Session) => {
      draftRef.current = next;
      drafts.push(next);
      setDraft(next);
    };
    const controller = useSoundboardController({
      backend,
      session: draft,
      sessionRunning: false,
      startSession: async () => true,
      report: () => {},
      assign: (nodeId, slot, sound, keepCaption) =>
        record(assignSoundboardSlotDraft(draftRef.current, nodeId, slot, sound, keepCaption)),
      announce: (message) => announced.push(message),
    });
    const node = draft.nodes[0];
    return (
      <SoundboardContext.Provider value={controller}>
        <section aria-label="canvas">
          <SoundboardNodeControls node={node} />
        </section>
        <section aria-label="properties">
          <SoundboardNodeEditor
            node={node}
            backend={backend}
            disabled={false}
            onChange={(name, value) => record(setNodeDraftParameter(draftRef.current, node.id, name, value))}
            onRemove={(names) => record(removeNodeDraftParameters(draftRef.current, node.id, names))}
          />
        </section>
      </SoundboardContext.Provider>
    );
  }
  const view = render(<Harness />);
  const parameters = () => drafts.at(-1)?.nodes[0].parameters ?? initial;
  return { view, announced, parameters };
}

const properties = () => within(screen.getByRole("region", { name: "properties" }));
const canvas = () => within(screen.getByRole("region", { name: "canvas" }));

function fileDrop(files: File[]) {
  return { dataTransfer: { types: ["Files"], files, dropEffect: "none" } };
}

const wav = (name = "horn.wav", bytes = 4) => new File([new Uint8Array(bytes)], name, { type: "audio/wav" });

describe("choosing and replacing sounds", () => {
  it("Change with a library sound replaces the slot, keeps its caption and clears its random set", async () => {
    const { backend } = recordingBackend();
    const { parameters } = renderBoard({ slot1Set: "bodily-humor", slot1Caption: "Gross", slot1Mode: "loop" }, backend);
    fireEvent.click(properties().getByRole("button", { name: "Change sound 1" }));
    const dialog = await screen.findByRole("dialog", { name: "Change sound 1" });
    fireEvent.click(await within(dialog).findByRole("button", { name: "Use Air horn 01" }));
    expect(parameters()).toEqual({
      slot1MediaId: "lib:air-horn-01",
      slot1FileName: "Air horn 01",
      slot1Caption: "Gross",
      slot1Mode: "loop",
    });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("Add puts a library sound on the first free slot with no caption", async () => {
    const { backend } = recordingBackend();
    const { parameters } = renderBoard({ slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" }, backend);
    fireEvent.click(properties().getByRole("button", { name: "Add sound" }));
    const dialog = await screen.findByRole("dialog", { name: "Add a sound" });
    fireEvent.click(await within(dialog).findByRole("button", { name: "Use Fart 02" }));
    expect(parameters()).toMatchObject({ slot2MediaId: "lib:fart-02", slot2FileName: "Fart 02" });
    expect(canvas().getByRole("button", { name: "Play Fart 02" })).toBeTruthy();
  });

  it("Delete removes every setting of the slot and the card button", async () => {
    const { backend } = recordingBackend();
    const { parameters } = renderBoard(
      {
        slot1MediaId: "lib:burp-01",
        slot1FileName: "Burp 01",
        slot1Caption: "Belch",
        slot1GainDb: -3,
        slot1Mode: "hold",
        slot1Set: "bodily-humor",
        slot2MediaId: "lib:fart-02",
        slot2FileName: "Fart 02",
      },
      backend,
    );
    fireEvent.click(properties().getByRole("button", { name: "Delete sound 1" }));
    expect(parameters()).toEqual({ slot2MediaId: "lib:fart-02", slot2FileName: "Fart 02" });
    expect(canvas().queryByRole("button", { name: "Play Belch" })).toBeNull();
  });

  it("the play mode, caption and level change only their own slot setting", () => {
    const { backend } = recordingBackend();
    const { parameters } = renderBoard({ slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" }, backend);
    fireEvent.change(properties().getByRole("combobox", { name: "When pressed" }), { target: { value: "toggle" } });
    fireEvent.change(properties().getByRole("textbox", { name: "Caption" }), { target: { value: "Oops" } });
    fireEvent.change(properties().getByRole("slider"), { target: { value: "-12" } });
    expect(parameters()).toEqual({
      slot1MediaId: "lib:burp-01",
      slot1FileName: "Burp 01",
      slot1Mode: "toggle",
      slot1Caption: "Oops",
      slot1GainDb: -12,
    });
    expect(canvas().getByRole("button", { name: "Play Oops" })).toBeTruthy();
  });
});

describe("dropping a sound file (SND-19)", () => {
  it("a file dropped on a card button replaces that sound and keeps the caption", async () => {
    const { backend, calls } = recordingBackend();
    const { parameters, announced } = renderBoard(
      { slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01", slot1Caption: "Belch", slot1Set: "bodily-humor" },
      backend,
    );
    const button = canvas().getByRole("button", { name: "Play Belch" });
    fireEvent.dragEnter(button, fileDrop([wav()]));
    expect(button.closest(".soundboard-drop")?.classList.contains("is-drop-target")).toBe(true);
    fireEvent.drop(button, fileDrop([wav()]));
    await waitFor(() => expect(parameters().slot1MediaId).toBe("media-dropped"));
    expect(parameters()).toEqual({ slot1MediaId: "media-dropped", slot1FileName: "horn.wav", slot1Caption: "Belch" });
    expect(calls).toEqual(["beginAudioUpload", "uploadAudioChunk", "finishAudioUpload"]);
    expect(announced.at(-1)).toMatch(/horn\.wav is on button 1/);
    expect(button.closest(".soundboard-drop")?.classList.contains("is-drop-target")).toBe(false);
  });

  it("a file dropped elsewhere on the card or on Properties fills the first free slot", async () => {
    const { backend } = recordingBackend();
    const { parameters } = renderBoard({ slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" }, backend);
    fireEvent.drop(canvas().getByRole("button", { name: /Add sound/ }), fileDrop([wav()]));
    await waitFor(() => expect(parameters().slot2MediaId).toBe("media-dropped"));
    fireEvent.drop(screen.getByLabelText("Soundboard settings"), fileDrop([wav()]));
    await waitFor(() => expect(parameters().slot3MediaId).toBe("media-dropped"));
    expect(await properties().findByText(/horn\.wav is on button 3/)).toBeTruthy();
  });

  it("checks type, size, count and free slots before any upload", async () => {
    const { backend, calls } = recordingBackend();
    const full = Object.fromEntries(
      Array.from({ length: 9 }, (_, index) => [
        [`slot${index + 1}MediaId`, `m${index}`],
        [`slot${index + 1}FileName`, `Sound ${index + 1}`],
      ]).flat(),
    );
    const { announced } = renderBoard({ slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" }, backend);
    const card = canvas().getByLabelText("Soundboard 1 sounds");
    fireEvent.drop(card, fileDrop([new File(["x"], "notes.txt")]));
    fireEvent.drop(card, fileDrop([wav("big.wav", 64 * 1024 * 1024 + 1)]));
    fireEvent.drop(card, fileDrop([wav("a.wav"), wav("b.wav")]));
    expect(announced).toEqual([
      "Choose a .wav or .mp3 file.",
      "Files are limited to 64 MiB.",
      "Drop one sound at a time.",
    ]);
    cleanup();
    const fullBoard = renderBoard(full, backend);
    fireEvent.drop(canvas().getByLabelText("Soundboard 1 sounds"), fileDrop([wav()]));
    expect(fullBoard.announced.at(-1)).toMatch(/All nine buttons have a sound/);
    expect(calls).toEqual([]);
  });

  it("refuses a sound longer than 120 seconds and leaves the slot unchanged", async () => {
    const { backend } = recordingBackend({
      finishAudioUpload: async () => ({
        mediaId: "too-long",
        fileName: "speech.mp3",
        format: "mp3",
        durationMs: 121_000,
        channels: 2,
        sampleRateHz: 48_000,
      }),
    });
    const { announced, parameters } = renderBoard({ slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" }, backend);
    fireEvent.drop(canvas().getByRole("button", { name: "Play Burp 01" }), fileDrop([wav("speech.mp3")]));
    await waitFor(() => expect(announced.at(-1)).toMatch(/longer than 120 seconds/));
    expect(parameters()).toEqual({ slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" });
  });

  it("runs one import at a time", async () => {
    let finish!: () => void;
    const { backend, calls } = recordingBackend({
      beginAudioUpload: () =>
        new Promise((resolve) => {
          finish = () => resolve({ uploadId: "upload-1", chunkBytes: 96 * 1024 });
        }),
    });
    const { announced, parameters } = renderBoard({}, backend);
    const card = canvas().getByLabelText("Soundboard 1 sounds");
    fireEvent.drop(card, fileDrop([wav("first.wav")]));
    fireEvent.drop(card, fileDrop([wav("second.wav")]));
    await waitFor(() => expect(announced[0]).toMatch(/Another sound is still being imported/));
    await act(async () => finish());
    await waitFor(() => expect(parameters().slot1MediaId).toBe("media-dropped"));
    expect(parameters().slot2MediaId).toBeUndefined();
    expect(calls.filter((name) => name === "beginAudioUpload")).toHaveLength(1);
  });

  it("leaves drags that carry no file (a tool from the library) to the canvas", () => {
    const { backend, calls } = recordingBackend();
    renderBoard({ slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" }, backend);
    const card = canvas().getByLabelText("Soundboard 1 sounds");
    const toolDrag = { dataTransfer: { types: ["application/x-audiorouter-node"], files: [] } };
    // fireEvent returns false when a handler called preventDefault (the card took the drop).
    expect(fireEvent.dragOver(card, toolDrag)).toBe(true);
    expect(fireEvent.drop(card, toolDrag)).toBe(true);
    expect(card.classList.contains("is-drop-target")).toBe(false);
    expect(fireEvent.dragOver(card, fileDrop([wav()]))).toBe(false);
    expect(calls).toEqual([]);
  });
});

describe("the sound picker (SND-12)", () => {
  it("searches title, tags and category name, filters by category and sorts by category then title", () => {
    const titles = (query: string, category = "") => pickerSounds(library, query, category).map((sound) => sound.title);
    expect(titles("")).toEqual(["Burp 01", "Fart 02", "Air horn 01"]);
    expect(titles("belch")).toEqual(["Burp 01"]);
    expect(titles("HORNS AND")).toEqual(["Air horn 01"]);
    expect(titles("humor")).toEqual(["Burp 01", "Fart 02"]);
    expect(titles("", "horns-alerts")).toEqual(["Air horn 01"]);
    expect(titles("fart", "horns-alerts")).toEqual([]);
  });

  it("filters in the dialog and previews on this computer without any route call", async () => {
    const started = vi.fn();
    class FakeContext {
      destination = {};
      decodeAudioData = vi.fn(async () => ({}));
      createBufferSource() {
        return { buffer: null, connect: vi.fn(), start: started, stop: vi.fn() };
      }
      close = vi.fn(async () => {});
    }
    vi.stubGlobal("AudioContext", FakeContext);
    const { backend, calls } = recordingBackend();
    renderBoard({ slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" }, backend);
    fireEvent.click(properties().getByRole("button", { name: "Add sound" }));
    const dialog = await screen.findByRole("dialog", { name: "Add a sound" });
    const list = within(dialog).getByRole("list", { name: "Built-in sounds" });
    await within(list).findByText("Air horn 01");
    fireEvent.change(within(dialog).getByRole("combobox", { name: "Category" }), { target: { value: "bodily-humor" } });
    expect(
      within(list)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual([expect.stringContaining("Burp 01"), expect.stringContaining("Fart 02")]);
    fireEvent.change(within(dialog).getByRole("searchbox", { name: "Search sounds" }), { target: { value: "fart" } });
    expect(within(list).getAllByRole("listitem")).toHaveLength(1);
    expect(within(dialog).getByRole("status").textContent).toMatch(
      /1 of 3 sounds\. Previews play on this computer only/,
    );
    fireEvent.click(within(dialog).getByRole("button", { name: "Preview Fart 02" }));
    await waitFor(() => expect(started).toHaveBeenCalledOnce());
    expect(calls).toEqual(["soundLibraryList", "soundLibraryRead"]);
    fireEvent.change(within(dialog).getByRole("searchbox", { name: "Search sounds" }), { target: { value: "zzz" } });
    expect(within(dialog).getByRole("status").textContent).toBe("No sound matches.");
  });

  it("imports the owner's own file through the same checks as a drop", async () => {
    const { backend } = recordingBackend();
    const { parameters } = renderBoard(
      { slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01", slot1Caption: "Belch" },
      backend,
    );
    fireEvent.click(properties().getByRole("button", { name: "Change sound 1" }));
    const dialog = await screen.findByRole("dialog", { name: "Change sound 1" });
    const input = within(dialog).getByLabelText(/Or use your own file/);
    fireEvent.change(input, { target: { files: [new File(["x"], "notes.txt")] } });
    expect(within(dialog).getByRole("status").textContent).toBe("Choose a .wav or .mp3 file.");
    fireEvent.change(input, { target: { files: [wav()] } });
    await waitFor(() => expect(parameters().slot1MediaId).toBe("media-dropped"));
    expect(parameters().slot1Caption).toBe("Belch");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });
});
