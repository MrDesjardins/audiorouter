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
                    if let Some(wait) = due.checked_duration_since(Instant::now()) {
                        std::thread::sleep(wait);
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
#[derive(Debug, Eq, PartialEq)]
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
                && available_names
                    .iter()
                    .all(|name| opened_names.iter().any(|opened| opened == name))
        }
        MidiSource::Device(name) => {
            available_names.iter().any(|available| available == name)
                && opened_names.iter().any(|opened| opened == name)
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
    requested: MidiSource,
    opened_names: Vec<String>,
    last_connected: bool,
    #[cfg(windows)]
    /// Held only to keep the devices open; dropping them closes the devices.
    _connections: Vec<audiorouter_windows_audio::MidiInputConnection>,
}

impl ControlPlane {
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
                    requested,
                    opened_names,
                    last_connected,
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
        let instruments = self
            .instrument_midi
            .lock()
            .map(|mut inputs| {
                inputs
                    .iter_mut()
                    .map(|((session_id, node_id), input)| {
                        let requested = match &input.requested {
                            MidiSource::KeyboardOnly => "keyboard".to_owned(),
                            MidiSource::All => "all".to_owned(),
                            MidiSource::Device(name) => name.clone(),
                        };
                        let connected = midi_connection_is_open(
                            &input.requested,
                            &input.opened_names,
                            &devices,
                        );
                        if input.last_connected && !connected {
                            // No MIDI input means no more key-up will arrive.
                            // Release every held note immediately when the
                            // status read discovers the device disappeared.
                            if let Some(bridge) = input.bridge.upgrade() {
                                bridge.midi_queue().release_all_notes();
                            }
                        }
                        input.last_connected = connected;
                        json!({
                            "sessionId": session_id,
                            "nodeId": node_id,
                            "midiInput": requested,
                            "connected": connected
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(json!({ "devices": devices, "instruments": instruments }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
