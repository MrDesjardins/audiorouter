//! Diagnostics helpers shared by every adapter (UI shell, HTTP, MCP, backend):
//! request correlation IDs, the opt-in verbose window, and support-bundle
//! redaction. Everything here is portable and free of I/O so it can be unit
//! tested on any host.
//!
//! A correlation ID is the optional top-level `requestId` member of a
//! JSON-RPC request. It names one user action or adapter request so its
//! records can be matched across `shell.jsonl`, `backend.jsonl`,
//! `mcp-activity.jsonl` and the client diagnostics. Backends that predate it
//! ignore the unknown member. It is never derived from user data.

use crate::JsonRpcRequest;
use serde::{Deserialize, Serialize};

/// JSON member that carries the correlation ID beside `jsonrpc`/`id`.
pub const CORRELATION_ID_MEMBER: &str = "requestId";
/// Longest accepted correlation ID, in ASCII characters.
pub const MAX_CORRELATION_ID_LEN: usize = 32;
/// Longest verbose-diagnostics window: one hour, then it switches itself off.
pub const MAX_VERBOSE_DIAGNOSTICS_MS: u64 = 60 * 60 * 1000;

/// Whether `value` is a well-formed correlation ID: 1 to 32 characters from
/// `[A-Za-z0-9-]`. Anything else is dropped (never logged or echoed).
pub fn is_valid_correlation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CORRELATION_ID_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// `value` when it is a valid correlation ID.
pub fn valid_correlation_id(value: Option<&str>) -> Option<&str> {
    value.filter(|value| is_valid_correlation_id(value))
}

/// The valid top-level `requestId` of one decoded request object.
pub fn correlation_id_of(request: &serde_json::Value) -> Option<&str> {
    valid_correlation_id(
        request
            .get(CORRELATION_ID_MEMBER)
            .and_then(serde_json::Value::as_str),
    )
}

/// Correlation IDs of a decoded JSON-RPC payload, in request order: one entry
/// for a single request, one per element for a batch.
pub fn payload_correlation_ids(payload: &serde_json::Value) -> Vec<Option<String>> {
    match payload {
        serde_json::Value::Array(requests) => requests
            .iter()
            .map(|request| correlation_id_of(request).map(str::to_owned))
            .collect(),
        request => vec![correlation_id_of(request).map(str::to_owned)],
    }
}

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Eight Crockford base-32 characters (40 bits) from `entropy`, mixed first so
/// nearby inputs (a counter, a clock) give unrelated IDs.
pub fn correlation_id_from_entropy(entropy: u64) -> String {
    let mut mixed = entropy.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^= mixed >> 31;
    (0..8)
        .map(|index| CROCKFORD[((mixed >> (index * 5)) & 31) as usize] as char)
        .collect()
}

/// A fresh correlation ID from the clock, the process ID and a per-process
/// counter. It identifies a request in local logs; it is not a secret.
pub fn new_correlation_id() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or_default();
    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    correlation_id_from_entropy(
        nanos ^ (u64::from(std::process::id()) << 40) ^ count.wrapping_mul(0x2545_F491_4F6C_DD1D),
    )
}

/// A request as it is sent on the wire with its correlation ID. `JsonRpcRequest`
/// itself stays unchanged; this wrapper adds the optional member.
#[derive(Debug, Serialize)]
pub struct CorrelatedRequest<'a> {
    #[serde(flatten)]
    pub request: &'a JsonRpcRequest,
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<&'a str>,
}

impl<'a> CorrelatedRequest<'a> {
    /// Attach `request_id` only when it is valid.
    pub fn new(request: &'a JsonRpcRequest, request_id: Option<&'a str>) -> Self {
        Self {
            request,
            request_id: valid_correlation_id(request_id),
        }
    }
}

/// A request received from an adapter that may carry a correlation ID. The ID
/// is kept as raw JSON so a malformed value is dropped instead of failing the
/// whole request.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct IncomingRequest {
    #[serde(flatten)]
    pub request: JsonRpcRequest,
    #[serde(rename = "requestId", default)]
    pub request_id: Option<serde_json::Value>,
}

impl IncomingRequest {
    /// The correlation ID when it is a valid string, otherwise `None`.
    pub fn correlation_id(&self) -> Option<&str> {
        valid_correlation_id(self.request_id.as_ref().and_then(serde_json::Value::as_str))
    }
}

/// Milliseconds since the Unix epoch (0 if the clock is before it).
pub fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

/// The opt-in verbose diagnostics window. While active, logs also record
/// routine-read successes and per-request durations (still never parameters,
/// paths or audio). It expires by itself at most one hour after it was
/// switched on; a clock that jumps back cannot extend it past that hour.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VerboseDiagnostics {
    until_unix_ms: Option<u64>,
}

impl VerboseDiagnostics {
    /// Switch verbose logging on for one hour from `now_unix_ms`, or off.
    pub fn set(&mut self, enabled: bool, now_unix_ms: u64) {
        self.until_unix_ms =
            enabled.then(|| now_unix_ms.saturating_add(MAX_VERBOSE_DIAGNOSTICS_MS));
    }

    /// A window that another process reported (`expiresAtUnixMs`). An expiry
    /// in the past or more than one hour ahead is treated as off.
    pub fn from_expiry(expires_at_unix_ms: Option<u64>, now_unix_ms: u64) -> Self {
        let window = Self {
            until_unix_ms: expires_at_unix_ms,
        };
        if window.is_active(now_unix_ms) {
            window
        } else {
            Self::default()
        }
    }

    /// Milliseconds left, or 0 when off or expired.
    pub fn remaining_ms(&self, now_unix_ms: u64) -> u64 {
        match self.until_unix_ms {
            Some(until)
                if until > now_unix_ms && until - now_unix_ms <= MAX_VERBOSE_DIAGNOSTICS_MS =>
            {
                until - now_unix_ms
            }
            _ => 0,
        }
    }

    pub fn is_active(&self, now_unix_ms: u64) -> bool {
        self.remaining_ms(now_unix_ms) > 0
    }

    /// The `diagnostics.getVerbose`/`setVerbose` result.
    pub fn status(&self, now_unix_ms: u64) -> serde_json::Value {
        let remaining = self.remaining_ms(now_unix_ms);
        serde_json::json!({
            "enabled": remaining > 0,
            "expiresAtUnixMs": (remaining > 0).then_some(self.until_unix_ms).flatten(),
            "remainingSeconds": remaining.div_ceil(1000),
            "maxSeconds": MAX_VERBOSE_DIAGNOSTICS_MS / 1000,
        })
    }
}

/// Replace anything that looks like a file-system path with `<path>`: drive
/// paths (`C:\…`, `C:/…`, including their JSON-escaped `C:\\…` form), UNC
/// paths (`\\server\share`) and `/Users/…` or `/home/…` paths. Used on log
/// lines before they enter a support bundle, as a second line of defence:
/// the logs are already written without parameters or paths.
pub fn redact_user_paths(text: &str) -> String {
    // Paths may contain spaces ("Program Files"), so only quotes, line
    // breaks and list punctuation end one. In JSON log lines the closing
    // quote always does.
    const STOP: &[char] = &[
        '"', '\'', '\t', '\r', '\n', ',', ';', ')', ']', '}', '>', '<', '|',
    ];
    let chars = text.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        let rest = &chars[index..];
        let previous_is_word = index > 0 && chars[index - 1].is_ascii_alphanumeric();
        let drive = !previous_is_word
            && rest.len() >= 3
            && rest[0].is_ascii_alphabetic()
            && rest[1] == ':'
            && (rest[2] == '\\' || rest[2] == '/');
        let unc = rest.len() >= 3 && rest[0] == '\\' && rest[1] == '\\' && {
            // `\\server` raw, or `\\\\server` as JSON-escaped text.
            let after = rest.iter().position(|character| *character != '\\');
            after.is_some_and(|offset| {
                (offset == 2 || offset == 4) && rest[offset].is_ascii_alphanumeric()
            })
        };
        let unix = !previous_is_word
            && ["/Users/", "/home/"]
                .iter()
                .any(|prefix| rest.iter().take(prefix.len()).copied().eq(prefix.chars()));
        if drive || unc || unix {
            output.push_str("<path>");
            while index < chars.len() && !STOP.contains(&chars[index]) {
                index += 1;
            }
            continue;
        }
        output.push(chars[index]);
        index += 1;
    }
    output
}

/// The last `max_lines` lines of `text`, at most `max_bytes` long (whole lines
/// only, so a cut never splits a JSON record).
pub fn tail_lines(text: &str, max_lines: usize, max_bytes: usize) -> String {
    let mut kept = Vec::new();
    let mut bytes = 0usize;
    for line in text.lines().rev().filter(|line| !line.is_empty()) {
        if kept.len() >= max_lines || bytes + line.len() + 1 > max_bytes {
            break;
        }
        bytes += line.len() + 1;
        kept.push(line);
    }
    kept.reverse();
    let mut output = kept.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlation_ids_accept_only_short_safe_ascii() {
        for valid in ["A", "abc-123", "K7Q2M9XD", &"a".repeat(32)] {
            assert!(is_valid_correlation_id(valid), "{valid}");
        }
        for invalid in [
            "",
            &"a".repeat(33),
            "has space",
            "under_score",
            "C:\\Users",
            "naïve",
            "a\nb",
            "{\"x\":1}",
        ] {
            assert!(!is_valid_correlation_id(invalid), "{invalid:?}");
        }
    }

    #[test]
    fn generated_ids_are_eight_crockford_characters_and_differ() {
        let first = correlation_id_from_entropy(1);
        let second = correlation_id_from_entropy(2);
        assert_eq!(first.len(), 8);
        assert_ne!(first, second);
        assert!(is_valid_correlation_id(&first));
        assert!(first.bytes().all(|byte| CROCKFORD.contains(&byte)));
        let ids = (0..64).map(|_| new_correlation_id()).collect::<Vec<_>>();
        let unique = ids.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn correlated_request_round_trips_and_old_parsers_ignore_the_member() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(7)),
            method: "graph.commit".into(),
            params: Some(serde_json::json!({"sessionId":"s"})),
        };
        let encoded =
            serde_json::to_value(CorrelatedRequest::new(&request, Some("AB12CD34"))).unwrap();
        assert_eq!(encoded["requestId"], "AB12CD34");
        assert_eq!(encoded["method"], "graph.commit");
        // A backend that knows nothing about the member parses it unchanged.
        let plain: JsonRpcRequest = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(plain, request);
        let parsed = crate::parse_rpc_message(encoded.to_string().as_bytes()).unwrap();
        assert_eq!(parsed, crate::RpcMessage::Single(request.clone()));
        let incoming: IncomingRequest = serde_json::from_value(encoded).unwrap();
        assert_eq!(incoming.request, request);
        assert_eq!(incoming.correlation_id(), Some("AB12CD34"));
        // An invalid ID is never attached, and an invalid incoming one is dropped.
        let encoded =
            serde_json::to_value(CorrelatedRequest::new(&request, Some("bad id"))).unwrap();
        assert!(encoded.get("requestId").is_none());
        let incoming: IncomingRequest = serde_json::from_value(
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"status.get","requestId":{"x":"C:\\private"}}),
        )
        .unwrap();
        assert_eq!(incoming.correlation_id(), None);
        let incoming: IncomingRequest =
            serde_json::from_value(serde_json::json!({"jsonrpc":"2.0","method":"status.get"}))
                .unwrap();
        assert_eq!(incoming.request.id, None);
        assert_eq!(incoming.correlation_id(), None);
    }

    #[test]
    fn payload_ids_follow_request_order_in_batches() {
        let batch = serde_json::json!([
            {"jsonrpc":"2.0","id":1,"method":"a","requestId":"ONE"},
            {"jsonrpc":"2.0","id":2,"method":"b"},
            {"jsonrpc":"2.0","id":3,"method":"c","requestId":"bad id"},
        ]);
        assert_eq!(
            payload_correlation_ids(&batch),
            vec![Some("ONE".to_owned()), None, None]
        );
        assert_eq!(
            payload_correlation_ids(&serde_json::json!({"requestId":"X-1"})),
            vec![Some("X-1".to_owned())]
        );
    }

    #[test]
    fn verbose_window_expires_after_one_hour_with_an_injected_clock() {
        let mut verbose = VerboseDiagnostics::default();
        assert!(!verbose.is_active(1_000));
        verbose.set(true, 1_000);
        assert!(verbose.is_active(1_000));
        assert_eq!(verbose.remaining_ms(1_000), MAX_VERBOSE_DIAGNOSTICS_MS);
        assert_eq!(verbose.status(1_000)["remainingSeconds"], 3_600);
        assert!(verbose.is_active(1_000 + MAX_VERBOSE_DIAGNOSTICS_MS - 1));
        assert_eq!(
            verbose.status(1_000 + MAX_VERBOSE_DIAGNOSTICS_MS - 1)["remainingSeconds"],
            1
        );
        assert!(!verbose.is_active(1_000 + MAX_VERBOSE_DIAGNOSTICS_MS));
        let expired = verbose.status(1_000 + MAX_VERBOSE_DIAGNOSTICS_MS);
        assert_eq!(expired["enabled"], false);
        assert_eq!(expired["expiresAtUnixMs"], serde_json::Value::Null);
        // A clock that jumps back more than the window cannot extend it.
        assert!(!verbose.is_active(0));
        verbose.set(false, 2_000);
        assert!(!verbose.is_active(2_000));
        assert_eq!(verbose.status(2_000)["enabled"], false);
    }

    #[test]
    fn reported_expiry_is_trusted_only_within_one_hour() {
        let now = 10_000_000;
        assert!(VerboseDiagnostics::from_expiry(Some(now + 60_000), now).is_active(now));
        assert!(!VerboseDiagnostics::from_expiry(Some(now - 1), now).is_active(now));
        assert!(
            !VerboseDiagnostics::from_expiry(Some(now + MAX_VERBOSE_DIAGNOSTICS_MS + 1), now)
                .is_active(now)
        );
        assert!(!VerboseDiagnostics::from_expiry(None, now).is_active(now));
    }

    #[test]
    fn redaction_removes_drive_unc_and_home_paths_in_raw_and_json_form() {
        let line = r#"{"method":"plugins.scan","x":"C:\\Users\\Ana\\VST3","y":"D:/Music/take.wav","z":"\\\\nas\\share\\a","w":"/home/ana/x","ok":"graph.commit"}"#;
        let redacted = redact_user_paths(line);
        assert!(!redacted.contains("Ana"), "{redacted}");
        assert!(!redacted.contains("Music"), "{redacted}");
        assert!(!redacted.contains("nas"), "{redacted}");
        assert!(!redacted.contains("ana"), "{redacted}");
        assert!(redacted.contains("graph.commit"));
        assert!(
            serde_json::from_str::<serde_json::Value>(&redacted).is_ok(),
            "{redacted}"
        );
        assert_eq!(
            redact_user_paths(r"Error at C:\Program Files\App, retry"),
            "Error at <path>, retry"
        );
        // Times such as 20:14 and hex values are not paths.
        assert_eq!(
            redact_user_paths("at 20:14 0xE000020B"),
            "at 20:14 0xE000020B"
        );
    }

    #[test]
    fn tail_keeps_whole_last_lines_within_both_bounds() {
        let text = "one\ntwo\nthree\nfour\n";
        assert_eq!(tail_lines(text, 2, 1_000), "three\nfour\n");
        assert_eq!(tail_lines(text, 10, 11), "three\nfour\n");
        assert_eq!(tail_lines(text, 10, 3), "");
        assert_eq!(tail_lines("", 10, 100), "");
    }
}
