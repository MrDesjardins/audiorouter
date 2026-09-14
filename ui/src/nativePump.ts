export type NativePumpKind = "endpoint" | "duplex";

/** Select a pump only from an authoritative adapter kind and matching method. */
export function selectNativePump(
  kind: string | null | undefined,
  endpointAvailable: boolean,
  duplexAvailable: boolean,
): NativePumpKind | null {
  if (kind === "duplex") return duplexAvailable ? "duplex" : null;
  if (kind === "endpoint") return endpointAvailable ? "endpoint" : null;
  return null;
}
