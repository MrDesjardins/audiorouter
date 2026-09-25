import { describe, expect, it } from "vitest";
import { appendClientDiagnostic, CLIENT_DIAGNOSTICS_STORAGE_KEY, readClientDiagnostics } from "./clientDiagnostics";

function memoryStorage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => { values.set(key, value); },
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
