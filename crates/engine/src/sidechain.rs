//! Preallocated callback handoff for one main and one key bus.
use crate::{
    AudioBlock, RuntimeBusQuantumIdentity, RuntimeBusScheduler, RuntimeBusSchedulerError,
    RuntimeGeneration,
};
use std::sync::{Arc, Mutex};

const MAX_SIDECHAIN_QUEUE_DEPTH: usize = 8;

struct State {
    silent_key: AudioBlock,
    pending: Option<RuntimeBusQuantumIdentity>,
    last_sequence: u64,
}

/// The graph owns this stage; a separate worker owner services its scheduler.
/// It never performs IPC or waits for the worker. One quantum of pipeline
/// delay is intentional; missing or mismatched results produce silence.
pub struct SidechainStage {
    scheduler: Arc<RuntimeBusScheduler>,
    generation: RuntimeGeneration,
    state: Mutex<State>,
}

impl SidechainStage {
    /// Retire any pending output at graph activation without resetting the
    /// worker's monotonic sequence. Queued older results cannot match a new
    /// pending quantum after this boundary.
    pub fn reset(&self) -> bool {
        let Ok(mut state) = self.state.try_lock() else {
            return false;
        };
        state.pending = None;
        true
    }

    pub fn new(
        scheduler: Arc<RuntimeBusScheduler>,
        generation: RuntimeGeneration,
        frames: usize,
    ) -> Result<Self, RuntimeBusSchedulerError> {
        let inputs = scheduler.input_layout().input_channels();
        let outputs = scheduler.output_layout().output_channels();
        if generation.value() == 0
            || frames == 0
            || frames > crate::PROCESSING_QUANTUM_FRAMES
            || scheduler.input_free.capacity() > MAX_SIDECHAIN_QUEUE_DEPTH
            || scheduler.output_free.capacity() > MAX_SIDECHAIN_QUEUE_DEPTH
            || inputs.len() != 2
            || outputs != [inputs[0]]
        {
            return Err(RuntimeBusSchedulerError::BlockShape);
        }
        let silent_key =
            AudioBlock::new(inputs[1], frames).map_err(|_| RuntimeBusSchedulerError::BlockShape)?;
        Ok(Self {
            scheduler,
            generation,
            state: Mutex::new(State {
                silent_key,
                pending: None,
                last_sequence: 0,
            }),
        })
    }

    /// Caller retains both input blocks. An absent connection is a silent key,
    /// never a copy of main. Only the main block receives worker output.
    pub fn process(
        &self,
        identity: RuntimeBusQuantumIdentity,
        main: &mut AudioBlock,
        key: Option<&AudioBlock>,
    ) {
        let Ok(mut state) = self.state.try_lock() else {
            main.clear();
            return;
        };
        if identity.sequence <= state.last_sequence || main.frames() != state.silent_key.frames() {
            main.clear();
            return;
        }
        let key = key.unwrap_or(&state.silent_key);
        if self
            .scheduler
            .try_submit_inputs(self.generation, identity, &[main, key])
            .is_err()
        {
            state.pending = None;
            main.clear();
            return;
        }
        state.last_sequence = identity.sequence;
        let previous = state.pending.replace(identity);
        if let Some(previous) = previous {
            // Bounded catch-up: a late quantum must not permanently shift the
            // output queue one identity behind the graph. Never publish it.
            for _ in 0..self
                .scheduler
                .output_ready()
                .clamp(1, MAX_SIDECHAIN_QUEUE_DEPTH)
            {
                if matches!(
                    self.scheduler
                        .try_publish_output(self.generation, previous, &mut [main]),
                    Ok(crate::RuntimeBusProcessOutcome::Processed)
                ) {
                    return;
                }
            }
        }
        main.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeBusLayout;

    #[test]
    fn graph_sidechain_instruction_never_falls_back_to_legacy_processing() {
        #[derive(Debug)]
        struct Legacy;
        impl crate::RealtimePluginProcessor for Legacy {
            fn process(&self, main: &mut AudioBlock) {
                main.apply_gain(2.0);
            }
        }
        let graph = crate::RuntimeGraph::prepare(
            RuntimeGeneration::new(1),
            vec![crate::ProcessingStage::SidechainPlugin {
                processor: Arc::new(Legacy),
            }],
        );
        let mut main = AudioBlock::new(2, 4).unwrap();
        main.channel_mut(0).unwrap().fill(0.5);
        let mut key = AudioBlock::new(1, 4).unwrap();
        key.channel_mut(0).unwrap().fill(1.0);
        graph.process_with_sidechain(&mut main, Some(&key));
        assert_eq!(main.channel(0).unwrap(), &[0.0; 4]);
        assert_eq!(key.channel(0).unwrap(), &[1.0; 4]);
    }

    fn stage() -> (SidechainStage, Arc<RuntimeBusScheduler>) {
        let layout = RuntimeBusLayout::new(vec![2, 1], vec![2]).unwrap();
        let scheduler = Arc::new(RuntimeBusScheduler::new(4, layout.clone(), layout, 4).unwrap());
        (
            SidechainStage::new(scheduler.clone(), RuntimeGeneration::new(1), 4).unwrap(),
            scheduler,
        )
    }
    fn id(sequence: u64) -> RuntimeBusQuantumIdentity {
        RuntimeBusQuantumIdentity::new(sequence, 10_000, 4).unwrap()
    }

    #[test]
    fn main_and_key_are_separate_and_unconnected_key_is_silent() {
        let (stage, scheduler) = stage();
        let mut main = AudioBlock::new(2, 4).unwrap();
        main.channel_mut(0).unwrap().fill(0.2);
        stage.process(id(1), &mut main, None);
        assert_eq!(main.channel(0).unwrap(), &[0.0; 4]);
        let input = scheduler.try_take_input().unwrap();
        assert_eq!(input.blocks()[0].channel(0).unwrap(), &[0.2; 4]);
        assert_eq!(input.blocks()[1].channel(0).unwrap(), &[0.0; 4]);
        scheduler.recycle_input(input);
        let mut key = AudioBlock::new(1, 4).unwrap();
        key.channel_mut(0).unwrap().fill(0.9);
        main.channel_mut(0).unwrap().fill(0.3);
        stage.process(id(2), &mut main, Some(&key));
        let input = scheduler.try_take_input().unwrap();
        assert_eq!(input.blocks()[0].channel(0).unwrap(), &[0.3; 4]);
        assert_eq!(input.blocks()[1].channel(0).unwrap(), &[0.9; 4]);
        assert_eq!(key.channel(0).unwrap(), &[0.9; 4]);
        assert_eq!(main.channel(0).unwrap(), &[0.0; 4]);
        scheduler.recycle_input(input);
    }

    #[test]
    fn stale_results_are_silenced_and_the_pipeline_recovers() {
        let (stage, scheduler) = stage();
        let mut main = AudioBlock::new(2, 4).unwrap();
        let mut result = AudioBlock::new(2, 4).unwrap();
        result.channel_mut(0).unwrap().fill(0.5);
        stage.process(id(1), &mut main, None);
        stage.process(id(2), &mut main, None); // result 1 missed its turn
        scheduler
            .try_submit_outputs(RuntimeGeneration::new(1), id(1), &[&result])
            .unwrap();
        scheduler
            .try_submit_outputs(RuntimeGeneration::new(1), id(2), &[&result])
            .unwrap();
        stage.process(id(3), &mut main, None);
        assert_eq!(main.channel(0).unwrap(), &[0.5; 4]);
        stage.process(id(4), &mut main, None);
        assert_eq!(main.channel(0).unwrap(), &[0.0; 4]);
    }

    #[test]
    fn reset_cannot_publish_a_result_from_before_activation() {
        let (stage, scheduler) = stage();
        let mut main = AudioBlock::new(2, 4).unwrap();
        let mut old = AudioBlock::new(2, 4).unwrap();
        old.channel_mut(0).unwrap().fill(0.75);
        stage.process(id(1), &mut main, None);
        scheduler
            .try_submit_outputs(RuntimeGeneration::new(1), id(1), &[&old])
            .unwrap();
        assert!(stage.reset());
        stage.process(id(2), &mut main, None);
        assert_eq!(main.channel(0).unwrap(), &[0.0; 4]);
        stage.process(id(3), &mut main, None);
        assert_eq!(main.channel(0).unwrap(), &[0.0; 4]);
    }

    #[test]
    fn contention_and_wrong_key_width_fail_closed() {
        let (stage, _) = stage();
        let mut main = AudioBlock::new(2, 4).unwrap();
        main.channel_mut(0).unwrap().fill(0.5);
        let held = stage.state.lock().unwrap();
        stage.process(id(1), &mut main, None);
        assert_eq!(main.channel(0).unwrap(), &[0.0; 4]);
        drop(held);
        main.channel_mut(0).unwrap().fill(0.5);
        let key = AudioBlock::new(2, 4).unwrap();
        stage.process(id(2), &mut main, Some(&key));
        assert_eq!(main.channel(0).unwrap(), &[0.0; 4]);
    }
}
