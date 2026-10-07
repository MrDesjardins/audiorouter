//! Portable wire rules of the Network Send / Network Receive datagram
//! (GRAPH-16, SEC-13): the header constants, pairing-key authentication and
//! the replay window. The socket code lives in `audiorouter-windows-audio`;
//! everything here is pure and tested on every platform.
//!
//! A datagram is a 20-byte header, `frames * channels` float32 samples and,
//! when the two computers share a pairing key, a 16-byte tag:
//!
//! ```text
//! offset  size  field
//!      0     4  magic "ARNA"
//!      4     1  version: 1 = unauthenticated, 2 = authenticated
//!      5     1  channels (1..=2)
//!      6     2  frames per packet, little endian
//!      8     4  sample rate, little endian (48000)
//!     12     4  stream id, little endian (random per sender start)
//!     16     4  sequence, little endian (wraps)
//!     20     .  frames * channels float32, little endian, interleaved
//!    end-16 16  version 2 only: HMAC-SHA256 over every byte before it,
//!               truncated to 16 bytes
//! ```
//!
//! The version byte is covered by the tag, so an attacker cannot strip the
//! tag and present the packet as version 1 to a paired receiver: a paired
//! receiver accepts only version 2, and an unpaired one only version 1.

use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const NETWORK_AUDIO_MAGIC: [u8; 4] = *b"ARNA";
/// Unauthenticated datagrams (no pairing key on either computer).
pub const NETWORK_AUDIO_VERSION: u8 = 1;
/// Datagrams carrying a pairing-key tag.
pub const NETWORK_AUDIO_VERSION_AUTHENTICATED: u8 = 2;
pub const NETWORK_AUDIO_HEADER_BYTES: usize = 20;
pub const NETWORK_AUDIO_TAG_BYTES: usize = 16;
const VERSION_OFFSET: usize = 4;

/// Fixed context of the key derivation, so the same passphrase used for
/// anything else never yields this key.
const KEY_DERIVATION_CONTEXT: &[u8] = b"AudioRouter network audio pairing key v1";

/// Accepted sequences remembered behind the newest one of a stream.
pub const NETWORK_REPLAY_WINDOW_PACKETS: u32 = 64;
/// Streams (sender restarts) whose sequences are remembered.
const REPLAY_STREAMS: usize = 8;

/// The keyed MAC of one Network Send/Receive pair. Derived once when the
/// node starts; tagging and verifying a packet then neither allocates nor
/// blocks. `Debug` never prints key material.
#[derive(Clone)]
pub struct NetworkPairingKey {
    mac: Hmac<Sha256>,
}

impl std::fmt::Debug for NetworkPairingKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NetworkPairingKey(redacted)")
    }
}

impl NetworkPairingKey {
    /// Derive the packet key from a pairing key: HMAC-SHA256 keyed with the
    /// pairing key over a fixed context string. Returns `None` for a blank
    /// key (the node is not paired). The backend validates the key's length
    /// and characters (`audiorouter_domain::valid_network_pairing_key`)
    /// before a node starts.
    pub fn derive(pairing_key: &str) -> Option<Self> {
        if pairing_key.is_empty() {
            return None;
        }
        let mut derivation = Hmac::<Sha256>::new_from_slice(pairing_key.as_bytes()).ok()?;
        derivation.update(KEY_DERIVATION_CONTEXT);
        let packet_key = derivation.finalize().into_bytes();
        let mac = Hmac::<Sha256>::new_from_slice(&packet_key).ok()?;
        Some(Self { mac })
    }

    /// The truncated tag of `authenticated` (header and payload).
    pub fn tag(&self, authenticated: &[u8]) -> [u8; NETWORK_AUDIO_TAG_BYTES] {
        let mut mac = self.mac.clone();
        mac.update(authenticated);
        let full = mac.finalize().into_bytes();
        let mut tag = [0; NETWORK_AUDIO_TAG_BYTES];
        tag.copy_from_slice(&full[..NETWORK_AUDIO_TAG_BYTES]);
        tag
    }

    /// Check `tag` against `authenticated` in constant time.
    pub fn verify(&self, authenticated: &[u8], tag: &[u8]) -> bool {
        if tag.len() != NETWORK_AUDIO_TAG_BYTES {
            return false;
        }
        let mut mac = self.mac.clone();
        mac.update(authenticated);
        mac.verify_truncated_left(tag).is_ok()
    }
}

/// Why a datagram failed pairing, counted separately from other rejections
/// so the receiving node can tell the user which side to fix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkAuthFailure {
    /// Both computers are paired, but with different keys (or the packet
    /// was forged or altered).
    WrongKey,
    /// This receiver has a pairing key; the sender has none.
    SenderNotPaired,
    /// The sender has a pairing key; this receiver has none.
    ReceiverNotPaired,
}

impl NetworkAuthFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WrongKey => "wrongKey",
            Self::SenderNotPaired => "senderNotPaired",
            Self::ReceiverNotPaired => "receiverNotPaired",
        }
    }
}

/// Turn the unauthenticated datagram `buffer[..length]` into an
/// authenticated one: set the version byte to 2 and append the tag. Returns
/// the new length, or `None` when it does not look like a datagram or the
/// buffer has no room for the tag.
pub fn seal_network_datagram(
    key: &NetworkPairingKey,
    buffer: &mut [u8],
    length: usize,
) -> Option<usize> {
    let sealed = length.checked_add(NETWORK_AUDIO_TAG_BYTES)?;
    if length < NETWORK_AUDIO_HEADER_BYTES
        || sealed > buffer.len()
        || buffer[..4] != NETWORK_AUDIO_MAGIC
    {
        return None;
    }
    buffer[VERSION_OFFSET] = NETWORK_AUDIO_VERSION_AUTHENTICATED;
    let tag = key.tag(&buffer[..length]);
    buffer[length..sealed].copy_from_slice(&tag);
    Some(sealed)
}

/// Check a datagram's pairing before anything else uses it. `key` is this
/// receiver's key (`None`: not paired). Returns the length of the header and
/// payload (the tag excluded) for the normal format checks, or `Ok(None)`
/// when the datagram is not AudioRouter audio of a known version at all
/// (the caller rejects it as malformed or unsupported).
pub fn open_network_datagram(
    key: Option<&NetworkPairingKey>,
    datagram: &[u8],
) -> Result<Option<usize>, NetworkAuthFailure> {
    if datagram.len() < NETWORK_AUDIO_HEADER_BYTES || datagram[..4] != NETWORK_AUDIO_MAGIC {
        return Ok(None);
    }
    match (datagram[VERSION_OFFSET], key) {
        (NETWORK_AUDIO_VERSION, None) => Ok(Some(datagram.len())),
        (NETWORK_AUDIO_VERSION, Some(_)) => Err(NetworkAuthFailure::SenderNotPaired),
        (NETWORK_AUDIO_VERSION_AUTHENTICATED, None) => Err(NetworkAuthFailure::ReceiverNotPaired),
        (NETWORK_AUDIO_VERSION_AUTHENTICATED, Some(key)) => {
            let Some(body) = datagram
                .len()
                .checked_sub(NETWORK_AUDIO_TAG_BYTES)
                .filter(|body| *body >= NETWORK_AUDIO_HEADER_BYTES)
            else {
                return Err(NetworkAuthFailure::WrongKey);
            };
            if key.verify(&datagram[..body], &datagram[body..]) {
                Ok(Some(body))
            } else {
                Err(NetworkAuthFailure::WrongKey)
            }
        }
        _ => Ok(None),
    }
}

/// What the replay window says about an authenticated packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayVerdict {
    /// Not seen before: hand it to playout (which may still find it late).
    Fresh,
    /// Already accepted, or too old to tell: drop and count as replayed.
    Replayed,
}

#[derive(Clone, Copy, Default)]
struct StreamWindow {
    used: bool,
    stream_id: u32,
    newest: u32,
    /// Bit `n` set: sequence `newest - n` was accepted.
    seen: u64,
    last_used: u64,
}

/// Sliding anti-replay window per stream id (the IPsec/DTLS scheme), for
/// the last few sender restarts. Fixed size, so it never allocates. Apply
/// it only to packets whose tag verified: unauthenticated packets could
/// otherwise move the window.
#[derive(Clone)]
pub struct ReplayWindow {
    streams: [StreamWindow; REPLAY_STREAMS],
    clock: u64,
}

impl Default for ReplayWindow {
    fn default() -> Self {
        Self {
            streams: [StreamWindow::default(); REPLAY_STREAMS],
            clock: 0,
        }
    }
}

impl ReplayWindow {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Record `sequence` of `stream_id` and say whether it is new.
    pub fn check_and_accept(&mut self, stream_id: u32, sequence: u32) -> ReplayVerdict {
        self.clock += 1;
        let clock = self.clock;
        let slot = match self
            .streams
            .iter()
            .position(|stream| stream.used && stream.stream_id == stream_id)
        {
            Some(slot) => slot,
            None => {
                // A stream not seen recently (a restarted sender): take the
                // least recently used slot.
                let slot = self
                    .streams
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, stream)| (stream.used, stream.last_used))
                    .map_or(0, |(slot, _)| slot);
                self.streams[slot] = StreamWindow {
                    used: true,
                    stream_id,
                    newest: sequence,
                    seen: 1,
                    last_used: clock,
                };
                return ReplayVerdict::Fresh;
            }
        };
        let stream = &mut self.streams[slot];
        stream.last_used = clock;
        let ahead = sequence.wrapping_sub(stream.newest);
        if ahead != 0 && ahead < u32::MAX / 2 {
            stream.seen = if ahead >= 64 {
                1
            } else {
                (stream.seen << ahead) | 1
            };
            stream.newest = sequence;
            return ReplayVerdict::Fresh;
        }
        let behind = stream.newest.wrapping_sub(sequence);
        if behind >= NETWORK_REPLAY_WINDOW_PACKETS {
            return ReplayVerdict::Replayed;
        }
        let bit = 1_u64 << behind;
        if stream.seen & bit != 0 {
            return ReplayVerdict::Replayed;
        }
        stream.seen |= bit;
        ReplayVerdict::Fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "correct horse battery staple";

    fn datagram(version: u8, stream_id: u32, sequence: u32) -> Vec<u8> {
        let mut bytes = vec![0_u8; NETWORK_AUDIO_HEADER_BYTES + 8 + NETWORK_AUDIO_TAG_BYTES];
        bytes[..4].copy_from_slice(&NETWORK_AUDIO_MAGIC);
        bytes[4] = version;
        bytes[5] = 1;
        bytes[6..8].copy_from_slice(&2_u16.to_le_bytes());
        bytes[8..12].copy_from_slice(&48_000_u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&stream_id.to_le_bytes());
        bytes[16..20].copy_from_slice(&sequence.to_le_bytes());
        bytes[20..24].copy_from_slice(&0.5_f32.to_le_bytes());
        bytes[24..28].copy_from_slice(&(-0.5_f32).to_le_bytes());
        bytes
    }

    fn sealed(key: &str, stream_id: u32, sequence: u32) -> Vec<u8> {
        let mut bytes = datagram(NETWORK_AUDIO_VERSION, stream_id, sequence);
        let length = seal_network_datagram(
            &NetworkPairingKey::derive(key).unwrap(),
            &mut bytes,
            NETWORK_AUDIO_HEADER_BYTES + 8,
        )
        .unwrap();
        bytes.truncate(length);
        bytes
    }

    #[test]
    fn a_blank_key_is_not_paired_and_debug_hides_the_key() {
        assert!(NetworkPairingKey::derive("").is_none());
        assert_eq!(
            format!("{:?}", NetworkPairingKey::derive(KEY).unwrap()),
            "NetworkPairingKey(redacted)"
        );
    }

    /// Pin the derivation and tag, so a change of either is deliberate: two
    /// AudioRouter versions must agree on them to pair.
    #[test]
    fn derivation_and_tag_are_stable_hmac_sha256() {
        let key = NetworkPairingKey::derive("ABCDEFGHJKLMNPQRSTUVWXYZ").unwrap();
        let tag = key.tag(b"ARNA");
        let hex = tag
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        // Computed independently with Python's hmac/hashlib:
        // HMAC-SHA256(HMAC-SHA256(b"ABCDEFGHJKLMNPQRSTUVWXYZ",
        //   b"AudioRouter network audio pairing key v1"), b"ARNA")[:16]
        assert_eq!(hex, "af51ba3e60dcad7c0461471579c598e1");
    }

    #[test]
    fn a_paired_receiver_accepts_only_its_own_key() {
        let key = NetworkPairingKey::derive(KEY).unwrap();
        let good = sealed(KEY, 7, 1);
        assert_eq!(good[4], NETWORK_AUDIO_VERSION_AUTHENTICATED);
        assert_eq!(
            open_network_datagram(Some(&key), &good),
            Ok(Some(NETWORK_AUDIO_HEADER_BYTES + 8))
        );
        // Correct address, wrong key.
        assert_eq!(
            open_network_datagram(Some(&key), &sealed("another pairing key!", 7, 1)),
            Err(NetworkAuthFailure::WrongKey)
        );
        // Any altered byte (header, audio or tag) fails.
        for index in [5, 16, 21, good.len() - 1] {
            let mut altered = good.clone();
            altered[index] ^= 0x01;
            assert_eq!(
                open_network_datagram(Some(&key), &altered),
                Err(NetworkAuthFailure::WrongKey),
                "byte {index}"
            );
        }
        // Stripping the tag and claiming version 1 does not help.
        let mut stripped = good[..good.len() - NETWORK_AUDIO_TAG_BYTES].to_vec();
        stripped[4] = NETWORK_AUDIO_VERSION;
        assert_eq!(
            open_network_datagram(Some(&key), &stripped),
            Err(NetworkAuthFailure::SenderNotPaired)
        );
        // A truncated authenticated datagram is a failed tag, not a panic.
        assert_eq!(
            open_network_datagram(Some(&key), &good[..NETWORK_AUDIO_HEADER_BYTES + 4]),
            Err(NetworkAuthFailure::WrongKey)
        );
    }

    #[test]
    fn an_unpaired_receiver_keeps_todays_format_and_names_a_paired_sender() {
        let plain = datagram(NETWORK_AUDIO_VERSION, 7, 1);
        assert_eq!(open_network_datagram(None, &plain), Ok(Some(plain.len())));
        assert_eq!(
            open_network_datagram(None, &sealed(KEY, 7, 1)),
            Err(NetworkAuthFailure::ReceiverNotPaired)
        );
        // Unknown versions and foreign traffic are left to format checks.
        assert_eq!(open_network_datagram(None, &datagram(9, 7, 1)), Ok(None));
        assert_eq!(open_network_datagram(None, b"not audio"), Ok(None));
        let mut short = [0_u8; 8];
        assert_eq!(
            seal_network_datagram(&NetworkPairingKey::derive(KEY).unwrap(), &mut short, 8),
            None
        );
    }

    #[test]
    fn the_replay_window_rejects_repeats_and_old_packets_but_keeps_reordering() {
        let mut window = ReplayWindow::default();
        for sequence in [0_u32, 1, 2, 4] {
            assert_eq!(window.check_and_accept(7, sequence), ReplayVerdict::Fresh);
        }
        // 3 arrives late but was never accepted: fresh (playout calls it late).
        assert_eq!(window.check_and_accept(7, 3), ReplayVerdict::Fresh);
        assert_eq!(window.check_and_accept(7, 3), ReplayVerdict::Replayed);
        assert_eq!(window.check_and_accept(7, 4), ReplayVerdict::Replayed);
        assert_eq!(window.check_and_accept(7, 0), ReplayVerdict::Replayed);
        // Far ahead, then everything older than the window is refused.
        assert_eq!(window.check_and_accept(7, 1_000), ReplayVerdict::Fresh);
        assert_eq!(
            window.check_and_accept(7, 1_000 - NETWORK_REPLAY_WINDOW_PACKETS),
            ReplayVerdict::Replayed
        );
        assert_eq!(window.check_and_accept(7, 999), ReplayVerdict::Fresh);
        // A restarted sender (new stream id) starts its own window; replaying
        // the earlier stream is still refused.
        assert_eq!(window.check_and_accept(8, 0), ReplayVerdict::Fresh);
        assert_eq!(window.check_and_accept(8, 1), ReplayVerdict::Fresh);
        assert_eq!(window.check_and_accept(7, 1_000), ReplayVerdict::Replayed);
        assert_eq!(window.check_and_accept(7, 2), ReplayVerdict::Replayed);
        // Sequences wrap.
        assert_eq!(window.check_and_accept(9, u32::MAX), ReplayVerdict::Fresh);
        assert_eq!(window.check_and_accept(9, 0), ReplayVerdict::Fresh);
        assert_eq!(
            window.check_and_accept(9, u32::MAX),
            ReplayVerdict::Replayed
        );
        window.reset();
        assert_eq!(window.check_and_accept(9, 0), ReplayVerdict::Fresh);
    }

    #[test]
    fn the_replay_window_remembers_the_most_recent_streams() {
        let mut window = ReplayWindow::default();
        for stream in 0..REPLAY_STREAMS as u32 + 1 {
            assert_eq!(window.check_and_accept(stream, 5), ReplayVerdict::Fresh);
        }
        // The newest eight are remembered; the oldest was evicted.
        for stream in 1..REPLAY_STREAMS as u32 + 1 {
            assert_eq!(
                window.check_and_accept(stream, 5),
                ReplayVerdict::Replayed,
                "stream {stream}"
            );
        }
        assert_eq!(window.check_and_accept(0, 5), ReplayVerdict::Fresh);
    }
}
