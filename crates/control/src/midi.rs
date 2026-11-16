//! MIDI for Instrument tools (phase P8b): `midi.sendEvents` plays notes on a
//! running instrument from the on-screen keyboard, a script or an assistant,
//! and `midi.devices` lists MIDI keyboards and says which instruments hear
//! which device.
//!
//! These calls only push into the instrument bridge's bounded `MidiQueue`;
//! the bridge's own runtime thread sends them to the worker. Nothing here
//! touches the audio callback. A hardware keyboard reaches the same queue
//! through `audiorouter_windows_audio::MidiInputConnection`.

use super::*;
use audiorouter_plugin_host::{MidiEvent, MidiQueue};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Most events one `midi.sendEvents` call may carry.
pub(crate) const MAX_SEND_EVENTS: usize = 64;
/// Longest a scheduled note or its start delay may be.
pub(crate) const MAX_SCHEDULE_MS: u64 = 30_000;
/// Timed sequences running at once (each is one short-lived thread).
const MAX_SEQUENCES: usize = 8;

static ACTIVE_SEQUENCES: AtomicUsize = AtomicUsize::new(0);

/// The pitch-bend centre value of a 14-bit message.
const BEND_CENTRE: i32 = 8192;

/// One parsed request event: a message, when it starts (`after_ms`), and how
/// long a note lasts before its release (`duration_ms`).
#[derive(Debug, PartialEq)]
pub(crate) enum ParsedEvent {
    Message {
        event: MidiEvent,
        after_ms: u64,
        duration_ms: Option<u64>,
    },
    AllNotesOff,
}

fn bad(message: impl Into<String>) -> ControlError {
    ControlError::InvalidRequest(message.into())
}

fn byte(value: &Value, name: &str, minimum: u64, maximum: u64) -> Result<Option<u8>, ControlError> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .filter(|number| (minimum..=maximum).contains(number))
            .map(|number| Some(number as u8))
            .ok_or_else(|| {
                bad(format!(
                    "{name} must be a whole number from {minimum} to {maximum}"
                ))
            }),
    }
}

fn milliseconds(value: &Value, name: &str) -> Result<Option<u64>, ControlError> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .filter(|number| *number <= MAX_SCHEDULE_MS)
            .map(Some)
            .ok_or_else(|| {
                bad(format!(
                    "{name} must be a whole number of milliseconds up to {MAX_SCHEDULE_MS}"
                ))
            }),
    }
}

/// Turns the `events` array of `midi.sendEvents` into MIDI messages. Types:
/// `noteOn` (note, velocity 1-127, default 100; optional `durationMs` to
/// release it later), `noteOff` (note), `controlChange` (controller, value),
/// `pitchBend` (bend from -1 to 1), `aftertouch` (value, channel pressure)
/// and `allNotesOff`. `channel` is 1-16 (default 1); `afterMs` delays an
/// event, so one call can play a short phrase.
pub(crate) fn parse_events(events: &Value) -> Result<Vec<ParsedEvent>, ControlError> {
    let events = events
        .as_array()
        .filter(|events| !events.is_empty())
        .ok_or_else(|| bad("events must be a non-empty array"))?;
    if events.len() > MAX_SEND_EVENTS {
        return Err(bad(format!(
            "send at most {MAX_SEND_EVENTS} events per call"
        )));
    }
    let mut parsed = Vec::with_capacity(events.len());
    for event in events {
        let kind = event
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| bad("every event needs a type"))?;
        let allowed_fields: &[&str] = match kind {
            "noteOn" => &["type", "channel", "afterMs", "durationMs", "note", "velocity"],
            "noteOff" => &["type", "channel", "afterMs", "note"],
            "controlChange" => &["type", "channel", "afterMs", "controller", "value"],
            "pitchBend" => &["type", "channel", "afterMs", "bend"],
            "aftertouch" => &["type", "channel", "afterMs", "value"],
            "allNotesOff" => &["type"],
            other => {
                return Err(bad(format!(
                    "unknown event type \"{other}\"; use noteOn, noteOff, controlChange, pitchBend, aftertouch or allNotesOff"
                )))
            }
        };
        if let Some(object) = event.as_object() {
            if let Some(field) = object
                .keys()
                .find(|field| !allowed_fields.contains(&field.as_str()))
            {
                return Err(bad(format!("{field} is not valid for {kind}")));
            }
        }
        let channel = byte(event, "channel", 1, 16)?.unwrap_or(1) - 1;
        let after_ms = milliseconds(event, "afterMs")?.unwrap_or(0);
        let duration_ms = milliseconds(event, "durationMs")?;
        let message = |status: u8, data1: u8, data2: u8| {
            MidiEvent::new(0, status | channel, data1, data2)
                .map_err(|error| bad(format!("invalid MIDI event: {error:?}")))
        };
        let (event, duration_ms) = match kind {
            "allNotesOff" => {
                if events.len() != 1 {
                    return Err(bad("allNotesOff must be the only event"));
                }
                parsed.push(ParsedEvent::AllNotesOff);
                continue;
            }
            "noteOn" => {
                let note = byte(event, "note", 0, 127)?.ok_or_else(|| bad("noteOn needs a note"))?;
                let velocity = byte(event, "velocity", 1, 127)?.unwrap_or(100);
                (message(0x90, note, velocity)?, duration_ms)
            }
            "noteOff" => {
                let note =
                    byte(event, "note", 0, 127)?.ok_or_else(|| bad("noteOff needs a note"))?;
                (message(0x80, note, 0)?, None)
            }
            "controlChange" => {
                let controller = byte(event, "controller", 0, 127)?
                    .ok_or_else(|| bad("controlChange needs a controller"))?;
                let value = byte(event, "value", 0, 127)?
                    .ok_or_else(|| bad("controlChange needs a value"))?;
                if matches!(controller, 120 | 123) && (events.len() != 1 || after_ms != 0) {
                    return Err(bad(
                        "all-sound-off and all-notes-off controllers must be sent alone and immediately",
                    ));
                }
                (message(0xB0, controller, value)?, None)
            }
            "pitchBend" => {
                let bend = event
                    .get("bend")
                    .and_then(Value::as_f64)
                    .filter(|bend| (-1.0..=1.0).contains(bend))
                    .ok_or_else(|| bad("pitchBend needs bend from -1 to 1"))?;
                let amount = (BEND_CENTRE + (bend * f64::from(BEND_CENTRE - 1)).round() as i32)
                    .clamp(0, 16_383);
                (
                    message(0xE0, (amount & 0x7F) as u8, ((amount >> 7) & 0x7F) as u8)?,
                    None,
                )
            }
            "aftertouch" => {
                let value = byte(event, "value", 0, 127)?
                    .ok_or_else(|| bad("aftertouch needs a value"))?;
                (message(0xD0, value, 0)?, None)
            }
            other => {
                return Err(bad(format!(
                    "unknown event type \"{other}\"; use noteOn, noteOff, controlChange, pitchBend, aftertouch or allNotesOff"
                )))
            }
        };
        if duration_ms.is_some() && kind != "noteOn" {
            return Err(bad("durationMs is only valid for noteOn"));
        }
        parsed.push(ParsedEvent::Message {
            event,
            after_ms,
            duration_ms,
        });
    }
    Ok(parsed)
}

struct ActiveSequenceGuard;

impl Drop for ActiveSequenceGuard {
    fn drop(&mut self) {
        ACTIVE_SEQUENCES.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Pushes immediate messages and hands delayed events to one bounded
/// short-lived sequence. A sequence waits until the immediate batch is
/// accepted, and its generation is canceled by panic/overflow/CC all-notes-off.
/// Returns how many events were accepted immediately and how many scheduled.
pub(crate) fn play_events(
    queue: &Arc<MidiQueue>,
    events: Vec<ParsedEvent>,
) -> Result<(usize, usize), ControlError> {
    if matches!(events.as_slice(), [ParsedEvent::AllNotesOff]) {
        queue.release_all_notes();
        return Ok((1, 0));
    }
    if events
        .iter()
        .any(|event| matches!(event, ParsedEvent::AllNotesOff))
    {
        return Err(bad("allNotesOff must be sent by itself"));
    }
    // Capture the generation before any preparation or thread creation. If a
    // concurrent MIDI panic happens during that work, both immediate and
    // delayed messages from this request are rejected as stale.
    let generation = queue
        .schedule_generation()
        .ok_or_else(|| bad("could not read instrument note queue state"))?;

    let mut immediate = Vec::new();
    let mut timeline: Vec<(u64, MidiEvent)> = Vec::new();
    for event in events {
        match event {
            ParsedEvent::AllNotesOff => unreachable!("allNotesOff was handled above"),
            ParsedEvent::Message {
                event,
                after_ms,
                duration_ms,
            } => {
                let release = duration_ms
                    .map(|length| {
                        let due = after_ms
                            .checked_add(length)
                            .filter(|due| *due <= MAX_SCHEDULE_MS)
                            .ok_or_else(|| bad("afterMs plus durationMs must be at most 30000"))?;
                        MidiEvent::new(0, (event.status & 0x0F) | 0x80, event.data1, 0)
                            .map(|release| (due, release))
                            .map_err(|_| bad("invalid note release"))
                    })
                    .transpose()?;
                if after_ms == 0 {
                    immediate.push(event);
                    if let Some((0, release)) = release {
                        immediate.push(release);
                    } else if let Some((due, release)) = release {
                        timeline.push((due, release));
                    }
                } else {
                    timeline.push((after_ms, event));
                    if let Some((due, release)) = release {
                        timeline.push((due, release));
                    }
                }
            }
        }
    }

    let scheduled = timeline.len();
    let mut start_sequence = None;
    if !timeline.is_empty() {
        let mut active = ACTIVE_SEQUENCES.load(Ordering::Acquire);
        loop {
            if active >= MAX_SEQUENCES {
                return Err(bad(
                    "too many timed note sequences are playing; wait a moment",
                ));
            }
            match ACTIVE_SEQUENCES.compare_exchange_weak(
                active,
                active + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => active = current,
            }
        }
        let (start_sender, start_receiver) = std::sync::mpsc::sync_channel::<Option<u64>>(1);
        let queue_for_thread = Arc::clone(queue);
        let spawn = std::thread::Builder::new()
            .name("midi-sequence".into())
            .spawn(move || {
                let _guard = ActiveSequenceGuard;
                let Ok(Some(generation)) = start_receiver.recv() else {
                    return;
                };
                // Stable ordering preserves note-on before its same-time
                // release and caller order for simultaneous events.
                timeline.sort_by_key(|(at, _)| *at);
                let started = Instant::now();
                for (at, event) in timeline {
                    let due = started + std::time::Duration::from_millis(at);
                    while let Some(wait) = due.checked_duration_since(Instant::now()) {
                        if queue_for_thread.schedule_generation() != Some(generation) {
                            return;
                        }
                        std::thread::sleep(wait.min(std::time::Duration::from_millis(20)));
                    }
                    if queue_for_thread.push_scheduled(generation, &[event]) == 0 {
                        break;
                    }
                }
            });
        if let Err(error) = spawn {
            ACTIVE_SEQUENCES.fetch_sub(1, Ordering::AcqRel);
            return Err(bad(format!("could not start the note sequence: {error}")));
        }
        start_sequence = Some(start_sender);
    }

    let accepted = queue.push_for_generation(generation, &immediate);
    if accepted != immediate.len() {
        if let Some(start_sequence) = start_sequence {
            let _ = start_sequence.send(None);
        }
        return Ok((accepted, 0));
    }
    if let Some(start_sequence) = start_sequence {
        if start_sequence.send(Some(generation)).is_err() {
            queue.release_all_notes();
            return Err(bad("the note sequence stopped before it could start"));
        }
    }
    Ok((accepted, scheduled))
}

/// What an instrument asks of MIDI hardware: nothing (the on-screen keyboard
/// and the API only), every connected device, or one device by name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MidiSource {
    KeyboardOnly,
    All,
    Device(String),
}

fn midi_connection_is_open(
    requested: &MidiSource,
    opened_names: &[String],
    available_names: &[String],
) -> bool {
    match requested {
        MidiSource::KeyboardOnly => true,
        MidiSource::All => {
            !available_names.is_empty()
                && opened_names.len() == available_names.len()
                && available_names.iter().all(|available| {
                    opened_names
                        .iter()
                        .filter(|opened| *opened == available)
                        .count()
                        == 1
                        && available_names
                            .iter()
                            .filter(|name| *name == available)
                            .count()
                            == 1
                })
        }
        MidiSource::Device(name) => {
            available_names
                .iter()
                .filter(|available| *available == name)
                .count()
                == 1
                && opened_names.iter().filter(|opened| *opened == name).count() == 1
        }
    }
}

pub(crate) fn midi_source_of(node: &audiorouter_domain::Node) -> MidiSource {
    match node.parameters.get("midiInput").and_then(Value::as_str) {
        None | Some("") | Some("keyboard") => MidiSource::KeyboardOnly,
        Some("all") => MidiSource::All,
        Some(name) => MidiSource::Device(name.to_owned()),
    }
}

/// One open hardware connection and the instrument bridge it feeds. The weak
/// reference lets the control plane close the device once the route's bridge
/// is gone.
pub(crate) struct InstrumentMidiInputs {
    bridge: std::sync::Weak<audiorouter_plugin_host::PluginRuntimeBridge>,
    queue: std::sync::Weak<audiorouter_plugin_host::MidiQueue>,
    requested: MidiSource,
    opened_names: Vec<String>,
    last_connected: bool,
    lost_opened_names: std::collections::HashSet<String>,
    #[cfg(windows)]
    /// Held only to keep the devices open; dropping them closes the devices.
    _connections: Vec<audiorouter_windows_audio::MidiInputConnection>,
}

/// Reconcile one instrument with the current device inventory. When its last
/// requested hardware connection disappears, invalidate queued notes and
/// release notes already sounding; a later device reconnect cannot inherit
/// those held notes.
fn update_midi_connection(
    last_connected: &mut bool,
    lost_opened_names: &mut std::collections::HashSet<String>,
    requested: &MidiSource,
    opened_names: &[String],
    available: &[String],
    mut release_notes: impl FnMut(),
) -> bool {
    for opened in opened_names {
        let device_is_available = available.iter().any(|available| available == opened);
        if !device_is_available && lost_opened_names.insert(opened.clone()) {
            // An "All" source may already be reported disconnected because
            // one requested device failed to open. Still release notes when
            // any successfully opened device disappears later.
            release_notes();
        }
    }
    // An old winmm handle is not re-opened when its name returns. Require a
    // new connection (created when the route is prepared again).
    let connected =
        lost_opened_names.is_empty() && midi_connection_is_open(requested, opened_names, available);
    *last_connected = connected;
    connected
}

fn refresh_instrument_midi_inputs(
    inputs: &std::sync::Mutex<HashMap<(EntityId, EntityId), InstrumentMidiInputs>>,
    available: &[String],
) -> Vec<(EntityId, EntityId, MidiSource, bool)> {
    let snapshots = inputs
        .lock()
        .map(|inputs| {
            inputs
                .iter()
                .map(|(key, input)| {
                    (
                        key.clone(),
                        input.requested.clone(),
                        input.opened_names.clone(),
                        input.last_connected,
                        input.lost_opened_names.clone(),
                        input.queue.clone(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let updates = snapshots
        .into_iter()
        .map(
            |(key, requested, opened_names, last_connected, mut lost, queue)| {
                let mut connected = last_connected;
                let now_connected = update_midi_connection(
                    &mut connected,
                    &mut lost,
                    &requested,
                    &opened_names,
                    available,
                    || {
                        if let Some(queue) = queue.upgrade() {
                            queue.release_all_notes();
                        }
                    },
                );
                (key, requested, opened_names, connected, lost, now_connected)
            },
        )
        .collect::<Vec<_>>();
    if let Ok(mut inputs) = inputs.lock() {
        for (key, requested, opened_names, connected, lost, _) in &updates {
            if let Some(input) = inputs.get_mut(key).filter(|input| {
                &input.requested == requested && &input.opened_names == opened_names
            }) {
                input.last_connected = *connected;
                input.lost_opened_names.clone_from(lost);
            }
        }
    }
    updates
        .into_iter()
        .map(|((session_id, node_id), requested, _, _, _, connected)| {
            (session_id, node_id, requested, connected)
        })
        .collect()
}

fn service_midi_device_inventory(
    last_check: &mut Option<std::time::Instant>,
    pending: &mut Option<std::sync::mpsc::Receiver<Vec<String>>>,
    inputs: &std::sync::Mutex<HashMap<(EntityId, EntityId), InstrumentMidiInputs>>,
    now: std::time::Instant,
    enumerate: impl FnOnce() -> Vec<String> + Send + 'static,
) {
    if let Some(receiver) = pending.take() {
        match receiver.try_recv() {
            Ok(devices) => {
                refresh_instrument_midi_inputs(inputs, &devices);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                *pending = Some(receiver);
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
    }
    let has_hardware_inputs = inputs.lock().is_ok_and(|inputs| {
        inputs
            .values()
            .any(|input| !matches!(input.requested, MidiSource::KeyboardOnly))
    });
    if !has_hardware_inputs {
        *last_check = Some(now);
        return;
    }
    if last_check
        .is_some_and(|checked| now.saturating_duration_since(checked) < MIDI_DEVICE_POLL_INTERVAL)
    {
        return;
    }
    *last_check = Some(now);
    let (sender, receiver) = std::sync::mpsc::channel();
    *pending = Some(receiver);
    if std::thread::Builder::new()
        .name("audiorouter-midi-device-inventory".into())
        .spawn(move || {
            let _ = sender.send(enumerate());
        })
        .is_err()
    {
        *pending = None;
    }
}

impl ControlPlane {
    /// Periodically reconcile opened MIDI inputs with Windows' current device
    /// list. This service-owned check makes note release independent of UI or
    /// client polling `midi.devices`.
    pub(crate) fn service_instrument_midi_connections(&mut self, now: std::time::Instant) {
        if self.last_midi_inventory_check.is_none_or(|checked| {
            now.saturating_duration_since(checked) >= MIDI_DEVICE_POLL_INTERVAL
        }) {
            self.prune_instrument_midi();
        }
        service_midi_device_inventory(
            &mut self.last_midi_inventory_check,
            &mut self.midi_inventory_worker,
            &self.instrument_midi,
            now,
            || {
                #[cfg(windows)]
                {
                    audiorouter_windows_audio::list_midi_input_devices()
                        .into_iter()
                        .map(|device| device.name)
                        .collect::<Vec<_>>()
                }
                #[cfg(not(windows))]
                {
                    Vec::new()
                }
            },
        );
    }

    /// Closes MIDI devices whose instrument has stopped playing.
    pub(crate) fn prune_instrument_midi(&self) {
        if let Ok(mut inputs) = self.instrument_midi.lock() {
            inputs.retain(|_, input| input.bridge.strong_count() > 0);
        }
    }

    /// Opens the MIDI device(s) an instrument node asked for and feeds its
    /// queue. A device that is missing does not stop the route (the on-screen
    /// keyboard still works); `midi.devices` reports it as not connected, and
    /// no other device is ever substituted.
    pub(crate) fn connect_instrument_midi(
        &self,
        session_id: &EntityId,
        node: &audiorouter_domain::Node,
        bridge: &Arc<audiorouter_plugin_host::PluginRuntimeBridge>,
    ) {
        let requested = midi_source_of(node);
        #[cfg(windows)]
        let connections = {
            let sink_for = |queue: Arc<MidiQueue>| {
                move |status: u8, data1: u8, data2: u8| {
                    if let Ok(event) = MidiEvent::new(0, status, data1 & 0x7F, data2 & 0x7F) {
                        queue.push(&[event]);
                    }
                }
            };
            let names = match &requested {
                MidiSource::KeyboardOnly => Vec::new(),
                MidiSource::All => audiorouter_windows_audio::list_midi_input_devices()
                    .into_iter()
                    .map(|device| device.name)
                    .collect(),
                MidiSource::Device(name) => vec![name.clone()],
            };
            names
                .iter()
                .filter_map(|name| {
                    audiorouter_windows_audio::MidiInputConnection::open(
                        name,
                        sink_for(bridge.midi_queue()),
                    )
                    .ok()
                })
                .collect::<Vec<_>>()
        };
        #[cfg(windows)]
        let opened_names = connections
            .iter()
            .map(|connection| connection.name().to_owned())
            .collect::<Vec<String>>();
        #[cfg(not(windows))]
        let opened_names = Vec::new();
        #[cfg(windows)]
        let available_names = audiorouter_windows_audio::list_midi_input_devices()
            .into_iter()
            .map(|device| device.name)
            .collect::<Vec<_>>();
        #[cfg(not(windows))]
        let available_names: Vec<String> = Vec::new();
        let last_connected = midi_connection_is_open(&requested, &opened_names, &available_names);
        if let Ok(mut inputs) = self.instrument_midi.lock() {
            inputs.insert(
                (session_id.clone(), node.id.clone()),
                InstrumentMidiInputs {
                    bridge: Arc::downgrade(bridge),
                    queue: Arc::downgrade(&bridge.midi_queue()),
                    requested,
                    opened_names,
                    last_connected,
                    lost_opened_names: Default::default(),
                    #[cfg(windows)]
                    _connections: connections,
                },
            );
        }
    }

    /// `midi.sendEvents`: play notes on a running Instrument.
    pub(crate) fn dispatch_midi_send_events(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| bad("nodeId and events are required"))?;
        let events = parse_events(params.get("events").unwrap_or(&Value::Null))?;
        let (session_id, node_id) = self.running_instrument_ids(&params)?;
        let bridge = self.plugin_bridge(&session_id, &node_id)?;
        let queue = bridge.midi_queue();
        let (accepted, scheduled) = play_events(&queue, events)?;
        Ok(json!({
            "sessionId": session_id,
            "nodeId": node_id,
            "accepted": accepted,
            "scheduled": scheduled,
            "heldNotes": queue.held_notes()
        }))
    }

    /// The session and node of a running Instrument named by `params`.
    fn running_instrument_ids(&self, params: &Value) -> Result<(EntityId, EntityId), ControlError> {
        let session_id = self.simple_session_id(params)?;
        let reference = params
            .get("nodeId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| bad("nodeId is required"))?;
        let node = simple::resolve_node(self.get_session(&session_id)?, reference)?;
        if node.kind != NodeKind::Instrument {
            return Err(bad("nodeId must name an Instrument tool"));
        }
        let node_id = node.id.clone();
        if !self
            .runtimes
            .get(&session_id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
        {
            return Err(bad("start the route before playing its instrument"));
        }
        Ok((session_id, node_id))
    }

    /// `midi.devices`: connected MIDI keyboards, and for each Instrument of the
    /// running routes what it asked to hear and whether that is connected.
    pub(crate) fn dispatch_midi_devices(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        self.prune_instrument_midi();
        #[cfg(windows)]
        let devices = audiorouter_windows_audio::list_midi_input_devices()
            .into_iter()
            .map(|device| device.name)
            .collect::<Vec<_>>();
        #[cfg(not(windows))]
        let devices: Vec<String> = Vec::new();
        let instruments = refresh_instrument_midi_inputs(&self.instrument_midi, &devices)
            .into_iter()
            .map(|(session_id, node_id, requested, connected)| {
                let requested = match requested {
                    MidiSource::KeyboardOnly => "keyboard".to_owned(),
                    MidiSource::All => "all".to_owned(),
                    MidiSource::Device(name) => name,
                };
                json!({
                    "sessionId": session_id,
                    "nodeId": node_id,
                    "midiInput": requested,
                    "connected": connected
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({ "devices": devices, "instruments": instruments }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static SEQUENCE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn parsed(events: Value) -> Vec<ParsedEvent> {
        parse_events(&events).unwrap()
    }

    #[test]
    fn note_events_become_channel_voice_messages_on_the_chosen_channel() {
        let events = parsed(json!([
            { "type": "noteOn", "note": 60 },
            { "type": "noteOn", "note": 64, "velocity": 40, "channel": 3 },
            { "type": "noteOff", "note": 60 },
            { "type": "controlChange", "controller": 64, "value": 127 },
            { "type": "aftertouch", "value": 90 }
        ]));
        let status = |index: usize| match &events[index] {
            ParsedEvent::Message { event, .. } => (event.status, event.data1, event.data2),
            ParsedEvent::AllNotesOff => unreachable!(),
        };
        assert_eq!(status(0), (0x90, 60, 100), "default velocity and channel");
        assert_eq!(status(1), (0x92, 64, 40), "channel 3 is nibble 2");
        assert_eq!(status(2), (0x80, 60, 0));
        assert_eq!(status(3), (0xB0, 64, 127));
        assert_eq!(status(4), (0xD0, 90, 0));
    }

    #[test]
    fn pitch_bend_maps_minus_one_to_one_onto_fourteen_bits() {
        let value = |bend: f64| match &parsed(json!([{ "type": "pitchBend", "bend": bend }]))[0] {
            ParsedEvent::Message { event, .. } => {
                i32::from(event.data1) | (i32::from(event.data2) << 7)
            }
            ParsedEvent::AllNotesOff => unreachable!(),
        };
        assert_eq!(value(0.0), 8192);
        assert_eq!(value(1.0), 16_383);
        assert_eq!(value(-1.0), 1);
        assert!(parse_events(&json!([{ "type": "pitchBend", "bend": 1.5 }])).is_err());
    }

    #[test]
    fn a_malformed_request_is_refused_with_a_reason() {
        for (events, reason) in [
            (json!([]), "non-empty"),
            (json!("C4"), "non-empty"),
            (json!([{ "note": 60 }]), "type"),
            (json!([{ "type": "noteOn" }]), "note"),
            (json!([{ "type": "noteOn", "note": 128 }]), "0 to 127"),
            (
                json!([{ "type": "noteOn", "note": 60, "velocity": 0 }]),
                "1 to 127",
            ),
            (
                json!([{ "type": "noteOn", "note": 60, "channel": 17 }]),
                "1 to 16",
            ),
            (
                json!([{ "type": "noteOn", "note": 60, "durationMs": 99999 }]),
                "milliseconds",
            ),
            (
                json!([{ "type": "noteOff", "note": 60, "velocity": 20 }]),
                "not valid for noteOff",
            ),
            (
                json!([{ "type": "noteOn", "note": 60, "controller": 1 }]),
                "not valid for noteOn",
            ),
            (
                json!([{ "type": "controlChange", "controller": 123, "value": 0, "afterMs": 10 }]),
                "alone and immediately",
            ),
            (
                json!([{ "type": "allNotesOff", "channel": 2 }]),
                "not valid for allNotesOff",
            ),
            (json!([{ "type": "sysex" }]), "unknown event type"),
        ] {
            let error = parse_events(&events).expect_err("must be refused");
            let text = format!("{error:?}");
            assert!(text.contains(reason), "{events}: {text}");
        }
        let many = json!(vec![
            json!({ "type": "noteOff", "note": 1 });
            MAX_SEND_EVENTS + 1
        ]);
        assert!(parse_events(&many).is_err());
    }

    #[test]
    fn immediate_events_reach_the_queue_and_timed_notes_release_themselves() {
        let _guard = SEQUENCE_TEST_LOCK.lock().unwrap();
        let queue = Arc::new(MidiQueue::default());
        let (accepted, scheduled) = play_events(
            &queue,
            parsed(json!([
                { "type": "noteOn", "note": 60 },
                { "type": "noteOn", "note": 64, "durationMs": 30 }
            ])),
        )
        .unwrap();
        assert_eq!((accepted, scheduled), (2, 1));
        assert_eq!(queue.held_notes(), 2);
        // The 30 ms note releases itself; the other stays until released.
        let deadline = Instant::now() + std::time::Duration::from_secs(2);
        while queue.held_notes() > 1 && Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(queue.held_notes(), 1);
        // The panic button releases the rest explicitly.
        play_events(&queue, parsed(json!([{ "type": "allNotesOff" }]))).unwrap();
        assert_eq!(queue.held_notes(), 0);
    }

    #[test]
    fn cancelled_timed_sequences_release_capacity_promptly() {
        let _guard = SEQUENCE_TEST_LOCK.lock().unwrap();
        let queue = Arc::new(MidiQueue::default());
        for note in 0..MAX_SEQUENCES {
            play_events(
                &queue,
                parsed(json!([{ "type": "noteOn", "note": note, "afterMs": 2000 }])),
            )
            .unwrap();
        }
        queue.release_all_notes();
        let deadline = Instant::now() + std::time::Duration::from_millis(500);
        while ACTIVE_SEQUENCES.load(Ordering::Acquire) != 0 && Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(ACTIVE_SEQUENCES.load(Ordering::Acquire), 0);
        assert!(play_events(
            &queue,
            parsed(json!([{ "type": "noteOn", "note": 60, "durationMs": 1 }]))
        )
        .is_ok());
        queue.release_all_notes();
        let deadline = Instant::now() + std::time::Duration::from_millis(500);
        while ACTIVE_SEQUENCES.load(Ordering::Acquire) != 0 && Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(ACTIVE_SEQUENCES.load(Ordering::Acquire), 0);
    }

    #[test]
    fn device_disconnect_releases_held_notes_without_a_status_request() {
        let queue = Arc::new(MidiQueue::default());
        let note_on = MidiEvent::new(0, 0x90, 60, 100).unwrap();
        assert_eq!(queue.push(&[note_on]), 1);
        assert_eq!(queue.held_notes(), 1);
        let queue_for_loss = Arc::downgrade(&queue);
        let mut connected = true;
        let mut lost_opened_names = std::collections::HashSet::new();
        let next = update_midi_connection(
            &mut connected,
            &mut lost_opened_names,
            &MidiSource::Device("USB Keyboard".into()),
            &["USB Keyboard".into()],
            &[],
            || {
                queue_for_loss
                    .upgrade()
                    .expect("instrument bridge still owns the queue")
                    .release_all_notes();
            },
        );
        assert!(!next);
        assert!(!connected);
        assert_eq!(
            queue.held_notes(),
            0,
            "lost input must enqueue note release"
        );
        // A returned name does not prove the old Windows handle reconnected.
        let mut releases = 0;
        update_midi_connection(
            &mut connected,
            &mut lost_opened_names,
            &MidiSource::Device("USB Keyboard".into()),
            &["USB Keyboard".into()],
            &[],
            || releases += 1,
        );
        assert_eq!(releases, 0);
        assert!(!update_midi_connection(
            &mut connected,
            &mut lost_opened_names,
            &MidiSource::Device("USB Keyboard".into()),
            &["USB Keyboard".into()],
            &["USB Keyboard".into()],
            || releases += 1,
        ));
        assert!(!update_midi_connection(
            &mut connected,
            &mut lost_opened_names,
            &MidiSource::Device("USB Keyboard".into()),
            &["USB Keyboard".into()],
            &[],
            || releases += 1,
        ));
        assert_eq!(releases, 0);
    }

    #[test]
    fn all_devices_disconnect_releases_each_opened_device_even_if_already_partial() {
        let mut connected = false; // "All" was already partial (one device failed to open).
        let opened = ["Launchkey".into()];
        let mut lost = std::collections::HashSet::new();
        let mut releases = 0;
        assert!(!update_midi_connection(
            &mut connected,
            &mut lost,
            &MidiSource::All,
            &opened,
            &["Launchkey".into(), "Pad".into()],
            || releases += 1,
        ));
        assert_eq!(releases, 0);
        assert!(!update_midi_connection(
            &mut connected,
            &mut lost,
            &MidiSource::All,
            &opened,
            &["Pad".into()],
            || releases += 1,
        ));
        assert_eq!(
            releases, 1,
            "the opened keyboard disappearing releases notes"
        );
        update_midi_connection(
            &mut connected,
            &mut lost,
            &MidiSource::All,
            &opened,
            &["Pad".into()],
            || releases += 1,
        );
        assert_eq!(releases, 1, "one disconnect edge releases once");
    }

    #[test]
    fn audio_service_enumerates_devices_and_releases_notes_outside_status_calls() {
        let queue = Arc::new(MidiQueue::default());
        let note_on = MidiEvent::new(0, 0x90, 60, 100).unwrap();
        queue.push(&[note_on]);
        let key = (EntityId::new("session"), EntityId::new("instrument"));
        let inputs = std::sync::Mutex::new(HashMap::from([(
            key,
            InstrumentMidiInputs {
                bridge: std::sync::Weak::new(),
                queue: Arc::downgrade(&queue),
                requested: MidiSource::Device("USB Keyboard".into()),
                opened_names: vec!["USB Keyboard".into()],
                last_connected: true,
                lost_opened_names: Default::default(),
                #[cfg(windows)]
                _connections: Vec::new(),
            },
        )]));
        let start = Instant::now();
        let mut checked = None;
        let mut pending = None;
        let enumerations = Arc::new(AtomicUsize::new(0));
        let started = Arc::clone(&enumerations);
        service_midi_device_inventory(&mut checked, &mut pending, &inputs, start, move || {
            started.fetch_add(1, Ordering::Relaxed);
            Vec::new()
        });
        let deadline = Instant::now() + std::time::Duration::from_secs(2);
        while pending.is_some() && Instant::now() < deadline {
            service_midi_device_inventory(&mut checked, &mut pending, &inputs, start, Vec::new);
            std::thread::yield_now();
        }
        assert!(pending.is_none(), "inventory worker must complete");
        assert_eq!(enumerations.load(Ordering::Relaxed), 1);
        assert_eq!(queue.held_notes(), 0);
        assert!(
            !inputs
                .lock()
                .unwrap()
                .values()
                .next()
                .unwrap()
                .last_connected
        );

        service_midi_device_inventory(
            &mut checked,
            &mut pending,
            &inputs,
            start + MIDI_DEVICE_POLL_INTERVAL / 2,
            Vec::new,
        );
        assert_eq!(
            enumerations.load(Ordering::Relaxed),
            1,
            "device enumeration is bounded by cadence"
        );
        service_midi_device_inventory(
            &mut checked,
            &mut pending,
            &inputs,
            start + MIDI_DEVICE_POLL_INTERVAL,
            Vec::new,
        );
        assert!(pending.is_some());
    }

    fn call(plane: &mut ControlPlane, method: &str, params: Value) -> JsonRpcResponse {
        plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        })
    }

    fn instrument_session() -> Session {
        let node = |id: &str, kind: NodeKind| audiorouter_domain::Node {
            id: EntityId::new(id),
            name: id.into(),
            kind,
            type_version: 1,
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![audiorouter_domain::Port {
                name: "out".into(),
                direction: PortDirection::Output,
                channels: 2,
            }],
        };
        Session {
            id: EntityId::new("midi-session"),
            name: "midi".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![
                node("Piano", NodeKind::Instrument),
                node("tone", NodeKind::TestSignal),
            ],
            edges: vec![],
        }
    }

    /// Through `ControlPlane::dispatch`, the real entry point: the node is
    /// resolved by name in the backend, and every refusal says what to do.
    #[test]
    fn midi_send_events_is_refused_with_a_reason_until_an_instrument_plays() {
        let mut plane = ControlPlane::default();
        plane.insert_session(instrument_session()).unwrap();
        let events = json!([{ "type": "noteOn", "note": 60 }]);
        let message = |response: JsonRpcResponse| response.error.expect("refused").message;
        // A node that is not an Instrument.
        let text = message(call(
            &mut plane,
            "midi.sendEvents",
            json!({ "sessionId": "midi-session", "nodeId": "tone", "events": events }),
        ));
        assert!(text.contains("Instrument"), "{text}");
        // An Instrument named by its exact name, while nothing plays.
        let text = message(call(
            &mut plane,
            "midi.sendEvents",
            json!({ "sessionId": "midi-session", "nodeId": "piano", "events": events }),
        ));
        assert!(text.contains("start the route"), "{text}");
        // A malformed event is refused before anything else is looked at.
        let text = message(call(
            &mut plane,
            "midi.sendEvents",
            json!({ "sessionId": "midi-session", "nodeId": "Piano", "events": [{ "type": "noteOn", "note": 200 }] }),
        ));
        assert!(text.contains("0 to 127"), "{text}");
        // A parameter outside the request schema is refused.
        let text = message(call(
            &mut plane,
            "midi.sendEvents",
            json!({ "nodeId": "Piano", "events": events, "extra": 1 }),
        ));
        assert!(text.contains("extra"), "{text}");
    }

    #[test]
    fn midi_send_events_uses_the_active_session_and_resolves_an_exact_node_name() {
        let mut plane = ControlPlane::default();
        let session = instrument_session();
        plane.active_session_id = Some(session.id.clone());
        plane.insert_session(session).unwrap();

        // This test reaches the target resolver before the expected stopped
        // route refusal; IDs remain what reaches the runtime bridge.
        let response = call(
            &mut plane,
            "midi.sendEvents",
            json!({ "nodeId": "PIANO", "events": [{ "type": "noteOn", "note": 60 }] }),
        );
        assert!(response.error.unwrap().message.contains("start the route"));

        let response = call(
            &mut plane,
            "midi.sendEvents",
            json!({ "nodeId": "absent", "events": [{ "type": "noteOn", "note": 60 }] }),
        );
        assert!(response.error.unwrap().message.contains("no node named"));
    }

    #[test]
    fn midi_devices_lists_hardware_and_never_fails_without_any() {
        let mut plane = ControlPlane::default();
        let response = call(&mut plane, "midi.devices", json!({}));
        let result = response.result.expect("midi.devices answers");
        assert!(result["devices"].is_array());
        assert_eq!(result["instruments"], json!([]));
    }

    #[test]
    fn the_midi_source_of_a_node_follows_its_midi_input_setting() {
        let node = |value: Option<&str>| {
            let mut node = audiorouter_domain::Node {
                id: EntityId::new("piano"),
                name: "piano".into(),
                kind: NodeKind::Instrument,
                type_version: 1,
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![],
            };
            if let Some(value) = value {
                node.parameters.insert("midiInput".into(), json!(value));
            }
            node
        };
        assert_eq!(midi_source_of(&node(None)), MidiSource::KeyboardOnly);
        assert_eq!(
            midi_source_of(&node(Some("keyboard"))),
            MidiSource::KeyboardOnly
        );
        assert_eq!(midi_source_of(&node(Some("all"))), MidiSource::All);
        assert_eq!(
            midi_source_of(&node(Some("Launchkey 49"))),
            MidiSource::Device("Launchkey 49".into())
        );
    }

    #[test]
    fn midi_device_status_requires_an_open_connection_and_all_requested_devices() {
        let keyboard = MidiSource::KeyboardOnly;
        assert!(midi_connection_is_open(&keyboard, &[], &[]));

        let device = MidiSource::Device("Launchkey".into());
        assert!(midi_connection_is_open(
            &device,
            &["Launchkey".into()],
            &["Launchkey".into()]
        ));
        assert!(!midi_connection_is_open(
            &device,
            &[],
            &["Launchkey".into()]
        ));
        assert!(!midi_connection_is_open(
            &device,
            &["Launchkey".into()],
            &[]
        ));
        assert!(!midi_connection_is_open(
            &device,
            &["Launchkey".into()],
            &["Launchkey".into(), "Launchkey".into()]
        ));

        let all = MidiSource::All;
        assert!(midi_connection_is_open(
            &all,
            &["Launchkey".into(), "Pad".into()],
            &["Launchkey".into(), "Pad".into()]
        ));
        assert!(!midi_connection_is_open(
            &all,
            &["Launchkey".into()],
            &["Launchkey".into(), "Pad".into()]
        ));
    }
}
