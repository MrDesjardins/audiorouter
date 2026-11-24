//! Native host windows for plugin editors.
//!
//! A plugin's own editor is created by its isolated worker process as a child
//! of a window owned by this shell. Each editor gets a plain top-level window
//! on its own message-loop thread.
//!
//! The window stays hidden until the plugin has put its editor inside it (the
//! worker sizes the window to the editor first), so the user never sees an
//! empty frame of the wrong size.
//!
//! Closing is ordered: the window is hidden, the caller's `on_close` hook
//! (which asks the backend to close the editor) runs on another thread while
//! this window's thread keeps handling messages, and only then is the window
//! destroyed. The plugin therefore removes its editor from a live parent that
//! still answers messages. Destroying the parent first, or blocking its
//! thread during the request, crashed or froze plugins on close (issue #136).

use std::sync::{mpsc, Arc};
use std::thread;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, GetWindow,
    GetWindowLongPtrW, IsWindow, LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage,
    CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, GW_CHILD, IDC_ARROW, MSG, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_HIDE, SW_SHOW, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_DPICHANGED, WM_NCCREATE,
    WM_NCDESTROY, WM_SETFOCUS, WM_TIMER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

const WINDOW_CLASS: PCWSTR = w!("AudioRouterPluginEditorHost");
/// Posted by the close thread once the backend has attempted to close the editor.
const CLOSE_FINISHED: u32 = WM_APP + 1;
const WATCH_TIMER: usize = 1;
const WATCH_INTERVAL_MS: u32 = 50;

type CloseHook = Arc<dyn Fn() -> bool + Send + Sync>;

/// What one watch tick should do with the host window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WatchAction {
    None,
    Show,
    Close,
}

/// Decides when the host window appears and when it closes by itself, from
/// whether the plugin's editor (a child window) is inside it.
#[derive(Debug, Default)]
struct EditorWatch {
    shown: bool,
    seen_child: bool,
    waited_ticks: u32,
    missing_ticks: u32,
}

impl EditorWatch {
    /// Show the window anyway after this long without an editor, so a slow
    /// or failing plugin does not leave the user looking at nothing.
    const SHOW_ANYWAY_TICKS: u32 = 3_000 / WATCH_INTERVAL_MS;
    /// An editor that stays away this long is gone (its worker stopped or
    /// crashed). Shorter gaps are a plugin rebuilding its own window.
    const GONE_TICKS: u32 = 1_500 / WATCH_INTERVAL_MS;

    fn tick(&mut self, has_child: bool) -> WatchAction {
        if has_child {
            self.seen_child = true;
            self.missing_ticks = 0;
            if !self.shown {
                self.shown = true;
                return WatchAction::Show;
            }
            return WatchAction::None;
        }
        if self.seen_child {
            self.missing_ticks = self.missing_ticks.saturating_add(1);
            if self.missing_ticks >= Self::GONE_TICKS {
                return WatchAction::Close;
            }
            return WatchAction::None;
        }
        self.waited_ticks += 1;
        if !self.shown && self.waited_ticks >= Self::SHOW_ANYWAY_TICKS {
            self.shown = true;
            return WatchAction::Show;
        }
        WatchAction::None
    }

    fn close_tick(&mut self, has_child: bool) -> WatchAction {
        if !has_child {
            return WatchAction::Close;
        }
        self.missing_ticks = self.missing_ticks.saturating_add(1);
        if self.missing_ticks >= Self::GONE_TICKS {
            self.missing_ticks = 0;
            WatchAction::Show
        } else {
            WatchAction::None
        }
    }
}

struct HostState {
    on_close: CloseHook,
    closing: bool,
    close_result: Option<bool>,
    watch_timer_active: bool,
    watch: EditorWatch,
}

/// Open an editor host window and return its handle as an integer (window
/// handles are not `Send`). The window starts hidden at `width` × `height`;
/// the plugin's worker sizes it to the editor, and it is shown once the
/// editor is inside it.
pub fn open_host_window(
    title: &str,
    width: i32,
    height: i32,
    on_close: CloseHook,
) -> Result<isize, String> {
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
    // SAFETY: called on this dedicated thread, which then runs the window's
    // message loop and frees `state` below; see `create_window`.
    let created = unsafe { create_window(title, width, height, on_close) };
    let (window, state) = match created {
        Ok(created) => created,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    // SAFETY: the window belongs to this thread; the timer is removed with it.
    let timer = unsafe { SetTimer(Some(window), WATCH_TIMER, WATCH_INTERVAL_MS, None) };
    // SAFETY: this thread owns the live window, and `state` is its sole,
    // window-owned GWLP_USERDATA allocation until WM_NCDESTROY.
    unsafe { (*state).watch_timer_active = timer != 0 };
    if timer == 0 {
        // Without polling, the host could stay hidden or be destroyed while
        // the plugin still owns a child window. Prefer a visible, closable
        // host over a hidden window with unsafe teardown.
        // SAFETY: this thread owns the live window and is in its initialization.
        let _ = unsafe { ShowWindow(window, SW_SHOW) };
    }
    let _ = ready.send(Ok(window.0 as isize));
    let mut message = MSG::default();
    // SAFETY: a standard message loop for windows created on this thread.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        // SAFETY: `message` was just filled by GetMessageW; dispatch runs the
        // window procedure of a window owned by this thread.
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    // SAFETY: the loop normally ends with the WM_QUIT that WM_NCDESTROY posts,
    // so the window is gone; if it ended early, destroy the window (owned by
    // this thread) first. Either way WM_NCDESTROY has detached `state` from
    // the window, so nothing else can reach it and it is reclaimed once.
    unsafe {
        if IsWindow(Some(window)).as_bool() {
            let _ = DestroyWindow(window);
        }
        drop(Box::from_raw(state));
    }
}

/// Create the editor host window on the calling thread.
///
/// The returned state pointer is read by the window procedure through
/// GWLP_USERDATA. The caller owns it and frees it with `Box::from_raw` only
/// after the window is destroyed; the window procedure never frees it. That
/// keeps one owner even when CreateWindowExW fails after WM_NCCREATE
/// (Windows then sends WM_NCDESTROY before returning).
///
/// # Safety
/// Call on the thread that will run the window's message loop.
unsafe fn create_window(
    title: &str,
    width: i32,
    height: i32,
    on_close: CloseHook,
) -> Result<(HWND, *mut HostState), String> {
    let instance =
        GetModuleHandleW(None).map_err(|error| format!("GetModuleHandleW failed: {error}"))?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: WINDOW_CLASS,
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        // Paint the area the editor does not cover; without a brush Windows
        // leaves whatever was on screen there.
        hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
        ..Default::default()
    };
    // A second registration fails with "class already exists", which is fine.
    RegisterClassW(&class);
    // The window borrows the state through GWLP_USERDATA; `run_window` frees
    // the box after the window is gone.
    let state = Box::into_raw(Box::new(HostState {
        on_close,
        closing: false,
        close_result: None,
        watch_timer_active: false,
        watch: EditorWatch::default(),
    }));
    let title = HSTRING::from(title);
    CreateWindowExW(
        Default::default(),
        WINDOW_CLASS,
        &title,
        // Hidden until the editor is inside (see `EditorWatch`).
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        width.clamp(200, 4096),
        height.clamp(150, 4096),
        None,
        None,
        Some(instance.into()),
        Some(state.cast()),
    )
    .map(|window| (window, state))
    .map_err(|error| {
        // No window exists (any partially created one has already received
        // WM_NCDESTROY, which only detaches the state), so free it once here.
        drop(Box::from_raw(state));
        format!("CreateWindowExW failed: {error}")
    })
}

/// `WM_ACTIVATE` carries WA_INACTIVE (0), WA_ACTIVE (1) or WA_CLICKACTIVE (2) in
/// the low word; only the active states take focus.
fn activation_gives_focus(wparam: usize) -> bool {
    matches!(wparam & 0xffff, 1 | 2)
}

/// Give keyboard focus to the plugin's editor window, if it is inside.
///
/// # Safety
/// Call on the window's own thread. The editor is a child created by another
/// process; Windows shares the input queue of parent and child, and a failed
/// SetFocus is harmless.
unsafe fn focus_editor(window: HWND) {
    if let Ok(child) = GetWindow(window, GW_CHILD) {
        if !child.is_invalid() {
            let _ = SetFocus(Some(child));
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY (whole function): the state pointer came from Box::into_raw in
    // `create_window` and stays valid until `run_window` frees it, which
    // happens only after WM_NCDESTROY has detached it from this window. It
    // is only touched on this window's thread.
    let state = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut HostState;
    match message {
        WM_NCCREATE => {
            // SAFETY: lparam is the CREATESTRUCTW for this window; its
            // lpCreateParams is the state pointer passed to CreateWindowExW.
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
            DefWindowProcW(window, message, wparam, lparam)
        }
        WM_TIMER if wparam.0 == WATCH_TIMER => {
            if let Some(state) = state.as_mut() {
                let has_child = GetWindow(window, GW_CHILD).is_ok_and(|child| !child.is_invalid());
                if state.close_result.is_some() {
                    match state.watch.close_tick(has_child) {
                        WatchAction::Close => {
                            let _ = DestroyWindow(window);
                            return LRESULT(0);
                        }
                        WatchAction::Show => {
                            // Do not destroy a window while a plugin editor
                            // is still parented to it. Restore the host and
                            // allow a deliberate retry instead of crashing it.
                            state.close_result = None;
                            state.closing = false;
                            let _ = ShowWindow(window, SW_SHOW);
                        }
                        WatchAction::None => {}
                    }
                } else if !state.closing {
                    match state.watch.tick(has_child) {
                        WatchAction::Show => {
                            let _ = ShowWindow(window, SW_SHOW);
                            let _ = SetForegroundWindow(window);
                            focus_editor(window);
                        }
                        WatchAction::Close => {
                            let _ = PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0));
                        }
                        WatchAction::None => {}
                    }
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let Some(state) = state.as_mut() else {
                let _ = DestroyWindow(window);
                return LRESULT(0);
            };
            if state.closing {
                return LRESULT(0);
            }
            state.closing = true;
            let _ = ShowWindow(window, SW_HIDE);
            // Run the backend request off this thread: the plugin removes its
            // editor from this window meanwhile and needs it to answer.
            let handle = window.0 as isize;
            let on_close = Arc::clone(&state.on_close);
            let spawned = thread::Builder::new()
                .name("audiorouter-plugin-editor-close".into())
                .spawn(move || {
                    let closed = on_close();
                    // SAFETY: posting to a window that no longer exists
                    // fails harmlessly.
                    let _ = unsafe {
                        PostMessageW(
                            Some(HWND(handle as *mut _)),
                            CLOSE_FINISHED,
                            WPARAM(usize::from(closed)),
                            LPARAM(0),
                        )
                    };
                });
            if spawned.is_err() {
                // The editor is still parented to this window; keep it alive
                // so the user can retry closing it.
                state.closing = false;
                let _ = ShowWindow(window, SW_SHOW);
            }
            LRESULT(0)
        }
        // Keyboard input belongs to the plugin's editor: hand focus to it
        // whenever this window gets it (opening, clicking the title bar,
        // switching back with Alt+Tab).
        WM_SETFOCUS => {
            focus_editor(window);
            LRESULT(0)
        }
        WM_ACTIVATE if activation_gives_focus(wparam.0) => {
            focus_editor(window);
            LRESULT(0)
        }
        // Moving the window to a monitor with another scale: Windows suggests
        // the new outer size. The worker refits the editor to the new client
        // area on its own pass.
        WM_DPICHANGED => {
            if let Some(suggested) = (lparam.0 as *const RECT).as_ref() {
                let _ = SetWindowPos(
                    window,
                    None,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            LRESULT(0)
        }
        CLOSE_FINISHED => {
            if let Some(state) = state.as_mut() {
                state.close_result = Some(wparam.0 != 0);
                state.watch.missing_ticks = 0;
                let has_child = GetWindow(window, GW_CHILD).is_ok_and(|child| !child.is_invalid());
                if !has_child {
                    let _ = DestroyWindow(window);
                } else if !state.watch_timer_active {
                    // Without polling, keep the parent visible for another
                    // explicit close attempt rather than orphaning the child.
                    state.close_result = None;
                    state.closing = false;
                    let _ = ShowWindow(window, SW_SHOW);
                }
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            // Detach the state so no later message can reach it; its owner
            // (`run_window`, or `create_window` on failure) frees it.
            SetWindowLongPtrW(window, GWLP_USERDATA, 0);
            PostQuitMessage(0);
            DefWindowProcW(window, message, wparam, lparam)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SendMessageW, SetWindowLongPtrW, GWL_EXSTYLE, SWP_FRAMECHANGED, SWP_NOSIZE,
        SW_SHOWNOACTIVATE, WA_ACTIVE, WA_CLICKACTIVE, WA_INACTIVE, WINDOW_EX_STYLE, WS_CHILD,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_TABSTOP, WS_VISIBLE,
    };

    /// Places the real host window where nobody can see it: far off every
    /// monitor, as a tool window (no taskbar button) that never activates.
    const OFF_SCREEN: i32 = -32_000;

    /// Runs `body(host, editor)` on a fresh thread with a real host window made
    /// by the production `create_window`/`window_proc` and one child window
    /// standing in for the plugin editor (an EDIT control, which can take
    /// focus). The watch timer is never started, so the window shows only when
    /// `show_off_screen` is set, and then off screen without activation.
    fn with_host(show_off_screen: bool, body: impl FnOnce(HWND, HWND) + Send + 'static) {
        thread::spawn(move || {
            // SAFETY: this thread creates the window, uses it only here and
            // destroys it before freeing the state, as `run_window` does.
            unsafe {
                let (host, state) =
                    create_window("test host", 300, 200, Arc::new(|| true)).expect("host window");
                let instance = GetModuleHandleW(None).unwrap();
                let editor = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("EDIT"),
                    w!(""),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                    10,
                    10,
                    120,
                    40,
                    Some(host),
                    None,
                    Some(HINSTANCE(instance.0)),
                    None,
                )
                .expect("editor stand-in");
                if show_off_screen {
                    SetWindowLongPtrW(
                        host,
                        GWL_EXSTYLE,
                        (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE).0 as isize,
                    );
                    SetWindowPos(
                        host,
                        None,
                        OFF_SCREEN,
                        OFF_SCREEN,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                    )
                    .unwrap();
                    let _ = ShowWindow(host, SW_SHOWNOACTIVATE);
                }
                body(host, editor);
                let _ = DestroyWindow(host);
                drop(Box::from_raw(state));
            }
        })
        .join()
        .expect("window test thread");
    }

    /// SAFETY (callers): `window` belongs to the calling thread.
    unsafe fn send(window: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
        SendMessageW(window, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))).0
    }

    #[test]
    fn focus_follows_set_focus_and_active_activation_into_the_editor() {
        // SAFETY: `with_host` runs the closure on the thread that created
        // both windows and keeps them alive for the whole call.
        with_host(true, |host, editor| unsafe {
            // Nothing in this window has focus yet.
            let _ = SetFocus(None);
            assert_ne!(GetFocus(), editor);

            assert_eq!(send(host, WM_SETFOCUS, 0, 0), 0);
            assert_eq!(GetFocus(), editor, "WM_SETFOCUS hands focus to the editor");

            // An inactive activation must leave focus alone ...
            let _ = SetFocus(None);
            assert_eq!(send(host, WM_ACTIVATE, WA_INACTIVE as usize, 0), 0);
            assert_ne!(GetFocus(), editor, "WA_INACTIVE does not take focus");
            // ... also when the minimized flag sits in the high word.
            assert_eq!(send(host, WM_ACTIVATE, 0x1_0000, 0), 0);
            assert_ne!(GetFocus(), editor);

            // Active and click-active activation do.
            for state in [WA_ACTIVE, WA_CLICKACTIVE] {
                let _ = SetFocus(None);
                assert_eq!(send(host, WM_ACTIVATE, state as usize, 0), 0);
                assert_eq!(GetFocus(), editor, "activation state {state} takes focus");
            }
        });
    }

    #[test]
    fn a_dpi_change_applies_the_suggested_rectangle_to_the_host() {
        // SAFETY: `with_host` runs the closure on the thread that created the
        // window and keeps it alive for the whole call; `suggested` outlives
        // the synchronous SendMessageW that reads it.
        with_host(false, |host, _| unsafe {
            // 150 percent: Windows suggests a new outer rectangle for the window.
            let suggested = RECT {
                left: OFF_SCREEN + 120,
                top: OFF_SCREEN + 80,
                right: OFF_SCREEN + 120 + 450,
                bottom: OFF_SCREEN + 80 + 330,
            };
            let dpi = 144_usize;
            let result = send(
                host,
                WM_DPICHANGED,
                (dpi << 16) | dpi,
                std::ptr::addr_of!(suggested) as isize,
            );
            assert_eq!(result, 0);
            let mut actual = RECT::default();
            GetWindowRect(host, &mut actual).unwrap();
            assert_eq!(
                (actual.left, actual.top, actual.right, actual.bottom),
                (
                    suggested.left,
                    suggested.top,
                    suggested.right,
                    suggested.bottom
                )
            );

            // A message without a rectangle is ignored and changes nothing.
            assert_eq!(send(host, WM_DPICHANGED, (dpi << 16) | dpi, 0), 0);
            let mut unchanged = RECT::default();
            GetWindowRect(host, &mut unchanged).unwrap();
            assert_eq!(
                (
                    unchanged.left,
                    unchanged.top,
                    unchanged.right,
                    unchanged.bottom
                ),
                (actual.left, actual.top, actual.right, actual.bottom)
            );
        });
    }

    #[test]
    fn only_active_activation_states_take_focus() {
        assert!(!activation_gives_focus(0));
        assert!(activation_gives_focus(1));
        assert!(activation_gives_focus(2));
        // The high word (minimized flag) must not change the decision.
        assert!(!activation_gives_focus(0x1_0000));
        assert!(activation_gives_focus(0x1_0001));
    }

    #[test]
    fn the_window_appears_when_the_editor_is_inside_it() {
        let mut watch = EditorWatch::default();
        assert_eq!(watch.tick(false), WatchAction::None);
        assert_eq!(watch.tick(true), WatchAction::Show);
        assert_eq!(watch.tick(true), WatchAction::None);
    }

    #[test]
    fn a_window_without_an_editor_is_shown_after_a_bounded_wait() {
        let mut watch = EditorWatch::default();
        for _ in 1..EditorWatch::SHOW_ANYWAY_TICKS {
            assert_eq!(watch.tick(false), WatchAction::None);
        }
        assert_eq!(watch.tick(false), WatchAction::Show);
        // It never had an editor, so it does not close by itself.
        for _ in 0..EditorWatch::GONE_TICKS * 2 {
            assert_eq!(watch.tick(false), WatchAction::None);
        }
    }

    #[test]
    fn the_window_closes_when_its_editor_is_gone_but_not_while_it_is_rebuilt() {
        let mut watch = EditorWatch::default();
        assert_eq!(watch.tick(true), WatchAction::Show);
        // A plugin that rebuilds its own window leaves a short gap.
        for _ in 1..EditorWatch::GONE_TICKS {
            assert_eq!(watch.tick(false), WatchAction::None);
        }
        assert_eq!(watch.tick(true), WatchAction::None);
        // A stopped or crashed worker takes the editor away for good.
        for _ in 1..EditorWatch::GONE_TICKS {
            assert_eq!(watch.tick(false), WatchAction::None);
        }
        assert_eq!(watch.tick(false), WatchAction::Close);
    }

    #[test]
    fn bounded_close_wait_restores_a_slow_editor_for_retry() {
        let mut watch = EditorWatch::default();
        assert_eq!(watch.tick(true), WatchAction::Show);
        for _ in 0..EditorWatch::GONE_TICKS - 1 {
            assert_eq!(watch.close_tick(true), WatchAction::None);
        }
        assert_eq!(watch.close_tick(true), WatchAction::Show);
        assert_eq!(watch.close_tick(false), WatchAction::Close);
    }
}
