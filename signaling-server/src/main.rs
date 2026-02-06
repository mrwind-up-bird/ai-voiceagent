//! Aurus Signaling Server — opaque WebSocket relay for cross-network sync.
//!
//! This server never sees plaintext sync data. It only relays encrypted blobs
//! between paired devices. Room IDs are SHA-256 hashes of pairing codes, so
//! the server cannot reverse them to the original codes.
//!
//! Wire protocol (JSON text frames):
//! - Client→Server: { "type": "join",  "room": "<sha256>", "from": "<device_id>" }
//! - Client→Server: { "type": "relay", "room": "...", "from": "...", "payload": "<base64>" }
//! - Server→Client: { "type": "relay", "room": "...", "from": "<other>", "payload": "..." }
//! - Server→Client: { "type": "peer_joined", "room": "...", "from": "<other>" }
//! - Server→Client: { "type": "peer_left",   "room": "...", "from": "<other>" }

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Max clients per room (1:1 sync).
const MAX_CLIENTS_PER_ROOM: usize = 2;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Rooms = Arc<RwLock<HashMap<String, Room>>>;

struct Room {
    clients: HashMap<String, mpsc::UnboundedSender<String>>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "relay")]
    Relay {
        room: String,
        from: String,
        payload: String,
    },
    #[serde(rename = "peer_joined")]
    PeerJoined { room: String, from: String },
    #[serde(rename = "peer_left")]
    PeerLeft { room: String, from: String },
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let rooms: Rooms = Arc::new(RwLock::new(HashMap::new()));

    let app = Router::new()
        .route("/ws", axum::routing::get(ws_handler))
        .with_state(rooms);

    let addr = "0.0.0.0:8765";
    tracing::info!("Signaling server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(rooms): State<Rooms>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, rooms))
}

async fn handle_socket(mut socket: WebSocket, rooms: Rooms) {
    let (relay_tx, mut relay_rx) = mpsc::unbounded_channel::<String>();

    let mut client_room: Option<String> = None;
    let mut client_id: Option<String> = None;

    loop {
        tokio::select! {
            // Outbound: relay messages to this client
            Some(msg) = relay_rx.recv() => {
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }

            // Inbound: read messages from this client
            result = socket.recv() => {
                match result {
                    Some(Ok(Message::Text(text))) => {
                        let text_str = text.to_string();
                        let parsed: ClientMessage = match serde_json::from_str(&text_str) {
                            Ok(m) => m,
                            Err(_) => continue,
                        };

                        match parsed {
                            ClientMessage::Join { room, from } => {
                                let mut rooms_guard = rooms.write().await;
                                let r = rooms_guard.entry(room.clone()).or_insert_with(|| Room {
                                    clients: HashMap::new(),
                                });

                                if r.clients.len() >= MAX_CLIENTS_PER_ROOM {
                                    tracing::warn!("Room {} is full, rejecting {}", room, from);
                                    continue;
                                }

                                // Notify existing peers
                                let peer_joined = ServerMessage::PeerJoined {
                                    room: room.clone(),
                                    from: from.clone(),
                                };
                                let peer_json = serde_json::to_string(&peer_joined).unwrap();
                                for (_, tx) in r.clients.iter() {
                                    let _ = tx.send(peer_json.clone());
                                }

                                r.clients.insert(from.clone(), relay_tx.clone());
                                client_room = Some(room.clone());
                                client_id = Some(from.clone());

                                tracing::info!("Client {} joined room {}", from, room);
                            }
                            ClientMessage::Relay {
                                room,
                                from,
                                payload,
                            } => {
                                let rooms_guard = rooms.read().await;
                                if let Some(r) = rooms_guard.get(&room) {
                                    let relay_msg = ServerMessage::Relay {
                                        room: room.clone(),
                                        from: from.clone(),
                                        payload,
                                    };
                                    let json = serde_json::to_string(&relay_msg).unwrap();
                                    for (id, tx) in r.clients.iter() {
                                        if id != &from {
                                            let _ = tx.send(json.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => continue,
                }
            }
        }
    }

    // Cleanup on disconnect
    if let (Some(room), Some(id)) = (client_room, client_id) {
        let mut rooms_guard = rooms.write().await;
        if let Some(r) = rooms_guard.get_mut(&room) {
            r.clients.remove(&id);

            // Notify remaining peers
            let peer_left = ServerMessage::PeerLeft {
                room: room.clone(),
                from: id.clone(),
            };
            let json = serde_json::to_string(&peer_left).unwrap();
            for (_, tx) in r.clients.iter() {
                let _ = tx.send(json.clone());
            }

            // Auto-cleanup empty rooms
            if r.clients.is_empty() {
                rooms_guard.remove(&room);
                tracing::info!("Room {} removed (empty)", room);
            }
        }

        tracing::info!("Client {} left room {}", id, room);
    }
}
