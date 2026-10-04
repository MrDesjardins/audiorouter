//! `network.jsonl`: what every Network Send and Network Receive did, so a
//! user's log folder is enough to tell why audio did not reach the other
//! computer. Records are written by the control thread (never the realtime
//! audio path): start and failure of each socket, a summary every few
//! seconds while playing, and the last summary when audio stops. Each record
//! carries a plain-language `hint` when the counters point at a cause.
//!
//! Content is bounded: node IDs, IP literals, ports, counters and Windows
//! socket error codes. No node names, paths or free-form error text.

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const LIMIT_BYTES: u64 = 2 * 1024 * 1024;
/// How often a playing network node is summarized: often at first, when
/// problems show, then rarely so long sessions keep a small log.
pub const SUMMARY_INTERVAL: Duration = Duration::from_secs(5);
pub const STEADY_SUMMARY_INTERVAL: Duration = Duration::from_secs(30);
const STEADY_AFTER: Duration = Duration::from_secs(60);

/// The Windows socket errors seen with network audio, explained.
pub fn socket_error_hint(code: i32) -> Option<&'static str> {
    Some(match code {
        10054 => "the receiving computer answered \"port closed\": AudioRouter is not listening on this port there (not playing, another port, or the port is used by another program)",
        10051 => "network unreachable from this computer: check the destination address and this computer's network connection",
        10065 => "the receiving computer cannot be reached: check its address and that both computers are on the same network",
        10013 => "blocked by Windows or security software: allow AudioRouter in Windows Firewall (private networks)",
        10048 => "the port is already used by another program on this computer: choose another port on both computers",
        10049 => "this address does not belong to this computer",
        _ => return None,
    })
}

/// Why a playing Network Receive may be silent, from its counters.
pub fn receive_hint(record: &ReceiveSummary) -> Option<String> {
    if let Some(sender) = record.last_rejected_sender.as_deref() {
        return Some(format!(
            "AudioRouter audio arrives from {sender}, but this Network Receive accepts only {expected}: set its sending computer address to {sender}",
            expected = record.expected_sender
        ));
    }
    if record.received_packets == 0 && record.rejected_datagrams == 0 && record.seconds_playing >= 5 {
        return Some(format!(
            "nothing arrived on UDP port {port}: on the sending computer, the Network Send must target {target}:{port} and be playing; Windows Firewall on this computer must allow AudioRouter (private networks); both computers must be on the same network",
            port = record.port,
            target = record.local_address_toward_sender.as_deref().unwrap_or("this computer's address"),
        ));
    }
    if let Some(code) = record.last_error_code {
        return socket_error_hint(code).map(str::to_owned);
    }
    if record.underruns_since_last > 0 {
        return Some("audio arrives but the buffer ran empty: raise the buffer (ms) on this Network Receive or use a wired connection".into());
    }
    None
}

/// Why a playing Network Send may not be heard, from its counters.
pub fn send_hint(record: &SendSummary) -> Option<String> {
    if let Some(hint) = record.last_error_code.and_then(socket_error_hint) {
        return Some(hint.to_owned());
    }
    if record.sent_packets > 0 {
        return Some(format!(
            "sending from {local} to {destination}: if the other computer hears nothing, its Network Receive must expect sender {local_ip} on port {port}, and its firewall must allow AudioRouter",
            local = record.local_address.as_deref().unwrap_or("unknown"),
            destination = record.destination,
            local_ip = record.local_address.as_deref().and_then(|address| address.rsplit_once(':').map(|(ip, _)| ip.trim_matches(['[', ']']))).unwrap_or("this computer's address"),
            port = record.destination.rsplit_once(':').map_or("", |(_, port)| port),
        ));
    }
    None
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReceiveSummary {
    pub node_id: String,
    pub expected_sender: String,
    pub port: u16,
    pub local_address_toward_sender: Option<String>,
    pub seconds_playing: u64,
    pub received_packets: u64,
    pub lost_packets: u64,
    pub late_packets: u64,
    pub rejected_datagrams: u64,
    pub last_rejected_sender: Option<String>,
    pub underruns: u64,
    pub underruns_since_last: u64,
    pub overflow_packets: u64,
    pub receive_errors: u64,
    pub last_error_code: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SendSummary {
    pub node_id: String,
    pub destination: String,
    pub local_address: Option<String>,
    pub seconds_playing: u64,
    pub sent_packets: u64,
    pub dropped_packets: u64,
    pub send_errors: u64,
    pub last_error_code: Option<i32>,
}

impl ReceiveSummary {
    pub fn to_record(&self, event: &str) -> Value {
        json!({
            "event": event, "role": "receive", "nodeId": self.node_id,
            "expectedSender": self.expected_sender, "port": self.port,
            "localAddressTowardSender": self.local_address_toward_sender,
            "secondsPlaying": self.seconds_playing,
            "receivedPackets": self.received_packets, "lostPackets": self.lost_packets,
            "latePackets": self.late_packets, "rejectedDatagrams": self.rejected_datagrams,
            "lastRejectedSender": self.last_rejected_sender, "underruns": self.underruns,
            "overflowPackets": self.overflow_packets, "receiveErrors": self.receive_errors,
            "lastErrorCode": self.last_error_code, "hint": receive_hint(self),
        })
    }
}

impl SendSummary {
    pub fn to_record(&self, event: &str) -> Value {
        json!({
            "event": event, "role": "send", "nodeId": self.node_id,
            "destination": self.destination, "localAddress": self.local_address,
            "secondsPlaying": self.seconds_playing,
            "sentPackets": self.sent_packets, "droppedPackets": self.dropped_packets,
            "sendErrors": self.send_errors, "lastErrorCode": self.last_error_code,
            "hint": send_hint(self),
        })
    }
}

/// Paces summaries and remembers the last one per node, to write a final
/// "stopped" record when the playing worker goes away.
#[derive(Default)]
pub struct Sampler {
    started: Option<Instant>,
    next: Option<Instant>,
    last: BTreeMap<String, Value>,
    previous_underruns: BTreeMap<String, u64>,
}

impl Sampler {
    /// Whether a summary is due; starts the clock on the first call.
    pub fn due(&mut self, now: Instant) -> bool {
        let started = *self.started.get_or_insert(now);
        let next = *self.next.get_or_insert(started + SUMMARY_INTERVAL);
        if now < next {
            return false;
        }
        let interval = if now.saturating_duration_since(started) < STEADY_AFTER { SUMMARY_INTERVAL } else { STEADY_SUMMARY_INTERVAL };
        self.next = Some(now + interval);
        true
    }

    pub fn seconds_playing(&self, now: Instant) -> u64 {
        self.started.map_or(0, |started| now.saturating_duration_since(started).as_secs())
    }

    /// Underruns since this node's previous summary.
    pub fn underruns_since_last(&mut self, node_id: &str, total: u64) -> u64 {
        let previous = self.previous_underruns.insert(node_id.to_owned(), total).unwrap_or(0);
        total.saturating_sub(previous)
    }

    pub fn remember(&mut self, node_id: &str, record: Value) {
        self.last.insert(node_id.to_owned(), record);
    }

    /// The final records (event "stopped") after audio stopped; resets.
    pub fn finish(&mut self) -> Vec<Value> {
        let records = std::mem::take(&mut self.last)
            .into_values()
            .map(|mut record| {
                record["event"] = json!("stopped");
                record
            })
            .collect();
        *self = Self::default();
        records
    }
}

fn log_directory() -> Option<PathBuf> {
    // Tests never write into the user's real log folder.
    if cfg!(test) {
        return None;
    }
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TEMP").map(PathBuf::from))
        .map(|root| root.join("AudioRouter").join("logs"))
}

/// Append one record to `network.jsonl` in the user's log folder.
pub fn write(record: Value) {
    if let Some(directory) = log_directory() {
        write_to(&directory, record);
    }
}

/// Append one record (with time, process and version) to `network.jsonl` in
/// `directory`, rotating to `network.previous.jsonl` at 2 MB.
pub fn write_to(directory: &Path, mut record: Value) {
    use std::io::Write;
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let Ok(_guard) = LOCK.lock() else { return };
    if std::fs::create_dir_all(directory).is_err() {
        return;
    }
    let path = directory.join("network.jsonl");
    if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() >= LIMIT_BYTES) {
        let previous = directory.join("network.previous.jsonl");
        let _ = std::fs::remove_file(&previous);
        let _ = std::fs::rename(&path, previous);
    }
    if let Some(fields) = record.as_object_mut() {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        fields.insert("timeUnixMs".into(), json!(now_ms));
        fields.insert("processId".into(), json!(std::process::id()));
        fields.insert("version".into(), json!(env!("CARGO_PKG_VERSION")));
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        if serde_json::to_writer(&mut file, &record).is_ok() {
            let _ = file.write_all(b"\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receive() -> ReceiveSummary {
        ReceiveSummary { node_id: "rx".into(), expected_sender: "192.168.1.20".into(), port: 50_600, local_address_toward_sender: Some("192.168.1.30".into()), seconds_playing: 10, ..Default::default() }
    }

    #[test]
    fn receive_hints_name_the_likely_cause() {
        let silent = receive();
        let hint = receive_hint(&silent).unwrap();
        assert!(hint.contains("nothing arrived on UDP port 50600") && hint.contains("192.168.1.30:50600") && hint.contains("Firewall"), "{hint}");
        let wrong_adapter = ReceiveSummary { rejected_datagrams: 40, last_rejected_sender: Some("10.0.0.7".into()), ..receive() };
        assert!(receive_hint(&wrong_adapter).unwrap().contains("set its sending computer address to 10.0.0.7"));
        let fine = ReceiveSummary { received_packets: 500, ..receive() };
        assert_eq!(receive_hint(&fine), None);
        let choppy = ReceiveSummary { received_packets: 500, underruns_since_last: 3, ..receive() };
        assert!(receive_hint(&choppy).unwrap().contains("raise the buffer"));
        let starting = ReceiveSummary { seconds_playing: 2, ..receive() };
        assert_eq!(receive_hint(&starting), None, "no verdict in the first seconds");
    }

    #[test]
    fn send_hints_explain_socket_errors_and_name_the_source_address() {
        let closed = SendSummary { node_id: "tx".into(), destination: "192.168.1.30:50600".into(), local_address: Some("192.168.1.20:61000".into()), send_errors: 3, last_error_code: Some(10054), ..Default::default() };
        assert!(send_hint(&closed).unwrap().contains("port closed"));
        let sending = SendSummary { last_error_code: None, send_errors: 0, sent_packets: 900, ..closed.clone() };
        let hint = send_hint(&sending).unwrap();
        assert!(hint.contains("expect sender 192.168.1.20 on port 50600"), "{hint}");
        assert_eq!(socket_error_hint(12345), None);
    }

    #[test]
    fn records_are_bounded_rotated_and_finished_on_stop() {
        let directory = std::env::temp_dir().join(format!("audiorouter-network-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        write_to(&directory, receive().to_record("summary"));
        let text = std::fs::read_to_string(directory.join("network.jsonl")).unwrap();
        let record: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(record["event"], "summary");
        assert_eq!(record["expectedSender"], "192.168.1.20");
        assert!(record["hint"].as_str().unwrap().contains("nothing arrived"));
        assert!(record["timeUnixMs"].as_u64().is_some() && record["version"].is_string());
        // Rotation keeps one previous file.
        std::fs::write(directory.join("network.jsonl"), vec![b'x'; LIMIT_BYTES as usize]).unwrap();
        write_to(&directory, receive().to_record("summary"));
        assert!(directory.join("network.previous.jsonl").exists());
        assert!(std::fs::metadata(directory.join("network.jsonl")).unwrap().len() < 4096);
        let _ = std::fs::remove_dir_all(&directory);

        let mut sampler = Sampler::default();
        let start = Instant::now();
        assert!(!sampler.due(start));
        assert!(sampler.due(start + SUMMARY_INTERVAL));
        assert!(!sampler.due(start + SUMMARY_INTERVAL + Duration::from_secs(1)));
        assert!(sampler.due(start + STEADY_AFTER + Duration::from_secs(1)));
        assert!(!sampler.due(start + STEADY_AFTER + Duration::from_secs(20)), "every 30 s after the first minute");
        assert!(sampler.due(start + STEADY_AFTER + Duration::from_secs(31)));
        assert_eq!(sampler.underruns_since_last("rx", 4), 4);
        assert_eq!(sampler.underruns_since_last("rx", 6), 2);
        sampler.remember("rx", receive().to_record("summary"));
        let finished = sampler.finish();
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0]["event"], "stopped");
        assert!(sampler.finish().is_empty(), "finish resets");
    }
}
