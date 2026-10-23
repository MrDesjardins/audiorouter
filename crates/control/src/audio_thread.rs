//! The dedicated audio thread that pumps the multi-input worker.
//!
//! Before this thread existed, the control thread both dispatched requests and
//! pumped every native worker, so a slow request stopped all audio for its
//! duration. The multi-input worker (every route started from the app) now
//! lives in a slot shared with one `audiorouter-audio` thread (COM MTA, MMCSS
//! "Pro Audio", 1 ms timer) that only pumps it. The control thread keeps every
//! decision: it creates and drops the worker, starts and stops it, and sets
//! `enabled` from the same checks the inline pump made (runtime state and
//! generation, endpoint invalidation, capture-sink leases).
//!
//! The audio thread never waits for the control thread: it only `try_lock`s
//! the slot and skips a pass when the control thread holds it, so a control
//! thread at normal priority cannot block it. The control thread holds the
//! slot only for short worker calls. The audio thread does not allocate, log
//! or touch the disk while pumping; a pump error is parked in the slot for the
//! control thread to report.
//!
//! Without a running backend service (tests, the CLI), nothing is spawned and
//! the control plane pumps through the same handle inline, as before.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

use audiorouter_domain::EntityId;
use audiorouter_windows_audio::{
    NativeMultiInputWorker, NativeMultiInputWorkerError, WasapiSchedulerPump,
};

/// Start of the monotonic clock behind pump deadlines.
static CLOCK_BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Microseconds since [`CLOCK_BASE`]; an atomic can hold it.
fn clock_micros() -> u64 {
    micros(CLOCK_BASE.get_or_init(Instant::now).elapsed())
}

/// No pump deadline: pumping continues while the control thread is busy.
const NO_DEADLINE: u64 = u64::MAX;

/// Time between audio thread passes; matches the backend loop's interval.
const AUDIO_THREAD_INTERVAL: Duration = Duration::from_millis(1);

/// The multi-input worker, asserted `Send` so the audio thread can call it.
///
/// The `windows` crate marks every COM interface `!Send`, because a COM object
/// may belong to a single-threaded apartment. The objects this worker owns are
/// WASAPI and MMDevice clients (shared-mode capture and render, process
/// loopback), which are free-threaded inside the multithreaded apartment.
struct MtaWorker(NativeMultiInputWorker);

// SAFETY: (1) both threads that touch the worker are in the COM MTA: the
// control thread through `AudioServiceThreadGuard` and the audio thread
// through its own guard, entered before its first pass. (2) The slot's mutex
// gives one thread access at a time, so nothing in the worker (including its
// `RefCell` binaural converter) is used concurrently. (3) The worker is
// created and dropped on the control thread: `MultiInputWorkerHandle::drop`
// and `AudioThread::drop` take it out of the slot there. Each capture and
// render client owns a `ComApartment` whose `CoUninitialize` must run on the
// thread that initialized it, which is that control thread. (4) No part of
// the worker is apartment-bound (no windows, no STA objects) or shares an
// `Rc` with code outside it.
unsafe impl Send for MtaWorker {}

/// A pump error parked by the audio thread for the control thread.
struct MtaError(NativeMultiInputWorkerError);

// SAFETY: the error may hold a `windows::core::Error`; it is created on the
// audio thread, stored under the slot's mutex and taken (and dropped) by the
// control thread, both in the MTA, one at a time.
unsafe impl Send for MtaError {}

#[derive(Default)]
struct Slot {
    worker: Option<MtaWorker>,
    /// Attach number of the handle that owns `worker`.
    owner: u64,
    /// The first pump error since the control thread last looked. Later
    /// errors only count, so the audio thread never drops an error value.
    error: Option<MtaError>,
}

/// State shared by the control plane, the worker handle and the audio thread.
#[derive(Default)]
pub(crate) struct AudioThreadShared {
    slot: Mutex<Slot>,
    /// Set by the control thread from its checks; the audio thread pumps only
    /// while it is true and the worker is running.
    enabled: AtomicBool,
    /// [`clock_micros`] after which the last checks no longer vouch for
    /// pumping (zero, the default, or [`NO_DEADLINE`] when nothing expires).
    pump_deadline_micros: AtomicU64,
    shutdown: AtomicBool,
    /// A pump panicked; pumping stopped until the control thread reacts.
    faulted: AtomicBool,
    mmcss_pro_audio: AtomicBool,
    /// The thread has entered its scheduling guard and runs passes.
    started: AtomicBool,
    passes: AtomicU64,
    late_gaps: AtomicU64,
    max_gap_micros: AtomicU64,
    pump_errors: AtomicU64,
    /// Passes skipped because the control thread held the slot.
    skipped_passes: AtomicU64,
    /// Maxima since the last `audioHealth` record (swapped to zero there).
    interval_max_gap_micros: AtomicU64,
    interval_max_pass_micros: AtomicU64,
    interval_skipped_passes: AtomicU64,
    /// What the audio thread pumped since the worker was attached; the
    /// control thread reports these instead of pumping itself.
    pumped: PumpTotalsAtomic,
}

#[derive(Default)]
struct PumpTotalsAtomic {
    packets: AtomicU64,
    captured_frames: AtomicU64,
    processed_quanta: AtomicU64,
    delivered_quanta: AtomicU64,
    rendered_frames: AtomicU64,
    render_backpressure_events: AtomicU64,
}

/// Running totals of one attached multi-input worker's pumps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PumpTotals {
    pub packets: u64,
    pub captured_frames: u64,
    pub processed_quanta: u64,
    pub delivered_quanta: u64,
    pub rendered_frames: u64,
    pub render_backpressure_events: u64,
}

impl PumpTotalsAtomic {
    fn add(&self, totals: PumpTotals) {
        self.packets.fetch_add(totals.packets, Ordering::Relaxed);
        self.captured_frames
            .fetch_add(totals.captured_frames, Ordering::Relaxed);
        self.processed_quanta
            .fetch_add(totals.processed_quanta, Ordering::Relaxed);
        self.delivered_quanta
            .fetch_add(totals.delivered_quanta, Ordering::Relaxed);
        self.rendered_frames
            .fetch_add(totals.rendered_frames, Ordering::Relaxed);
        self.render_backpressure_events
            .fetch_add(totals.render_backpressure_events, Ordering::Relaxed);
    }

    fn load(&self) -> PumpTotals {
        PumpTotals {
            packets: self.packets.load(Ordering::Relaxed),
            captured_frames: self.captured_frames.load(Ordering::Relaxed),
            processed_quanta: self.processed_quanta.load(Ordering::Relaxed),
            delivered_quanta: self.delivered_quanta.load(Ordering::Relaxed),
            rendered_frames: self.rendered_frames.load(Ordering::Relaxed),
            render_backpressure_events: self.render_backpressure_events.load(Ordering::Relaxed),
        }
    }

    fn reset(&self) {
        for counter in [
            &self.packets,
            &self.captured_frames,
            &self.processed_quanta,
            &self.delivered_quanta,
            &self.rendered_frames,
            &self.render_backpressure_events,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
    }
}

/// Counters of the audio thread for diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AudioThreadCounters {
    pub passes: u64,
    pub late_gaps: u64,
    pub max_gap_micros: u64,
    pub pump_errors: u64,
    pub skipped_passes: u64,
}

/// One `audioHealth` interval of the audio thread.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AudioThreadInterval {
    pub max_gap_micros: u64,
    pub max_pass_micros: u64,
    pub skipped_passes: u64,
    pub mmcss_pro_audio: bool,
}

fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

impl AudioThreadShared {
    /// Lock the slot from the control thread. A poisoned lock (a panic while
    /// it was held) still yields the slot: the panic is reported separately.
    fn lock(&self) -> MutexGuard<'_, Slot> {
        self.slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    /// Let the audio thread pump only for `valid_for` after this call, or
    /// with no limit (`None`). A driver lease the control thread renews
    /// between requests sets a limit, so a long request cannot keep audio
    /// flowing through an owner whose lease it never renewed.
    pub(crate) fn set_pump_valid_for(&self, valid_for: Option<Duration>) {
        let deadline = valid_for.map_or(NO_DEADLINE, |valid_for| {
            clock_micros().saturating_add(micros(valid_for)).max(1)
        });
        self.pump_deadline_micros.store(deadline, Ordering::Release);
    }

    fn checks_expired(&self) -> bool {
        let deadline = self.pump_deadline_micros.load(Ordering::Acquire);
        // Zero is the unset default.
        deadline != 0 && deadline != NO_DEADLINE && clock_micros() > deadline
    }

    pub(crate) fn thread_started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }

    pub(crate) fn is_faulted(&self) -> bool {
        self.faulted.load(Ordering::Acquire)
    }

    pub(crate) fn counters(&self) -> AudioThreadCounters {
        AudioThreadCounters {
            passes: self.passes.load(Ordering::Relaxed),
            late_gaps: self.late_gaps.load(Ordering::Relaxed),
            max_gap_micros: self.max_gap_micros.load(Ordering::Relaxed),
            pump_errors: self.pump_errors.load(Ordering::Relaxed),
            skipped_passes: self.skipped_passes.load(Ordering::Relaxed),
        }
    }

    /// What the audio thread pumped since the current worker was attached.
    pub(crate) fn pumped(&self) -> PumpTotals {
        self.pumped.load()
    }

    /// The interval maxima since the previous call, then reset them.
    pub(crate) fn take_interval(&self) -> AudioThreadInterval {
        AudioThreadInterval {
            max_gap_micros: self.interval_max_gap_micros.swap(0, Ordering::Relaxed),
            max_pass_micros: self.interval_max_pass_micros.swap(0, Ordering::Relaxed),
            skipped_passes: self.interval_skipped_passes.swap(0, Ordering::Relaxed),
            mmcss_pro_audio: self.mmcss_pro_audio.load(Ordering::Relaxed),
        }
    }

    /// The pump error the audio thread parked, if any. Control thread only.
    pub(crate) fn take_error(&self) -> Option<NativeMultiInputWorkerError> {
        self.lock().error.take().map(|error| error.0)
    }

    /// Take the worker out of the slot and stop pumping, if `owner` (a
    /// handle's attach number, or `None` for any) still owns it. The caller
    /// drops it on the control thread, outside the lock.
    fn take_worker(&self, owner: Option<u64>) -> Option<NativeMultiInputWorker> {
        let mut slot = self.lock();
        if owner.is_some_and(|owner| owner != slot.owner) {
            return None;
        }
        self.set_enabled(false);
        slot.worker.take().map(|worker| worker.0)
    }

    /// One audio thread pass: pump the worker if pumping is enabled and the
    /// slot is free. Returns whether a pump ran.
    fn pass(&self, max_packets: u32) -> bool {
        if !self.enabled.load(Ordering::Acquire)
            || self.faulted.load(Ordering::Acquire)
            || self.checks_expired()
        {
            return false;
        }
        let mut slot = match self.slot.try_lock() {
            Ok(slot) => slot,
            Err(TryLockError::WouldBlock) => {
                self.skipped_passes.fetch_add(1, Ordering::Relaxed);
                self.interval_skipped_passes.fetch_add(1, Ordering::Relaxed);
                return false;
            }
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        };
        let Some(worker) = slot.worker.as_mut().filter(|worker| worker.0.is_running()) else {
            return false;
        };
        let worker = &mut worker.0;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pump_once(worker, max_packets)
        }));
        match result {
            Ok(Ok(totals)) => self.pumped.add(totals),
            Ok(Err(error)) => {
                self.pump_errors.fetch_add(1, Ordering::Relaxed);
                if slot.error.is_none() {
                    slot.error = Some(MtaError(error));
                }
            }
            Err(_) => {
                // The panic hook records it; the control thread restarts
                // the backend as it did when pumping panicked inline.
                self.faulted.store(true, Ordering::Release);
                self.enabled.store(false, Ordering::Release);
            }
        }
        true
    }

    /// Record a pass that started at `start` and pumped (`pumped`), or end
    /// the gap series when nothing was pumped.
    fn record(&self, last_pump: &mut Option<Instant>, start: Instant, pumped: bool) {
        if !pumped {
            // A skipped pass keeps the series going, so its delay shows as
            // a longer gap; a disabled or stopped worker ends the series.
            if !self.enabled.load(Ordering::Relaxed) {
                *last_pump = None;
            }
            return;
        }
        if let Some(previous) = *last_pump {
            let gap = micros(start.saturating_duration_since(previous));
            self.max_gap_micros.fetch_max(gap, Ordering::Relaxed);
            self.interval_max_gap_micros
                .fetch_max(gap, Ordering::Relaxed);
            if gap > micros(crate::AUDIO_SERVICE_LATE_GAP) {
                self.late_gaps.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.interval_max_pass_micros
            .fetch_max(micros(start.elapsed()), Ordering::Relaxed);
        self.passes.fetch_add(1, Ordering::Relaxed);
        *last_pump = Some(start);
    }
}

/// Pump the worker once, exactly as the inline service pass does, and
/// return what it moved.
pub(crate) fn pump_once(
    worker: &mut NativeMultiInputWorker,
    max_packets: u32,
) -> Result<PumpTotals, NativeMultiInputWorkerError> {
    let (input, delivered, render) = if worker.has_output_owner() {
        worker.pump_and_process_outputs(max_packets)?
    } else {
        (
            worker.pump_available(max_packets)?,
            0,
            WasapiSchedulerPump::default(),
        )
    };
    Ok(PumpTotals {
        packets: u64::from(input.packets),
        captured_frames: u64::from(input.captured_frames),
        processed_quanta: u64::from(input.processed_quanta),
        delivered_quanta: u64::try_from(delivered).unwrap_or(u64::MAX),
        rendered_frames: u64::from(render.rendered_frames),
        render_backpressure_events: u64::from(render.render_backpressure_events),
    })
}

/// The running audio thread. Dropping it stops and joins the thread, then
/// drops any worker still in the slot on the dropping (control) thread.
pub(crate) struct AudioThread {
    shared: Arc<AudioThreadShared>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl AudioThread {
    /// Start the thread. Fails, so the caller keeps pumping inline, when the
    /// thread cannot be created or cannot join the COM MTA (the worker's
    /// `Send` assertion depends on it).
    pub(crate) fn spawn(shared: Arc<AudioThreadShared>) -> std::io::Result<Self> {
        let thread_shared = shared.clone();
        let (ready_sender, ready) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("audiorouter-audio".into())
            .spawn(move || {
                let (_scheduling, capabilities) =
                    audiorouter_windows_audio::AudioServiceThreadGuard::enter();
                let _ = ready_sender.send(capabilities.com_multithreaded);
                if !capabilities.com_multithreaded {
                    return;
                }
                thread_shared
                    .mmcss_pro_audio
                    .store(capabilities.mmcss_pro_audio, Ordering::Relaxed);
                thread_shared.started.store(true, Ordering::Release);
                let budget = audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE;
                let mut last_pump = None;
                while !thread_shared.shutdown.load(Ordering::Acquire) {
                    let start = Instant::now();
                    let pumped = thread_shared.pass(budget);
                    thread_shared.record(&mut last_pump, start, pumped);
                    std::thread::sleep(AUDIO_THREAD_INTERVAL);
                }
            })?;
        if ready.recv() != Ok(true) {
            let _ = thread.join();
            return Err(std::io::Error::other(
                "the audio thread could not join the COM multithreaded apartment",
            ));
        }
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }
}

impl Drop for AudioThread {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        drop(self.shared.take_worker(None));
    }
}

/// The control plane's handle to the multi-input worker in the shared slot.
/// Each method locks the slot for one short worker call. Dropping the handle
/// takes its worker out and drops it on the calling (control) thread.
pub(crate) struct MultiInputWorkerHandle {
    shared: Arc<AudioThreadShared>,
    owner: u64,
}

impl MultiInputWorkerHandle {
    /// Move a prepared worker into the shared slot. Pumping stays disabled
    /// until the control thread enables it.
    pub(crate) fn attach(shared: &Arc<AudioThreadShared>, worker: NativeMultiInputWorker) -> Self {
        shared.set_enabled(false);
        shared.faulted.store(false, Ordering::Release);
        shared.pumped.reset();
        let mut slot = shared.lock();
        slot.error = None;
        slot.owner = slot.owner.wrapping_add(1);
        let owner = slot.owner;
        let previous = slot.worker.replace(MtaWorker(worker));
        drop(slot);
        // The plane attaches only when no worker is attached; a leftover
        // worker is still dropped here, on the control thread, and its
        // handle no longer owns the slot.
        drop(previous);
        Self {
            shared: shared.clone(),
            owner,
        }
    }

    /// Enable or disable pumping by the audio thread.
    pub(crate) fn set_pumping(&self, enabled: bool) {
        self.shared.set_enabled(enabled);
    }

    /// Run `f` with the worker while holding the slot.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&NativeMultiInputWorker) -> R) -> R {
        let slot = self.shared.lock();
        f(&slot
            .worker
            .as_ref()
            .expect("an attached handle always has a worker")
            .0)
    }

    /// Run `f` with the worker mutably while holding the slot.
    pub(crate) fn with_mut<R>(&self, f: impl FnOnce(&mut NativeMultiInputWorker) -> R) -> R {
        let mut slot = self.shared.lock();
        f(&mut slot
            .worker
            .as_mut()
            .expect("an attached handle always has a worker")
            .0)
    }

    pub(crate) fn is_running(&self) -> bool {
        self.with(NativeMultiInputWorker::is_running)
    }

    pub(crate) fn output_count(&self) -> usize {
        self.with(NativeMultiInputWorker::output_count)
    }

    pub(crate) fn has_output_owner(&self) -> bool {
        self.with(NativeMultiInputWorker::has_output_owner)
    }

    pub(crate) fn path_count(&self) -> usize {
        self.with(NativeMultiInputWorker::path_count)
    }

    pub(crate) fn output_node_ids(&self) -> Vec<EntityId> {
        self.with(|worker| worker.output_node_ids().to_vec())
    }

    pub(crate) fn input_node_ids(&self) -> Vec<EntityId> {
        self.with(|worker| worker.input_node_ids().to_vec())
    }

    pub(crate) fn input_wait_ms(&self) -> Vec<Option<f64>> {
        self.with(NativeMultiInputWorker::input_wait_ms)
    }

    pub(crate) fn input_discontinuities(&self) -> Vec<u64> {
        self.with(NativeMultiInputWorker::input_discontinuities)
    }

    pub(crate) fn input_trimmed_packets(&self) -> Vec<u64> {
        self.with(NativeMultiInputWorker::input_trimmed_packets)
    }

    pub(crate) fn output_underruns(&self) -> u64 {
        self.with(NativeMultiInputWorker::output_underruns)
    }

    pub(crate) fn output_underruns_per_output(&self) -> Vec<u64> {
        self.with(NativeMultiInputWorker::output_underruns_per_output)
    }

    pub(crate) fn output_full_drops(&self) -> Vec<u64> {
        self.with(NativeMultiInputWorker::output_full_drops)
    }

    pub(crate) fn output_queue_ms(&self) -> Vec<Option<f64>> {
        self.with(NativeMultiInputWorker::output_queue_ms)
    }

    pub(crate) fn bindings_affected_by(
        &self,
        changes: &[audiorouter_windows_audio::EndpointChange],
    ) -> bool {
        self.with(|worker| worker.bindings_affected_by(changes))
    }

    pub(crate) fn network_receive_stats(
        &self,
        input: usize,
    ) -> Option<audiorouter_windows_audio::NetworkReceiveStats> {
        self.with(|worker| worker.network_receive_stats(input))
    }

    pub(crate) fn network_send_stats(
        &self,
        output: usize,
    ) -> Option<audiorouter_windows_audio::NetworkSendStats> {
        self.with(|worker| worker.network_send_stats(output))
    }

    pub(crate) fn reset_meter_for_node(&self, node_id: &EntityId) -> bool {
        self.with(|worker| worker.reset_meter_for_node(node_id))
    }

    pub(crate) fn meter_snapshot_for_node(
        &self,
        node_id: &EntityId,
    ) -> Option<audiorouter_engine::BlockMeterSnapshot> {
        self.with(|worker| worker.meter_snapshot_for_node(node_id))
    }

    pub(crate) fn stage_timing_for_node(
        &self,
        node_id: &EntityId,
    ) -> Option<audiorouter_engine::StageTiming> {
        self.with(|worker| worker.stage_timing_for_node(node_id))
    }

    pub(crate) fn processor_telemetry_for_node(
        &self,
        node_id: &EntityId,
    ) -> Option<audiorouter_engine::ProcessorTelemetry> {
        self.with(|worker| worker.processor_telemetry_for_node(node_id))
    }

    pub(crate) fn plugin_health_for_node(
        &self,
        node_id: &EntityId,
    ) -> Option<audiorouter_engine::PluginWorkerHealth> {
        self.with(|worker| worker.plugin_health_for_node(node_id))
    }

    pub(crate) fn voice_changer_for_node(
        &self,
        node_id: &EntityId,
    ) -> Option<Arc<audiorouter_engine::VoiceChangerState>> {
        self.with(|worker| worker.voice_changer_for_node(node_id))
    }

    pub(crate) fn noise_profile_for_node(&self, node_id: &EntityId) -> Option<String> {
        self.with(|worker| worker.noise_profile_for_node(node_id))
    }

    pub(crate) fn spectrum_levels_for_node(
        &self,
        node_id: &EntityId,
    ) -> Option<audiorouter_engine::SpectrumLevels> {
        self.with(|worker| worker.spectrum_levels_for_node(node_id))
    }

    pub(crate) fn test_signal_source_for_node(
        &self,
        node_id: &EntityId,
    ) -> Option<Arc<audiorouter_engine::TestSignalSource>> {
        self.with(|worker| worker.test_signal_source_for_node(node_id))
    }

    pub(crate) fn audio_file_source_for_node(
        &self,
        node_id: &EntityId,
    ) -> Option<Arc<audiorouter_engine::AudioFileSource>> {
        self.with(|worker| worker.audio_file_source_for_node(node_id))
    }

    pub(crate) fn capture_is_silent(&self, index: usize) -> bool {
        self.with(|worker| worker.capture_is_silent(index))
    }

    pub(crate) fn capture_spatial_options(
        &self,
        index: usize,
    ) -> Option<audiorouter_dsp::binaural::SpatialOptions> {
        self.with(|worker| worker.capture_spatial_options(index))
    }

    pub(crate) fn set_privacy_muted(&self, muted: bool) {
        self.with(|worker| worker.set_privacy_muted(muted));
    }

    pub(crate) fn start(&self) -> Result<(), NativeMultiInputWorkerError> {
        self.with_mut(NativeMultiInputWorker::start)
    }

    /// Stop the worker. Pumping is disabled first, so the audio thread does
    /// not race the stop for the slot.
    pub(crate) fn stop(&self) -> Result<(), NativeMultiInputWorkerError> {
        self.shared.set_enabled(false);
        self.with_mut(NativeMultiInputWorker::stop)
    }

    pub(crate) fn attach_output_fanout(
        &self,
        outputs: audiorouter_windows_audio::WasapiOutputFanout,
    ) -> Result<(), NativeMultiInputWorkerError> {
        self.with_mut(|worker| worker.attach_output_fanout(outputs))
    }

    pub(crate) fn attach_output_branch_tap_sets(
        &self,
        tap_sets: Vec<audiorouter_engine::AudioTapSet>,
    ) -> Result<(), NativeMultiInputWorkerError> {
        self.with_mut(|worker| worker.attach_output_branch_tap_sets(tap_sets))
    }

    pub(crate) fn attach_tap_only_branches(
        &self,
        tap_sets: Vec<audiorouter_engine::AudioTapSet>,
        channels: usize,
        quantum_frames: usize,
    ) -> Result<(), NativeMultiInputWorkerError> {
        self.with_mut(|worker| worker.attach_tap_only_branches(tap_sets, channels, quantum_frames))
    }

    /// Replace one capture. The replaced capture is dropped inside the call,
    /// on this (control) thread, which created it.
    pub(crate) fn replace_capture(
        &self,
        index: usize,
        capture: audiorouter_windows_audio::MultiInputCaptureSource,
    ) -> Result<(), NativeMultiInputWorkerError> {
        self.with_mut(|worker| worker.replace_capture(index, capture))
    }

    pub(crate) fn replace_path_set(
        &self,
        set: audiorouter_engine::CompiledPathSet,
    ) -> Result<(), audiorouter_engine::MixerFanoutError> {
        self.with_mut(|worker| worker.replace_path_set(set))
    }

    /// Pump on the calling thread (no audio thread, or a request that asks
    /// for an explicit pump without one).
    pub(crate) fn pump_available(
        &self,
        max_packets: u32,
    ) -> Result<WasapiSchedulerPump, NativeMultiInputWorkerError> {
        self.with_mut(|worker| worker.pump_available(max_packets))
    }

    pub(crate) fn pump_and_process_outputs(
        &self,
        max_packets: u32,
    ) -> Result<(WasapiSchedulerPump, usize, WasapiSchedulerPump), NativeMultiInputWorkerError>
    {
        self.with_mut(|worker| worker.pump_and_process_outputs(max_packets))
    }
}

impl Drop for MultiInputWorkerHandle {
    fn drop(&mut self) {
        drop(self.shared.take_worker(Some(self.owner)));
    }
}

#[cfg(test)]
#[path = "audio_thread_tests.rs"]
mod tests;
