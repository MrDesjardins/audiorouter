//! Global Windows keybindings for soundboard, privacy mute / push-to-talk,
//! session playback, and microphone switching.
//!
//! Uses a dedicated low-level Win32 hook thread (`WH_KEYBOARD_LL` +
//! `WH_MOUSE_LL`) that never blocks on I/O: matched events are pushed onto a
//! bounded channel and executed on a separate RPC worker thread via the
//! shell's backend pipe.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeybindMode {
    /// Fire once on key/button press.
    #[default]
    Press,
    /// Fire on press (`release: false`) and again on release (`release: true`).
    Hold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeybindTrigger {
    pub keys: String,
    #[serde(default)]
    pub mode: KeybindMode,
    /// When true, other applications still receive the key/button event.
    #[serde(default = "default_passthrough")]
    pub passthrough: bool,
}

fn default_passthrough() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum KeybindAction {
    SessionToggle,
    PrivacyMuteToggle,
    /// Unmute while held (`press`), mute again when released (`release`).
    PrivacyMutePushToTalk,
    #[serde(rename_all = "camelCase")]
    SoundboardTrigger {
        #[serde(default)]
        session_id: Option<String>,
        node_id: String,
        slot: u8,
        #[serde(default)]
        mode: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    SoundboardStopAll {
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        node_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    NodeToggleEnabled {
        #[serde(default)]
        session_id: Option<String>,
        node_id: String,
    },
    #[serde(rename_all = "camelCase")]
    MicSwitch {
        #[serde(default)]
        session_id: Option<String>,
        active_node_id: String,
        #[serde(default)]
        mute_node_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeybindRule {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub trigger: KeybindTrigger,
    pub action: KeybindAction,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModifierSide {
    #[default]
    None,
    Any,
    Left,
    Right,
}

impl ModifierSide {
    fn matches(self, left_down: bool, right_down: bool) -> bool {
        match self {
            Self::None => !left_down && !right_down,
            Self::Any => left_down || right_down,
            Self::Left => left_down && !right_down,
            Self::Right => right_down && !left_down,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub ctrl: ModifierSide,
    pub alt: ModifierSide,
    pub shift: ModifierSide,
    pub meta: ModifierSide,
}

impl Modifiers {
    pub fn matches(&self, state: &HookMatcherState, exclude_vk: Option<u16>) -> bool {
        let is_down = |vk: u16| exclude_vk != Some(vk) && state.pressed_vks.contains(&vk);
        self.ctrl
            .matches(is_down(0xA2) || is_down(0x11), is_down(0xA3))
            && self
                .shift
                .matches(is_down(0xA0) || is_down(0x10), is_down(0xA1))
            && self
                .alt
                .matches(is_down(0xA4) || is_down(0x12), is_down(0xA5))
            && self.meta.matches(is_down(0x5B), is_down(0x5C))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Middle,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerCode {
    Keyboard(u16),
    Mouse(MouseButton),
}

#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub rule: KeybindRule,
    pub modifiers: Modifiers,
    pub code: TriggerCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindInvocation {
    pub rule_id: String,
    pub action: KeybindAction,
    pub release: bool,
}

/// Parse a trigger string such as `"Control+Shift+F1"` or `"Mouse4"` into
/// `(Modifiers, TriggerCode)`.
pub fn parse_trigger_keys(spec: &str) -> Result<(Modifiers, TriggerCode), String> {
    let parts: Vec<&str> = spec
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Err("empty keybind trigger".into());
    }
    let mut modifiers = Modifiers::default();
    let Some((&key_part, modifier_parts)) = parts.split_last() else {
        return Err("empty keybind trigger".into());
    };
    for &modifier in modifier_parts {
        match modifier.to_ascii_lowercase().as_str() {
            "control" | "ctrl" => modifiers.ctrl = ModifierSide::Any,
            "lcontrol" | "lctrl" => modifiers.ctrl = ModifierSide::Left,
            "rcontrol" | "rctrl" => modifiers.ctrl = ModifierSide::Right,
            "alt" => modifiers.alt = ModifierSide::Any,
            "lalt" => modifiers.alt = ModifierSide::Left,
            "ralt" => modifiers.alt = ModifierSide::Right,
            "shift" => modifiers.shift = ModifierSide::Any,
            "lshift" => modifiers.shift = ModifierSide::Left,
            "rshift" => modifiers.shift = ModifierSide::Right,
            "meta" | "win" | "super" => modifiers.meta = ModifierSide::Any,
            "lmeta" | "lwin" => modifiers.meta = ModifierSide::Left,
            "rmeta" | "rwin" => modifiers.meta = ModifierSide::Right,
            other => return Err(format!("unknown modifier '{other}'")),
        }
    }
    let code = parse_trigger_code(key_part)?;
    Ok((modifiers, code))
}

fn parse_trigger_code(token: &str) -> Result<TriggerCode, String> {
    let lower = token.to_ascii_lowercase();
    if let Some(button) = match lower.as_str() {
        "mousemiddle" | "mouse3" => Some(MouseButton::Middle),
        "mouse4" | "mousex1" => Some(MouseButton::X1),
        "mouse5" | "mousex2" => Some(MouseButton::X2),
        _ => None,
    } {
        return Ok(TriggerCode::Mouse(button));
    }

    if lower.len() == 1 {
        let ch = lower.as_bytes()[0];
        if ch.is_ascii_lowercase() {
            return Ok(TriggerCode::Keyboard(u16::from(ch.to_ascii_uppercase())));
        }
        if ch.is_ascii_digit() {
            return Ok(TriggerCode::Keyboard(u16::from(ch)));
        }
    }

    if let Some(num_str) = lower.strip_prefix('f') {
        if let Ok(num) = num_str.parse::<u16>() {
            if (1..=24).contains(&num) {
                return Ok(TriggerCode::Keyboard(0x70 + (num - 1)));
            }
        }
    }

    if let Some(num_str) = lower.strip_prefix("numpad") {
        if let Ok(num) = num_str.parse::<u16>() {
            if num <= 9 {
                return Ok(TriggerCode::Keyboard(0x60 + num));
            }
        }
    }

    let vk = match lower.as_str() {
        "space" | " " => 0x20,
        "enter" | "return" => 0x0D,
        "tab" => 0x09,
        "escape" | "esc" => 0x1B,
        "backspace" => 0x08,
        "insert" => 0x2D,
        "delete" => 0x2E,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "arrowleft" | "left" => 0x25,
        "arrowup" | "up" => 0x26,
        "arrowright" | "right" => 0x27,
        "arrowdown" | "down" => 0x28,
        "capslock" => 0x14,
        "lcontrol" | "lctrl" => 0xA2,
        "rcontrol" | "rctrl" => 0xA3,
        "lshift" => 0xA0,
        "rshift" => 0xA1,
        "lalt" => 0xA4,
        "ralt" => 0xA5,
        "lmeta" | "lwin" => 0x5B,
        "rmeta" | "rwin" => 0x5C,
        "scrolllock" => 0x91,
        "pause" => 0x13,
        "printscreen" => 0x2C,
        "mediaplaypause" => 0xB3,
        "mediastop" => 0xB2,
        "medianexttrack" => 0xB0,
        "mediaprevioustrack" => 0xB1,
        "audiovolumeup" => 0xAF,
        "audiovolumedown" => 0xAE,
        "audiovolumemute" => 0xAD,
        ";" | "semicolon" => 0xBA,
        "=" | "equal" => 0xBB,
        "," | "comma" => 0xBC,
        "-" | "minus" => 0xBD,
        "." | "period" => 0xBE,
        "/" | "slash" => 0xBF,
        "`" | "backquote" => 0xC0,
        "[" | "bracketleft" => 0xDB,
        "\\" | "backslash" => 0xDC,
        "]" | "bracketright" => 0xDD,
        "'" | "quote" => 0xDE,
        _ => return Err(format!("unsupported key '{token}'")),
    };
    Ok(TriggerCode::Keyboard(vk))
}

pub fn compile_rules(rules: &[KeybindRule]) -> Result<Vec<CompiledRule>, String> {
    let mut compiled = Vec::with_capacity(rules.len());
    for rule in rules {
        if !rule.enabled {
            continue;
        }
        let (modifiers, code) = parse_trigger_keys(&rule.trigger.keys)?;
        compiled.push(CompiledRule {
            rule: rule.clone(),
            modifiers,
            code,
        });
    }
    Ok(compiled)
}

#[derive(Default)]
pub struct HookMatcherState {
    pressed_vks: HashSet<u16>,
    pressed_mouse: HashSet<MouseButton>,
    active_holds: HashMap<TriggerCode, Vec<(String, KeybindAction, bool)>>,
}

impl HookMatcherState {
    /// Process a single key or mouse button transition and return matched
    /// invocations plus whether the input event should be swallowed.
    pub fn handle_event(
        &mut self,
        code: TriggerCode,
        is_down: bool,
        rules: &[CompiledRule],
    ) -> (Vec<KeybindInvocation>, bool) {
        let is_repeat = match (code, is_down) {
            (TriggerCode::Keyboard(vk), true) => !self.pressed_vks.insert(vk),
            (TriggerCode::Keyboard(vk), false) => {
                self.pressed_vks.remove(&vk);
                false
            }
            (TriggerCode::Mouse(btn), true) => !self.pressed_mouse.insert(btn),
            (TriggerCode::Mouse(btn), false) => {
                self.pressed_mouse.remove(&btn);
                false
            }
        };

        if is_down {
            if is_repeat {
                // Keep swallowing repeats of a non-passthrough held trigger.
                let swallow = self
                    .active_holds
                    .get(&code)
                    .is_some_and(|items| items.iter().any(|(_, _, passthrough)| !passthrough));
                return (Vec::new(), swallow);
            }
            let exclude_vk = match code {
                TriggerCode::Keyboard(vk) => Some(vk),
                TriggerCode::Mouse(_) => None,
            };
            let mut invocations = Vec::new();
            let mut swallow = false;
            for compiled in rules {
                if compiled.code == code && compiled.modifiers.matches(self, exclude_vk) {
                    if !compiled.rule.trigger.passthrough {
                        swallow = true;
                    }
                    invocations.push(KeybindInvocation {
                        rule_id: compiled.rule.id.clone(),
                        action: compiled.rule.action.clone(),
                        release: false,
                    });
                    if compiled.rule.trigger.mode == KeybindMode::Hold
                        || matches!(
                            compiled.rule.action,
                            KeybindAction::SoundboardTrigger { .. }
                        )
                    {
                        self.active_holds.entry(code).or_default().push((
                            compiled.rule.id.clone(),
                            compiled.rule.action.clone(),
                            compiled.rule.trigger.passthrough,
                        ));
                    }
                }
            }
            (invocations, swallow)
        } else {
            let Some(holds) = self.active_holds.remove(&code) else {
                return (Vec::new(), false);
            };
            let mut swallow = false;
            let mut invocations = Vec::with_capacity(holds.len());
            for (rule_id, action, passthrough) in holds {
                if !passthrough {
                    swallow = true;
                }
                invocations.push(KeybindInvocation {
                    rule_id,
                    action,
                    release: true,
                });
            }
            (invocations, swallow)
        }
    }
}

pub struct KeybindEngine {
    suspended: AtomicBool,
    rules: RwLock<Vec<CompiledRule>>,
}

impl KeybindEngine {
    pub fn new(initial: &[KeybindRule]) -> Self {
        let compiled = compile_rules(initial).unwrap_or_default();
        Self {
            suspended: AtomicBool::new(false),
            rules: RwLock::new(compiled),
        }
    }

    pub fn set_suspended(&self, suspended: bool) {
        self.suspended.store(suspended, Ordering::Relaxed);
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended.load(Ordering::Relaxed)
    }

    pub fn update_rules(&self, rules: &[KeybindRule]) -> Result<(), String> {
        let compiled = compile_rules(rules)?;
        let mut guard = self
            .rules
            .write()
            .map_err(|_| "keybind state lock poisoned".to_owned())?;
        *guard = compiled;
        Ok(())
    }
}

pub struct KeybindService {
    pub engine: Arc<KeybindEngine>,
    #[cfg(windows)]
    listener: std::sync::Mutex<Option<windows_hook::GlobalKeybindListener>>,
}

impl KeybindService {
    pub fn new(initial_rules: &[KeybindRule]) -> Self {
        Self {
            engine: Arc::new(KeybindEngine::new(initial_rules)),
            #[cfg(windows)]
            listener: std::sync::Mutex::new(None),
        }
    }

    pub fn start_listener(&self, pipe_name: String) {
        #[cfg(windows)]
        {
            let Ok(mut slot) = self.listener.lock() else {
                return;
            };
            if slot.is_some() {
                return;
            };
            let (sender, receiver) = mpsc::sync_channel::<KeybindInvocation>(64);
            let rpc_pipe = pipe_name;
            let _ = thread::Builder::new()
                .name("audiorouter-keybind-rpc".into())
                .spawn(move || {
                    for invocation in receiver {
                        execute_invocation(&rpc_pipe, invocation);
                    }
                });
            match windows_hook::GlobalKeybindListener::start(Arc::clone(&self.engine), sender) {
                Ok(listener) => *slot = Some(listener),
                Err(error) => eprintln!("AudioRouter global keybind hook unavailable: {error}"),
            }
        }
        #[cfg(not(windows))]
        let _ = pipe_name;
    }
}

use crate::tray_playback::idempotency_key;

fn with_session(mut params: Value, session_id: Option<&str>) -> Value {
    if let Some(sid) = session_id {
        params["sessionId"] = json!(sid);
    }
    params
}

fn resolve_session_id(
    call: &mut crate::tray_playback::Call<'_>,
    explicit: Option<&str>,
) -> Option<String> {
    if let Some(id) = explicit.filter(|id| !id.trim().is_empty()) {
        return Some(id.to_owned());
    }
    call("sessions.active.get", json!({}))
        .ok()
        .and_then(|value| {
            value
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
}

fn execute_invocation(pipe_name: &str, invocation: KeybindInvocation) {
    let mut call = crate::backend_call(pipe_name);
    let result: Result<Value, String> = match invocation.action {
        KeybindAction::SessionToggle => call(
            "sessions.togglePlay",
            json!({ "idempotencyKey": idempotency_key("kb-session") }),
        ),
        KeybindAction::PrivacyMuteToggle => call(
            "safety.togglePrivacyMute",
            json!({ "idempotencyKey": idempotency_key("kb-mute") }),
        ),
        KeybindAction::PrivacyMutePushToTalk => call(
            "safety.setPrivacyMute",
            json!({
                "muted": invocation.release,
                "idempotencyKey": idempotency_key("kb-ptt")
            }),
        ),
        KeybindAction::SoundboardTrigger {
            session_id,
            node_id,
            slot,
            mode: _,
        } => {
            let Some(session_id) = resolve_session_id(&mut call, session_id.as_deref()) else {
                eprintln!(
                    "AudioRouter keybind '{}' skipped: no active session",
                    invocation.rule_id
                );
                return;
            };
            let action = if invocation.release {
                "release"
            } else {
                "press"
            };
            let params = json!({
                "sessionId": session_id,
                "nodeId": node_id,
                "slot": slot,
                "action": action,
            });
            call("soundboard.trigger", params)
        }
        KeybindAction::SoundboardStopAll {
            session_id,
            node_id,
        } => {
            let Some(session_id) = resolve_session_id(&mut call, session_id.as_deref()) else {
                return;
            };
            let mut params = json!({ "sessionId": session_id });
            if let Some(node_id) = node_id {
                params["nodeId"] = json!(node_id);
            }
            call("soundboard.stop", params)
        }
        KeybindAction::NodeToggleEnabled {
            session_id,
            node_id,
        } => {
            let params = with_session(
                json!({
                    "node": node_id,
                    "target": "enabled",
                    "idempotencyKey": idempotency_key("kb-node-toggle"),
                }),
                session_id.as_deref(),
            );
            call("nodes.toggle", params)
        }
        KeybindAction::MicSwitch {
            session_id,
            active_node_id,
            mute_node_ids,
        } => {
            for mute_id in mute_node_ids {
                if mute_id == active_node_id {
                    continue;
                }
                let params = with_session(
                    json!({
                        "node": mute_id,
                        "enabled": false,
                        "idempotencyKey": idempotency_key("kb-mic-off"),
                    }),
                    session_id.as_deref(),
                );
                let _ = call("nodes.set", params);
            }
            let params = with_session(
                json!({
                    "node": active_node_id,
                    "enabled": true,
                    "idempotencyKey": idempotency_key("kb-mic-on"),
                }),
                session_id.as_deref(),
            );
            call("nodes.set", params)
        }
    };
    if let Err(error) = result {
        eprintln!(
            "AudioRouter keybind '{}' failed: {error}",
            invocation.rule_id
        );
    }
}

#[cfg(windows)]
mod windows_hook {
    use super::*;
    use std::cell::RefCell;
    use std::sync::OnceLock;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT,
        WH_KEYBOARD_LL, WH_MOUSE_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    };

    const STOP_MESSAGE: u32 = WM_APP + 2;

    struct HookContext {
        engine: Arc<KeybindEngine>,
        sender: SyncSender<KeybindInvocation>,
    }

    static HOOK_CONTEXT: OnceLock<HookContext> = OnceLock::new();

    thread_local! {
        static MATCHER_STATE: RefCell<HookMatcherState> = RefCell::new(HookMatcherState::default());
    }

    pub struct GlobalKeybindListener {
        thread_id: u32,
        thread: Option<JoinHandle<()>>,
    }

    impl GlobalKeybindListener {
        pub fn start(
            engine: Arc<KeybindEngine>,
            sender: SyncSender<KeybindInvocation>,
        ) -> Result<Self, String> {
            let _ = HOOK_CONTEXT.set(HookContext { engine, sender });
            let (ready_tx, ready_rx) = mpsc::sync_channel(1);
            let thread = thread::Builder::new()
                .name("audiorouter-keybind-hook".into())
                .spawn(move || run_hook_loop(ready_tx))
                .map_err(|error| format!("keybind hook thread failed: {error}"))?;
            let thread_id = ready_rx
                .recv()
                .map_err(|_| "keybind hook thread exited before initialization".to_owned())??;
            Ok(Self {
                thread_id,
                thread: Some(thread),
            })
        }
    }

    impl Drop for GlobalKeybindListener {
        fn drop(&mut self) {
            // SAFETY: PostThreadMessageW posts an integer message to the hook
            // thread's queue without blocking. If the thread already exited, the
            // error is safely ignored.
            let _ =
                unsafe { PostThreadMessageW(self.thread_id, STOP_MESSAGE, WPARAM(0), LPARAM(0)) };
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn run_hook_loop(ready_tx: SyncSender<Result<u32, String>>) {
        // SAFETY: GetCurrentThreadId and GetModuleHandleW(None) have no
        // preconditions and return OS handles valid for this process/thread.
        let (thread_id, module) = unsafe { (GetCurrentThreadId(), GetModuleHandleW(None).ok()) };
        let hmod = module.map(Into::into);

        // SAFETY: keyboard_hook_proc and mouse_hook_proc are valid
        // `extern "system"` callbacks that do not panic or block, and both
        // hooks are unhooked before this thread returns.
        let kbd_hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), hmod, 0) };
        // SAFETY: `mouse_hook_proc` is a valid `extern "system"` callback that
        // does not panic or block, and `mouse_hook` is unhooked before return.
        let mouse_hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), hmod, 0) };

        let (Ok(kbd_hook), Ok(mouse_hook)) =
            (kbd_hook.as_ref().copied(), mouse_hook.as_ref().copied())
        else {
            if let Ok(hook) = kbd_hook {
                // SAFETY: `hook` was just returned by `SetWindowsHookExW`.
                let _ = unsafe { UnhookWindowsHookEx(hook) };
            }
            if let Ok(hook) = mouse_hook {
                // SAFETY: `hook` was just returned by `SetWindowsHookExW`.
                let _ = unsafe { UnhookWindowsHookEx(hook) };
            }
            let _ = ready_tx.send(Err("SetWindowsHookExW failed".into()));
            return;
        };

        let _ = ready_tx.send(Ok(thread_id));

        let mut msg = MSG::default();
        loop {
            // SAFETY: `msg` is stack-allocated and valid for the duration of
            // `GetMessageW` on this thread's message queue.
            let status = unsafe { GetMessageW(&mut msg, None, 0, 0) };
            if status.0 <= 0 || msg.message == STOP_MESSAGE {
                break;
            }
            // SAFETY: `msg` was populated by `GetMessageW`.
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // SAFETY: Both hook handles belong to this thread and have not been
        // unhooked yet.
        unsafe {
            let _ = UnhookWindowsHookEx(kbd_hook);
            let _ = UnhookWindowsHookEx(mouse_hook);
        }
    }

    fn sync_modifiers_from_os(state: &mut HookMatcherState) {
        const MODIFIER_VKS: [u16; 8] = [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5B, 0x5C];
        for vk in MODIFIER_VKS {
            // SAFETY: `GetAsyncKeyState` is a thread-safe Win32 query taking
            // an integer virtual-key code with no pointer/lifetime invariants.
            let down = (unsafe { GetAsyncKeyState(i32::from(vk)) } as u16 & 0x8000) != 0;
            if down {
                state.pressed_vks.insert(vk);
            } else {
                state.pressed_vks.remove(&vk);
            }
        }
    }

    fn dispatch_trigger(code: TriggerCode, is_down: bool) -> bool {
        let Some(ctx) = HOOK_CONTEXT.get() else {
            return false;
        };
        let suspended = ctx.engine.is_suspended();
        let rules_guard = if suspended {
            None
        } else {
            ctx.engine.rules.try_read().ok()
        };
        let rules_slice: &[CompiledRule] = rules_guard.as_deref().map_or(&[], Vec::as_slice);
        let (invocations, swallow) = MATCHER_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if is_down {
                sync_modifiers_from_os(&mut state);
            }
            state.handle_event(code, is_down, rules_slice)
        });
        if suspended {
            return false;
        }
        for invocation in invocations {
            let _ = ctx.sender.try_send(invocation);
        }
        swallow
    }

    unsafe extern "system" fn keyboard_hook_proc(
        ncode: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if ncode >= 0 && lparam.0 != 0 {
            let msg = wparam.0 as u32;
            let is_down = match msg {
                WM_KEYDOWN | WM_SYSKEYDOWN => Some(true),
                WM_KEYUP | WM_SYSKEYUP => Some(false),
                _ => None,
            };
            if let Some(is_down) = is_down {
                // SAFETY: When `ncode >= 0` in `WH_KEYBOARD_LL`, Windows passes
                // a valid pointer to `KBDLLHOOKSTRUCT` in `lparam`.
                let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
                if let Ok(vk) = u16::try_from(info.vkCode) {
                    if dispatch_trigger(TriggerCode::Keyboard(vk), is_down) {
                        return LRESULT(1);
                    }
                }
            }
        }
        // SAFETY: Forwarding unhandled events to `CallNextHookEx` is required
        // by the Win32 hook contract.
        unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
    }

    unsafe extern "system" fn mouse_hook_proc(
        ncode: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if ncode >= 0 && lparam.0 != 0 {
            let msg = wparam.0 as u32;
            let event = match msg {
                WM_MBUTTONDOWN => Some((MouseButton::Middle, true)),
                WM_MBUTTONUP => Some((MouseButton::Middle, false)),
                WM_XBUTTONDOWN | WM_XBUTTONUP => {
                    // SAFETY: When `ncode >= 0` in `WH_MOUSE_LL`, Windows passes
                    // a valid pointer to `MSLLHOOKSTRUCT` in `lparam`.
                    let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
                    let button = ((info.mouseData >> 16) & 0xFFFF) as u16;
                    let mapped = match button {
                        1 => Some(MouseButton::X1),
                        2 => Some(MouseButton::X2),
                        _ => None,
                    };
                    mapped.map(|btn| (btn, msg == WM_XBUTTONDOWN))
                }
                _ => None,
            };
            if let Some((button, is_down)) = event {
                if dispatch_trigger(TriggerCode::Mouse(button), is_down) {
                    return LRESULT(1);
                }
            }
        }
        // SAFETY: Forwarding unhandled events to `CallNextHookEx` is required
        // by the Win32 hook contract.
        unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rule(id: &str, keys: &str, mode: KeybindMode, passthrough: bool) -> KeybindRule {
        KeybindRule {
            id: id.into(),
            name: String::new(),
            enabled: true,
            trigger: KeybindTrigger {
                keys: keys.into(),
                mode,
                passthrough,
            },
            action: KeybindAction::SoundboardTrigger {
                session_id: None,
                node_id: "sb-1".into(),
                slot: 1,
                mode: None,
            },
        }
    }

    #[test]
    fn parses_keyboard_and_mouse_triggers() {
        let (mods, code) = parse_trigger_keys("Control+Shift+F13").unwrap();
        assert_eq!(mods.ctrl, ModifierSide::Any);
        assert_eq!(mods.shift, ModifierSide::Any);
        assert_eq!(mods.alt, ModifierSide::None);
        assert_eq!(mods.meta, ModifierSide::None);
        assert_eq!(code, TriggerCode::Keyboard(0x7C));

        let (mods, code) = parse_trigger_keys("Alt+Mouse4").unwrap();
        assert_eq!(mods.alt, ModifierSide::Any);
        assert_eq!(mods.ctrl, ModifierSide::None);
        assert_eq!(code, TriggerCode::Mouse(MouseButton::X1));

        let (mods, code) = parse_trigger_keys("RControl+F1").unwrap();
        assert_eq!(mods.ctrl, ModifierSide::Right);
        assert_eq!(code, TriggerCode::Keyboard(0x70));

        let (mods, code) = parse_trigger_keys("RControl").unwrap();
        assert_eq!(mods.ctrl, ModifierSide::None);
        assert_eq!(code, TriggerCode::Keyboard(0xA3));
    }

    #[test]
    fn differentiates_left_and_right_control() {
        let rules =
            compile_rules(&[sample_rule("rc", "RControl+F1", KeybindMode::Press, true)]).unwrap();
        let mut state = HookMatcherState::default();

        // Left Ctrl + F1 must NOT trigger RControl+F1.
        let _ = state.handle_event(TriggerCode::Keyboard(0xA2), true, &rules);
        let (inv, _) = state.handle_event(TriggerCode::Keyboard(0x70), true, &rules);
        assert!(inv.is_empty());
        let _ = state.handle_event(TriggerCode::Keyboard(0x70), false, &rules);
        let _ = state.handle_event(TriggerCode::Keyboard(0xA2), false, &rules);

        // Right Ctrl + F1 MUST trigger RControl+F1.
        let _ = state.handle_event(TriggerCode::Keyboard(0xA3), true, &rules);
        let (inv, _) = state.handle_event(TriggerCode::Keyboard(0x70), true, &rules);
        assert_eq!(inv.len(), 1);
    }

    #[test]
    fn deserializes_camel_case_typescript_payload() {
        let json = r#"[{"id":"kb-1","enabled":true,"trigger":{"keys":"RControl+F1","mode":"press","passthrough":true},"action":{"type":"soundboardTrigger","nodeId":"sb-1","slot":2,"mode":"once"}}]"#;
        let parsed: Vec<KeybindRule> = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].action,
            KeybindAction::SoundboardTrigger {
                session_id: None,
                node_id: "sb-1".into(),
                slot: 2,
                mode: Some("once".into()),
            }
        );
    }

    #[test]
    fn ignores_repeat_and_fires_hold_release_even_after_modifier_up() {
        let rules = compile_rules(&[sample_rule(
            "hold-1",
            "Control+F1",
            KeybindMode::Hold,
            false,
        )])
        .unwrap();
        let mut state = HookMatcherState::default();

        // Press Ctrl, then F1.
        let (inv, swallow) = state.handle_event(TriggerCode::Keyboard(0xA2), true, &rules);
        assert!(inv.is_empty() && !swallow);

        let (inv, swallow) = state.handle_event(TriggerCode::Keyboard(0x70), true, &rules);
        assert_eq!(inv.len(), 1);
        assert!(!inv[0].release);
        assert!(swallow);

        // OS key-repeat on F1 must not re-trigger.
        let (inv, swallow) = state.handle_event(TriggerCode::Keyboard(0x70), true, &rules);
        assert!(inv.is_empty());
        assert!(swallow);

        // User releases Ctrl before releasing F1; F1 release must still fire.
        let _ = state.handle_event(TriggerCode::Keyboard(0xA2), false, &rules);
        let (inv, swallow) = state.handle_event(TriggerCode::Keyboard(0x70), false, &rules);
        assert_eq!(inv.len(), 1);
        assert!(inv[0].release);
        assert!(swallow);
    }
}
