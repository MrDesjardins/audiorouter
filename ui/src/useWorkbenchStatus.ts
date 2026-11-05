import { useEffect, useState } from "react";
import type { McpActivity, McpSetupInfo, McpTunnelStatus, WorkbenchTab } from "./Workbench";
import { safeErrorName } from "./ErrorBoundary";

/** Shell status for the MCP and Logs tabs (moved from AppContent). */
export function useWorkbenchStatus(tab: WorkbenchTab, recordUiDiagnostic: (message: string) => void) {
  const [mcpActivity, setMcpActivity] = useState<McpActivity[]>([]);
  const [mcpSetupInfo, setMcpSetupInfo] = useState<McpSetupInfo | null>(null);
  const [mcpTunnelStatus, setMcpTunnelStatus] = useState<McpTunnelStatus | null>(null);
  const [backendActivity, setBackendActivity] = useState<Record<string, unknown>[]>([]);
  useEffect(() => {
    if ((tab !== "mcp" && tab !== "diagnostics") || !window.__TAURI_INTERNALS__?.invoke) return;
    let active = true;
    const failed = new Set<string>();
    const reportFailure = (category: string, error: unknown) => {
      if (!active || failed.has(category)) return;
      failed.add(category);
      recordUiDiagnostic(`${category} failed (${safeErrorName(error)})`);
    };
    void window.__TAURI_INTERNALS__
      .invoke("mcp_setup_info")
      .then((info) => {
        if (active) {
          failed.delete("MCP setup read");
          setMcpSetupInfo(info as McpSetupInfo);
        }
      })
      .catch((error: unknown) => reportFailure("MCP setup read", error));
    const refreshTunnelStatus = () => {
      void window.__TAURI_INTERNALS__
        ?.invoke?.("mcp_tunnel_status")
        .then((status) => {
          if (active) {
            failed.delete("MCP tunnel status read");
            setMcpTunnelStatus(status as McpTunnelStatus);
          }
        })
        .catch((error: unknown) => reportFailure("MCP tunnel status read", error));
    };
    refreshTunnelStatus();
    void window.__TAURI_INTERNALS__
      .invoke("backend_diagnostics_list")
      .then((items) => {
        if (active && Array.isArray(items)) {
          failed.delete("Backend activity read");
          setBackendActivity(items as Record<string, unknown>[]);
        }
      })
      .catch((error: unknown) => reportFailure("Backend activity read", error));
    const refreshActivity = () => {
      void window.__TAURI_INTERNALS__
        ?.invoke?.("mcp_activity_list")
        .then((items) => {
          if (active && Array.isArray(items)) {
            failed.delete("MCP activity read");
            setMcpActivity(items as McpActivity[]);
          }
        })
        .catch((error: unknown) => reportFailure("MCP activity read", error));
      refreshTunnelStatus();
      void window.__TAURI_INTERNALS__
        ?.invoke?.("backend_diagnostics_list")
        .then((items) => {
          if (active && Array.isArray(items)) {
            failed.delete("Backend activity read");
            setBackendActivity(items as Record<string, unknown>[]);
          }
        })
        .catch((error: unknown) => reportFailure("Backend activity read", error));
    };
    refreshActivity();
    const timer = window.setInterval(refreshActivity, 2000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [tab, recordUiDiagnostic]);
  return { mcpActivity, mcpSetupInfo, mcpTunnelStatus, backendActivity };
}
