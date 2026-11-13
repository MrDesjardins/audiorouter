//! Simulated-clock tests of a multi-device route.
//!
//! Capture and render devices here run on one virtual clock, each at its own
//! sample rate, and the real feeder, mixer and output pumps move the audio.
//! That reproduces what real devices do over hours (clock drift, a stalled
//! input) in a few seconds of test time, with exact counts. See
//! `docs/plans/active/audio-latency.md`.

use super::*;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

const RATE: f64 = 48_000.0;
const QUANTUM: usize = 128;
const PACKET: usize = 480;
const GENERATION: u64 = 4;
/// One audio thread pass every 1.5 ms, as measured on the owner's PC.
const PASS_SECONDS: f64 = 0.001_5;
/// Each input carries this tone.
const TONE_HZ: f64 = 440.0;
const TONE_LEVEL: f64 = 0.1;

type Clock = Rc<Cell<f64>>;

/// A capture device that delivers 10 ms packets on its own clock.
struct SimCapture {
    clock: Clock,
    rate: f64,
    /// Frames per packet (10 ms unless a test says otherwise).
    packet: usize,
    /// Packets the device buffer holds before it loses the oldest.
    capacity_packets: usize,
    /// Timestamp each packet with its capture time, as WASAPI does.
    stamped: bool,
    /// Timestamp and number of each queued packet.
    queue: RefCell<VecDeque<(u64, u64)>>,
    produced_packets: Cell<u64>,
    overflowed_packets: Cell<u64>,
    /// The device delivers nothing in this interval, then all of it at once.
    stall: Cell<Option<(f64, f64)>>,
}

impl SimCapture {
    fn new(clock: &Clock, ppm: f64, capacity_packets: usize) -> Self {
        Self {
            clock: Rc::clone(clock),
            rate: RATE * (1.0 + ppm / 1e6),
            packet: PACKET,
            capacity_packets,
            stamped: true,
            queue: RefCell::new(VecDeque::new()),
            produced_packets: Cell::new(0),
            overflowed_packets: Cell::new(0),
            stall: Cell::new(None),
        }
    }

    fn advance(&self) {
        let now = self.clock.get();
        if self
            .stall
            .get()
            .is_some_and(|(from, to)| now >= from && now < to)
        {
            return;
        }
        loop {
            let produced = self.produced_packets.get();
            let first_frame_at = (produced * self.packet as u64) as f64 / self.rate;
            let complete_at = ((produced + 1) * self.packet as u64) as f64 / self.rate;
            if complete_at > now {
                return;
            }
            self.produced_packets.set(produced + 1);
            let mut queue = self.queue.borrow_mut();
            if queue.len() == self.capacity_packets {
                queue.pop_front();
                self.overflowed_packets
                    .set(self.overflowed_packets.get() + 1);
            }
            let stamp = if self.stamped {
                (first_frame_at * 1e7) as u64
            } else {
                0
            };
            queue.push_back((stamp, produced));
        }
    }
}

impl AudioCaptureSource for SimCapture {
    fn next_packet_into(
        &self,
        destination: &mut [u8],
        bytes_per_frame: usize,
    ) -> Result<Option<(CapturePacket, usize)>, AudioError> {
        let Some((qpc_position, packet)) = self.queue.borrow_mut().pop_front() else {
            return Ok(None);
        };
        let bytes = self.packet * bytes_per_frame;
        // A tone that is continuous across packets, so a lost or repeated
        // stretch shows as a step at the device.
        for (frame, sample) in destination[..bytes].chunks_exact_mut(4).enumerate() {
            let position = (packet * self.packet as u64 + frame as u64) as f64;
            let value = TONE_LEVEL * (std::f64::consts::TAU * TONE_HZ * position / RATE).sin();
            sample.copy_from_slice(&(value as f32).to_ne_bytes());
        }
        Ok(Some((
            CapturePacket {
                frames: self.packet as u32,
                flags: 0,
                device_position: 0,
                qpc_position,
            },
            bytes,
        )))
    }

    fn has_queued_packet(&self) -> bool {
        !self.queue.borrow().is_empty()
    }
}

/// A render device that plays one 10 ms period at a time on its own clock.
struct SimDevice {
    clock: Clock,
    rate: f64,
    capacity_frames: u32,
    /// Frames per device period (10 ms unless a test says otherwise).
    packet: usize,
    queued_frames: Cell<u32>,
    played_periods: Cell<u64>,
    started: Cell<bool>,
    /// Frames the device wanted after start and did not have.
    starved_frames: Cell<u64>,
    last_sample: Cell<f32>,
    /// Largest change between two consecutive samples received.
    largest_step: Cell<f32>,
}

impl SimDevice {
    fn new(clock: &Clock, ppm: f64) -> Self {
        Self {
            clock: Rc::clone(clock),
            rate: RATE * (1.0 + ppm / 1e6),
            // The 50 ms device buffer the control plane requests.
            capacity_frames: 2_400,
            packet: PACKET,
            queued_frames: Cell::new(0),
            played_periods: Cell::new(0),
            started: Cell::new(false),
            starved_frames: Cell::new(0),
            last_sample: Cell::new(0.0),
            largest_step: Cell::new(0.0),
        }
    }

    fn advance(&self) {
        let now = self.clock.get();
        loop {
            let played = self.played_periods.get();
            if ((played + 1) * self.packet as u64) as f64 / self.rate > now {
                return;
            }
            self.played_periods.set(played + 1);
            if !self.started.get() {
                continue;
            }
            let queued = self.queued_frames.get();
            let taken = queued.min(self.packet as u32);
            self.queued_frames.set(queued - taken);
            self.starved_frames
                .set(self.starved_frames.get() + u64::from(self.packet as u32 - taken));
        }
    }
}

impl RenderSink for SimDevice {
    fn submit_bytes(&self, source: &[u8], bytes_per_frame: usize) -> Result<u32, AudioError> {
        let free = self.capacity_frames - self.queued_frames.get();
        let frames = free.min((source.len() / bytes_per_frame) as u32);
        self.queued_frames.set(self.queued_frames.get() + frames);
        for sample in source[..frames as usize * bytes_per_frame].chunks_exact(4) {
            let sample = f32::from_ne_bytes(sample.try_into().unwrap());
            self.largest_step.set(
                self.largest_step
                    .get()
                    .max((sample - self.last_sample.get()).abs()),
            );
            self.last_sample.set(sample);
        }
        if frames != 0 {
            self.started.set(true);
        }
        Ok(frames)
    }

    fn queued_frames(&self) -> Option<u32> {
        Some(self.queued_frames.get())
    }

    fn clock_100ns(&self) -> Option<u64> {
        Some((self.clock.get() * 1e7) as u64)
    }
}

/// Two mono inputs into a Mixer that plays to two outputs.
fn two_input_two_output_mixer() -> audiorouter_engine::RealtimeMixerFanout {
    use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};

    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 1,
    };
    let node = |id: &str, kind, ports: Vec<Port>| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports,
    };
    let edge = |source: &str, source_port: &str, destination: &str| Edge {
        id: EntityId::new(format!("{source}-{destination}")),
        source_node: EntityId::new(source),
        source_port: source_port.into(),
        destination_node: EntityId::new(destination),
        destination_port: "main".into(),
        matrix: vec![1.0],
        enabled: true,
    };
    let input = |id: &str| {
        node(
            id,
            NodeKind::PhysicalInput,
            vec![port("main", PortDirection::Output)],
        )
    };
    let output = |id: &str| {
        node(
            id,
            NodeKind::PhysicalOutput,
            vec![port("main", PortDirection::Input)],
        )
    };
    let session = Session {
        id: EntityId::new("drift-sim"),
        name: "drift sim".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            input("left"),
            input("right"),
            node(
                "mixer",
                NodeKind::Mixer,
                vec![
                    port("main", PortDirection::Input),
                    port("out", PortDirection::Output),
                ],
            ),
            output("first"),
            output("second"),
        ],
        edges: vec![
            edge("left", "main", "mixer"),
            edge("right", "main", "mixer"),
            edge("mixer", "out", "first"),
            edge("mixer", "out", "second"),
        ],
    };
    let graph = audiorouter_engine::compile_mixer_fanout_session(
        &session,
        audiorouter_engine::RuntimeGeneration::new(GENERATION),
    )
    .unwrap();
    audiorouter_engine::RealtimeMixerFanout::new(graph, 4, &[1, 1], 1, QUANTUM).unwrap()
}

struct Sim {
    clock: Clock,
    feeder: WasapiMultiInputFanout,
    captures: Vec<SimCapture>,
    rings: Vec<Arc<audiorouter_engine::AudioBlockRing>>,
    pumps: Vec<RingOutputPump<SimDevice>>,
    /// Random state for irregular pass timing; `None` runs every 1.5 ms.
    late_passes: Option<u32>,
}

impl Sim {
    /// A route whose two inputs and two outputs run this many parts per
    /// million away from the nominal rate.
    fn new(input_ppm: [f64; 2], output_ppm: [f64; 2]) -> Self {
        let clock: Clock = Rc::new(Cell::new(0.0));
        let feeder =
            WasapiMultiInputFanout::new(two_input_two_output_mixer(), &[1, 1], QUANTUM, 4_096)
                .unwrap();
        let captures = input_ppm
            .iter()
            .map(|&ppm| SimCapture::new(&clock, ppm, 16))
            .collect();
        let mut rings = Vec::new();
        let mut pumps = Vec::new();
        for ppm in output_ppm {
            // The 8-quantum ring the control plane gives each output.
            let ring = Arc::new(audiorouter_engine::AudioBlockRing::new(8, 1, QUANTUM).unwrap());
            pumps.push(
                RingOutputPump::new(
                    SimDevice::new(&clock, ppm),
                    Arc::clone(&ring),
                    GENERATION,
                    1,
                    QUANTUM,
                )
                .unwrap(),
            );
            rings.push(ring);
        }
        Self {
            clock,
            feeder,
            captures,
            rings,
            pumps,
            late_passes: None,
        }
    }

    /// One audio thread pass, as `NativeMultiInputWorker` runs it.
    fn pass(&mut self) {
        for capture in &self.captures {
            capture.advance();
        }
        for pump in &self.pumps {
            pump.sink.advance();
        }
        let now = Some((self.clock.get() * 1e7) as u64);
        let captures: Vec<&dyn AudioCaptureSource> = self
            .captures
            .iter()
            .map(|capture| capture as &dyn AudioCaptureSource)
            .collect();
        let destinations: Vec<&audiorouter_engine::AudioBlockRing> =
            self.rings.iter().map(Arc::as_ref).collect();
        for _ in 0..MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE {
            let input = self.feeder.pump_inputs_at(&captures, now).unwrap();
            let delivered = self.feeder.mixer_mut().process_once(&destinations).unwrap();
            let mut rendered = WasapiSchedulerPump::default();
            for pump in &mut self.pumps {
                rendered.accumulate(pump.pump_available(1).unwrap());
            }
            if input.packets == 0
                && input.processed_quanta == 0
                && delivered == 0
                && rendered.packets == 0
                && rendered.rendered_frames == 0
            {
                break;
            }
        }
    }

    /// Time until the next audio thread pass. With `late_passes` set, about
    /// one pass in 50 comes 6 ms late and one in 500 comes 12 ms late, like
    /// the scheduling the owner's log shows on a loaded PC.
    fn next_pass_interval(&mut self) -> f64 {
        let Some(state) = self.late_passes.as_mut() else {
            return PASS_SECONDS;
        };
        *state ^= *state << 13;
        *state ^= *state >> 17;
        *state ^= *state << 5;
        match *state % 500 {
            0 => 0.012,
            1..=10 => 0.006,
            _ => PASS_SECONDS,
        }
    }

    /// Run for `seconds` and return each output's mean queue (frames) over
    /// the last five seconds.
    fn run(&mut self, seconds: f64) -> Vec<f64> {
        let end = self.clock.get() + seconds;
        let mut sums = vec![0.0; self.pumps.len()];
        let mut samples = 0_u32;
        while self.clock.get() < end {
            let interval = self.next_pass_interval();
            self.clock.set(self.clock.get() + interval);
            self.pass();
            if self.clock.get() >= end - 5.0 {
                samples += 1;
                for (sum, pump) in sums.iter_mut().zip(&self.pumps) {
                    *sum += pump.queued_frames() as f64;
                }
            }
        }
        sums.into_iter()
            .map(|sum| sum / f64::from(samples.max(1)))
            .collect()
    }

    fn underruns(&self) -> Vec<u64> {
        self.pumps.iter().map(|pump| pump.underruns).collect()
    }

    fn full_drops(&self) -> Vec<u64> {
        self.rings.iter().map(|ring| ring.full_drops()).collect()
    }

    fn starved_frames(&self) -> Vec<u64> {
        self.pumps
            .iter()
            .map(|pump| pump.sink.starved_frames.get())
            .collect()
    }

    fn output_slips(&self) -> Vec<u64> {
        self.pumps.iter().map(|pump| pump.slipped_frames).collect()
    }

    /// Whether every sample each device received followed the one before
    /// it as two summed tones do: no missing, repeated or misplaced stretch.
    fn assert_continuous_tone(&self) {
        // The steepest step of one tone, doubled for two inputs, with 2%
        // for a block played one frame short.
        let limit = 2.0 * TONE_LEVEL * std::f64::consts::TAU * TONE_HZ / RATE * 1.02;
        for pump in &self.pumps {
            assert_eq!(pump.latency_trims, 0, "blocks discarded as backlog");
            let step = f64::from(pump.sink.largest_step.get());
            assert!(step <= limit, "a step of {step} (limit {limit})");
        }
    }
}

/// The most an output's mean queue should be once it has settled: the
/// default cushion, one device period, the band in which the pump leaves
/// the queue alone, and one block (about 28 ms; 20 ms is typical).
const SETTLED_QUEUE_FRAMES: f64 =
    (RENDER_JITTER_CUSHION_FRAMES + PACKET + drift::SLIP_ENGAGE_FRAMES as usize + QUANTUM) as f64;

fn assert_settled(queues: &[f64]) {
    assert!(
        queues.iter().all(|&queue| queue <= SETTLED_QUEUE_FRAMES),
        "the queue is at {queues:?} frames"
    );
}

#[test]
fn matched_clocks_stay_at_the_cushion_and_pass_audio_untouched() {
    let mut sim = Sim::new([0.0, 0.0], [0.0, 0.0]);
    sim.run(10.0);
    let settled_slips = sim.output_slips();
    let queues = sim.run(50.0);
    assert_eq!(sim.underruns(), [0, 0]);
    assert_eq!(sim.full_drops(), [0, 0]);
    assert_settled(&queues);
    // Once settled, no block is altered: the audio is bit-exact.
    assert_eq!(sim.output_slips(), settled_slips);
    assert_eq!(sim.feeder.input_slipped_frames(), &[0, 0]);
    sim.assert_continuous_tone();
}

#[test]
fn an_output_slower_than_the_path_keeps_its_target_depth_without_dropping_blocks() {
    // Both inputs run 500 ppm fast: the path makes 24 frames a second more
    // than the outputs play. Before the repair the queue grew to the 74 ms
    // capacity of the device buffer and ring, then lost a block at a time.
    let mut sim = Sim::new([500.0, 500.0], [0.0, 0.0]);
    let queues = sim.run(300.0);
    assert_eq!(sim.full_drops(), [0, 0], "blocks dropped at a full ring");
    assert_eq!(sim.underruns(), [0, 0]);
    assert_settled(&queues);
    // 24 frames a second for 300 s, removed one frame at a time.
    assert!(sim
        .output_slips()
        .iter()
        .all(|slips| (6_000..9_000).contains(slips)));
    sim.assert_continuous_tone();
}

#[test]
fn an_output_faster_than_the_path_does_not_underrun() {
    // Before the repair the queue drained and the device ran empty about
    // every 20 seconds.
    let mut sim = Sim::new([-500.0, -500.0], [0.0, 0.0]);
    let queues = sim.run(300.0);
    assert_eq!(sim.underruns(), [0, 0]);
    assert_eq!(sim.starved_frames(), [0, 0]);
    assert_settled(&queues);
    sim.assert_continuous_tone();
}

#[test]
fn a_stalled_input_does_not_leave_its_backlog_as_lasting_delay() {
    for stamped in [true, false] {
        let mut sim = Sim::new([0.0, 0.0], [0.0, 0.0]);
        sim.captures[0].stamped = stamped;
        sim.run(20.0);
        // The input delivers nothing for 80 ms, then all of it at once.
        sim.captures[0].stall.set(Some((20.0, 20.08)));
        let queues = sim.run(20.0);
        assert_eq!(
            sim.full_drops(),
            [0, 0],
            "stamped={stamped}: blocks dropped at a full ring"
        );
        // The gap itself is heard once. It is a stall, so the cushion is
        // not deepened for it.
        assert_eq!(sim.underruns(), [1, 1], "stamped={stamped}");
        assert!(sim
            .pumps
            .iter()
            .all(|pump| pump.target_frames() == RENDER_JITTER_CUSHION_FRAMES as u64));
        assert_settled(&queues);
        // The input that kept running was held back while the other
        // stalled; that backlog is gone too (it stayed for good before).
        assert!(
            sim.feeder.input_depth_frames(1) <= (2 * PACKET) as u64,
            "stamped={stamped}: {} frames",
            sim.feeder.input_depth_frames(1)
        );
        // The feeder reports which input stalled.
        let gaps = sim.feeder.take_input_max_packet_gap_ms();
        assert!(gaps[0] >= 75.0 && gaps[1] < 15.0, "{gaps:?}");
    }
}

#[test]
fn a_short_hiccup_still_deepens_the_cushion() {
    let mut sim = Sim::new([0.0, 0.0], [0.0, 0.0]);
    sim.run(5.0);
    // Both inputs are 30 ms late once: jitter a deeper cushion covers.
    for capture in &sim.captures {
        capture.stall.set(Some((5.0, 5.03)));
    }
    sim.run(5.0);
    assert_eq!(sim.underruns(), [1, 1]);
    let deeper = (RENDER_JITTER_CUSHION_FRAMES + RENDER_JITTER_CUSHION_STEP_FRAMES) as u64;
    assert!(sim.pumps.iter().all(|pump| pump.target_frames() == deeper));
    // Five quiet minutes later the unused step is given back.
    let queues = sim.run(330.0);
    assert_eq!(sim.underruns(), [1, 1]);
    assert!(sim
        .pumps
        .iter()
        .all(|pump| pump.target_frames() == RENDER_JITTER_CUSHION_FRAMES as u64));
    assert_settled(&queues);
}

#[test]
fn an_input_faster_than_the_path_is_kept_current_without_losing_packets() {
    // The second input runs 500 ppm faster than the first, which paces the
    // path. Its device buffer holds two packets, like a shared-mode capture.
    // Before the repair it backed up through 100 ms of internal buffers and
    // then lost packets in the device.
    let mut sim = Sim::new([0.0, 500.0], [0.0, 0.0]);
    sim.captures[1].capacity_packets = 2;
    let mut deepest = 0;
    for _ in 0..300 {
        sim.run(1.0);
        deepest = deepest.max(sim.feeder.input_depth_frames(1));
    }
    assert_eq!(
        sim.captures[1].overflowed_packets.get(),
        0,
        "the capture buffer overflowed"
    );
    // Never more than three packets (30 ms) behind.
    assert!(deepest <= (3 * PACKET) as u64, "{deepest} frames");
    let slips = sim.feeder.input_slipped_frames();
    assert_eq!(slips[0], 0, "the pacing input is never altered");
    assert!((6_000..9_000).contains(&slips[1]), "{slips:?}");
    assert_eq!(sim.underruns(), [0, 0]);
    assert_eq!(sim.full_drops(), [0, 0]);
    sim.assert_continuous_tone();
}

#[test]
fn four_devices_on_four_clocks_run_for_ten_minutes_without_a_gap() {
    // Like the owner's route: every capture and render device has its own
    // clock, a little off in a different direction.
    let mut sim = Sim::new([80.0, -60.0], [120.0, -150.0]);
    for capture in &mut sim.captures {
        capture.capacity_packets = 2;
    }
    let queues = sim.run(600.0);
    assert_eq!(sim.underruns(), [0, 0]);
    assert_eq!(sim.starved_frames(), [0, 0]);
    assert_eq!(sim.full_drops(), [0, 0]);
    assert!(sim
        .captures
        .iter()
        .all(|capture| capture.overflowed_packets.get() == 0));
    assert_settled(&queues);
    sim.assert_continuous_tone();
}

#[test]
fn a_device_with_a_longer_period_gets_a_deeper_target_instead_of_underruns() {
    // An output that takes 20 ms at a time dips twice as far below its
    // mean. The pump measures the dip and holds the mean that much higher.
    let mut sim = Sim::new([300.0, 300.0], [0.0, 0.0]);
    for pump in &mut sim.pumps {
        pump.sink.packet = 2 * PACKET;
    }
    // The first underruns teach the pump the dip; they are heard.
    sim.run(30.0);
    let settled = sim.underruns();
    for pump in &sim.pumps {
        pump.sink.largest_step.set(0.0);
    }
    sim.run(270.0);
    assert_eq!(sim.underruns(), settled, "underruns after settling");
    assert_eq!(sim.full_drops(), [0, 0]);
    sim.assert_continuous_tone();
}

#[test]
fn an_input_that_waits_for_a_larger_packet_is_not_mistaken_for_a_backlog() {
    // The pacing input delivers 30 ms packets, so the other input always
    // has up to 30 ms waiting here. That is not a backlog: it empties with
    // every pacing packet, and removing frames from it would starve it.
    let mut sim = Sim::new([0.0, 0.0], [0.0, 0.0]);
    sim.captures[0].packet = 3 * PACKET;
    sim.run(120.0);
    assert_eq!(sim.feeder.input_slipped_frames(), &[0, 0]);
    assert_eq!(sim.feeder.input_trimmed_packets(), &[0, 0]);
    assert_eq!(sim.full_drops(), [0, 0]);
    sim.assert_continuous_tone();
}

#[test]
fn late_passes_are_absorbed_by_the_cushion_not_mistaken_for_a_backlog() {
    let mut sim = Sim::new([80.0, -60.0], [120.0, -150.0]);
    sim.late_passes = Some(0x2545_f491);
    // A 12 ms late pass can empty the default 10 ms cushion once or twice;
    // the cushion then deepens and holds.
    let mut timeline = Vec::new();
    for _ in 0..10 {
        sim.run(60.0);
        timeline.push(sim.underruns());
    }
    println!("underruns by minute: {timeline:?}");
    let settled = sim.underruns();
    assert!(
        settled.iter().all(|&underruns| underruns <= 4),
        "{settled:?}"
    );
    // Each underrun deepened the cushion one step, and no further.
    for (pump, underruns) in sim.pumps.iter().zip(&settled) {
        assert_eq!(
            pump.target_frames(),
            (RENDER_JITTER_CUSHION_FRAMES + RENDER_JITTER_CUSHION_STEP_FRAMES * *underruns as usize)
                as u64
        );
    }
    // Two packets arriving in one late pass are not a backlog: nothing is
    // discarded on either side.
    assert_eq!(sim.full_drops(), [0, 0]);
    assert_eq!(sim.feeder.input_trimmed_packets(), &[0, 0]);
    assert!(sim.pumps.iter().all(|pump| pump.latency_trims == 0));
    println!(
        "underruns {settled:?}, cushions {:?} frames, output slips {:?}, input slips {:?}",
        sim.pumps
            .iter()
            .map(RingOutputPump::target_frames)
            .collect::<Vec<_>>(),
        sim.output_slips(),
        sim.feeder.input_slipped_frames(),
    );
}
