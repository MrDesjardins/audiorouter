use super::*;

#[test]
fn a_disabled_or_empty_slot_never_pumps() {
    let shared = AudioThreadShared::default();
    assert!(!shared.pass(64), "disabled");
    shared.set_enabled(true);
    assert!(!shared.pass(64), "no worker attached");
    assert_eq!(shared.counters().skipped_passes, 0);
}

#[test]
fn a_pass_skips_instead_of_waiting_while_the_control_thread_holds_the_slot() {
    let shared = Arc::new(AudioThreadShared::default());
    shared.set_enabled(true);
    let held = shared.lock();
    let audio = {
        let shared = shared.clone();
        std::thread::spawn(move || shared.pass(64))
    };
    // The pass returns while the lock is still held: it did not wait.
    assert!(!audio.join().unwrap());
    drop(held);
    assert_eq!(shared.counters().skipped_passes, 1);
    assert_eq!(shared.take_interval().skipped_passes, 1);
    assert_eq!(shared.take_interval().skipped_passes, 0);
}

#[test]
fn a_faulted_thread_stops_pumping_until_a_new_worker_attaches() {
    let shared = AudioThreadShared::default();
    shared.set_enabled(true);
    shared.faulted.store(true, Ordering::Release);
    assert!(shared.is_faulted());
    assert!(!shared.pass(64));
    assert_eq!(shared.counters().skipped_passes, 0);
}

#[test]
fn gaps_between_pumps_are_measured_and_late_ones_counted() {
    let shared = AudioThreadShared::default();
    shared.set_enabled(true);
    let start = Instant::now();
    let mut last = None;
    shared.record(&mut last, start, true);
    shared.record(&mut last, start + Duration::from_millis(2), true);
    shared.record(&mut last, start + Duration::from_millis(12), true);
    let counters = shared.counters();
    assert_eq!(counters.passes, 3);
    assert_eq!(counters.late_gaps, 1, "only the 10 ms gap is late");
    assert_eq!(counters.max_gap_micros, 10_000);
    assert_eq!(shared.take_interval().max_gap_micros, 10_000);
    assert_eq!(shared.take_interval().max_gap_micros, 0);
}

#[test]
fn a_skipped_pass_lengthens_the_gap_but_a_disabled_worker_ends_the_series() {
    let shared = AudioThreadShared::default();
    shared.set_enabled(true);
    let start = Instant::now();
    let mut last = None;
    shared.record(&mut last, start, true);
    // Skipped while enabled (contention): the series continues.
    shared.record(&mut last, start + Duration::from_millis(1), false);
    shared.record(&mut last, start + Duration::from_millis(3), true);
    assert_eq!(shared.counters().max_gap_micros, 3_000);
    // Disabled (stopped route): the next pump starts a new series.
    shared.set_enabled(false);
    shared.record(&mut last, start + Duration::from_millis(4), false);
    assert_eq!(last, None);
    shared.set_enabled(true);
    shared.record(&mut last, start + Duration::from_millis(500), true);
    assert_eq!(shared.counters().late_gaps, 0);
    assert_eq!(shared.counters().max_gap_micros, 3_000);
}

#[test]
fn only_the_owning_handle_takes_the_worker_out() {
    let shared = AudioThreadShared::default();
    shared.lock().owner = 2;
    shared.set_enabled(true);
    // A stale handle (attach number 1) leaves the slot and pumping alone.
    assert!(shared.take_worker(Some(1)).is_none());
    assert!(shared.enabled.load(Ordering::Acquire));
    // The owner (or the plane, with `None`) stops pumping.
    assert!(shared.take_worker(Some(2)).is_none());
    assert!(!shared.enabled.load(Ordering::Acquire));
    shared.set_enabled(true);
    assert!(shared.take_worker(None).is_none());
    assert!(!shared.enabled.load(Ordering::Acquire));
}

#[test]
fn the_audio_thread_starts_reports_its_scheduling_and_stops_on_drop() {
    let shared = Arc::new(AudioThreadShared::default());
    let thread = AudioThread::spawn(shared.clone()).unwrap();
    // The flag is set once the thread has entered its scheduling guard.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && !shared.thread_started() {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(shared.thread_started());
    drop(thread);
    assert!(shared.shutdown.load(Ordering::Acquire));
}

#[test]
fn pump_totals_accumulate_until_reset() {
    // The control thread reports these totals while the audio thread pumps;
    // they used to be hard-coded zeros.
    let shared = AudioThreadShared::default();
    let pass = PumpTotals {
        packets: 2,
        captured_frames: 480,
        processed_quanta: 3,
        delivered_quanta: 6,
        rendered_frames: 384,
        render_backpressure_events: 1,
    };
    shared.pumped.add(pass);
    shared.pumped.add(pass);
    assert_eq!(
        shared.pumped(),
        PumpTotals {
            packets: 4,
            captured_frames: 960,
            processed_quanta: 6,
            delivered_quanta: 12,
            rendered_frames: 768,
            render_backpressure_events: 2,
        }
    );
    shared.pumped.reset();
    assert_eq!(shared.pumped(), PumpTotals::default());
}

#[test]
fn a_lease_bound_deadline_stops_pumping_until_the_checks_run_again() {
    let shared = AudioThreadShared::default();
    assert!(!shared.checks_expired(), "no deadline by default");
    shared.set_pump_valid_for(None);
    assert!(!shared.checks_expired());
    shared.set_pump_valid_for(Some(Duration::ZERO));
    std::thread::sleep(Duration::from_millis(2));
    assert!(shared.checks_expired());
    // The next successful checks renew it.
    shared.set_pump_valid_for(Some(Duration::from_secs(60)));
    assert!(!shared.checks_expired());
}
