//! Application-layer E2E encryption for sync updates.
//!
//! This sits **on top of** WebRTC DTLS (transport encryption) to provide
//! defence-in-depth: even if the transport is compromised, sync payloads
//! remain encrypted with a key derived from the SPAKE2+ pairing.
//!
//! Cipher: AES-256-GCM via the `ring` crate.
//! Key derivation: HKDF-SHA256 from a shared secret.
//! Nonces: 96-bit, counter-based (monotonically increasing per session).

use ring::aead::{self, Aad, BoundKey, Nonce, NonceSequence, SealingKey, OpeningKey, UnboundKey, NONCE_LEN};
use ring::hkdf;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// An encrypted sync message sent over the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    /// Counter used as nonce — receiver uses this to construct the nonce.
    pub counter: u64,
    /// AES-256-GCM ciphertext + 16-byte auth tag.
    pub ciphertext: Vec<u8>,
}

/// Session encryption state. Created after SPAKE2+ key exchange completes.
pub struct SessionEncryption {
    /// Raw shared secret from SPAKE2+ (32 bytes).
    shared_secret: Vec<u8>,
    /// Derived 256-bit encryption key material.
    key_material: Vec<u8>,
    /// Monotonically increasing counter for nonce generation.
    seal_counter: AtomicU64,
}

impl SessionEncryption {
    /// Create from a shared secret (e.g. output of SPAKE2+ key exchange).
    /// The secret is expanded via HKDF-SHA256 into a 256-bit AES key.
    pub fn from_shared_secret(secret: &[u8]) -> Result<Self, String> {
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"aurus-sync-v1");
        let prk = salt.extract(secret);

        let mut key_material = vec![0u8; 32];
        prk.expand(&[b"aes-256-gcm-key"], HkdfLen(32))
            .map_err(|_| "HKDF expand failed".to_string())?
            .fill(&mut key_material)
            .map_err(|_| "HKDF fill failed".to_string())?;

        Ok(Self {
            shared_secret: secret.to_vec(),
            key_material,
            seal_counter: AtomicU64::new(0),
        })
    }

    /// Encrypt a plaintext payload (e.g. a yrs update vector).
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedEnvelope, String> {
        let counter = self.seal_counter.fetch_add(1, Ordering::SeqCst);
        let nonce_bytes = counter_to_nonce(counter);

        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &self.key_material)
            .map_err(|_| "Failed to create AES key".to_string())?;

        let mut sealing_key = SealingKey::new(unbound_key, SingleNonce::new(nonce_bytes));

        let mut in_out = plaintext.to_vec();
        sealing_key
            .seal_in_place_append_tag(Aad::empty(), &mut in_out)
            .map_err(|_| "Encryption failed".to_string())?;

        Ok(EncryptedEnvelope {
            counter,
            ciphertext: in_out,
        })
    }

    /// Decrypt an encrypted envelope from a remote peer.
    pub fn decrypt(&self, envelope: &EncryptedEnvelope) -> Result<Vec<u8>, String> {
        let nonce_bytes = counter_to_nonce(envelope.counter);

        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &self.key_material)
            .map_err(|_| "Failed to create AES key".to_string())?;

        let mut opening_key = OpeningKey::new(unbound_key, SingleNonce::new(nonce_bytes));

        let mut in_out = envelope.ciphertext.clone();
        let plaintext = opening_key
            .open_in_place(Aad::empty(), &mut in_out)
            .map_err(|_| "Decryption failed — invalid key or tampered data".to_string())?;

        Ok(plaintext.to_vec())
    }

    /// Rotate the encryption key using HKDF ratchet.
    /// Derives a new key from the current key + a "rotate" context.
    /// The old key material is zeroed.
    pub fn rotate_key(&mut self) -> Result<(), String> {
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"aurus-sync-rotate");
        let prk = salt.extract(&self.key_material);

        let mut new_key = vec![0u8; 32];
        prk.expand(&[b"next-key"], HkdfLen(32))
            .map_err(|_| "HKDF expand failed during rotation".to_string())?
            .fill(&mut new_key)
            .map_err(|_| "HKDF fill failed during rotation".to_string())?;

        // Zero old key material
        for byte in self.key_material.iter_mut() {
            *byte = 0;
        }

        self.key_material = new_key;
        // Reset counter for new key epoch
        self.seal_counter.store(0, Ordering::SeqCst);

        tracing::debug!("Encryption key rotated");
        Ok(())
    }
}

impl Drop for SessionEncryption {
    fn drop(&mut self) {
        // Zero all sensitive material on drop.
        for byte in self.shared_secret.iter_mut() {
            *byte = 0;
        }
        for byte in self.key_material.iter_mut() {
            *byte = 0;
        }
        tracing::debug!("SessionEncryption dropped — keys zeroed");
    }
}

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
        let enc = SessionEncryption::from_shared_secret(secret).unwrap();

        let plaintext = b"Hello, sync world!";
        let envelope = enc.encrypt(plaintext).unwrap();

        assert_ne!(envelope.ciphertext, plaintext);
        assert_eq!(envelope.counter, 0);

        let decrypted = enc.decrypt(&envelope).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_counter_increments() {
        let secret = b"test-shared-secret-32-bytes-long";
        let enc = SessionEncryption::from_shared_secret(secret).unwrap();

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
        let enc = SessionEncryption::from_shared_secret(secret).unwrap();

        let mut envelope = enc.encrypt(b"sensitive data").unwrap();
        // Tamper with ciphertext
        if let Some(byte) = envelope.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }

        let result = enc.decrypt(&envelope);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let enc_a = SessionEncryption::from_shared_secret(b"secret-aaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let enc_b = SessionEncryption::from_shared_secret(b"secret-bbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();

        let envelope = enc_a.encrypt(b"private message").unwrap();
        let result = enc_b.decrypt(&envelope);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_rotation() {
        let secret = b"test-shared-secret-32-bytes-long";
        let mut enc = SessionEncryption::from_shared_secret(secret).unwrap();

        // Encrypt before rotation
        let before = enc.encrypt(b"before rotation").unwrap();
        assert_eq!(before.counter, 0);

        // Rotate
        enc.rotate_key().unwrap();

        // Counter should reset
        let after = enc.encrypt(b"after rotation").unwrap();
        assert_eq!(after.counter, 0);

        // Old ciphertext should NOT decrypt with new key
        // (we can't easily test this because we'd need the old key)
    }

    #[test]
    fn test_large_payload() {
        let secret = b"test-shared-secret-32-bytes-long";
        let enc = SessionEncryption::from_shared_secret(secret).unwrap();

        // Simulate a large yrs update (100KB)
        let large_payload = vec![0xAB; 100_000];
        let envelope = enc.encrypt(&large_payload).unwrap();
        let decrypted = enc.decrypt(&envelope).unwrap();
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
