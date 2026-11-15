//! Control-owned bridge for a graph's explicit auxiliary plugin instruction.
use crate::{
    worker_clock_tick, EditorParentAuthorization, ParameterEvent, PluginRuntimeBridgeError,
    PluginStateAsset, SupervisedBusWorkerLoop, SupervisedWorkerProcess, WorkerLoopError,
    WorkerState,
};
use audiorouter_engine::{
    sidechain::SidechainStage, AudioBlock, PluginWorkerHealth, PluginWorkerState,
    RealtimePluginProcessor, RuntimeBusLayout, RuntimeBusQuantumIdentity, RuntimeBusScheduler,
    RuntimeGeneration,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Retain a control-plane owner until the audio graph retires. Dropping this
/// object joins its worker thread and must happen off the audio callback.
pub struct SidechainRuntimeBridge {
    owner: SupervisedBusWorkerLoop,
    stage: SidechainStage,
    sequence: AtomicU64,
    /// One scheduler quantum of pipeline delay, in frames.
    pipeline_frames: u32,
    worker_id: u64,
}

impl std::fmt::Debug for SidechainRuntimeBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SidechainRuntimeBridge")
            .finish_non_exhaustive()
    }
}

impl SidechainRuntimeBridge {
    pub fn spawn(
        worker: SupervisedWorkerProcess,
        generation: RuntimeGeneration,
        frames: usize,
        depth: usize,
    ) -> Result<Arc<Self>, WorkerLoopError> {
        Self::spawn_with_parameters(worker, generation, frames, depth, Vec::new())
    }

    pub fn spawn_with_parameters(
        mut worker: SupervisedWorkerProcess,
        generation: RuntimeGeneration,
        frames: usize,
        depth: usize,
        parameters: Vec<ParameterEvent>,
    ) -> Result<Arc<Self>, WorkerLoopError> {
        if worker.instrument || worker.identity.format != crate::PluginFormat::Vst3 {
            return Err(WorkerLoopError::InvalidLayout);
        }
        let layout = worker
            .bus_layout
            .as_ref()
            .ok_or(WorkerLoopError::InvalidLayout)?;
        let engine_layout = RuntimeBusLayout::new(
            layout
                .input_buses()
                .iter()
                .map(|&width| usize::from(width))
                .collect(),
            layout
                .output_buses()
                .iter()
                .map(|&width| usize::from(width))
                .collect(),
        )
        .map_err(|_| WorkerLoopError::InvalidLayout)?;
        if !(1..=8).contains(&depth) {
            return Err(WorkerLoopError::InvalidLayout);
        }
        let scheduler = Arc::new(
            RuntimeBusScheduler::new(depth, engine_layout.clone(), engine_layout, frames)
                .map_err(|_| WorkerLoopError::InvalidLayout)?,
        );
        let stage = SidechainStage::new(Arc::clone(&scheduler), generation, frames)
            .map_err(|_| WorkerLoopError::InvalidLayout)?;
        let rate = worker.sample_rate_hz;
        let latency = worker
            .report_latency(
                crate::WorkerLatency {
                    samples: 0,
                    sample_rate_hz: rate,
                },
                std::time::Instant::now(),
            )
            .map_err(|_| WorkerLoopError::ThreadStart)?;
        let owner = SupervisedBusWorkerLoop::spawn_with_parameters(
            worker,
            scheduler,
            parameters,
            rate,
            latency.samples,
        )?;
        static NEXT_SIDECHAIN_WORKER_ID: AtomicU64 = AtomicU64::new(1);
        Ok(Arc::new(Self {
            owner,
            stage,
            sequence: AtomicU64::new(0),
            pipeline_frames: u32::try_from(frames).unwrap_or(u32::MAX),
            worker_id: NEXT_SIDECHAIN_WORKER_ID.fetch_add(1, Ordering::Relaxed),
        }))
    }

    /// Replace the bounded parameter template. Control thread only.
    pub fn set_parameters(
        &self,
        parameters: Vec<ParameterEvent>,
    ) -> Result<(), PluginRuntimeBridgeError> {
        self.owner.set_parameters(parameters)
    }

    /// Capture the processing instance's opaque state (control thread).
    pub fn save_state(&self) -> Result<PluginStateAsset, String> {
        self.owner.save_state()
    }

    /// Open the plugin's own editor inside an authorized parent window.
    pub fn open_editor(&self, authorization: EditorParentAuthorization) -> Result<(), String> {
        self.owner.open_editor(authorization)
    }

    /// Close the editor; the worker applies its edits to the processing instance.
    pub fn close_editor(&self) -> Result<(), String> {
        self.owner.close_editor()
    }

    pub fn passes_audio_unchanged(&self) -> bool {
        self.owner.passes_audio_unchanged()
    }

    pub fn set_silence_if_unchanged(&self, silence: bool) {
        self.owner.set_silence_if_unchanged(silence);
    }

    pub fn silenced_for_unchanged_audio(&self) -> bool {
        self.owner.silenced_for_unchanged_audio()
    }

    pub fn resume_after_unchanged(&self) -> bool {
        self.owner.resume_after_unchanged()
    }

    /// The delay the plugin itself reports, without the pipeline quantum.
    pub fn reported_latency_samples(&self) -> u32 {
        self.owner.reported_latency_samples()
    }

    /// Bounded callback counters; output misses include startup warm-up.
    pub fn continuity_counts(&self) -> (u32, u32) {
        self.owner.continuity_counts()
    }

    pub fn failure_reason(&self) -> Option<String> {
        self.owner.failure_reason()
    }

    pub fn health_state(&self) -> WorkerState {
        if self.owner.is_quarantined() {
            WorkerState::Quarantined
        } else if self.owner.has_failed() {
            WorkerState::Failed
        } else {
            WorkerState::Running
        }
    }

    pub fn health_failure_count(&self) -> u32 {
        u32::from(self.owner.has_failed())
    }
}

impl RealtimePluginProcessor for SidechainRuntimeBridge {
    fn reset(&self) -> bool {
        !self.owner.has_failed() && self.stage.reset()
    }

    fn process(&self, main: &mut AudioBlock) {
        main.clear();
    }

    fn process_with_sidechain(&self, main: &mut AudioBlock, key: Option<&AudioBlock>) {
        if self.owner.has_failed() {
            main.clear();
            return;
        }
        let Ok(previous) =
            self.sequence
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                })
        else {
            main.clear();
            return;
        };
        let Ok(identity) = RuntimeBusQuantumIdentity::new(
            previous + 1,
            worker_clock_tick().saturating_add(10_000),
            main.frames(),
        ) else {
            main.clear();
            return;
        };
        self.stage.process(identity, main, key);
    }

    fn health(&self) -> PluginWorkerHealth {
        PluginWorkerHealth {
            state: match self.health_state() {
                WorkerState::Quarantined => PluginWorkerState::Quarantined,
                WorkerState::Failed | WorkerState::Stopped => PluginWorkerState::Failed,
                WorkerState::Running => PluginWorkerState::Running,
            },
            failure_count: self.health_failure_count(),
            worker_id: self.worker_id,
        }
    }

    fn latency_samples(&self) -> u32 {
        self.owner
            .reported_latency_samples()
            .saturating_add(self.pipeline_frames)
    }
}
