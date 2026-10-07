// Contexts shared by the workspace and its panels (moved from App.tsx).
import { createContext } from "react";
import type { PluginParametersResult } from "@audiorouter/contracts";
import { createDisconnectedBackend, type UiBackend } from "./backend";

export const defaultBackend = createDisconnectedBackend();

export const EqBackendContext = createContext<UiBackend>(defaultBackend);

export const PluginParameterContext = createContext<{
  parameters: PluginParametersResult | null;
  error: string | null;
}>({
  parameters: null,
  error: null,
});
