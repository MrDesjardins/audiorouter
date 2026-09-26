//! Native host windows for plugin editors.
//!
//! A plugin's own editor is created by its isolated worker process as a child
//! of a window owned by this shell. Each editor gets a plain top-level window
//! on its own message-loop thread. Closing the window first runs the caller's
//! `on_close` hook (which asks the backend to close the editor and apply its
//! edits), then destroys the window.

use std::sync::mpsc;
use std::thread;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassW,
    SetWindowLongPtrW, TranslateMessage, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW,
    MSG, WM_CLOSE, WM_NCCREATE, WM_NCDESTROY, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

const WINDOW_CLASS: PCWSTR = w!("AudioRouterPluginEditorHost");

type CloseHook = Box<dyn FnOnce() + Send>;

/// Open an editor host window and return its handle as an integer (window
/// handles are not `Send`). The window starts at `width` × `height` client
/// pixels and can be resized; the plugin draws inside it.
pub fn open_host_window(title: &str, width: i32, height: i32, on_close: CloseHook) -> Result<isize, String> {
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let title = title.to_owned();
    thread::Builder::new()
        .name("audiorouter-plugin-editor".into())
        .spawn(move || run_window(&title, width, height, on_close, ready_sender))
        .map_err(|error| format!("editor window thread failed: {error}"))?;
    ready_receiver
        .recv()
        .map_err(|_| "editor window thread exited before creating its window".to_owned())?
}

/// Close an editor host window from another thread (for example when the
/// backend refused to open the editor).
pub fn close_host_window(window: isize) {
    // SAFETY: posting to a window handle that no longer exists fails
    // harmlessly; no memory is shared through this message.
    let _ = unsafe { PostMessageW(Some(HWND(window as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0)) };
}

fn run_window(
    title: &str,
    width: i32,
    height: i32,
    on_close: CloseHook,
    ready: mpsc::SyncSender<Result<isize, String>>,
) {
    let created = unsafe { create_window(title, width, height, on_close) };
    let window = match created {
        Ok(window) => window,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    let _ = ready.send(Ok(window.0 as isize));
    let mut message = MSG::default();
    // SAFETY: a standard message loop for windows created on this thread.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe fn create_window(title: &str, width: i32, height: i32, on_close: CloseHook) -> Result<HWND, String> {
    let instance = GetModuleHandleW(None).map_err(|error| format!("GetModuleHandleW failed: {error}"))?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: WINDOW_CLASS,
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        ..Default::default()
    };
    // A second registration fails with "class already exists", which is fine.
    RegisterClassW(&class);
    // Ownership of the hook passes to the window (GWLP_USERDATA) and is
    // reclaimed exactly once, in WM_CLOSE or WM_NCDESTROY.
    let hook = Box::into_raw(Box::new(Some(on_close)));
    let title = HSTRING::from(title);
    CreateWindowExW(
        Default::default(),
        WINDOW_CLASS,
        &title,
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        width.clamp(200, 4096),
        height.clamp(150, 4096),
        None,
        None,
        Some(instance.into()),
        Some(hook.cast()),
    )
    .map_err(|error| {
        drop(Box::from_raw(hook));
        format!("CreateWindowExW failed: {error}")
    })
}

unsafe extern "system" fn window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_NCCREATE => {
            // SAFETY: lparam is the CREATESTRUCTW for this window; its
            // lpCreateParams is the hook pointer passed to CreateWindowExW.
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
            DefWindowProcW(window, message, wparam, lparam)
        }
        WM_CLOSE => {
            let hook = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Option<CloseHook>;
            if !hook.is_null() {
                // SAFETY: the pointer came from Box::into_raw in create_window
                // and stays valid until WM_NCDESTROY frees it.
                if let Some(on_close) = (*hook).take() {
                    on_close();
                }
            }
            let _ = DestroyWindow(window);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let hook = SetWindowLongPtrW(window, GWLP_USERDATA, 0) as *mut Option<CloseHook>;
            if !hook.is_null() {
                // SAFETY: reclaim the Box exactly once; the pointer is cleared above.
                drop(Box::from_raw(hook));
            }
            PostQuitMessage(0);
            DefWindowProcW(window, message, wparam, lparam)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}
