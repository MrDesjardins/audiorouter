//! File encoder ownership off the native audio service thread (REC-01/08).
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
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
        let thread = std::thread::Builder::new()
            .name("audiorouter-recorder".into())
            .spawn(move || {
                let mut faulted = false;
                let mut made_progress = false;
                loop {
                    // Do not sleep between batches while audio is queued: a
                    // one-chunk API budget must not throttle encoder throughput.
                    let wait = if made_progress {
                        Duration::ZERO
                    } else {
                        Duration::from_millis(2)
                    };
                    match receive.recv_timeout(wait) {
                        Ok(command) => command(worker.as_mut()),
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
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
        })
    }

    // Explicit lifecycle commands may wait for their durable outcome. Routine
    // pumping never calls this: it reads only bounded progress/failure state.
    fn call<R: Send + 'static>(
        &mut self,
        operation: impl FnOnce(&mut dyn RecorderWorker) -> R + Send + 'static,
    ) -> Result<R, String> {
        let (reply, response) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .ok_or("recorder worker stopped")?
            .send(Box::new(move |worker| {
                let _ = reply.send(operation(worker));
            }))
            .map_err(|_| "recorder worker stopped".to_owned())?;
        response
            .recv()
            .map_err(|_| "recorder worker stopped before replying".to_owned())
    }
}

impl RecorderWorker for ThreadedRecorderWorker {
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
        // Lifecycle/control ownership only. Dropping the sender lets the writer
        // finish its current bounded batch and release the file on its own thread.
        self.commands.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
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
}
