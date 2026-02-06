//! Signaling relay client for cross-network WebRTC pairing.
//!
//! Connects to the signaling server via WebSocket, joins a room derived from
//! the pairing code (SHA-256 hash), and relays opaque byte payloads (SPAKE2
//! blobs, encrypted SDP/ICE) between peers.
//!
//! The signaling server is untrusted — it only sees hashed room IDs and
//! encrypted payloads.

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::Message;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{SinkExt, StreamExt};
use ring::digest;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Wire protocol types (match signaling-server)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "join")]
    Join { room: String, from: String },
    #[serde(rename = "relay")]
    Relay {
        room: String,
        from: String,
        payload: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "relay")]
    Relay {
        #[allow(dead_code)]
        room: String,
        #[allow(dead_code)]
        from: String,
        payload: String,
    },
    #[serde(rename = "peer_joined")]
    PeerJoined {
        #[allow(dead_code)]
        room: String,
        from: String,
    },
    #[serde(rename = "peer_left")]
    PeerLeft {
        #[allow(dead_code)]
        room: String,
        from: String,
    },
}

// ---------------------------------------------------------------------------
// SignalingClient
// ---------------------------------------------------------------------------

/// A connection to the signaling relay server.
///
/// Provides async send/recv of opaque byte payloads, and notifies when
/// a peer joins or leaves the room.
pub struct SignalingClient {
    /// Send payloads to the relay (base64-encoded internally).
    outbound_tx: mpsc::Sender<Vec<u8>>,
    /// Receive payloads relayed from the peer.
    inbound_rx: mpsc::Receiver<SignalingEvent>,
    /// Room identifier (SHA-256 of pairing code).
    room: String,
    /// This device's identifier.
    device_id: String,
    /// Handle to the background task (kept alive).
    _task: tokio::task::JoinHandle<()>,
}

/// Events received from the signaling server.
#[derive(Debug)]
pub enum SignalingEvent {
    /// A byte payload relayed from the peer.
    Payload(Vec<u8>),
    /// A peer joined the room.
    PeerJoined(String),
    /// A peer left the room.
    PeerLeft(String),
}

impl SignalingClient {
    /// Connect to the signaling server and join a room derived from the pairing code.
    pub async fn connect(
        url: &str,
        pairing_code: &str,
        device_id: &str,
    ) -> Result<Self, String> {
        let room = room_id_from_pairing_code(pairing_code);

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| format!("Failed to connect to signaling server: {}", e))?;

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // Join the room
        let join_msg = ClientMessage::Join {
            room: room.clone(),
            from: device_id.to_string(),
        };
        let json = serde_json::to_string(&join_msg).map_err(|e| e.to_string())?;
        ws_write
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Failed to join room: {}", e))?;

        tracing::info!("Signaling: joined room {}", &room[..16]);

        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Vec<u8>>(32);
        let (inbound_tx, inbound_rx) = mpsc::channel::<SignalingEvent>(32);

        let room_clone = room.clone();
        let device_id_owned = device_id.to_string();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Outbound: send payload to relay
                    outbound = outbound_rx.recv() => {
                        match outbound {
                            Some(bytes) => {
                                let relay = ClientMessage::Relay {
                                    room: room_clone.clone(),
                                    from: device_id_owned.clone(),
                                    payload: BASE64.encode(&bytes),
                                };
                                let json = match serde_json::to_string(&relay) {
                                    Ok(j) => j,
                                    Err(_) => continue,
                                };
                                if ws_write.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                            None => break, // Channel closed
                        }
                    }

                    // Inbound: receive from relay
                    incoming = ws_read.next() => {
                        match incoming {
                            Some(Ok(Message::Text(text))) => {
                                let text_str = text.to_string();
                                match serde_json::from_str::<ServerMessage>(&text_str) {
                                    Ok(ServerMessage::Relay { payload, .. }) => {
                                        if let Ok(bytes) = BASE64.decode(&payload) {
                                            if inbound_tx.send(SignalingEvent::Payload(bytes)).await.is_err() {
                                                break;
                                            }
                                        }
                                    }
                                    Ok(ServerMessage::PeerJoined { from, .. }) => {
                                        if inbound_tx.send(SignalingEvent::PeerJoined(from)).await.is_err() {
                                            break;
                                        }
                                    }
                                    Ok(ServerMessage::PeerLeft { from, .. }) => {
                                        if inbound_tx.send(SignalingEvent::PeerLeft(from)).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok(Self {
            outbound_tx,
            inbound_rx,
            room,
            device_id: device_id.to_string(),
            _task: task,
        })
    }

    /// Send an opaque byte payload to the peer via the relay.
    pub async fn send(&self, payload: &[u8]) -> Result<(), String> {
        self.outbound_tx
            .send(payload.to_vec())
            .await
            .map_err(|_| "Signaling channel closed".to_string())
    }

    /// Receive the next event from the signaling server.
    pub async fn recv(&mut self) -> Result<SignalingEvent, String> {
        self.inbound_rx
            .recv()
            .await
            .ok_or_else(|| "Signaling channel closed".to_string())
    }

    /// Wait until a peer joins the room. Returns the peer's device_id.
    pub async fn wait_for_peer(&mut self) -> Result<String, String> {
        loop {
            match self.recv().await? {
                SignalingEvent::PeerJoined(id) => return Ok(id),
                _ => continue,
            }
        }
    }

    /// Close the signaling connection.
    pub fn close(self) {
        // Dropping outbound_tx closes the channel, which terminates the task.
        // _task JoinHandle is also dropped, which is fine since the task will
        // stop on its own when the channel closes.
        drop(self);
    }

    /// Get the room ID.
    pub fn room_id(&self) -> &str {
        &self.room
    }

    /// Get the device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive a room ID from a pairing code using SHA-256.
/// The server never sees the actual pairing code.
pub fn room_id_from_pairing_code(code: &str) -> String {
    let digest = digest::digest(&digest::SHA256, code.as_bytes());
    hex_encode(digest.as_ref())
}

/// Hex-encode bytes (without pulling in a `hex` crate dependency).
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_id_from_pairing_code() {
        let id1 = room_id_from_pairing_code("7-violet-castle");
        let id2 = room_id_from_pairing_code("7-violet-castle");
        let id3 = room_id_from_pairing_code("3-amber-forge");

        // Same code → same room
        assert_eq!(id1, id2);
        // Different code → different room
        assert_ne!(id1, id3);
        // SHA-256 hex = 64 chars
        assert_eq!(id1.len(), 64);
    }

    #[test]
    fn test_room_id_is_sha256() {
        let id = room_id_from_pairing_code("test-code");
        // Verify it's valid hex
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::Join {
            room: "abc123".to_string(),
            from: "device-1".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"join\""));
        assert!(json.contains("\"room\":\"abc123\""));
    }

    #[test]
    fn test_server_message_deserialization() {
        let json = r#"{"type":"peer_joined","room":"abc","from":"device-2"}"#;
        let msg: ServerMessage = serde_json::from_str(json).unwrap();
        match msg {
            ServerMessage::PeerJoined { from, .. } => assert_eq!(from, "device-2"),
            _ => panic!("Expected PeerJoined"),
        }
    }

    #[test]
    fn test_relay_message_round_trip() {
        let payload = vec![1, 2, 3, 4, 5];
        let encoded = BASE64.encode(&payload);
        let json = format!(
            r#"{{"type":"relay","room":"r","from":"d","payload":"{}"}}"#,
            encoded
        );
        let msg: ServerMessage = serde_json::from_str(&json).unwrap();
        match msg {
            ServerMessage::Relay {
                payload: p, ..
            } => {
                let decoded = BASE64.decode(&p).unwrap();
                assert_eq!(decoded, payload);
            }
            _ => panic!("Expected Relay"),
        }
    }
}
