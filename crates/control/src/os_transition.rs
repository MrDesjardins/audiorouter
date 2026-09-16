//! Bounded policy decisions for operating-system session transitions.
//!
//! The Windows adapter owns notification delivery and endpoint revalidation.
//! This module is deliberately side-effect free: it cannot open audio,
//! restart a process, or substitute an endpoint while deciding what to do.

use audiorouter_domain::EntityId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OsTransition {
    Lock,
    SignOut,
    Sleep,
    Resume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OsTransitionAction {
    KeepRunning,
    StopAndRelease,
    RevalidateBeforeRestart,
    RemainStopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OsTransitionDecision {
    pub action: OsTransitionAction,
    pub session_ids: Vec<EntityId>,
}

/// Decide the safe action for sessions that were explicitly running when the
/// OS transition began. Native sessions are never eligible for automatic
/// resume because endpoint identity and availability must be revalidated by
/// the Windows adapter first. Recorders are never resumed automatically.
pub fn plan_os_transition(
    transition: OsTransition,
    running_sessions: &[EntityId],
    native_sessions: &[EntityId],
    recording_sessions: &[EntityId],
) -> OsTransitionDecision {
    let mut session_ids = running_sessions.to_vec();
    session_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    session_ids.dedup();

    match transition {
        OsTransition::Lock => OsTransitionDecision {
            action: OsTransitionAction::KeepRunning,
            session_ids,
        },
        OsTransition::SignOut | OsTransition::Sleep => OsTransitionDecision {
            action: OsTransitionAction::StopAndRelease,
            session_ids,
        },
        OsTransition::Resume => {
            let native: std::collections::HashSet<_> = native_sessions.iter().collect();
            let recording: std::collections::HashSet<_> = recording_sessions.iter().collect();
            let eligible: Vec<EntityId> = session_ids
                .into_iter()
                .filter(|id| !native.contains(&id) && !recording.contains(&id))
                .collect();
            OsTransitionDecision {
                action: if eligible.is_empty() {
                    OsTransitionAction::RemainStopped
                } else {
                    OsTransitionAction::RevalidateBeforeRestart
                },
                session_ids: eligible,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> Vec<EntityId> {
        values.iter().map(|value| EntityId::new(*value)).collect()
    }

    #[test]
    fn lock_keeps_explicitly_running_sessions_and_is_sorted() {
        let decision = plan_os_transition(OsTransition::Lock, &ids(&["z", "a", "z"]), &[], &[]);
        assert_eq!(decision.action, OsTransitionAction::KeepRunning);
        assert_eq!(decision.session_ids, ids(&["a", "z"]));
    }

    #[test]
    fn sign_out_and_sleep_release_every_running_session() {
        for transition in [OsTransition::SignOut, OsTransition::Sleep] {
            let decision = plan_os_transition(transition, &ids(&["call", "music"]), &[], &[]);
            assert_eq!(decision.action, OsTransitionAction::StopAndRelease);
            assert_eq!(decision.session_ids, ids(&["call", "music"]));
        }
    }

    #[test]
    fn resume_excludes_native_and_recording_sessions() {
        let decision = plan_os_transition(
            OsTransition::Resume,
            &ids(&["native", "recording", "safe"]),
            &ids(&["native"]),
            &ids(&["recording"]),
        );
        assert_eq!(decision.action, OsTransitionAction::RevalidateBeforeRestart);
        assert_eq!(decision.session_ids, ids(&["safe"]));
    }

    #[test]
    fn resume_with_only_protected_sessions_remains_stopped() {
        let decision = plan_os_transition(
            OsTransition::Resume,
            &ids(&["native", "recording"]),
            &ids(&["native"]),
            &ids(&["recording"]),
        );
        assert_eq!(decision.action, OsTransitionAction::RemainStopped);
        assert!(decision.session_ids.is_empty());
    }

    #[test]
    fn repeated_suspend_resume_cycles_remain_bounded_and_fail_closed() {
        let running = ids(&["voice", "desktop"]);
        let native = ids(&["voice"]);

        for _ in 0..100 {
            let stopped = plan_os_transition(OsTransition::Sleep, &running, &native, &[]);
            assert_eq!(stopped.action, OsTransitionAction::StopAndRelease);
            assert_eq!(stopped.session_ids, ids(&["desktop", "voice"]));

            let resumed =
                plan_os_transition(OsTransition::Resume, &stopped.session_ids, &native, &[]);
            assert_eq!(resumed.action, OsTransitionAction::RevalidateBeforeRestart);
            assert_eq!(resumed.session_ids, ids(&["desktop"]));
        }
    }
}
