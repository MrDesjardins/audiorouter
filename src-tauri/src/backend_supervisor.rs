//! Bounded supervision policy for the shell-owned control backend.
//!
//! This policy only decides whether the control service may be constructed
//! again after its serving loop returns. Reconstructing the control plane
//! intentionally does not start sessions or reopen native endpoints; durable
//! recovery must remain an explicit, validated action.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

const FAILURE_WINDOW: Duration = Duration::from_secs(10 * 60);
const MAX_FAILURES_IN_WINDOW: usize = 3;
const RETRY_BACKOFFS: [Duration; 2] = [Duration::from_millis(100), Duration::from_millis(500)];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendRestartDecision {
    Restart { delay: Duration },
    StopSafeMode,
}

#[derive(Debug, Default)]
pub struct BackendSupervisor {
    failures: VecDeque<Instant>,
}

impl BackendSupervisor {
    pub fn record_failure(&mut self, now: Instant) -> BackendRestartDecision {
        self.failures.retain(|failure| {
            now < *failure || now.saturating_duration_since(*failure) <= FAILURE_WINDOW
        });
        self.failures.push_back(now);
        while self.failures.len() > MAX_FAILURES_IN_WINDOW {
            self.failures.pop_front();
        }
        if self.failures.len() >= MAX_FAILURES_IN_WINDOW {
            BackendRestartDecision::StopSafeMode
        } else {
            BackendRestartDecision::Restart {
                delay: RETRY_BACKOFFS[self.failures.len() - 1],
            }
        }
    }

    pub fn failure_count(&mut self, now: Instant) -> usize {
        self.failures.retain(|failure| {
            now < *failure || now.saturating_duration_since(*failure) <= FAILURE_WINDOW
        });
        self.failures.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_with_bounded_backoff_then_enters_safe_mode() {
        let start = Instant::now();
        let mut supervisor = BackendSupervisor::default();
        assert_eq!(
            supervisor.record_failure(start),
            BackendRestartDecision::Restart {
                delay: Duration::from_millis(100)
            }
        );
        assert_eq!(
            supervisor.record_failure(start + Duration::from_secs(1)),
            BackendRestartDecision::Restart {
                delay: Duration::from_millis(500)
            }
        );
        assert_eq!(
            supervisor.record_failure(start + Duration::from_secs(2)),
            BackendRestartDecision::StopSafeMode
        );
        assert_eq!(supervisor.failure_count(start + Duration::from_secs(2)), 3);
    }

    #[test]
    fn old_failures_expire_before_counting_a_new_restart_window() {
        let start = Instant::now();
        let mut supervisor = BackendSupervisor::default();
        supervisor.record_failure(start);
        supervisor.record_failure(start + Duration::from_secs(1));
        assert_eq!(
            supervisor.record_failure(start + FAILURE_WINDOW + Duration::from_secs(2)),
            BackendRestartDecision::Restart {
                delay: Duration::from_millis(100)
            }
        );
        assert_eq!(
            supervisor.failure_count(start + FAILURE_WINDOW + Duration::from_secs(2)),
            1
        );
    }

    #[test]
    fn future_timestamp_does_not_panic_or_drop_the_marker() {
        let start = Instant::now();
        let mut supervisor = BackendSupervisor::default();
        supervisor.record_failure(start + Duration::from_secs(10));
        assert_eq!(supervisor.failure_count(start), 1);
    }
}
