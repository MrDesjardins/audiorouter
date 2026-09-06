//! Allocation-free audio-block primitives for the M02 realtime boundary.
//!
//! Construction and graph preparation happen off the callback thread. Once an
//! `AudioBlock` exists, the operations below reuse its storage and perform no
//! heap allocation, locking, I/O, or logging.

use std::sync::atomic::{AtomicU64, Ordering};

pub const INTERNAL_SAMPLE_RATE_HZ: u32 = 48_000;
pub const PROCESSING_QUANTUM_FRAMES: usize = 128;
pub const MAX_CHANNELS: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockError {
    InvalidChannels,
    InvalidFrameCount,
    ShapeMismatch,
    InvalidSampleRate,
}

/// A preallocated planar float32 block. Samples are stored channel-major:
/// `channel * frames + frame`.
pub struct AudioBlock {
    channels: usize,
    frames: usize,
    samples: Vec<f32>,
}

/// Bounded per-frame gain transition for de-clicked parameter changes.
/// Construction and target changes occur off the callback thread; applying a
/// ramp only updates existing block samples and this small state object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GainRamp {
    current: f32,
    target: f32,
    step: f32,
    remaining_frames: usize,
}

/// Process-local privacy gate. It silences blocks at a boundary and does not
/// alter Windows privacy permissions or other applications' microphone use.
#[derive(Debug, Default)]
pub struct PrivacyMute {
    muted: std::sync::atomic::AtomicBool,
}

impl PrivacyMute {
    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Release);
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Acquire)
    }

    pub fn apply(&self, block: &mut AudioBlock) {
        if self.is_muted() {
            block.clear();
        }
    }
}

impl GainRamp {
    pub fn new(initial: f32) -> Self {
        let initial = initial.is_finite().then_some(initial).unwrap_or(0.0);
        Self {
            current: initial,
            target: initial,
            step: 0.0,
            remaining_frames: 0,
        }
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    /// Set a finite target and transition over at most `ramp_frames` frames.
    /// A zero-length ramp changes the gain immediately.
    pub fn set_target(&mut self, target: f32, ramp_frames: usize) {
        let target = target.is_finite().then_some(target).unwrap_or(0.0);
        self.target = target;
        if ramp_frames == 0 {
            self.current = target;
            self.step = 0.0;
            self.remaining_frames = 0;
        } else {
            self.step = (target - self.current) / ramp_frames as f32;
            self.remaining_frames = ramp_frames;
        }
    }

    /// Apply the current ramp to every channel of a block without allocating.
    pub fn apply(&mut self, block: &mut AudioBlock) {
        for frame in 0..block.frames {
            if self.remaining_frames > 0 {
                self.current += self.step;
                self.remaining_frames -= 1;
                if self.remaining_frames == 0 {
                    self.current = self.target;
                    self.step = 0.0;
                }
            }
            for channel in 0..block.channels {
                block.channel_mut(channel).unwrap()[frame] *= self.current;
            }
        }
    }
}

impl AudioBlock {
    /// Allocate a block during preparation, before entering the realtime path.
    pub fn new(channels: usize, frames: usize) -> Result<Self, BlockError> {
        if !(1..=MAX_CHANNELS).contains(&channels) {
            return Err(BlockError::InvalidChannels);
        }
        if !(1..=PROCESSING_QUANTUM_FRAMES).contains(&frames) {
            return Err(BlockError::InvalidFrameCount);
        }
        Ok(Self {
            channels,
            frames,
            samples: vec![0.0; channels * frames],
        })
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn channel(&self, channel: usize) -> Option<&[f32]> {
        (channel < self.channels).then(|| {
            let start = channel * self.frames;
            &self.samples[start..start + self.frames]
        })
    }

    pub fn channel_mut(&mut self, channel: usize) -> Option<&mut [f32]> {
        (channel < self.channels).then(|| {
            let start = channel * self.frames;
            &mut self.samples[start..start + self.frames]
        })
    }

    /// Clear the existing storage without allocating.
    pub fn clear(&mut self) {
        self.samples.fill(0.0);
    }

    /// Copy a same-shaped block without allocating.
    pub fn copy_from(&mut self, source: &Self) -> Result<(), BlockError> {
        if self.channels != source.channels || self.frames != source.frames {
            return Err(BlockError::ShapeMismatch);
        }
        self.samples.copy_from_slice(&source.samples);
        Ok(())
    }

    /// Apply a constant gain without allocating. Non-finite gain is treated as
    /// zero so invalid control input cannot inject NaN/Inf into the graph.
    pub fn apply_gain(&mut self, gain: f32) {
        let gain = gain.is_finite().then_some(gain).unwrap_or(0.0);
        for sample in &mut self.samples {
            *sample *= gain;
        }
    }

    /// Add a same-shaped source block into this block without allocating.
    pub fn mix_from(&mut self, source: &Self, gain: f32) -> Result<(), BlockError> {
        if self.channels != source.channels || self.frames != source.frames {
            return Err(BlockError::ShapeMismatch);
        }
        let gain = gain.is_finite().then_some(gain).unwrap_or(0.0);
        for (destination, source) in self.samples.iter_mut().zip(&source.samples) {
            *destination += *source * gain;
        }
        Ok(())
    }

    /// Apply an explicit source-channel-to-destination-channel matrix without
    /// allocating. Matrix order is destination-major: `dst * source_channels
    /// + src`. This keeps mono/stereo conversion visible in the compiled graph.
    pub fn map_from(&mut self, source: &Self, matrix: &[f32]) -> Result<(), BlockError> {
        if self.frames != source.frames
            || matrix.len() != self.channels.saturating_mul(source.channels)
        {
            return Err(BlockError::ShapeMismatch);
        }
        for destination_channel in 0..self.channels {
            let destination = self.channel_mut(destination_channel).unwrap();
            for (frame, sample) in destination.iter_mut().enumerate() {
                let mut value = 0.0;
                for source_channel in 0..source.channels {
                    value += source.channel(source_channel).unwrap()[frame]
                        * matrix[destination_channel * source.channels + source_channel];
                }
                *sample = value;
            }
        }
        Ok(())
    }

    /// Linearly resample a same-channel source into this preallocated block.
    /// This is a bounded format-conversion primitive; clock-drift correction
    /// and cross-block phase management belong to the later stream scheduler.
    pub fn resample_linear_from(
        &mut self,
        source: &Self,
        input_rate_hz: u32,
        output_rate_hz: u32,
    ) -> Result<(), BlockError> {
        if self.channels != source.channels {
            return Err(BlockError::ShapeMismatch);
        }
        if input_rate_hz == 0 || output_rate_hz == 0 {
            return Err(BlockError::InvalidSampleRate);
        }
        let ratio = input_rate_hz as f64 / output_rate_hz as f64;
        for destination_channel in 0..self.channels {
            let destination = self.channel_mut(destination_channel).unwrap();
            let input = source.channel(destination_channel).unwrap();
            for (frame, sample) in destination.iter_mut().enumerate() {
                let position = frame as f64 * ratio;
                let lower = position.floor() as usize;
                let lower = lower.min(source.frames - 1);
                let upper = (lower + 1).min(source.frames - 1);
                let fraction = (position - lower as f64) as f32;
                *sample = input[lower] + (input[upper] - input[lower]) * fraction;
            }
        }
        Ok(())
    }

    /// Replace non-finite samples with silence and return the number repaired.
    pub fn sanitize_non_finite(&mut self) -> usize {
        let mut repaired = 0;
        for sample in &mut self.samples {
            if !sample.is_finite() {
                *sample = 0.0;
                repaired += 1;
            }
        }
        repaired
    }

    pub fn all_finite(&self) -> bool {
        self.samples.iter().all(|sample| sample.is_finite())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeGeneration(u64);

impl RuntimeGeneration {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DriftController {
    nominal_ratio: f64,
    correction_ppm: f64,
    target_frames: f64,
    max_correction_ppm: f64,
}

impl DriftController {
    pub fn new(
        input_rate_hz: u32,
        output_rate_hz: u32,
        target_frames: usize,
        max_correction_ppm: f64,
    ) -> Result<Self, BlockError> {
        if input_rate_hz == 0
            || output_rate_hz == 0
            || target_frames == 0
            || !max_correction_ppm.is_finite()
            || max_correction_ppm < 0.0
        {
            return Err(BlockError::InvalidSampleRate);
        }
        Ok(Self {
            nominal_ratio: input_rate_hz as f64 / output_rate_hz as f64,
            correction_ppm: 0.0,
            target_frames: target_frames as f64,
            max_correction_ppm,
        })
    }

    /// Update correction from bounded FIFO occupancy. The proportional gain
    /// is deliberately conservative; callers still need xrun/discontinuity
    /// policy around the stream scheduler.
    pub fn observe_queue(&mut self, queue_frames: usize) {
        let error = (queue_frames as f64 - self.target_frames) / self.target_frames;
        let requested = error * self.max_correction_ppm;
        self.correction_ppm = requested.clamp(-self.max_correction_ppm, self.max_correction_ppm);
    }

    pub fn correction_ppm(&self) -> f64 {
        self.correction_ppm
    }

    pub fn adjusted_ratio(&self) -> f64 {
        self.nominal_ratio * (1.0 + self.correction_ppm / 1_000_000.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessingStage {
    Gain { linear: f32 },
    Mute { muted: bool },
}

#[derive(Debug, Default)]
pub struct CallbackMetrics {
    processed_quanta: AtomicU64,
    repaired_samples: AtomicU64,
}

impl CallbackMetrics {
    pub fn processed_quanta(&self) -> u64 {
        self.processed_quanta.load(Ordering::Relaxed)
    }

    pub fn repaired_samples(&self) -> u64 {
        self.repaired_samples.load(Ordering::Relaxed)
    }

    fn record(&self, repaired: usize) {
        self.processed_quanta.fetch_add(1, Ordering::Relaxed);
        self.repaired_samples
            .fetch_add(repaired as u64, Ordering::Relaxed);
    }
}

#[derive(Debug, PartialEq)]
pub enum GraphCompileError {
    InvalidGraph(Vec<audiorouter_domain::ValidationError>),
    UnsupportedTopology,
}

/// Prepare the currently supported processing subset of a validated domain
/// graph. Device nodes and edge mixing are intentionally not activated here;
/// they remain owned by the Windows scheduler milestone. A graph containing
/// enabled edges is rejected until buffer routing is implemented. Gain has no
/// scalar field in the v1 domain contract yet, so a non-bypassed gain is unity.
pub fn compile_session(
    session: &audiorouter_domain::Session,
    generation: RuntimeGeneration,
) -> Result<RuntimeGraph, GraphCompileError> {
    use audiorouter_domain::{validate_session, NodeKind};
    use std::collections::{HashMap, VecDeque};

    validate_session(session).map_err(GraphCompileError::InvalidGraph)?;
    if session.edges.iter().any(|edge| edge.enabled) {
        return Err(GraphCompileError::UnsupportedTopology);
    }
    let mut indegree = session
        .nodes
        .iter()
        .map(|node| (node.id.clone(), 0usize))
        .collect::<HashMap<_, _>>();
    let mut outgoing = HashMap::<audiorouter_domain::EntityId, Vec<_>>::new();
    for edge in session.edges.iter().filter(|edge| edge.enabled) {
        *indegree.get_mut(&edge.destination_node).unwrap() += 1;
        outgoing
            .entry(edge.source_node.clone())
            .or_default()
            .push(edge.destination_node.clone());
    }
    let mut ready = session
        .nodes
        .iter()
        .filter(|node| indegree[&node.id] == 0)
        .map(|node| node.id.clone())
        .collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(session.nodes.len());
    while let Some(node_id) = ready.pop_front() {
        order.push(node_id.clone());
        if let Some(children) = outgoing.get(&node_id) {
            for child in children {
                let count = indegree.get_mut(child).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.push_back(child.clone());
                }
            }
        }
    }

    let mut stages = Vec::new();
    for node_id in order {
        let node = session
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .unwrap();
        if !node.enabled || node.bypass {
            continue;
        }
        match node.kind {
            NodeKind::Gain => stages.push(ProcessingStage::Gain { linear: 1.0 }),
            NodeKind::Mute => stages.push(ProcessingStage::Mute { muted: true }),
            NodeKind::PhysicalInput
            | NodeKind::ApplicationCapture
            | NodeKind::EndpointLoopback
            | NodeKind::PhysicalOutput
            | NodeKind::Mixer
            | NodeKind::Meter => {}
        }
    }
    Ok(RuntimeGraph::prepare(generation, stages))
}

/// An immutable, prepared processing schedule. The stage vector is created
/// before realtime execution; `process` only mutates the caller's block.
pub struct RuntimeGraph {
    stages: Vec<ProcessingStage>,
    generation: RuntimeGeneration,
}

/// Publication point for prepared immutable graphs. Preparation and stores
/// happen on the control thread; readers obtain an owned immutable snapshot,
/// and the previous graph is reclaimed only after its last reader releases it.
pub struct RuntimePublication {
    current: arc_swap::ArcSwapOption<RuntimeGraph>,
}

impl Default for RuntimePublication {
    fn default() -> Self {
        Self {
            current: arc_swap::ArcSwapOption::empty(),
        }
    }
}

impl RuntimePublication {
    pub fn new(initial: Option<RuntimeGraph>) -> Self {
        Self {
            current: arc_swap::ArcSwapOption::from(initial.map(std::sync::Arc::new)),
        }
    }

    /// Publish a fully prepared graph. Existing readers continue using their
    /// old generation while new readers observe the replacement.
    pub fn publish(&self, graph: RuntimeGraph) {
        self.current.store(Some(std::sync::Arc::new(graph)));
    }

    /// Load the current graph without taking a mutex. `None` means the runtime
    /// has not been activated yet.
    pub fn load(&self) -> Option<std::sync::Arc<RuntimeGraph>> {
        self.current.load_full()
    }
}

/// Integrated block-processing boundary used by a future Windows scheduler.
/// It provides safe silence before activation, publishes only prepared graphs,
/// applies the process-local privacy gate, and exposes callback counters.
pub struct RuntimeProcessor {
    publication: RuntimePublication,
    privacy_mute: PrivacyMute,
    metrics: CallbackMetrics,
}

impl Default for RuntimeProcessor {
    fn default() -> Self {
        Self {
            publication: RuntimePublication::default(),
            privacy_mute: PrivacyMute::default(),
            metrics: CallbackMetrics::default(),
        }
    }
}

impl RuntimeProcessor {
    pub fn publish(&self, graph: RuntimeGraph) {
        self.publication.publish(graph);
    }

    pub fn set_privacy_muted(&self, muted: bool) {
        self.privacy_mute.set_muted(muted);
    }

    pub fn metrics(&self) -> &CallbackMetrics {
        &self.metrics
    }

    /// Process one block and return the active generation. Before a graph is
    /// published, the block is cleared and `None` is returned.
    pub fn process(&self, block: &mut AudioBlock) -> Option<RuntimeGeneration> {
        let Some(graph) = self.publication.load() else {
            block.clear();
            return None;
        };
        graph.process_instrumented(block, &self.metrics);
        self.privacy_mute.apply(block);
        Some(graph.generation())
    }
}

impl RuntimeGraph {
    pub fn prepare(generation: RuntimeGeneration, stages: Vec<ProcessingStage>) -> Self {
        Self { stages, generation }
    }

    pub fn generation(&self) -> RuntimeGeneration {
        self.generation
    }

    pub fn process(&self, block: &mut AudioBlock) -> usize {
        self.process_inner(block, None)
    }

    /// Process one quantum and update optional atomic callback counters. The
    /// counters never allocate, lock, log, or perform I/O.
    pub fn process_instrumented(&self, block: &mut AudioBlock, metrics: &CallbackMetrics) -> usize {
        self.process_inner(block, Some(metrics))
    }

    fn process_inner(&self, block: &mut AudioBlock, metrics: Option<&CallbackMetrics>) -> usize {
        for stage in &self.stages {
            match *stage {
                ProcessingStage::Gain { linear } => block.apply_gain(linear),
                ProcessingStage::Mute { muted: true } => block.clear(),
                ProcessingStage::Mute { muted: false } => {}
            }
        }
        let repaired = block.sanitize_non_finite();
        if let Some(metrics) = metrics {
            metrics.record(repaired);
        }
        repaired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_reuses_planar_storage_for_gain_and_mix() {
        let mut destination = AudioBlock::new(2, 4).unwrap();
        let mut source = AudioBlock::new(2, 4).unwrap();
        source.channel_mut(0).unwrap().fill(1.0);
        source.channel_mut(1).unwrap().fill(-0.5);
        destination.mix_from(&source, 2.0).unwrap();
        destination.apply_gain(0.5);
        assert_eq!(destination.channel(0).unwrap(), &[1.0; 4]);
        assert_eq!(destination.channel(1).unwrap(), &[-0.5; 4]);
    }

    #[test]
    fn invalid_shapes_and_bounds_are_rejected_before_allocation() {
        assert!(matches!(
            AudioBlock::new(0, 128),
            Err(BlockError::InvalidChannels)
        ));
        assert!(matches!(
            AudioBlock::new(3, 128),
            Err(BlockError::InvalidChannels)
        ));
        assert!(matches!(
            AudioBlock::new(2, 0),
            Err(BlockError::InvalidFrameCount)
        ));
        assert!(matches!(
            AudioBlock::new(2, 129),
            Err(BlockError::InvalidFrameCount)
        ));
        let mut block = AudioBlock::new(1, 4).unwrap();
        let other = AudioBlock::new(2, 4).unwrap();
        assert_eq!(block.copy_from(&other), Err(BlockError::ShapeMismatch));
        assert_eq!(block.mix_from(&other, 1.0), Err(BlockError::ShapeMismatch));
    }

    #[test]
    fn non_finite_samples_are_silenced_and_counted() {
        let mut block = AudioBlock::new(1, 4).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[1.0, f32::NAN, f32::INFINITY, -1.0]);
        assert!(!block.all_finite());
        assert_eq!(block.sanitize_non_finite(), 2);
        assert!(block.all_finite());
        assert_eq!(block.channel(0).unwrap(), &[1.0, 0.0, 0.0, -1.0]);
    }

    #[test]
    fn non_finite_gain_is_safe_silence() {
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        block.apply_gain(f32::NAN);
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
    }

    #[test]
    fn explicit_channel_maps_cover_mono_stereo_conversion() {
        let mut mono = AudioBlock::new(1, 2).unwrap();
        mono.channel_mut(0).unwrap().copy_from_slice(&[0.25, -0.5]);
        let mut stereo = AudioBlock::new(2, 2).unwrap();
        stereo.map_from(&mono, &[1.0, 1.0]).unwrap();
        assert_eq!(stereo.channel(0).unwrap(), &[0.25, -0.5]);
        assert_eq!(stereo.channel(1).unwrap(), &[0.25, -0.5]);

        let mut downmix = AudioBlock::new(1, 2).unwrap();
        downmix.map_from(&stereo, &[0.5, 0.5]).unwrap();
        assert_eq!(downmix.channel(0).unwrap(), &[0.25, -0.5]);
    }

    #[test]
    fn channel_map_rejects_wrong_matrix_shape() {
        let source = AudioBlock::new(2, 4).unwrap();
        let mut destination = AudioBlock::new(1, 4).unwrap();
        assert_eq!(
            destination.map_from(&source, &[1.0]),
            Err(BlockError::ShapeMismatch)
        );
    }

    #[test]
    fn linear_resampler_converts_rates_into_preallocated_output() {
        let mut source = AudioBlock::new(1, 4).unwrap();
        source
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.0, 1.0, 2.0, 3.0]);
        let mut output = AudioBlock::new(1, 2).unwrap();
        output
            .resample_linear_from(&source, 48_000, 24_000)
            .unwrap();
        assert_eq!(output.channel(0).unwrap(), &[0.0, 2.0]);
        assert!(matches!(
            output.resample_linear_from(&source, 0, 48_000),
            Err(BlockError::InvalidSampleRate)
        ));
    }

    #[test]
    fn drift_controller_clamps_fifo_correction() {
        let mut controller = DriftController::new(48_000, 48_000, 128, 100.0).unwrap();
        controller.observe_queue(256);
        assert_eq!(controller.correction_ppm(), 100.0);
        assert!(controller.adjusted_ratio() > 1.0);
        controller.observe_queue(0);
        assert_eq!(controller.correction_ppm(), -100.0);
        assert!(matches!(
            DriftController::new(0, 48_000, 128, 100.0),
            Err(BlockError::InvalidSampleRate)
        ));
    }

    #[test]
    fn prepared_runtime_graph_processes_stages_in_order() {
        let graph = RuntimeGraph::prepare(
            RuntimeGeneration::new(7),
            vec![
                ProcessingStage::Gain { linear: 2.0 },
                ProcessingStage::Mute { muted: false },
                ProcessingStage::Gain { linear: 0.5 },
            ],
        );
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(0.75);
        assert_eq!(graph.process(&mut block), 0);
        assert_eq!(graph.generation().value(), 7);
        assert_eq!(block.channel(0).unwrap(), &[0.75, 0.75]);

        let mute = RuntimeGraph::prepare(
            RuntimeGeneration::new(8),
            vec![ProcessingStage::Mute { muted: true }],
        );
        assert_eq!(mute.process(&mut block), 0);
        assert_eq!(block.channel(0).unwrap(), &[0.0, 0.0]);
    }

    #[test]
    fn compiler_prepares_supported_processing_nodes() {
        use audiorouter_domain::{EntityId, Node, NodeKind, Session};

        let session = Session {
            id: EntityId::new("session"),
            name: "processing-only".into(),
            schema_version: 1,
            revision: 1,
            nodes: vec![Node {
                id: EntityId::new("mute"),
                kind: NodeKind::Mute,
                name: "Mute".into(),
                enabled: true,
                bypass: false,
                ports: vec![],
            }],
            edges: vec![],
        };
        let graph = compile_session(&session, RuntimeGeneration::new(3)).unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        graph.process(&mut block);
        assert_eq!(graph.generation().value(), 3);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);
    }

    #[test]
    fn publication_replaces_generation_without_invalidating_old_reader() {
        let first = RuntimeGraph::prepare(RuntimeGeneration::new(1), vec![]);
        let second = RuntimeGraph::prepare(RuntimeGeneration::new(2), vec![]);
        let publication = RuntimePublication::new(Some(first));
        let old_reader = publication.load().unwrap();
        publication.publish(second);
        assert_eq!(old_reader.generation().value(), 1);
        assert_eq!(publication.load().unwrap().generation().value(), 2);
    }

    #[test]
    fn instrumented_processing_records_only_atomic_counters() {
        let graph = RuntimeGraph::prepare(RuntimeGeneration::new(1), vec![]);
        let metrics = CallbackMetrics::default();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[f32::NAN, 1.0]);
        assert_eq!(graph.process_instrumented(&mut block, &metrics), 1);
        assert_eq!(metrics.processed_quanta(), 1);
        assert_eq!(metrics.repaired_samples(), 1);
    }

    #[test]
    fn gain_ramp_reaches_target_without_a_block_discontinuity() {
        let mut ramp = GainRamp::new(0.0);
        ramp.set_target(1.0, 4);
        let mut block = AudioBlock::new(1, 4).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        ramp.apply(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.25, 0.5, 0.75, 1.0]);
        assert_eq!(ramp.current(), 1.0);

        ramp.set_target(0.0, 0);
        assert_eq!(ramp.current(), 0.0);
    }

    #[test]
    fn privacy_mute_silences_only_the_process_local_block() {
        let mute = PrivacyMute::default();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        mute.set_muted(true);
        mute.apply(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);
        assert!(mute.is_muted());
        mute.set_muted(false);
        block.channel_mut(0).unwrap().fill(1.0);
        mute.apply(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[1.0; 2]);
    }

    #[test]
    fn processor_silences_before_activation_and_applies_published_generation() {
        let processor = RuntimeProcessor::default();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block.channel_mut(0).unwrap().fill(1.0);
        assert_eq!(processor.process(&mut block), None);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);

        processor.publish(RuntimeGraph::prepare(
            RuntimeGeneration::new(9),
            vec![ProcessingStage::Gain { linear: 2.0 }],
        ));
        block.channel_mut(0).unwrap().fill(1.0);
        assert_eq!(
            processor.process(&mut block).map(RuntimeGeneration::value),
            Some(9)
        );
        assert_eq!(block.channel(0).unwrap(), &[2.0; 2]);
        processor.set_privacy_muted(true);
        processor.process(&mut block);
        assert_eq!(block.channel(0).unwrap(), &[0.0; 2]);
        assert_eq!(processor.metrics().processed_quanta(), 2);
    }
}
