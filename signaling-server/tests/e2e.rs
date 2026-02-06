//! End-to-end tests for the signaling server.
//!
//! Requires the signaling server running on localhost:8765.
//! Start it with: `cargo run` in the signaling-server directory.
//!
//! Tests verify:
//! 1. Bidirectional message relay between two clients
//! 2. Peer join/leave notifications
//! 3. Room isolation (no cross-room leaking)
//! 4. Room capacity enforcement (max 2 clients)
//! 5. Full SPAKE2 → SDP → ICE signaling flow

use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::Message;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{timeout, Duration};

static ROOM_COUNTER: AtomicU32 = AtomicU32::new(0);

fn unique_room(prefix: &str) -> String {
    let n = ROOM_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}", prefix, n)
}

const SERVER_URL: &str = "ws://localhost:8765/ws";
const TIMEOUT: Duration = Duration::from_secs(5);

type WsStream = async_tungstenite::WebSocketStream<
    async_tungstenite::stream::Stream<
        async_tungstenite::tokio::TokioAdapter<tokio::net::TcpStream>,
        async_tungstenite::tokio::TokioAdapter<
            tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        >,
    >,
>;
type Rx = SplitStream<WsStream>;
type Tx = SplitSink<WsStream, Message>;

async fn recv_json(rx: &mut Rx) -> Value {
    let msg = timeout(TIMEOUT, rx.next())
        .await
        .expect("timeout waiting for message")
        .expect("stream ended")
        .expect("ws error");
    match msg {
        Message::Text(t) => serde_json::from_str(&t).expect("invalid json"),
        other => panic!("expected text, got: {:?}", other),
    }
}

async fn send_json(tx: &mut Tx, val: Value) {
    SinkExt::send(tx, Message::Text(val.to_string()))
        .await
        .unwrap();
}

async fn connect() -> (Tx, Rx) {
    let (ws, _) = connect_async(SERVER_URL).await.unwrap();
    ws.split()
}

#[tokio::test]
async fn test_basic_bidirectional_relay() {
    let room = unique_room("relay");
    let (mut tx_a, mut rx_a) = connect().await;
    let (mut tx_b, mut rx_b) = connect().await;

    send_json(&mut tx_a, json!({"type":"join","room":room,"from":"a"})).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    send_json(&mut tx_b, json!({"type":"join","room":room,"from":"b"})).await;

    // A gets peer_joined for B
    let n = recv_json(&mut rx_a).await;
    assert_eq!(n["type"], "peer_joined");
    assert_eq!(n["from"], "b");

    // A → B
    send_json(
        &mut tx_a,
        json!({"type":"relay","room":&room,"from":"a","payload":"aGVsbG8="}),
    )
    .await;
    let m = recv_json(&mut rx_b).await;
    assert_eq!(m["type"], "relay");
    assert_eq!(m["from"], "a");
    assert_eq!(m["payload"], "aGVsbG8=");

    // B → A
    send_json(
        &mut tx_b,
        json!({"type":"relay","room":&room,"from":"b","payload":"d29ybGQ="}),
    )
    .await;
    let m = recv_json(&mut rx_a).await;
    assert_eq!(m["payload"], "d29ybGQ=");
}

#[tokio::test]
async fn test_peer_join_leave_notifications() {
    let room = unique_room("notify");
    let (mut tx_a, mut rx_a) = connect().await;
    let (mut tx_b, rx_b) = connect().await;

    send_json(&mut tx_a, json!({"type":"join","room":&room,"from":"a"})).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    send_json(&mut tx_b, json!({"type":"join","room":&room,"from":"b"})).await;

    let n = recv_json(&mut rx_a).await;
    assert_eq!(n["type"], "peer_joined");
    assert_eq!(n["from"], "b");

    // B sends close frame then drops
    SinkExt::send(&mut tx_b, Message::Close(None)).await.ok();
    drop(tx_b);
    drop(rx_b);

    let n = recv_json(&mut rx_a).await;
    assert_eq!(n["type"], "peer_left");
    assert_eq!(n["from"], "b");
}

#[tokio::test]
async fn test_room_isolation() {
    let room_x = unique_room("iso-x");
    let room_y = unique_room("iso-y");
    let (mut tx_a, _rx_a) = connect().await;
    let (mut tx_b, mut rx_b): (Tx, Rx) = connect().await;

    send_json(&mut tx_a, json!({"type":"join","room":&room_x,"from":"a"})).await;
    send_json(&mut tx_b, json!({"type":"join","room":&room_y,"from":"b"})).await;

    // A sends in room-x
    send_json(
        &mut tx_a,
        json!({"type":"relay","room":&room_x,"from":"a","payload":"secret"}),
    )
    .await;

    // B should NOT receive it (different room)
    let result: Result<_, _> = timeout(Duration::from_millis(500), rx_b.next()).await;
    assert!(result.is_err(), "B shouldn't receive from different room");
}

#[tokio::test]
async fn test_room_full_rejection() {
    let room = unique_room("full");
    let (mut tx_a, mut rx_a) = connect().await;
    let (mut tx_b, mut rx_b): (Tx, Rx) = connect().await;
    let (mut tx_c, _rx_c) = connect().await;

    send_json(&mut tx_a, json!({"type":"join","room":&room,"from":"a"})).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    send_json(&mut tx_b, json!({"type":"join","room":&room,"from":"b"})).await;

    let n = recv_json(&mut rx_a).await;
    assert_eq!(n["type"], "peer_joined");
    assert_eq!(n["from"], "b");

    // C tries to join — should be silently rejected (room full at 2)
    send_json(&mut tx_c, json!({"type":"join","room":&room,"from":"c"})).await;

    // Neither A nor B should get a peer_joined for C
    let result_a: Result<_, _> = timeout(Duration::from_millis(500), rx_a.next()).await;
    assert!(
        result_a.is_err(),
        "A should not get peer_joined for rejected C"
    );

    let result_b: Result<_, _> = timeout(Duration::from_millis(100), rx_b.next()).await;
    assert!(
        result_b.is_err(),
        "B should not get peer_joined for rejected C"
    );
}

#[tokio::test]
async fn test_full_signaling_flow() {
    let room = unique_room("flow");
    let (mut tx_c, mut rx_c) = connect().await;
    let (mut tx_j, mut rx_j) = connect().await;

    send_json(
        &mut tx_c,
        json!({"type":"join","room":&room,"from":"creator"}),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    send_json(
        &mut tx_j,
        json!({"type":"join","room":&room,"from":"joiner"}),
    )
    .await;

    // Creator sees peer_joined
    let n = recv_json(&mut rx_c).await;
    assert_eq!(n["type"], "peer_joined");

    // 1. Creator → Joiner: SPAKE2
    send_json(
        &mut tx_c,
        json!({"type":"relay","room":&room,"from":"creator","payload":"spake2_creator"}),
    )
    .await;
    let m = recv_json(&mut rx_j).await;
    assert_eq!(m["payload"], "spake2_creator");

    // 2. Joiner → Creator: SPAKE2 reply
    send_json(
        &mut tx_j,
        json!({"type":"relay","room":&room,"from":"joiner","payload":"spake2_joiner"}),
    )
    .await;
    let m = recv_json(&mut rx_c).await;
    assert_eq!(m["payload"], "spake2_joiner");

    // 3. Creator → Joiner: encrypted SDP offer
    send_json(
        &mut tx_c,
        json!({"type":"relay","room":&room,"from":"creator","payload":"enc_sdp_offer"}),
    )
    .await;
    let m = recv_json(&mut rx_j).await;
    assert_eq!(m["payload"], "enc_sdp_offer");

    // 4. Creator → Joiner: 3 ICE candidates
    for i in 0..3 {
        let p = format!("ice_c_{}", i);
        send_json(
            &mut tx_c,
            json!({"type":"relay","room":&room,"from":"creator","payload":p}),
        )
        .await;
        let m = recv_json(&mut rx_j).await;
        assert_eq!(m["type"], "relay");
    }

    // 5. Joiner → Creator: SDP answer
    send_json(
        &mut tx_j,
        json!({"type":"relay","room":&room,"from":"joiner","payload":"enc_sdp_answer"}),
    )
    .await;
    let m = recv_json(&mut rx_c).await;
    assert_eq!(m["payload"], "enc_sdp_answer");

    // 6. Joiner → Creator: 2 ICE candidates
    for i in 0..2 {
        let p = format!("ice_j_{}", i);
        send_json(
            &mut tx_j,
            json!({"type":"relay","room":&room,"from":"joiner","payload":p}),
        )
        .await;
        let m = recv_json(&mut rx_c).await;
        assert_eq!(m["type"], "relay");
    }
}
