//! Engine processing between two AudioRouter cables (WP-09, first slice).
//!
//! A cable's render-source lease delivers interleaved float64 blocks of the
//! negotiated bridge size (480 frames by default), while the engine processes
//! fixed [`PROCESSING_QUANTUM_FRAMES`] quanta. [`CableRouteProcessor`]
//! re-blocks between the two with preallocated FIFOs, so the realtime thread
//! that owns it never allocates, locks or waits.
//!
//! Every full input block yields exactly one output block, at a constant
//! delay of [`reblock_delay_frames`]. Emitting whenever a block happened to
//! be complete instead gave 0, 1, 1, 2 blocks per 480-frame input; the
//! downstream capture sink takes one block per period, so its queue depth,
//! and the route's latency, toggled by a whole block (10 ms jitter in the
//! 2026-10-10 VM run r7).

use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    RealtimeScheduler, RuntimeGraph, MAX_CHANNELS, PROCESSING_QUANTUM_FRAMES,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CableRouteError {
    /// Channel count outside 1..=`MAX_CHANNELS` (the engine's width).
    UnsupportedChannels,
    /// Bridge block size of zero frames.
    InvalidBlockFrames,
    /// A channel matrix whose size is not output × input channels, or with a
    /// non-finite coefficient.
    InvalidMatrix,
}

/// Most channels a cable carries (the driver's `AR_BRIDGE_MAX_CHANNELS`).
pub const MAX_CABLE_ROUTE_CHANNELS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
enum MatrixRow {
    /// Exactly one coefficient, 1.0: the sample is copied (bit-exact,
    /// including −0.0).
    Copy(usize),
    /// All coefficients zero.
    Silent,
    /// Any other mix: the sum of the products.
    Mix,
}

/// Direct multichannel cable route (WP-09 slice 5): maps each interleaved
/// float64 block of `inputs` channels to `outputs` channels through an edge
/// matrix (row-major: one row per output channel, one coefficient per input
/// channel), without the engine. Identity rows copy, so 8-channel
/// pass-through is bit-exact (VCAB-20); other rows up- or downmix. Prepared
/// once; `process` never allocates.
#[derive(Clone, Debug)]
pub struct CableMatrixRoute {
    inputs: usize,
    outputs: usize,
    frames: usize,
    matrix: Vec<f64>,
    rows: Vec<MatrixRow>,
}

impl CableMatrixRoute {
    pub fn new(
        inputs: usize,
        outputs: usize,
        frames: usize,
        matrix: &[f32],
    ) -> Result<Self, CableRouteError> {
        if !(1..=MAX_CABLE_ROUTE_CHANNELS).contains(&inputs)
            || !(1..=MAX_CABLE_ROUTE_CHANNELS).contains(&outputs)
        {
            return Err(CableRouteError::UnsupportedChannels);
        }
        if frames == 0 {
            return Err(CableRouteError::InvalidBlockFrames);
        }
        if matrix.len() != inputs * outputs || matrix.iter().any(|value| !value.is_finite()) {
            return Err(CableRouteError::InvalidMatrix);
        }
        let rows = matrix
            .chunks_exact(inputs)
            .map(|row| {
                let mut nonzero = (0..inputs).filter(|&input| row[input] != 0.0);
                match (nonzero.next(), nonzero.next()) {
                    (None, _) => MatrixRow::Silent,
                    (Some(input), None) if row[input] == 1.0 => MatrixRow::Copy(input),
                    _ => MatrixRow::Mix,
                }
            })
            .collect();
        Ok(Self {
            inputs,
            outputs,
            frames,
            matrix: matrix.iter().map(|&value| f64::from(value)).collect(),
            rows,
        })
    }

    pub fn input_samples(&self) -> usize {
        self.frames * self.inputs
    }

    pub fn output_samples(&self) -> usize {
        self.frames * self.outputs
    }

    /// Map one block. A shorter input (a final partial block) maps only its
    /// whole frames; the rest of `output` becomes silence.
    pub fn process(&self, input: &[f64], output: &mut [f64]) {
        let frames = (input.len() / self.inputs)
            .min(output.len() / self.outputs)
            .min(self.frames);
        for frame in 0..frames {
            let source = &input[frame * self.inputs..(frame + 1) * self.inputs];
            let destination = &mut output[frame * self.outputs..(frame + 1) * self.outputs];
            for (channel, (sample, row)) in destination.iter_mut().zip(&self.rows).enumerate() {
                *sample = match *row {
                    MatrixRow::Copy(input) => source[input],
                    MatrixRow::Silent => 0.0,
                    MatrixRow::Mix => {
                        let coefficients =
                            &self.matrix[channel * self.inputs..(channel + 1) * self.inputs];
                        source
                            .iter()
                            .zip(coefficients)
                            .fold(0.0, |sum, (value, coefficient)| sum + value * coefficient)
                    }
                };
            }
        }
        output[frames * self.outputs..].fill(0.0);
    }
}

/// Session for a plain cable route: `render_bus`'s render source feeds
/// `capture_bus`'s capture sink through an identity matrix, no tools.
pub fn cable_route_session(render_bus: &str, capture_bus: &str, channels: u8) -> Session {
    let identity = (0..usize::from(channels))
        .flat_map(|row| {
            (0..usize::from(channels)).map(move |column| f32::from(u8::from(row == column)))
        })
        .collect();
    let node = |id: &str, kind, bus: &str, port: Port| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::Map::from_iter([("busId".to_owned(), bus.into())]),
        ports: vec![port],
    };
    Session {
        id: EntityId::new("cable-route"),
        name: "Cable route".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node(
                "source",
                NodeKind::VirtualRenderSource,
                render_bus,
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels,
                },
            ),
            node(
                "sink",
                NodeKind::VirtualCaptureSink,
                capture_bus,
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels,
                },
            ),
        ],
        edges: vec![Edge {
            id: EntityId::new("source-sink"),
            source_node: EntityId::new("source"),
            source_port: "out".into(),
            destination_node: EntityId::new("sink"),
            destination_port: "in".into(),
            matrix: identity,
            enabled: true,
        }],
    }
}

/// Silence the output starts with so that an output block is ready after
/// every full input block of `block_frames`: the largest input residue
/// re-blocking can leave, `quantum - gcd(block, quantum)`, or zero when the
/// block is a multiple of the quantum.
pub fn reblock_delay_frames(block_frames: usize) -> usize {
    let quantum = PROCESSING_QUANTUM_FRAMES;
    if block_frames == 0 || block_frames % quantum == 0 {
        return 0;
    }
    let (mut a, mut b) = (block_frames, quantum);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    quantum - a
}

/// Runs a published engine graph on bridge blocks, re-blocked to engine
/// quanta. Owned by exactly one realtime thread.
pub struct CableRouteProcessor {
    scheduler: RealtimeScheduler,
    channels: usize,
    block_frames: usize,
    input: Vec<f32>,
    input_frames: usize,
    output: Vec<f64>,
    output_frames: usize,
    quantum: Vec<f32>,
    processed_quanta: u64,
    silent_quanta: u64,
}

impl CableRouteProcessor {
    /// Allocates every buffer and publishes `graph`; call before the
    /// realtime thread starts.
    pub fn new(
        channels: usize,
        block_frames: usize,
        graph: RuntimeGraph,
    ) -> Result<Self, CableRouteError> {
        if !(1..=MAX_CHANNELS).contains(&channels) {
            return Err(CableRouteError::UnsupportedChannels);
        }
        if block_frames == 0 {
            return Err(CableRouteError::InvalidBlockFrames);
        }
        let scheduler = RealtimeScheduler::new(2, channels, PROCESSING_QUANTUM_FRAMES)
            .map_err(|_| CableRouteError::UnsupportedChannels)?;
        scheduler.publish(graph);
        let quantum = PROCESSING_QUANTUM_FRAMES;
        Ok(Self {
            scheduler,
            channels,
            block_frames,
            // Input residue stays below one quantum before a block arrives.
            input: vec![0.0; (block_frames + quantum) * channels],
            input_frames: 0,
            // Before emission the output holds at most the delay plus one
            // block (below block + quantum frames).
            output: vec![0.0; (2 * block_frames + quantum) * channels],
            output_frames: reblock_delay_frames(block_frames),
            quantum: vec![0.0; quantum * channels],
            processed_quanta: 0,
            silent_quanta: 0,
        })
    }

    /// Constant route delay added by re-blocking, in frames.
    pub fn delay_frames(&self) -> usize {
        reblock_delay_frames(self.block_frames)
    }

    /// Engine quanta processed so far.
    pub fn processed_quanta(&self) -> u64 {
        self.processed_quanta
    }

    /// Quanta replaced by silence because the engine produced no output
    /// (no published graph, or a scheduler fault). Fails closed.
    pub fn silent_quanta(&self) -> u64 {
        self.silent_quanta
    }

    /// Process one bridge block (interleaved, `block_frames` frames) and
    /// hand the completed output block to `emit`: exactly one for every
    /// full block. A shorter block (not produced by the bridge) is
    /// processed too, but then shifts the emission phase.
    pub fn push_block(&mut self, samples: &[f64], mut emit: impl FnMut(&[f64])) {
        let channels = self.channels;
        let frames = (samples.len() / channels).min(self.block_frames);
        let start = self.input_frames * channels;
        for (target, source) in self.input[start..start + frames * channels]
            .iter_mut()
            .zip(samples)
        {
            // Bridge samples come from float32 endpoints: this is exact.
            *target = *source as f32;
        }
        self.input_frames += frames;
        let quantum = PROCESSING_QUANTUM_FRAMES;
        let mut consumed = 0;
        while self.input_frames - consumed >= quantum {
            let range = consumed * channels..(consumed + quantum) * channels;
            self.process_quantum(range);
            consumed += quantum;
        }
        self.input
            .copy_within(consumed * channels..self.input_frames * channels, 0);
        self.input_frames -= consumed;

        let block = self.block_frames * channels;
        // At most one block per push: with blocks shorter than a quantum one
        // quantum completes several blocks at once; the rest wait for the
        // following pushes. The delay guarantees one is always ready.
        if self.output_frames * channels >= block {
            emit(&self.output[..block]);
            self.output
                .copy_within(block..self.output_frames * channels, 0);
            self.output_frames -= self.block_frames;
        }
    }

    fn process_quantum(&mut self, range: std::ops::Range<usize>) {
        let produced = self.run_engine(range);
        if produced {
            self.processed_quanta += 1;
        } else {
            self.quantum.fill(0.0);
            self.silent_quanta += 1;
        }
        let start = self.output_frames * self.channels;
        for (target, source) in self.output[start..start + self.quantum.len()]
            .iter_mut()
            .zip(&self.quantum)
        {
            *target = f64::from(*source);
        }
        self.output_frames += PROCESSING_QUANTUM_FRAMES;
    }

    /// One engine step into `self.quantum`; false when no output exists.
    fn run_engine(&mut self, range: std::ops::Range<usize>) -> bool {
        let Some(mut block) = self.scheduler.acquire_input() else {
            return false;
        };
        if block.copy_from_interleaved(&self.input[range]).is_err() {
            let _ = self.scheduler.input().try_recycle(block);
            return false;
        }
        if let Err(block) = self.scheduler.submit_input(block) {
            let _ = self.scheduler.input().try_recycle(block);
            return false;
        }
        let Ok(Some(generation)) = self.scheduler.process_once() else {
            // Nothing published: the scheduler dropped the input; drain any
            // stale output so the pool cannot run dry.
            while let Some(stale) = self.scheduler.receive_output() {
                let _ = self.scheduler.output().try_recycle(stale);
            }
            return false;
        };
        let Some(output) = self.scheduler.receive_output_for_generation(generation) else {
            return false;
        };
        let copied = output.copy_to_interleaved(&mut self.quantum).is_ok();
        let _ = self.scheduler.output().try_recycle(output);
        copied
    }
}

/// Default [`StandingDelayTrim`] window: 100 blocks, about 1 s at the
/// 480-frame default bridge block.
pub const STANDING_DELAY_WINDOW_BLOCKS: u32 = 100;

/// Latency recovery for a block FIFO whose producer and consumer run at the
/// same nominal block rate (spec 17, "Latency recovery").
///
/// When the consumer finds the FIFO empty it sends silence, and the block that
/// arrives late then stays queued for the rest of the stream: a standing
/// extra delay of one block (2026-10-11 traced VM run, one +10 ms step per
/// silence insert). The consumer calls [`Self::after_take`] with the FIFO
/// depth left after each take. When that depth stayed at least one for a
/// whole window, the oldest queued block has waited a full block period at
/// every take, so dropping it returns the FIFO to its normal delay. A burst
/// or jitter that empties the FIFO once restarts the window, so on-time
/// audio is never dropped and at most one block is dropped per window.
/// Plain integer state: no allocation, lock or wait.
#[derive(Clone, Debug)]
pub struct StandingDelayTrim {
    window: u32,
    held: u32,
    trimmed: u64,
}

impl StandingDelayTrim {
    /// `window_blocks` of zero is treated as one.
    pub const fn new(window_blocks: u32) -> Self {
        Self {
            window: if window_blocks == 0 { 1 } else { window_blocks },
            held: 0,
            trimmed: 0,
        }
    }

    /// Record one consumer take (a block or silence) that left `depth`
    /// blocks queued. Returns true when the caller must drop exactly one
    /// queued block now, as a counted latency-recovery drop.
    pub fn after_take(&mut self, depth: usize) -> bool {
        if depth == 0 {
            self.held = 0;
            return false;
        }
        self.held += 1;
        if self.held < self.window {
            return false;
        }
        self.held = 0;
        self.trimmed += 1;
        true
    }

    /// Blocks dropped so far.
    pub fn trimmed(&self) -> u64 {
        self.trimmed
    }
}

/// When to publish silence instead of waiting for the next block.
///
/// A cable route's two sides tick at the same block rate from one driver
/// clock, with a fixed but arbitrary phase between them. When the reader's
/// acknowledgement lands just before the next block arrives, taking the
/// relay at once publishes silence. The late block then stands queued, so
/// latency recovery drops one, which moves the route back to the edge: a
/// silence-and-drop cycle. The 2026-10-11 VM tone run showed 86 silences
/// and 83 trims in 10 minutes (about 250 glitches) at an unlucky phase, and
/// 4 at a lucky one. Waiting up to `deadline` after the acknowledgement for
/// a block absorbs that jitter. The reader then still gets a block well
/// before it needs the next one (deadline is half a block period).
/// Integer state only, times in microseconds: no allocation, lock or wait.
#[derive(Clone, Debug)]
pub struct SilenceHoldoff {
    deadline_us: u64,
    waiting_since_us: Option<u64>,
}

impl SilenceHoldoff {
    pub const fn new(deadline_us: u64) -> Self {
        Self {
            deadline_us,
            waiting_since_us: None,
        }
    }

    /// The reader has acknowledged and wants a block now. Returns true when
    /// the caller must publish now: a block is ready, or none came within
    /// the deadline (publish counted silence).
    pub fn publish_now(&mut self, now_us: u64, block_ready: bool) -> bool {
        let since = *self.waiting_since_us.get_or_insert(now_us);
        if block_ready || now_us.saturating_sub(since) >= self.deadline_us {
            self.waiting_since_us = None;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_engine::{compile_session_at_sample_rate, RuntimeGeneration};

    fn processor(channels: u8, block_frames: usize) -> CableRouteProcessor {
        let graph = compile_session_at_sample_rate(
            &cable_route_session("cable-a", "cable-b", channels),
            RuntimeGeneration::new(1),
            48_000,
        )
        .expect("cable route compiles");
        CableRouteProcessor::new(usize::from(channels), block_frames, graph).unwrap()
    }

    /// Distinct, exactly representable float32 values: frame index coded.
    fn ramp(first_frame: usize, frames: usize, channels: usize) -> Vec<f64> {
        (0..frames * channels)
            .map(|index| {
                let frame = first_frame + index / channels;
                let channel = index % channels;
                let value = ((frame * channels + channel) % (1 << 22)) as f64 / (1u32 << 23) as f64;
                if channel % 2 == 0 {
                    value
                } else {
                    -value
                }
            })
            .collect()
    }

    fn stream(block_frames: usize, channels: u8, blocks: usize) {
        let channels_usize = usize::from(channels);
        let mut route = processor(channels, block_frames);
        let mut received = Vec::new();
        for block in 0..blocks {
            let input = ramp(block * block_frames, block_frames, channels_usize);
            let mut emitted = 0;
            route.push_block(&input, |out| {
                assert_eq!(out.len(), block_frames * channels_usize);
                received.extend_from_slice(out);
                emitted += 1;
            });
            // Lockstep: a varying count would vary the downstream latency.
            assert_eq!(emitted, 1, "block {block} of {block_frames} frames");
        }
        // Constant delay: the leading silence, then every sample in order.
        let delay = route.delay_frames() * channels_usize;
        assert!(received[..delay].iter().all(|sample| *sample == 0.0));
        let received = &received[delay..];
        let sent = ramp(0, blocks * block_frames, channels_usize);
        assert_eq!(received.len() + delay, sent.len());
        for (index, (expected, actual)) in sent.iter().zip(received).enumerate() {
            // -0.0 arrives as +0.0 by engine design; ramp values are exact.
            assert_eq!(
                (*expected as f32).to_bits(),
                (*actual as f32).to_bits(),
                "sample {index}: {expected:e} became {actual:e}"
            );
            assert_eq!(*expected, *actual);
        }
        assert_eq!(route.silent_quanta(), 0);
        assert_eq!(
            route.processed_quanta() as usize,
            blocks * block_frames / PROCESSING_QUANTUM_FRAMES
        );
    }

    #[test]
    fn reblock_delay_is_the_largest_input_residue() {
        for (block, delay) in [
            (480, 96),
            (128, 0),
            (256, 0),
            (4_096, 0),
            (144, 112),
            (16, 112),
            (441, 127),
            (0, 0),
        ] {
            assert_eq!(reblock_delay_frames(block), delay, "{block}-frame blocks");
        }
    }

    #[test]
    fn default_bridge_blocks_reblock_without_loss_or_change() {
        // 480 is not a multiple of 128: exercises every residue phase.
        stream(480, 2, 200);
        stream(480, 1, 200);
    }

    #[test]
    fn low_latency_and_large_blocks_reblock_exactly() {
        stream(128, 2, 100);
        stream(144, 2, 100);
        stream(4_096, 2, 8);
        stream(16, 2, 400);
    }

    #[test]
    fn missing_graph_fails_closed_to_counted_silence() {
        let mut route = processor(2, 480);
        route.scheduler.deactivate();
        let mut blocks = 0;
        for block in 0..10 {
            route.push_block(&ramp(block * 480, 480, 2), |out| {
                assert!(out.iter().all(|sample| *sample == 0.0));
                blocks += 1;
            });
        }
        assert!(blocks >= 9);
        assert_eq!(route.processed_quanta(), 0);
        assert!(route.silent_quanta() >= 37);
    }

    #[test]
    fn rejects_shapes_the_engine_cannot_process() {
        let graph = || {
            compile_session_at_sample_rate(
                &cable_route_session("cable-a", "cable-b", 2),
                RuntimeGeneration::new(1),
                48_000,
            )
            .unwrap()
        };
        assert_eq!(
            CableRouteProcessor::new(8, 480, graph()).err(),
            Some(CableRouteError::UnsupportedChannels)
        );
        assert_eq!(
            CableRouteProcessor::new(2, 0, graph()).err(),
            Some(CableRouteError::InvalidBlockFrames)
        );
    }

    /// FIFO model at 10 ticks per block: block `n` arrives at `arrivals[n]`;
    /// the consumer takes once per block period at `10 * k + 5`. Returns, per
    /// take, how long the delivered block waited since it arrived (`None`
    /// for silence), and the number of blocks the trim dropped.
    fn run_fifo(arrivals: &[u64], takes: u64, window: u32) -> (Vec<Option<u64>>, u64) {
        let mut queue = std::collections::VecDeque::new();
        let mut next = 0;
        let mut trim = StandingDelayTrim::new(window);
        let mut delivered = Vec::new();
        for take in 0..takes {
            let now = 10 * take + 5;
            while next < arrivals.len() && arrivals[next] <= now {
                queue.push_back(arrivals[next]);
                next += 1;
            }
            delivered.push(queue.pop_front().map(|arrival| now - arrival));
            if trim.after_take(queue.len()) {
                queue.pop_front().expect("trim drops only a queued block");
            }
        }
        (delivered, trim.trimmed())
    }

    fn on_time(blocks: u64) -> Vec<u64> {
        (0..blocks).map(|n| 10 * n + 1).collect()
    }

    #[test]
    fn standing_delay_trim_leaves_on_time_audio_untouched() {
        let (delivered, trimmed) = run_fifo(&on_time(1_000), 1_000, 100);
        assert_eq!(trimmed, 0);
        assert!(delivered.iter().all(|wait| *wait == Some(4)));
    }

    #[test]
    fn standing_delay_trim_sheds_the_block_a_silence_insert_left_behind() {
        // Block 50 arrives just after its take: one silence, then every later
        // block waits one extra period (the traced r20 starts 6 and 10).
        let mut arrivals = on_time(1_000);
        arrivals[50] = 10 * 50 + 6;
        let (without, _) = run_fifo(&arrivals, 1_000, u32::MAX);
        assert_eq!(without[50], None);
        assert!(without[52..].iter().all(|wait| *wait == Some(14)));
        let (delivered, trimmed) = run_fifo(&arrivals, 1_000, 100);
        assert_eq!(trimmed, 1);
        assert_eq!(delivered.iter().filter(|wait| wait.is_none()).count(), 1);
        // Normal wait again after one window; nothing else is lost.
        assert!(delivered[160..].iter().all(|wait| *wait == Some(4)));
        let blocks = delivered.iter().flatten().count() as u64;
        assert_eq!(blocks + trimmed, 1_000);
    }

    #[test]
    fn standing_delay_trim_sheds_one_block_per_silence_after_a_late_burst() {
        // Blocks 200-202 arrive together after two takes found nothing: two
        // silences, so two blocks stand until trimmed.
        let mut burst = on_time(1_000);
        for (n, arrival) in burst.iter_mut().enumerate().skip(200).take(3) {
            *arrival = 10 * 202 + 4 + (n as u64 - 200);
        }
        let (delivered, trimmed) = run_fifo(&burst, 1_000, 100);
        assert_eq!(delivered.iter().filter(|wait| wait.is_none()).count(), 2);
        assert_eq!(trimmed, 2);
        assert!(delivered[450..].iter().all(|wait| *wait == Some(4)));
    }

    #[test]
    fn standing_delay_trim_ignores_early_blocks_and_jitter() {
        // One block arriving a period early queues for one take only.
        let mut early = on_time(1_000);
        early[301] = 10 * 300 + 2;
        let (delivered, trimmed) = run_fifo(&early, 1_000, 100);
        assert_eq!(trimmed, 0);
        assert!(delivered.iter().all(Option::is_some));
        // Jitter inside the period: never a standing block.
        let jitter: Vec<u64> = (0..1_000)
            .map(|n| 10 * n + if n % 2 == 0 { 1 } else { 4 })
            .collect();
        let (delivered, trimmed) = run_fifo(&jitter, 1_000, 100);
        assert_eq!(trimmed, 0);
        assert!(delivered.iter().all(Option::is_some));
    }

    #[test]
    fn standing_delay_trim_drops_at_most_one_block_per_window() {
        let mut trim = StandingDelayTrim::new(100);
        let drops: Vec<u32> = (0..1_000).filter(|_| trim.after_take(3)).collect();
        assert_eq!(drops, (0..10).map(|k| 99 + 100 * k).collect::<Vec<_>>());
        assert_eq!(trim.trimmed(), 10);
        // A producer three blocks ahead is brought back one block per window.
        let ahead: Vec<u64> = (0..1_000u64)
            .map(|n| (10 * n + 1).saturating_sub(30))
            .collect();
        let (delivered, trimmed) = run_fifo(&ahead, 990, 100);
        assert_eq!(trimmed, 3);
        assert!(delivered[400..].iter().all(|wait| *wait == Some(4)));
        assert_eq!(StandingDelayTrim::new(0).window, 1);
    }

    /// Route timing model in microseconds: blocks arrive every `P` at
    /// `phase` plus jitter, and every `spike_every`-th block `spike_us`
    /// late (a VM hiccup); the reader acknowledges every `P` at 0; the
    /// capture side polls every 1 ms. Returns (silences, trims) after the
    /// first 10 periods, over 60 s.
    fn route_model(
        phase_us: u64,
        jitter_us: u64,
        spike_every: u64,
        spike_us: u64,
        holdoff_us: u64,
    ) -> (u64, u64) {
        const P: u64 = 10_000;
        const PERIODS: u64 = 6_000;
        let mut seed = 0x2545_F491_4F6C_DD1Du64 ^ phase_us;
        let mut arrivals = std::collections::VecDeque::new();
        for n in 0..PERIODS + 2 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let jitter = seed % (2 * jitter_us + 1);
            let spike = if spike_every != 0 && n % spike_every == spike_every - 1 {
                spike_us
            } else {
                0
            };
            arrivals.push_back(P * (n + 1) + phase_us + jitter + spike - jitter_us);
        }
        let mut queue = 0_u64;
        let mut holdoff = SilenceHoldoff::new(holdoff_us);
        let mut trim = StandingDelayTrim::new(STANDING_DELAY_WINDOW_BLOCKS);
        let (mut acknowledged, mut published) = (0_u64, 0_u64);
        let (mut silences, mut trims) = (0, 0);
        let mut now = 300;
        while now < P * PERIODS {
            while arrivals.front().is_some_and(|arrival| *arrival <= now) {
                arrivals.pop_front();
                queue += 1;
            }
            acknowledged = acknowledged.max(now / P);
            if acknowledged > published && holdoff.publish_now(now, queue > 0) {
                published += 1;
                let counted = now > 10 * P;
                if queue > 0 {
                    queue -= 1;
                } else if counted {
                    silences += 1;
                }
                if trim.after_take(queue as usize) {
                    queue -= 1;
                    trims += u64::from(counted);
                }
            }
            now += 1_000;
        }
        (silences, trims)
    }

    #[test]
    fn silence_holdoff_ends_the_silence_and_drop_cycle_at_every_phase() {
        // 0.5 ms jitter and a 3 ms late block every 700 blocks (7 s), the
        // r25 VM rate. Without the hold-off, a phase that leaves the block
        // arriving just before the reader acknowledges turns every spike into
        // a silence and, a window later, a trim. With it, nothing.
        let mut worst_without = (0, 0);
        for phase in (0..10_000).step_by(250) {
            let (silences, trims) = route_model(phase, 500, 700, 3_000, 5_000);
            assert!(
                silences <= 1 && trims <= 1,
                "phase {phase} us: {silences} silences, {trims} trims"
            );
            let without = route_model(phase, 500, 700, 3_000, 0);
            if without.0 + without.1 > worst_without.0 + worst_without.1 {
                worst_without = without;
            }
        }
        assert!(
            worst_without.0 >= 8 && worst_without.1 >= 6,
            "{worst_without:?}"
        );
        // Plain jitter of up to 2 ms is absorbed at every phase as well.
        for phase in (0..10_000).step_by(250) {
            let (silences, trims) = route_model(phase, 2_000, 0, 0, 5_000);
            assert!(silences <= 1 && trims <= 1, "phase {phase} us");
        }
        assert_eq!(route_model(3_000, 0, 0, 0, 5_000), (0, 0));
        let mut holdoff = SilenceHoldoff::new(5_000);
        assert!(
            holdoff.publish_now(100, true),
            "a ready block is published at once"
        );
        assert!(!holdoff.publish_now(200, false) && !holdoff.publish_now(5_100, false));
        assert!(
            holdoff.publish_now(5_200, false),
            "silence only after the deadline"
        );
        assert!(
            !holdoff.publish_now(5_300, false),
            "the next wait starts afresh"
        );
    }

    #[test]
    fn matrix_route_passes_eight_channels_bit_exact() {
        let identity: Vec<f32> = (0..64).map(|i| f32::from(u8::from(i % 9 == 0))).collect();
        let route = CableMatrixRoute::new(8, 8, 480, &identity).unwrap();
        // float32-grid noise plus signed zeros and the extremes.
        let mut input: Vec<f64> = (0..480_u64 * 8)
            .map(|i| {
                let grid = (i.wrapping_mul(2_654_435_761) % (1 << 24)) as f64;
                (grid - f64::from(1 << 23)) / f64::from(1 << 24)
            })
            .collect();
        input[..4].copy_from_slice(&[-0.0, 0.0, 1.0, -1.0]);
        let mut output = vec![9.0; 480 * 8];
        route.process(&input, &mut output);
        assert!(input
            .iter()
            .zip(&output)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        assert_eq!(
            (route.input_samples(), route.output_samples()),
            (3_840, 3_840)
        );
    }

    #[test]
    fn matrix_route_down_and_up_mixes_and_swaps() {
        // 7.1 (L R C LFE Ls Rs Lb Rb) down to stereo: each side gets its
        // front, side and back channel plus half the centre.
        let mut down = vec![0.0_f32; 16];
        for (output, inputs) in [(0, [0, 4, 6]), (1, [1, 5, 7])] {
            for input in inputs {
                down[output * 8 + input] = 1.0;
            }
            down[output * 8 + 2] = 0.5;
        }
        let route = CableMatrixRoute::new(8, 2, 1, &down).unwrap();
        let mut output = [0.0; 2];
        route.process(&[0.1, 0.2, 0.4, 0.9, 0.01, 0.02, 0.001, 0.002], &mut output);
        assert!((output[0] - (0.1 + 0.2 + 0.01 + 0.001)).abs() < 1e-12);
        assert!((output[1] - (0.2 + 0.2 + 0.02 + 0.002)).abs() < 1e-12);
        // Stereo up to 8 (front pair only) and a left/right swap.
        let mut up = vec![0.0_f32; 16];
        up[0] = 1.0;
        up[3] = 1.0;
        let route = CableMatrixRoute::new(2, 8, 1, &up).unwrap();
        let mut wide = [7.0; 8];
        route.process(&[0.25, -0.5], &mut wide);
        assert_eq!(wide, [0.25, -0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let route = CableMatrixRoute::new(2, 2, 1, &[0.0, 1.0, 1.0, 0.0]).unwrap();
        let mut swapped = [0.0; 2];
        route.process(&[0.25, -0.5], &mut swapped);
        assert_eq!(swapped, [-0.5, 0.25]);
        // A short final block leaves the rest silent; bad shapes are refused.
        let route = CableMatrixRoute::new(2, 2, 4, &[1.0, 0.0, 0.0, 1.0]).unwrap();
        let mut partial = [5.0; 8];
        route.process(&[0.5, 0.5], &mut partial);
        assert_eq!(partial, [0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            CableMatrixRoute::new(9, 2, 1, &[0.0; 18]).err(),
            Some(CableRouteError::UnsupportedChannels)
        );
        assert_eq!(
            CableMatrixRoute::new(2, 2, 1, &[1.0; 3]).err(),
            Some(CableRouteError::InvalidMatrix)
        );
        assert_eq!(
            CableMatrixRoute::new(1, 1, 1, &[f32::NAN]).err(),
            Some(CableRouteError::InvalidMatrix)
        );
        assert_eq!(
            CableMatrixRoute::new(1, 1, 0, &[1.0]).err(),
            Some(CableRouteError::InvalidBlockFrames)
        );
    }
}
