//! OBS Studio integration (DEC-22, stages O1 and O2).
//!
//! A client only, on `127.0.0.1`. AudioRouter reads OBS's audio sources
//! through obs-websocket v5, which is off by default in OBS and which the
//! user turns on. While OBS is closed it reads the last saved scene
//! collection file instead. It changes an OBS source only through
//! `obs.apply`, after `obs.preview` and the user's confirmation, and keeps
//! the previous values for `obs.restore`.
//!
//! Every socket and file operation runs on the `audiorouter-obs` thread: the
//! control thread services audio every millisecond and must not wait on OBS.
//! Control methods read the shared state or queue a command.
//!
//! The WebSocket password is read from OBS's own settings on this PC when the
//! user connects, or pasted by the user. It is kept in memory on the OBS
//! thread only, and is never logged, returned or stored. Scene data is never
//! logged either.

use std::collections::VecDeque;
use std::io::{ErrorKind, Read};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use audiorouter_windows_audio::{EndpointDirection, EndpointInfo};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use tungstenite::{protocol::WebSocketConfig, Error as SocketError, Message, WebSocket};

/// obs-websocket's default port (OBS 28 and later).
pub(crate) const OBS_DEFAULT_PORT: u16 = 4455;
pub(crate) const MAX_OBS_AUDIO_INPUTS: usize = 64;
pub(crate) const MAX_OBS_NAME_BYTES: usize = 256;
pub(crate) const MAX_OBS_DEVICE_ID_BYTES: usize = 512;
pub(crate) const MAX_OBS_CHANGES: usize = 16;
pub(crate) const MAX_OBS_PASSWORD_BYTES: usize = 256;
pub(crate) const OBS_PREVIEW_SECONDS: u64 = 300;

const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_SCENE_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SETTINGS_FILE_BYTES: u64 = 256 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
/// While connected: how long a read waits, so pings are answered and a
/// closed OBS is noticed.
const IDLE_READ: Duration = Duration::from_millis(200);
/// While not connected: how long the thread sleeps between commands.
const IDLE_WAIT: Duration = Duration::from_secs(5);
/// obs-websocket's close code for a wrong password.
const AUTHENTICATION_FAILED: u16 = 4009;

/// OBS's global audio devices (Settings › Audio): the names
/// `GetSpecialInputs` returns, and the matching keys of a scene file.
const SPECIAL_INPUTS: [(&str, &str); 6] = [
    ("desktop1", "DesktopAudioDevice1"),
    ("desktop2", "DesktopAudioDevice2"),
    ("mic1", "AuxAudioDevice1"),
    ("mic2", "AuxAudioDevice2"),
    ("mic3", "AuxAudioDevice3"),
    ("mic4", "AuxAudioDevice4"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AudioKind {
    /// Audio Input Capture: records a Windows recording device.
    Input,
    /// Audio Output Capture: records what a Windows playback device plays.
    Output,
    /// Application Audio Capture: records one program.
    Application,
}

impl AudioKind {
    fn from_obs(kind: &str) -> Option<Self> {
        match kind {
            "wasapi_input_capture" => Some(Self::Input),
            "wasapi_output_capture" => Some(Self::Output),
            "wasapi_process_output_capture" => Some(Self::Application),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Input => "inputCapture",
            Self::Output => "outputCapture",
            Self::Application => "applicationCapture",
        }
    }
}

/// One OBS audio source, reduced to what the checks need.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ObsAudioInput {
    pub(crate) name: String,
    pub(crate) uuid: Option<String>,
    pub(crate) kind: AudioKind,
    /// "desktop1", "mic1", …: a global device from OBS's Settings › Audio.
    pub(crate) special: Option<&'static str>,
    /// Windows endpoint ID, or "default" for the Windows default device.
    /// None for an Application Audio Capture.
    pub(crate) device_id: Option<String>,
    /// The executable an Application Audio Capture follows.
    pub(crate) application: Option<String>,
    pub(crate) muted: Option<bool>,
    pub(crate) active: Option<bool>,
    pub(crate) audio_tracks: Option<Vec<u8>>,
    pub(crate) monitor_only: Option<bool>,
}

impl ObsAudioInput {
    fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "uuid": self.uuid,
            "kind": self.kind.name(),
            "special": self.special,
            "deviceId": self.device_id,
            "application": self.application,
            "muted": self.muted,
            "active": self.active,
            "audioTracks": self.audio_tracks,
            "monitorOnly": self.monitor_only,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Connection {
    /// Not connected; the user has not asked, or disconnected.
    #[default]
    Off,
    Connecting,
    Connected,
    /// OBS is closed, or its WebSocket server is off.
    ServerOff,
    /// OBS asks for a password and none was found in its settings.
    NeedsPassword,
    WrongPassword,
    Failed,
}

impl Connection {
    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::ServerOff => "serverOff",
            Self::NeedsPassword => "needsPassword",
            Self::WrongPassword => "wrongPassword",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Source {
    Live,
    SavedFile,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ChangeState {
    #[default]
    None,
    Applying,
    Applied,
    Restoring,
    Restored,
    Failed,
}

impl ChangeState {
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Applying => "applying",
            Self::Applied => "applied",
            Self::Restoring => "restoring",
            Self::Restored => "restored",
            Self::Failed => "failed",
        }
    }
}

/// One planned change to an OBS source, with the value it replaces.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Change {
    Device {
        input: String,
        before: String,
        after: String,
    },
    Muted {
        input: String,
        before: bool,
        after: bool,
    },
}

impl Change {
    fn input(&self) -> &str {
        match self {
            Self::Device { input, .. } | Self::Muted { input, .. } => input,
        }
    }

    fn field(&self) -> &'static str {
        match self {
            Self::Device { .. } => "device",
            Self::Muted { .. } => "muted",
        }
    }

    fn inverse(&self) -> Self {
        match self {
            Self::Device {
                input,
                before,
                after,
            } => Self::Device {
                input: input.clone(),
                before: after.clone(),
                after: before.clone(),
            },
            Self::Muted {
                input,
                before,
                after,
            } => Self::Muted {
                input: input.clone(),
                before: *after,
                after: *before,
            },
        }
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Device {
                input,
                before,
                after,
            } => {
                json!({ "inputName": input, "field": "device", "before": before, "after": after })
            }
            Self::Muted {
                input,
                before,
                after,
            } => json!({ "inputName": input, "field": "muted", "before": before, "after": after }),
        }
    }
}

#[derive(Debug, Default)]
struct Snapshot {
    installed: bool,
    connection: Connection,
    source: Option<Source>,
    scene_collection: Option<String>,
    inputs: Vec<ObsAudioInput>,
    /// Bumped on every live inspection; a preview is valid only for the one
    /// it was made from.
    inspection: u64,
    change: ChangeState,
    changed: usize,
    change_error: Option<&'static str>,
    restorable: bool,
    password_source: Option<&'static str>,
}

/// A write is bound to the inspected collection and public source identity.
/// Never fall back to a name when a source UUID is unavailable or disappears.
#[derive(Clone)]
struct BoundChange {
    collection: String,
    uuid: String,
    change: Change,
}

impl BoundChange {
    fn inverse(&self) -> Self {
        Self {
            collection: self.collection.clone(),
            uuid: self.uuid.clone(),
            change: self.change.inverse(),
        }
    }
}

enum Command {
    Connect { password: Option<String> },
    Disconnect,
    Refresh,
    Apply(Vec<BoundChange>),
    Restore,
}

struct Shared {
    snapshot: Mutex<Snapshot>,
    commands: Mutex<VecDeque<Command>>,
    wake: Condvar,
    stopping: Arc<AtomicBool>,
}

impl Shared {
    fn snapshot(&self) -> std::sync::MutexGuard<'_, Snapshot> {
        self.snapshot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn push(&self, command: Command) {
        let mut commands = self.commands.lock().unwrap_or_else(PoisonError::into_inner);
        // Keep at most one pending connection choice/password. A disconnect
        // supersedes queued connects; apply/restore are single-flight below.
        if matches!(&command, Command::Connect { .. }) {
            commands.retain(|item| !matches!(item, Command::Connect { .. }));
        } else if matches!(&command, Command::Disconnect) {
            commands.retain(|item| !matches!(item, Command::Connect { .. } | Command::Disconnect));
        }
        // A newer refresh makes a queued one redundant.
        if matches!(command, Command::Refresh)
            && commands.iter().any(|item| matches!(item, Command::Refresh))
        {
            return;
        }
        commands.push_back(command);
        self.wake.notify_one();
    }

    fn next(&self, wait: Duration) -> Option<Command> {
        let commands = self.commands.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut commands, _) = self
            .wake
            .wait_timeout_while(commands, wait, |commands| {
                commands.is_empty() && !self.stopping.load(Ordering::Acquire)
            })
            .unwrap_or_else(PoisonError::into_inner);
        commands.pop_front()
    }
}

/// Where OBS keeps its settings, and the port to use instead of the one in
/// them (tests point both at their own folder and server).
#[derive(Clone, Debug)]
pub(crate) struct ObsPaths {
    pub(crate) settings: Option<PathBuf>,
    pub(crate) port: Option<u16>,
}

impl ObsPaths {
    fn from_environment() -> Self {
        Self {
            settings: std::env::var_os("APPDATA")
                .map(|folder| Path::new(&folder).join("obs-studio")),
            port: None,
        }
    }
}

struct Preview {
    id: String,
    inspection: u64,
    changes: Vec<BoundChange>,
    created: Instant,
}

/// The control plane's handle on the OBS thread, which starts on first use.
pub(crate) struct ObsIntegration {
    shared: Arc<Shared>,
    paths: ObsPaths,
    started: bool,
    preview: Option<Preview>,
    previews: u64,
}

impl Default for ObsIntegration {
    fn default() -> Self {
        // This crate's tests never read the developer's own OBS settings.
        if cfg!(test) {
            return Self::new(ObsPaths {
                settings: None,
                port: None,
            });
        }
        Self::new(ObsPaths::from_environment())
    }
}

impl ObsIntegration {
    pub(crate) fn new(paths: ObsPaths) -> Self {
        Self {
            shared: Arc::new(Shared {
                snapshot: Mutex::new(Snapshot::default()),
                commands: Mutex::new(VecDeque::new()),
                wake: Condvar::new(),
                stopping: Arc::new(AtomicBool::new(false)),
            }),
            paths,
            started: false,
            preview: None,
            previews: 0,
        }
    }

    fn ensure_started(&mut self) {
        if self.started {
            return;
        }
        let (shared, paths) = (Arc::clone(&self.shared), self.paths.clone());
        let spawned = std::thread::Builder::new()
            .name("audiorouter-obs".into())
            .spawn(move || run(&shared, &paths));
        self.started = spawned.is_ok();
        if self.started {
            // Show the saved setup at once, before any connection.
            self.shared.push(Command::Refresh);
        }
    }

    pub(crate) fn status(&mut self) -> Value {
        self.ensure_started();
        let snapshot = self.shared.snapshot();
        json!({
            "installed": snapshot.installed,
            "connection": snapshot.connection.name(),
            "source": snapshot.source.map(|source| match source {
                Source::Live => "live",
                Source::SavedFile => "savedFile",
            }),
            "sceneCollection": snapshot.scene_collection,
            "inputs": snapshot.inputs.iter().map(ObsAudioInput::to_json).collect::<Vec<_>>(),
            "passwordSource": snapshot.password_source,
            "lastChange": {
                "state": snapshot.change.name(),
                "changed": snapshot.changed,
                "restorable": snapshot.restorable,
                "error": snapshot.change_error,
            },
        })
    }

    pub(crate) fn refresh(&mut self) -> Value {
        self.ensure_started();
        self.shared.push(Command::Refresh);
        json!({ "queued": true })
    }

    pub(crate) fn connect(&mut self, password: Option<String>) -> Value {
        self.ensure_started();
        self.preview = None;
        self.shared.snapshot().connection = Connection::Connecting;
        self.shared.push(Command::Connect { password });
        json!({ "connection": "connecting" })
    }

    pub(crate) fn disconnect(&mut self) -> Value {
        self.ensure_started();
        self.preview = None;
        self.shared.push(Command::Disconnect);
        json!({ "connection": "off" })
    }

    /// Check the requested changes against the latest live inspection and
    /// return them with the values they replace. Nothing is sent to OBS.
    pub(crate) fn preview(
        &mut self,
        requested: &[Value],
        endpoints: Option<&[EndpointInfo]>,
    ) -> Result<Value, String> {
        let snapshot = self.shared.snapshot();
        if snapshot.connection != Connection::Connected || snapshot.source != Some(Source::Live) {
            return Err("connect to OBS before previewing a change".into());
        }
        if matches!(
            snapshot.change,
            ChangeState::Applying | ChangeState::Restoring
        ) {
            return Err("an OBS change is already in progress".into());
        }
        let changes = plan_changes(&snapshot.inputs, requested, endpoints)?;
        let collection = snapshot
            .scene_collection
            .as_ref()
            .ok_or("OBS did not identify its scene collection; refresh before changing it")?;
        let changes = changes
            .into_iter()
            .map(|change| {
                let uuid = snapshot
                    .inputs
                    .iter()
                    .find(|input| input.name == change.input())
                    .and_then(|input| input.uuid.clone())
                    .ok_or("OBS did not identify this source; change it in OBS instead")?;
                Ok(BoundChange {
                    collection: collection.clone(),
                    uuid,
                    change,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let inspection = snapshot.inspection;
        drop(snapshot);
        self.previews += 1;
        let id = format!("obs-preview-{}", self.previews);
        let value = json!({
            "previewId": id,
            "changes": changes.iter().map(|bound| bound.change.to_json()).collect::<Vec<_>>(),
            "expiresInSeconds": OBS_PREVIEW_SECONDS,
        });
        self.preview = Some(Preview {
            id,
            inspection,
            changes,
            created: Instant::now(),
        });
        Ok(value)
    }

    pub(crate) fn apply(&mut self, preview_id: &str) -> Result<Value, String> {
        self.apply_at(preview_id, Instant::now())
    }

    // Tests advance from creation instead of backdating an Instant, which can
    // underflow on a newly booted Windows runner. Production uses the same clock.
    fn apply_at(&mut self, preview_id: &str, now: Instant) -> Result<Value, String> {
        let Some(preview) = self.preview.take_if(|preview| preview.id == preview_id) else {
            return Err("unknown or already used previewId; preview the change again".into());
        };
        if now.saturating_duration_since(preview.created) > Duration::from_secs(OBS_PREVIEW_SECONDS)
        {
            return Err("the preview expired; preview the change again".into());
        }
        {
            let mut snapshot = self.shared.snapshot();
            if snapshot.connection != Connection::Connected {
                return Err("OBS is not connected".into());
            }
            if matches!(
                snapshot.change,
                ChangeState::Applying | ChangeState::Restoring
            ) {
                return Err("an OBS change is already in progress".into());
            }
            if snapshot.inspection != preview.inspection {
                return Err("OBS changed since the preview; preview the change again".into());
            }
            snapshot.change = ChangeState::Applying;
            snapshot.change_error = None;
        }
        let count = preview.changes.len();
        self.shared.push(Command::Apply(preview.changes));
        Ok(json!({ "state": "applying", "changes": count }))
    }

    pub(crate) fn restore(&mut self) -> Result<Value, String> {
        self.preview = None;
        {
            let mut snapshot = self.shared.snapshot();
            if matches!(
                snapshot.change,
                ChangeState::Applying | ChangeState::Restoring
            ) {
                return Err("an OBS change is already in progress".into());
            }
            if !snapshot.restorable {
                return Err("there is no OBS change to restore".into());
            }
            if snapshot.connection != Connection::Connected {
                return Err("OBS is not connected".into());
            }
            snapshot.change = ChangeState::Restoring;
            snapshot.change_error = None;
        }
        self.shared.push(Command::Restore);
        Ok(json!({ "state": "restoring" }))
    }
}

impl Drop for ObsIntegration {
    fn drop(&mut self) {
        // Never join a socket thread on the control plane. Cancel queued
        // work/passwords and wake it; bounded I/O and request cancellation
        // release the connection after the in-flight operation returns.
        self.shared.stopping.store(true, Ordering::Release);
        self.shared
            .commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.shared.wake.notify_one();
    }
}

/// Turn `obs.preview` change requests into changes with their current
/// values. Each request is `{inputName, deviceId}` or `{inputName, muted}`.
pub(crate) fn plan_changes(
    inputs: &[ObsAudioInput],
    requested: &[Value],
    endpoints: Option<&[EndpointInfo]>,
) -> Result<Vec<Change>, String> {
    if requested.is_empty() || requested.len() > MAX_OBS_CHANGES {
        return Err(format!(
            "changes must list between 1 and {MAX_OBS_CHANGES} changes"
        ));
    }
    let mut changes: Vec<Change> = Vec::with_capacity(requested.len());
    for item in requested {
        let Some(object) = item.as_object() else {
            return Err("each change must be an object".into());
        };
        if let Some(field) = object
            .keys()
            .find(|key| !matches!(key.as_str(), "inputName" | "deviceId" | "muted"))
        {
            return Err(format!("unknown change field: {field}"));
        }
        let name = object
            .get("inputName")
            .and_then(Value::as_str)
            .ok_or("each change needs an inputName")?;
        let input = inputs
            .iter()
            .find(|input| input.name == name)
            .ok_or_else(|| format!("OBS has no audio source named {name:?}"))?;
        let change = match (object.get("deviceId"), object.get("muted")) {
            (Some(device), None) => {
                let after = device.as_str().ok_or("deviceId must be a string")?;
                let direction = match input.kind {
                    AudioKind::Input => EndpointDirection::Capture,
                    AudioKind::Output => EndpointDirection::Render,
                    AudioKind::Application => {
                        return Err(format!("{name:?} records an application, not a device"))
                    }
                };
                if after != "default" {
                    if after.is_empty() || after.len() > MAX_OBS_DEVICE_ID_BYTES {
                        return Err("deviceId is not a Windows endpoint ID".into());
                    }
                    if let Some(endpoints) = endpoints {
                        if !endpoints
                            .iter()
                            .any(|endpoint| endpoint.id == after && endpoint.direction == direction)
                        {
                            return Err(format!(
                                "{after:?} is not an active {} device",
                                match direction {
                                    EndpointDirection::Capture => "recording",
                                    EndpointDirection::Render => "playback",
                                }
                            ));
                        }
                    }
                }
                let before = input.device_id.clone().unwrap_or_else(|| "default".into());
                if before == after {
                    return Err(format!("{name:?} already uses that device"));
                }
                Change::Device {
                    input: name.into(),
                    before,
                    after: after.into(),
                }
            }
            (None, Some(muted)) => {
                let after = muted.as_bool().ok_or("muted must be a boolean")?;
                let before = input
                    .muted
                    .ok_or_else(|| format!("OBS did not report whether {name:?} is muted"))?;
                if before == after {
                    return Err(format!(
                        "{name:?} is already {}",
                        if after { "muted" } else { "unmuted" }
                    ));
                }
                Change::Muted {
                    input: name.into(),
                    before,
                    after,
                }
            }
            _ => return Err("each change sets exactly one of deviceId or muted".into()),
        };
        if changes.iter().any(|existing| {
            existing.input() == change.input() && existing.field() == change.field()
        }) {
            return Err(format!("{name:?} is changed twice"));
        }
        changes.push(change);
    }
    Ok(changes)
}

/// obs-websocket v5 authentication:
/// `base64(sha256(base64(sha256(password + salt)) + challenge))`.
pub(crate) fn authentication(password: &str, salt: &str, challenge: &str) -> String {
    let secret = base64(&Sha256::digest(format!("{password}{salt}").as_bytes()));
    base64(&Sha256::digest(format!("{secret}{challenge}").as_bytes()))
}

/// Standard base64 with padding (RFC 4648 §4).
pub(crate) fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let group = (u32::from(chunk[0]) << 16)
            | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        for index in 0..4 {
            if index <= chunk.len() {
                let sextet = (group >> (18 - 6 * index)) & 63;
                encoded.push(char::from(ALPHABET[sextet as usize]));
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

/// The parts of obs-websocket's `config.json` AudioRouter uses. Not `Debug`:
/// it holds the password.
#[derive(Default)]
pub(crate) struct WebSocketSettings {
    pub(crate) port: Option<u16>,
    pub(crate) auth_required: Option<bool>,
    pub(crate) password: Option<String>,
}

pub(crate) fn parse_websocket_settings(text: &str) -> WebSocketSettings {
    let Ok(Value::Object(settings)) = serde_json::from_str::<Value>(text) else {
        return WebSocketSettings::default();
    };
    WebSocketSettings {
        port: settings
            .get("server_port")
            .and_then(Value::as_u64)
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port >= 1024),
        auth_required: settings.get("auth_required").and_then(Value::as_bool),
        password: settings
            .get("server_password")
            .and_then(Value::as_str)
            .filter(|password| !password.is_empty() && password.len() <= MAX_OBS_PASSWORD_BYTES)
            .map(str::to_owned),
    }
}

fn read_bounded(path: &Path, limit: u64) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > limit {
        return None;
    }
    let mut text = String::new();
    // Enforce the limit on the read too: OBS may grow/replace a file after
    // the metadata check. Inspect the opened handle, not another path lookup.
    file.take(limit.checked_add(1)?)
        .read_to_string(&mut text)
        .ok()?;
    (u64::try_from(text.len()).ok()? <= limit).then_some(text)
}

fn websocket_settings(paths: &ObsPaths) -> WebSocketSettings {
    paths
        .settings
        .as_deref()
        .and_then(|folder| {
            read_bounded(
                &folder
                    .join("plugin_config")
                    .join("obs-websocket")
                    .join("config.json"),
                MAX_SETTINGS_FILE_BYTES,
            )
        })
        .map(|text| parse_websocket_settings(&text))
        .unwrap_or_default()
}

/// `SceneCollectionFile` from an OBS ini file's `[Basic]` section.
pub(crate) fn scene_collection_file(ini: &str) -> Option<String> {
    let mut in_basic = false;
    for line in ini.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_basic = line == "[Basic]";
        } else if in_basic {
            if let Some(value) = line.strip_prefix("SceneCollectionFile=") {
                let value = value.trim();
                // A bare file name: never a path out of the scenes folder.
                let safe = !value.is_empty()
                    && value.len() <= 128
                    && !value.contains(['/', '\\', ':'])
                    && !value.contains("..");
                return safe.then(|| value.to_owned());
            }
        }
    }
    None
}

/// The active scene collection, read from disk while OBS is closed. OBS 31
/// and later keep the setting in `user.ini`; older versions in `global.ini`.
fn saved_scene_collection(paths: &ObsPaths) -> Option<(Option<String>, Vec<ObsAudioInput>)> {
    let folder = paths.settings.as_deref()?;
    let file = ["user.ini", "global.ini"]
        .iter()
        .filter_map(|name| read_bounded(&folder.join(name), MAX_SETTINGS_FILE_BYTES))
        .find_map(|ini| scene_collection_file(&ini))?;
    let text = read_bounded(
        &folder
            .join("basic")
            .join("scenes")
            .join(format!("{file}.json")),
        MAX_SCENE_FILE_BYTES,
    )?;
    let document = serde_json::from_str::<Value>(&text).ok()?;
    Some(parse_scene_collection(&document))
}

fn bounded_string(value: Option<&Value>, limit: usize) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty() && text.len() <= limit)
        .map(str::to_owned)
}

/// OBS has six track flags. An incomplete/malformed response is unknown,
/// not an empty track list or an assumption that track 1 is enabled.
fn parse_audio_tracks(value: &Value) -> Option<Vec<u8>> {
    let tracks = value.as_object()?;
    let mut enabled = Vec::new();
    for track in 1..=6u8 {
        if tracks.get(&track.to_string())?.as_bool()? {
            enabled.push(track);
        }
    }
    Some(enabled)
}

/// The executable from an Application Audio Capture's `window` setting,
/// "title:class:executable" with ':' inside a part written as "#3A".
pub(crate) fn window_executable(window: &str) -> Option<String> {
    let executable = window.rsplit(':').next()?.replace("#3A", ":");
    (!executable.is_empty() && executable.len() <= MAX_OBS_NAME_BYTES).then_some(executable)
}

fn audio_input(
    name: String,
    uuid: Option<String>,
    kind: AudioKind,
    special: Option<&'static str>,
    settings: &Value,
    muted: Option<bool>,
) -> ObsAudioInput {
    let (device_id, application) = match kind {
        AudioKind::Application => (
            None,
            settings
                .get("window")
                .and_then(Value::as_str)
                .and_then(window_executable),
        ),
        // OBS leaves default values out of saved and reported settings.
        _ => (
            Some(
                bounded_string(settings.get("device_id"), MAX_OBS_DEVICE_ID_BYTES)
                    .unwrap_or_else(|| "default".into()),
            ),
            None,
        ),
    };
    ObsAudioInput {
        name,
        uuid,
        kind,
        special,
        device_id,
        application,
        muted,
        active: None,
        audio_tracks: None,
        monitor_only: None,
    }
}

fn saved_source(
    source: &Map<String, Value>,
    special: Option<&'static str>,
) -> Option<ObsAudioInput> {
    let kind = ["id", "versioned_id"]
        .iter()
        .filter_map(|key| source.get(*key).and_then(Value::as_str))
        .find_map(AudioKind::from_obs)?;
    let name = bounded_string(source.get("name"), MAX_OBS_NAME_BYTES)?;
    Some(audio_input(
        name,
        bounded_string(source.get("uuid"), MAX_OBS_NAME_BYTES),
        kind,
        special,
        source.get("settings").unwrap_or(&Value::Null),
        source.get("muted").and_then(Value::as_bool),
    ))
}

/// The audio sources of a saved scene collection document.
pub(crate) fn parse_scene_collection(document: &Value) -> (Option<String>, Vec<ObsAudioInput>) {
    let name = bounded_string(document.get("name"), MAX_OBS_NAME_BYTES);
    let mut inputs: Vec<ObsAudioInput> = Vec::new();
    for (special, key) in SPECIAL_INPUTS {
        if let Some(input) = document
            .get(key)
            .and_then(Value::as_object)
            .and_then(|source| saved_source(source, Some(special)))
        {
            inputs.push(input);
        }
    }
    for source in document
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
    {
        if inputs.len() >= MAX_OBS_AUDIO_INPUTS {
            break;
        }
        if let Some(input) = saved_source(source, None) {
            if !inputs.iter().any(|existing| existing.name == input.name) {
                inputs.push(input);
            }
        }
    }
    (name, inputs)
}

#[derive(Debug)]
enum RequestError {
    /// OBS answered that the request failed.
    Refused,
    /// The connection closed or broke.
    Lost,
}

struct LiveSession {
    socket: WebSocket<TcpStream>,
    requests: u64,
    stopping: Arc<AtomicBool>,
}

enum Incoming {
    Message(Value),
    Closed(Option<u16>),
}

fn read_until(socket: &mut WebSocket<TcpStream>, deadline: Instant) -> Result<Incoming, ()> {
    loop {
        if Instant::now() > deadline {
            return Err(());
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                return serde_json::from_str(text.as_str())
                    .map(Incoming::Message)
                    .map_err(|_| ())
            }
            Ok(Message::Close(frame)) => {
                return Ok(Incoming::Closed(frame.map(|frame| u16::from(frame.code))))
            }
            Ok(_) => {}
            Err(SocketError::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(SocketError::ConnectionClosed | SocketError::AlreadyClosed) => {
                return Ok(Incoming::Closed(None))
            }
            Err(_) => return Err(()),
        }
    }
}

fn send(socket: &mut WebSocket<TcpStream>, value: &Value) -> Result<(), ()> {
    socket
        .send(Message::Text(value.to_string().into()))
        .map_err(|_| ())
}

impl LiveSession {
    /// Connect and identify. The error is the state to show.
    fn open(
        port: u16,
        password: Option<&str>,
        stopping: Arc<AtomicBool>,
    ) -> Result<Self, Connection> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
            .map_err(|_| Connection::ServerOff)?;
        stream
            .set_read_timeout(Some(IDLE_READ))
            .map_err(|_| Connection::Failed)?;
        stream
            .set_write_timeout(Some(CONNECT_TIMEOUT))
            .map_err(|_| Connection::Failed)?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_MESSAGE_BYTES));
        let (mut socket, _) = tungstenite::client::client_with_config(
            format!("ws://127.0.0.1:{port}/"),
            stream,
            Some(config),
        )
        .map_err(|_| Connection::Failed)?;
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        let Ok(Incoming::Message(hello)) = read_until(&mut socket, deadline) else {
            return Err(Connection::Failed);
        };
        if hello["op"] != 0 {
            return Err(Connection::Failed);
        }
        let mut identify = json!({ "rpcVersion": 1, "eventSubscriptions": 0 });
        if let Some(challenge) = hello["d"]
            .get("authentication")
            .filter(|challenge| !challenge.is_null())
        {
            let (Some(salt), Some(challenge)) = (
                challenge.get("salt").and_then(Value::as_str),
                challenge.get("challenge").and_then(Value::as_str),
            ) else {
                return Err(Connection::Failed);
            };
            let Some(password) = password else {
                return Err(Connection::NeedsPassword);
            };
            identify["authentication"] = json!(authentication(password, salt, challenge));
        }
        send(&mut socket, &json!({ "op": 1, "d": identify })).map_err(|_| Connection::Failed)?;
        loop {
            match read_until(&mut socket, deadline) {
                Ok(Incoming::Message(message)) if message["op"] == 2 => {
                    return Ok(Self {
                        socket,
                        requests: 0,
                        stopping,
                    })
                }
                Ok(Incoming::Message(_)) => {}
                Ok(Incoming::Closed(Some(AUTHENTICATION_FAILED))) => {
                    return Err(Connection::WrongPassword)
                }
                Ok(Incoming::Closed(_)) | Err(()) => return Err(Connection::Failed),
            }
        }
    }

    fn request(&mut self, request_type: &str, data: Value) -> Result<Value, RequestError> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(RequestError::Lost);
        }
        self.requests += 1;
        let id = format!("audiorouter-{}", self.requests);
        send(
            &mut self.socket,
            &json!({ "op": 6, "d": { "requestType": request_type, "requestId": id, "requestData": data } }),
        )
        .map_err(|_| RequestError::Lost)?;
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        loop {
            match read_until(&mut self.socket, deadline) {
                Ok(Incoming::Message(message))
                    if message["op"] == 7 && message["d"]["requestId"] == id.as_str() =>
                {
                    let response = &message["d"];
                    if response["requestStatus"]["result"] != true {
                        return Err(RequestError::Refused);
                    }
                    return Ok(response
                        .get("responseData")
                        .cloned()
                        .unwrap_or_else(|| json!({})));
                }
                // Events are not subscribed; anything else is skipped.
                Ok(Incoming::Message(_)) => {}
                Ok(Incoming::Closed(_)) | Err(()) => return Err(RequestError::Lost),
            }
        }
    }

    /// Answer pings and notice a closed OBS. False when the connection is gone.
    fn keep_alive(&mut self) -> bool {
        match self.socket.read() {
            Ok(Message::Close(_)) => false,
            Ok(_) => true,
            Err(SocketError::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                // Sends queued pong replies; a timeout here is not a failure.
                let _ = self.socket.flush();
                true
            }
            Err(_) => false,
        }
    }

    fn close(mut self) {
        let _ = self.socket.close(None);
        let _ = self.socket.flush();
    }

    /// Read every audio source: OBS's global devices and each audio input.
    fn inspect(&mut self) -> Result<(Option<String>, Vec<ObsAudioInput>), RequestError> {
        let collection = self
            .request("GetSceneCollectionList", json!({}))
            .ok()
            .and_then(|data| {
                bounded_string(data.get("currentSceneCollectionName"), MAX_OBS_NAME_BYTES)
            });
        let specials = self.request("GetSpecialInputs", json!({}))?;
        let special_of = |name: &str| {
            SPECIAL_INPUTS
                .iter()
                .find(|(key, _)| specials.get(*key).and_then(Value::as_str) == Some(name))
                .map(|(key, _)| *key)
        };
        let list = self.request("GetInputList", json!({}))?;
        let candidates: Vec<(String, Option<String>, Option<AudioKind>)> = list
            .get("inputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| {
                let name = bounded_string(item.get("inputName"), MAX_OBS_NAME_BYTES)?;
                let kind = ["unversionedInputKind", "inputKind"]
                    .iter()
                    .filter_map(|key| item.get(*key).and_then(Value::as_str))
                    .find_map(AudioKind::from_obs)?;
                Some((
                    name,
                    bounded_string(item.get("inputUuid"), MAX_OBS_NAME_BYTES),
                    Some(kind),
                ))
            })
            .collect();
        // OBS's global devices may be missing from the input list; their kind
        // then comes from their settings.
        let mut ordered: Vec<(String, Option<String>, Option<AudioKind>)> = SPECIAL_INPUTS
            .iter()
            .filter_map(|(key, _)| bounded_string(specials.get(*key), MAX_OBS_NAME_BYTES))
            .filter(|name| !candidates.iter().any(|(existing, _, _)| existing == name))
            .map(|name| (name, None, None))
            .collect();
        ordered.extend(candidates);
        let candidates = ordered;
        let mut inputs = Vec::new();
        for (name, uuid, kind) in candidates.into_iter().take(MAX_OBS_AUDIO_INPUTS) {
            let settings = match self.request("GetInputSettings", json!({ "inputName": name })) {
                Ok(settings) => settings,
                Err(RequestError::Refused) => continue,
                Err(RequestError::Lost) => return Err(RequestError::Lost),
            };
            let Some(kind) = kind.or_else(|| {
                settings
                    .get("inputKind")
                    .and_then(Value::as_str)
                    .and_then(AudioKind::from_obs)
            }) else {
                continue;
            };
            let muted = match self.request("GetInputMute", json!({ "inputName": name })) {
                Ok(data) => data.get("inputMuted").and_then(Value::as_bool),
                Err(RequestError::Refused) => None,
                Err(RequestError::Lost) => return Err(RequestError::Lost),
            };
            let special = special_of(&name);
            let active = match self.request("GetSourceActive", json!({ "sourceName": name })) {
                Ok(data) => data.get("videoActive").and_then(Value::as_bool),
                Err(RequestError::Refused) => None,
                Err(RequestError::Lost) => return Err(RequestError::Lost),
            };
            let audio_tracks =
                match self.request("GetInputAudioTracks", json!({ "inputName": name })) {
                    Ok(data) => parse_audio_tracks(&data["inputAudioTracks"]),
                    Err(RequestError::Refused) => None,
                    Err(RequestError::Lost) => return Err(RequestError::Lost),
                };
            let monitor_only =
                match self.request("GetInputAudioMonitorType", json!({ "inputName": name })) {
                    Ok(data) => match data["monitorType"].as_str() {
                        Some(
                            "OBS_MONITORING_TYPE_NONE" | "OBS_MONITORING_TYPE_MONITOR_AND_OUTPUT",
                        ) => Some(false),
                        Some("OBS_MONITORING_TYPE_MONITOR_ONLY") => Some(true),
                        _ => None,
                    },
                    Err(RequestError::Refused) => None,
                    Err(RequestError::Lost) => return Err(RequestError::Lost),
                };
            let mut input = audio_input(
                name,
                uuid,
                kind,
                special,
                settings.get("inputSettings").unwrap_or(&Value::Null),
                muted,
            );
            input.active = active;
            input.audio_tracks = audio_tracks;
            input.monitor_only = monitor_only;
            inputs.push(input);
        }
        Ok((collection, inputs))
    }

    /// The current value of a change's field, as OBS reports it now.
    fn current_matches(&mut self, bound: &BoundChange) -> Result<bool, RequestError> {
        if !self.collection_matches(&bound.collection)? {
            return Ok(false);
        }
        // Renaming/deleting/recreating a source must invalidate both apply and
        // restore, even when the new source has the same name and device.
        let list = self.request("GetInputList", json!({}))?;
        let identity_matches = list["inputs"].as_array().is_some_and(|inputs| {
            inputs.iter().any(|input| {
                input["inputUuid"].as_str() == Some(bound.uuid.as_str())
                    && input["inputName"].as_str() == Some(bound.change.input())
            })
        });
        if !identity_matches {
            return Ok(false);
        }
        match &bound.change {
            Change::Device { before, .. } => {
                let settings =
                    self.request("GetInputSettings", json!({ "inputUuid": bound.uuid }))?;
                let current = settings["inputSettings"]
                    .get("device_id")
                    .and_then(Value::as_str)
                    .unwrap_or("default");
                Ok(current == before)
            }
            Change::Muted { before, .. } => {
                let data = self.request("GetInputMute", json!({ "inputUuid": bound.uuid }))?;
                Ok(data.get("inputMuted").and_then(Value::as_bool) == Some(*before))
            }
        }
    }

    fn collection_matches(&mut self, collection: &str) -> Result<bool, RequestError> {
        let data = self.request("GetSceneCollectionList", json!({}))?;
        Ok(data["currentSceneCollectionName"].as_str() == Some(collection))
    }

    fn set(&mut self, bound: &BoundChange) -> Result<(), RequestError> {
        match &bound.change {
            Change::Device { after, .. } => self
                .request(
                    "SetInputSettings",
                    json!({ "inputUuid": bound.uuid, "inputSettings": { "device_id": after }, "overlay": true }),
                )
                .map(drop),
            Change::Muted { after, .. } => self
                .request(
                    "SetInputMute",
                    json!({ "inputUuid": bound.uuid, "inputMuted": after }),
                )
                .map(drop),
        }
    }

    /// Make each change whose current value is still the one previewed, in
    /// order, stopping at the first that is not. Returns the changes made.
    fn apply(&mut self, changes: &[BoundChange]) -> (Vec<BoundChange>, Option<&'static str>) {
        let mut made = Vec::new();
        for change in changes {
            match self.current_matches(change) {
                Ok(true) => {}
                Ok(false) => return (made, Some("changedInObs")),
                Err(RequestError::Refused) => return (made, Some("refusedByObs")),
                Err(RequestError::Lost) => return (made, Some("connectionLost")),
            }
            // OBS has no compare-and-set request. Check again as late as
            // possible, then address the write by UUID, never by reused name.
            match self.collection_matches(&change.collection) {
                Ok(true) => {}
                Ok(false) => return (made, Some("changedInObs")),
                Err(RequestError::Refused) => return (made, Some("refusedByObs")),
                Err(RequestError::Lost) => return (made, Some("connectionLost")),
            }
            match self.set(change) {
                Ok(()) => made.push(change.clone()),
                Err(RequestError::Refused) => return (made, Some("refusedByObs")),
                Err(RequestError::Lost) => return (made, Some("connectionLost")),
            }
        }
        (made, None)
    }
}

fn show_saved(shared: &Shared, paths: &ObsPaths) {
    let installed = paths
        .settings
        .as_deref()
        .is_some_and(|folder| folder.is_dir());
    let saved = saved_scene_collection(paths);
    let mut snapshot = shared.snapshot();
    snapshot.installed = installed;
    match saved {
        Some((collection, inputs)) => {
            snapshot.source = Some(Source::SavedFile);
            snapshot.scene_collection = collection;
            snapshot.inputs = inputs;
        }
        None => {
            snapshot.source = None;
            snapshot.scene_collection = None;
            snapshot.inputs.clear();
        }
    }
}

fn show_live(shared: &Shared, collection: Option<String>, inputs: Vec<ObsAudioInput>) {
    let mut snapshot = shared.snapshot();
    snapshot.connection = Connection::Connected;
    // A successfully identified OBS server is stronger evidence than its
    // config folder (portable installations may keep settings elsewhere).
    snapshot.installed = true;
    snapshot.source = Some(Source::Live);
    snapshot.scene_collection = collection;
    snapshot.inputs = inputs;
    snapshot.inspection += 1;
}

/// The OBS thread: owns the connection, the password and the restore list.
fn run(shared: &Shared, paths: &ObsPaths) {
    let mut session: Option<LiveSession> = None;
    let mut wanted = false;
    let mut pasted: Option<String> = None;
    let mut restore: Vec<BoundChange> = Vec::new();
    while !shared.stopping.load(Ordering::Acquire) {
        let wait = if session.is_some() {
            IDLE_READ
        } else {
            IDLE_WAIT
        };
        let command = shared.next(wait);
        if shared.stopping.load(Ordering::Acquire) {
            break;
        }
        match command {
            Some(Command::Connect { password }) => {
                wanted = true;
                pasted = password;
                if let Some(old) = session.take() {
                    old.close();
                }
                session = connect(shared, paths, pasted.as_deref());
            }
            Some(Command::Disconnect) => {
                wanted = false;
                pasted = None;
                restore.clear();
                if let Some(old) = session.take() {
                    old.close();
                }
                {
                    let mut snapshot = shared.snapshot();
                    snapshot.connection = Connection::Off;
                    snapshot.password_source = None;
                    snapshot.restorable = false;
                    snapshot.change = ChangeState::None;
                    snapshot.changed = 0;
                    snapshot.change_error = None;
                }
                show_saved(shared, paths);
            }
            Some(Command::Refresh) => {
                if let Some(live) = session.as_mut() {
                    match live.inspect() {
                        Ok((collection, inputs)) => show_live(shared, collection, inputs),
                        Err(_) => session = lose(shared, paths, session.take()),
                    }
                } else if wanted {
                    // OBS may have started since: connect again.
                    session = connect(shared, paths, pasted.as_deref());
                } else {
                    show_saved(shared, paths);
                }
            }
            Some(Command::Apply(changes)) => {
                let Some(live) = session.as_mut() else {
                    finish_change(
                        shared,
                        ChangeState::Failed,
                        0,
                        Some("connectionLost"),
                        &restore,
                    );
                    continue;
                };
                let (made, error) = live.apply(&changes);
                if !made.is_empty() {
                    // Restore undoes this fix, most recent change first.
                    restore = made.iter().rev().map(BoundChange::inverse).collect();
                }
                let state = if error.is_none() {
                    ChangeState::Applied
                } else {
                    ChangeState::Failed
                };
                // Re-read OBS first, so "applied" is never shown with the old values.
                match live.inspect() {
                    Ok((collection, inputs)) => show_live(shared, collection, inputs),
                    Err(_) => session = lose(shared, paths, session.take()),
                }
                finish_change(shared, state, made.len(), error, &restore);
            }
            Some(Command::Restore) => {
                let Some(live) = session.as_mut() else {
                    finish_change(
                        shared,
                        ChangeState::Failed,
                        0,
                        Some("connectionLost"),
                        &restore,
                    );
                    continue;
                };
                let (made, error) = live.apply(&restore);
                // What was restored is no longer pending; a partial restore
                // keeps the rest available.
                restore = restore.split_off(made.len());
                let state = if error.is_none() {
                    ChangeState::Restored
                } else {
                    ChangeState::Failed
                };
                // Re-read OBS first, so "applied" is never shown with the old values.
                match live.inspect() {
                    Ok((collection, inputs)) => show_live(shared, collection, inputs),
                    Err(_) => session = lose(shared, paths, session.take()),
                }
                finish_change(shared, state, made.len(), error, &restore);
            }
            None => {
                if let Some(live) = session.as_mut() {
                    if !live.keep_alive() {
                        session = lose(shared, paths, session.take());
                    }
                }
            }
        }
    }
}

fn finish_change(
    shared: &Shared,
    state: ChangeState,
    changed: usize,
    error: Option<&'static str>,
    restore: &[BoundChange],
) {
    let mut snapshot = shared.snapshot();
    snapshot.change = state;
    snapshot.changed = changed;
    snapshot.change_error = error;
    snapshot.restorable = !restore.is_empty();
}

/// The connection broke (OBS closed): show the saved setup instead. A
/// later refresh reconnects.
fn lose(shared: &Shared, paths: &ObsPaths, session: Option<LiveSession>) -> Option<LiveSession> {
    drop(session);
    shared.snapshot().connection = Connection::ServerOff;
    show_saved(shared, paths);
    None
}

fn connect(shared: &Shared, paths: &ObsPaths, pasted: Option<&str>) -> Option<LiveSession> {
    shared.snapshot().connection = Connection::Connecting;
    let settings = websocket_settings(paths);
    let port = paths.port.or(settings.port).unwrap_or(OBS_DEFAULT_PORT);
    let (password, password_source) = match pasted {
        Some(password) => (Some(password.to_owned()), Some("pasted")),
        None => match settings.password {
            Some(password) if settings.auth_required != Some(false) => {
                (Some(password), Some("obsSettings"))
            }
            _ => (None, None),
        },
    };
    let opened = LiveSession::open(port, password.as_deref(), Arc::clone(&shared.stopping));
    drop(password);
    match opened {
        Ok(mut live) => match live.inspect() {
            Ok((collection, inputs)) => {
                shared.snapshot().password_source = password_source;
                show_live(shared, collection, inputs);
                Some(live)
            }
            Err(_) => {
                shared.snapshot().connection = Connection::Failed;
                show_saved(shared, paths);
                None
            }
        },
        Err(state) => {
            shared.snapshot().connection = state;
            show_saved(shared, paths);
            None
        }
    }
}

fn required_key(params: &Value) -> Result<&str, crate::ControlError> {
    params
        .get("idempotencyKey")
        .and_then(Value::as_str)
        .filter(|key| !key.is_empty())
        .ok_or_else(|| crate::ControlError::InvalidRequest("idempotencyKey is required".into()))
}

impl crate::ControlPlane {
    /// The `obs.*` methods. None waits on OBS: they read the OBS thread's
    /// latest state or queue work for it.
    pub(crate) fn dispatch_obs(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, crate::ControlError> {
        use crate::ControlError::InvalidRequest;
        let params = params.unwrap_or_else(|| json!({}));
        match method {
            "obs.status" => Ok(self.obs.status()),
            "obs.refresh" => Ok(self.obs.refresh()),
            "obs.connect" => {
                required_key(&params)?;
                let password = match params.get("password") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(password))
                        if !password.is_empty() && password.len() <= MAX_OBS_PASSWORD_BYTES =>
                    {
                        Some(password.clone())
                    }
                    Some(_) => {
                        return Err(InvalidRequest(format!(
                            "password must be a non-empty string of at most {MAX_OBS_PASSWORD_BYTES} bytes"
                        )))
                    }
                };
                // Never journaled: the request may carry a pasted password.
                Ok(self.obs.connect(password))
            }
            "obs.disconnect" => {
                required_key(&params)?;
                Ok(self.obs.disconnect())
            }
            "obs.preview" => {
                let changes = params
                    .get("changes")
                    .and_then(Value::as_array)
                    .ok_or_else(|| InvalidRequest("changes must be an array".into()))?;
                // Devices are checked against the latest endpoint snapshot
                // when one exists (the UI lists devices before previewing).
                let endpoints = self
                    .endpoint_monitor
                    .as_ref()
                    .map(|monitor| monitor.snapshot().to_vec());
                self.obs
                    .preview(changes, endpoints.as_deref())
                    .map_err(InvalidRequest)
            }
            "obs.apply" | "obs.restore" => {
                let key = required_key(&params)?.to_owned();
                let preview_id = if method == "obs.apply" {
                    Some(
                        params
                            .get("previewId")
                            .and_then(Value::as_str)
                            .filter(|id| !id.is_empty())
                            .ok_or_else(|| InvalidRequest("previewId is required".into()))?
                            .to_owned(),
                    )
                } else {
                    None
                };
                let scoped_key = self.scoped_idempotency_key(method, &key);
                let request_hash = Self::request_hash(&json!({ "previewId": preview_id }));
                if let Some(previous) = self.lookup_idempotent_result(&scoped_key, &request_hash)? {
                    return Ok(previous);
                }
                let result = match preview_id {
                    Some(preview_id) => self.obs.apply(&preview_id),
                    None => self.obs.restore(),
                }
                .map_err(InvalidRequest)?;
                self.journal_idempotent_result(&scoped_key, method, &request_hash, &result)?;
                Ok(result)
            }
            _ => Err(InvalidRequest("method not found".into())),
        }
    }
}

#[cfg(test)]
#[path = "obs_tests.rs"]
mod tests;
