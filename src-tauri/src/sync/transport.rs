//! Transport layer for sync: sends/receives encrypted yrs update vectors
//! between paired devices.
//!
//! Phase 5b implements `LocalTransport` — a direct WebSocket connection
//! between devices on the same local network. The creator runs a WS server,
//! the joiner connects as a client.
//!
//! Phase 5c will add `WebRtcTransport` for cross-network sync.

use async_tungstenite::tokio::{accept_async, connect_async};
use async_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::sync::document::SyncDocument;
use crate::sync::encryption::{EncryptedEnvelope, SessionEncryption};
use crate::sync::pairing::{PairingCreator, PairingJoiner};

// ---------------------------------------------------------------------------
// Wire protocol messages
// ---------------------------------------------------------------------------

/// Messages sent over the WebSocket between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SyncMessage {
    /// SPAKE2 key exchange message (during pairing).
    #[serde(rename = "spake2")]
    Spake2 { payload: Vec<u8> },
    /// Encrypted yrs document update.
    #[serde(rename = "update")]
    Update { envelope: EncryptedEnvelope },
    /// Device identification (sent encrypted after pairing).
    #[serde(rename = "device_info")]
    DeviceInfo {
        device_id: String,
        device_name: String,
    },
    /// Request the peer's full state vector.
    #[serde(rename = "state_vector_request")]
    StateVectorRequest,
    /// Response with state vector for diff computation.
    #[serde(rename = "state_vector")]
    StateVector { sv: Vec<u8> },
    /// Heartbeat / keepalive.
    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: i64 },
    /// Session terminated by peer.
    #[serde(rename = "goodbye")]
    Goodbye,
}

// ---------------------------------------------------------------------------
// Transport handle
// ---------------------------------------------------------------------------

/// Handle for sending plaintext updates through an active transport connection.
/// The transport encrypts internally before putting data on the wire.
/// Cloneable — safe to pass into multiple tasks.
#[derive(Clone)]
pub struct TransportHandle {
    update_tx: mpsc::Sender<Vec<u8>>,
}

impl TransportHandle {
    /// Send a plaintext yrs update to the peer (encrypted by the transport).
    pub async fn send_update(&self, update: &[u8]) -> Result<(), String> {
        self.update_tx
            .send(update.to_vec())
            .await
            .map_err(|_| "Transport channel closed".to_string())
    }
}

// ---------------------------------------------------------------------------
// Creator: Start a WebSocket server and wait for joiner
// ---------------------------------------------------------------------------

/// Start a local WebSocket server on a random port.
/// Returns the bound port and a handle for sending updates.
///
/// Flow:
/// 1. Bind TCP listener on random port
/// 2. Wait for joiner to connect
/// 3. Perform SPAKE2 key exchange
/// 4. Exchange device info
/// 5. Enter sync loop: send/receive encrypted yrs updates
pub async fn start_creator_transport(
    app: AppHandle,
    pairing_code: String,
    doc: Arc<Mutex<SyncDocument>>,
    sync_state: super::SyncManager,
) -> Result<(u16, TransportHandle), String> {
    let listener = TcpListener::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind TCP listener: {}", e))?;

    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?
        .port();

    let (update_tx, update_rx) = mpsc::channel::<Vec<u8>>(64);
    let handle = TransportHandle { update_tx };

    tokio::spawn(async move {
        tracing::info!("Sync transport: listening on port {}", port);

        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                tracing::info!("Sync transport: peer connected from {}", peer_addr);
                if let Err(e) = handle_creator_connection(
                    app,
                    stream,
                    pairing_code,
                    doc,
                    update_rx,
                    sync_state,
                )
                .await
                {
                    tracing::error!("Sync transport error: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Sync transport: accept failed: {}", e);
            }
        }
    });

    Ok((port, handle))
}

/// Handle the WebSocket connection as the creator (server side).
async fn handle_creator_connection(
    app: AppHandle,
    stream: tokio::net::TcpStream,
    pairing_code: String,
    doc: Arc<Mutex<SyncDocument>>,
    update_rx: mpsc::Receiver<Vec<u8>>,
    sync_state: super::SyncManager,
) -> Result<(), String> {
    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| format!("WebSocket accept failed: {}", e))?;

    let (mut ws_write, mut ws_read) = ws_stream.split();

    // --- SPAKE2 Key Exchange ---
    let creator = PairingCreator::start(&pairing_code);
    let spake_msg = SyncMessage::Spake2 {
        payload: creator.outbound_msg.clone(),
    };
    let json = serde_json::to_string(&spake_msg).map_err(|e| e.to_string())?;
    ws_write
        .send(Message::Text(json))
        .await
        .map_err(|e| format!("Failed to send SPAKE2 message: {}", e))?;

    let joiner_spake = ws_read
        .next()
        .await
        .ok_or("Connection closed before SPAKE2 exchange")?
        .map_err(|e| format!("Failed to receive SPAKE2 message: {}", e))?;
    let joiner_payload = extract_spake2_payload(joiner_spake)?;
    let encryption = Arc::new(creator.finish(&joiner_payload)?);
    tracing::info!("Sync transport: SPAKE2 key exchange complete (creator)");

    // --- Exchange device info ---
    let (device_id, device_name) = {
        let s = sync_state.lock().await;
        (s.device_id.clone(), s.device_name.clone())
    };
    send_encrypted_device_info(&mut ws_write, &encryption, &device_id, &device_name).await?;
    let peer_info = receive_encrypted_device_info(&mut ws_read, &encryption).await?;

    // Update state to Connected
    {
        let mut s = sync_state.lock().await;
        s.status = super::SyncStatus::Connected;
        s.peer = Some(peer_info);
        let event = s.status_event();
        drop(s);
        app.emit("sync-status-changed", &event).ok();
    }

    // --- Send initial state to joiner ---
    {
        let doc_guard = doc.lock().await;
        let full_update = doc_guard.encode_state_as_update();
        let envelope = encryption.encrypt(&full_update)?;
        let msg = SyncMessage::Update { envelope };
        let json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
        ws_write
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Failed to send initial state: {}", e))?;
    }

    // --- Sync loop ---
    let result = run_sync_loop(
        app.clone(),
        ws_write,
        ws_read,
        update_rx,
        encryption,
        doc,
    )
    .await;

    // Clean up state on disconnect (only if not already disconnected by leave command)
    {
        let mut s = sync_state.lock().await;
        if s.status != super::SyncStatus::Disconnected {
            s.transport = None;
            s.reset_session();
            let event = s.status_event();
            drop(s);
            app.emit("sync-status-changed", &event).ok();
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Joiner: Connect to a creator's WebSocket server
// ---------------------------------------------------------------------------

/// Connect to a creator's sync server and perform pairing.
pub async fn start_joiner_transport(
    app: AppHandle,
    address: String,
    port: u16,
    pairing_code: String,
    doc: Arc<Mutex<SyncDocument>>,
    sync_state: super::SyncManager,
) -> Result<TransportHandle, String> {
    let url = format!("ws://{}:{}", address, port);

    let (ws_stream, _) = connect_async(&url)
        .await
        .map_err(|e| format!("Failed to connect to creator: {}", e))?;

    let (mut ws_write, mut ws_read) = ws_stream.split();

    // --- SPAKE2 Key Exchange ---
    let joiner = PairingJoiner::start(&pairing_code);

    let creator_spake = ws_read
        .next()
        .await
        .ok_or("Connection closed before SPAKE2 exchange")?
        .map_err(|e| format!("Failed to receive SPAKE2 message: {}", e))?;
    let creator_payload = extract_spake2_payload(creator_spake)?;

    let spake_msg = SyncMessage::Spake2 {
        payload: joiner.outbound_msg.clone(),
    };
    let json = serde_json::to_string(&spake_msg).map_err(|e| e.to_string())?;
    ws_write
        .send(Message::Text(json))
        .await
        .map_err(|e| format!("Failed to send SPAKE2 message: {}", e))?;

    let encryption = Arc::new(joiner.finish(&creator_payload)?);
    tracing::info!("Sync transport: SPAKE2 key exchange complete (joiner)");

    // --- Exchange device info (joiner receives first, then sends) ---
    let peer_info = receive_encrypted_device_info(&mut ws_read, &encryption).await?;
    let (device_id, device_name) = {
        let s = sync_state.lock().await;
        (s.device_id.clone(), s.device_name.clone())
    };
    send_encrypted_device_info(&mut ws_write, &encryption, &device_id, &device_name).await?;

    // Update state to Connected
    {
        let mut s = sync_state.lock().await;
        s.status = super::SyncStatus::Connected;
        s.peer = Some(peer_info);
        let event = s.status_event();
        drop(s);
        app.emit("sync-status-changed", &event).ok();
    }

    // --- Sync loop ---
    let (update_tx, update_rx) = mpsc::channel::<Vec<u8>>(64);
    let handle = TransportHandle { update_tx };

    let sync_state_clone = sync_state.clone();
    tokio::spawn(async move {
        let result = run_sync_loop(
            app.clone(),
            ws_write,
            ws_read,
            update_rx,
            encryption,
            doc,
        )
        .await;

        if let Err(ref e) = result {
            tracing::error!("Sync transport error (joiner): {}", e);
        }

        // Clean up state on disconnect
        {
            let mut s = sync_state_clone.lock().await;
            if s.status != super::SyncStatus::Disconnected {
                s.transport = None;
                s.reset_session();
                let event = s.status_event();
                drop(s);
                app.emit("sync-status-changed", &event).ok();
            }
        }
    });

    Ok(handle)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a SPAKE2 payload from a WebSocket message.
fn extract_spake2_payload(msg: Message) -> Result<Vec<u8>, String> {
    match msg {
        Message::Text(text) => {
            let msg: SyncMessage =
                serde_json::from_str(&text).map_err(|e| format!("Invalid SPAKE2 message: {}", e))?;
            match msg {
                SyncMessage::Spake2 { payload } => Ok(payload),
                _ => Err("Expected SPAKE2 message".to_string()),
            }
        }
        _ => Err("Expected text message for SPAKE2".to_string()),
    }
}

/// Send encrypted device info to the peer.
async fn send_encrypted_device_info<S>(
    ws_write: &mut S,
    encryption: &SessionEncryption,
    device_id: &str,
    device_name: &str,
) -> Result<(), String>
where
    S: futures_util::Sink<Message, Error = async_tungstenite::tungstenite::Error> + Unpin,
{
    let info = SyncMessage::DeviceInfo {
        device_id: device_id.to_string(),
        device_name: device_name.to_string(),
    };
    let info_json = serde_json::to_string(&info).map_err(|e| e.to_string())?;
    let envelope = encryption.encrypt(info_json.as_bytes())?;
    let msg = SyncMessage::Update { envelope };
    let json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    ws_write
        .send(Message::Text(json))
        .await
        .map_err(|e| format!("Failed to send device info: {}", e))
}

/// Receive and decrypt device info from the peer.
async fn receive_encrypted_device_info<R>(
    ws_read: &mut R,
    encryption: &SessionEncryption,
) -> Result<super::PeerInfo, String>
where
    R: futures_util::Stream<Item = Result<Message, async_tungstenite::tungstenite::Error>> + Unpin,
{
    let msg = ws_read
        .next()
        .await
        .ok_or("Connection closed before device info exchange")?
        .map_err(|e| format!("Failed to receive device info: {}", e))?;

    match msg {
        Message::Text(text) => {
            let sync_msg: SyncMessage = serde_json::from_str(&text)
                .map_err(|e| format!("Invalid device info message: {}", e))?;
            match sync_msg {
                SyncMessage::Update { envelope } => {
                    let decrypted = encryption.decrypt(&envelope)?;
                    let info: SyncMessage = serde_json::from_slice(&decrypted)
                        .map_err(|e| format!("Invalid device info payload: {}", e))?;
                    match info {
                        SyncMessage::DeviceInfo {
                            device_id,
                            device_name,
                        } => Ok(super::PeerInfo {
                            device_id,
                            device_name,
                            connected_at: chrono::Utc::now().timestamp(),
                        }),
                        _ => Err("Expected DeviceInfo message".to_string()),
                    }
                }
                _ => Err("Expected encrypted DeviceInfo".to_string()),
            }
        }
        _ => Err("Expected text message for device info".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Shared sync loop
// ---------------------------------------------------------------------------

/// The main sync loop shared by both creator and joiner.
/// Receives plaintext updates from the local app via `outbound_rx`,
/// encrypts them, and sends over the WebSocket. Incoming encrypted
/// updates are decrypted and applied to the local CRDT document.
async fn run_sync_loop<S, R>(
    app: AppHandle,
    mut ws_write: S,
    mut ws_read: R,
    mut outbound_rx: mpsc::Receiver<Vec<u8>>,
    encryption: Arc<SessionEncryption>,
    doc: Arc<Mutex<SyncDocument>>,
) -> Result<(), String>
where
    S: futures_util::Sink<Message, Error = async_tungstenite::tungstenite::Error> + Unpin,
    R: futures_util::Stream<Item = Result<Message, async_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        tokio::select! {
            // Inbound: message from remote peer
            incoming = ws_read.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<SyncMessage>(&text) {
                            Ok(SyncMessage::Update { envelope }) => {
                                match encryption.decrypt(&envelope) {
                                    Ok(update_bytes) => {
                                        let doc_guard = doc.lock().await;
                                        if let Err(e) = doc_guard.apply_update(&update_bytes) {
                                            tracing::warn!("Failed to apply sync update: {}", e);
                                        } else {
                                            let snapshot = doc_guard.snapshot();
                                            app.emit("sync-state-updated", &snapshot).ok();
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to decrypt sync update: {}", e);
                                    }
                                }
                            }
                            Ok(SyncMessage::Heartbeat { .. }) => {
                                // Peer is alive — nothing to do
                            }
                            Ok(SyncMessage::Goodbye) => {
                                tracing::info!("Peer disconnected gracefully");
                                break;
                            }
                            Ok(_) => {
                                // Ignore unexpected message types in sync loop
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse sync message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!("Sync transport: connection closed");
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("Sync transport read error: {}", e);
                        break;
                    }
                    _ => {} // Ignore ping/pong/binary
                }
            }

            // Outbound: plaintext update from local application
            outbound = outbound_rx.recv() => {
                match outbound {
                    Some(plaintext) => {
                        match encryption.encrypt(&plaintext) {
                            Ok(envelope) => {
                                let msg = SyncMessage::Update { envelope };
                                let json = serde_json::to_string(&msg)
                                    .map_err(|e| format!("Failed to serialize: {}", e))?;
                                if ws_write.send(Message::Text(json)).await.is_err() {
                                    tracing::error!("Sync transport: failed to send update");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Sync transport: encryption failed: {}", e);
                            }
                        }
                    }
                    None => {
                        // Channel closed (TransportHandle dropped) — send goodbye
                        let goodbye = serde_json::to_string(&SyncMessage::Goodbye).unwrap_or_default();
                        let _ = ws_write.send(Message::Text(goodbye)).await;
                        break;
                    }
                }
            }
        }
    }

    app.emit("sync-disconnected", ()).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_message_serialization() {
        let msg = SyncMessage::Heartbeat {
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("heartbeat"));

        let parsed: SyncMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SyncMessage::Heartbeat { timestamp } => assert_eq!(timestamp, 1234567890),
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[test]
    fn test_sync_message_spake2_serialization() {
        let msg = SyncMessage::Spake2 {
            payload: vec![1, 2, 3, 4],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SyncMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SyncMessage::Spake2 { payload } => assert_eq!(payload, vec![1, 2, 3, 4]),
            _ => panic!("Expected Spake2"),
        }
    }

    #[test]
    fn test_goodbye_serialization() {
        let msg = SyncMessage::Goodbye;
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SyncMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, SyncMessage::Goodbye));
    }

    #[test]
    fn test_device_info_serialization() {
        let msg = SyncMessage::DeviceInfo {
            device_id: "test-id".to_string(),
            device_name: "Test Device".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SyncMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SyncMessage::DeviceInfo {
                device_id,
                device_name,
            } => {
                assert_eq!(device_id, "test-id");
                assert_eq!(device_name, "Test Device");
            }
            _ => panic!("Expected DeviceInfo"),
        }
    }
}
