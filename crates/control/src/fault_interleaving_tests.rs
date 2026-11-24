//! Fault interleavings (quality plan Q4.2), driven through
//! `ControlPlane::dispatch` wherever the user path goes through the API.
//!
//! Real devices and plugin workers cannot be removed or crashed on demand, so
//! the faults are the same markers the production code reads:
//!
//! * a device loss is `arm_device_loss_recovery()`, which is what
//!   `invalidate_changed_native_endpoint_worker` calls after it stops a worker
//!   whose endpoint disappeared (the session marker stands in for the worker);
//! * a plugin worker failure is a dead `Weak` bridge, which
//!   `session_has_failed_plugin` reports exactly as it reports a bridge whose
//!   worker stopped;
//! * time is the `now` handed to `service_running_native_audio`, never a sleep.
//!
//! A device that *returns* cannot be shown here: the recovery's prepare step
//! resolves real endpoints, so with no device chosen it always fails. What is
//! asserted instead is the fail-closed half of recovery: nothing is restarted
//! that the user did not ask for, nothing is substituted, and the privacy mute
//! is never relaxed by a fault. The device-return half stays a hardware gate.

use super::*;
use crate::test_support::*;

/// A control plane with a running-route witness and a fixed-step clock.
struct Rig {
    plane: ControlPlane,
    keys: u32,
    seconds: u64,
    witness: Arc<std::sync::Mutex<Vec<EntityId>>>,
}

impl Rig {
    fn new() -> Self {
        let witness = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut plane = ControlPlane::default();
        plane.set_running_witness(witness.clone());
        Self {
            plane,
            keys: 0,
            seconds: 0,
            witness,
        }
    }

    fn call(&mut self, method: &str, params: Value) -> JsonRpcResponse {
        self.plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(method)),
            method: method.into(),
            params: Some(params),
        })
    }

    /// A mutating call with a fresh idempotency key.
    fn change(&mut self, method: &str, mut params: Value) -> JsonRpcResponse {
        self.keys += 1;
        params["idempotencyKey"] = json!(format!("fault-{}", self.keys));
        self.call(method, params)
    }

    fn ok(&mut self, method: &str, params: Value) -> Value {
        let response = self.change(method, params);
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    }

    fn add(&mut self, session: Session) -> EntityId {
        let id = session.id.clone();
        self.plane.insert_session(session).unwrap();
        id
    }

    fn start(&mut self, id: &EntityId) -> JsonRpcResponse {
        self.change("session.start", json!({ "sessionId": id }))
    }

    fn stop(&mut self, id: &EntityId) -> JsonRpcResponse {
        self.change("session.stop", json!({ "sessionId": id }))
    }

    fn os(&mut self, transition: &str) -> JsonRpcResponse {
        self.change("system.osTransition", json!({ "transition": transition }))
    }

    fn mute(&mut self, muted: bool) {
        let result = self.ok("safety.setPrivacyMute", json!({ "muted": muted }));
        assert_eq!(result["muted"], muted);
    }

    /// Whether `status.get` reports the mute, the way the tray and UI read it.
    fn muted(&mut self) -> bool {
        let status = self.call("status.get", json!({})).result.unwrap();
        status["privacyMute"]["muted"].as_bool().unwrap()
    }

    fn active(&mut self) -> Vec<String> {
        self.plane.status_snapshot().unwrap()["activeSessionIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect()
    }

    fn running(&self, id: &EntityId) -> bool {
        self.plane
            .runtimes
            .get(id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running)
    }

    fn generation(&self, id: &EntityId) -> u64 {
        self.plane
            .runtimes
            .get(id)
            .map_or(0, |runtime| runtime.generation())
    }

    fn witnessed(&self) -> Vec<String> {
        self.witness
            .lock()
            .unwrap()
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect()
    }

    /// The plugin worker of `id` stops (crash, heartbeat timeout, bad output).
    fn fail_plugin(&mut self, id: &EntityId) {
        self.plane.plugin_bridges.lock().unwrap().insert(
            (id.clone(), EntityId::new("voice-plugin")),
            std::sync::Weak::new(),
        );
    }

    /// Preparing a route registers fresh bridges and drops the dead ones of
    /// that session (`PluginStageSet::register`); a deliberate Play does this.
    fn replace_plugin_bridges(&mut self, id: &EntityId) {
        self.plane
            .plugin_bridges
            .lock()
            .unwrap()
            .retain(|(owner, _), _| owner != id);
    }

    fn failed_plugin(&self, id: &EntityId) -> bool {
        self.plane.session_has_failed_plugin(id)
    }

    fn record(&mut self, id: &EntityId) {
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(0).unwrap();
        self.plane.recorders.insert(id.clone(), recorder);
    }

    /// One audio-service pass two clock seconds after the last, which is past
    /// every one-second recovery back-off.
    fn tick(&mut self) {
        self.seconds += 2;
        let now = Instant::now() + Duration::from_secs(self.seconds);
        self.plane.service_running_native_audio(now);
    }

    fn events(&mut self) -> Vec<String> {
        self.plane
            .events
            .since(0, 500)
            .unwrap()
            .into_iter()
            .map(|event| event.category)
            .collect()
    }
}

fn named(mut route: Session, id: &str) -> Session {
    route.id = EntityId::new(id);
    route
}

/// A route that plays through the API with no device (tone to meter).
fn talk(id: &str) -> Session {
    named(session_without_devices(), id)
}

/// The fixture route with real device nodes, as a prepared route would hold.
fn device_route(id: &str) -> Session {
    named(session(), id)
}

fn gain_route(id: &str) -> Session {
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, kind, name: &str, ports| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: name.into(),
        enabled: true,
        bypass: false,
        parameters: if kind == NodeKind::Gain {
            serde_json::from_value(json!({ "gainDb": 0.0 })).unwrap()
        } else {
            Default::default()
        },
        ports,
    };
    let edge = |id: &str, from: &str, to: &str| Edge {
        id: EntityId::new(id),
        source_node: EntityId::new(from),
        source_port: "out".into(),
        destination_node: EntityId::new(to),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    };
    Session {
        id: EntityId::new(id),
        name: "Gain route".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node(
                "tone",
                NodeKind::TestSignal,
                "Tone",
                vec![port("out", PortDirection::Output)],
            ),
            node(
                "voice",
                NodeKind::Gain,
                "Voice gain",
                vec![
                    port("in", PortDirection::Input),
                    port("out", PortDirection::Output),
                ],
            ),
            node(
                "meter",
                NodeKind::Meter,
                "Meter",
                vec![port("in", PortDirection::Input)],
            ),
        ],
        edges: vec![edge("e1", "tone", "voice"), edge("e2", "voice", "meter")],
    }
}

// ---------------------------------------------------------------------------
// Device removed while a session starts
// ---------------------------------------------------------------------------

/// The device vanishes between "prepared" and "running": activation fails. The
/// runtime must roll back completely, so nothing reports playing, and a retry
/// (the device is back, or the user tried again) must work.
#[test]
fn a_start_that_loses_its_device_rolls_back_and_a_retry_plays() {
    let mut rig = Rig::new();
    let id = rig.add(talk("starting"));
    rig.plane.fail_next_session_activation = true;
    let failed = rig.start(&id);
    assert!(failed.error.is_some(), "{failed:?}");
    assert!(!rig.running(&id));
    assert!(rig.active().is_empty());
    assert!(rig.witnessed().is_empty());
    assert!(!rig.events().iter().any(|event| event == "runtime.started"));

    // A same-key retry is not a replay of a failure: failures are not journaled.
    let retried = rig.start(&id);
    assert!(retried.error.is_none(), "{retried:?}");
    assert!(rig.running(&id));
    assert_eq!(rig.witnessed(), vec!["starting".to_owned()]);
}

/// A device route whose device is not open (it was removed before Play, or the
/// route was never prepared) is refused with guidance. It must not start, and
/// it must not pick another device: the saved nodes stay byte-identical.
#[test]
fn a_device_route_with_no_open_device_is_refused_and_keeps_its_saved_devices() {
    let mut rig = Rig::new();
    let id = rig.add(device_route("unprepared"));
    let before = serde_json::to_value(rig.plane.get_session(&id).unwrap()).unwrap();
    for _ in 0..3 {
        let refused = rig.start(&id);
        let error = refused
            .error
            .expect("an unprepared device route is refused");
        assert!(error.message.contains("not prepared"), "{}", error.message);
    }
    assert!(!rig.running(&id));
    assert!(rig.active().is_empty());
    assert_eq!(
        serde_json::to_value(rig.plane.get_session(&id).unwrap()).unwrap(),
        before
    );
}

// ---------------------------------------------------------------------------
// Device removed while running, while stopping, and when it returns
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod device_loss {
    use super::*;

    /// A running device route owned by the multi-path worker, as a device
    /// change would find it. No endpoint is opened.
    fn running_device_route(rig: &mut Rig, id: &str) -> EntityId {
        let id = rig.add(device_route(id));
        rig.plane.session_start(&id).unwrap();
        rig.plane.native_multi_input_worker_session = Some(id.clone());
        id
    }

    fn pending_recovery(rig: &Rig) -> Option<(EntityId, u8, u8, bool)> {
        rig.plane.sleep_route_recovery.as_ref().map(|r| {
            (
                r.session_id.clone(),
                r.attempts,
                r.max_attempts,
                r.stop_first,
            )
        })
    }

    /// Device lost, then the user presses Stop before the first attempt: the
    /// route stays stopped through every later pass.
    #[test]
    fn stop_after_a_device_loss_wins_over_the_pending_recovery() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "lost-then-stopped");
        rig.plane.arm_device_loss_recovery();
        assert!(pending_recovery(&rig).is_some());
        let generation = rig.generation(&id);
        rig.stop(&id);
        assert!(pending_recovery(&rig).is_none());
        for _ in 0..5 {
            rig.tick();
        }
        assert!(!rig.running(&id));
        assert_eq!(rig.generation(&id), generation);
        assert!(!rig
            .events()
            .iter()
            .any(|event| event == "runtime.recovered"));
    }

    /// REGRESSION (Q4.2 defect): the user presses Stop, the worker's stop
    /// reports an error and leaves the worker "running", and the next device
    /// change invalidates it. Recovery used to be armed for the stopped route
    /// and, once the device was back, started it again. Recovery is only armed
    /// for a route that is still playing.
    #[test]
    fn a_device_loss_after_stop_arms_nothing_a_pass_could_restart() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "stopped-then-lost");
        rig.stop(&id);
        let generation = rig.generation(&id);
        rig.plane.arm_device_loss_recovery();
        assert!(
            pending_recovery(&rig).is_none(),
            "a route the user stopped must not be armed for recovery"
        );
        for _ in 0..5 {
            rig.tick();
        }
        assert!(!rig.running(&id));
        assert_eq!(rig.generation(&id), generation);
        assert!(rig.active().is_empty());
        assert!(!rig
            .events()
            .iter()
            .any(|event| event == "runtime.recovered"));
    }

    /// Stop then Start while the device is away: the user's Stop cancels the
    /// recovery, the user's Start is the single owner of the route, and the
    /// device-loss recovery never comes back for it.
    #[test]
    fn a_user_stop_and_start_during_device_recovery_leave_one_owner() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "lost-stop-start");
        rig.plane.arm_device_loss_recovery();
        assert!(pending_recovery(&rig).is_some());
        rig.plane.session_stop(&id).unwrap();
        assert!(pending_recovery(&rig).is_none());
        rig.plane.session_start(&id).unwrap();
        let generation = rig.generation(&id);
        for _ in 0..3 {
            rig.tick();
        }
        assert!(pending_recovery(&rig).is_none());
        assert!(rig.running(&id));
        assert_eq!(rig.generation(&id), generation);
    }

    /// Start pressed on a route whose device just left (the runtime still says
    /// playing, its worker is gone): `session_start` cancels the pending
    /// recovery and, finding the runtime running, starts nothing.
    ///
    /// OWNER DECISION (recorded, not changed): that leaves the route silent
    /// with no recovery pending until the user presses Stop and Play. The
    /// desktop Play prepares the route first, which refuses while the old
    /// worker is attached, so the window is narrow; an API client that calls
    /// `session.start` directly can reach it. Pinned as current behaviour.
    #[test]
    fn a_start_on_a_device_lost_route_cancels_its_recovery_and_starts_nothing() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "lost-then-started");
        rig.plane.arm_device_loss_recovery();
        let generation = rig.generation(&id);
        rig.plane.session_start(&id).unwrap();
        assert!(pending_recovery(&rig).is_none());
        for _ in 0..3 {
            rig.tick();
        }
        assert!(rig.running(&id));
        assert_eq!(rig.generation(&id), generation, "no second start happened");
        assert!(!rig
            .events()
            .iter()
            .any(|event| event == "runtime.recovered"));
    }

    /// A device that never returns: the route is silenced on the first pass,
    /// every attempt fails closed, the saved route is never changed, and the
    /// attempts are bounded.
    #[test]
    fn a_device_that_never_returns_silences_then_gives_up_without_changing_the_route() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "never-returns");
        let before = serde_json::to_value(rig.plane.get_session(&id).unwrap()).unwrap();
        rig.plane.arm_device_loss_recovery();
        let (_, _, max, stop_first) = pending_recovery(&rig).unwrap();
        assert!(stop_first);
        assert_eq!(max, DEVICE_RECOVERY_ATTEMPTS);
        rig.tick();
        // The interrupted route is silenced on the first pass, not left
        // "playing" while its worker is gone.
        assert!(!rig.running(&id));
        assert!(rig.witnessed().is_empty());
        for _ in 0..usize::from(DEVICE_RECOVERY_ATTEMPTS) + 3 {
            rig.tick();
            assert!(!rig.running(&id));
        }
        assert!(pending_recovery(&rig).is_none());
        let events = rig.events();
        assert_eq!(
            events
                .iter()
                .filter(|event| *event == "runtime.recoveryFailed")
                .count(),
            1
        );
        assert!(!events.iter().any(|event| event == "runtime.recovered"));
        assert_eq!(
            serde_json::to_value(rig.plane.get_session(&id).unwrap()).unwrap(),
            before,
            "no device was substituted into the saved route"
        );
    }

    /// Device lost during a recording: recovery is never armed for it, the
    /// recording and the route stay as they were, and Stop then finalizes.
    #[test]
    fn a_device_loss_during_a_recording_never_restarts_or_ends_it() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "recording-loses-device");
        rig.record(&id);
        rig.plane.arm_device_loss_recovery();
        assert!(pending_recovery(&rig).is_none());
        for _ in 0..3 {
            rig.tick();
        }
        assert!(rig.running(&id));
        assert_eq!(rig.plane.recorders[&id].state(), RecorderState::Recording);
        // The recording route is not offered to a crash restore either.
        assert!(rig.witnessed().is_empty());
        let stopped = rig.stop(&id);
        assert!(stopped.error.is_none(), "{stopped:?}");
        assert!(!rig.running(&id));
        assert_ne!(rig.plane.recorders[&id].state(), RecorderState::Recording);
    }

    /// The recording starts after the device was lost and recovery is armed.
    /// The next pass must drop the recovery rather than restart the route
    /// under a recording.
    ///
    /// OWNER DECISION (recorded, not changed): the dropped recovery leaves the
    /// runtime marked running although its worker stopped with the device. The
    /// UI is told by `devices.bindingInvalidated`; stopping here would
    /// interact with recorder finalization, which needs the owner's call.
    #[test]
    fn a_recording_started_after_a_device_loss_cancels_the_recovery() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "record-after-loss");
        rig.plane.arm_device_loss_recovery();
        rig.record(&id);
        let generation = rig.generation(&id);
        rig.tick();
        assert!(pending_recovery(&rig).is_none());
        assert_eq!(
            rig.generation(&id),
            generation,
            "the route was not restarted"
        );
        assert_eq!(rig.plane.recorders[&id].state(), RecorderState::Recording);
        assert!(!rig
            .events()
            .iter()
            .any(|event| event == "runtime.recovered"));
    }

    /// The user edits the route while a device recovery is pending. The
    /// recovery was armed for the saved revision, so it must not apply to the
    /// new one: it is dropped, and never restarts the edited route.
    ///
    /// OWNER DECISION (recorded, not changed): as above, the dropped recovery
    /// leaves the runtime marked running until the user presses Stop or Play.
    #[test]
    fn a_graph_commit_during_device_recovery_cancels_the_stale_recovery() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "commit-during-recovery");
        rig.plane.arm_device_loss_recovery();
        let mut candidate = rig.plane.get_session(&id).unwrap().clone();
        candidate.name = "edited while the device was away".into();
        let plan = rig
            .call(
                "graph.plan",
                json!({ "sessionId": id, "baseRevision": 0, "candidate": candidate }),
            )
            .result
            .unwrap();
        rig.ok(
            "graph.commit",
            json!({ "planId": plan["planId"], "baseRevision": 0 }),
        );
        assert_eq!(rig.plane.get_session(&id).unwrap().revision, 1);
        let generation = rig.generation(&id);
        rig.tick();
        assert!(pending_recovery(&rig).is_none());
        assert_eq!(rig.generation(&id), generation);
        assert!(!rig
            .events()
            .iter()
            .any(|event| event == "runtime.recovered"));
        // The edit is saved exactly once, with the new name.
        assert_eq!(
            rig.plane.get_session(&id).unwrap().name,
            "edited while the device was away"
        );
    }

    /// A plugin worker fails after the device loss armed recovery. The failed
    /// processor stays silent: recovery is dropped and does not restart it.
    #[test]
    fn a_plugin_failure_after_a_device_loss_cancels_the_recovery() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "plugin-after-loss");
        rig.plane.arm_device_loss_recovery();
        rig.fail_plugin(&id);
        let generation = rig.generation(&id);
        rig.tick();
        assert!(pending_recovery(&rig).is_none());
        assert_eq!(rig.generation(&id), generation);
        assert!(rig.witnessed().is_empty());
        assert!(rig.failed_plugin(&id));
        assert!(!rig
            .events()
            .iter()
            .any(|event| event == "runtime.recovered"));
    }

    /// The device returns in the same pass in which the plugin was already
    /// failed: nothing is armed in the first place.
    #[test]
    fn a_device_loss_on_a_route_with_a_failed_plugin_arms_nothing() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "plugin-then-loss");
        rig.fail_plugin(&id);
        rig.plane.arm_device_loss_recovery();
        assert!(pending_recovery(&rig).is_none());
        for _ in 0..3 {
            rig.tick();
        }
        assert!(rig.failed_plugin(&id));
        assert!(rig.witnessed().is_empty());
    }

    // -----------------------------------------------------------------------
    // Privacy mute against faults
    // -----------------------------------------------------------------------

    /// The mute is toggled while a device recovery is pending and keeps its
    /// last requested value through every failed attempt and the final give-up.
    #[test]
    fn the_mute_is_never_relaxed_by_a_device_loss_or_its_recovery() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "mute-and-loss");
        rig.mute(true);
        rig.plane.arm_device_loss_recovery();
        for step in 0..usize::from(DEVICE_RECOVERY_ATTEMPTS) + 3 {
            rig.tick();
            assert!(rig.muted(), "muted after attempt {step}");
            assert!(rig.plane.privacy_muted);
        }
        // The user unmutes in the middle of a second loss: that, and only
        // that, changes the state.
        rig.plane.session_start(&id).unwrap();
        rig.plane.arm_device_loss_recovery();
        rig.mute(false);
        rig.tick();
        assert!(!rig.muted());
        rig.mute(true);
        rig.tick();
        assert!(rig.muted());
    }

    /// The mute toggled in the same instant that the device is lost, in both
    /// orders, leaves the same final state and the same single recovery.
    #[test]
    fn mute_and_device_loss_commute() {
        for mute_first in [true, false] {
            let mut rig = Rig::new();
            let id = running_device_route(&mut rig, "commute");
            if mute_first {
                rig.mute(true);
                rig.plane.arm_device_loss_recovery();
            } else {
                rig.plane.arm_device_loss_recovery();
                rig.mute(true);
            }
            assert!(rig.muted());
            assert_eq!(pending_recovery(&rig).unwrap().0, id);
            rig.tick();
            assert!(rig.muted());
            assert!(!rig.running(&id), "silenced on the first pass either way");
        }
    }

    // -----------------------------------------------------------------------
    // Sleep and resume, interleaved with start and stop
    // -----------------------------------------------------------------------

    /// Sleep arms a bounded restart of the exact saved route; Stop after the
    /// sleep cancels it, and the later resume has nothing to restart.
    #[test]
    fn stop_between_sleep_and_resume_cancels_the_restart() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "sleep-stop-resume");
        assert!(rig.os("sleep").error.is_none());
        assert!(pending_recovery(&rig).is_some());
        rig.stop(&id);
        assert!(pending_recovery(&rig).is_none());
        let resumed = rig.os("resume").result.unwrap();
        assert_eq!(resumed["sessionIds"], json!([]));
        for _ in 0..3 {
            rig.tick();
        }
        assert!(!rig.running(&id));
    }

    /// The user presses Play between sleep and resume (the machine woke up and
    /// the user was quicker than the resume notification): the resume must not
    /// report that route as waiting, and nothing restarts it a second time.
    #[test]
    fn start_between_sleep_and_resume_is_not_started_twice() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "sleep-start-resume");
        rig.os("sleep");
        rig.plane.session_start(&id).unwrap();
        assert!(pending_recovery(&rig).is_none());
        let generation = rig.generation(&id);
        let resumed = rig.os("resume").result.unwrap();
        assert_eq!(resumed["sessionIds"], json!([]));
        for _ in 0..3 {
            rig.tick();
        }
        assert!(rig.running(&id));
        assert_eq!(rig.generation(&id), generation);
    }

    /// Sleep, then the plugin worker fails (it does not survive the suspend),
    /// then resume: the failed processor is not restarted by the OS transition.
    #[test]
    fn a_plugin_that_fails_across_a_sleep_is_not_restarted_by_resume() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "sleep-plugin-resume");
        rig.os("sleep");
        assert!(pending_recovery(&rig).is_some());
        rig.fail_plugin(&id);
        rig.os("resume");
        let generation = rig.generation(&id);
        for _ in 0..3 {
            rig.tick();
        }
        assert!(!rig.running(&id));
        assert_eq!(rig.generation(&id), generation);
        assert!(pending_recovery(&rig).is_none());
        assert!(rig.witnessed().is_empty());
    }

    /// A device loss while a sleep restart is pending (the device is slow to
    /// wake) must not shorten or duplicate the recovery, and a second suspend
    /// pauses rather than discards it.
    #[test]
    fn a_second_suspend_pauses_the_pending_recovery_instead_of_dropping_it() {
        let mut rig = Rig::new();
        let id = running_device_route(&mut rig, "suspend-twice");
        rig.plane.arm_device_loss_recovery();
        assert!(pending_recovery(&rig).is_some());
        rig.os("sleep");
        let paused = rig.plane.sleep_route_recovery.as_ref().unwrap();
        assert!(paused.next_attempt.is_none(), "paused while suspended");
        for _ in 0..3 {
            rig.tick();
        }
        assert_eq!(
            pending_recovery(&rig).unwrap().1,
            0,
            "no attempt runs while the machine is suspended"
        );
        rig.os("resume");
        rig.tick();
        assert_eq!(pending_recovery(&rig).map(|r| r.1), Some(1));
        assert_eq!(pending_recovery(&rig).unwrap().0, id);
    }

    /// A sleep with a recording is refused: no route is stopped, and the
    /// recording route keeps playing through the failed transition and a
    /// resume that follows it.
    #[test]
    fn sleep_with_a_recording_changes_nothing_and_resume_after_it_restarts_nothing() {
        let mut rig = Rig::new();
        let recording = running_device_route(&mut rig, "sleep-recording");
        let other = rig.add(talk("sleep-bystander"));
        rig.start(&other);
        rig.record(&recording);
        let refused = rig.os("sleep");
        assert!(refused.error.is_some(), "{refused:?}");
        assert!(rig.running(&recording) && rig.running(&other));
        assert!(pending_recovery(&rig).is_none());
        let resumed = rig.os("resume").result.unwrap();
        assert_eq!(resumed["sessionIds"], json!([]));
        assert_eq!(resumed["nativeSessionIds"], json!([]));
        assert!(rig.running(&recording) && rig.running(&other));
        assert_eq!(
            rig.plane.recorders[&recording].state(),
            RecorderState::Recording
        );
    }

    /// Two sleeps without a resume between them (a missed notification). The
    /// second sees nothing running and replaces the first sleep's resume list
    /// with an empty one; pinned as the current, fail-closed behaviour.
    ///
    /// OWNER DECISION (recorded, not changed): the first sleep's list is
    /// forgotten, so Resume offers nothing to restart although the first sleep
    /// stopped a route. Nothing restarts silently, but a merged list would be
    /// kinder. The route's own recovery is unaffected (it is kept paused).
    #[test]
    fn a_second_sleep_forgets_the_first_sleeps_resume_list() {
        let mut rig = Rig::new();
        let id = rig.add(talk("two-sleeps"));
        rig.start(&id);
        rig.os("sleep");
        assert!(!rig.running(&id));
        rig.os("sleep");
        let resumed = rig.os("resume").result.unwrap();
        assert_eq!(resumed["sessionIds"], json!([]));
        assert!(!rig.running(&id));
    }
}

// ---------------------------------------------------------------------------
// Delayed or duplicated OS transitions (portable)
// ---------------------------------------------------------------------------

/// The shell retries a transition whose reply it never saw. A late duplicate
/// "sleep" arrives after the machine resumed and the user pressed Play: the
/// idempotent replay returns the original reply and must not stop the route.
#[test]
fn a_delayed_duplicate_sleep_does_not_stop_a_route_started_after_resume() {
    let mut rig = Rig::new();
    let id = rig.add(talk("duplicate-sleep"));
    rig.start(&id);
    let sleep = || JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!("sleep")),
        method: "system.osTransition".into(),
        params: Some(json!({ "transition": "sleep", "idempotencyKey": "shell-sleep-1" })),
    };
    let first = rig.plane.dispatch(sleep()).result.unwrap();
    assert_eq!(first["action"], "stopAndRelease");
    assert!(!rig.running(&id));
    rig.os("resume");
    rig.start(&id);
    assert!(rig.running(&id));
    let generation = rig.generation(&id);

    let replay = rig.plane.dispatch(sleep());
    assert_eq!(replay.result, Some(first));
    assert!(rig.running(&id), "the replay did not stop the new run");
    assert_eq!(rig.generation(&id), generation);

    // The same key for a different transition is a conflict, not a stop.
    let conflict = rig.plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!("conflict")),
        method: "system.osTransition".into(),
        params: Some(json!({ "transition": "signOut", "idempotencyKey": "shell-sleep-1" })),
    });
    assert!(conflict.error.is_some());
    assert!(rig.running(&id));
}

/// Lock keeps routes playing and is not a reason to stop, restart or forget
/// anything, even between a sleep and its resume.
#[test]
fn lock_between_sleep_and_resume_does_not_disturb_the_resume_list() {
    let mut rig = Rig::new();
    let id = rig.add(talk("lock-in-the-middle"));
    rig.start(&id);
    rig.os("sleep");
    let locked = rig.os("lock").result.unwrap();
    assert_eq!(locked["action"], "keepRunning");
    let resumed = rig.os("resume").result.unwrap();
    assert_eq!(resumed["action"], "revalidateBeforeRestart");
    assert_eq!(resumed["sessionIds"], json!(["lock-in-the-middle"]));
    assert!(!rig.running(&id), "resume never restarts by itself");
}

/// Sleep refuses while a session records, and the refusal is total: a
/// bystander route that could have been stopped keeps playing, so a half-done
/// transition cannot leave one route silenced and another released.
#[test]
fn a_refused_sleep_leaves_every_route_exactly_as_it_was() {
    let mut rig = Rig::new();
    let recording = rig.add(talk("refuse-recording"));
    let bystander = rig.add(talk("refuse-bystander"));
    rig.start(&recording);
    rig.start(&bystander);
    rig.record(&recording);
    let generations = (rig.generation(&recording), rig.generation(&bystander));
    for transition in ["sleep", "signOut"] {
        let refused = rig.os(transition);
        assert!(refused.error.is_some(), "{transition}: {refused:?}");
        assert_eq!(
            (rig.generation(&recording), rig.generation(&bystander)),
            generations
        );
        assert!(rig.running(&recording) && rig.running(&bystander));
    }
    // Stop finalizes the recording; only then can the machine sleep.
    rig.stop(&recording);
    let slept = rig.os("sleep");
    assert!(slept.error.is_none(), "{slept:?}");
    assert!(!rig.running(&bystander));
}

// ---------------------------------------------------------------------------
// Privacy mute with a failed plugin on a protected voice path (portable)
// ---------------------------------------------------------------------------

/// A failed processor stays silent until a deliberate Stop and Play. Neither
/// the mute going on, the mute going off, a sleep, a resume, nor a lock may
/// bring it back, and the mute reads the same value from every surface.
#[test]
fn a_failed_voice_plugin_stays_silent_through_mute_sleep_and_resume() {
    let mut rig = Rig::new();
    let id = rig.add(talk("protected-voice"));
    rig.start(&id);
    assert_eq!(rig.witnessed(), vec!["protected-voice".to_owned()]);
    rig.fail_plugin(&id);
    rig.tick();
    assert!(rig.failed_plugin(&id));
    assert!(
        rig.witnessed().is_empty(),
        "a failed route is never restored"
    );

    rig.mute(true);
    assert!(rig.muted());
    rig.mute(false);
    assert!(!rig.muted());
    rig.mute(true);
    for transition in ["lock", "sleep", "resume"] {
        rig.os(transition);
        assert!(rig.failed_plugin(&id), "{transition}");
        assert!(rig.witnessed().is_empty(), "{transition}");
        assert!(rig.muted(), "{transition} kept the mute");
    }
    rig.tick();
    assert!(rig.witnessed().is_empty());
    rig.mute(false);
    assert!(rig.failed_plugin(&id), "unmuting is not a recovery");
    assert!(rig.witnessed().is_empty());
}

/// Deliberate recovery is Stop then Play: the new run registers fresh bridges,
/// the failure marker clears, and the route is offered for restore again.
#[test]
fn stop_and_play_is_the_only_recovery_of_a_failed_voice_plugin() {
    let mut rig = Rig::new();
    let id = rig.add(talk("deliberate-recovery"));
    rig.start(&id);
    rig.fail_plugin(&id);
    let resume = rig.change(
        "plugins.resume",
        json!({ "sessionId": id, "nodeId": "voice-plugin" }),
    );
    assert!(
        resume.error.is_some(),
        "a stopped worker cannot be resumed in place: {resume:?}"
    );
    assert!(rig.failed_plugin(&id));
    rig.stop(&id);
    rig.replace_plugin_bridges(&id);
    assert!(rig.start(&id).error.is_none());
    assert!(!rig.failed_plugin(&id));
    rig.tick();
    assert_eq!(rig.witnessed(), vec!["deliberate-recovery".to_owned()]);
}

/// The mute is a property of the backend, not of any route: a route that is
/// started, stopped, failed and restarted in any order never changes it, and a
/// mute set while no route exists applies to the next one (nothing re-reads a
/// microphone setting that could silently pick another input).
#[test]
fn the_mute_outlives_every_route_event() {
    let mut rig = Rig::new();
    rig.mute(true);
    let id = rig.add(talk("mute-first"));
    rig.start(&id);
    assert!(rig.muted());
    rig.fail_plugin(&id);
    rig.stop(&id);
    rig.start(&id);
    rig.os("sleep");
    rig.os("resume");
    assert!(rig.muted());
    assert!(rig.plane.privacy_muted);
    // The toggle tool flips relative to the current value, once per key.
    let toggled = rig.ok("safety.togglePrivacyMute", json!({}));
    assert_eq!(toggled["muted"], false);
    assert!(!rig.muted());
}

// ---------------------------------------------------------------------------
// Plugin worker failure during a graph commit or a live parameter change
// ---------------------------------------------------------------------------

/// The worker dies between `graph.plan` and `graph.commit`. The commit is
/// atomic and applies once at the planned revision, the failure is not
/// forgotten by it, and a stale second plan is rejected without a partial save.
#[test]
fn a_plugin_failure_between_plan_and_commit_does_not_split_the_commit() {
    let mut rig = Rig::new();
    let id = rig.add(gain_route("commit-race"));
    rig.start(&id);
    let planned = |rig: &mut Rig, name: &str| {
        let mut candidate = rig.plane.get_session(&id).unwrap().clone();
        candidate.name = name.into();
        let revision = candidate.revision;
        let plan = rig
            .call(
                "graph.plan",
                json!({ "sessionId": id, "baseRevision": revision, "candidate": candidate }),
            )
            .result
            .unwrap();
        (plan["planId"].clone(), revision)
    };
    let (first_plan, revision) = planned(&mut rig, "first");
    let (second_plan, _) = planned(&mut rig, "second");
    rig.fail_plugin(&id);
    rig.ok(
        "graph.commit",
        json!({ "planId": first_plan, "baseRevision": revision }),
    );
    assert_eq!(rig.plane.get_session(&id).unwrap().name, "first");
    assert_eq!(rig.plane.get_session(&id).unwrap().revision, revision + 1);
    assert!(rig.failed_plugin(&id));

    let stale = rig.change(
        "graph.commit",
        json!({ "planId": second_plan, "baseRevision": revision }),
    );
    let error = stale.error.expect("the stale plan is refused");
    assert_eq!(error.data.unwrap()["code"], "revisionConflict");
    assert_eq!(rig.plane.get_session(&id).unwrap().name, "first");
    assert_eq!(rig.plane.get_session(&id).unwrap().revision, revision + 1);
    assert!(
        rig.running(&id),
        "an edit does not stop or restart the route"
    );
    assert!(rig.failed_plugin(&id));
    assert!(rig.witnessed().is_empty());
}

/// A live parameter change after the worker died is saved once (the route is
/// still the user's), is idempotent on retry, and neither revives the failed
/// processor nor turns the failure into a restart.
#[test]
fn a_live_parameter_change_after_a_plugin_failure_saves_but_does_not_revive() {
    let mut rig = Rig::new();
    let id = rig.add(gain_route("live-change"));
    rig.start(&id);
    rig.fail_plugin(&id);
    let generation = rig.generation(&id);
    let set = |gain: f64| {
        json!({ "sessionId": "live-change", "node": "Voice gain",
                "parameters": { "gainDb": gain }, "idempotencyKey": "live-gain-1" })
    };
    let first = rig.call("nodes.set", set(-6.0));
    assert!(first.error.is_none(), "{first:?}");
    assert_eq!(rig.plane.get_session(&id).unwrap().revision, 1);
    // A retry of the same request (the reply was lost) is a replay.
    let replay = rig.call("nodes.set", set(-6.0));
    assert_eq!(replay.result, first.result);
    assert_eq!(rig.plane.get_session(&id).unwrap().revision, 1);
    // Same key, different value: refused, nothing changes.
    let changed = rig.call("nodes.set", set(-12.0));
    assert!(changed.error.is_some());
    assert_eq!(
        rig.plane.get_session(&id).unwrap().nodes[1].parameters["gainDb"],
        json!(-6.0)
    );
    assert!(rig.failed_plugin(&id));
    // A live edit publishes the new graph as the next generation of the same
    // run; it is not a restart, so the route is still the one that was playing.
    assert!(rig.running(&id));
    assert!(rig.generation(&id) >= generation);
    assert!(rig.witnessed().is_empty());
}

// ---------------------------------------------------------------------------
// A seeded interleaving scheduler
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod seeded {
    use super::*;

    /// Small deterministic generator (xorshift64*), so a failing seed replays
    /// the same sequence on every machine.
    struct Seed(u64);

    impl Seed {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, bound: u64) -> u64 {
            self.next() % bound
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Op {
        StartDevice,
        StopDevice,
        StartTalk,
        StopTalk,
        Sleep,
        Resume,
        Lock,
        Mute(bool),
        FailDevicePlugin,
        FailTalkPlugin,
        LoseDevice,
        Tick,
        Rename,
        RecordTalk,
        RecordDevice,
    }

    const OPS: [Op; 16] = [
        Op::StartDevice,
        Op::StopDevice,
        Op::StartTalk,
        Op::StopTalk,
        Op::Sleep,
        Op::Resume,
        Op::Lock,
        Op::Mute(true),
        Op::Mute(false),
        Op::FailDevicePlugin,
        Op::FailTalkPlugin,
        Op::LoseDevice,
        Op::Tick,
        Op::Tick,
        Op::RecordTalk,
        Op::RecordDevice,
    ];

    /// What the user did, independent of what the backend reports.
    struct Model {
        muted: bool,
        renames: u64,
        device_recording: bool,
        talk_recording: bool,
    }

    fn run(seed: u64, steps: usize) {
        let mut random = Seed(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut rig = Rig::new();
        let device = rig.add(device_route("device"));
        let talk_id = rig.add(talk("talk"));
        rig.plane.native_multi_input_worker_session = Some(device.clone());
        let saved_devices =
            serde_json::to_value(&rig.plane.get_session(&device).unwrap().nodes).unwrap();
        let mut model = Model {
            muted: false,
            renames: 0,
            device_recording: false,
            talk_recording: false,
        };
        let mut trace = Vec::new();
        for step in 0..steps {
            let op = if step == 0 {
                Op::StartTalk
            } else {
                // Also allow a rename, which is not in the dense list.
                let pick = random.below(OPS.len() as u64 + 1) as usize;
                OPS.get(pick).copied().unwrap_or(Op::Rename)
            };
            trace.push(op);
            let context = || format!("seed {seed}, step {step}, trace {trace:?}");
            let before = [&device, &talk_id].map(|id| (rig.running(id), rig.generation(id)));
            let any_recording = model.device_recording || model.talk_recording;
            let mut started = [false, false];
            let mut expected_refusal = false;
            match op {
                Op::StartDevice => {
                    // A device route is started by the same call Play makes
                    // after preparing it (the API refuses an unprepared one).
                    rig.plane.session_start(&device).unwrap();
                    rig.replace_plugin_bridges(&device);
                    started[0] = true;
                }
                Op::StopDevice => {
                    assert!(rig.stop(&device).error.is_none(), "{}", context());
                    model.device_recording = false;
                }
                Op::StartTalk => {
                    rig.replace_plugin_bridges(&talk_id);
                    assert!(rig.start(&talk_id).error.is_none(), "{}", context());
                    started[1] = true;
                }
                Op::StopTalk => {
                    assert!(rig.stop(&talk_id).error.is_none(), "{}", context());
                    model.talk_recording = false;
                }
                Op::Sleep => {
                    let response = rig.os("sleep");
                    expected_refusal = any_recording;
                    assert_eq!(response.error.is_some(), expected_refusal, "{}", context());
                }
                Op::Resume => {
                    let result = rig.os("resume").result.unwrap();
                    for id in result["sessionIds"].as_array().unwrap() {
                        let id = EntityId::new(id.as_str().unwrap());
                        assert!(
                            !rig.running(&id),
                            "resume offered a playing route: {}",
                            context()
                        );
                    }
                }
                Op::Lock => {
                    assert!(rig.os("lock").error.is_none(), "{}", context());
                }
                Op::Mute(muted) => {
                    rig.mute(muted);
                    model.muted = muted;
                }
                Op::FailDevicePlugin => rig.fail_plugin(&device),
                Op::FailTalkPlugin => rig.fail_plugin(&talk_id),
                Op::LoseDevice => rig.plane.arm_device_loss_recovery(),
                Op::Tick => rig.tick(),
                Op::Rename => {
                    let mut candidate = rig.plane.get_session(&device).unwrap().clone();
                    let revision = candidate.revision;
                    candidate.name = format!("edit {}", model.renames + 1);
                    let plan = rig
                        .call(
                            "graph.plan",
                            json!({ "sessionId": device, "baseRevision": revision,
                                    "candidate": candidate }),
                        )
                        .result
                        .unwrap();
                    rig.ok(
                        "graph.commit",
                        json!({ "planId": plan["planId"], "baseRevision": revision }),
                    );
                    model.renames += 1;
                }
                Op::RecordTalk => {
                    if rig.running(&talk_id) && !model.talk_recording {
                        rig.record(&talk_id);
                        model.talk_recording = true;
                    }
                }
                Op::RecordDevice => {
                    if rig.running(&device) && !model.device_recording {
                        rig.record(&device);
                        model.device_recording = true;
                    }
                }
            }

            // The mute is exactly what the user last asked for, on every
            // surface, whatever the faults did.
            assert_eq!(rig.muted(), model.muted, "mute drifted: {}", context());
            assert_eq!(rig.plane.privacy_muted, model.muted, "{}", context());

            // Nothing is started that the user did not start, and nothing
            // that was stopped comes back (a recovery that cannot reach a
            // device must never leave a route half-restarted).
            let after = [&device, &talk_id].map(|id| (rig.running(id), rig.generation(id)));
            for index in 0..2 {
                if !started[index] {
                    assert!(
                        !after[index].0 || before[index].0,
                        "unrequested start of route {index}: {}",
                        context()
                    );
                    // A live edit of a playing route publishes its next
                    // generation; every other event leaves it alone.
                    if !matches!(op, Op::Rename) {
                        assert_eq!(after[index].1, before[index].1, "generation: {}", context());
                    }
                }
            }
            if expected_refusal {
                assert_eq!(
                    before,
                    after,
                    "a refused transition changed a route: {}",
                    context()
                );
            }

            // The saved devices are never edited by a fault.
            assert_eq!(
                serde_json::to_value(&rig.plane.get_session(&device).unwrap().nodes).unwrap(),
                saved_devices,
                "a device was substituted: {}",
                context()
            );
            assert_eq!(
                rig.plane.get_session(&device).unwrap().revision,
                model.renames,
                "revision: {}",
                context()
            );

            // The witness never lists a route a restore must not touch.
            rig.plane.publish_running_witness();
            for id in rig.witnessed() {
                let id = EntityId::new(id);
                assert!(
                    rig.running(&id),
                    "witness lists a stopped route: {}",
                    context()
                );
                assert!(
                    !rig.failed_plugin(&id),
                    "witness lists a failed plugin: {}",
                    context()
                );
                assert!(
                    !rig.plane.session_is_recording(&id),
                    "witness lists a recording: {}",
                    context()
                );
            }

            // A pending recovery is bounded, for the saved route only, and
            // after a pass never for a recording or failed route.
            if let Some(recovery) = rig.plane.sleep_route_recovery.as_ref() {
                assert_eq!(recovery.session_id, device, "{}", context());
                assert!(recovery.attempts <= recovery.max_attempts, "{}", context());
                // A recovery paused by a suspend waits for the resume, which
                // re-checks everything; only an active one has been examined.
                if matches!(op, Op::Tick) && recovery.next_attempt.is_some() {
                    assert!(!rig.failed_plugin(&device), "{}", context());
                    assert!(!rig.plane.session_is_recording(&device), "{}", context());
                    assert_eq!(
                        recovery.revision,
                        rig.plane.get_session(&device).unwrap().revision,
                        "stale recovery survived a pass: {}",
                        context()
                    );
                }
            }
        }
        // Whatever happened, a final stop of everything leaves nothing playing.
        for id in [&device, &talk_id] {
            rig.stop(id);
        }
        assert!(rig.active().is_empty(), "seed {seed}: {trace:?}");
        assert!(rig.plane.sleep_route_recovery.is_none(), "seed {seed}");
    }

    #[test]
    fn fixed_seeds_keep_every_interleaving_fail_closed() {
        // Pull requests run a fixed, fast set. A nightly or a local soak can
        // widen it with `AUDIOROUTER_FAULT_SEEDS=<count>`; a failure names its
        // seed and full trace, so it replays exactly.
        let seeds = std::env::var("AUDIOROUTER_FAULT_SEEDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map_or(120, |count| count.clamp(1, 100_000));
        for seed in 1..=seeds {
            run(seed, 40);
        }
    }
}
