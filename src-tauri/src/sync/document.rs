//! CRDT document layer using `yrs` (Rust port of Yjs).
//!
//! Maps the Zustand voiceStore structure to a Y.Doc. The document lives
//! entirely in memory — it is never persisted to disk.

use serde::{Deserialize, Serialize};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Any, Doc, Map, MapRef, Out, ReadTxn, Transact, Update, WriteTxn};

// ---------------------------------------------------------------------------
// Well-known map / key names (must match frontend voiceStore fields)
// ---------------------------------------------------------------------------

pub const MAP_SESSION: &str = "session";
pub const MAP_ACTION_ITEMS: &str = "action_items";
pub const MAP_TONE_SHIFT: &str = "tone_shift";
pub const MAP_TRANSLATION: &str = "translation";
pub const MAP_DEV_LOG: &str = "dev_log";
pub const MAP_BRAIN_DUMP: &str = "brain_dump";
pub const MAP_MENTAL_MIRROR: &str = "mental_mirror";
pub const MAP_MUSIC: &str = "music";
pub const MAP_PREFERENCES: &str = "preferences";

pub const KEY_TRANSCRIPT: &str = "transcript";
pub const KEY_RECORDING_STATE: &str = "recording_state";
pub const KEY_RECORDING_DURATION: &str = "recording_duration";
pub const KEY_ACTIVE_AGENT: &str = "active_agent";
pub const KEY_RESULT: &str = "result";

// ---------------------------------------------------------------------------
// SyncDocument
// ---------------------------------------------------------------------------

/// Wraps a `yrs::Doc` with typed helpers for the Aurus data model.
pub struct SyncDocument {
    doc: Doc,
}

impl SyncDocument {
    /// Create a new empty document and pre-initialise all expected maps.
    pub fn new() -> Self {
        let doc = Doc::new();

        // Pre-create top-level maps so they exist when peers connect.
        {
            let mut txn = doc.transact_mut();
            txn.get_or_insert_map(MAP_SESSION);
            txn.get_or_insert_map(MAP_ACTION_ITEMS);
            txn.get_or_insert_map(MAP_TONE_SHIFT);
            txn.get_or_insert_map(MAP_TRANSLATION);
            txn.get_or_insert_map(MAP_DEV_LOG);
            txn.get_or_insert_map(MAP_BRAIN_DUMP);
            txn.get_or_insert_map(MAP_MENTAL_MIRROR);
            txn.get_or_insert_map(MAP_MUSIC);
            txn.get_or_insert_map(MAP_PREFERENCES);
        }

        Self { doc }
    }

    // ----- State vector & update encoding -----------------------------------

    /// Encode the full document state as a binary update vector.
    /// This is sent to a new peer so they can catch up.
    pub fn encode_state_as_update(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_state_as_update_v1(&yrs::StateVector::default())
    }

    /// Encode only the diff since `remote_state_vector`.
    pub fn encode_diff(&self, remote_sv: &[u8]) -> Result<Vec<u8>, String> {
        let sv = yrs::StateVector::decode_v1(remote_sv)
            .map_err(|e| format!("Invalid state vector: {}", e))?;
        let txn = self.doc.transact();
        Ok(txn.encode_state_as_update_v1(&sv))
    }

    /// Encode this document's state vector (so a peer can compute a diff).
    pub fn encode_state_vector(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.state_vector().encode_v1()
    }

    /// Apply a binary update received from a remote peer.
    pub fn apply_update(&self, update: &[u8]) -> Result<(), String> {
        let update =
            Update::decode_v1(update).map_err(|e| format!("Failed to decode update: {}", e))?;
        let mut txn = self.doc.transact_mut();
        txn.apply_update(update)
            .map_err(|e| format!("Failed to apply update: {}", e))
    }

    // ----- Session map helpers ----------------------------------------------

    /// Set the transcript text.
    pub fn set_transcript(&self, text: &str) {
        let mut txn = self.doc.transact_mut();
        let session = txn.get_or_insert_map(MAP_SESSION);
        session.insert(&mut txn, KEY_TRANSCRIPT, text);
    }

    /// Get the current transcript.
    pub fn get_transcript(&self) -> String {
        let mut txn = self.doc.transact_mut();
        let session = txn.get_or_insert_map(MAP_SESSION);
        extract_string(&txn, &session, KEY_TRANSCRIPT).unwrap_or_default()
    }

    /// Set the recording state ("idle", "recording", "processing").
    pub fn set_recording_state(&self, state: &str) {
        let mut txn = self.doc.transact_mut();
        let session = txn.get_or_insert_map(MAP_SESSION);
        session.insert(&mut txn, KEY_RECORDING_STATE, state);
    }

    /// Set the recording duration in seconds.
    pub fn set_recording_duration(&self, duration: f64) {
        let mut txn = self.doc.transact_mut();
        let session = txn.get_or_insert_map(MAP_SESSION);
        session.insert(&mut txn, KEY_RECORDING_DURATION, duration);
    }

    /// Set the active agent name (or empty string for null).
    pub fn set_active_agent(&self, agent: &str) {
        let mut txn = self.doc.transact_mut();
        let session = txn.get_or_insert_map(MAP_SESSION);
        session.insert(&mut txn, KEY_ACTIVE_AGENT, agent);
    }

    // ----- Agent result helpers ---------------------------------------------

    /// Store an agent result as a JSON string in its map.
    pub fn set_agent_result(&self, map_name: &str, result_json: &str) {
        let mut txn = self.doc.transact_mut();
        let map = txn.get_or_insert_map(map_name);
        map.insert(&mut txn, KEY_RESULT, result_json);
    }

    /// Retrieve an agent result JSON string.
    pub fn get_agent_result(&self, map_name: &str) -> Option<String> {
        let mut txn = self.doc.transact_mut();
        let map = txn.get_or_insert_map(map_name);
        extract_string(&txn, &map, KEY_RESULT)
    }

    // ----- Preferences helpers ----------------------------------------------

    /// Set a preference key/value.
    pub fn set_preference(&self, key: &str, value: &str) {
        let mut txn = self.doc.transact_mut();
        let prefs = txn.get_or_insert_map(MAP_PREFERENCES);
        prefs.insert(&mut txn, key, value);
    }

    /// Get a preference value.
    pub fn get_preference(&self, key: &str) -> Option<String> {
        let mut txn = self.doc.transact_mut();
        let prefs = txn.get_or_insert_map(MAP_PREFERENCES);
        extract_string(&txn, &prefs, key)
    }

    // ----- Snapshot (for emitting full state to frontend) --------------------

    /// Export the entire document as a flat JSON object for the frontend.
    /// Used when a remote update arrives — emit this as a Tauri event so
    /// the frontend can reconcile with voiceStore.
    pub fn snapshot(&self) -> SyncSnapshot {
        let mut txn = self.doc.transact_mut();

        let session = txn.get_or_insert_map(MAP_SESSION);
        let transcript = extract_string(&txn, &session, KEY_TRANSCRIPT).unwrap_or_default();
        let recording_state = extract_string(&txn, &session, KEY_RECORDING_STATE)
            .unwrap_or_else(|| "idle".to_string());
        let active_agent = extract_string(&txn, &session, KEY_ACTIVE_AGENT);

        let action_items = {
            let m = txn.get_or_insert_map(MAP_ACTION_ITEMS);
            extract_string(&txn, &m, KEY_RESULT)
        };
        let tone_shift = {
            let m = txn.get_or_insert_map(MAP_TONE_SHIFT);
            extract_string(&txn, &m, KEY_RESULT)
        };
        let translation = {
            let m = txn.get_or_insert_map(MAP_TRANSLATION);
            extract_string(&txn, &m, KEY_RESULT)
        };
        let dev_log = {
            let m = txn.get_or_insert_map(MAP_DEV_LOG);
            extract_string(&txn, &m, KEY_RESULT)
        };
        let brain_dump = {
            let m = txn.get_or_insert_map(MAP_BRAIN_DUMP);
            extract_string(&txn, &m, KEY_RESULT)
        };
        let mental_mirror = {
            let m = txn.get_or_insert_map(MAP_MENTAL_MIRROR);
            extract_string(&txn, &m, KEY_RESULT)
        };
        let music = {
            let m = txn.get_or_insert_map(MAP_MUSIC);
            extract_string(&txn, &m, KEY_RESULT)
        };

        SyncSnapshot {
            transcript,
            recording_state,
            active_agent,
            action_items,
            tone_shift,
            translation,
            dev_log,
            brain_dump,
            mental_mirror,
            music,
        }
    }
}

// ---------------------------------------------------------------------------
// Value extraction helpers
// ---------------------------------------------------------------------------

/// Extract a String value from a MapRef key.
fn extract_string<T: ReadTxn>(txn: &T, map: &MapRef, key: &str) -> Option<String> {
    match map.get(txn, key)? {
        Out::Any(Any::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Snapshot type (emitted to frontend)
// ---------------------------------------------------------------------------

/// Full state snapshot for the frontend to reconcile with voiceStore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshot {
    pub transcript: String,
    pub recording_state: String,
    pub active_agent: Option<String>,
    pub action_items: Option<String>,
    pub tone_shift: Option<String>,
    pub translation: Option<String>,
    pub dev_log: Option<String>,
    pub brain_dump: Option<String>,
    pub mental_mirror: Option<String>,
    pub music: Option<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document_has_maps() {
        let doc = SyncDocument::new();
        // Verify maps were created by trying to read from them
        assert_eq!(doc.get_transcript(), "");
        assert!(doc.get_agent_result(MAP_ACTION_ITEMS).is_none());
    }

    #[test]
    fn test_transcript_round_trip() {
        let doc = SyncDocument::new();
        doc.set_transcript("Hello, world!");
        assert_eq!(doc.get_transcript(), "Hello, world!");
    }

    #[test]
    fn test_agent_result_round_trip() {
        let doc = SyncDocument::new();
        let json = r#"{"items":[{"task":"Buy milk","priority":"low"}]}"#;
        doc.set_agent_result(MAP_ACTION_ITEMS, json);
        assert_eq!(doc.get_agent_result(MAP_ACTION_ITEMS).unwrap(), json);
    }

    #[test]
    fn test_preference_round_trip() {
        let doc = SyncDocument::new();
        doc.set_preference("selectedTone", "professional");
        assert_eq!(
            doc.get_preference("selectedTone").unwrap(),
            "professional"
        );
    }

    #[test]
    fn test_state_sync_between_docs() {
        let doc_a = SyncDocument::new();
        let doc_b = SyncDocument::new();

        // Write on doc A
        doc_a.set_transcript("Meeting notes from today");
        doc_a.set_active_agent("action-items");
        doc_a.set_agent_result(MAP_ACTION_ITEMS, r#"{"items":[]}"#);

        // Sync A → B
        let update = doc_a.encode_state_as_update();
        doc_b.apply_update(&update).unwrap();

        // Verify B has A's data
        assert_eq!(doc_b.get_transcript(), "Meeting notes from today");
        assert_eq!(
            doc_b.get_agent_result(MAP_ACTION_ITEMS).unwrap(),
            r#"{"items":[]}"#
        );
    }

    #[test]
    fn test_diff_sync() {
        let doc_a = SyncDocument::new();
        let doc_b = SyncDocument::new();

        // Initial sync
        doc_a.set_transcript("Hello");
        let full_update = doc_a.encode_state_as_update();
        doc_b.apply_update(&full_update).unwrap();

        // Now do incremental update
        doc_a.set_transcript("Hello, updated");
        let sv_b = doc_b.encode_state_vector();
        let diff = doc_a.encode_diff(&sv_b).unwrap();
        doc_b.apply_update(&diff).unwrap();

        assert_eq!(doc_b.get_transcript(), "Hello, updated");
    }

    #[test]
    fn test_snapshot() {
        let doc = SyncDocument::new();
        doc.set_transcript("Test transcript");
        doc.set_recording_state("recording");
        doc.set_active_agent("tone-shifter");

        let snap = doc.snapshot();
        assert_eq!(snap.transcript, "Test transcript");
        assert_eq!(snap.recording_state, "recording");
        assert_eq!(snap.active_agent, Some("tone-shifter".to_string()));
    }
}
