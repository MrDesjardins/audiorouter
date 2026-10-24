//! The application inventory (`apps.list`), built on its own thread.
//!
//! Enumerating processes and their audio sessions takes 30-100 ms on a real
//! PC. On the control thread every other request waited behind it, so the
//! backend service builds the list here instead and `apps.list` answers at
//! once with the latest one, asking for a newer one when it has aged.

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::ControlError;

/// A list older than this is still returned, and a newer one is requested.
const REFRESH_AFTER: Duration = Duration::from_millis(500);
/// After failed refreshes, the last list is returned only while it is this
/// recent; then the failure itself is, so a list never silently goes stale.
const STALE_LIMIT: Duration = Duration::from_secs(30);
/// How long the first `apps.list` waits for the first list. The thread starts
/// with the backend, so the list is normally ready long before the UI asks.
const FIRST_LIST_WAIT: Duration = Duration::from_secs(1);

pub(crate) type Enumerate = Arc<dyn Fn() -> Result<Value, ControlError> + Send + Sync>;

#[derive(Default)]
struct State {
    list: Option<(Instant, Value)>,
    last_error: Option<ControlError>,
    attempts: u64,
    refresh: bool,
    stopping: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

impl Shared {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The running inventory thread. Dropping it stops and joins the thread; an
/// enumeration in progress finishes first (bounded by the OS calls).
pub(crate) struct AppInventory {
    shared: Arc<Shared>,
    thread: Option<std::thread::JoinHandle<()>>,
    stale_limit: Duration,
}

impl AppInventory {
    pub(crate) fn spawn(enumerate: Enumerate) -> std::io::Result<Self> {
        Self::spawn_with(enumerate, STALE_LIMIT)
    }

    pub(crate) fn spawn_with(enumerate: Enumerate, stale_limit: Duration) -> std::io::Result<Self> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                // Build the first list at once.
                refresh: true,
                ..State::default()
            }),
            wake: Condvar::new(),
        });
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("audiorouter-apps".into())
            .spawn(move || run(&thread_shared, &enumerate))?;
        Ok(Self {
            shared,
            thread: Some(thread),
            stale_limit,
        })
    }

    /// The latest list, without enumerating on the caller's thread.
    pub(crate) fn list(&self) -> Result<Value, ControlError> {
        let state = self.shared.state();
        // Only the very first request can find no answer yet.
        let (mut state, _) = self
            .shared
            .wake
            .wait_timeout_while(state, FIRST_LIST_WAIT, |state| {
                state.list.is_none() && state.attempts == 0
            })
            .unwrap_or_else(PoisonError::into_inner);
        let age = state.list.as_ref().map(|(at, _)| at.elapsed());
        if age.is_none_or(|age| age >= REFRESH_AFTER) && !state.refresh {
            state.refresh = true;
            self.shared.wake.notify_all();
        }
        match (&state.list, &state.last_error) {
            (Some((_, list)), None) => Ok(list.clone()),
            (Some((at, list)), Some(_)) if at.elapsed() < self.stale_limit => Ok(list.clone()),
            (_, Some(error)) => Err(error.clone()),
            (None, None) => Err(ControlError::Audio {
                code: "serviceUnavailable",
                operation: None,
                hresult: 0,
                retryable: true,
                remediation: "Try again in a moment.",
                message: "the application list is still loading".into(),
                resource_ids: Vec::new(),
            }),
        }
    }
}

impl Drop for AppInventory {
    fn drop(&mut self) {
        self.shared.state().stopping = true;
        self.shared.wake.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(shared: &Shared, enumerate: &Enumerate) {
    loop {
        {
            let state = shared.state();
            let mut state = shared
                .wake
                .wait_while(state, |state| !state.refresh && !state.stopping)
                .unwrap_or_else(PoisonError::into_inner);
            if state.stopping {
                return;
            }
            state.refresh = false;
        }
        let result = enumerate();
        let mut state = shared.state();
        state.attempts += 1;
        match result {
            Ok(list) => {
                state.list = Some((Instant::now(), list));
                state.last_error = None;
            }
            // Keep the last list; `list` decides how long it may be served.
            Err(error) => state.last_error = Some(error),
        }
        shared.wake.notify_all();
    }
}

#[cfg(test)]
#[path = "app_inventory_tests.rs"]
mod tests;
