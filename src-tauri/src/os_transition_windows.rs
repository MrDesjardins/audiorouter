//! Windows OS-session transition listener.
//!
//! The listener owns a message-only window on a dedicated thread. The window
//! procedure does only bounded channel delivery; backend RPC, endpoint
//! enumeration, and session lifecycle work stay on the receiver thread.

use audiorouter_control::os_transition::OsTransition;
use std::sync::mpsc::{self, SyncSender};
use std::thread::{self, JoinHandle};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
    NOTIFY_FOR_THIS_SESSION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, PostQuitMessage, PostThreadMessageW, RegisterClassW, SetWindowLongPtrW,
    TranslateMessage, CREATESTRUCTW, GWLP_USERDATA, HWND_MESSAGE, MSG, WM_APP, WM_NCCREATE,
    WM_NCDESTROY, WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WNDCLASSW, PBT_APMRESUMEAUTOMATIC,
    PBT_APMSUSPEND, WTS_SESSION_LOCK, WTS_SESSION_LOGOFF,
};
use windows::Win32::System::Threading::GetCurrentThreadId;

const STOP_MESSAGE: u32 = WM_APP + 1;
const WINDOW_CLASS: PCWSTR = w!("AudioRouterOsTransitionListener");

pub struct OsTransitionListener {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl OsTransitionListener {
    pub fn start(sender: SyncSender<OsTransition>) -> Result<Self, String> {
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("audiorouter-os-transitions".into())
            .spawn(move || run_message_loop(sender, ready_sender))
            .map_err(|error| format!("OS transition listener thread failed: {error}"))?;
        let thread_id = ready_receiver
            .recv()
            .map_err(|_| "OS transition listener exited before initialization".to_owned())??;
        Ok(Self {
            thread_id,
            thread: Some(thread),
        })
    }
}

impl Drop for OsTransitionListener {
    fn drop(&mut self) {
        // The message is posted to the listener thread's queue and never
        // waits on the window procedure. A shutdown race is harmless because
        // the thread also exits when its message queue closes.
        let _ = unsafe {
            PostThreadMessageW(self.thread_id, STOP_MESSAGE, WPARAM(0), LPARAM(0))
        };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_message_loop(sender: SyncSender<OsTransition>, ready_sender: mpsc::SyncSender<Result<u32, String>>) {
    let result = unsafe { create_message_window(sender) };
    let Ok((thread_id, window)) = result else {
        let error = result
            .err()
            .unwrap_or_else(|| "unknown OS transition listener error".into());
        let _ = ready_sender.send(Err(error));
        return;
    };
    let _ = ready_sender.send(Ok(thread_id));
    let mut message = MSG::default();
    loop {
        let status = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if status.0 <= 0 {
            break;
        }
        if message.message == STOP_MESSAGE {
            unsafe { PostQuitMessage(0) };
            continue;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    unsafe {
        let _ = WTSUnRegisterSessionNotification(window);
        let _ = DestroyWindow(window);
    }
}

unsafe fn create_message_window(sender: SyncSender<OsTransition>) -> Result<(u32, HWND), String> {
    let instance = GetModuleHandleW(None)
        .map_err(|error| format!("GetModuleHandleW failed: {error}"))?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: WINDOW_CLASS,
        ..Default::default()
    };
    if RegisterClassW(&class) == 0 {
        return Err("RegisterClassW failed".into());
    }
    let sender = Box::into_raw(Box::new(sender));
    let window = match CreateWindowExW(
        Default::default(),
        WINDOW_CLASS,
        w!("AudioRouter OS transitions"),
        Default::default(),
        0,
        0,
        0,
        0,
        Some(HWND_MESSAGE),
        None,
        Some(instance.into()),
        Some(sender.cast()),
    ) {
        Ok(window) => window,
        Err(error) => {
            drop(Box::from_raw(sender));
            return Err(format!("CreateWindowExW failed: {error}"));
        }
    };
    if let Err(error) = WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION) {
        let _ = DestroyWindow(window);
        return Err(format!("WTSRegisterSessionNotification failed: {error}"));
    }
    Ok((GetCurrentThreadId(), window))
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam.0 as *const CREATESTRUCTW;
        if !create.is_null() {
            SetWindowLongPtrW(window, GWLP_USERDATA, (*create).lpCreateParams as isize);
        }
    }
    let sender = GetWindowLongPtrW(window, GWLP_USERDATA) as *const SyncSender<OsTransition>;
    if !sender.is_null() {
        let transition = match (message, wparam.0 as u32) {
            (WM_POWERBROADCAST, PBT_APMSUSPEND) => Some(OsTransition::Sleep),
            (WM_POWERBROADCAST, PBT_APMRESUMEAUTOMATIC) => Some(OsTransition::Resume),
            (WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK) => Some(OsTransition::Lock),
            (WM_WTSSESSION_CHANGE, WTS_SESSION_LOGOFF) => Some(OsTransition::SignOut),
            _ => None,
        };
        if let Some(transition) = transition {
            let _ = (*sender).try_send(transition);
            return LRESULT(1);
        }
    }
    if message == WM_NCDESTROY && !sender.is_null() {
        drop(Box::from_raw(sender as *mut SyncSender<OsTransition>));
        SetWindowLongPtrW(window, GWLP_USERDATA, 0);
    }
    DefWindowProcW(window, message, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_starts_and_stops_without_audio_or_transition_side_effects() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let listener = OsTransitionListener::start(sender).expect("native listener starts");
        assert!(receiver.try_recv().is_err());
        drop(listener);
        assert!(receiver.try_recv().is_err());
    }
}
