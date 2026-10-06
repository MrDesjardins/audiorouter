//! Built-in Stats.cc feed for Ducks that follow the Rainbow Six Siege round
//! (`trigger: "siegeRound"`).
//!
//! Stats.cc (inspected 1.8.1) can serve its game state as JSON snapshots on a
//! local WebSocket when its feed file is present. This module is a client
//! only: it connects to `127.0.0.1` while a playing route needs it, maps each
//! snapshot to a [`RoundPhase`] with the same rules as the
//! `examples/integrations/stats-cc-siege` service, and refreshes the engine's
//! lock-free round signal. Snapshots contain player data and are never logged
//! or stored. Any failure publishes `Unknown`, so following Ducks release.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use audiorouter_engine::{siege_round_signal, RoundPhase};
use serde_json::{json, Value};

/// Stats.cc's feed port as configured by its feed file (the example default).
pub(crate) const STATS_CC_FEED_PORT: u16 = 17_892;
/// Stats.cc reads this file in its user-data folder to enable the feed.
pub(crate) const STATS_CC_FEED_FILE: &str = "ceb44052-d616-4bdc-993c-70bae41091e9.txt";

const MAX_SNAPSHOT_BYTES: usize = 1024 * 1024;
const READ_TIMEOUT: Duration = Duration::from_millis(250);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const PING_INTERVAL: Duration = Duration::from_secs(30);
const SILENCE_LIMIT: Duration = Duration::from_secs(75);
const IDLE_POLL: Duration = Duration::from_millis(500);
const MAX_BACKOFF: Duration = Duration::from_secs(15);

/// Map one Stats.cc snapshot to a round phase. Mirrors `phaseState` in the
/// example's `core.mjs`: no reliable state is `Unknown` (Ducks release).
pub(crate) fn round_phase(snapshot: &Value) -> RoundPhase {
    let Some(object) = snapshot.as_object() else {
        return RoundPhase::Unknown;
    };
    match object.get("status").and_then(Value::as_str) {
        Some("connected") => {}
        // The game is not running ("disconnected", "loading", "error") or
        // Stats.cc is not attached to it.
        _ => return RoundPhase::Unknown,
    }
    let Some(game_match) = object.get("match") else {
        return RoundPhase::Unknown;
    };
    if game_match.is_null() {
        // Main menu or matchmaking queue.
        return RoundPhase::Menu;
    }
    let Some(game_match) = game_match.as_object() else {
        return RoundPhase::Unknown;
    };
    let Some(phase) = game_match.get("phase") else {
        return RoundPhase::Unknown;
    };
    if game_match
        .get("ended_at")
        .is_some_and(|ended| !ended.is_null())
    {
        return RoundPhase::BetweenRounds;
    }
    match phase {
        Value::Null => RoundPhase::Prep,
        Value::String(phase) => match phase.as_str() {
            "action" => RoundPhase::Action,
            "results" => RoundPhase::BetweenRounds,
            "planning" | "prep" => RoundPhase::Prep,
            // Map/ban selection and future non-action phases stay quiet.
            other if !other.is_empty() && other.len() <= 80 => RoundPhase::Prep,
            _ => RoundPhase::Unknown,
        },
        _ => RoundPhase::Unknown,
    }
}

pub(crate) fn phase_name(phase: RoundPhase) -> &'static str {
    match phase {
        RoundPhase::Unknown => "unknown",
        RoundPhase::Menu => "menu",
        RoundPhase::Prep => "prep",
        RoundPhase::BetweenRounds => "betweenRounds",
        RoundPhase::Action => "action",
    }
}

/// Connection state reported to the UI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeedState {
    /// No playing route follows the round.
    Off,
    Connecting,
    /// Connected; Stats.cc sends a snapshot only when the state changes.
    WaitingForUpdate,
    Connected,
    /// Stats.cc is not running, or its feed file is missing.
    Unavailable,
}

impl FeedState {
    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Connecting => "connecting",
            Self::WaitingForUpdate => "waitingForUpdate",
            Self::Connected => "connected",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug)]
struct Status {
    state: FeedState,
    phase: RoundPhase,
}

/// Owner of the background feed client. The thread starts on first use and
/// connects only while `set_wanted(true)`.
pub(crate) struct SiegeRoundFeed {
    port: u16,
    wanted: Arc<AtomicBool>,
    status: Arc<Mutex<Status>>,
    started: bool,
}

impl Default for SiegeRoundFeed {
    fn default() -> Self {
        Self::new(STATS_CC_FEED_PORT)
    }
}

impl SiegeRoundFeed {
    pub(crate) fn new(port: u16) -> Self {
        Self {
            port,
            wanted: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(Status {
                state: FeedState::Off,
                phase: RoundPhase::Unknown,
            })),
            started: false,
        }
    }

    /// Connect while a playing route has a Duck following the round.
    pub(crate) fn set_wanted(&mut self, wanted: bool) {
        self.wanted.store(wanted, Ordering::Relaxed);
        if wanted && !self.started {
            let (port, wanted, status) = (
                self.port,
                Arc::clone(&self.wanted),
                Arc::clone(&self.status),
            );
            let spawned = std::thread::Builder::new()
                .name("audiorouter-siege-round".into())
                .spawn(move || run_feed(port, &wanted, &status));
            self.started = spawned.is_ok();
        }
    }

    pub(crate) fn status_json(&self) -> Value {
        let status = self
            .status
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        json!({
            "source": "statsCc",
            "state": status.state.name(),
            "phase": phase_name(status.phase),
            "feedConfigured": feed_file_present(),
        })
    }
}

/// Whether Stats.cc's feed file exists (its feed is enabled at Stats.cc start).
pub(crate) fn feed_file_present() -> Option<bool> {
    let folder = std::env::var_os("APPDATA")?;
    Some(
        std::path::Path::new(&folder)
            .join("stats.cc")
            .join(STATS_CC_FEED_FILE)
            .is_file(),
    )
}

fn set_status(status: &Mutex<Status>, state: FeedState, phase: RoundPhase) {
    let mut current = status
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    current.state = state;
    current.phase = phase;
}

fn run_feed(port: u16, wanted: &AtomicBool, status: &Mutex<Status>) {
    let signal = siege_round_signal();
    let mut backoff = Duration::from_secs(1);
    loop {
        if !wanted.load(Ordering::Relaxed) {
            signal.publish(RoundPhase::Unknown);
            set_status(status, FeedState::Off, RoundPhase::Unknown);
            std::thread::sleep(IDLE_POLL);
            continue;
        }
        set_status(status, FeedState::Connecting, RoundPhase::Unknown);
        match follow_feed(port, wanted, status) {
            // A normal close or an unwanted feed reconnects promptly.
            Ok(()) => backoff = Duration::from_secs(1),
            Err(()) => {
                signal.publish(RoundPhase::Unknown);
                set_status(status, FeedState::Unavailable, RoundPhase::Unknown);
                let until = Instant::now() + backoff;
                while Instant::now() < until && wanted.load(Ordering::Relaxed) {
                    std::thread::sleep(
                        IDLE_POLL.min(until.saturating_duration_since(Instant::now())),
                    );
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

/// Follow one connection until it closes, fails or is no longer wanted.
fn follow_feed(port: u16, wanted: &AtomicBool, status: &Mutex<Status>) -> Result<(), ()> {
    use tungstenite::{protocol::WebSocketConfig, Error, Message};

    let signal = siege_round_signal();
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map_err(|_| ())?;
    stream
        .set_read_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|_| ())?;
    stream
        .set_write_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|_| ())?;
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_SNAPSHOT_BYTES))
        .max_frame_size(Some(MAX_SNAPSHOT_BYTES));
    let (mut socket, _) = tungstenite::client::client_with_config(
        format!("ws://127.0.0.1:{port}/"),
        stream,
        Some(config),
    )
    .map_err(|_| ())?;
    socket
        .get_ref()
        .set_read_timeout(Some(READ_TIMEOUT))
        .map_err(|_| ())?;
    // Stats.cc sends no snapshot on connect, only on the next state change.
    let mut phase = RoundPhase::Unknown;
    set_status(status, FeedState::WaitingForUpdate, phase);
    let mut last_heard = Instant::now();
    let mut last_ping = Instant::now();
    loop {
        if !wanted.load(Ordering::Relaxed) {
            let _ = socket.close(None);
            let _ = socket.flush();
            return Ok(());
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                last_heard = Instant::now();
                phase = serde_json::from_str::<Value>(text.as_str())
                    .map_or(RoundPhase::Unknown, |snapshot| round_phase(&snapshot));
                set_status(status, FeedState::Connected, phase);
            }
            Ok(Message::Close(_)) => return Ok(()),
            Ok(_) => last_heard = Instant::now(),
            Err(Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(Error::ConnectionClosed) => return Ok(()),
            Err(_) => return Err(()),
        }
        // Refresh the signal at least every read timeout so Ducks stay engaged.
        signal.publish(phase);
        if last_heard.elapsed() > SILENCE_LIMIT {
            return Err(());
        }
        if last_ping.elapsed() > PING_INTERVAL {
            last_ping = Instant::now();
            if socket.send(Message::Ping(Vec::new().into())).is_err() {
                return Err(());
            }
        } else {
            // Sends queued pong replies; a timeout here is not a failure.
            let _ = socket.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(value: Value) -> RoundPhase {
        round_phase(&value)
    }

    #[test]
    fn snapshots_map_to_the_example_service_phases() {
        assert_eq!(
            phase(json!({ "status": "connected", "match": null })),
            RoundPhase::Menu
        );
        assert_eq!(
            phase(json!({ "status": "connected", "match": null, "startedQueuingAt": 5 })),
            RoundPhase::Menu
        );
        assert_eq!(
            phase(json!({ "status": "connected", "match": { "phase": null } })),
            RoundPhase::Prep
        );
        for quiet in ["planning", "prep", "ban"] {
            assert_eq!(
                phase(json!({ "status": "connected", "match": { "phase": quiet } })),
                RoundPhase::Prep,
                "{quiet}"
            );
        }
        assert_eq!(
            phase(json!({ "status": "connected", "match": { "phase": "action" } })),
            RoundPhase::Action
        );
        assert_eq!(
            phase(json!({ "status": "connected", "match": { "phase": "results" } })),
            RoundPhase::BetweenRounds
        );
        assert_eq!(
            phase(json!({ "status": "connected", "match": { "phase": "action", "ended_at": 12 } })),
            RoundPhase::BetweenRounds,
            "an ended match is results, not action"
        );
    }

    #[test]
    fn missing_or_malformed_state_is_unknown() {
        for value in [
            json!(null),
            json!([]),
            json!({}),
            json!({ "status": "disconnected", "match": null }),
            json!({ "status": "loading", "match": null }),
            json!({ "status": "weird", "match": null }),
            json!({ "status": "connected" }),
            json!({ "status": "connected", "match": [] }),
            json!({ "status": "connected", "match": {} }),
            json!({ "status": "connected", "match": { "phase": "" } }),
            json!({ "status": "connected", "match": { "phase": "x".repeat(81) } }),
            json!({ "status": "connected", "match": { "phase": 4 } }),
        ] {
            assert_eq!(phase(value.clone()), RoundPhase::Unknown, "{value}");
        }
    }

    #[test]
    #[ignore = "connects read-only to the running Stats.cc feed on 127.0.0.1:17892"]
    fn live_stats_cc_feed_accepts_the_client() {
        let mut feed = SiegeRoundFeed::default();
        feed.set_wanted(true);
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline
            && !matches!(
                feed.status_json()["state"].as_str(),
                Some("waitingForUpdate" | "connected")
            )
        {
            std::thread::sleep(Duration::from_millis(50));
        }
        let observed = Instant::now() + Duration::from_secs(10);
        while Instant::now() < observed && feed.status_json()["state"] != "connected" {
            std::thread::sleep(Duration::from_millis(100));
        }
        // State and phase only: snapshots contain player data.
        eprintln!("Stats.cc feed: {}", feed.status_json());
        assert!(matches!(
            feed.status_json()["state"].as_str(),
            Some("waitingForUpdate" | "connected")
        ));
        feed.set_wanted(false);
    }

    #[test]
    fn feed_client_publishes_phases_from_a_local_server_and_releases_on_close() {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            for phase in ["prep", "action"] {
                std::thread::sleep(Duration::from_millis(300));
                let snapshot =
                    json!({ "status": "connected", "match": { "phase": phase } }).to_string();
                socket
                    .send(tungstenite::Message::Text(snapshot.into()))
                    .unwrap();
            }
            std::thread::sleep(Duration::from_millis(300));
            let _ = socket.close(None);
            let _ = socket.flush();
        });
        let mut feed = SiegeRoundFeed::new(port);
        let wait_for = |feed: &SiegeRoundFeed, phase: &str| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if feed.status_json()["phase"] == phase {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        };
        feed.set_wanted(true);
        assert!(wait_for(&feed, "prep"), "{}", feed.status_json());
        assert_eq!(feed.status_json()["state"], "connected");
        assert!(wait_for(&feed, "action"));
        server.join().unwrap();
        // The server closed: the phase falls back to unknown (Ducks release).
        assert!(wait_for(&feed, "unknown"), "{}", feed.status_json());
        feed.set_wanted(false);
        let deadline = Instant::now() + Duration::from_secs(5);
        while feed.status_json()["state"] != "off" && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(feed.status_json()["state"], "off");
    }
}
