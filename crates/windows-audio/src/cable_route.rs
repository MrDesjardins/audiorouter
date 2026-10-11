//! Engine processing between two AudioRouter cables (WP-09, first slice).
//!
//! A cable's render-source lease delivers interleaved float64 blocks of the
//! negotiated bridge size (480 frames by default), while the engine processes
//! fixed [`PROCESSING_QUANTUM_FRAMES`] quanta. [`CableRouteProcessor`]
//! re-blocks between the two with preallocated FIFOs, so the realtime thread
//! that owns it never allocates, locks or waits. Re-blocking delays audio by
//! less than one engine quantum.

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
            // Output residue stays below one block; one push adds at most
            // block + quantum - 1 frames.
            output: vec![0.0; (2 * block_frames + quantum) * channels],
            output_frames: 0,
            quantum: vec![0.0; quantum * channels],
            processed_quanta: 0,
            silent_quanta: 0,
        })
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

    /// Process one bridge block (interleaved, `block_frames` frames or
    /// fewer) and hand every completed output block to `emit`, oldest first.
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
        let mut emitted = 0;
        while self.output_frames * channels - emitted >= block {
            emit(&self.output[emitted..emitted + block]);
            emitted += block;
        }
        self.output
            .copy_within(emitted..self.output_frames * channels, 0);
        self.output_frames -= emitted / channels;
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
            route.push_block(&input, |out| {
                assert_eq!(out.len(), block_frames * channels_usize);
                received.extend_from_slice(out);
            });
        }
        let sent = ramp(0, blocks * block_frames, channels_usize);
        // Output trails input by less than one engine quantum.
        let missing = sent.len() - received.len();
        assert!(missing < (PROCESSING_QUANTUM_FRAMES + block_frames) * channels_usize);
        for (index, (expected, actual)) in sent.iter().zip(&received).enumerate() {
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
}
