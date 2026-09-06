import type { NodeKind } from "@audiorouter/contracts";

export type LibraryEntry = {
  id: string;
  label: string;
  category: string;
  kind?: Extract<NodeKind, "mixer" | "gain" | "mute" | "meter">;
  unavailableReason?: string;
};

export const libraryEntries: LibraryEntry[] = [
  { id: "physical-input", label: "Physical input", category: "Source", unavailableReason: "Requires the M02 Windows audio adapter" },
  { id: "gain", label: "Gain", category: "Effect", kind: "gain" },
  { id: "mixer", label: "Mixer", category: "Routing", kind: "mixer" },
  { id: "recorder", label: "Recorder", category: "Output", unavailableReason: "Requires the M04 runtime integration" },
  { id: "mute", label: "Mute", category: "Effect", kind: "mute" },
  { id: "meter", label: "Meter", category: "Monitor", kind: "meter" },
];

export function filterLibraryEntries(entries: LibraryEntry[], query: string): LibraryEntry[] {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) return entries;
  return entries.filter((entry) => `${entry.label} ${entry.category} ${entry.unavailableReason ?? ""}`.toLocaleLowerCase().includes(normalized));
}
