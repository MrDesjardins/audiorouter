//! Keep browser commands out of the desktop editor without swallowing DOM keys.
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2Settings3,
};
use windows_core::Interface;

fn disable_browser_keys(controller: &ICoreWebView2Controller) -> Result<(), String> {
    // SAFETY: Tauri supplies a live, reference-counted controller on its UI
    // thread through with_webview. All derived COM interfaces remain local to
    // that callback and are released before it returns. The BOOL out pointer
    // refers to initialized stack storage for the duration of the call.
    unsafe {
        let settings = controller
            .CoreWebView2()
            .and_then(|view| view.Settings())
            .and_then(|settings| settings.cast::<ICoreWebView2Settings3>())
            .map_err(|error| format!("Cannot read browser accelerator settings: {error}"))?;
        settings
            .SetAreBrowserAcceleratorKeysEnabled(false)
            .map_err(|error| format!("Cannot disable browser accelerators: {error}"))?;
        let mut enabled = true.into();
        settings
            .AreBrowserAcceleratorKeysEnabled(&mut enabled)
            .map_err(|error| format!("Cannot verify browser accelerator settings: {error}"))?;
        if enabled.as_bool() {
            return Err("Browser accelerators remained enabled".into());
        }
    }
    Ok(())
}

pub fn configure(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.with_webview(|view| {
        let request = crate::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!("browser-accelerators")),
            method: "shell.browserAccelerators".into(),
            params: None,
        };
        let result = disable_browser_keys(&view.controller()).map(|()| {
            crate::JsonRpcResponse::success(
                request.id.clone(),
                serde_json::json!({"state": "disabled"}),
            )
        });
        // Leave a usable editor on an unsupported runtime, and make failure
        // visible to diagnostics and the packaged regression gate.
        crate::log_shell_rpc(&request, &result);
    })
}
