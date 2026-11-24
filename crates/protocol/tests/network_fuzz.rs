//! Q4.3 bounded fuzzing of the Network Send / Receive wire rules, the only
//! parser that takes datagrams from another computer: header checks, the v2
//! tag, the encrypted v3 packet, the v3 handshake and the replay window.
//!
//! Properties: no panic for any byte string, no allocation, a rejected packet
//! never leaves the caller's buffer half-decrypted, and a packet that fails
//! the tag is never accepted.
use audiorouter_protocol::network_audio::secure::{
    SecureNetworkKey, SecurePacketError, CHALLENGE_BYTES, DATA_OVERHEAD, NONCE_BYTES, PROBE_BYTES,
    RESPONSE_BYTES,
};
use audiorouter_protocol::network_audio::{
    open_network_datagram, seal_network_datagram, NetworkPairingKey, ReplayVerdict, ReplayWindow,
    NETWORK_AUDIO_HEADER_BYTES, NETWORK_AUDIO_MAGIC, NETWORK_AUDIO_TAG_BYTES,
    NETWORK_AUDIO_VERSION, NETWORK_REPLAY_WINDOW_PACKETS,
};

#[path = "../../../tests/support/fuzz.rs"]
mod fuzz;

#[global_allocator]
static ALLOCATOR: fuzz::TrackingAllocator = fuzz::TrackingAllocator;

const SEED: u64 = 0x4E45_5457_4F52_4B21; // "NETWORK!"
const PAIRING: &str = "fuzz fixture pairing key, not a secret";

fn pairing_key() -> NetworkPairingKey {
    NetworkPairingKey::derive(PAIRING).unwrap()
}

fn secure_key() -> SecureNetworkKey {
    SecureNetworkKey::from_pairing_key(&pairing_key())
}

/// A plain v1 packet with `frames` stereo frames.
fn plain_packet(frames: usize, sequence: u32) -> Vec<u8> {
    let mut bytes = vec![0_u8; NETWORK_AUDIO_HEADER_BYTES + frames * 2 * 4];
    bytes[..4].copy_from_slice(&NETWORK_AUDIO_MAGIC);
    bytes[4] = NETWORK_AUDIO_VERSION;
    bytes[5] = 2;
    bytes[6..8].copy_from_slice(&(frames as u16).to_le_bytes());
    bytes[8..12].copy_from_slice(&48_000_u32.to_le_bytes());
    bytes[12..16].copy_from_slice(&0xABCD_u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&sequence.to_le_bytes());
    bytes
}

fn v2_packet(frames: usize) -> Vec<u8> {
    let mut bytes = plain_packet(frames, 5);
    let plain = bytes.len();
    bytes.resize(plain + NETWORK_AUDIO_TAG_BYTES, 0);
    let length = seal_network_datagram(&pairing_key(), &mut bytes, plain).unwrap();
    bytes.truncate(length);
    bytes
}

fn v3_packet(frames: usize) -> Vec<u8> {
    let mut bytes = plain_packet(frames, 9);
    let plain = bytes.len();
    bytes.resize(plain + DATA_OVERHEAD, 0);
    let length = secure_key()
        .seal(&mut bytes, plain, &[7; CHALLENGE_BYTES], &[3; NONCE_BYTES])
        .unwrap();
    bytes.truncate(length);
    bytes
}

fn assert_no_allocation<R>(what: &str, input: &[u8], work: impl FnOnce() -> R) -> R {
    let (result, largest) = fuzz::measure_allocation(work);
    // Verification clones the HMAC state (a few hundred bytes); nothing may
    // scale with, or exceed, the datagram itself.
    assert!(
        largest <= 1024,
        "{what} requested {largest} bytes for {}: {}",
        fuzz::hex(input),
        input.len()
    );
    result
}

#[test]
fn every_length_of_every_packet_shape_is_handled() {
    let key = pairing_key();
    let secure = secure_key();
    let receiver = [7_u8; CHALLENGE_BYTES];
    let shapes = [plain_packet(2, 1), v2_packet(2), v3_packet(2)];
    for shape in &shapes {
        for length in 0..=shape.len() + 8 {
            let mut datagram = shape.clone();
            datagram.resize(length, 0xA5);
            assert_no_allocation("open_network_datagram", &datagram, || {
                let _ = open_network_datagram(None, &datagram);
                let _ = open_network_datagram(Some(&key), &datagram);
            });
            let mut copy = datagram.clone();
            assert_no_allocation("secure open", &datagram, || {
                let _ = secure.authentic(&datagram);
                let _ = secure.open(&mut copy, &receiver);
            });
        }
    }
}

#[test]
fn mutated_packets_never_panic_and_a_bad_tag_is_never_accepted() {
    let key = pairing_key();
    let secure = secure_key();
    let receiver = [7_u8; CHALLENGE_BYTES];
    let seeds = [v2_packet(1), v2_packet(64), v3_packet(1), v3_packet(256)];
    let mut accepted_unmodified = 0;
    fuzz::run("protocol/network-packets", SEED, 4_000, |_, rng| {
        let seed = rng.pick(&seeds).clone();
        let datagram = fuzz::mutate(rng, &seed, 4 * 1024);
        let unchanged = datagram == seed;
        let paired = assert_no_allocation("paired open", &datagram, || {
            open_network_datagram(Some(&key), &datagram)
        });
        if let Ok(Some(body)) = paired {
            // Accepted by the v2 rule: the body is at least a header and the
            // bytes it covers are exactly those the tag was computed over.
            assert!(body >= NETWORK_AUDIO_HEADER_BYTES && body <= datagram.len());
            assert!(
                unchanged || datagram[4] != 2 || key.verify(&datagram[..body], &datagram[body..])
            );
        }
        let mut copy = datagram.clone();
        let opened = assert_no_allocation("secure open", &datagram, || {
            secure.open(&mut copy, &receiver)
        });
        match opened {
            Ok(length) => {
                // Only the untouched packet carries a valid tag.
                assert!(
                    unchanged,
                    "mutated packet accepted: {}",
                    fuzz::hex(&datagram)
                );
                assert!(length >= NETWORK_AUDIO_HEADER_BYTES && length <= copy.len());
                accepted_unmodified += 1;
            }
            Err(_) => assert_eq!(copy, datagram, "a rejected packet was modified"),
        }
        // The tag covers every byte before it, so only the original verifies.
        assert!(!secure.authentic(&datagram) || unchanged);
    });
    assert!(
        accepted_unmodified > 0,
        "the valid seeds were never replayed"
    );
}

#[test]
fn handshake_messages_reject_every_corruption() {
    let secure = secure_key();
    let challenge = [1_u8; CHALLENGE_BYTES];
    let receiver = [2_u8; CHALLENGE_BYTES];
    let probe = secure.probe(&challenge);
    let response = secure.response(&challenge, &receiver);
    assert_eq!(secure.read_probe(&probe), Some(challenge));
    assert_eq!(secure.read_response(&response, &challenge), Some(receiver));
    assert_eq!((probe.len(), response.len()), (PROBE_BYTES, RESPONSE_BYTES));
    fuzz::run("protocol/handshake", SEED ^ 1, 3_000, |_, rng| {
        let seed: &[u8] = if rng.one_in(2) { &probe } else { &response };
        let message = fuzz::mutate(rng, seed, 128);
        assert_no_allocation("handshake", &message, || {
            let probed = secure.read_probe(&message);
            let answered = secure.read_response(&message, &challenge);
            if message != probe {
                assert!(probed.is_none(), "{}", fuzz::hex(&message));
            }
            if message != response {
                assert!(answered.is_none(), "{}", fuzz::hex(&message));
            }
        });
    });
    // Every length, including the exact sizes with a flipped tag bit.
    for length in 0..=RESPONSE_BYTES + 4 {
        let message = vec![0_u8; length];
        assert!(secure.read_probe(&message).is_none());
        assert!(secure.read_response(&message, &challenge).is_none());
    }
    let mut flipped = probe;
    flipped[PROBE_BYTES - 1] ^= 1;
    assert!(secure.read_probe(&flipped).is_none());
}

#[test]
fn open_distinguishes_the_documented_failures_at_the_boundaries() {
    let secure = secure_key();
    let receiver = [7_u8; CHALLENGE_BYTES];
    let valid = v3_packet(2);
    let mut other_receiver = valid.clone();
    assert_eq!(
        secure.open(&mut other_receiver, &[8; CHALLENGE_BYTES]),
        Err(SecurePacketError::ReceiverChanged)
    );
    let mut outdated = valid.clone();
    outdated[4] = 2;
    assert_eq!(
        secure.open(&mut outdated, &receiver),
        Err(SecurePacketError::Outdated)
    );
    let mut tag_flip = valid.clone();
    *tag_flip.last_mut().unwrap() ^= 1;
    assert_eq!(
        secure.open(&mut tag_flip, &receiver),
        Err(SecurePacketError::WrongKey)
    );
    // v2 at the exact edges of "a tag after a header".
    let key = pairing_key();
    for length in [
        NETWORK_AUDIO_HEADER_BYTES - 1,
        NETWORK_AUDIO_HEADER_BYTES,
        NETWORK_AUDIO_HEADER_BYTES + NETWORK_AUDIO_TAG_BYTES - 1,
        NETWORK_AUDIO_HEADER_BYTES + NETWORK_AUDIO_TAG_BYTES,
    ] {
        let mut datagram = vec![0_u8; length];
        datagram[..4].copy_from_slice(&NETWORK_AUDIO_MAGIC);
        if length > 4 {
            datagram[4] = 2;
        }
        assert!(
            !matches!(open_network_datagram(Some(&key), &datagram), Ok(Some(_))),
            "an unsigned {length}-byte datagram was accepted"
        );
    }
}

/// The replay window must stay consistent for any sequence of (stream,
/// sequence) pairs, including the wraparound at `u32::MAX`, and never accept
/// the same pair twice while it is inside the window.
#[test]
fn replay_window_never_accepts_a_pair_twice_inside_the_window() {
    fuzz::run("protocol/replay-window", SEED ^ 2, 300, |_, rng| {
        let mut window = ReplayWindow::default();
        let mut accepted: Vec<(u32, u32)> = Vec::new();
        let mut newest: [Option<u32>; 4] = [None; 4];
        let base = *rng.pick(&[0_u32, 1, 1_000, u32::MAX - 40, u32::MAX - 3]);
        for _ in 0..200 {
            let stream = rng.below(4);
            let sequence = match newest[stream] {
                None => base,
                Some(previous) => match rng.below(5) {
                    0 => previous,
                    1 => previous.wrapping_sub(rng.below(80) as u32),
                    2 => previous.wrapping_add(rng.below(5) as u32 + 1),
                    3 => previous.wrapping_add(rng.below(200) as u32),
                    _ => rng.next_u32(),
                },
            };
            let stream_id = 0x100 + stream as u32;
            let verdict = window.check_and_accept(stream_id, sequence);
            if verdict == ReplayVerdict::Fresh {
                // Fresh means the pair is not one already accepted within
                // the window behind the newest accepted sequence.
                let duplicate = accepted.iter().any(|(id, seen)| {
                    *id == stream_id
                        && *seen == sequence
                        && newest[stream].is_some_and(|newest| {
                            newest.wrapping_sub(sequence) < NETWORK_REPLAY_WINDOW_PACKETS
                        })
                });
                assert!(
                    !duplicate,
                    "stream {stream_id:#x} sequence {sequence} accepted twice"
                );
                accepted.push((stream_id, sequence));
                let ahead = newest[stream].map_or(true, |newest| {
                    let step = sequence.wrapping_sub(newest);
                    step != 0 && step < u32::MAX / 2
                });
                if ahead {
                    newest[stream] = Some(sequence);
                }
            }
        }
    });
}
