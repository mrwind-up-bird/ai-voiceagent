//! SPAKE2 password-authenticated key exchange for device pairing.
//!
//! Uses the pairing code (e.g. "7-violet-castle") as the SPAKE2 password.
//! Creator runs as side A, joiner as side B. After exchanging one message
//! each, both sides derive an identical shared secret used to initialise
//! `SessionEncryption`.

use spake2::{Ed25519Group, Identity, Password, Spake2};

use crate::sync::encryption::SessionEncryption;

/// Identity string used for the session creator (side A).
const IDENTITY_CREATOR: &[u8] = b"aurus-sync-creator";
/// Identity string used for the session joiner (side B).
const IDENTITY_JOINER: &[u8] = b"aurus-sync-joiner";

// ---------------------------------------------------------------------------
// Creator side (A)
// ---------------------------------------------------------------------------

/// State held by the session creator while waiting for the joiner's message.
pub struct PairingCreator {
    state: Spake2<Ed25519Group>,
    /// The outbound SPAKE2 message to send to the joiner.
    pub outbound_msg: Vec<u8>,
}

impl PairingCreator {
    /// Start the creator side of the SPAKE2 exchange.
    /// `pairing_code` is the human-readable code shown to the user.
    pub fn start(pairing_code: &str) -> Self {
        let (state, outbound_msg) = Spake2::<Ed25519Group>::start_a(
            &Password::new(pairing_code.as_bytes()),
            &Identity::new(IDENTITY_CREATOR),
            &Identity::new(IDENTITY_JOINER),
        );
        Self {
            state,
            outbound_msg,
        }
    }

    /// Finish the exchange with the joiner's inbound message.
    /// Returns a `SessionEncryption` initialised with the derived shared key.
    pub fn finish(self, joiner_msg: &[u8]) -> Result<SessionEncryption, String> {
        let shared_key = self
            .state
            .finish(joiner_msg)
            .map_err(|e| format!("SPAKE2 key exchange failed: {:?}", e))?;

        SessionEncryption::from_shared_secret(&shared_key)
    }
}

// ---------------------------------------------------------------------------
// Joiner side (B)
// ---------------------------------------------------------------------------

/// State held by the session joiner while waiting for the creator's message.
pub struct PairingJoiner {
    state: Spake2<Ed25519Group>,
    /// The outbound SPAKE2 message to send to the creator.
    pub outbound_msg: Vec<u8>,
}

impl PairingJoiner {
    /// Start the joiner side of the SPAKE2 exchange.
    /// `pairing_code` is the code entered by the user.
    pub fn start(pairing_code: &str) -> Self {
        let (state, outbound_msg) = Spake2::<Ed25519Group>::start_b(
            &Password::new(pairing_code.as_bytes()),
            &Identity::new(IDENTITY_CREATOR),
            &Identity::new(IDENTITY_JOINER),
        );
        Self {
            state,
            outbound_msg,
        }
    }

    /// Finish the exchange with the creator's inbound message.
    /// Returns a `SessionEncryption` initialised with the derived shared key.
    pub fn finish(self, creator_msg: &[u8]) -> Result<SessionEncryption, String> {
        let shared_key = self
            .state
            .finish(creator_msg)
            .map_err(|e| format!("SPAKE2 key exchange failed: {:?}", e))?;

        SessionEncryption::from_shared_secret(&shared_key)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_exchange_same_code() {
        let code = "7-violet-castle";

        let creator = PairingCreator::start(code);
        let joiner = PairingJoiner::start(code);

        // Save outbound messages before finish() consumes the structs
        let creator_msg = creator.outbound_msg.clone();
        let joiner_msg = joiner.outbound_msg.clone();

        let creator_enc = creator.finish(&joiner_msg).unwrap();
        let joiner_enc = joiner.finish(&creator_msg).unwrap();

        // Both sides should be able to encrypt/decrypt each other's messages
        let plaintext = b"sync payload test";
        let envelope = creator_enc.encrypt(plaintext).unwrap();
        let decrypted = joiner_enc.decrypt(&envelope).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_pairing_exchange_wrong_code_fails() {
        let creator = PairingCreator::start("7-violet-castle");
        let joiner = PairingJoiner::start("3-amber-forge");

        let creator_msg = creator.outbound_msg.clone();
        let joiner_msg = joiner.outbound_msg.clone();

        // Exchange messages — SPAKE2 will derive different keys
        let creator_enc = creator.finish(&joiner_msg).unwrap();
        let joiner_enc = joiner.finish(&creator_msg).unwrap();

        // Encryption with mismatched keys should fail to decrypt
        let envelope = creator_enc.encrypt(b"secret data").unwrap();
        let result = joiner_enc.decrypt(&envelope);
        assert!(result.is_err(), "Mismatched codes should fail decryption");
    }

    #[test]
    fn test_pairing_bidirectional() {
        let code = "4-jade-summit";

        let creator = PairingCreator::start(code);
        let joiner = PairingJoiner::start(code);

        let creator_msg = creator.outbound_msg.clone();
        let joiner_msg = joiner.outbound_msg.clone();

        let creator_enc = creator.finish(&joiner_msg).unwrap();
        let joiner_enc = joiner.finish(&creator_msg).unwrap();

        // Creator → Joiner
        let env1 = creator_enc.encrypt(b"from creator").unwrap();
        assert_eq!(joiner_enc.decrypt(&env1).unwrap(), b"from creator");

        // Joiner → Creator
        let env2 = joiner_enc.encrypt(b"from joiner").unwrap();
        assert_eq!(creator_enc.decrypt(&env2).unwrap(), b"from joiner");
    }
}
