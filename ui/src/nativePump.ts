export type NativePumpKind = "endpoint" | "duplex" | "renderSource" | "multiInput";

/** Select a pump only from an authoritative adapter kind and matching method. */
export function selectNativePump(
  kind: string | null | undefined,
  endpointAvailable: boolean,
  duplexAvailable: boolean,
  renderSourceAvailable = false,
  multiInputAvailable = false,
): NativePumpKind | null {
  if (kind === "duplex") return duplexAvailable ? "duplex" : null;
  if (kind === "endpoint") return endpointAvailable ? "endpoint" : null;
  if (kind === "render-source") return renderSourceAvailable ? "renderSource" : null;
  if (kind === "multi-input") return multiInputAvailable ? "multiInput" : null;
  return null;
}
