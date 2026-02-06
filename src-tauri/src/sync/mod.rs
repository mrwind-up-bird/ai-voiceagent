pub mod document;
pub mod encryption;

use std::fmt;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::sync::document::SyncDocument;
use crate::sync::encryption::SessionEncryption;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Connection status exposed to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Disconnected,
    WaitingForPeer,
    Connecting,
    Connected,
}

/// Information about a connected peer device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub device_id: String,
    pub device_name: String,
    pub connected_at: i64,
}

/// Payload emitted to the frontend on status changes.
#[derive(Debug, Clone, Serialize)]
pub struct SyncStatusEvent {
    pub status: SyncStatus,
    pub session_id: Option<String>,
    pub pairing_code: Option<String>,
    pub peer: Option<PeerInfo>,
}

/// Payload emitted when sync state is updated from a remote peer.
#[derive(Debug, Clone, Serialize)]
pub struct SyncStateUpdateEvent {
    pub source: String, // "remote"
    pub update_type: String, // e.g. "transcript", "agent_result", "full_state"
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum SyncError {
    NotConnected,
    AlreadyInSession,
    SessionNotFound,
    PeerNotFound,
    EncryptionFailed(String),
    DocumentError(String),
    InvalidPairingCode,
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => write!(f, "Not connected to any sync session"),
            Self::AlreadyInSession => write!(f, "Already in an active sync session"),
            Self::SessionNotFound => write!(f, "Sync session not found"),
            Self::PeerNotFound => write!(f, "Peer device not found"),
            Self::EncryptionFailed(msg) => write!(f, "Encryption error: {}", msg),
            Self::DocumentError(msg) => write!(f, "Document error: {}", msg),
            Self::InvalidPairingCode => write!(f, "Invalid pairing code"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<SyncError> for String {
    fn from(err: SyncError) -> Self {
        err.to_string()
    }
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// All sync state lives here — entirely in-memory, never persisted.
/// When this struct is dropped, all session data is gone.
pub struct SyncState {
    /// This device's unique (ephemeral) identifier.
    pub device_id: String,
    /// Human-readable device name (e.g. "Oliver's MacBook").
    pub device_name: String,
    /// Active session identifier. None if not in a session.
    pub session_id: Option<String>,
    /// Current connection status.
    pub status: SyncStatus,
    /// Pairing code shown to the user (creator only).
    pub pairing_code: Option<String>,
    /// Connected peer, if any. (Phase 5a supports 1:1 sync.)
    pub peer: Option<PeerInfo>,
    /// The CRDT document holding all synced state.
    pub doc: SyncDocument,
    /// Session encryption layer (initialised after pairing).
    pub encryption: Option<SessionEncryption>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            device_id: Uuid::new_v4().to_string(),
            device_name: Self::default_device_name(),
            session_id: None,
            status: SyncStatus::Disconnected,
            pairing_code: None,
            peer: None,
            doc: SyncDocument::new(),
            encryption: None,
        }
    }
}

impl Drop for SyncState {
    fn drop(&mut self) {
        // Zero sensitive material on drop — defence in depth.
        self.pairing_code = None;
        self.encryption = None;
        tracing::info!("SyncState dropped — all session data wiped from memory");
    }
}

impl SyncState {
    fn default_device_name() -> String {
        #[cfg(target_os = "macos")]
        { "Mac".to_string() }
        #[cfg(target_os = "windows")]
        { "Windows PC".to_string() }
        #[cfg(target_os = "linux")]
        { "Linux".to_string() }
        #[cfg(target_os = "ios")]
        { "iPhone".to_string() }
        #[cfg(target_os = "android")]
        { "Android".to_string() }
    }

    /// Build a status event payload for the frontend.
    fn status_event(&self) -> SyncStatusEvent {
        SyncStatusEvent {
            status: self.status.clone(),
            session_id: self.session_id.clone(),
            pairing_code: self.pairing_code.clone(),
            peer: self.peer.clone(),
        }
    }

    /// Tear down the session — wipes doc, keys, and peer info.
    fn reset_session(&mut self) {
        self.session_id = None;
        self.pairing_code = None;
        self.peer = None;
        self.encryption = None;
        self.doc = SyncDocument::new();
        self.status = SyncStatus::Disconnected;
    }
}

pub type SyncManager = Arc<Mutex<SyncState>>;

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

/// Create a new sync session. Returns a human-readable pairing code.
#[tauri::command]
pub async fn create_sync_session(
    app: AppHandle,
    state: tauri::State<'_, SyncManager>,
) -> Result<String, String> {
    let mut s = state.lock().await;

    if s.status != SyncStatus::Disconnected {
        return Err(SyncError::AlreadyInSession.into());
    }

    let session_id = Uuid::new_v4().to_string();
    let pairing_code = generate_pairing_code();

    s.session_id = Some(session_id.clone());
    s.pairing_code = Some(pairing_code.clone());
    s.status = SyncStatus::WaitingForPeer;
    s.doc = SyncDocument::new();

    let event = s.status_event();
    drop(s); // release lock before emitting

    app.emit("sync-status-changed", &event).map_err(|e| e.to_string())?;
    tracing::info!("Sync session created: {}", session_id);

    Ok(pairing_code)
}

/// Join an existing sync session using a pairing code.
#[tauri::command]
pub async fn join_sync_session(
    app: AppHandle,
    pairing_code: String,
    state: tauri::State<'_, SyncManager>,
) -> Result<(), String> {
    let mut s = state.lock().await;

    if s.status != SyncStatus::Disconnected {
        return Err(SyncError::AlreadyInSession.into());
    }

    if pairing_code.trim().is_empty() {
        return Err(SyncError::InvalidPairingCode.into());
    }

    s.session_id = Some(Uuid::new_v4().to_string());
    s.status = SyncStatus::Connecting;
    s.doc = SyncDocument::new();

    let event = s.status_event();
    drop(s);

    app.emit("sync-status-changed", &event).map_err(|e| e.to_string())?;
    tracing::info!("Joining sync session with code: {}", pairing_code);

    // Actual connection logic comes in Phase 5b (transport layer).
    Ok(())
}

/// Leave the current sync session and wipe all session data.
#[tauri::command]
pub async fn leave_sync_session(
    app: AppHandle,
    state: tauri::State<'_, SyncManager>,
) -> Result<(), String> {
    let mut s = state.lock().await;

    if s.status == SyncStatus::Disconnected {
        return Ok(()); // Already disconnected
    }

    tracing::info!("Leaving sync session: {:?}", s.session_id);
    s.reset_session();

    let event = s.status_event();
    drop(s);

    app.emit("sync-status-changed", &event).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get current sync status.
#[tauri::command]
pub async fn get_sync_status(
    state: tauri::State<'_, SyncManager>,
) -> Result<SyncStatusEvent, String> {
    let s = state.lock().await;
    Ok(s.status_event())
}

/// Get the current pairing code (if session creator).
#[tauri::command]
pub async fn get_pairing_code(
    state: tauri::State<'_, SyncManager>,
) -> Result<Option<String>, String> {
    let s = state.lock().await;
    Ok(s.pairing_code.clone())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a human-readable pairing code: "digit-word-word" (e.g. "7-violet-castle").
fn generate_pairing_code() -> String {
    use rand::Rng;

    const ADJECTIVES: &[&str] = &[
        "amber", "azure", "coral", "crimson", "golden", "ivory",
        "jade", "lemon", "lilac", "olive", "peach", "plum",
        "rose", "ruby", "sage", "silver", "teal", "violet",
    ];
    const NOUNS: &[&str] = &[
        "arrow", "badge", "candle", "castle", "cliff", "crown",
        "delta", "ember", "falcon", "forge", "harbor", "lantern",
        "maple", "nexus", "orbit", "prism", "quartz", "ridge",
        "spark", "storm", "summit", "torch", "vault", "zenith",
    ];

    let mut rng = rand::thread_rng();
    let digit = rng.gen_range(2..10); // 2-9 (avoid 0/1 ambiguity)
    let adj = ADJECTIVES[rng.gen_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.gen_range(0..NOUNS.len())];

    format!("{}-{}-{}", digit, adj, noun)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_code_format() {
        let code = generate_pairing_code();
        let parts: Vec<&str> = code.split('-').collect();
        assert_eq!(parts.len(), 3, "code should be digit-word-word");
        let digit: u8 = parts[0].parse().expect("first part should be a digit");
        assert!(digit >= 2 && digit <= 9);
        assert!(!parts[1].is_empty());
        assert!(!parts[2].is_empty());
    }

    #[test]
    fn test_sync_state_default() {
        let state = SyncState::default();
        assert_eq!(state.status, SyncStatus::Disconnected);
        assert!(state.session_id.is_none());
        assert!(state.pairing_code.is_none());
        assert!(state.peer.is_none());
        assert!(state.encryption.is_none());
        assert!(!state.device_id.is_empty());
    }

    #[test]
    fn test_sync_state_reset() {
        let mut state = SyncState::default();
        state.session_id = Some("test-session".to_string());
        state.pairing_code = Some("3-azure-prism".to_string());
        state.status = SyncStatus::Connected;

        state.reset_session();

        assert_eq!(state.status, SyncStatus::Disconnected);
        assert!(state.session_id.is_none());
        assert!(state.pairing_code.is_none());
    }
}
