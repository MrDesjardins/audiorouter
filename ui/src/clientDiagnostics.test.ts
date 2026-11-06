import { describe, expect, it } from "vitest";
import {
  appendClientDiagnostic,
  CLIENT_DIAGNOSTICS_STORAGE_KEY,
  describePromiseRejection,
  describeWindowError,
  isNewUiErrorRow,
  readClientDiagnostics,
  resetRpcFailureRepeats,
} from "./clientDiagnostics";

const windowError = (overrides: Partial<Parameters<typeof describeWindowError>[0]>) => ({
  message: "",
  error: null,
  filename: "",
  lineno: 0,
  colno: 0,
  ...overrides,
});

describe("uncaught error rows", () => {
  it("does not report the browser's resize notice as an error", () => {
    // What the packaged app received: no error object, and a page address with no file name.
    expect(
      describeWindowError(
        windowError({
          message: "ResizeObserver loop completed with undelivered notifications.",
          filename: "http://tauri.localhost/",
        }),
      ),
    ).toBeNull();
  });

  it("says what went wrong without the error's own text", () => {
    const row = describeWindowError(
      windowError({
        message: "Uncaught TypeError: Cannot read properties of undefined (reading 'C:\\private\\voice.wav')",
        error: new TypeError("Cannot read properties of undefined (reading 'C:\\private\\voice.wav')"),
        filename: "http://tauri.localhost/assets/index-abc.js?x=1",
        lineno: 12,
        colno: 34,
      }),
    );
    expect(row).toBe("UI error (TypeError) at index-abc.js:12:34: used a value that was missing");
    expect(row).not.toContain("voice.wav");
  });

  it("keeps a React error's number, which identifies it", () => {
    expect(
      describeWindowError(
        windowError({
          message: "Minified React error #185; visit https://react.dev/errors/185",
          error: new Error("x"),
        }),
      ),
    ).toBe("UI error (Error): React error 185");
  });

  it("never logs unrecognized text", () => {
    const secret = "my private session name";
    expect(describeWindowError(windowError({ message: secret }))).toBe("UI error: the browser gave no details");
    expect(describeWindowError(windowError({ message: secret, error: new RangeError(secret) }))).toBe(
      "UI error (RangeError)",
    );
    expect(describePromiseRejection(new Error(secret))).toBe("Unhandled promise rejection (Error)");
    expect(describePromiseRejection(secret)).toBe("Unhandled promise rejection (string)");
  });

  it("describes a rejection it recognizes", () => {
    expect(describePromiseRejection(new TypeError("Failed to fetch dynamically imported module: x.js"))).toBe(
      "Unhandled promise rejection (TypeError): a part of the interface failed to load",
    );
  });

  it("logs a repeating error once in 30 seconds", () => {
    resetRpcFailureRepeats();
    expect(isNewUiErrorRow("UI error (TypeError)", 1_000)).toBe(true);
    expect(isNewUiErrorRow("UI error (TypeError)", 20_000)).toBe(false);
    expect(isNewUiErrorRow("UI error (RangeError)", 20_000)).toBe(true);
    expect(isNewUiErrorRow("UI error (TypeError)", 31_000)).toBe(true);
  });

  it("drops the old rows that say nothing and keeps the rest", () => {
    const storage = memoryStorage();
    storage.setItem(
      CLIENT_DIAGNOSTICS_STORAGE_KEY,
      JSON.stringify([
        "7:20:21 PM UI error (unknown)",
        "7:20:20 PM UI error (TypeError) at a.js:1:2",
        "7:20:19 PM Graph checkpoint: 1 nodes",
      ]),
    );
    expect(readClientDiagnostics(storage)).toEqual([
      "7:20:20 PM UI error (TypeError) at a.js:1:2",
      "7:20:19 PM Graph checkpoint: 1 nodes",
    ]);
  });
});

function memoryStorage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
    clear: () => values.clear(),
  };
}

describe("client diagnostics persistence", () => {
  it("retains bounded entries across reloads", () => {
    const storage = memoryStorage();
    storage.clear();
    let rows: string[] = [];
    for (let index = 0; index < 90; index += 1) {
      rows = appendClientDiagnostic(storage, rows, `Graph checkpoint: ${index} nodes`, `t${index}`);
    }

    expect(rows).toHaveLength(80);
    expect(rows[0]).toBe("t89 Graph checkpoint: 89 nodes");
    expect(readClientDiagnostics(storage)).toEqual(rows);
    expect(storage.getItem(CLIENT_DIAGNOSTICS_STORAGE_KEY)!.length).toBeLessThan(30_000);
  });

  it("truncates entries and tolerates malformed or unavailable storage", () => {
    const storage = memoryStorage();
    storage.setItem(CLIENT_DIAGNOSTICS_STORAGE_KEY, "not json");
    expect(readClientDiagnostics(storage)).toEqual([]);
    expect(appendClientDiagnostic(null, [], "x".repeat(500), "time")[0]).toHaveLength(320);
    expect(readClientDiagnostics(null)).toEqual([]);
  });
});
