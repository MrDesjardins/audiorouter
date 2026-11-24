//! A client waits for a free pipe instance up to its own request timeout.
//!
//! The control thread runs one request at a time, and a bundle export or
//! import holds it for seconds. Meanwhile every server instance is held by a
//! client waiting for its turn, so another client finds no free instance. The
//! client used to give up after a fixed two seconds with "timed out waiting
//! for a free named-pipe instance", although its own timeout (60 s for the
//! window, HTTP adapter, MCP and CLI) and the instance freeing up a moment
//! later would have served it. Evidence:
//! `docs/plans/active/evidence/2026-11-24-synthetic-service-timing.md`
//! ("Pipe-busy follow-up").
//!
//! The servers here hold their instances with a gate instead of a slow
//! request, so the tests need no database and no disk. Every pipe has a
//! unique name; none is the default control pipe.
#![cfg(windows)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use audiorouter_transport::{
    probe_pipe, round_trip, round_trip_with_timeout, send_oneway, serve_once_with_client,
    PipeProbe, TransportError, SERVER_PIPE_INSTANCES,
};

/// Opens once; every handler waits for it.
#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    fn wait(&self) {
        let mut open = self.open.lock().unwrap();
        while !*open {
            open = self.changed.wait(open).unwrap();
        }
    }

    fn open(&self) {
        *self.open.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

/// A pipe whose every server instance is busy until the gate opens.
struct BusyPipe {
    name: String,
    gate: Arc<Gate>,
    stop: Arc<AtomicBool>,
    entered: Arc<AtomicUsize>,
    holders: Vec<std::thread::JoinHandle<()>>,
}

fn frame() -> Vec<u8> {
    audiorouter_protocol::encode_frame(&serde_json::json!({ "ok": true })).unwrap()
}

impl BusyPipe {
    /// Serve `SERVER_PIPE_INSTANCES` instances (as the production backend
    /// does) and connect one client to each, so that all of them are busy
    /// inside their handler.
    fn saturate(label: &str) -> Self {
        let name = format!(
            r"\\.\pipe\audiorouter-busy-wait-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let gate = Arc::new(Gate::default());
        let stop = Arc::new(AtomicBool::new(false));
        let entered = Arc::new(AtomicUsize::new(0));
        for _ in 0..SERVER_PIPE_INSTANCES {
            let (name, gate, stop, entered) =
                (name.clone(), gate.clone(), stop.clone(), entered.clone());
            // Like the production I/O threads: serve one client, then the next.
            std::thread::spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    let _ = serve_once_with_client(&name, |_, _request| {
                        if stop.load(Ordering::Acquire) {
                            return Err(TransportError::Protocol("stopped".into()));
                        }
                        entered.fetch_add(1, Ordering::AcqRel);
                        gate.wait();
                        Ok(frame())
                    });
                }
            });
        }
        let holders = (0..SERVER_PIPE_INSTANCES)
            .map(|_| {
                let name = name.clone();
                std::thread::spawn(move || {
                    // The server may not be listening yet; this retries
                    // within the client's own wait.
                    round_trip(&name, &frame()).expect("a holder is served once the gate opens");
                })
            })
            .collect();
        let pipe = Self {
            name,
            gate,
            stop,
            entered,
            holders,
        };
        let started = Instant::now();
        while pipe.entered.load(Ordering::Acquire) < SERVER_PIPE_INSTANCES as usize {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "the holders did not occupy every instance"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        pipe
    }

    /// Let the held requests finish. The servers keep serving, so a client
    /// that waited for an instance is answered.
    fn open_gate(&mut self) {
        self.gate.open();
        for holder in self.holders.drain(..) {
            holder.join().unwrap();
        }
    }

    /// Stop the servers and wake any that wait for a client. Call this only
    /// after every client of the test has finished, or it may take the
    /// instance a waiting client needs.
    fn shutdown(mut self) {
        self.open_gate();
        self.stop.store(true, Ordering::Release);
        for _ in 0..SERVER_PIPE_INSTANCES {
            let _ = send_oneway(&self.name, &0_u32.to_le_bytes());
        }
    }
}

/// With every instance busy for longer than the old fixed two-second wait, a
/// client whose timeout is longer waits and is served when an instance frees.
#[test]
fn a_client_waits_for_a_free_instance_for_as_long_as_its_timeout() {
    let mut pipe = BusyPipe::saturate("waits");
    let name = pipe.name.clone();
    let client = std::thread::spawn(move || {
        let started = Instant::now();
        let result = round_trip_with_timeout(&name, &frame(), Duration::from_secs(30));
        (result, started.elapsed())
    });
    // Past the two seconds a client used to wait, and the client must still be
    // waiting.
    std::thread::sleep(Duration::from_millis(2_600));
    assert!(
        !client.is_finished(),
        "the client gave up while its 30 s timeout had not run out"
    );
    pipe.open_gate();
    // The servers are still running, so the waiting client is answered.
    let (result, waited) = client.join().unwrap();
    pipe.shutdown();
    assert!(
        result.is_ok(),
        "the client was not served once an instance freed: {result:?}"
    );
    assert!(
        waited >= Duration::from_millis(2_500),
        "the client was served before the instances freed ({waited:?})"
    );
}

/// A probe asks "does a server own this pipe?". Every instance busy still
/// means yes, and it answers at once instead of waiting for an instance; only
/// a pipe nobody created is `Missing`.
#[test]
fn a_probe_reports_a_busy_pipe_as_owned_and_a_missing_pipe_as_missing() {
    let pipe = BusyPipe::saturate("probe");
    let started = Instant::now();
    let found = probe_pipe(&pipe.name);
    let waited = started.elapsed();
    pipe.shutdown();
    assert_eq!(found, Ok(PipeProbe::Busy));
    assert!(
        found.unwrap().is_owned(),
        "a busy pipe has a server behind it"
    );
    assert!(
        waited < Duration::from_millis(500),
        "the probe waited {waited:?} on a busy pipe"
    );

    let missing = format!(
        r"\\.\pipe\audiorouter-busy-wait-missing-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let found = probe_pipe(&missing);
    assert_eq!(found, Ok(PipeProbe::Missing));
    assert!(!found.unwrap().is_owned());
}

/// The wait for an instance is bounded by the caller's timeout, not only by
/// a constant: the shell gives `status.get` two seconds and a probe may give
/// less, so a short timeout must still fail quickly.
#[test]
fn a_short_timeout_bounds_the_wait_for_a_free_instance() {
    let pipe = BusyPipe::saturate("short");
    let started = Instant::now();
    let result = round_trip_with_timeout(&pipe.name, &frame(), Duration::from_millis(300));
    let waited = started.elapsed();
    let message = format!("{result:?}");
    assert!(
        message.contains("timed out waiting for a free named-pipe instance"),
        "expected the busy-pipe error, got {message}"
    );
    assert!(
        waited < Duration::from_millis(1_500),
        "a 300 ms timeout waited {waited:?} for an instance"
    );
    pipe.shutdown();
}
