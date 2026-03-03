//! Aurus Signaling Server — opaque WebSocket relay for cross-network sync.
//!
//! This server never sees plaintext sync data. It only relays encrypted blobs
//! between paired devices. Room IDs are SHA-256 hashes of pairing codes, so
//! the server cannot reverse them to the original codes.
//!
//! Security hardening (Phase 3B):
//! - Rate limiting per IP (5 joins/min, 100 relays/min)
//! - Room TTL (10 minute auto-expiry)
//! - Identity validation (relay `from` must match registered client_id)
//! - Max message size (64KB)
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
        ConnectInfo,
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, Instant};

/// Max clients per room (1:1 sync).
const MAX_CLIENTS_PER_ROOM: usize = 2;
/// Max joins per IP per minute.
const MAX_JOINS_PER_MIN: usize = 5;
/// Max relays per IP per minute.
const MAX_RELAYS_PER_MIN: usize = 100;
/// Room time-to-live (10 minutes).
const ROOM_TTL: Duration = Duration::from_secs(600);
/// Max message size (64KB).
const MAX_MESSAGE_SIZE: usize = 65536;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Rooms = Arc<RwLock<HashMap<String, Room>>>;
type RateLimits = Arc<RwLock<HashMap<IpAddr, RateLimitEntry>>>;

struct Room {
    clients: HashMap<String, mpsc::UnboundedSender<String>>,
    created_at: Instant,
}

struct RateLimitEntry {
    joins: Vec<Instant>,
    relays: Vec<Instant>,
}

type SharedState = (Rooms, RateLimits);

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
// Rate limiting
// ---------------------------------------------------------------------------

fn check_rate_limit(
    limits: &mut RateLimitEntry,
    action: &str,
    max: usize,
) -> bool {
    let window = Instant::now() - Duration::from_secs(60);
    let counts = match action {
        "join" => &mut limits.joins,
        "relay" => &mut limits.relays,
        _ => return true,
    };
    counts.retain(|t| *t > window);
    if counts.len() >= max {
        return false; // rate limited
    }
    counts.push(Instant::now());
    true
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let rooms: Rooms = Arc::new(RwLock::new(HashMap::new()));
    let rate_limits: RateLimits = Arc::new(RwLock::new(HashMap::new()));

    // Background task: expire stale rooms every 60s
    let cleanup_rooms = rooms.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut rooms_guard = cleanup_rooms.write().await;
            let now = Instant::now();
            rooms_guard.retain(|room_id, room| {
                if now.duration_since(room.created_at) > ROOM_TTL {
                    tracing::info!("Room {} expired (TTL)", room_id);
                    false
                } else {
                    true
                }
            });
        }
    });

    let state: SharedState = (rooms, rate_limits);

    let app = Router::new()
        .route("/ws", axum::routing::get(ws_handler))
        .with_state(state);

    let addr = "0.0.0.0:8765";
    tracing::info!("Signaling server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State((rooms, rate_limits)): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, rooms, rate_limits, addr.ip()))
}

async fn handle_socket(
    mut socket: WebSocket,
    rooms: Rooms,
    rate_limits: RateLimits,
    client_ip: IpAddr,
) {
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

                        // Max message size check
                        if text_str.len() > MAX_MESSAGE_SIZE {
                            tracing::warn!(
                                "Oversized message ({} bytes) from {:?}, dropping",
                                text_str.len(),
                                client_ip
                            );
                            continue;
                        }

                        let parsed: ClientMessage = match serde_json::from_str(&text_str) {
                            Ok(m) => m,
                            Err(_) => continue,
                        };

                        match parsed {
                            ClientMessage::Join { room, from } => {
                                // Rate limit joins
                                {
                                    let mut limits = rate_limits.write().await;
                                    let entry = limits.entry(client_ip).or_insert_with(|| RateLimitEntry {
                                        joins: Vec::new(),
                                        relays: Vec::new(),
                                    });
                                    if !check_rate_limit(entry, "join", MAX_JOINS_PER_MIN) {
                                        tracing::warn!("Rate limit exceeded for {:?} (joins)", client_ip);
                                        continue;
                                    }
                                }

                                let mut rooms_guard = rooms.write().await;
                                let r = rooms_guard.entry(room.clone()).or_insert_with(|| Room {
                                    clients: HashMap::new(),
                                    created_at: Instant::now(),
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
                                // Validate sender identity matches registered client
                                if client_id.as_ref() != Some(&from) {
                                    tracing::warn!(
                                        "Relay from spoofed identity: claimed {}, registered {:?}",
                                        from,
                                        client_id
                                    );
                                    continue;
                                }

                                // Rate limit relays
                                {
                                    let mut limits = rate_limits.write().await;
                                    let entry = limits.entry(client_ip).or_insert_with(|| RateLimitEntry {
                                        joins: Vec::new(),
                                        relays: Vec::new(),
                                    });
                                    if !check_rate_limit(entry, "relay", MAX_RELAYS_PER_MIN) {
                                        tracing::warn!("Rate limit exceeded for {:?} (relays)", client_ip);
                                        continue;
                                    }
                                }

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
