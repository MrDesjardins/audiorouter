import { useEffect, useState } from "react";
import type { McpActivity, McpSetupInfo, McpTunnelStatus, WorkbenchTab } from "./Workbench";

/** Shell status for the MCP and Logs tabs (moved from AppContent). */
export function useWorkbenchStatus(tab: WorkbenchTab, recordUiDiagnostic: (message: string) => void) {
  const [mcpActivity, setMcpActivity] = useState<McpActivity[]>([]);
  const [mcpSetupInfo, setMcpSetupInfo] = useState<McpSetupInfo | null>(null);
  const [mcpTunnelStatus, setMcpTunnelStatus] = useState<McpTunnelStatus | null>(null);
  const [backendActivity, setBackendActivity] = useState<Record<string, unknown>[]>([]);
  useEffect(() => {
    if ((tab !== "mcp" && tab !== "diagnostics") || !window.__TAURI_INTERNALS__?.invoke) return;
    let active = true;
    void window.__TAURI_INTERNALS__
      .invoke("mcp_setup_info")
      .then((info) => {
        if (active) setMcpSetupInfo(info as McpSetupInfo);
      })
      .catch(() => {
        if (active) setMcpSetupInfo(null);
      });
    const refreshTunnelStatus = () => {
      void window.__TAURI_INTERNALS__
        ?.invoke?.("mcp_tunnel_status")
        .then((status) => {
          if (active) setMcpTunnelStatus(status as McpTunnelStatus);
        })
        .catch(() => {
          if (active) setMcpTunnelStatus(null);
        });
    };
    refreshTunnelStatus();
    void window.__TAURI_INTERNALS__
      .invoke("backend_diagnostics_list")
      .then((items) => {
        if (active && Array.isArray(items)) setBackendActivity(items as Record<string, unknown>[]);
      })
      .catch(() => {
        if (active) setBackendActivity([]);
      });
    const refreshActivity = () => {
      void window.__TAURI_INTERNALS__
        ?.invoke?.("mcp_activity_list")
        .then((items) => {
          if (active && Array.isArray(items)) setMcpActivity(items as McpActivity[]);
        })
        .catch((error: unknown) => {
          if (active) {
            const type = error instanceof Error ? error.name.slice(0, 48) : typeof error;
            recordUiDiagnostic(`MCP activity read failed (${type})`);
          }
        });
      refreshTunnelStatus();
      void window.__TAURI_INTERNALS__
        ?.invoke?.("backend_diagnostics_list")
        .then((items) => {
          if (active && Array.isArray(items)) setBackendActivity(items as Record<string, unknown>[]);
        })
        .catch(() => undefined);
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
