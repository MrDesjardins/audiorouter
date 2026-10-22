// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import {
  MAX_PRESENTATION_JSON_CHARS,
  optionalLocalStorage,
  readPresentationJson,
  writePresentationJson,
} from "./presentationStorage";

afterEach(() => vi.restoreAllMocks());
it("treats a denied browser storage getter as unavailable", () => {
  vi.spyOn(window, "localStorage", "get").mockImplementation(() => {
    throw new DOMException("Denied", "SecurityError");
  });
  expect(optionalLocalStorage()).toBeNull();
  expect(readPresentationJson(optionalLocalStorage(), "key")).toBeNull();
  expect(() => writePresentationJson(optionalLocalStorage(), "key", {})).not.toThrow();
});
it("bounds JSON before parsing and accepts the exact limit", () => {
  const parse = vi.spyOn(JSON, "parse");
  const storage = { getItem: () => " ".repeat(MAX_PRESENTATION_JSON_CHARS - 1) + "{}" };
  expect(readPresentationJson(storage, "key")).toBeNull();
  expect(parse).not.toHaveBeenCalled();
  expect(readPresentationJson({ getItem: () => " ".repeat(MAX_PRESENTATION_JSON_CHARS - 2) + "{}" }, "key")).toEqual(
    {},
  );
});
it("does not overwrite an existing preference with an oversized or unserializable value", () => {
  const setItem = vi.fn();
  writePresentationJson({ setItem }, "key", "x".repeat(MAX_PRESENTATION_JSON_CHARS));
  const cyclic: { value?: unknown } = {};
  cyclic.value = cyclic;
  writePresentationJson({ setItem }, "key", cyclic);
  expect(setItem).not.toHaveBeenCalled();
  writePresentationJson({ setItem }, "key", { x: 1 });
  expect(setItem).toHaveBeenCalledWith("key", '{"x":1}');
});
it("contains read, parse and write exceptions", () => {
  for (const raw of [null, "", "{", "undefined", "\0", "[1,"])
    expect(readPresentationJson({ getItem: () => raw }, "key")).toBeNull();
  expect(
    readPresentationJson(
      {
        getItem: () => {
          throw Error("read");
        },
      },
      "key",
    ),
  ).toBeNull();
  expect(() =>
    writePresentationJson(
      {
        setItem: () => {
          throw Error("quota");
        },
      },
      "key",
      {},
    ),
  ).not.toThrow();
});
