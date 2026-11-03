//! Version-3 encrypted audio and authenticated transport freshness negotiation.
//! Only network threads use this module; it has no sockets or random source.

use super::{NetworkPairingKey, NETWORK_AUDIO_HEADER_BYTES, NETWORK_AUDIO_MAGIC};
use chacha20::{
    cipher::{KeyIvInit, StreamCipher},
    XChaCha20,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const VERSION: u8 = 3;
pub const CHALLENGE_BYTES: usize = 16;
pub const NONCE_BYTES: usize = 24;
pub const TAG_BYTES: usize = 16;
pub const DATA_OVERHEAD: usize = CHALLENGE_BYTES + NONCE_BYTES + TAG_BYTES;
pub const PROBE_BYTES: usize = 6 + CHALLENGE_BYTES + TAG_BYTES;
pub const RESPONSE_BYTES: usize = PROBE_BYTES + CHALLENGE_BYTES;
const SECURE_HEADER_BYTES: usize = NETWORK_AUDIO_HEADER_BYTES + CHALLENGE_BYTES + NONCE_BYTES;
const MAX_PAYLOAD_BYTES: usize = 256 * 2 * 4;
const HANDSHAKE_MAGIC: [u8; 4] = *b"ARNH";

/// Encryption and authentication use independent, context-derived keys.
#[derive(Clone)]
pub struct SecureNetworkKey {
    encryption: [u8; 32],
    authentication: Hmac<Sha256>,
}

impl std::fmt::Debug for SecureNetworkKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecureNetworkKey(redacted)")
    }
}

impl SecureNetworkKey {
    /// Recognize authenticated audio for the wrong-address hint without
    /// decrypting or accepting it for this receiver's current token.
    pub fn authentic(&self, datagram: &[u8]) -> bool {
        datagram.len() >= SECURE_HEADER_BYTES + TAG_BYTES
            && datagram.len() <= SECURE_HEADER_BYTES + MAX_PAYLOAD_BYTES + TAG_BYTES
            && datagram[..4] == NETWORK_AUDIO_MAGIC
            && datagram[4] == VERSION
            && self.verify(
                &datagram[..datagram.len() - TAG_BYTES],
                &datagram[datagram.len() - TAG_BYTES..],
            )
    }

    pub fn from_pairing_key(key: &NetworkPairingKey) -> Self {
        let mut encryption = key.mac.clone();
        encryption.update(b"AudioRouter network audio v3 XChaCha20 key");
        let encryption = encryption.finalize().into_bytes().into();
        let mut authentication = key.mac.clone();
        authentication.update(b"AudioRouter network audio v3 authentication\0");
        Self {
            encryption,
            authentication,
        }
    }

    fn tag(&self, bytes: &[u8]) -> [u8; TAG_BYTES] {
        let mut mac = self.authentication.clone();
        mac.update(bytes);
        let mut tag = [0; TAG_BYTES];
        tag.copy_from_slice(&mac.finalize().into_bytes()[..TAG_BYTES]);
        tag
    }

    fn verify(&self, body: &[u8], tag: &[u8]) -> bool {
        if tag.len() != TAG_BYTES {
            return false;
        }
        let mut mac = self.authentication.clone();
        mac.update(body);
        mac.verify_truncated_left(tag).is_ok()
    }

    /// Encrypt a plain v1 packet in place. The caller must supply a fresh
    /// CSPRNG nonce for every packet under this key. Header and receiver token
    /// are authenticated along with the ciphertext before any plaintext use.
    pub fn seal(
        &self,
        buffer: &mut [u8],
        plain_len: usize,
        receiver: &[u8; CHALLENGE_BYTES],
        nonce: &[u8; NONCE_BYTES],
    ) -> Option<usize> {
        let payload = plain_len.checked_sub(NETWORK_AUDIO_HEADER_BYTES)?;
        let sealed_len = plain_len.checked_add(DATA_OVERHEAD)?;
        if payload > MAX_PAYLOAD_BYTES
            || sealed_len > buffer.len()
            || buffer[..4] != NETWORK_AUDIO_MAGIC
            || buffer[4] != 1
        {
            return None;
        }
        buffer.copy_within(NETWORK_AUDIO_HEADER_BYTES..plain_len, SECURE_HEADER_BYTES);
        buffer[4] = VERSION;
        buffer[NETWORK_AUDIO_HEADER_BYTES..NETWORK_AUDIO_HEADER_BYTES + CHALLENGE_BYTES]
            .copy_from_slice(receiver);
        buffer[NETWORK_AUDIO_HEADER_BYTES + CHALLENGE_BYTES..SECURE_HEADER_BYTES]
            .copy_from_slice(nonce);
        let body = sealed_len - TAG_BYTES;
        XChaCha20::new(&self.encryption.into(), nonce.into())
            .apply_keystream(&mut buffer[SECURE_HEADER_BYTES..body]);
        let tag = self.tag(&buffer[..body]);
        buffer[body..sealed_len].copy_from_slice(&tag);
        Some(sealed_len)
    }

    /// Authenticate and check the receiver's fresh token before decryption.
    /// On success restore the v1 header/payload layout for the existing shape
    /// validator; on failure leave the input unchanged.
    pub fn open(
        &self,
        datagram: &mut [u8],
        receiver: &[u8; CHALLENGE_BYTES],
    ) -> Result<usize, SecurePacketError> {
        if datagram.len() < NETWORK_AUDIO_HEADER_BYTES || datagram[..4] != NETWORK_AUDIO_MAGIC {
            return Err(SecurePacketError::Malformed);
        }
        if datagram[4] != VERSION {
            return Err(SecurePacketError::Outdated);
        }
        if datagram.len() < SECURE_HEADER_BYTES + TAG_BYTES
            || datagram.len() > SECURE_HEADER_BYTES + MAX_PAYLOAD_BYTES + TAG_BYTES
        {
            return Err(SecurePacketError::Malformed);
        }
        let body = datagram.len() - TAG_BYTES;
        if !self.verify(&datagram[..body], &datagram[body..]) {
            return Err(SecurePacketError::WrongKey);
        }
        if datagram[NETWORK_AUDIO_HEADER_BYTES..NETWORK_AUDIO_HEADER_BYTES + CHALLENGE_BYTES]
            != *receiver
        {
            return Err(SecurePacketError::ReceiverChanged);
        }
        let nonce: [u8; NONCE_BYTES] = datagram
            [NETWORK_AUDIO_HEADER_BYTES + CHALLENGE_BYTES..SECURE_HEADER_BYTES]
            .try_into()
            .expect("fixed secure header");
        XChaCha20::new(&self.encryption.into(), &nonce.into())
            .apply_keystream(&mut datagram[SECURE_HEADER_BYTES..body]);
        datagram.copy_within(SECURE_HEADER_BYTES..body, NETWORK_AUDIO_HEADER_BYTES);
        datagram[4] = 1;
        Ok(NETWORK_AUDIO_HEADER_BYTES + body - SECURE_HEADER_BYTES)
    }

    pub fn probe(&self, challenge: &[u8; CHALLENGE_BYTES]) -> [u8; PROBE_BYTES] {
        let mut message = [0; PROBE_BYTES];
        message[..4].copy_from_slice(&HANDSHAKE_MAGIC);
        message[4] = VERSION;
        message[5] = 0;
        message[6..6 + CHALLENGE_BYTES].copy_from_slice(challenge);
        let body = PROBE_BYTES - TAG_BYTES;
        let tag = self.tag(&message[..body]);
        message[body..].copy_from_slice(&tag);
        message
    }

    pub fn read_probe(&self, message: &[u8]) -> Option<[u8; CHALLENGE_BYTES]> {
        if !self.valid_handshake(message, PROBE_BYTES, 0) {
            return None;
        }
        message[6..6 + CHALLENGE_BYTES].try_into().ok()
    }

    pub fn response(
        &self,
        challenge: &[u8; CHALLENGE_BYTES],
        receiver: &[u8; CHALLENGE_BYTES],
    ) -> [u8; RESPONSE_BYTES] {
        let mut message = [0; RESPONSE_BYTES];
        message[..PROBE_BYTES - TAG_BYTES]
            .copy_from_slice(&self.probe(challenge)[..PROBE_BYTES - TAG_BYTES]);
        message[5] = 1;
        message[6 + CHALLENGE_BYTES..6 + CHALLENGE_BYTES * 2].copy_from_slice(receiver);
        let body = RESPONSE_BYTES - TAG_BYTES;
        let tag = self.tag(&message[..body]);
        message[body..].copy_from_slice(&tag);
        message
    }

    /// Only the response to this sender's outstanding random challenge can
    /// select a token. Discard the outstanding challenge after accepting it.
    pub fn read_response(
        &self,
        message: &[u8],
        challenge: &[u8; CHALLENGE_BYTES],
    ) -> Option<[u8; CHALLENGE_BYTES]> {
        if !self.valid_handshake(message, RESPONSE_BYTES, 1)
            || message[6..6 + CHALLENGE_BYTES] != *challenge
        {
            return None;
        }
        message[6 + CHALLENGE_BYTES..6 + CHALLENGE_BYTES * 2]
            .try_into()
            .ok()
    }

    fn valid_handshake(&self, message: &[u8], expected: usize, kind: u8) -> bool {
        message.len() == expected
            && message[..4] == HANDSHAKE_MAGIC
            && message[4] == VERSION
            && message[5] == kind
            && self.verify(
                &message[..expected - TAG_BYTES],
                &message[expected - TAG_BYTES..],
            )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurePacketError {
    Malformed,
    Outdated,
    WrongKey,
    ReceiverChanged,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SecureNetworkKey {
        SecureNetworkKey::from_pairing_key(
            &NetworkPairingKey::derive("ABCDEFGHIJKLMNOPQRSTUVWX").unwrap(),
        )
    }

    fn sealed() -> Vec<u8> {
        let mut packet = vec![0; NETWORK_AUDIO_HEADER_BYTES + 128 + DATA_OVERHEAD];
        packet[..4].copy_from_slice(&NETWORK_AUDIO_MAGIC);
        packet[4] = 1;
        packet[20..148].fill(42);
        key()
            .seal(&mut packet, 148, &[7; CHALLENGE_BYTES], &[8; NONCE_BYTES])
            .unwrap();
        packet
    }

    #[test]
    fn encrypts_samples_and_authenticates_every_byte_before_decrypting() {
        let encrypted = sealed();
        assert_ne!(
            &encrypted[SECURE_HEADER_BYTES..SECURE_HEADER_BYTES + 128],
            &[42; 128]
        );
        for index in 5..encrypted.len() {
            let mut changed = encrypted.clone();
            changed[index] ^= 1;
            let before = changed.clone();
            assert_eq!(
                key().open(&mut changed, &[7; CHALLENGE_BYTES]),
                Err(SecurePacketError::WrongKey)
            );
            assert_eq!(
                changed, before,
                "decrypting byte {index} before authentication"
            );
        }
        let mut packet = encrypted;
        assert_eq!(key().open(&mut packet, &[7; CHALLENGE_BYTES]), Ok(148));
        assert_eq!(packet[4], 1);
        assert_eq!(&packet[20..148], &[42; 128]);
    }

    #[test]
    fn a_restarted_receiver_refuses_captured_audio_and_old_challenge_responses() {
        let mut recorded = sealed();
        assert_eq!(
            key().open(&mut recorded, &[9; CHALLENGE_BYTES]),
            Err(SecurePacketError::ReceiverChanged)
        );
        let response = key().response(&[1; CHALLENGE_BYTES], &[7; CHALLENGE_BYTES]);
        assert_eq!(
            key().read_response(&response, &[1; CHALLENGE_BYTES]),
            Some([7; CHALLENGE_BYTES])
        );
        assert_eq!(key().read_response(&response, &[2; CHALLENGE_BYTES]), None);
        assert_eq!(
            key().read_probe(&key().probe(&[1; CHALLENGE_BYTES])),
            Some([1; CHALLENGE_BYTES])
        );
        let wrong = SecureNetworkKey::from_pairing_key(
            &NetworkPairingKey::derive("wrong-pairing-key").unwrap(),
        );
        assert_eq!(wrong.read_probe(&key().probe(&[1; CHALLENGE_BYTES])), None);
        assert_eq!(wrong.read_response(&response, &[1; CHALLENGE_BYTES]), None);
        assert_eq!(
            wrong.open(&mut recorded, &[7; CHALLENGE_BYTES]),
            Err(SecurePacketError::WrongKey)
        );
    }

    #[test]
    fn bounds_frames_and_refuses_downgrade_or_truncated_handshakes() {
        assert_eq!(
            key().open(&mut [0; 4], &[7; CHALLENGE_BYTES]),
            Err(SecurePacketError::Malformed)
        );
        let mut old = sealed();
        old[4] = 2;
        assert_eq!(
            key().open(&mut old, &[7; CHALLENGE_BYTES]),
            Err(SecurePacketError::Outdated)
        );
        let probe = key().probe(&[0; CHALLENGE_BYTES]);
        for length in 0..probe.len() {
            assert!(key().read_probe(&probe[..length]).is_none());
        }
        let mut changed = probe;
        changed[6] ^= 1;
        assert!(key().read_probe(&changed).is_none());
        assert_eq!(format!("{:?}", key()), "SecureNetworkKey(redacted)");
    }
}
