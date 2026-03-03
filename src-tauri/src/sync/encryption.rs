//! Application-layer E2E encryption for sync updates.
//!
//! This sits **on top of** WebRTC DTLS (transport encryption) to provide
//! defence-in-depth: even if the transport is compromised, sync payloads
//! remain encrypted with a key derived from the SPAKE2+ pairing.
//!
//! Cipher: AES-256-GCM via the `ring` crate.
//! Key derivation: HKDF-SHA256 from a shared secret.
//! Nonces: 96-bit, counter-based (monotonically increasing per session).
//! Direction: Separate keys for creator→joiner and joiner→creator to prevent
//!   nonce reuse if both sides send simultaneously.

use ring::aead::{self, Aad, BoundKey, Nonce, NonceSequence, SealingKey, OpeningKey, UnboundKey, NONCE_LEN};
use ring::hkdf;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use zeroize::Zeroizing;

/// An encrypted sync message sent over the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    /// Counter used as nonce — receiver uses this to construct the nonce.
    pub counter: u64,
    /// AES-256-GCM ciphertext + 16-byte auth tag.
    pub ciphertext: Vec<u8>,
}

/// Session encryption state. Created after SPAKE2+ key exchange completes.
///
/// Uses direction-specific keys: the creator and joiner derive different
/// send/recv key pairs from the same shared secret, preventing nonce reuse
/// when both sides send simultaneously.
pub struct SessionEncryption {
    /// Key this side uses for SENDING (sealing).
    send_key: Zeroizing<Vec<u8>>,
    /// Key this side uses for RECEIVING (opening).
    recv_key: Zeroizing<Vec<u8>>,
    /// Monotonically increasing counter for nonce generation (send direction only).
    seal_counter: AtomicU64,
    /// Highest counter seen from remote peer (for replay protection).
    /// Initialised to u64::MAX as sentinel meaning "no messages received yet".
    max_recv_counter: AtomicU64,
    /// Whether this side is the creator (needed for AAD direction).
    is_creator: bool,
}

impl SessionEncryption {
    /// Create from a shared secret (e.g. output of SPAKE2+ key exchange).
    /// The secret is expanded via HKDF-SHA256 into two 256-bit AES keys:
    /// one for creator→joiner and one for joiner→creator.
    ///
    /// `is_creator` determines which key is used for sending vs receiving.
    pub fn from_shared_secret(secret: &[u8], is_creator: bool) -> Result<Self, String> {
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"aurus-sync-v1");
        let prk = salt.extract(secret);

        // Derive creator-to-joiner key
        let mut c2j_key = Zeroizing::new(vec![0u8; 32]);
        prk.expand(&[b"aurus-sync-v1-c2j"], HkdfLen(32))
            .map_err(|_| "HKDF expand failed (c2j)".to_string())?
            .fill(&mut c2j_key)
            .map_err(|_| "HKDF fill failed (c2j)".to_string())?;

        // Derive joiner-to-creator key
        let mut j2c_key = Zeroizing::new(vec![0u8; 32]);
        prk.expand(&[b"aurus-sync-v1-j2c"], HkdfLen(32))
            .map_err(|_| "HKDF expand failed (j2c)".to_string())?
            .fill(&mut j2c_key)
            .map_err(|_| "HKDF fill failed (j2c)".to_string())?;

        let (send_key, recv_key) = if is_creator {
            (c2j_key, j2c_key)
        } else {
            (j2c_key, c2j_key)
        };

        Ok(Self {
            send_key,
            recv_key,
            seal_counter: AtomicU64::new(0),
            max_recv_counter: AtomicU64::new(u64::MAX),
            is_creator,
        })
    }

    /// Return direction-specific AAD bytes for authenticated encryption.
    fn direction_aad(&self, sending: bool) -> &'static [u8] {
        if (self.is_creator && sending) || (!self.is_creator && !sending) {
            b"aurus-c2j"
        } else {
            b"aurus-j2c"
        }
    }

    /// Encrypt a plaintext payload (e.g. a yrs update vector).
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedEnvelope, String> {
        let counter = self.seal_counter.fetch_add(1, Ordering::SeqCst);
        let nonce_bytes = counter_to_nonce(counter);

        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &self.send_key)
            .map_err(|_| "Failed to create AES key".to_string())?;

        let mut sealing_key = SealingKey::new(unbound_key, SingleNonce::new(nonce_bytes));

        let mut in_out = plaintext.to_vec();
        sealing_key
            .seal_in_place_append_tag(Aad::from(self.direction_aad(true)), &mut in_out)
            .map_err(|_| "Encryption failed".to_string())?;

        Ok(EncryptedEnvelope {
            counter,
            ciphertext: in_out,
        })
    }

    /// Decrypt an encrypted envelope from a remote peer.
    pub fn decrypt(&self, envelope: &EncryptedEnvelope) -> Result<Vec<u8>, String> {
        // Replay protection: reject messages with non-advancing counters
        let max = self.max_recv_counter.load(Ordering::SeqCst);
        if max != u64::MAX && envelope.counter <= max {
            return Err("Replay detected: counter not advancing".to_string());
        }

        let nonce_bytes = counter_to_nonce(envelope.counter);

        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &self.recv_key)
            .map_err(|_| "Failed to create AES key".to_string())?;

        let mut opening_key = OpeningKey::new(unbound_key, SingleNonce::new(nonce_bytes));

        let mut in_out = envelope.ciphertext.clone();
        let plaintext = opening_key
            .open_in_place(Aad::from(self.direction_aad(false)), &mut in_out)
            .map_err(|_| "Decryption failed — invalid key or tampered data".to_string())?;

        // Update max counter only after successful decryption
        self.max_recv_counter.store(envelope.counter, Ordering::SeqCst);

        Ok(plaintext.to_vec())
    }

    /// Rotate the encryption key using HKDF ratchet.
    /// Derives new keys from the current keys + a "rotate" context.
    /// The old key material is zeroed automatically by Zeroizing.
    /// The counter continues monotonically (not reset).
    pub fn rotate_key(&mut self) -> Result<(), String> {
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"aurus-sync-rotate");

        // HKDF info must match between creator send and joiner recv (and vice versa)
        // so both sides derive the same rotated key for each direction.
        let (send_info, recv_info): (&[u8], &[u8]) = if self.is_creator {
            (b"next-c2j-key", b"next-j2c-key")
        } else {
            (b"next-j2c-key", b"next-c2j-key")
        };

        // Rotate send key
        let prk = salt.extract(&self.send_key);
        let mut new_send = Zeroizing::new(vec![0u8; 32]);
        prk.expand(&[send_info], HkdfLen(32))
            .map_err(|_| "HKDF expand failed during rotation (send)".to_string())?
            .fill(&mut new_send)
            .map_err(|_| "HKDF fill failed during rotation (send)".to_string())?;
        self.send_key = new_send;

        // Rotate recv key
        let prk_recv = salt.extract(&self.recv_key);
        let mut new_recv = Zeroizing::new(vec![0u8; 32]);
        prk_recv.expand(&[recv_info], HkdfLen(32))
            .map_err(|_| "HKDF expand failed during rotation (recv)".to_string())?
            .fill(&mut new_recv)
            .map_err(|_| "HKDF fill failed during rotation (recv)".to_string())?;
        self.recv_key = new_recv;

        // DO NOT reset seal_counter — continue monotonically
        tracing::debug!("Encryption keys rotated (both directions)");
        Ok(())
    }
}

// No manual Drop needed — Zeroizing<Vec<u8>> handles zeroing on drop.

// ---------------------------------------------------------------------------
// Nonce helpers
// ---------------------------------------------------------------------------

/// Convert a u64 counter to a 96-bit (12-byte) nonce.
/// Layout: [0, 0, 0, 0, counter_be_bytes(8)]
fn counter_to_nonce(counter: u64) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    nonce[4..12].copy_from_slice(&counter.to_be_bytes());
    nonce
}

/// A `NonceSequence` that yields exactly one nonce.
struct SingleNonce {
    nonce: Option<[u8; NONCE_LEN]>,
}

impl SingleNonce {
    fn new(nonce: [u8; NONCE_LEN]) -> Self {
        Self { nonce: Some(nonce) }
    }
}

impl NonceSequence for SingleNonce {
    fn advance(&mut self) -> Result<Nonce, ring::error::Unspecified> {
        self.nonce
            .take()
            .map(Nonce::assume_unique_for_key)
            .ok_or(ring::error::Unspecified)
    }
}

/// Helper for HKDF output length.
struct HkdfLen(usize);

impl hkdf::KeyType for HkdfLen {
    fn len(&self) -> usize {
        self.0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let secret = b"test-shared-secret-32-bytes-long";
        let creator = SessionEncryption::from_shared_secret(secret, true).unwrap();
        let joiner = SessionEncryption::from_shared_secret(secret, false).unwrap();

        let plaintext = b"Hello, sync world!";
        let envelope = creator.encrypt(plaintext).unwrap();

        assert_ne!(envelope.ciphertext, plaintext);
        assert_eq!(envelope.counter, 0);

        let decrypted = joiner.decrypt(&envelope).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_counter_increments() {
        let secret = b"test-shared-secret-32-bytes-long";
        let enc = SessionEncryption::from_shared_secret(secret, true).unwrap();

        let e1 = enc.encrypt(b"msg1").unwrap();
        let e2 = enc.encrypt(b"msg2").unwrap();
        let e3 = enc.encrypt(b"msg3").unwrap();

        assert_eq!(e1.counter, 0);
        assert_eq!(e2.counter, 1);
        assert_eq!(e3.counter, 2);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let secret = b"test-shared-secret-32-bytes-long";
        let creator = SessionEncryption::from_shared_secret(secret, true).unwrap();
        let joiner = SessionEncryption::from_shared_secret(secret, false).unwrap();

        let mut envelope = creator.encrypt(b"sensitive data").unwrap();
        // Tamper with ciphertext
        if let Some(byte) = envelope.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }

        let result = joiner.decrypt(&envelope);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let enc_a = SessionEncryption::from_shared_secret(b"secret-aaaaaaaaaaaaaaaaaaaaaaaaaa", true).unwrap();
        let enc_b = SessionEncryption::from_shared_secret(b"secret-bbbbbbbbbbbbbbbbbbbbbbbbbb", false).unwrap();

        let envelope = enc_a.encrypt(b"private message").unwrap();
        let result = enc_b.decrypt(&envelope);
        assert!(result.is_err());
    }

    #[test]
    fn test_direction_specific_keys() {
        // Both sides derive from same secret — but use different keys for send/recv
        let secret = b"test-shared-secret-32-bytes-long";
        let creator = SessionEncryption::from_shared_secret(secret, true).unwrap();
        let joiner = SessionEncryption::from_shared_secret(secret, false).unwrap();

        // Creator → Joiner
        let env1 = creator.encrypt(b"from creator").unwrap();
        assert_eq!(joiner.decrypt(&env1).unwrap(), b"from creator");

        // Joiner → Creator
        let env2 = joiner.encrypt(b"from joiner").unwrap();
        assert_eq!(creator.decrypt(&env2).unwrap(), b"from joiner");

        // Creator cannot decrypt its own message (different key for recv)
        let env3 = creator.encrypt(b"self test").unwrap();
        assert!(creator.decrypt(&env3).is_err(), "Should not decrypt own message");
    }

    #[test]
    fn test_replay_protection() {
        let secret = b"test-shared-secret-32-bytes-long";
        let creator = SessionEncryption::from_shared_secret(secret, true).unwrap();
        let joiner = SessionEncryption::from_shared_secret(secret, false).unwrap();

        let env1 = creator.encrypt(b"msg1").unwrap();
        let env2 = creator.encrypt(b"msg2").unwrap();

        // First decrypt succeeds
        joiner.decrypt(&env1).unwrap();
        // Second decrypt succeeds (counter advances)
        joiner.decrypt(&env2).unwrap();
        // Replaying env1 should fail (counter went backwards)
        assert!(joiner.decrypt(&env1).is_err(), "Replay should be rejected");
        // Replaying env2 should also fail (counter not advancing)
        assert!(joiner.decrypt(&env2).is_err(), "Replay should be rejected");
    }

    #[test]
    fn test_key_rotation() {
        let secret = b"test-shared-secret-32-bytes-long";
        let mut creator = SessionEncryption::from_shared_secret(secret, true).unwrap();
        let mut joiner = SessionEncryption::from_shared_secret(secret, false).unwrap();

        // Encrypt before rotation
        let before = creator.encrypt(b"before rotation").unwrap();
        let before_counter = before.counter;
        joiner.decrypt(&before).unwrap();

        // Rotate both sides
        creator.rotate_key().unwrap();
        joiner.rotate_key().unwrap();

        // Counter should NOT reset — continues monotonically
        let after = creator.encrypt(b"after rotation").unwrap();
        assert!(after.counter > before_counter, "Counter should continue after rotation");

        // New message should decrypt with rotated keys
        joiner.decrypt(&after).unwrap();
    }

    #[test]
    fn test_large_payload() {
        let secret = b"test-shared-secret-32-bytes-long";
        let creator = SessionEncryption::from_shared_secret(secret, true).unwrap();
        let joiner = SessionEncryption::from_shared_secret(secret, false).unwrap();

        // Simulate a large yrs update (100KB)
        let large_payload = vec![0xAB; 100_000];
        let envelope = creator.encrypt(&large_payload).unwrap();
        let decrypted = joiner.decrypt(&envelope).unwrap();
        assert_eq!(decrypted, large_payload);
    }

    #[test]
    fn test_nonce_format() {
        let nonce = counter_to_nonce(0);
        assert_eq!(nonce, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let nonce = counter_to_nonce(1);
        assert_eq!(nonce, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

        let nonce = counter_to_nonce(256);
        assert_eq!(nonce, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0]);
    }
}
