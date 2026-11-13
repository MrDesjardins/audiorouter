//! File encoder ownership off the native audio service thread (REC-01/08).
use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};

type Command = Box<dyn FnOnce(&mut dyn RecorderWorker) + Send>;

pub(super) struct ThreadedRecorderWorker {
    commands: Option<SyncSender<Command>>,
    thread: Option<std::thread::JoinHandle<()>>,
    queue: Arc<RecordingQueue>,
    tap: Option<Arc<dyn AudioTap>>,
    drained: Arc<AtomicUsize>,
    failures: mpsc::Receiver<String>,
    failure: Option<String>,
    finalized: Vec<FinalizedRecording>,
    stop: Arc<AtomicBool>,
    finished: mpsc::Receiver<()>,
    shutdown_deadline: Option<Instant>,
}

impl ThreadedRecorderWorker {
    pub(super) fn new(
        mut worker: Box<dyn RecorderWorker>,
        maximum_chunks: usize,
    ) -> Result<Self, String> {
        let queue = worker
            .shared_recording_queue()
            .ok_or("file encoder has no shared queue")?;
        let tap = worker.shared_audio_tap();
        // Idle/armed recording must not consume pool capacity before Start.
        queue.close_tap_admission();
        let (commands, receive) = mpsc::sync_channel::<Command>(1);
        let (failed, failures) = mpsc::sync_channel(1);
        let drained = Arc::new(AtomicUsize::new(0));
        let progress = drained.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let (completed, finished) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("audiorouter-recorder".into())
            .spawn(move || {
                let mut faulted = false;
                let mut made_progress = false;
                loop {
                    if stopping.load(Ordering::Acquire) {
                        break;
                    }
                    // Do not sleep between batches while audio is queued: a
                    // one-chunk API budget must not throttle encoder throughput.
                    let wait = if made_progress {
                        Duration::ZERO
                    } else {
                        Duration::from_millis(2)
                    };
                    match receive.recv_timeout(wait) {
                        Ok(command) => {
                            if stopping.load(Ordering::Acquire) {
                                break;
                            }
                            command(worker.as_mut());
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    if stopping.load(Ordering::Acquire) {
                        break;
                    }
                    made_progress = false;
                    if !faulted {
                        match worker.drain_pending(maximum_chunks) {
                            Ok(count) => {
                                made_progress = count != 0;
                                progress.fetch_add(count, Ordering::Release);
                            }
                            Err(error) => {
                                faulted = true;
                                let _ = failed.try_send(error);
                            }
                        }
                    }
                }
                drop(worker);
                let _ = completed.try_send(());
            })
            .map_err(|error| format!("recorder worker start failed: {error}"))?;
        Ok(Self {
            commands: Some(commands),
            thread: Some(thread),
            queue,
            tap,
            drained,
            failures,
            failure: None,
            finalized: Vec::new(),
            stop,
            finished,
            shutdown_deadline: None,
        })
    }

    // Explicit lifecycle commands may wait for their durable outcome. Routine
    // pumping never calls this: it reads only bounded progress/failure state.
    fn call<R: Send + 'static>(
        &mut self,
        operation: impl FnOnce(&mut dyn RecorderWorker) -> R + Send + 'static,
    ) -> Result<R, String> {
        let wait = self
            .shutdown_deadline
            .map_or(Duration::from_secs(10), |deadline| {
                deadline.saturating_duration_since(Instant::now())
            });
        if wait.is_zero() {
            return Err(self.timeout());
        }
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .ok_or("recorder worker stopped")?
            .try_send(Box::new(move |worker| {
                let _ = reply.send(operation(worker));
            }))
            .map_err(|_| "recorder worker stopped or command queue is busy".to_owned())?;
        match response.recv_timeout(wait) {
            Ok(response) => Ok(response),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(self.timeout()),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err("recorder worker stopped before replying".into())
            }
        }
    }

    fn timeout(&mut self) -> String {
        self.queue.close_tap_admission();
        self.stop.store(true, Ordering::Release);
        self.commands.take();
        let message =
            "recorder command timed out; recorded prefix retained for recovery".to_owned();
        self.failure = Some(message.clone());
        message
    }
}

impl RecorderWorker for ThreadedRecorderWorker {
    fn set_shutdown_deadline(&mut self, deadline: Instant) {
        self.shutdown_deadline = Some(deadline);
    }
    fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
        Some(self.queue.clone())
    }
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        self.tap.clone()
    }
    fn committed_end_frame(&self) -> Option<u64> {
        self.queue.committed_end_frame()
    }
    fn set_library_identity(&mut self, identity: FileRecordingIdentity) -> Result<(), String> {
        self.call(move |worker| worker.set_library_identity(identity))?
    }
    fn arm(&mut self) -> Result<(), String> {
        self.call(|worker| worker.arm())?
    }
    fn start(&mut self, frame: u64) -> Result<(), String> {
        self.call(move |worker| worker.start(frame))??;
        self.queue.open_tap_admission();
        Ok(())
    }
    fn pause(&mut self, frame: u64) -> Result<(), String> {
        self.call(move |worker| worker.pause(frame))?
    }
    fn resume(&mut self, frame: u64) -> Result<(), String> {
        self.call(move |worker| worker.resume(frame))?
    }
    fn split(&mut self, frame: u64) -> Result<(), String> {
        let queue = self.queue.clone();
        self.call(move |worker| {
            // Retire pre-split callbacks and queued frames before changing
            // the encoder's timeline, just as Pause does.
            drain_before_recorder_pause(worker, &queue)?;
            let result = worker.split(frame);
            if result.is_ok() {
                queue.open_tap_admission();
            }
            result
        })?
    }
    fn drain_pending(&mut self, _maximum_chunks: usize) -> Result<usize, String> {
        if self.queue.overruns() != 0 {
            self.queue.close_tap_admission();
            return Err("recording queue overflow; recorded prefix retained".into());
        }
        if self.failure.is_none() {
            self.failure = self.failures.try_recv().ok();
        }
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if self
            .thread
            .as_ref()
            .is_some_and(|thread| thread.is_finished())
        {
            return Err("recorder worker stopped unexpectedly".into());
        }
        Ok(self.drained.swap(0, Ordering::AcqRel))
    }
    fn finalized_recordings(&self) -> Vec<FinalizedRecording> {
        self.finalized.clone()
    }
    fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        self.queue.close_tap_admission();
        let lost = self.queue.overruns() != 0;
        let (mut outcome, mut recordings) = self.call(move |worker| {
            let outcome = worker.finalize(frame);
            (outcome, worker.finalized_recordings())
        })?;
        // A dropped tail may have no later block to expose a frame gap.
        if lost {
            if let Ok(outcome) = &mut outcome {
                outcome.state = "failed".into();
            }
            for recording in &mut recordings {
                recording.state = "failed".into();
            }
        }
        self.finalized = recordings;
        outcome
    }
}

impl Drop for ThreadedRecorderWorker {
    fn drop(&mut self) {
        // An OS write can stall indefinitely. Cancel pending commands and
        // leave file ownership on that thread until it unblocks; never turn
        // Drop into an unbounded second wait after a finalization timeout.
        self.queue.close_tap_admission();
        self.stop.store(true, Ordering::Release);
        self.commands.take();
        let wait = self
            .shutdown_deadline
            .map_or(Duration::from_millis(50), |deadline| {
                deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(50))
            });
        let _ = self.finished.recv_timeout(wait);
        if let Some(thread) = self.thread.take() {
            if thread.is_finished() {
                let _ = thread.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct ReleaseOnDrop(Option<SyncSender<()>>);
    impl ReleaseOnDrop {
        fn release(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.try_send(());
            }
        }
    }
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            self.release();
        }
    }
    #[test]
    fn a_stalled_finalize_and_worker_drop_return_before_storage_unblocks() {
        struct Stalled {
            queue: Arc<RecordingQueue>,
            entered: SyncSender<()>,
            release: mpsc::Receiver<()>,
            dropped: SyncSender<()>,
        }
        impl Drop for Stalled {
            fn drop(&mut self) {
                let _ = self.dropped.try_send(());
            }
        }
        impl RecorderWorker for Stalled {
            fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
                Some(self.queue.clone())
            }
            fn finalize(&mut self, _: u64) -> Result<RecorderFinalizationOutcome, String> {
                self.entered.send(()).unwrap();
                self.release.recv().unwrap();
                Ok(RecorderFinalizationOutcome {
                    state: "completed".into(),
                    file_finalized: true,
                    recoverable: false,
                })
            }
        }
        let (entered, entering) = mpsc::sync_channel(1);
        let (release, blocked) = mpsc::sync_channel(1);
        let mut release = ReleaseOnDrop(Some(release));
        let (dropped, dropping) = mpsc::sync_channel(1);
        let (answer, result) = mpsc::sync_channel(1);
        let caller = std::thread::spawn(move || {
            let mut worker = ThreadedRecorderWorker::new(
                Box::new(Stalled {
                    queue: Arc::new(RecordingQueue::new(4).unwrap()),
                    entered,
                    release: blocked,
                    dropped,
                }),
                4,
            )
            .unwrap();
            worker.set_shutdown_deadline(Instant::now() + Duration::from_millis(50));
            let outcome = worker.finalize(0);
            drop(worker);
            answer.send(outcome).unwrap();
        });
        entering.recv_timeout(Duration::from_secs(2)).unwrap();
        // A rescue always unblocks the fake, so the regression fails rather
        // than hanging the test process on the old unbounded wait/join.
        let outcome = result.recv_timeout(Duration::from_secs(2));
        release.release();
        caller.join().unwrap();
        let failure = outcome
            .expect("shutdown must return while the file worker is still blocked")
            .unwrap_err();
        assert!(failure.contains("timed out"), "{failure}");
        dropping.recv_timeout(Duration::from_secs(2)).unwrap();
    }

    #[test]
    fn dispatcher_stop_and_quit_retire_multiple_stalled_encoders() {
        struct Stalled {
            queue: Arc<RecordingQueue>,
            tap: Arc<dyn AudioTap>,
            release: mpsc::Receiver<()>,
        }
        impl RecorderWorker for Stalled {
            fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
                Some(self.queue.clone())
            }
            fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
                Some(self.tap.clone())
            }
            fn finalize(&mut self, _: u64) -> Result<RecorderFinalizationOutcome, String> {
                self.release.recv().unwrap();
                Ok(RecorderFinalizationOutcome {
                    state: "completed".into(),
                    file_finalized: true,
                    recoverable: false,
                })
            }
        }
        for method in ["sessions.stop", "system.quit"] {
            let mut releases = Vec::new();
            let mut blocked = Vec::new();
            for _ in 0..2 {
                let (release, receive) = mpsc::sync_channel(1);
                releases.push(ReleaseOnDrop(Some(release)));
                blocked.push(receive);
            }
            let (answer, result) = mpsc::sync_channel(1);
            let caller = std::thread::spawn(move || {
                let mut plane = ControlPlane {
                    recorder_shutdown_budget: Some(Duration::from_millis(50)),
                    ..ControlPlane::default()
                };
                let mut saved = crate::test_support::session();
                for id in ["recorder-a", "recorder-b"] {
                    saved.nodes.push(audiorouter_domain::Node {
                        id: EntityId::new(id),
                        kind: NodeKind::Recorder,
                        type_version: 1,
                        name: id.into(),
                        enabled: true,
                        bypass: false,
                        parameters: Default::default(),
                        ports: vec![],
                    });
                }
                let id = saved.id.clone();
                plane.insert_session(saved).unwrap();
                plane.session_start(&id).unwrap();
                for (name, release) in ["recorder-a", "recorder-b"].into_iter().zip(blocked) {
                    let queue = Arc::new(RecordingQueue::new(4).unwrap());
                    let tap: Arc<dyn AudioTap> = Arc::new(RecorderAudioTap::new(queue.clone()));
                    let worker = ThreadedRecorderWorker::new(
                        Box::new(Stalled {
                            queue,
                            tap,
                            release,
                        }),
                        4,
                    )
                    .unwrap();
                    let node = EntityId::new(name);
                    plane
                        .attach_recorder_worker_to_node(&id, node.clone(), Box::new(worker))
                        .unwrap();
                    plane
                        .control_recorder_node(&node, "recorders.arm", None)
                        .unwrap();
                    plane
                        .control_recorder_node(&node, "recorders.start", Some(0))
                        .unwrap();
                }
                let params = if method == "sessions.stop" {
                    json!({"sessionId": id, "idempotencyKey": "stop"})
                } else {
                    json!({"idempotencyKey": "quit"})
                };
                let response = plane.dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: method.into(),
                    params: Some(params),
                });
                answer.send(json!({"result": response.result, "active": plane.status_snapshot().unwrap()["activeSessionIds"],
                    "workers": plane.recorder_node_workers.len()})).unwrap();
            });
            let response = result.recv_timeout(Duration::from_secs(2));
            for release in &mut releases {
                release.release();
            }
            caller.join().unwrap();
            let response = response
                .expect("actual Stop/Quit dispatch must return before either encoder is released");
            assert_eq!(response["result"]["state"], "stopped", "{response}");
            assert_eq!(response["active"], json!([]));
            assert_eq!(response["workers"], 0);
            let recorders = response["result"]["recorders"].as_array().unwrap();
            assert_eq!(recorders.len(), 2);
            assert!(recorders
                .iter()
                .all(|recorder| recorder["state"] == "failed" && recorder["recoverable"] == true));
        }
    }

    #[test]
    fn blocked_file_writer_overflow_keeps_a_playable_prefix_and_fails_only_recording() {
        struct BlockedFile {
            worker: SegmentedWavRecorderWorker,
            block: Arc<std::sync::atomic::AtomicBool>,
            entered: SyncSender<()>,
            release: mpsc::Receiver<()>,
        }
        impl RecorderWorker for BlockedFile {
            fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
                Some(self.worker.queue.clone())
            }
            fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
                self.worker.shared_audio_tap()
            }
            fn arm(&mut self) -> Result<(), String> {
                self.worker.arm()
            }
            fn start(&mut self, frame: u64) -> Result<(), String> {
                self.worker.start(frame)
            }
            fn drain_pending(&mut self, maximum: usize) -> Result<usize, String> {
                if self.block.swap(false, Ordering::AcqRel) {
                    self.entered.send(()).unwrap();
                    self.release.recv().unwrap();
                }
                self.worker.drain_pending(maximum)
            }
            fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String> {
                self.worker.finalize(frame)
            }
            fn finalized_recordings(&self) -> Vec<FinalizedRecording> {
                self.worker.finalized_recordings()
            }
        }
        let root = std::env::temp_dir().join(format!(
            "audiorouter-threaded-overflow-{}-{}",
            std::process::id(),
            unix_epoch_seconds()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let writer = SegmentedWavRecorderWorker::new(
            RecordingPathPolicy::new(&root).unwrap(),
            "fixture",
            "rec",
            WavFormat::Pcm16,
            1,
            48_000,
            4,
            4,
            48_000,
        )
        .unwrap();
        let path = writer.initial_path().to_owned();
        let block = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (entered, entered_receiver) = mpsc::sync_channel(1);
        let (release, release_receiver) = mpsc::sync_channel(1);
        let mut worker = ThreadedRecorderWorker::new(
            Box::new(BlockedFile {
                worker: writer,
                block: block.clone(),
                entered,
                release: release_receiver,
            }),
            4,
        )
        .unwrap();
        let mut release_guard = ReleaseOnDrop(Some(release));
        worker.arm().unwrap();
        worker.start(0).unwrap();
        let queue = worker.queue.clone();
        let tap = worker.shared_audio_tap().unwrap();
        let mut audio = AudioBlock::new(1, 128).unwrap();
        audio.channel_mut(0).unwrap().fill(0.25);
        tap.on_processed_block(0, &audio);
        // Split is a durable barrier and guarantees this prefix is written.
        worker
            .call(|writer| writer.drain_pending(4))
            .unwrap()
            .unwrap();
        block.store(true, Ordering::Release);
        entered_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        for index in 1..8 {
            tap.on_processed_block(index * 128, &audio);
        }
        assert_eq!(queue.len(), 4);
        assert!(queue.committed_end_frame().unwrap() < 8 * 128);
        // The service thread still advances while the file writer is blocked.
        assert!(worker.drain_pending(4).unwrap_err().contains("overflow"));
        release_guard.release();
        // A dropped tail with no subsequent audio must still be marked failed.
        let finalized = worker.finalize(9 * 128).unwrap();
        assert_eq!(finalized.state, "failed");
        assert!(finalized.file_finalized);
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len() - 8
        );
        assert!(bytes.len() > 44 + 128 * 2);
        drop(worker);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_blocked_encoder_never_blocks_service_progress_or_frame_reads() {
        struct Slow {
            queue: Arc<RecordingQueue>,
            entered: SyncSender<()>,
            release: mpsc::Receiver<()>,
        }
        impl RecorderWorker for Slow {
            fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
                Some(self.queue.clone())
            }
            fn drain_pending(&mut self, _: usize) -> Result<usize, String> {
                self.entered.send(()).unwrap();
                self.release.recv().unwrap();
                Err("simulated storage failure".into())
            }
            fn finalize(&mut self, _: u64) -> Result<RecorderFinalizationOutcome, String> {
                Ok(RecorderFinalizationOutcome {
                    state: "failed".into(),
                    file_finalized: false,
                    recoverable: true,
                })
            }
        }
        let queue = Arc::new(RecordingQueue::new_pooled(4, 1, 128).unwrap());
        let (entered, entered_receiver) = mpsc::sync_channel(1);
        let (release, release_receiver) = mpsc::sync_channel(1);
        let mut worker = ThreadedRecorderWorker::new(
            Box::new(Slow {
                queue: queue.clone(),
                entered,
                release: release_receiver,
            }),
            4,
        )
        .unwrap();
        let mut release_guard = ReleaseOnDrop(Some(release));
        entered_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        // The writer cannot finish until this thread releases it. These reads
        // therefore prove independence without a fragile elapsed-time assertion.
        for _ in 0..1000 {
            assert_eq!(worker.drain_pending(4).unwrap(), 0);
        }
        queue.note_committed_end_frame(384);
        assert_eq!(worker.committed_end_frame(), Some(384));
        release_guard.release();
        assert_eq!(worker.finalize(384).unwrap().state, "failed");
        assert_eq!(
            worker.drain_pending(4).unwrap_err(),
            "simulated storage failure"
        );
    }
    #[test]
    fn the_recorder_thread_writes_audio_while_the_control_thread_never_drains() {
        // A slow control request stops the backend's service pass, but the
        // audio thread keeps feeding recorder taps. Ten times the queue's
        // capacity goes through with no `drain_pending` call from this thread.
        let root = std::env::temp_dir().join(format!(
            "audiorouter-threaded-independent-{}-{}",
            std::process::id(),
            unix_epoch_seconds()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let writer = SegmentedWavRecorderWorker::new(
            RecordingPathPolicy::new(&root).unwrap(),
            "fixture",
            "rec",
            WavFormat::Pcm16,
            1,
            48_000,
            4,
            4,
            48_000,
        )
        .unwrap();
        let path = writer.initial_path().to_owned();
        let mut worker = ThreadedRecorderWorker::new(Box::new(writer), 4).unwrap();
        worker.arm().unwrap();
        worker.start(0).unwrap();
        let queue = worker.queue.clone();
        let tap = worker.shared_audio_tap().unwrap();
        let mut audio = AudioBlock::new(1, 128).unwrap();
        audio.channel_mut(0).unwrap().fill(0.25);
        let blocks = 40_u64;
        for index in 0..blocks {
            tap.on_processed_block(index * 128, &audio);
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while !queue.is_empty() {
                assert!(std::time::Instant::now() < deadline, "queue not drained");
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        assert_eq!(queue.overruns(), 0);
        let finalized = worker.finalize(blocks * 128).unwrap();
        assert!(finalized.file_finalized, "{}", finalized.state);
        assert_eq!(
            crate::test_support::assert_playable_wav(&path),
            blocks * 128 * 2
        );
        drop(worker);
        std::fs::remove_dir_all(root).unwrap();
    }
}
